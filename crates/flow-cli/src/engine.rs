use flow_core::model::*;
use flow_core::numbers::numbers;
use flow_mesh::{check, Issue, Mesh, Quality};
use flow_post::values::{along, arrows, monitor, sample, Sample, Values};
use flow_post::{fields, Locator};
use flow_solve::{solve, Row, Setup, Solved, SparseLu, State, Stats, Step};
use serde::Serialize;
use std::sync::Arc;

type R<T> = Result<T, String>;

/// The mesh, in the compact form the window draws from.
#[derive(Clone, Debug, Serialize)]
pub struct MeshView {
    /// x, y of every node (corner nodes first).
    pub nodes: Vec<f32>,
    pub nv: usize,
    /// Six node numbers per element.
    pub tris: Vec<u32>,
    /// Four numbers per boundary edge: start, middle, end node, boundary id.
    pub edges: Vec<u32>,
    /// Smallest angle of every element (degrees).
    pub angle: Vec<f32>,
    pub quality: Quality,
    pub size: f64,
    pub seconds: f64,
}

/// One field at every node, in SI units.
#[derive(Clone, Debug, Serialize)]
pub struct NodalField {
    pub id: String,
    pub name: &'static str,
    /// Kind of unit ("speed", "pressure", "temp", …).
    pub kind: &'static str,
    pub values: Vec<f32>,
    pub min: f64,
    pub max: f64,
    /// For the stream function: the levels to draw as streamlines.
    pub levels: Vec<f64>,
}

/// One line of the values table.
#[derive(Clone, Debug, Serialize)]
pub struct ValueRow {
    pub group: &'static str,
    pub name: String,
    pub value: f64,
    pub kind: &'static str,
    /// A remark beside the value ("should be near zero").
    pub note: &'static str,
}

/// How the last solve went.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    pub converged: bool,
    pub stopped: bool,
    pub message: String,
    pub advice: Vec<String>,
    pub stats: Stats,
    pub history: Vec<Row>,
    /// The monitors' names, kinds of unit, and their value after every iteration.
    pub monitor_names: Vec<String>,
    pub monitor_kinds: Vec<&'static str>,
    pub monitors: Vec<Vec<f64>>,
}

/// What the window is told after every iteration.
#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub row: Row,
    pub monitors: Vec<f64>,
}

struct Results {
    solved: Solved,
    locator: Locator,
    psi: Option<Vec<f64>>,
    summary: Summary,
}

/// The mesh and the results that belong to one revision of the model.
#[derive(Default)]
pub struct Engine {
    /// The document revision the mesh and results belong to.
    pub revision: u64,
    mesh: Option<(Arc<Mesh>, f64)>,
    results: Option<Results>,
}

/// A solve, ready to run away from the document (on a worker thread): it owns copies of what it needs.
pub struct Job {
    pub revision: u64,
    model: Model,
    mesh: Arc<Mesh>,
    setup: Setup,
    start: Option<State>,
}

impl Job {
    pub fn monitor_names(&self) -> Vec<String> {
        self.model.monitors.iter().map(|m| m.name.clone()).collect()
    }
    /// Run the solve. `on` hears every iteration and stops it by returning false.
    pub fn run(&self, on: &mut dyn FnMut(&Progress) -> bool) -> R<(Solved, Vec<Vec<f64>>)> {
        let loc = Locator::new(&self.mesh);
        let mut history = vec![];
        let solved = solve(&self.mesh, &self.setup, &self.model.solver, self.start.as_ref(), &mut SparseLu::default(), &mut |s: &Step| {
            let state = State::from_vec(s.dofs, s.x);
            let v = Values::new(&self.model, &self.mesh, &state, s.residual, s.dofs);
            let monitors: Vec<f64> = self.model.monitors.iter().map(|m| monitor(&v, &loc, &m.kind).0).collect();
            history.push(monitors.clone());
            on(&Progress { row: s.row, monitors })
        })?;
        Ok((solved, history))
    }
}

impl Engine {
    pub fn invalidate(&mut self) {
        *self = Engine::default();
    }
    pub fn has_mesh(&self) -> bool {
        self.mesh.is_some()
    }
    pub fn has_results(&self) -> bool {
        self.results.is_some()
    }
    pub fn mesh_ref(&self) -> Option<&Mesh> {
        self.mesh.as_ref().map(|m| &*m.0)
    }

    /// Mesh the model (again).
    pub fn mesh(&mut self, m: &Model, revision: u64) -> R<MeshView> {
        let t = std::time::Instant::now();
        let g = flow_mesh::mesh(m)?;
        self.results = None;
        self.mesh = Some((Arc::new(g), t.elapsed().as_secs_f64()));
        self.revision = revision;
        self.mesh_view().ok_or_else(|| "There is no mesh.".into())
    }
    pub fn mesh_view(&self) -> Option<MeshView> {
        let (g, seconds) = self.mesh.as_ref()?;
        Some(MeshView {
            nodes: g.nodes.iter().flat_map(|p| [p[0] as f32, p[1] as f32]).collect(),
            nv: g.nv,
            tris: g.tris.iter().flatten().copied().collect(),
            edges: g.edges.iter().flat_map(|e| [e.n[0], e.n[1], e.n[2], e.boundary]).collect(),
            angle: g.angle.clone(),
            quality: g.quality.clone(),
            size: g.size,
            seconds: *seconds,
        })
    }
    /// The checks before solving, against the mesh if there is one.
    pub fn check(&self, m: &Model) -> Vec<Issue> {
        check(m, self.mesh_ref())
    }

    /// Get a solve ready: mesh if needed, refuse if the checks find an error.
    pub fn job(&mut self, m: &Model, revision: u64) -> R<Job> {
        if self.mesh.is_none() || self.revision != revision {
            self.mesh(m, revision)?;
        }
        let mesh = self.mesh.as_ref().expect("meshed above").0.clone();
        let issues = check(m, Some(&mesh));
        if let Some(e) = issues.iter().find(|i| i.level == "error") {
            return Err(format!("{} {}", e.text, e.fix));
        }
        let setup = Setup::from_model(m, &mesh)?;
        // Carry on from the last answer on this mesh, if there is one (resume).
        let start = self.results.as_ref().map(|r| r.solved.state.clone());
        Ok(Job { revision, model: m.clone(), mesh, setup, start })
    }
    /// Keep what a job found. Ignored if the model has moved on since the job was made.
    pub fn finish(&mut self, job: &Job, solved: Solved, monitors: Vec<Vec<f64>>) -> Option<Summary> {
        if self.revision != job.revision || !self.mesh.as_ref().is_some_and(|m| Arc::ptr_eq(&m.0, &job.mesh)) {
            return None;
        }
        let advice = if solved.converged || solved.stopped { vec![] } else { flow_solve::advice::advice(&job.model, &job.mesh) };
        let kinds = {
            let v = Values::new(&job.model, &job.mesh, &solved.state, &solved.residual, solved.dofs);
            let loc = Locator::new(&job.mesh);
            job.model.monitors.iter().map(|m| monitor(&v, &loc, &m.kind).1).collect()
        };
        let summary = Summary { converged: solved.converged, stopped: solved.stopped, message: solved.message.clone(), advice, stats: solved.stats.clone(), history: solved.history.clone(), monitor_names: job.monitor_names(), monitor_kinds: kinds, monitors };
        self.results = Some(Results { locator: Locator::new(&job.mesh), solved, psi: None, summary: summary.clone() });
        Some(summary)
    }
    /// Solve here and now (the command line and the tests; the window uses a worker thread).
    pub fn solve_now(&mut self, m: &Model, revision: u64, on: &mut dyn FnMut(&Progress) -> bool) -> R<Summary> {
        let job = self.job(m, revision)?;
        let (solved, monitors) = job.run(on)?;
        self.finish(&job, solved, monitors).ok_or_else(|| "The model changed while it was being solved.".into())
    }
    pub fn summary(&self) -> Option<&Summary> {
        self.results.as_ref().map(|r| &r.summary)
    }

    fn need(&self) -> R<(&Mesh, &Results)> {
        match (&self.mesh, &self.results) {
            (Some(m), Some(r)) => Ok((&m.0, r)),
            _ => Err("There are no results yet: solve the model first (Solve › Solve).".into()),
        }
    }
    fn psi(&mut self) -> R<Vec<f64>> {
        let (mesh, r) = self.need()?;
        if let Some(p) = &r.psi {
            return Ok(p.clone());
        }
        let p = fields::stream_function(mesh, &r.solved.state, &mut SparseLu::default())?;
        self.results.as_mut().expect("checked").psi = Some(p.clone());
        Ok(p)
    }
    /// A field at every node. "stream" also brings the levels for `lines` streamlines.
    pub fn field(&mut self, id: &str, lines: usize) -> R<NodalField> {
        let psi = if id == "stream" { Some(self.psi()?) } else { None };
        let (mesh, r) = self.need()?;
        let (_, name, kind) = fields::FIELDS.iter().find(|f| f.0 == id).ok_or("That field does not exist.")?;
        let v = fields::nodal(mesh, &r.solved.state, id, psi.as_deref()).ok_or("This model has no temperature: turn heat on in Physics and solve again.")?;
        let [min, max] = fields::range(&v);
        Ok(NodalField { id: id.into(), name, kind, levels: if id == "stream" { fields::stream_levels(&v, lines.clamp(1, 200)) } else { vec![] }, values: v.iter().map(|x| *x as f32).collect(), min, max })
    }
    /// Velocity arrows on a grid of about `count` points: x, y, u, v.
    pub fn arrows(&self, m: &Model, count: usize) -> R<Vec<[f32; 4]>> {
        let (mesh, r) = self.need()?;
        let b = m.bounds().ok_or("The model has no flow region.")?;
        Ok(arrows(mesh, &r.solved.state, &r.locator, b, count).into_iter().map(|a| a.map(|v| v as f32)).collect())
    }
    pub fn probe(&self, x: f64, y: f64) -> R<Sample> {
        let (mesh, r) = self.need()?;
        Ok(sample(mesh, &r.solved.state, &r.locator, [x, y]))
    }
    pub fn probes(&self, m: &Model) -> R<Vec<(String, Sample)>> {
        let (mesh, r) = self.need()?;
        Ok(m.probes.iter().enumerate().map(|(i, p)| (if p.name.is_empty() { format!("Probe {}", i + 1) } else { p.name.clone() }, sample(mesh, &r.solved.state, &r.locator, [p.x, p.y]))).collect())
    }
    /// The solution along one of the model's plot lines: distance and values.
    pub fn line(&self, m: &Model, index: usize) -> R<Vec<(f64, Sample)>> {
        let (mesh, r) = self.need()?;
        let l = m.lines.get(index).ok_or("That plot line no longer exists.")?;
        Ok(along(mesh, &r.solved.state, &r.locator, l))
    }
    /// The values table: monitors, every boundary's numbers, the balances and the extremes.
    pub fn values(&self, m: &Model) -> R<Vec<ValueRow>> {
        let (mesh, r) = self.need()?;
        let v = Values::new(m, mesh, &r.solved.state, &r.solved.residual, r.solved.dofs);
        let mut out = vec![];
        let mut row = |group: &'static str, name: String, value: f64, kind: &'static str, note: &'static str| out.push(ValueRow { group, name, value, kind, note });
        for mon in &m.monitors {
            let (value, kind) = monitor(&v, &r.locator, &mon.kind);
            row("Monitors", mon.name.clone(), value, kind, "");
        }
        let heat = r.solved.dofs.heat;
        for b in m.boundaries.iter().filter(|b| mesh.edges.iter().any(|e| e.boundary == b.id)) {
            match b.kind {
                BcKind::Inlet { .. } | BcKind::Outlet { .. } => {
                    row("Boundaries", format!("{}: flow out", b.name), v.flow_rate(b.id), "flow", "");
                    row("Boundaries", format!("{}: mean pressure", b.name), v.mean_pressure(b.id), "pressure", "");
                    if heat { row("Boundaries", format!("{}: mean temperature", b.name), v.mean_temperature(b.id), "temp", ""); }
                }
                BcKind::Wall { .. } => {
                    let f = v.force(b.id);
                    row("Boundaries", format!("{}: force x", b.name), f[0], "force", "");
                    row("Boundaries", format!("{}: force y", b.name), f[1], "force", "");
                    if heat && b.heat != HeatBc::Insulated {
                        row("Boundaries", format!("{}: heat into the fluid", b.name), v.heat_flow(b.id), "power", "");
                        if matches!(b.heat, HeatBc::Temperature { .. }) && numbers(m).dt > 0.0 { row("Boundaries", format!("{}: Nusselt number", b.name), v.nusselt(b.id), "ratio", ""); }
                    }
                }
                BcKind::Symmetry => {}
            }
        }
        let (net, inflow) = v.balance();
        row("Balances", "Flow in".into(), inflow, "flow", "");
        row("Balances", "Net flow through all boundaries".into(), net, "flow", "mass conservation: should be near zero");
        if heat {
            let (conducted, carried) = v.heat_balance();
            row("Balances", "Heat conducted in through boundaries".into(), conducted, "power", "");
            row("Balances", "Heat carried out by the flow".into(), carried, "power", "should match the heat conducted in");
        }
        let s = &r.solved.state;
        row("Extremes", "Largest speed".into(), s.u.iter().map(|u| u[0].hypot(u[1])).fold(0.0, f64::max), "speed", "");
        let pr = fields::range(&s.p);
        row("Extremes", "Lowest pressure".into(), pr[0], "pressure", "");
        row("Extremes", "Highest pressure".into(), pr[1], "pressure", "");
        if heat {
            let tr = fields::range(&s.t);
            row("Extremes", "Lowest temperature".into(), tr[0], "temp", "");
            row("Extremes", "Highest temperature".into(), tr[1], "temp", "");
        }
        Ok(out)
    }
    pub fn vtu(&self) -> R<String> {
        let (mesh, r) = self.need()?;
        Ok(flow_post::export::vtu(mesh, &r.solved.state))
    }
    pub fn probes_csv(&self, m: &Model) -> R<String> {
        let heat = self.need()?.1.solved.dofs.heat;
        Ok(flow_post::export::probes_csv(m.units, heat, &self.probes(m)?))
    }
    pub fn line_csv(&self, m: &Model, index: usize) -> R<String> {
        let heat = self.need()?.1.solved.dofs.heat;
        Ok(flow_post::export::line_csv(m.units, heat, &self.line(m, index)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::examples;

    #[test]
    fn a_session_meshes_checks_solves_and_reads_the_cylinder() {
        let m = examples::example("cylinder").unwrap();
        let mut e = Engine::default();
        assert!(e.field("speed", 0).unwrap_err().contains("no results yet") && !e.has_mesh());
        let view = e.mesh(&m, 7).unwrap();
        assert!(view.tris.len() == 6 * view.quality.elements && view.nodes.len() == 2 * view.quality.nodes && view.edges.len() % 4 == 0 && e.revision == 7);
        assert_eq!(e.check(&m)[0].text, "Ready to solve.");
        let mut seen = vec![];
        let s = e.solve_now(&m, 7, &mut |p| { seen.push(p.clone()); true }).unwrap();
        assert!(s.converged && s.advice.is_empty() && s.history.len() == seen.len() && s.monitors.len() == seen.len(), "{}", s.message);
        assert!(s.monitor_names == ["Drag on the cylinder", "Lift on the cylinder"] && s.monitor_kinds == ["force", "force"]);
        // The drag coefficient (force × 500) of the benchmark, on the example's own mesh: 5.5795.
        let rows = e.values(&m).unwrap();
        let get = |name: &str| rows.iter().find(|r| r.name == name).unwrap_or_else(|| panic!("{name}")).value;
        assert!((get("Drag on the cylinder") * 500.0 - 5.5795).abs() < 0.002 && (get("Lift on the cylinder") * 500.0 - 0.0106).abs() < 0.0005, "{}", get("Drag on the cylinder") * 500.0);
        // The monitors were followed while solving and end at the same value.
        assert!((seen.last().unwrap().monitors[0] - get("Drag on the cylinder")).abs() < 1e-12);
        assert!(get("Net flow through all boundaries").abs() < 1e-12 * get("Flow in") && (get("Flow in") - 0.2 * 0.41).abs() < 1e-9);
        assert!((get("Cylinder: force x") - get("Drag on the cylinder")).abs() < 1e-12);
        // Pressure difference front to back: 0.1175.
        let dp = e.probe(0.15, 0.2).unwrap().pressure - e.probe(0.25, 0.2).unwrap().pressure;
        assert!((dp - 0.1175).abs() < 0.0005, "{dp}");
        assert!(!e.probe(0.2, 0.2).unwrap().inside);
        // Fields, streamlines, arrows, lines and files.
        let f = e.field("speed", 0).unwrap();
        assert!(f.values.len() == view.quality.nodes && f.min == 0.0 && f.max > 0.3 && f.max < 0.45 && f.kind == "speed" && f.levels.is_empty());
        let psi = e.field("stream", 20).unwrap();
        assert!(psi.levels.len() == 20 && (psi.max - psi.min - 0.2 * 0.41).abs() < 1e-3);
        assert!(e.field("temperature", 0).unwrap_err().contains("no temperature") && e.field("nope", 0).is_err());
        assert!(e.arrows(&m, 300).unwrap().len() > 200);
        assert_eq!(e.line(&m, 0).unwrap().len(), 201);
        assert!(e.line(&m, 9).is_err() && e.probes(&m).unwrap().len() == 2);
        assert!(e.vtu().unwrap().contains("UnstructuredGrid") && e.probes_csv(&m).unwrap().lines().count() == 3 && e.line_csv(&m, 0).unwrap().lines().count() == 202);
        // Solving again resumes from the answer: one iteration.
        let again = e.solve_now(&m, 7, &mut |_| true).unwrap();
        assert!(again.converged && again.history.len() == 1);
        // A new mesh drops the results.
        e.mesh(&m, 8).unwrap();
        assert!(!e.has_results() && e.has_mesh());
    }

    #[test]
    fn a_blocked_or_failed_solve_says_why_and_what_to_try() {
        let mut e = Engine::default();
        let mut m = examples::example("channel").unwrap();
        m.boundaries[1].kind = BcKind::Wall { u: [0.0; 2] };
        let err = e.solve_now(&m, 1, &mut |_| true).unwrap_err();
        assert!(err.contains("no outlet") && err.contains("pressure outlet"), "{err}");
        // A cavity far too fast for its mesh, with continuation off and few iterations: no
        // convergence, a message and advice, and still a flow to look at.
        let mut m = examples::cavity(20_000.0);
        m.mesh = MeshSettings { size: 0.1, wall: 1.0, corner: 1.0, ..Default::default() };
        m.solver = SolverSettings { max_iter: 6, continuation: Continuation::Off, allow_turbulent: true, ..Default::default() };
        let s = e.solve_now(&m, 2, &mut |_| true).unwrap();
        assert!(!s.converged && s.message.starts_with("Did not converge") && !s.advice.is_empty(), "{s:?}");
        assert!(s.advice.iter().any(|a| a.contains("Raise Re in steps")) && e.has_results());
        // Stopped by the user.
        let m = examples::example("cavity-100").unwrap();
        let s = e.solve_now(&m, 3, &mut |_| false).unwrap();
        assert!(s.stopped && !s.converged && s.advice.is_empty());
    }

    fn solved(id: &str) -> (Model, Engine, Summary, f64) {
        let m = examples::example(id).unwrap();
        let mut e = Engine::default();
        let t = std::time::Instant::now();
        let s = e.solve_now(&m, 1, &mut |_| true).unwrap_or_else(|err| panic!("{id}: {err}"));
        (m, e, s, t.elapsed().as_secs_f64())
    }
    fn conserves(id: &str, limit: f64, most_newton: usize) {
        let (m, e, s, secs) = solved(id);
        assert!(s.converged, "{id}: {}", s.message);
        let rows = e.values(&m).unwrap();
        let get = |name: &str| rows.iter().find(|r| r.name == name).map(|r| r.value).unwrap_or(f64::NAN);
        let n = numbers(&m);
        // Net flow against the flow in, or (a closed box) against speed × size.
        let scale = get("Flow in").max(n.speed.max(get("Largest speed")) * n.length * 1e-3);
        assert!(get("Net flow through all boundaries").abs() <= 1e-9 * scale, "{id}: net {} of {}", get("Net flow through all boundaries"), scale);
        // Newton takes over after the Picard start and finishes in a few steps.
        let newton = s.history.iter().filter(|h| h.kind == "Newton").count();
        assert!(newton <= most_newton && s.stats.steps == 0, "{id}: {newton} Newton steps, {} continuation steps", s.stats.steps);
        assert!(secs < limit, "{id} took {secs:.0} s");
        eprintln!("{id}: {} unknowns, {} iterations ({newton} Newton), {secs:.1} s", s.stats.unknowns, s.history.len());
    }

    #[test]
    fn examples_conserve_mass_and_converge_in_a_few_newton_steps() {
        for id in ["channel", "cavity-100", "cylinder", "heated-1e4", "heatsink"] {
            conserves(id, 120.0, 8);
        }
    }

    #[test]
    #[ignore = "the slower examples: run in release mode, nightly and before a release"]
    fn every_example_solves_in_under_a_minute_and_conserves_mass() {
        for (id, _, _) in examples::EXAMPLES {
            // Ra 10⁶ needs damped Newton steps at first: more of them, still no continuation.
            if id != "empty" { conserves(id, 60.0, if id == "heated-1e6" { 20 } else { 10 }); }
        }
    }

    #[test]
    #[ignore = "the speed gate: run in release mode, nightly and before a release"]
    fn two_hundred_thousand_unknowns_solve_within_a_few_minutes() {
        let mut m = examples::cavity(100.0);
        m.mesh = MeshSettings { size: 0.0069, wall: 1.0, corner: 1.0, ..Default::default() };
        let mut e = Engine::default();
        let t = std::time::Instant::now();
        let s = e.solve_now(&m, 1, &mut |_| true).unwrap();
        let secs = t.elapsed().as_secs_f64();
        eprintln!("speed gate: {} unknowns, {} iterations, {secs:.0} s", s.stats.unknowns, s.history.len());
        assert!(s.converged && s.stats.unknowns >= 200_000, "{} unknowns: {}", s.stats.unknowns, s.message);
        assert!(secs < 300.0, "{secs:.0} s");
    }

    #[test]
    fn a_coarser_and_a_finer_mesh_agree_on_the_examples_numbers() {
        // Mesh independence: halving the element size moves each example's key number only a little.
        let key = |id: &str, factor: f64, name: &str| -> f64 {
            let mut m = examples::example(id).unwrap();
            m.mesh.size /= factor;
            for r in &mut m.mesh.refine { r.size /= factor; }
            let mut e = Engine::default();
            assert!(e.solve_now(&m, 1, &mut |_| true).unwrap().converged);
            e.values(&m).unwrap().iter().find(|r| r.name == name).unwrap_or_else(|| panic!("{name}")).value
        };
        for (id, name, within) in [("channel", "Pressure drop", 1e-9), ("cylinder", "Drag on the cylinder", 2e-3), ("heated-1e4", "Nusselt number, warm wall", 2e-3), ("heatsink", "Heat removed from the fin", 1e-2), ("heatsink", "Pressure drop", 1e-2)] {
            let (coarse, fine) = (key(id, 0.7, name), key(id, 1.4, name));
            assert!((coarse - fine).abs() <= within * fine.abs(), "{id} {name}: {coarse} on the coarse mesh, {fine} on the fine one");
            eprintln!("{id} {name}: {coarse:.6e} → {fine:.6e} ({:.1e})", (coarse - fine).abs() / fine.abs());
        }
    }
}
