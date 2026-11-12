//! The nonlinear solve: a few Picard iterations (robust, slow), then Newton with a line search
//! (fast near the answer). If that fails, or when asked, the driving is raised in steps from a
//! gentle flow (continuation in the Reynolds or Rayleigh number), each step starting from the last.

use crate::assemble::{assemble, free, pattern, Dofs, Free, Load};
use crate::setup::Setup;
use crate::sparse::{Csc, LinearSolver};
use flow_core::model::{Continuation, SolverSettings};
use flow_mesh::Mesh;
use serde::Serialize;

/// The solution as fields: velocity per node, pressure per corner node, temperature per node
/// (empty without heat).
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct State {
    pub u: Vec<[f64; 2]>,
    pub p: Vec<f64>,
    pub t: Vec<f64>,
}

impl State {
    pub fn from_vec(d: Dofs, x: &[f64]) -> State {
        State {
            u: (0..d.nn).map(|i| [x[d.u(i, 0)], x[d.u(i, 1)]]).collect(),
            p: (0..d.nv).map(|i| x[d.p(i)]).collect(),
            t: if d.heat { (0..d.nn).map(|i| x[d.t(i)]).collect() } else { vec![] },
        }
    }
    pub fn to_vec(&self, d: Dofs) -> Vec<f64> {
        let mut x = vec![0.0; d.n()];
        for i in 0..d.nn.min(self.u.len()) {
            x[d.u(i, 0)] = self.u[i][0];
            x[d.u(i, 1)] = self.u[i][1];
        }
        for i in 0..d.nv.min(self.p.len()) {
            x[d.p(i)] = self.p[i];
        }
        if d.heat {
            for i in 0..d.nn.min(self.t.len()) {
                x[d.t(i)] = self.t[i];
            }
        }
        x
    }
}

/// One line of the convergence history: residuals scaled by their largest value so far.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Row {
    pub iter: u32,
    /// "Picard" or "Newton".
    pub kind: &'static str,
    /// Share of the full driving applied (1 unless continuation is stepping up).
    pub load: f64,
    pub momentum: f64,
    pub continuity: f64,
    pub energy: f64,
}
impl Row {
    pub fn worst(&self) -> f64 { self.momentum.max(self.continuity).max(self.energy) }
}

/// What the caller sees after every iteration; return false from the callback to stop.
pub struct Step<'a> {
    pub row: Row,
    pub dofs: Dofs,
    /// The current solution vector and the residual of every equation (reactions at fixed unknowns).
    pub x: &'a [f64],
    pub residual: &'a [f64],
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Stats {
    pub unknowns: usize,
    pub nonzeros: usize,
    pub seconds: f64,
    pub linear_solver: &'static str,
    /// Continuation steps taken (0 = solved directly).
    pub steps: u32,
}

#[derive(Clone, Debug)]
pub struct Solved {
    pub state: State,
    pub x: Vec<f64>,
    /// The residual of every equation at the answer; at fixed unknowns it is the reaction.
    pub residual: Vec<f64>,
    pub dofs: Dofs,
    pub converged: bool,
    /// The user stopped it.
    pub stopped: bool,
    pub history: Vec<Row>,
    /// Why it ended as it did, in plain words.
    pub message: String,
    pub stats: Stats,
}

enum End {
    Converged,
    Stopped,
    /// Ran out of iterations or stalled; the text says which.
    Failed(String),
}

struct Run<'a> {
    mesh: &'a Mesh,
    s: &'a Setup,
    d: Dofs,
    fr: Free,
    jac: Csc,
    /// Largest norm of each block of the residual seen so far: the scale of "converged".
    scale: [f64; 3],
    /// ‖lumped corner areas‖ / L: times a speed, the continuity residual of a flow whose
    /// divergence is as large as speed / L (L = √area). The continuity scale never falls below it.
    weight: f64,
    len: f64,
    /// Largest speed seen so far.
    umax: f64,
    history: Vec<Row>,
    iter: u32,
}

impl Run<'_> {
    fn norms(&self, r: &[f64]) -> [f64; 3] {
        let d = self.d;
        let mut n = [0.0f64; 3];
        for (i, v) in r.iter().enumerate() {
            if self.fr.map[i] != usize::MAX {
                n[if i < 2 * d.nn { 0 } else if i < 2 * d.nn + d.nv { 1 } else { 2 }] += v * v;
            }
        }
        n.map(f64::sqrt)
    }
    fn scaled(&mut self, r: &[f64], x: &[f64], grow: bool) -> [f64; 3] {
        let d = self.d;
        let mut n = self.norms(r);
        if grow {
            self.umax = x[..2 * d.nn].chunks(2).fold(self.umax, |m, u| m.max(u[0].hypot(u[1])));
            for b in 0..3 {
                self.scale[b] = self.scale[b].max(n[b]);
            }
            // A fluid starting at rest has no continuity residual at all: measure it against the
            // speeds that then appear, not against round-off.
            self.scale[1] = self.scale[1].max(self.umax * self.weight);
        }
        if d.heat {
            // An energy residual at the level of round-off of the absolute temperatures is zero.
            let tmax = x[2 * d.nn + d.nv..].iter().fold(0.0f64, |m, t| m.max(t.abs()));
            if n[2] <= 1e-12 * (self.s.k + self.s.rho * self.s.cp * self.umax * self.len) * tmax * (d.nn as f64).sqrt() {
                n[2] = 0.0;
            }
        }
        [0, 1, 2].map(|b| if self.scale[b] > 0.0 { n[b] / self.scale[b] } else { 0.0 })
    }
    /// Set the fixed unknowns of `x` for a share `lam` of the driving.
    fn fix(&self, x: &mut [f64], lam: f64) {
        let d = self.d;
        for (i, v) in self.s.vel.iter().enumerate() {
            if let Some(v) = v { x[i] = v * lam; }
        }
        if let Some(p) = self.s.pin { x[d.p(p)] = 0.0; }
        if d.heat {
            for (i, t) in self.s.temp.iter().enumerate() {
                if let Some(t) = t { x[d.t(i)] = *t; }
            }
        }
    }

    /// Iterate at one load level until the scaled residual is under `tol`.
    #[allow(clippy::too_many_arguments)]
    fn level(&mut self, x: &mut Vec<f64>, r: &mut Vec<f64>, lam: f64, tol: f64, max_iter: u32, picard: u32, solver: &mut dyn LinearSolver, on: &mut dyn FnMut(&Step) -> bool) -> Result<End, String> {
        let (mesh, s, d) = (self.mesh, self.s, self.d);
        self.fix(x, lam);
        *r = assemble(mesh, s, d, x, Load(lam), false, None);
        let mut force_picard = 0u32;
        let mut stalls = 0u32;
        let mut first = f64::NAN;
        for it in 0..=max_iter {
            let sc = self.scaled(r, x, true);
            let merit = (sc[0] * sc[0] + sc[1] * sc[1] + sc[2] * sc[2]).sqrt();
            if !merit.is_finite() {
                return Ok(End::Failed("the numbers grew without limit".into()));
            }
            if first.is_nan() { first = merit; }
            let newton = it >= picard && force_picard == 0;
            let row = Row { iter: self.iter, kind: if it == 0 || !newton { "Picard" } else { "Newton" }, load: lam, momentum: sc[0], continuity: sc[1], energy: sc[2] };
            self.history.push(row);
            if !on(&Step { row, dofs: d, x, residual: r }) {
                return Ok(End::Stopped);
            }
            if row.worst() <= tol {
                return Ok(End::Converged);
            }
            if it == max_iter {
                break;
            }
            self.iter += 1;
            assemble(mesh, s, d, x, Load(lam), newton, Some((&mut self.jac, &self.fr)));
            let mut b = vec![0.0; self.fr.n];
            for (i, v) in r.iter().enumerate() {
                if self.fr.map[i] != usize::MAX { b[self.fr.map[i]] = -v; }
            }
            let dx = solver.solve(&self.jac, &b)?;
            let step = |x: &[f64], a: f64, fr: &Free| -> Vec<f64> { x.iter().enumerate().map(|(i, v)| if fr.map[i] != usize::MAX { v + a * dx[fr.map[i]] } else { *v }).collect() };
            if !newton {
                force_picard = force_picard.saturating_sub(1);
                *x = step(x, 1.0, &self.fr);
                *r = assemble(mesh, s, d, x, Load(lam), false, None);
                continue;
            }
            // Newton with a backtracking line search on the scaled residual.
            let mut taken = false;
            let mut a = 1.0;
            for _ in 0..6 {
                let xt = step(x, a, &self.fr);
                let rt = assemble(mesh, s, d, &xt, Load(lam), false, None);
                let st = self.scaled(&rt, &xt, false);
                let mt = (st[0] * st[0] + st[1] * st[1] + st[2] * st[2]).sqrt();
                // Near round-off the residual cannot fall further: a full step is as good as it gets.
                if mt.is_finite() && (mt < (1.0 - 1e-4 * a) * merit || (a == 1.0 && merit < 1e-11)) {
                    *x = xt;
                    *r = rt;
                    taken = true;
                    break;
                }
                a *= 0.5;
            }
            if !taken {
                stalls += 1;
                if stalls > 2 {
                    return Ok(End::Failed("Newton's steps stopped making progress".into()));
                }
                force_picard = 2;
            }
        }
        Ok(End::Failed(format!("the residual fell by {:.0e} in {max_iter} iterations but did not reach the tolerance", (self.history.last().map(|h| h.worst()).unwrap_or(1.0) / first.max(1e-300)).clamp(1e-99, 1e99))))
    }
}

/// Solve the problem on the mesh. `start` continues from an earlier answer (resume, or a nearby
/// case). `on` is called after every iteration and stops the solve by returning false.
pub fn solve(mesh: &Mesh, s: &Setup, opt: &SolverSettings, start: Option<&State>, solver: &mut dyn LinearSolver, on: &mut dyn FnMut(&Step) -> bool) -> Result<Solved, String> {
    let t0 = std::time::Instant::now();
    let d = Dofs::of(mesh, s.heat);
    if s.vel.len() != 2 * d.nn || s.temp.len() != d.nn {
        return Err("The mesh has changed since the boundaries were set up; mesh again.".into());
    }
    let fr = free(d, s);
    let jac = pattern(mesh, d, &fr);
    let mut stats = Stats { unknowns: fr.n, nonzeros: jac.nnz(), linear_solver: solver.name(), ..Default::default() };
    let mut lumped = vec![0.0f64; d.nv];
    for t in &mesh.tris {
        let p = [mesh.nodes[t[0] as usize], mesh.nodes[t[1] as usize], mesh.nodes[t[2] as usize]];
        let a = 0.5 * ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1])).abs();
        for k in 0..3 { lumped[t[k] as usize] += a / 3.0; }
    }
    let len = lumped.iter().sum::<f64>().sqrt().max(1e-300);
    let weight = lumped.iter().map(|a| a * a).sum::<f64>().sqrt() / len;
    let mut run = Run { mesh, s, d, fr, jac, scale: [0.0; 3], weight, len, umax: 0.0, history: vec![], iter: 0 };
    // The cold start: fluid at rest at the reference temperature. Its residual at full driving
    // sets the scale every later residual is measured against.
    let mut cold = vec![0.0; d.n()];
    if d.heat {
        for i in 0..d.nn { cold[d.t(i)] = s.t_ref; }
    }
    run.fix(&mut cold, 1.0);
    let r_cold = assemble(mesh, s, d, &cold, Load(1.0), false, None);
    run.scaled(&r_cold, &cold, true);
    if run.scale.iter().all(|v| *v == 0.0) {
        return Err("Nothing drives the flow: the fluid at rest already satisfies every equation.".into());
    }
    let mut x = match start {
        Some(st) if st.u.len() == d.nn && st.p.len() == d.nv && (!d.heat || st.t.len() == d.nn) => st.to_vec(d),
        _ => cold.clone(),
    };
    let mut r = vec![];
    let finish = |run: Run, x: Vec<f64>, r: Vec<f64>, converged: bool, stopped: bool, message: String, mut stats: Stats| {
        stats.seconds = t0.elapsed().as_secs_f64();
        Solved { state: State::from_vec(d, &x), x, residual: r, dofs: d, converged, stopped, history: run.history, message, stats }
    };
    let mut why = String::new();
    if opt.continuation != Continuation::Always {
        match run.level(&mut x, &mut r, 1.0, opt.tol, opt.max_iter, opt.picard, solver, on) {
            Ok(End::Converged) => { let n = run.iter; return Ok(finish(run, x, r, true, false, format!("Converged in {n} iterations."), stats)); }
            Ok(End::Stopped) => return Ok(finish(run, x, r, false, true, "Stopped before it converged. Solve again to carry on from here.".into(), stats)),
            Ok(End::Failed(e)) | Err(e) => why = e,
        }
        if opt.continuation == Continuation::Off {
            return Ok(finish(run, x, r, false, false, format!("Did not converge: {why}."), stats));
        }
    }
    // Continuation: from a tenth of the driving up to all of it, each level starting from the last.
    let mut good = cold;
    let (mut lam_good, mut lam, mut factor) = (0.0f64, 0.1f64, 2.0f64);
    for _ in 0..40 {
        stats.steps += 1;
        let mut xt = good.clone();
        let last = lam >= 1.0;
        let tol = if last { opt.tol } else { opt.tol.max(1e-5) };
        match run.level(&mut xt, &mut r, lam, tol, opt.max_iter, opt.picard.max(2), solver, on) {
            Ok(End::Converged) => {
                good = xt;
                lam_good = lam;
                if last {
                    let n = run.iter;
                    let steps = stats.steps;
                    return Ok(finish(run, good, r, true, false, format!("Converged in {n} iterations, raising the flow in {steps} steps."), stats));
                }
                lam = (lam * factor).min(1.0);
                factor = (factor * 1.25).min(4.0);
            }
            Ok(End::Stopped) => return Ok(finish(run, xt, r, false, true, "Stopped before it converged. Solve again to carry on from here.".into(), stats)),
            Ok(End::Failed(e)) | Err(e) => {
                why = e;
                // Half the step (in ratio), from the last level that worked.
                let next = if lam_good > 0.0 { (lam_good * lam).sqrt() } else { lam * 0.25 };
                factor = 1.0 + (factor - 1.0) * 0.5;
                if (lam_good > 0.0 && next / lam_good < 1.03) || next < 1e-4 {
                    break;
                }
                lam = next;
            }
        }
    }
    // Leave the best answer found (a gentler flow) so the user can see where it struggles.
    let mut x = good;
    run.fix(&mut x, lam_good.max(0.0));
    let r = assemble(mesh, s, d, &x, Load(lam_good), false, None);
    let reached = if lam_good > 0.0 { format!(" The view shows the flow at {:.0}% of the speed or heating asked for.", lam_good * 100.0) } else { String::new() };
    Ok(finish(run, x, r, false, false, format!("Did not converge: {why}.{reached}"), stats))
}
