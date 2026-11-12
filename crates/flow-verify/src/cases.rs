//! The verification cases. Each returns rows of the report; `run` gathers them. Reference values
//! are keyed in from the cited papers; `confirmed` says how each was checked.

use crate::gci::gci;
use crate::report::Check;
use crate::run::{refined, solve_model, Run};
use flow_core::examples;
use flow_core::model::*;
use flow_mesh::Mesh;
use flow_post::values::sample;
use flow_solve::fe::{geo, QP};
use flow_solve::{solve, Setup, SparseLu, State};
use std::f64::consts::PI;
use std::sync::Arc;

const SECONDARY: &str = "secondary sources";

#[allow(clippy::too_many_arguments)]
fn row(tier: u8, case: &str, quantity: &str, reference: f64, result: f64, tolerance: f64, absolute: bool, source: &str, confirmed: &str, note: &str) -> Check {
    let error = if absolute { (result - reference).abs() } else { (result - reference).abs() / reference.abs().max(1e-300) };
    Check { tier, case: case.into(), quantity: quantity.into(), reference, result, error, tolerance, absolute, pass: error.is_finite() && error <= tolerance, source: source.into(), confirmed: confirmed.into(), note: note.into(), steps: vec![] }
}
fn failed(tier: u8, case: &str, why: String) -> Vec<Check> {
    vec![Check { tier, case: case.into(), quantity: "The case ran".into(), reference: 1.0, result: 0.0, error: 1.0, tolerance: 0.0, absolute: true, pass: false, source: String::new(), confirmed: String::new(), note: why, steps: vec![] }]
}
fn tight() -> SolverSettings {
    SolverSettings { tol: 1e-12, max_iter: 60, ..Default::default() }
}

/// L2 norms of the errors of velocity, pressure (after removing the difference of the means,
/// since a closed flow fixes pressure only up to a constant) and temperature.
fn l2_errors(mesh: &Mesh, s: &State, u: &dyn Fn([f64; 2]) -> [f64; 2], p: &dyn Fn([f64; 2]) -> f64, t: Option<&dyn Fn([f64; 2]) -> f64>) -> [f64; 3] {
    let (mut eu, mut et, mut area, mut dp_mean, mut pts) = (0.0, 0.0, 0.0, 0.0, vec![]);
    for tri in &mesh.tris {
        let xy = tri.map(|n| mesh.nodes[n as usize]);
        for q in QP {
            let g = geo(&xy, q.0, q.1);
            let w = g.det * q.2;
            let (mut uh, mut ph, mut th) = ([0.0; 2], 0.0, 0.0);
            for a in 0..6 {
                uh[0] += g.n[a] * s.u[tri[a] as usize][0];
                uh[1] += g.n[a] * s.u[tri[a] as usize][1];
                if !s.t.is_empty() { th += g.n[a] * s.t[tri[a] as usize]; }
            }
            for b in 0..3 { ph += g.l[b] * s.p[tri[b] as usize]; }
            let ue = u(g.x);
            eu += w * ((uh[0] - ue[0]).powi(2) + (uh[1] - ue[1]).powi(2));
            if let Some(t) = t { et += w * (th - t(g.x)).powi(2); }
            let d = ph - p(g.x);
            dp_mean += w * d;
            area += w;
            pts.push((w, d));
        }
    }
    let mean = dp_mean / area;
    let ep: f64 = pts.iter().map(|(w, d)| w * (d - mean).powi(2)).sum();
    [eu.sqrt(), ep.sqrt(), et.sqrt()]
}

// ------------------------------------------------------------------ tier 1: exact solutions

/// Couette flow: one plate slides over another; the velocity is a straight line. In the
/// element's space, so the answer is exact to round-off at any Reynolds number.
pub fn couette() -> Vec<Check> {
    let mesh = Mesh::grid(0.0, 0.0, 2.0, 1.0, 8, 4, [1, 2, 3, 4]);
    let mut s = Setup::new(&mesh, 1.0, 0.01);
    s.enclose(&mesh, |x| [x[1], 0.0]);
    let r = match solve(&mesh, &s, &tight(), None, &mut SparseLu::default(), &mut |_| true) {
        Ok(r) if r.converged => r,
        Ok(r) => return failed(1, "Couette flow", r.message),
        Err(e) => return failed(1, "Couette flow", e),
    };
    let err = mesh.nodes.iter().zip(&r.state.u).map(|(x, u)| (u[0] - x[1]).abs().max(u[1].abs())).fold(0.0, f64::max);
    let dp = r.state.p.iter().fold(0.0f64, |m, p| m.max(p.abs()));
    vec![
        row(1, "Couette flow", "Largest error of the velocity, over the plate speed", 0.0, err, 1e-8, true, "exact (u = U y / H)", "exact", "Re 100 on the gap; a regular mesh of 64 elements."),
        row(1, "Couette flow", "Largest pressure (it should be uniform), over ρU²", 0.0, dp, 1e-8, true, "exact (p = constant)", "exact", ""),
    ]
}

/// Plane Poiseuille flow through the Channel example: parabolic inlet, pressure outlet, automatic
/// mesh. The parabola and the linear pressure are in the element's space.
pub fn poiseuille() -> Vec<Check> {
    let mut m = examples::channel();
    m.solver = tight();
    let r = match solve_model(&m) { Ok(r) => r, Err(e) => return failed(1, "Plane Poiseuille flow", e) };
    let (h, l, um) = (0.01, 0.1, 0.05);
    let err = r.mesh.nodes.iter().zip(&r.solved.state.u).map(|(x, u)| (u[0] - 6.0 * um * (x[1] / h) * (1.0 - x[1] / h)).abs().max(u[1].abs())).fold(0.0, f64::max) / um;
    let v = r.values();
    let dp = 12.0 * m.fluid.mu * um * l / (h * h);
    let (net, inflow) = v.balance();
    let src = "exact (u = 6 U (y/H)(1 − y/H), Δp = 12 μ U L / H²)";
    vec![
        row(1, "Plane Poiseuille flow", "Largest error of the velocity, over the mean speed", 0.0, err, 1e-8, true, src, "exact", "The Channel example: water, Re 500, automatic mesh, fully developed inlet, pressure outlet."),
        row(1, "Plane Poiseuille flow", "Pressure drop inlet to outlet (Pa)", dp, v.mean_pressure(2) - v.mean_pressure(3), 1e-8, false, src, "exact", ""),
        row(1, "Plane Poiseuille flow", "Friction force on the plates (N per metre depth)", dp * h, v.drag_lift(4)[0], 1e-8, false, "exact (it balances the pressure drop: Δp H)", "exact", "From the reactions at the wall nodes."),
        row(1, "Plane Poiseuille flow", "Net flow through all boundaries, over the flow in", 0.0, (net / inflow).abs(), 1e-10, true, "exact (mass is conserved)", "exact", ""),
    ]
}

/// Pure conduction between a warm and a cool wall, the fluid at rest: the temperature is a
/// straight line and the Nusselt number is exactly 1.
pub fn conduction() -> Vec<Check> {
    let mut m = examples::heated_cavity(1e4);
    m.physics.buoyancy = false;
    m.solver = tight();
    let r = match solve_model(&m) { Ok(r) => r, Err(e) => return failed(1, "Conduction between two walls", e) };
    let t0 = m.physics.t_ref;
    let err = r.mesh.nodes.iter().zip(&r.solved.state.t).map(|(x, t)| (t - (t0 + 0.5 - x[0])).abs()).fold(0.0, f64::max);
    vec![
        row(1, "Conduction between two walls", "Largest error of the temperature, over the temperature difference", 0.0, err, 1e-8, true, "exact (T falls in a straight line)", "exact", "The Heated cavity example with buoyancy off."),
        row(1, "Conduction between two walls", "Nusselt number on the warm wall", 1.0, r.values().nusselt(2), 1e-8, false, "exact (Nu = 1 without flow)", "exact", "From the reactions of the energy equation."),
    ]
}

// ------------------------------------------------------------------ tier 2: order of accuracy

/// The observed order of accuracy from errors on meshes each half the size of the last: the
/// order between the two finest, with every mesh's error in the row's steps.
fn order_rows(case: &str, what: [&str; 3], errors: &[(f64, [f64; 3])], formal: [f64; 3], source: &str, note: &str) -> Vec<Check> {
    let n = errors.len();
    (0..3).filter(|k| errors[n - 1].1[*k] > 0.0).map(|k| {
        let order = (errors[n - 2].1[k] / errors[n - 1].1[k]).ln() / (errors[n - 2].0 / errors[n - 1].0).ln();
        let mut c = row(2, case, &format!("Observed order of accuracy: {}", what[k]), formal[k], order, 0.1, true, source, "exact", note);
        c.steps = errors.iter().map(|(h, e)| [*h, e[k]]).collect();
        c
    }).collect()
}

/// A manufactured solution (Salari & Knupp 2000) with flow, heat and buoyancy together:
/// u = π sin πx cos πy, v = −π cos πx sin πy, p = cos πx sin πy, T = T₀ + cos πx cos πy on the
/// unit square, with the body force and heat source that make it satisfy the equations.
pub fn manufactured(grids: &[usize]) -> Vec<Check> {
    let (rho, mu, k, cp, beta, g, t0) = (1.3, 0.05, 0.4, 2.0, 0.6, [0.3, -1.1], 300.0);
    let u = |x: [f64; 2]| [PI * (PI * x[0]).sin() * (PI * x[1]).cos(), -PI * (PI * x[0]).cos() * (PI * x[1]).sin()];
    let p = |x: [f64; 2]| (PI * x[0]).cos() * (PI * x[1]).sin();
    let t = move |x: [f64; 2]| t0 + (PI * x[0]).cos() * (PI * x[1]).cos();
    let force = move |x: [f64; 2]| {
        let (sx, cx, sy, cy) = ((PI * x[0]).sin(), (PI * x[0]).cos(), (PI * x[1]).sin(), (PI * x[1]).cos());
        let ue = [PI * sx * cy, -PI * cx * sy];
        let dt = cx * cy;
        // ρ (u·∇)u − μ ∇²u + ∇p + ρ β (T − T₀) g
        [rho * PI.powi(3) * sx * cx + 2.0 * PI * PI * mu * ue[0] - PI * sx * sy + rho * beta * dt * g[0],
         rho * PI.powi(3) * sy * cy + 2.0 * PI * PI * mu * ue[1] + PI * cx * cy + rho * beta * dt * g[1]]
    };
    let source = move |x: [f64; 2]| {
        let (sx, cx, sy, cy) = ((PI * x[0]).sin(), (PI * x[0]).cos(), (PI * x[1]).sin(), (PI * x[1]).cos());
        // ρ cp u·∇T − k ∇²T
        rho * cp * PI * PI * (cx * cx * sy * sy - sx * sx * cy * cy) + 2.0 * PI * PI * k * cx * cy
    };
    let mut errors = vec![];
    for &n in grids {
        let mesh = Mesh::grid(0.0, 0.0, 1.0, 1.0, n, n, [1, 2, 3, 4]);
        let mut s = Setup::new(&mesh, rho, mu);
        (s.k, s.cp, s.beta, s.g, s.t_ref, s.heat, s.buoyancy) = (k, cp, beta, g, t0, true, true);
        s.enclose(&mesh, u);
        for e in &mesh.edges {
            for nd in e.n { s.temp[nd as usize] = Some(t(mesh.nodes[nd as usize])); }
        }
        s.force = Some(Arc::new(force));
        s.source = Some(Arc::new(source));
        match solve(&mesh, &s, &tight(), None, &mut SparseLu::default(), &mut |_| true) {
            Ok(r) if r.converged => errors.push((1.0 / n as f64, l2_errors(&mesh, &r.state, &u, &p, Some(&t)))),
            Ok(r) => return failed(2, "Manufactured solution", r.message),
            Err(e) => return failed(2, "Manufactured solution", e),
        }
    }
    order_rows("Manufactured solution: flow, heat and buoyancy", ["velocity (L2 norm)", "pressure (L2 norm)", "temperature (L2 norm)"], &errors, [3.0, 2.0, 3.0],
        "Method of manufactured solutions (Salari & Knupp 2000); formal orders 3, 2, 3", "Regular meshes of the unit square, each half the size of the last; the order is taken from the two finest.")
}

/// Kovasznay's exact solution of the Navier–Stokes equations (1948), the flow behind a row of
/// bars, at Re 40. It is exponential, so no element reproduces it: its error must fall at the
/// formal rate.
pub fn kovasznay(grids: &[usize]) -> Vec<Check> {
    let re = 40.0f64;
    let lam = re / 2.0 - (re * re / 4.0 + 4.0 * PI * PI).sqrt();
    let u = move |x: [f64; 2]| [1.0 - (lam * x[0]).exp() * (2.0 * PI * x[1]).cos(), lam / (2.0 * PI) * (lam * x[0]).exp() * (2.0 * PI * x[1]).sin()];
    let p = move |x: [f64; 2]| 0.5 * (1.0 - (2.0 * lam * x[0]).exp());
    let mut errors = vec![];
    for &n in grids {
        let mesh = Mesh::grid(-0.5, -0.5, 1.0, 1.5, 3 * n, 4 * n, [1, 2, 3, 4]);
        let mut s = Setup::new(&mesh, 1.0, 1.0 / re);
        s.enclose(&mesh, u);
        match solve(&mesh, &s, &tight(), None, &mut SparseLu::default(), &mut |_| true) {
            Ok(r) if r.converged => errors.push((0.5 / n as f64, l2_errors(&mesh, &r.state, &u, &p, None))),
            Ok(r) => return failed(2, "Kovasznay flow", r.message),
            Err(e) => return failed(2, "Kovasznay flow", e),
        }
    }
    let mut rows = order_rows("Kovasznay flow, Re 40", ["velocity (L2 norm)", "pressure (L2 norm)", ""], &errors, [3.0, 2.0, 0.0], "Exact solution of Kovasznay (1948); formal orders 3 and 2",
        "An exact solution of the full equations. It is exponential, so it is checked by its rate of convergence rather than at round-off.");
    let fine = errors[errors.len() - 1].1[0];
    rows.push(row(1, "Kovasznay flow, Re 40", "Velocity error (L2 norm) on the finest mesh", 0.0, fine, 1e-4, true, "Exact solution of Kovasznay (1948)", SECONDARY, "Exact, but not a polynomial: the error is small, not round-off. The formula was checked for consistency (it satisfies continuity) and against later papers that quote it."));
    rows
}

// ------------------------------------------------------------------ tier 3: benchmarks

/// A quantity on three meshes, fine to coarse, as a benchmark row. The error must be within
/// twice the Grid Convergence Index of the fine mesh (Celik et al. 2008), or within the
/// reference's own stated accuracy when that is larger. When the three values do not fall
/// steadily the order cannot be established, and the band is three times the largest change
/// between meshes (the safety factor recommended when the order is unknown).
#[allow(clippy::too_many_arguments)]
fn benchmark(case: &str, quantity: &str, reference: f64, ref_accuracy: f64, formal: f64, h: [f64; 3], phi: [f64; 3], source: &str, extra: &str) -> Check {
    let g = gci(h, phi, formal);
    let band = if g.oscillating { 3.0 * (phi[1] - phi[0]).abs().max((phi[2] - phi[1]).abs()) / phi[0].abs().max(1e-300) } else { g.gci };
    let tol = (2.0 * band).max(ref_accuracy);
    let how = if g.oscillating {
        format!("The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: {:.2e}.", band)
    } else {
        format!("Grid Convergence Index of the fine mesh {:.2e} (apparent order {:.2}, extrapolated value {}).", band, g.order, flow_core::units::num(g.extrapolated, 7))
    };
    let which = if 2.0 * band >= ref_accuracy { "Tolerance: twice that band.".to_string() } else { format!("Tolerance: the reference's own accuracy ({:.1}%), which is wider than twice that band.", ref_accuracy * 100.0) };
    let mut c = row(3, case, quantity, reference, phi[0], tol, false, source, SECONDARY, &format!("{how} {which} {extra}"));
    c.steps = (0..3).rev().map(|i| [h[i], phi[i]]).collect();
    c
}

/// Solve a model on three meshes (sizes ÷ factors, coarse to fine given); returns fine to coarse.
fn three(m: &Model, factors: [f64; 3]) -> Result<[Run; 3], String> {
    let mut runs = vec![];
    for f in factors.iter().rev() {
        let mut mm = refined(m, *f);
        mm.solver.tol = 1e-10;
        runs.push(solve_model(&mm)?);
    }
    runs.try_into().map_err(|_| "three meshes".to_string())
}

/// Schäfer & Turek (1996), case 2D-1: steady flow past a cylinder at Re 20.
pub fn cylinder() -> Vec<Check> {
    let case = "Cylinder in a channel, Re 20";
    let runs = match three(&examples::cylinder(), [0.525, 0.7875, 1.18125]) { Ok(r) => r, Err(e) => return failed(3, case, e) };
    let h = [runs[0].h(), runs[1].h(), runs[2].h()];
    let get = |f: &dyn Fn(&Run) -> f64| [f(&runs[0]), f(&runs[1]), f(&runs[2])];
    let src = "Schäfer & Turek (1996), case 2D-1; reference values from the benchmark's results page";
    let (net, inflow) = runs[0].values().balance();
    vec![
        benchmark(case, "Drag coefficient", 5.57953523384, 0.0, 3.0, h, get(&|r| r.values().drag_lift(6)[0] * 500.0), src, "Coefficient = 2 F / (ρ U² D) with the mean inlet speed."),
        benchmark(case, "Lift coefficient", 0.010618948146, 0.0, 3.0, h, get(&|r| r.values().drag_lift(6)[1] * 500.0), src, ""),
        benchmark(case, "Pressure difference, front to back of the cylinder (Pa)", 0.11752016697, 0.0, 2.0, h, get(&|r| { let l = r.locator(); sample(&r.mesh, &r.solved.state, &l, [0.15, 0.2]).pressure - sample(&r.mesh, &r.solved.state, &l, [0.25, 0.2]).pressure }), src, ""),
        row(3, case, "Net flow through all boundaries, over the flow in", 0.0, (net / inflow).abs(), 1e-10, true, "exact (mass is conserved)", "exact", "On the fine mesh."),
    ]
}

/// Botella & Peyret (1998): the lid-driven cavity at Re 1000, by a spectral method.
pub fn cavity() -> Vec<Check> {
    let case = "Lid-driven cavity, Re 1000";
    let runs = match three(&examples::cavity(1000.0), [0.6, 0.9, 1.35]) { Ok(r) => r, Err(e) => return failed(3, case, e) };
    let h = [runs[0].h(), runs[1].h(), runs[2].h()];
    let psi: Vec<Vec<f64>> = match runs.iter().map(|r| flow_post::stream_function(&r.mesh, &r.solved.state, &mut SparseLu::default())).collect() { Ok(p) => p, Err(e) => return failed(3, case, e) };
    let ext: Vec<(f64, [f64; 2])> = (0..3).map(|i| flow_post::fields::extremum(&runs[i].mesh, &psi[i], true)).collect();
    let src = "Botella & Peyret (1998), spectral solution";
    let at = |x: f64, y: f64, c: usize| -> [f64; 3] { [0, 1, 2].map(|i| { let s = sample(&runs[i].mesh, &runs[i].solved.state, &runs[i].locator(), [x, y]); [s.u, s.v][c] }) };
    let lid = "Their lid moves in −x; values are mirrored here for a lid moving in +x.";
    let mut rows = vec![
        benchmark(case, "Stream function at the centre of the main eddy", -0.1189366, 0.0, 3.0, h, [ext[0].0, ext[1].0, ext[2].0], src, lid),
        row(3, case, "Main eddy centre, x", 0.5308, ext[0].1[0], 0.002, true, src, SECONDARY, "Position on the fine mesh; tolerance 0.002 of the side (the reference gives four figures; the stream function is flat near its minimum)."),
        row(3, case, "Main eddy centre, y", 0.5652, ext[0].1[1], 0.002, true, src, SECONDARY, ""),
    ];
    // Velocity on the centre lines, from their Tables 9 and 10.
    for (y, u) in [(0.9531, 0.4723329), (0.5, -0.0620561), (0.1719, -0.3885691)] {
        rows.push(benchmark(case, &format!("u on the vertical centre line at y = {y}"), u, 0.0, 3.0, h, at(0.5, y, 0), src, lid));
    }
    for (x, v) in [(0.9063, -0.5264392), (0.5, 0.0257995), (0.1563, 0.3769189)] {
        rows.push(benchmark(case, &format!("v on the horizontal centre line at x = {x}"), v, 0.0, 3.0, h, at(x, 0.5, 1), src, lid));
    }
    rows
}

/// de Vahl Davis (1983): natural convection of air in a square cavity.
pub fn heated(ra: f64, nu: f64, factors: [f64; 3]) -> Vec<Check> {
    let case = format!("Heated cavity, Ra 1e{:.0}", ra.log10());
    let runs = match three(&examples::heated_cavity(ra), factors) { Ok(r) => r, Err(e) => return failed(3, &case, e) };
    let h = [runs[0].h(), runs[1].h(), runs[2].h()];
    let v = runs[0].values();
    let (hot, cold) = (v.nusselt(2), v.nusselt(3));
    vec![
        benchmark(&case, "Average Nusselt number", nu, 0.01, 3.0, h, [hot, runs[1].values().nusselt(2), runs[2].values().nusselt(2)], "de Vahl Davis (1983), benchmark solution",
            "The reference is his average over the cavity; here it is the heat through the warm wall, which is the same number for a steady flow. He states his solution is accurate to better than 1% at the highest Rayleigh number."),
        row(3, &case, "Heat out through the cool wall, over heat in through the warm wall", 1.0, cold / hot, 1e-4, false, "exact (energy is conserved)", "exact", "On the fine mesh."),
    ]
}

// ------------------------------------------------------------------ tier 4: validation

/// Where the flow reattaches behind the step, in step heights: the place on the floor where the
/// flow just above it turns from backwards to forwards.
pub fn reattachment(r: &Run) -> f64 {
    let s = 0.0049;
    let loc = r.locator();
    let u = |x: f64| sample(&r.mesh, &r.solved.state, &loc, [x, s * 0.01]).u;
    let n = 3000;
    let mut last = 0.0;
    for i in 1..n {
        let (a, b) = (30.0 * s * (i - 1) as f64 / n as f64 + 1e-6, 30.0 * s * i as f64 / n as f64);
        if u(a) < 0.0 && u(b) >= 0.0 {
            last = a + (b - a) * (-u(a)) / (u(b) - u(a));
            break;
        }
    }
    last / s
}

/// Backward-facing step: reattachment lengths at the Reynolds numbers of Armaly et al. (1983).
/// Returned as text: the measured values could not be read from the paper, so there is no verdict.
pub fn step_lengths() -> String {
    let mut out = vec![];
    for re in [100.0, 200.0, 389.0] {
        let mut m = examples::step(re);
        m.solver.continuation = Continuation::Auto;
        match solve_model(&m) {
            Ok(r) => out.push(format!("{:.2} step heights at Re {re:.0}", reattachment(&r))),
            Err(e) => out.push(format!("no answer at Re {re:.0} ({e})")),
        }
    }
    out.join(", ")
}

// ------------------------------------------------------------------ the suite

/// The cases quick enough for every `cargo test`: exact solutions, orders of accuracy, the cylinder.
pub fn quick() -> Vec<Check> {
    let mut c = vec![];
    c.extend(couette());
    c.extend(poiseuille());
    c.extend(conduction());
    c.extend(kovasznay(&[4, 8, 16]));
    c.extend(manufactured(&[8, 16, 32]));
    c.extend(cylinder());
    c
}

/// Every case (minutes).
pub fn full() -> Vec<Check> {
    let mut c = quick();
    c.extend(cavity());
    c.extend(heated(1e3, 1.118, [0.6, 0.9, 1.35]));
    c.extend(heated(1e4, 2.243, [0.6, 0.9, 1.35]));
    c.extend(heated(1e5, 4.519, [0.6, 0.9, 1.35]));
    c.extend(heated(1e6, 8.800, [0.6, 0.9, 1.35]));
    c.sort_by_key(|x| x.tier);
    c
}
