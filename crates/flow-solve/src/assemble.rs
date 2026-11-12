//! Assembly: the residual of the discrete equations at a state, and its Jacobian (the full Newton
//! one, or the Picard one that leaves out how the convecting velocity changes).
//!
//! Unknowns: velocity u on all six nodes, pressure p on the corners, temperature T on all six.
//!   momentum    ρ (u·∇)u − μ ∇²u + ∇p + ρ β (T − T₀) g = F
//!   continuity  ∇·u = 0
//!   energy      ρ cp u·∇T − k ∇²T = S
//! in weak form, with test functions v, q, w:
//!   R_u = ∫ ρ (u·∇u)·v + μ ∇u:∇v − p ∇·v + γ (∇·u)(∇·v) + ρ β (T − T₀) g·v − F·v  +  ∫_out p_out n·v
//!   R_p = −∫ q ∇·u
//!   R_T = ∫ ρ cp (u·∇T) w + k ∇T·∇w − S w  −  ∫_flux q w

use crate::fe::{edge_geo, geo, G3, QP};
use crate::setup::Setup;
use crate::sparse::Csc;
use flow_mesh::Mesh;

/// Where each unknown lives in the solution vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dofs {
    pub nn: usize,
    pub nv: usize,
    pub heat: bool,
}
impl Dofs {
    pub fn of(mesh: &Mesh, heat: bool) -> Dofs { Dofs { nn: mesh.nodes.len(), nv: mesh.nv, heat } }
    pub fn n(&self) -> usize { 2 * self.nn + self.nv + if self.heat { self.nn } else { 0 } }
    #[inline] pub fn u(&self, node: usize, c: usize) -> usize { 2 * node + c }
    #[inline] pub fn p(&self, corner: usize) -> usize { 2 * self.nn + corner }
    #[inline] pub fn t(&self, node: usize) -> usize { 2 * self.nn + self.nv + node }
}

/// The free unknowns: `map[dof]` is its row in the reduced system, or usize::MAX when it is fixed.
pub struct Free {
    pub map: Vec<usize>,
    pub n: usize,
}

pub fn free(d: Dofs, s: &Setup) -> Free {
    let mut map = vec![usize::MAX; d.n()];
    let mut n = 0;
    for (i, slot) in map.iter_mut().enumerate() {
        let fixed = if i < 2 * d.nn { s.vel[i].is_some() } else if i < 2 * d.nn + d.nv { s.pin == Some(i - 2 * d.nn) } else { s.temp[i - 2 * d.nn - d.nv].is_some() };
        if !fixed {
            *slot = n;
            n += 1;
        }
    }
    Free { map, n }
}

/// The pattern of the reduced Jacobian: an unknown couples with the unknowns on every node that
/// shares an element with its own node (pressure with pressure, and pressure with temperature,
/// never couple).
pub fn pattern(mesh: &Mesh, d: Dofs, f: &Free) -> Csc {
    let mut adj: Vec<Vec<u32>> = vec![vec![]; d.nn];
    for t in &mesh.tris {
        for a in t {
            adj[*a as usize].extend_from_slice(t);
        }
    }
    for a in adj.iter_mut() {
        a.sort_unstable();
        a.dedup();
    }
    let mut cols: Vec<Vec<usize>> = vec![vec![]; f.n];
    let push = |col: &mut Vec<usize>, dof: usize| {
        if f.map[dof] != usize::MAX { col.push(f.map[dof]); }
    };
    for node in 0..d.nn {
        let near = &adj[node];
        // Rows in the order of the unknowns: velocities, pressures, temperatures.
        let mut vel = vec![];
        let mut pre = vec![];
        let mut tem = vec![];
        for &m in near {
            let m = m as usize;
            push(&mut vel, d.u(m, 0));
            push(&mut vel, d.u(m, 1));
            if m < d.nv { push(&mut pre, d.p(m)); }
            if d.heat { push(&mut tem, d.t(m)); }
        }
        for c in 0..2 {
            let col = f.map[d.u(node, c)];
            if col != usize::MAX {
                cols[col] = vel.iter().chain(&pre).chain(&tem).copied().collect();
            }
        }
        if node < d.nv {
            let col = f.map[d.p(node)];
            if col != usize::MAX { cols[col] = vel.clone(); }
        }
        if d.heat {
            let col = f.map[d.t(node)];
            if col != usize::MAX { cols[col] = vel.iter().chain(&tem).copied().collect(); }
        }
    }
    Csc::from_columns(cols)
}

/// How much of the driving (boundary speeds, outlet pressures, gravity, body forces) is applied:
/// 1 for the real problem, less during continuation.
#[derive(Clone, Copy, Debug)]
pub struct Load(pub f64);

/// The residual of every equation (fixed unknowns included: there it is the reaction), and, when
/// `jac` is given, the Jacobian on the free unknowns.
pub fn assemble(mesh: &Mesh, s: &Setup, d: Dofs, x: &[f64], load: Load, newton: bool, mut jac: Option<(&mut Csc, &Free)>) -> Vec<f64> {
    let mut r = vec![0.0; d.n()];
    if let Some((j, _)) = jac.as_mut() {
        j.clear();
    }
    let lam = load.0;
    let g = [s.g[0] * lam, s.g[1] * lam];
    let buoy = if s.buoyancy { s.rho * s.beta } else { 0.0 };
    let rcp = s.rho * s.cp;
    let nl = if d.heat { 21 } else { 15 };
    let mut ke = vec![0.0f64; 21 * 21];
    let mut idx = [0usize; 21];
    for tri in &mesh.tris {
        let mut xy = [[0.0; 2]; 6];
        let mut ue = [[0.0; 2]; 6];
        let mut pe = [0.0; 3];
        let mut te = [0.0; 6];
        for a in 0..6 {
            let n = tri[a] as usize;
            xy[a] = mesh.nodes[n];
            ue[a] = [x[d.u(n, 0)], x[d.u(n, 1)]];
            idx[2 * a] = d.u(n, 0);
            idx[2 * a + 1] = d.u(n, 1);
            if d.heat {
                te[a] = x[d.t(n)];
                idx[15 + a] = d.t(n);
            }
        }
        for b in 0..3 {
            pe[b] = x[d.p(tri[b] as usize)];
            idx[12 + b] = d.p(tri[b] as usize);
        }
        let mut re = [0.0f64; 21];
        let want = jac.is_some();
        if want {
            ke.iter_mut().for_each(|v| *v = 0.0);
        }
        for q in QP {
            let e = geo(&xy, q.0, q.1);
            let w = e.det * q.2;
            let (mut u, mut gu, mut p, mut t, mut gt) = ([0.0; 2], [[0.0; 2]; 2], 0.0, 0.0, [0.0; 2]);
            for a in 0..6 {
                for i in 0..2 {
                    u[i] += e.n[a] * ue[a][i];
                    gu[i][0] += e.dn[a][0] * ue[a][i];
                    gu[i][1] += e.dn[a][1] * ue[a][i];
                }
                t += e.n[a] * te[a];
                gt[0] += e.dn[a][0] * te[a];
                gt[1] += e.dn[a][1] * te[a];
            }
            for b in 0..3 {
                p += e.l[b] * pe[b];
            }
            let div = gu[0][0] + gu[1][1];
            let conv = [u[0] * gu[0][0] + u[1] * gu[0][1], u[0] * gu[1][0] + u[1] * gu[1][1]];
            let body = match &s.force { Some(f) => { let v = f(e.x); [v[0] * lam, v[1] * lam] } None => [0.0; 2] };
            let dt = t - s.t_ref;
            // u·∇N for every shape function.
            let mut adv = [0.0; 6];
            for a in 0..6 {
                adv[a] = u[0] * e.dn[a][0] + u[1] * e.dn[a][1];
            }
            for a in 0..6 {
                for i in 0..2 {
                    re[2 * a + i] += w * (s.rho * conv[i] * e.n[a] + s.mu * (gu[i][0] * e.dn[a][0] + gu[i][1] * e.dn[a][1]) - p * e.dn[a][i] + s.grad_div * div * e.dn[a][i] + (buoy * dt * g[i] - body[i]) * e.n[a]);
                }
            }
            for b in 0..3 {
                re[12 + b] -= w * e.l[b] * div;
            }
            if d.heat {
                let src = s.source.as_ref().map(|f| f(e.x)).unwrap_or(0.0);
                let ugt = u[0] * gt[0] + u[1] * gt[1];
                for a in 0..6 {
                    re[15 + a] += w * (rcp * ugt * e.n[a] + s.k * (gt[0] * e.dn[a][0] + gt[1] * e.dn[a][1]) - src * e.n[a]);
                }
            }
            if want {
                for a in 0..6 {
                    for b in 0..6 {
                        let lap = e.dn[a][0] * e.dn[b][0] + e.dn[a][1] * e.dn[b][1];
                        let same = w * (s.rho * adv[b] * e.n[a] + s.mu * lap);
                        for i in 0..2 {
                            for j in 0..2 {
                                let mut v = w * s.grad_div * e.dn[a][i] * e.dn[b][j];
                                if i == j { v += same; }
                                if newton { v += w * s.rho * e.n[b] * gu[i][j] * e.n[a]; }
                                ke[(2 * a + i) * 21 + 2 * b + j] += v;
                            }
                            if d.heat {
                                // Momentum against temperature (buoyancy), energy against velocity.
                                ke[(2 * a + i) * 21 + 15 + b] += w * buoy * g[i] * e.n[b] * e.n[a];
                                if newton { ke[(15 + a) * 21 + 2 * b + i] += w * rcp * e.n[b] * gt[i] * e.n[a]; }
                            }
                        }
                        if d.heat {
                            ke[(15 + a) * 21 + 15 + b] += w * (rcp * adv[b] * e.n[a] + s.k * lap);
                        }
                    }
                    for b in 0..3 {
                        for i in 0..2 {
                            let v = -w * e.l[b] * e.dn[a][i];
                            ke[(2 * a + i) * 21 + 12 + b] += v;
                            ke[(12 + b) * 21 + 2 * a + i] += v;
                        }
                    }
                }
            }
        }
        for a in 0..nl {
            r[idx[a]] += re[a];
        }
        if let Some((j, f)) = jac.as_mut() {
            for a in 0..nl {
                let row = f.map[idx[a]];
                if row == usize::MAX { continue; }
                for b in 0..nl {
                    let col = f.map[idx[b]];
                    let v = ke[a * 21 + b];
                    if col != usize::MAX && v != 0.0 {
                        j.add(row, col, v);
                    }
                }
            }
        }
    }
    // Boundary loads: the pressure at open edges, and heat fluxes.
    let edge = |i: usize| { let e = &mesh.edges[i]; (e.n, mesh.nodes[e.n[0] as usize], mesh.nodes[e.n[1] as usize], mesh.nodes[e.n[2] as usize]) };
    for &(i, pressure) in &s.outlets {
        if pressure == 0.0 { continue; }
        let (n, a, m, b) = edge(i);
        for (sp, wq) in G3 {
            let (_, t, sh) = edge_geo(a, m, b, sp);
            // The outward normal times the length element: the fluid is on the left of the edge.
            let nds = [t[1] * wq, -t[0] * wq];
            for k in 0..3 {
                r[d.u(n[k] as usize, 0)] += pressure * lam * nds[0] * sh[k];
                r[d.u(n[k] as usize, 1)] += pressure * lam * nds[1] * sh[k];
            }
        }
    }
    if d.heat {
        for &(i, q) in &s.fluxes {
            let (n, a, m, b) = edge(i);
            for (sp, wq) in G3 {
                let (_, t, sh) = edge_geo(a, m, b, sp);
                let ds = t[0].hypot(t[1]) * wq;
                for k in 0..3 {
                    r[d.t(n[k] as usize)] -= q * ds * sh[k];
                }
            }
        }
    }
    r
}
