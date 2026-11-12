//! The numbers that say what kind of flow this is: Reynolds, Rayleigh and Prandtl, worked out
//! from the model alone, with the speed, length and temperature difference they were built from.

use crate::model::*;
use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Numbers {
    /// Reference speed (m/s): the fastest inlet (mean speed) or moving wall.
    pub speed: f64,
    /// Reference length (m): the one set in Physics, else the inlet's width, else the region's smaller side.
    pub length: f64,
    /// Largest difference between the temperatures set on boundaries (K).
    pub dt: f64,
    /// Reynolds number ρ U L / μ.
    pub re: f64,
    /// Rayleigh number g β ΔT L³ / (ν α); 0 without buoyancy.
    pub ra: f64,
    /// Prandtl number ν / α.
    pub pr: f64,
    /// Flow comes in somewhere (a channel or a body in a stream), as opposed to a closed box.
    pub through: bool,
}

/// Length of one edge of a shape (m), arcs included.
pub fn edge_length(g: &Geom, edge: usize) -> f64 {
    match g {
        Geom::Circle { r, .. } => std::f64::consts::TAU * r,
        Geom::Poly { pts, bulge } => {
            if pts.is_empty() { return 0.0; }
            let (p, q) = (pts[edge % pts.len()], pts[(edge + 1) % pts.len()]);
            let c = (q[0] - p[0]).hypot(q[1] - p[1]);
            let b = bulge.get(edge).copied().unwrap_or(0.0);
            if b == 0.0 { c } else { let th = 4.0 * b.atan(); (c / (2.0 * (th / 2.0).sin()).abs()) * th.abs() }
        }
    }
}

pub fn numbers(m: &Model) -> Numbers {
    let len = |b: &Boundary| -> f64 { b.edges.iter().filter_map(|e| m.shape(e.shape).map(|s| edge_length(&s.geom, e.edge as usize))).sum() };
    let mut speed = 0.0f64;
    let mut inlet_width = 0.0f64;
    for b in &m.boundaries {
        if b.edges.is_empty() { continue; }
        match &b.kind {
            BcKind::Inlet { speed: s, .. } => {
                if *s > speed || inlet_width == 0.0 { inlet_width = len(b); }
                speed = speed.max(*s);
            }
            BcKind::Wall { u } => speed = speed.max(u[0].hypot(u[1])),
            _ => {}
        }
    }
    let through = inlet_width > 0.0;
    let side = m.bounds().map(|b| (b[2] - b[0]).min(b[3] - b[1])).unwrap_or(0.0);
    let length = if m.physics.length > 0.0 { m.physics.length } else if through { inlet_width } else { side };
    let temps: Vec<f64> = m.boundaries.iter().filter(|b| !b.edges.is_empty()).filter_map(|b| if let HeatBc::Temperature { t } = b.heat { Some(t) } else { None }).collect();
    let dt = if m.physics.heat && !temps.is_empty() {
        let (lo, hi) = temps.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, t| (a.0.min(*t), a.1.max(*t)));
        // One temperature only: the fluid starts at the reference temperature.
        if temps.len() == 1 { (hi - m.physics.t_ref).abs() } else { hi - lo }
    } else { 0.0 };
    let f = &m.fluid;
    let g = m.physics.gravity[0].hypot(m.physics.gravity[1]);
    let ra = if m.physics.heat && m.physics.buoyancy { g * f.beta.abs() * dt * length.powi(3) / (f.nu() * f.alpha()) } else { 0.0 };
    Numbers { speed, length, dt, re: f.rho * speed * length / f.mu, ra, pr: f.pr(), through }
}

/// Above these the flow is probably turbulent and 0.1 will not solve it unless told to.
pub const RE_THROUGH: f64 = 4000.0;
pub const RE_CLOSED: f64 = 10_000.0;
pub const RA_LIMIT: f64 = 1e9;

impl Numbers {
    /// Why the flow is probably turbulent, in plain words; None when it should be laminar.
    pub fn turbulent(&self) -> Option<String> {
        let limit = if self.through { RE_THROUGH } else { RE_CLOSED };
        if self.re > limit {
            return Some(format!("Re ≈ {}: this flow is probably turbulent; 0.1 solves laminar flow only.", crate::units::num(self.re, 2).replace("e", " × 10^")));
        }
        if self.ra > RA_LIMIT {
            return Some(format!("Ra ≈ {}: this natural convection is probably turbulent; 0.1 solves laminar flow only.", crate::units::num(self.ra, 2).replace("e", " × 10^")));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples::*;
    #[test]
    fn the_examples_give_their_stated_numbers() {
        let n = numbers(&cavity(1000.0));
        assert!((n.re - 1000.0).abs() < 1e-9 && !n.through && n.turbulent().is_none(), "{n:?}");
        let n = numbers(&cylinder());
        assert!((n.re - 20.0).abs() < 1e-9 && n.through);
        // Water at 0.05 m/s in a 10 mm gap: Re = 998.2 × 0.05 × 0.01 / 1.002e-3 = 498.1.
        let n = numbers(&channel());
        assert!((n.re - 498.1).abs() < 0.1 && (n.length - 0.01).abs() < 1e-12, "{n:?}");
        let n = numbers(&heated_cavity(1e6));
        assert!((n.ra - 1e6).abs() < 1e-3 && (n.pr - 0.71).abs() < 1e-12 && n.re == 0.0 && (n.dt - 1.0).abs() < 1e-12);
        assert!((numbers(&step(100.0)).re - 100.0).abs() < 1e-9);
        // The heat sink: air at 1 m/s, length 4 mm: about 265.
        let n = numbers(&heatsink());
        assert!(n.re > 250.0 && n.re < 280.0 && n.ra == 0.0 && (n.dt - 40.0).abs() < 1e-9, "{n:?}");
    }
    #[test]
    fn fast_flows_are_called_turbulent_in_plain_words() {
        let mut m = channel();
        m.boundaries[0].kind = BcKind::Inlet { speed: 1.2, profile: Profile::Uniform };
        let n = numbers(&m);
        let w = n.turbulent().unwrap();
        assert!(w.starts_with("Re ≈ 12000:") && w.contains("probably turbulent"), "{w}");
        assert!(numbers(&heated_cavity(1e10)).turbulent().unwrap().starts_with("Ra ≈ 1 × 10^10"));
        assert_eq!(edge_length(&Geom::Poly { pts: vec![[0.0, 0.0], [2.0, 0.0], [0.0, 1.0]], bulge: vec![1.0, 0.0, 0.0] }, 0), std::f64::consts::PI);
    }
}
