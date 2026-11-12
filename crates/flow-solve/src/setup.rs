//! The problem the solver is given: fluid properties, which unknowns are fixed and at what value,
//! and the loads on the boundary. Built from a model and its mesh, or by hand (the verification
//! cases set exact solutions on the boundary and add body forces).

use crate::fe::edge_geo;
use flow_core::model::*;
use flow_mesh::Mesh;
use std::sync::Arc;

pub type VecFn = Arc<dyn Fn([f64; 2]) -> [f64; 2] + Send + Sync>;
pub type ScalarFn = Arc<dyn Fn([f64; 2]) -> f64 + Send + Sync>;

#[derive(Clone)]
pub struct Setup {
    pub rho: f64,
    pub mu: f64,
    pub k: f64,
    pub cp: f64,
    pub beta: f64,
    pub g: [f64; 2],
    pub t_ref: f64,
    /// Solve the temperature too.
    pub heat: bool,
    /// The temperature pushes the flow (Boussinesq).
    pub buoyancy: bool,
    pub grad_div: f64,
    /// Fixed velocity components, two per node (x then y); None = free.
    pub vel: Vec<Option<f64>>,
    /// Fixed temperatures, one per node.
    pub temp: Vec<Option<f64>>,
    /// A corner node whose pressure is held at zero (closed regions, where pressure has no level).
    pub pin: Option<usize>,
    /// Boundary edges (index into the mesh's edges) open to a pressure.
    pub outlets: Vec<(usize, f64)>,
    /// Boundary edges with a heat flux into the fluid (W/m²).
    pub fluxes: Vec<(usize, f64)>,
    /// Body force per volume (N/m³), for manufactured solutions.
    pub force: Option<VecFn>,
    /// Heat source per volume (W/m³), for manufactured solutions.
    pub source: Option<ScalarFn>,
}

impl Setup {
    /// A bare problem on a mesh: nothing fixed, no loads.
    pub fn new(mesh: &Mesh, rho: f64, mu: f64) -> Setup {
        Setup { rho, mu, k: 1.0, cp: 1.0, beta: 0.0, g: [0.0; 2], t_ref: 0.0, heat: false, buoyancy: false, grad_div: 0.0, vel: vec![None; 2 * mesh.nodes.len()], temp: vec![None; mesh.nodes.len()], pin: None, outlets: vec![], fluxes: vec![], force: None, source: None }
    }

    /// Fix the velocity on every boundary node to `u(x)` and pin the pressure (an enclosed
    /// problem with a known solution).
    pub fn enclose(&mut self, mesh: &Mesh, u: impl Fn([f64; 2]) -> [f64; 2]) {
        for e in &mesh.edges {
            for n in e.n {
                let v = u(mesh.nodes[n as usize]);
                self.vel[2 * n as usize] = Some(v[0]);
                self.vel[2 * n as usize + 1] = Some(v[1]);
            }
        }
        self.pin = Some(0);
    }

    /// The problem a model poses on its mesh.
    pub fn from_model(m: &Model, mesh: &Mesh) -> Result<Setup, String> {
        let nn = mesh.nodes.len();
        let f = &m.fluid;
        let mut s = Setup {
            rho: f.rho, mu: f.mu, k: f.k, cp: f.cp, beta: f.beta, g: m.physics.gravity, t_ref: m.physics.t_ref, heat: m.physics.heat, buoyancy: m.physics.heat && m.physics.buoyancy,
            grad_div: m.solver.grad_div, vel: vec![None; 2 * nn], temp: vec![None; nn], pin: None, outlets: vec![], fluxes: vec![], force: None, source: None,
        };
        if mesh.edges.iter().any(|e| e.boundary == 0 || m.boundary(e.boundary).is_none()) {
            return Err("An edge of the flow region has no boundary. Say what every edge is before solving.".into());
        }
        // The inward normal at every node of each boundary (averaged where edges meet).
        let mut normal: std::collections::HashMap<(u32, u32), [f64; 2]> = std::collections::HashMap::new();
        for e in &mesh.edges {
            let (a, mid, b) = (mesh.nodes[e.n[0] as usize], mesh.nodes[e.n[1] as usize], mesh.nodes[e.n[2] as usize]);
            for (k, sp) in [0.0, 0.5, 1.0].into_iter().enumerate() {
                let (_, t, _) = edge_geo(a, mid, b, sp);
                // The fluid is on the left of the edge.
                let n = normal.entry((e.boundary, e.n[k])).or_insert([0.0; 2]);
                n[0] -= t[1];
                n[1] += t[0];
            }
        }
        // Later passes win where boundaries meet: symmetry, then inlets, then moving walls, then fixed walls.
        let rank = |b: &Boundary| match b.kind { BcKind::Symmetry => 0, BcKind::Inlet { .. } => 1, BcKind::Wall { u } if u != [0.0, 0.0] => 2, BcKind::Wall { .. } => 3, BcKind::Outlet { .. } => 4 };
        for pass in 0..4 {
            for (i, e) in mesh.edges.iter().enumerate() {
                let b = m.boundary(e.boundary).expect("checked above");
                if rank(b) != pass && !(pass == 0 && rank(b) == 4) {
                    continue;
                }
                match &b.kind {
                    BcKind::Outlet { pressure } => s.outlets.push((i, *pressure)),
                    BcKind::Symmetry => {
                        let (p, q) = (mesh.nodes[e.n[0] as usize], mesh.nodes[e.n[2] as usize]);
                        // No flow across: the component along the normal is zero.
                        let c = if (q[0] - p[0]).abs() >= (q[1] - p[1]).abs() { 1 } else { 0 };
                        for n in e.n {
                            s.vel[2 * n as usize + c] = Some(0.0);
                        }
                    }
                    BcKind::Inlet { speed, profile } => {
                        let ends = m.shape(e.shape).map(|sh| sh.geom.edge_ends(e.edge as usize));
                        for n in e.n {
                            let x = mesh.nodes[n as usize];
                            let nrm = normal[&(e.boundary, n)];
                            let len = nrm[0].hypot(nrm[1]).max(1e-300);
                            let mut v = *speed;
                            if let (Profile::Parabolic, Some((p, q))) = (profile, ends) {
                                let l2 = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2)).max(1e-300);
                                let t = (((x[0] - p[0]) * (q[0] - p[0]) + (x[1] - p[1]) * (q[1] - p[1])) / l2).clamp(0.0, 1.0);
                                v *= 6.0 * t * (1.0 - t);
                            }
                            s.vel[2 * n as usize] = Some(v * nrm[0] / len);
                            s.vel[2 * n as usize + 1] = Some(v * nrm[1] / len);
                        }
                    }
                    BcKind::Wall { u } => {
                        for n in e.n {
                            s.vel[2 * n as usize] = Some(u[0]);
                            s.vel[2 * n as usize + 1] = Some(u[1]);
                        }
                    }
                }
            }
        }
        if s.heat {
            for (i, e) in mesh.edges.iter().enumerate() {
                match m.boundary(e.boundary).expect("checked above").heat {
                    HeatBc::Flux { q } => if q != 0.0 { s.fluxes.push((i, q)) },
                    HeatBc::Temperature { t } => for n in e.n { s.temp[n as usize] = Some(t) },
                    HeatBc::Insulated => {}
                }
            }
            if s.temp.iter().all(|t| t.is_none()) {
                return Err("Heat is on but no boundary has a temperature, so the temperature level is not set.".into());
            }
        }
        if s.outlets.is_empty() {
            // A closed region: the pressure has no level of its own. Hold it at one corner node.
            s.pin = Some(0);
        }
        Ok(s)
    }
}
