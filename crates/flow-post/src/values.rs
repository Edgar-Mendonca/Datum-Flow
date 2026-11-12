//! Numbers read off a solution: values at points and along lines, and values integrated over
//! boundaries. Forces and wall heat flows come from the reactions of the discrete equations (the
//! residual at the fixed unknowns), which is far more accurate than differentiating the fields.

use crate::locate::{Locator, Place};
use flow_core::model::*;
use flow_core::numbers::numbers;
use flow_mesh::{BEdge, Mesh};
use flow_solve::fe::{edge_geo, geo, G3};
use flow_solve::{Dofs, State};
use serde::Serialize;

/// Everything at one point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Sample {
    pub x: f64,
    pub y: f64,
    pub u: f64,
    pub v: f64,
    pub speed: f64,
    pub pressure: f64,
    /// NaN without heat.
    pub temperature: f64,
    pub vorticity: f64,
    /// False when the point is outside the fluid (the values are then NaN).
    pub inside: bool,
}

pub fn sample_at(mesh: &Mesh, s: &State, at: Place) -> Sample {
    let t = &mesh.tris[at.element];
    let xy = t.map(|n| mesh.nodes[n as usize]);
    let g = geo(&xy, at.xi, at.eta);
    let (mut u, mut v, mut p, mut temp, mut om) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for a in 0..6 {
        let n = t[a] as usize;
        u += g.n[a] * s.u[n][0];
        v += g.n[a] * s.u[n][1];
        om += g.dn[a][0] * s.u[n][1] - g.dn[a][1] * s.u[n][0];
        if !s.t.is_empty() { temp += g.n[a] * s.t[n]; }
    }
    for b in 0..3 {
        p += g.l[b] * s.p[t[b] as usize];
    }
    Sample { x: g.x[0], y: g.x[1], u, v, speed: u.hypot(v), pressure: p, temperature: if s.t.is_empty() { f64::NAN } else { temp }, vorticity: om, inside: true }
}

/// The solution at a point; outside the fluid every value is NaN.
pub fn sample(mesh: &Mesh, s: &State, loc: &Locator, p: [f64; 2]) -> Sample {
    match loc.find(mesh, p) {
        Some(at) => Sample { x: p[0], y: p[1], ..sample_at(mesh, s, at) },
        None => Sample { x: p[0], y: p[1], u: f64::NAN, v: f64::NAN, speed: f64::NAN, pressure: f64::NAN, temperature: f64::NAN, vorticity: f64::NAN, inside: false },
    }
}

/// The solution at `n` points from a to b, with the distance from a.
pub fn along(mesh: &Mesh, s: &State, loc: &Locator, l: &Line) -> Vec<(f64, Sample)> {
    let n = l.n.clamp(2, 5000) as usize;
    let len = (l.b[0] - l.a[0]).hypot(l.b[1] - l.a[1]);
    (0..n).map(|k| {
        let t = k as f64 / (n - 1) as f64;
        (t * len, sample(mesh, s, loc, [l.a[0] + (l.b[0] - l.a[0]) * t, l.a[1] + (l.b[1] - l.a[1]) * t]))
    }).collect()
}

/// Velocity arrows on a regular grid of about `count` points inside the fluid: (x, y, u, v).
pub fn arrows(mesh: &Mesh, s: &State, loc: &Locator, bounds: [f64; 4], count: usize) -> Vec<[f64; 4]> {
    let (w, h) = (bounds[2] - bounds[0], bounds[3] - bounds[1]);
    if !(w > 0.0 && h > 0.0) || count == 0 {
        return vec![];
    }
    let step = (w * h / count.min(40_000) as f64).sqrt();
    let (nx, ny) = ((w / step).round().max(1.0) as usize, (h / step).round().max(1.0) as usize);
    let mut out = vec![];
    for j in 0..ny {
        for i in 0..nx {
            let p = [bounds[0] + (i as f64 + 0.5) * w / nx as f64, bounds[1] + (j as f64 + 0.5) * h / ny as f64];
            if let Some(at) = loc.find(mesh, p) {
                let v = sample_at(mesh, s, at);
                out.push([p[0], p[1], v.u, v.v]);
            }
        }
    }
    out
}

/// A solution with what is needed to integrate over its boundaries.
pub struct Values<'a> {
    pub m: &'a Model,
    pub mesh: &'a Mesh,
    pub state: &'a State,
    /// The residual of every equation (reactions at fixed unknowns), from the solver.
    pub residual: &'a [f64],
    pub dofs: Dofs,
    /// The element each boundary edge belongs to, and which of its three sides it is.
    owner: Vec<(u32, u8)>,
}

impl<'a> Values<'a> {
    pub fn new(m: &'a Model, mesh: &'a Mesh, state: &'a State, residual: &'a [f64], dofs: Dofs) -> Values<'a> {
        let mut side: std::collections::HashMap<(u32, u32), (u32, u8)> = std::collections::HashMap::with_capacity(mesh.edges.len());
        let wanted: std::collections::HashSet<(u32, u32)> = mesh.edges.iter().map(|e| (e.n[0], e.n[2])).collect();
        for (k, t) in mesh.tris.iter().enumerate() {
            for j in 0..3 {
                let key = (t[j], t[(j + 1) % 3]);
                if wanted.contains(&key) { side.insert(key, (k as u32, j as u8)); }
            }
        }
        let owner = mesh.edges.iter().map(|e| side.get(&(e.n[0], e.n[2])).copied().unwrap_or((0, 0))).collect();
        Values { m, mesh, state, residual, dofs, owner }
    }
}

/// Which nodes of a boundary take their reaction whole, and which are corners shared with
/// another boundary (there the reaction mixes both, so that node's part is integrated directly).
struct Split {
    own: Vec<u32>,
    shared: std::collections::HashSet<u32>,
}

impl Values<'_> {
    fn split(&self, boundary: u32, counts: impl Fn(&Boundary) -> bool) -> Split {
        let mut nodes: Vec<u32> = self.edges(boundary).flat_map(|e| e.n).collect();
        nodes.sort_unstable();
        nodes.dedup();
        let mine: std::collections::HashSet<u32> = nodes.iter().copied().collect();
        let mut shared = std::collections::HashSet::new();
        for e in self.mesh.edges.iter().filter(|e| e.boundary != boundary) {
            if self.m.boundary(e.boundary).is_some_and(&counts) {
                shared.extend(e.n.iter().filter(|n| mine.contains(n)));
            }
        }
        Split { own: nodes.into_iter().filter(|n| !shared.contains(n)).collect(), shared }
    }
    /// ∫ over a boundary's own edges of f × (the shape function of each shared node), where f
    /// gets the element's fields at the point: (velocity gradient, pressure, temperature
    /// gradient, outward normal). Returns the sum over the shared nodes.
    fn direct(&self, boundary: u32, shared: &std::collections::HashSet<u32>, mut f: impl FnMut([[f64; 2]; 2], f64, [f64; 2], [f64; 2]) -> [f64; 2]) -> [f64; 2] {
        let mut sum = [0.0; 2];
        if shared.is_empty() {
            return sum;
        }
        const CORNER: [[f64; 2]; 3] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        for (i, e) in self.mesh.edges.iter().enumerate().filter(|(_, e)| e.boundary == boundary) {
            if !e.n.iter().any(|n| shared.contains(n)) {
                continue;
            }
            let (el, j) = (self.owner[i].0 as usize, self.owner[i].1 as usize);
            let t = &self.mesh.tris[el];
            let xy = t.map(|n| self.mesh.nodes[n as usize]);
            let (a, m, b) = (self.mesh.nodes[e.n[0] as usize], self.mesh.nodes[e.n[1] as usize], self.mesh.nodes[e.n[2] as usize]);
            let (c0, c1) = (CORNER[j], CORNER[(j + 1) % 3]);
            for (sp, w) in G3 {
                let (_, tan, sh) = edge_geo(a, m, b, sp);
                let ds = tan[0].hypot(tan[1]);
                let nrm = [tan[1] / ds, -tan[0] / ds];
                let g = geo(&xy, c0[0] + sp * (c1[0] - c0[0]), c0[1] + sp * (c1[1] - c0[1]));
                let (mut gu, mut gt, mut p) = ([[0.0; 2]; 2], [0.0; 2], 0.0);
                for k in 0..6 {
                    let n = t[k] as usize;
                    for c in 0..2 {
                        gu[c][0] += g.dn[k][0] * self.state.u[n][c];
                        gu[c][1] += g.dn[k][1] * self.state.u[n][c];
                    }
                    if !self.state.t.is_empty() {
                        gt[0] += g.dn[k][0] * self.state.t[n];
                        gt[1] += g.dn[k][1] * self.state.t[n];
                    }
                }
                for k in 0..3 {
                    p += g.l[k] * self.state.p[t[k] as usize];
                }
                let v = f(gu, p, gt, nrm);
                let weight: f64 = (0..3).filter(|k| shared.contains(&e.n[*k])).map(|k| sh[k]).sum();
                sum[0] += w * ds * weight * v[0];
                sum[1] += w * ds * weight * v[1];
            }
        }
        sum
    }
    fn edges(&self, boundary: u32) -> impl Iterator<Item = &BEdge> {
        self.mesh.edges.iter().filter(move |e| e.boundary == boundary)
    }
    /// ∫ f ds over a boundary; f gets the point's shape functions, the edge's nodes and the
    /// outward normal (unit).
    fn integrate(&self, boundary: u32, mut f: impl FnMut(&[f64; 3], &[u32; 3], [f64; 2]) -> f64) -> f64 {
        let mut sum = 0.0;
        for e in self.edges(boundary) {
            let (a, m, b) = (self.mesh.nodes[e.n[0] as usize], self.mesh.nodes[e.n[1] as usize], self.mesh.nodes[e.n[2] as usize]);
            for (sp, w) in G3 {
                let (_, t, sh) = edge_geo(a, m, b, sp);
                let ds = t[0].hypot(t[1]);
                sum += w * ds * f(&sh, &e.n, [t[1] / ds, -t[0] / ds]);
            }
        }
        sum
    }
    /// Length of a boundary (m).
    pub fn length(&self, boundary: u32) -> f64 {
        self.integrate(boundary, |_, _, _| 1.0)
    }
    /// Volume flow leaving the fluid through a boundary (m³/s; negative comes in).
    pub fn flow_rate(&self, boundary: u32) -> f64 {
        let u = &self.state.u;
        self.m.physics.depth * self.integrate(boundary, |sh, n, nrm| (0..3).map(|k| sh[k] * (u[n[k] as usize][0] * nrm[0] + u[n[k] as usize][1] * nrm[1])).sum())
    }
    /// Mean pressure on a boundary (Pa).
    pub fn mean_pressure(&self, boundary: u32) -> f64 {
        let p = crate::fields::pressure(self.mesh, self.state);
        // Pressure is linear along an edge: its ends carry it, the midside node is their mean.
        self.integrate(boundary, |sh, n, _| (sh[0] + 0.5 * sh[1]) * p[n[0] as usize] + (sh[2] + 0.5 * sh[1]) * p[n[2] as usize]) / self.length(boundary).max(1e-300)
    }
    /// The force of the fluid on a wall (N: x, y), pressure and friction together.
    pub fn force(&self, boundary: u32) -> [f64; 2] {
        let d = self.dofs;
        let sp = self.split(boundary, |_| true);
        // The reaction is the traction the wall puts on the fluid; the force on the wall is its opposite.
        let mut f = [0.0; 2];
        for n in &sp.own {
            f[0] -= self.residual[d.u(*n as usize, 0)];
            f[1] -= self.residual[d.u(*n as usize, 1)];
        }
        let mu = self.m.fluid.mu;
        let t = self.direct(boundary, &sp.shared, |gu, p, _, n| [mu * (gu[0][0] * n[0] + gu[0][1] * n[1]) - p * n[0], mu * (gu[1][0] * n[0] + gu[1][1] * n[1]) - p * n[1]]);
        [(f[0] - t[0]) * self.m.physics.depth, (f[1] - t[1]) * self.m.physics.depth]
    }
    /// The direction of the oncoming flow: the mean direction fluid enters through the inlets, or
    /// +x when there are none.
    pub fn flow_dir(&self) -> [f64; 2] {
        let mut d = [0.0; 2];
        for b in self.m.boundaries.iter().filter(|b| matches!(b.kind, BcKind::Inlet { .. })) {
            let u = &self.state.u;
            d[0] += self.integrate(b.id, |sh, n, _| (0..3).map(|k| sh[k] * u[n[k] as usize][0]).sum());
            d[1] += self.integrate(b.id, |sh, n, _| (0..3).map(|k| sh[k] * u[n[k] as usize][1]).sum());
        }
        let l = d[0].hypot(d[1]);
        if l > 0.0 { [d[0] / l, d[1] / l] } else { [1.0, 0.0] }
    }
    /// Drag (along the oncoming flow) and lift (across it, to its left) on a wall (N).
    pub fn drag_lift(&self, boundary: u32) -> [f64; 2] {
        let (f, d) = (self.force(boundary), self.flow_dir());
        [f[0] * d[0] + f[1] * d[1], -f[0] * d[1] + f[1] * d[0]]
    }
    /// Heat passing from a boundary into the fluid by conduction (W).
    pub fn heat_flow(&self, boundary: u32) -> f64 {
        if !self.dofs.heat {
            return 0.0;
        }
        let depth = self.m.physics.depth;
        match self.m.boundary(boundary).map(|b| &b.heat) {
            Some(HeatBc::Flux { q }) => q * self.length(boundary) * depth,
            Some(HeatBc::Temperature { .. }) => {
                // Corners shared with an insulated edge keep their reaction (no heat crosses there).
                let sp = self.split(boundary, |b| b.heat != HeatBc::Insulated);
                let k = self.m.fluid.k;
                let own: f64 = sp.own.iter().map(|n| self.residual[self.dofs.t(*n as usize)]).sum();
                depth * (own + self.direct(boundary, &sp.shared, |_, _, gt, n| [k * (gt[0] * n[0] + gt[1] * n[1]), 0.0])[0])
            }
            _ => 0.0,
        }
    }
    /// Heat carried out of the fluid through a boundary by the flow, relative to the reference
    /// temperature (W).
    pub fn heat_carried(&self, boundary: u32) -> f64 {
        if !self.dofs.heat {
            return 0.0;
        }
        let (u, t, t0) = (&self.state.u, &self.state.t, self.m.physics.t_ref);
        self.m.fluid.rho * self.m.fluid.cp * self.m.physics.depth * self.integrate(boundary, |sh, n, nrm| {
            let un: f64 = (0..3).map(|k| sh[k] * (u[n[k] as usize][0] * nrm[0] + u[n[k] as usize][1] * nrm[1])).sum();
            let tt: f64 = (0..3).map(|k| sh[k] * t[n[k] as usize]).sum();
            un * (tt - t0)
        })
    }
    /// Mean temperature on a boundary (K): weighted by the flow through it where there is flow
    /// (the "mixing-cup" temperature), by length otherwise.
    pub fn mean_temperature(&self, boundary: u32) -> f64 {
        if !self.dofs.heat {
            return f64::NAN;
        }
        let (u, t) = (&self.state.u, &self.state.t);
        let flow = self.integrate(boundary, |sh, n, nrm| (0..3).map(|k| sh[k] * (u[n[k] as usize][0] * nrm[0] + u[n[k] as usize][1] * nrm[1])).sum());
        let len = self.length(boundary).max(1e-300);
        let speed = numbers(self.m).speed;
        if flow.abs() > 1e-9 * speed.max(1e-300) * len {
            self.integrate(boundary, |sh, n, nrm| {
                let un: f64 = (0..3).map(|k| sh[k] * (u[n[k] as usize][0] * nrm[0] + u[n[k] as usize][1] * nrm[1])).sum();
                un * (0..3).map(|k| sh[k] * t[n[k] as usize]).sum::<f64>()
            }) / flow
        } else {
            self.integrate(boundary, |sh, n, _| (0..3).map(|k| sh[k] * t[n[k] as usize]).sum()) / len
        }
    }
    /// The average Nusselt number on a wall: its heat flow against pure conduction across the
    /// reference length under the model's temperature difference.
    pub fn nusselt(&self, boundary: u32) -> f64 {
        let n = numbers(self.m);
        let area = self.length(boundary) * self.m.physics.depth;
        self.heat_flow(boundary).abs() * n.length / (self.m.fluid.k * n.dt * area).max(1e-300)
    }
    /// Mass conservation: the net volume flow through all boundaries, and the flow that comes in.
    /// The first should be close to zero beside the second.
    pub fn balance(&self) -> (f64, f64) {
        let mut ids: Vec<u32> = self.mesh.edges.iter().map(|e| e.boundary).collect();
        ids.sort_unstable();
        ids.dedup();
        let flows: Vec<f64> = ids.iter().map(|b| self.flow_rate(*b)).collect();
        (flows.iter().sum(), flows.iter().filter(|f| **f < 0.0).map(|f| -f).sum())
    }
    /// Heat balance (W): conducted in through walls, carried out by the flow (net).
    pub fn heat_balance(&self) -> (f64, f64) {
        let mut ids: Vec<u32> = self.mesh.edges.iter().map(|e| e.boundary).collect();
        ids.sort_unstable();
        ids.dedup();
        (ids.iter().map(|b| self.heat_flow(*b)).sum(), ids.iter().map(|b| self.heat_carried(*b)).sum())
    }
}

/// A monitor's value and the kind of unit it is shown in.
pub fn monitor(v: &Values, loc: &Locator, kind: &MonitorKind) -> (f64, &'static str) {
    match kind {
        MonitorKind::Drag { boundary } => (v.drag_lift(*boundary)[0], "force"),
        MonitorKind::Lift { boundary } => (v.drag_lift(*boundary)[1], "force"),
        MonitorKind::FlowRate { boundary } => (v.flow_rate(*boundary), "flow"),
        MonitorKind::PressureDrop { from, to } => (v.mean_pressure(*from) - v.mean_pressure(*to), "pressure"),
        MonitorKind::MeanTemperature { boundary } => (v.mean_temperature(*boundary), "temp"),
        MonitorKind::HeatFlow { boundary } => (v.heat_flow(*boundary), "power"),
        MonitorKind::Nusselt { boundary } => (v.nusselt(*boundary), "ratio"),
        MonitorKind::Speed { x, y } => (sample(v.mesh, v.state, loc, [*x, *y]).speed, "speed"),
    }
}
