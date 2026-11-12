//! First checks of the solver against exact answers (the full set is in flow-verify).

use crate::*;
use flow_core::examples;
use flow_core::model::SolverSettings;
use flow_mesh::Mesh;

fn quiet() -> impl FnMut(&Step) -> bool { |_| true }

#[test]
fn stokes_poiseuille_is_exact() {
    // Between plates at y = 0 and 1, u = 4 y (1 − y), p = 8 μ (L − x): quadratic velocity and
    // linear pressure are in the element's space, so the answer is exact to round-off.
    let mesh = Mesh::grid(0.0, 0.0, 2.0, 1.0, 6, 4, [1, 2, 3, 4]);
    let mu = 0.5;
    let mut s = Setup::new(&mesh, 1.0, mu);
    for e in &mesh.edges {
        if e.edge == 1 {
            s.outlets.push((mesh.edges.iter().position(|x| std::ptr::eq(x, e)).unwrap(), 0.0));
            continue;
        }
        for n in e.n {
            let y = mesh.nodes[n as usize][1];
            s.vel[2 * n as usize] = Some(4.0 * y * (1.0 - y));
            s.vel[2 * n as usize + 1] = Some(0.0);
        }
    }
    let r = solve(&mesh, &s, &SolverSettings::default(), None, &mut SparseLu::default(), &mut quiet()).unwrap();
    assert!(r.converged, "{}", r.message);
    for (i, p) in mesh.nodes.iter().enumerate() {
        assert!((r.state.u[i][0] - 4.0 * p[1] * (1.0 - p[1])).abs() < 1e-11 && r.state.u[i][1].abs() < 1e-11, "u at {p:?}: {:?}", r.state.u[i]);
        if i < mesh.nv {
            assert!((r.state.p[i] - 8.0 * mu * (2.0 - p[0])).abs() < 1e-10, "p at {p:?}: {}", r.state.p[i]);
        }
    }
    assert!(r.history.len() <= 6 && r.stats.unknowns > 0, "{:?}", r.history);
}

#[test]
fn the_cavity_example_converges_with_newton_in_a_few_steps() {
    let m = examples::cavity(100.0);
    let mesh = flow_mesh::mesh(&m).unwrap();
    let s = Setup::from_model(&m, &mesh).unwrap();
    let mut rows = vec![];
    let r = solve(&mesh, &s, &m.solver, None, &mut SparseLu::default(), &mut |st: &Step| { rows.push(st.row); true }).unwrap();
    assert!(r.converged && rows.len() == r.history.len(), "{}", r.message);
    // Three Picard steps, then Newton converges quadratically: a handful of steps in all.
    let newton = r.history.iter().filter(|h| h.kind == "Newton").count();
    assert!(newton <= 6 && r.history.len() <= 10, "{:?}", r.history);
    assert!(r.history.last().unwrap().worst() < 1e-8);
    // The main eddy turns with the lid: flow to the left along the bottom half of the centre line.
    let near = |x: f64, y: f64| (0..mesh.nodes.len()).min_by(|a, b| (mesh.nodes[*a][0] - x).hypot(mesh.nodes[*a][1] - y).total_cmp(&(mesh.nodes[*b][0] - x).hypot(mesh.nodes[*b][1] - y))).unwrap();
    assert!(r.state.u[near(0.5, 0.3)][0] < -0.1 && r.state.u[near(0.5, 0.95)][0] > 0.4);
    // Resuming from the answer converges at once.
    let again = solve(&mesh, &s, &m.solver, Some(&r.state), &mut SparseLu::default(), &mut quiet()).unwrap();
    assert!(again.converged && again.history.len() == 1, "{:?}", again.history);
    // Stopping at the second iteration leaves a usable state and says so.
    let mut n = 0;
    let stopped = solve(&mesh, &s, &m.solver, None, &mut SparseLu::default(), &mut |_| { n += 1; n < 2 }).unwrap();
    assert!(stopped.stopped && !stopped.converged && stopped.message.contains("Stopped") && stopped.state.u.len() == mesh.nodes.len());
}
