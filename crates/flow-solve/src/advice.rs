//! Plain advice when a solve does not converge: what to try, most likely first.

use flow_core::model::*;
use flow_core::numbers::numbers;
use flow_mesh::Mesh;

pub fn advice(m: &Model, mesh: &Mesh) -> Vec<String> {
    let n = numbers(m);
    let mut out = vec![];
    let cell = m.fluid.rho * n.speed * mesh.quality.edges[1] / m.fluid.mu;
    if let Some(body) = m.shapes.iter().find(|s| s.hole) {
        if cell > 20.0 || m.mesh.wall >= 1.0 {
            out.push(format!("Try a finer mesh near the {}: add a local size on its edge (Mesh › Local Size).", body.name.to_lowercase()));
        }
    }
    if cell > 40.0 {
        out.push(format!("The mesh is coarse for this speed (about {cell:.0} for the Reynolds number of one element): use a smaller element size, or more refinement at walls."));
    }
    if m.solver.continuation != Continuation::Always && (n.re > 200.0 || n.ra > 1e5) {
        out.push(if n.ra > 1e5 && n.re <= 200.0 { "Raise Ra in steps: set Solve › Continuation to Always.".into() } else { "Raise Re in steps: set Solve › Continuation to Always.".to_string() });
    }
    if n.ra > 1e5 && m.mesh.wall > 0.35 {
        out.push("Refine the mesh at the heated and cooled walls (Mesh › Wall refinement): the boundary layers there are thin.".into());
    }
    if n.ra > 1e6 && m.solver.grad_div == 0.0 {
        out.push("Turn on grad-div stabilisation (Solve › Settings) for strong natural convection.".into());
    }
    if n.turbulent().is_some() || (n.through && n.re > 2300.0) || n.ra > 1e8 {
        out.push("This flow may not have a steady laminar answer at all: it is at or beyond the start of turbulence.".into());
    }
    if m.solver.max_iter < 30 {
        out.push("Allow more iterations (Solve › Settings).".into());
    }
    if out.is_empty() {
        out.push("Try a finer mesh, then set Solve › Continuation to Always.".into());
    }
    out
}
