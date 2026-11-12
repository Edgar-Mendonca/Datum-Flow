//! The Grid Convergence Index of Celik et al. (2008): from one quantity on three meshes, the
//! apparent order of accuracy, the value extrapolated to zero mesh size, and an error band for
//! the finest mesh.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Gci {
    /// Apparent order of accuracy.
    pub order: f64,
    /// The value extrapolated to zero element size.
    pub extrapolated: f64,
    /// The fine-mesh Grid Convergence Index: a relative uncertainty of the fine-mesh value.
    pub gci: f64,
    /// The three values did not fall steadily (the estimate is then cautious, not sharp).
    pub oscillating: bool,
}

/// `h` and `phi` from the finest mesh to the coarsest. `formal` is the method's order for this
/// quantity; the apparent order is kept between ½ and it, so a lucky fine pair cannot shrink the
/// band to nothing.
pub fn gci(h: [f64; 3], phi: [f64; 3], formal: f64) -> Gci {
    let (r21, r32) = (h[1] / h[0], h[2] / h[1]);
    let (e21, e32) = (phi[1] - phi[0], phi[2] - phi[1]);
    let rel = (e21 / phi[0].abs().max(1e-300)).abs();
    if e21 == 0.0 || e32 == 0.0 || !(r21 > 1.0 && r32 > 1.0) {
        // No change between meshes: nothing to extrapolate; the band is the change itself.
        return Gci { order: formal, extrapolated: phi[0], gci: 1.25 * rel.max((e32 / phi[0].abs().max(1e-300)).abs()) / (r21.powf(formal) - 1.0).max(1e-300), oscillating: e21 != e32 };
    }
    let s = (e32 / e21).signum();
    let mut p = formal;
    for _ in 0..100 {
        let q = ((r21.powf(p) - s) / (r32.powf(p) - s)).abs().ln();
        let next = ((e32 / e21).abs().ln() + q).abs() / r21.ln();
        if !next.is_finite() { break; }
        if (next - p).abs() < 1e-12 { p = next; break; }
        p = next;
    }
    let order = p;
    let used = if p.is_finite() { p.clamp(0.5, formal) } else { 0.5 };
    let rp = r21.powf(used);
    Gci { order, extrapolated: (rp * phi[0] - phi[1]) / (rp - 1.0), gci: 1.25 * rel / (rp - 1.0), oscillating: s < 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_index_matches_a_worked_example() {
        // φ(h) = 2 + 0.5 h² on meshes h = 1, 1.5, 2.25: order 2, extrapolated 2.
        let f = |h: f64| 2.0 + 0.5 * h * h;
        let g = gci([1.0, 1.5, 2.25], [f(1.0), f(1.5), f(2.25)], 3.0);
        assert!((g.order - 2.0).abs() < 1e-9 && (g.extrapolated - 2.0).abs() < 1e-9 && !g.oscillating, "{g:?}");
        // GCI = 1.25 × |φ2 − φ1| / φ1 / (r² − 1) = 1.25 × 0.625 / 2.5 / 1.25 = 0.25: here exactly
        // 1.25 × the true relative error 0.5 / 2.5.
        assert!((g.gci - 0.25).abs() < 1e-9);
        // The worked example of the paper's Table 1 (first column): N = 18000, 8000, 4500 cells in 2D,
        // φ = 6.063, 5.972, 5.863: p = 1.53, extrapolated 6.1685, GCI 2.2%.
        let h = [1.0 / 18000f64.sqrt(), 1.0 / 8000f64.sqrt(), 1.0 / 4500f64.sqrt()];
        let g = gci(h, [6.063, 5.972, 5.863], 2.0);
        assert!((g.order - 1.53).abs() < 0.01 && (g.extrapolated - 6.1685).abs() < 2e-3 && (g.gci - 0.022).abs() < 1e-3, "{g:?}");
        // Values that go up then down are flagged, and equal values do not divide by zero.
        assert!(gci([1.0, 1.5, 2.25], [1.0, 1.1, 1.05], 2.0).oscillating);
        assert!(gci([1.0, 1.5, 2.25], [1.0, 1.0, 1.0], 2.0).gci == 0.0);
    }
}
