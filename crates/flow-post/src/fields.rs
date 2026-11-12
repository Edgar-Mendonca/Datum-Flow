//! Fields at the mesh nodes, ready to draw: one number per node (corner and midside).

use flow_mesh::Mesh;
use flow_solve::fe::{geo, NODES_REF, QP};
use flow_solve::{Csc, LinearSolver, State};

/// The fields the window can colour by: (id, name, kind of unit).
pub const FIELDS: [(&str, &str, &str); 7] = [
    ("speed", "Speed", "speed"),
    ("pressure", "Pressure", "pressure"),
    ("temperature", "Temperature", "temp"),
    ("vorticity", "Vorticity", "vorticity"),
    ("u", "Velocity x", "speed"),
    ("v", "Velocity y", "speed"),
    ("stream", "Stream function", "stream"),
];

/// Pressure at every node: the corner values, and their mean at midside nodes.
pub fn pressure(mesh: &Mesh, s: &State) -> Vec<f64> {
    let mut p = vec![0.0; mesh.nodes.len()];
    p[..mesh.nv].copy_from_slice(&s.p);
    for t in &mesh.tris {
        for j in 0..3 {
            p[t[3 + j] as usize] = 0.5 * (s.p[t[j] as usize] + s.p[t[(j + 1) % 3] as usize]);
        }
    }
    p
}

/// Vorticity ∂v/∂x − ∂u/∂y at every node: each element's value at the node, averaged.
pub fn vorticity(mesh: &Mesh, s: &State) -> Vec<f64> {
    let mut w = vec![0.0; mesh.nodes.len()];
    let mut count = vec![0u32; mesh.nodes.len()];
    for t in &mesh.tris {
        let xy = t.map(|n| mesh.nodes[n as usize]);
        for (a, r) in NODES_REF.iter().enumerate() {
            let g = geo(&xy, r.0, r.1);
            let mut om = 0.0;
            for b in 0..6 {
                let u = s.u[t[b] as usize];
                om += g.dn[b][0] * u[1] - g.dn[b][1] * u[0];
            }
            w[t[a] as usize] += om;
            count[t[a] as usize] += 1;
        }
    }
    for (v, c) in w.iter_mut().zip(count) {
        if c > 0 { *v /= c as f64; }
    }
    w
}

/// The stream function ψ at every node (u = ∂ψ/∂y, v = −∂ψ/∂x): its level lines are the
/// streamlines, and the difference between two lines is the flow passing between them. Found as
/// the function whose gradient best matches (−v, u); zero on the first fixed wall.
pub fn stream_function(mesh: &Mesh, s: &State, solver: &mut dyn LinearSolver) -> Result<Vec<f64>, String> {
    let nn = mesh.nodes.len();
    if nn < 2 {
        return Ok(vec![0.0; nn]);
    }
    // Node 0 is held at zero; the others are unknown.
    let mut adj: Vec<Vec<usize>> = vec![vec![]; nn];
    for t in &mesh.tris {
        for a in t {
            adj[*a as usize].extend(t.iter().filter(|b| **b != 0).map(|b| *b as usize - 1));
        }
    }
    for a in adj.iter_mut() {
        a.sort_unstable();
        a.dedup();
    }
    let mut k = Csc::from_columns(adj.into_iter().skip(1).collect());
    let mut rhs = vec![0.0; nn - 1];
    for t in &mesh.tris {
        let xy = t.map(|n| mesh.nodes[n as usize]);
        for q in QP {
            let g = geo(&xy, q.0, q.1);
            let w = g.det * q.2;
            let mut u = [0.0; 2];
            for a in 0..6 {
                u[0] += g.n[a] * s.u[t[a] as usize][0];
                u[1] += g.n[a] * s.u[t[a] as usize][1];
            }
            for a in 0..6 {
                let ia = t[a] as usize;
                if ia == 0 { continue; }
                rhs[ia - 1] += w * (-u[1] * g.dn[a][0] + u[0] * g.dn[a][1]);
                for b in 0..6 {
                    let ib = t[b] as usize;
                    if ib != 0 {
                        k.add(ia - 1, ib - 1, w * (g.dn[a][0] * g.dn[b][0] + g.dn[a][1] * g.dn[b][1]));
                    }
                }
            }
        }
    }
    let x = solver.solve(&k, &rhs)?;
    let mut psi = vec![0.0; nn];
    psi[1..].copy_from_slice(&x);
    // Zero on a wall that does not move, if there is one (the usual convention).
    let still = mesh.edges.iter().find(|e| e.n.iter().all(|n| s.u[*n as usize] == [0.0, 0.0])).map(|e| psi[e.n[0] as usize]).unwrap_or(0.0);
    psi.iter_mut().for_each(|v| *v -= still);
    Ok(psi)
}

/// A field at every node by its id; None for an unknown id or temperature without heat.
/// `psi` is the stream function if it has been worked out.
pub fn nodal(mesh: &Mesh, s: &State, field: &str, psi: Option<&[f64]>) -> Option<Vec<f64>> {
    Some(match field {
        "speed" => s.u.iter().map(|u| u[0].hypot(u[1])).collect(),
        "u" => s.u.iter().map(|u| u[0]).collect(),
        "v" => s.u.iter().map(|u| u[1]).collect(),
        "pressure" => pressure(mesh, s),
        "temperature" if !s.t.is_empty() => s.t.clone(),
        "vorticity" => vorticity(mesh, s),
        "stream" => psi?.to_vec(),
        _ => return None,
    })
}

/// Levels for drawing `n` streamlines: evenly spaced through the sorted nodal values, so slow
/// eddies get lines too, not only the main stream.
pub fn stream_levels(psi: &[f64], n: usize) -> Vec<f64> {
    let mut v: Vec<f64> = psi.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() || n == 0 {
        return vec![];
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let mut out: Vec<f64> = (1..=n).map(|k| v[((k as f64 / (n + 1) as f64) * (v.len() - 1) as f64).round() as usize]).collect();
    out.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * (a.abs() + b.abs()) + f64::MIN_POSITIVE);
    out
}

/// The smallest and largest value, ignoring anything not a number.
pub fn range(v: &[f64]) -> [f64; 2] {
    let r = v.iter().filter(|x| x.is_finite()).fold([f64::INFINITY, f64::NEG_INFINITY], |r, x| [r[0].min(*x), r[1].max(*x)]);
    if r[0] > r[1] { [0.0, 0.0] } else { r }
}

/// The lowest (or highest) value of a nodal field anywhere on the mesh, not only at the nodes,
/// and where it is: the elements round the extreme node are searched on a fine grid of their
/// quadratic shape functions.
pub fn extremum(mesh: &Mesh, v: &[f64], lowest: bool) -> (f64, [f64; 2]) {
    let better = |a: f64, b: f64| if lowest { a < b } else { a > b };
    let Some(node) = (0..v.len()).filter(|i| v[*i].is_finite()).reduce(|a, b| if better(v[b], v[a]) { b } else { a }) else { return (f64::NAN, [f64::NAN; 2]) };
    let mut best = (v[node], mesh.nodes[node]);
    const N: usize = 48;
    for t in mesh.tris.iter().filter(|t| t.contains(&(node as u32))) {
        let xy = t.map(|n| mesh.nodes[n as usize]);
        for i in 0..=N {
            for j in 0..=(N - i) {
                let g = geo(&xy, i as f64 / N as f64, j as f64 / N as f64);
                let val: f64 = (0..6).map(|a| g.n[a] * v[t[a] as usize]).sum();
                if better(val, best.0) { best = (val, g.x); }
            }
        }
    }
    best
}
