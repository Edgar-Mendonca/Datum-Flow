//! The document: the model plus its edits. Every change goes through `apply`, which works on a
//! copy and keeps it only if the result is sound, so a failed edit leaves the document exactly as
//! it was. Undo and redo keep whole models (they are small).

use crate::model::*;
use serde::{Deserialize, Serialize};

/// One edit. The window, the command line and the tests all change the model with these.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    SetName { name: String },
    SetNotes { notes: String },
    SetUnits { units: Units },
    SetGrid { grid: f64 },
    /// Add a shape; its id is chosen here. `edges` may name the edges.
    AddShape { name: String, hole: bool, geom: Geom, #[serde(default)] edges: Vec<String> },
    /// Add several shapes at once (an imported drawing): all or none.
    AddShapes { shapes: Vec<(String, bool, Geom)> },
    /// Change a shape's outline (boundaries on edges that remain are kept).
    SetGeom { id: u32, geom: Geom },
    SetShape { id: u32, name: String, hole: bool },
    /// Make a polygon's side an arc through a point (a point on the side's line makes it straight).
    ArcThrough { shape: u32, edge: u32, through: [f64; 2] },
    /// Round a polygon's corner with an arc of the given radius, between its two straight sides.
    RoundCorner { shape: u32, corner: u32, radius: f64 },
    MoveShape { id: u32, dx: f64, dy: f64 },
    RemoveShape { id: u32 },
    NameEdge { shape: u32, edge: u32, name: String },
    SetFluid { fluid: Fluid },
    SetPhysics { physics: Physics },
    /// Add (id 0) or change a boundary. Its edges leave any other boundary.
    SetBoundary { boundary: Boundary },
    RemoveBoundary { id: u32 },
    /// Put every edge that has no boundary yet on a new fixed wall.
    WallTheRest,
    SetMesh { mesh: MeshSettings },
    SetSolver { solver: SolverSettings },
    SetMonitors { monitors: Vec<Monitor> },
    SetProbes { probes: Vec<Probe> },
    SetLines { lines: Vec<Line> },
    SetView { view: View },
    /// Replace the whole model (templates and examples).
    Replace { model: Box<Model> },
}

const UNDO_LIMIT: usize = 200;

#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub m: Model,
    undo: Vec<Model>,
    redo: Vec<Model>,
    /// Counts every change, so the shell knows when results are out of date.
    pub revision: u64,
}

/// Which part of the model an edit touches: results stay valid after edits that only change how
/// they are shown.
pub fn keeps_results(op: &Op) -> bool {
    matches!(op, Op::SetName { .. } | Op::SetNotes { .. } | Op::SetUnits { .. } | Op::SetGrid { .. } | Op::SetMonitors { .. } | Op::SetProbes { .. } | Op::SetLines { .. } | Op::SetView { .. } | Op::NameEdge { .. })
}

impl Doc {
    pub fn new(m: Model) -> Doc {
        Doc { m, undo: vec![], redo: vec![], revision: 0 }
    }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }

    /// Apply one edit. On an error nothing has changed.
    pub fn apply(&mut self, op: Op) -> Result<(), String> {
        let mut m = self.m.clone();
        edit(&mut m, op)?;
        m.normalize();
        if m == self.m {
            return Ok(());
        }
        self.undo.push(std::mem::replace(&mut self.m, m));
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.revision += 1;
        Ok(())
    }
    pub fn undo(&mut self) -> bool {
        let Some(m) = self.undo.pop() else { return false };
        self.redo.push(std::mem::replace(&mut self.m, m));
        self.revision += 1;
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(m) = self.redo.pop() else { return false };
        self.undo.push(std::mem::replace(&mut self.m, m));
        self.revision += 1;
        true
    }
}

fn finite(v: &[f64], what: &str) -> Result<(), String> {
    if v.iter().all(|x| x.is_finite()) { Ok(()) } else { Err(format!("{what} must be a number.")) }
}
fn name(s: &str) -> String {
    s.trim().chars().filter(|c| !c.is_control()).take(80).collect()
}

fn edit(m: &mut Model, op: Op) -> Result<(), String> {
    match op {
        Op::SetName { name: n } => {
            let n = name(&n);
            if n.is_empty() { return Err("The model needs a name.".into()); }
            m.name = n;
        }
        Op::SetNotes { notes } => m.notes = notes.chars().filter(|c| !c.is_control() || *c == '\n').take(4000).collect(),
        Op::SetUnits { units } => m.units = units,
        Op::SetGrid { grid } => {
            if !(grid.is_finite() && grid > 0.0) { return Err("The grid step must be above zero.".into()); }
            m.grid = grid;
        }
        Op::AddShape { name: n, hole, geom, edges } => {
            if let Some(p) = geom.problem() { return Err(format!("This shape cannot be used: {p}.")); }
            if m.shapes.len() >= 500 { return Err("A model can have at most 500 shapes.".into()); }
            let id = m.next_id();
            let n = name(&n);
            let kind = match &geom { Geom::Circle { .. } => "Circle", Geom::Poly { pts, bulge } if pts.len() == 4 && bulge.iter().all(|b| *b == 0.0) => "Rectangle", _ => "Polygon" };
            let n = if n.is_empty() { format!("{kind} {id}") } else { n };
            let mut e: Vec<String> = edges.iter().map(|x| name(x)).collect();
            e.resize(geom.edges(), String::new());
            m.shapes.push(Shape { id, name: n, hole, geom, edges: e });
        }
        Op::AddShapes { shapes } => {
            if shapes.is_empty() { return Err("There is nothing to add.".into()); }
            for (n, hole, geom) in shapes {
                edit(m, Op::AddShape { name: n, hole, geom, edges: vec![] })?;
            }
        }
        Op::SetGeom { id, geom } => {
            if let Some(p) = geom.problem() { return Err(format!("This shape cannot be used: {p}.")); }
            let s = m.shapes.iter_mut().find(|s| s.id == id).ok_or("That shape no longer exists.")?;
            s.edges.resize(geom.edges(), String::new());
            s.geom = geom;
        }
        Op::ArcThrough { shape, edge, through } => {
            finite(&through, "The point")?;
            let s = m.shapes.iter_mut().find(|s| s.id == shape).ok_or("That shape no longer exists.")?;
            let Geom::Poly { pts, bulge } = &mut s.geom else { return Err("Only the sides of a polygon or rectangle can be made arcs.".into()) };
            let n = pts.len();
            let e = edge as usize;
            if e >= n { return Err("That side no longer exists.".into()); }
            let b = bulge_through(pts[e], pts[(e + 1) % n], through).ok_or("No arc goes there: pick a point away from the side's ends.")?;
            bulge.resize(n, 0.0);
            bulge[e] = b;
            if let Some(p) = s.geom.problem() { return Err(format!("That arc cannot be used: {p}.")); }
        }
        Op::RoundCorner { shape, corner, radius } => {
            finite(&[radius], "The radius")?;
            if radius <= 0.0 { return Err("The radius must be above zero.".into()); }
            let s = m.shapes.iter_mut().find(|s| s.id == shape).ok_or("That shape no longer exists.")?;
            let Geom::Poly { pts, bulge } = &mut s.geom else { return Err("Only the corners of a polygon or rectangle can be rounded.".into()) };
            let n = pts.len();
            let i = corner as usize;
            if i >= n { return Err("That corner no longer exists.".into()); }
            bulge.resize(n, 0.0);
            let (before, after) = ((i + n - 1) % n, i);
            if bulge[before] != 0.0 || bulge[after] != 0.0 { return Err("Only a corner between two straight sides can be rounded.".into()); }
            let (p, a, b) = (pts[i], pts[(i + n - 1) % n], pts[(i + 1) % n]);
            let (la, lb) = ((a[0] - p[0]).hypot(a[1] - p[1]), (b[0] - p[0]).hypot(b[1] - p[1]));
            let (u, w) = ([(a[0] - p[0]) / la, (a[1] - p[1]) / la], [(b[0] - p[0]) / lb, (b[1] - p[1]) / lb]);
            let alpha = (u[0] * w[0] + u[1] * w[1]).clamp(-1.0, 1.0).acos();
            if !(1e-3..=std::f64::consts::PI - 1e-3).contains(&alpha) { return Err("That corner is straight or folded back; there is nothing to round.".into()); }
            let d = radius / (alpha / 2.0).tan();
            if d >= 0.999 * la.min(lb) {
                return Err(format!("The radius is too large for the sides at that corner: at most about {:.3e} m here.", 0.999 * la.min(lb) * (alpha / 2.0).tan()));
            }
            let (ta, tb) = ([p[0] + u[0] * d, p[1] + u[1] * d], [p[0] + w[0] * d, p[1] + w[1] * d]);
            // The arc is tangent to both sides and bends towards the corner.
            let mag = ((std::f64::consts::PI - alpha) / 4.0).tan();
            let side = |x: [f64; 2], y: [f64; 2], q: [f64; 2]| (y[0] - x[0]) * (q[1] - x[1]) - (y[1] - x[1]) * (q[0] - x[0]);
            let fb = if side(ta, tb, arc_mid(ta, tb, mag)).signum() == side(ta, tb, p).signum() { mag } else { -mag };
            pts[i] = ta;
            pts.insert(i + 1, tb);
            bulge.insert(i, fb);
            // Sides after the new arc move up by one: keep names, boundaries and local sizes on them.
            let name = s.edges.get(before).cloned().unwrap_or_default();
            s.edges.insert(i, if name.is_empty() { String::new() } else { format!("{name} (round)") });
            if let Some(p) = s.geom.problem() { return Err(format!("That rounding cannot be used: {p}.")); }
            let shift = |e: &mut EdgeRef| if e.shape == shape && e.edge as usize >= i { e.edge += 1 };
            for b in &mut m.boundaries {
                b.edges.iter_mut().for_each(shift);
                // The new arc joins the boundary of its two sides when they share one.
                let has = |k: usize| b.edges.contains(&EdgeRef { shape, edge: k as u32 });
                if has(before) && has(i + 1) { b.edges.push(EdgeRef { shape, edge: i as u32 }); b.edges.sort(); }
            }
            for r in &mut m.mesh.refine {
                if let Target::Edge { shape: sh, edge } = &mut r.at { if *sh == shape && *edge as usize >= i { *edge += 1; } }
            }
        }
        Op::SetShape { id, name: n, hole } => {
            let s = m.shapes.iter_mut().find(|s| s.id == id).ok_or("That shape no longer exists.")?;
            let n = name(&n);
            if !n.is_empty() { s.name = n; }
            s.hole = hole;
        }
        Op::MoveShape { id, dx, dy } => {
            finite(&[dx, dy], "The distance")?;
            let s = m.shapes.iter_mut().find(|s| s.id == id).ok_or("That shape no longer exists.")?;
            match &mut s.geom {
                Geom::Circle { c, .. } => { c[0] += dx; c[1] += dy; }
                Geom::Poly { pts, .. } => for p in pts { p[0] += dx; p[1] += dy; },
            }
            if let Some(p) = s.geom.problem() { return Err(format!("The shape cannot go there: {p}.")); }
        }
        Op::RemoveShape { id } => {
            let n = m.shapes.len();
            m.shapes.retain(|s| s.id != id);
            if m.shapes.len() == n { return Err("That shape no longer exists.".into()); }
            m.mesh.refine.retain(|r| !matches!(r.at, Target::Edge { shape, .. } if shape == id));
        }
        Op::NameEdge { shape, edge, name: n } => {
            let s = m.shapes.iter_mut().find(|s| s.id == shape).ok_or("That shape no longer exists.")?;
            let slot = s.edges.get_mut(edge as usize).ok_or("That edge no longer exists.")?;
            *slot = name(&n);
        }
        Op::SetFluid { fluid } => {
            finite(&[fluid.rho, fluid.mu, fluid.k, fluid.cp, fluid.beta, fluid.at], "Every fluid property")?;
            if fluid.rho <= 0.0 { return Err("The density must be above zero.".into()); }
            if fluid.mu <= 0.0 { return Err("The viscosity must be above zero.".into()); }
            if fluid.k <= 0.0 { return Err("The conductivity must be above zero.".into()); }
            if fluid.cp <= 0.0 { return Err("The specific heat must be above zero.".into()); }
            if fluid.at <= 0.0 { return Err("The temperature must be above absolute zero.".into()); }
            m.fluid = Fluid { name: name(&fluid.name), ..fluid };
        }
        Op::SetPhysics { physics } => {
            finite(&[physics.gravity[0], physics.gravity[1], physics.t_ref, physics.length, physics.depth], "Every physics value")?;
            if physics.t_ref <= 0.0 { return Err("The reference temperature must be above absolute zero.".into()); }
            if physics.depth <= 0.0 { return Err("The depth must be above zero.".into()); }
            if physics.length < 0.0 { return Err("The reference length cannot be negative.".into()); }
            m.physics = Physics { buoyancy: physics.buoyancy && physics.heat, ..physics };
        }
        Op::SetBoundary { mut boundary } => {
            match &boundary.kind {
                BcKind::Inlet { speed, .. } => {
                    finite(&[*speed], "The inlet speed")?;
                    if *speed < 0.0 { return Err("The inlet speed cannot be negative (use an outlet for flow that leaves).".into()); }
                }
                BcKind::Outlet { pressure } => finite(&[*pressure], "The outlet pressure")?,
                BcKind::Wall { u } => finite(u, "The wall speed")?,
                BcKind::Symmetry => {}
            }
            match &boundary.heat {
                HeatBc::Temperature { t } => {
                    finite(&[*t], "The temperature")?;
                    if *t <= 0.0 { return Err("The temperature must be above absolute zero.".into()); }
                }
                HeatBc::Flux { q } => finite(&[*q], "The heat flux")?,
                HeatBc::Insulated => {}
            }
            boundary.name = name(&boundary.name);
            let all = m.all_edges();
            boundary.edges.retain(|e| all.contains(e));
            boundary.edges.sort();
            boundary.edges.dedup();
            if boundary.id == 0 {
                if m.boundaries.len() >= 500 { return Err("A model can have at most 500 boundaries.".into()); }
                boundary.id = m.next_id();
            } else if m.boundary(boundary.id).is_none() {
                return Err("That boundary no longer exists.".into());
            }
            if boundary.name.is_empty() {
                boundary.name = format!("{} {}", match boundary.kind { BcKind::Inlet { .. } => "Inlet", BcKind::Outlet { .. } => "Outlet", BcKind::Wall { .. } => "Wall", BcKind::Symmetry => "Symmetry" }, boundary.id);
            }
            for b in &mut m.boundaries {
                if b.id != boundary.id { b.edges.retain(|e| !boundary.edges.contains(e)); }
            }
            match m.boundaries.iter_mut().find(|b| b.id == boundary.id) {
                Some(b) => *b = boundary,
                None => m.boundaries.push(boundary),
            }
        }
        Op::RemoveBoundary { id } => {
            let n = m.boundaries.len();
            m.boundaries.retain(|b| b.id != id);
            if m.boundaries.len() == n { return Err("That boundary no longer exists.".into()); }
        }
        Op::WallTheRest => {
            let free: Vec<EdgeRef> = m.all_edges().into_iter().filter(|e| m.boundary_of(*e).is_none()).collect();
            if free.is_empty() { return Err("Every edge already has a boundary.".into()); }
            let id = m.next_id();
            m.boundaries.push(Boundary { id, name: "Walls".into(), edges: free, kind: BcKind::Wall { u: [0.0; 2] }, heat: HeatBc::Insulated });
        }
        Op::SetMesh { mesh } => {
            finite(&[mesh.size, mesh.wall, mesh.corner, mesh.wake, mesh.growth, mesh.min_angle], "Every mesh setting")?;
            if mesh.size < 0.0 { return Err("The element size cannot be negative (0 means automatic).".into()); }
            if mesh.refine.iter().any(|r| !(r.size.is_finite() && r.size > 0.0)) { return Err("A local element size must be above zero.".into()); }
            if mesh.refine.len() > 500 { return Err("A model can have at most 500 local mesh sizes.".into()); }
            m.mesh = mesh;
        }
        Op::SetSolver { solver } => {
            finite(&[solver.tol, solver.grad_div], "Every solver setting")?;
            if solver.max_iter == 0 { return Err("The solver needs at least one iteration.".into()); }
            if solver.tol <= 0.0 { return Err("The tolerance must be above zero.".into()); }
            m.solver = solver;
        }
        Op::SetMonitors { monitors } => {
            if monitors.len() > 100 { return Err("A model can have at most 100 monitors.".into()); }
            m.monitors = monitors.into_iter().map(|x| Monitor { name: name(&x.name), kind: x.kind }).collect();
        }
        Op::SetProbes { probes } => {
            if probes.len() > 1000 { return Err("A model can have at most 1000 probes.".into()); }
            if probes.iter().any(|p| !(p.x.is_finite() && p.y.is_finite())) { return Err("A probe's position must be a number.".into()); }
            m.probes = probes.into_iter().map(|p| Probe { name: name(&p.name), ..p }).collect();
        }
        Op::SetLines { lines } => {
            if lines.len() > 100 { return Err("A model can have at most 100 plot lines.".into()); }
            for l in &lines {
                finite(&[l.a[0], l.a[1], l.b[0], l.b[1]], "A plot line's ends")?;
                if l.a == l.b { return Err("A plot line needs two different ends.".into()); }
            }
            m.lines = lines.into_iter().map(|l| Line { name: name(&l.name), ..l }).collect();
        }
        Op::SetView { view } => m.view = view,
        Op::Replace { model } => *m = *model,
    }
    Ok(())
}
