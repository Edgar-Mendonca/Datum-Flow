//! The element: a six-node triangle (velocity and temperature on all six nodes, pressure on the
//! three corners: Taylor–Hood), its shape functions and its quadrature. The geometry is mapped
//! with the same six nodes, so elements on arcs are curved.

/// Quadrature on the reference triangle (ξ, η, weight; weights sum to ½): seven points, exact for
/// polynomials of degree 5 (Radon 1948).
pub const QP: [(f64, f64, f64); 7] = {
    const S: f64 = 3.872_983_346_207_417; // √15
    const A: f64 = (6.0 - S) / 21.0;
    const B: f64 = (6.0 + S) / 21.0;
    const WA: f64 = (155.0 - S) / 2400.0;
    const WB: f64 = (155.0 + S) / 2400.0;
    [(1.0 / 3.0, 1.0 / 3.0, 9.0 / 80.0), (A, A, WA), (1.0 - 2.0 * A, A, WA), (A, 1.0 - 2.0 * A, WA), (B, B, WB), (1.0 - 2.0 * B, B, WB), (B, 1.0 - 2.0 * B, WB)]
};

/// Three-point Gauss rule on [0, 1] (position, weight), exact for degree 5.
pub const G3: [(f64, f64); 3] = [(0.112_701_665_379_258_3, 5.0 / 18.0), (0.5, 8.0 / 18.0), (0.887_298_334_620_741_7, 5.0 / 18.0)];

/// Reference coordinates of the six nodes.
pub const NODES_REF: [(f64, f64); 6] = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (0.5, 0.0), (0.5, 0.5), (0.0, 0.5)];

/// The six quadratic shape functions and their (ξ, η) derivatives.
pub fn shape6(xi: f64, eta: f64) -> ([f64; 6], [[f64; 2]; 6]) {
    let (l1, l2, l3) = (1.0 - xi - eta, xi, eta);
    (
        [l1 * (2.0 * l1 - 1.0), l2 * (2.0 * l2 - 1.0), l3 * (2.0 * l3 - 1.0), 4.0 * l1 * l2, 4.0 * l2 * l3, 4.0 * l3 * l1],
        [
            [-(4.0 * l1 - 1.0), -(4.0 * l1 - 1.0)],
            [4.0 * l2 - 1.0, 0.0],
            [0.0, 4.0 * l3 - 1.0],
            [4.0 * (l1 - l2), -4.0 * l2],
            [4.0 * l3, 4.0 * l2],
            [-4.0 * l3, 4.0 * (l1 - l3)],
        ],
    )
}

/// Everything about one point of one element.
pub struct Geo {
    /// Quadratic shape functions and their x, y derivatives.
    pub n: [f64; 6],
    pub dn: [[f64; 2]; 6],
    /// Linear (corner) shape functions and their x, y derivatives.
    pub l: [f64; 3],
    pub dl: [[f64; 2]; 3],
    /// det J (twice the area for a straight triangle).
    pub det: f64,
    /// The point itself.
    pub x: [f64; 2],
}

/// Shape functions and derivatives at (ξ, η) of the element with nodes `xy`.
pub fn geo(xy: &[[f64; 2]; 6], xi: f64, eta: f64) -> Geo {
    let (n, d) = shape6(xi, eta);
    let (mut j11, mut j12, mut j21, mut j22, mut x) = (0.0, 0.0, 0.0, 0.0, [0.0; 2]);
    for k in 0..6 {
        j11 += d[k][0] * xy[k][0];
        j12 += d[k][0] * xy[k][1];
        j21 += d[k][1] * xy[k][0];
        j22 += d[k][1] * xy[k][1];
        x[0] += n[k] * xy[k][0];
        x[1] += n[k] * xy[k][1];
    }
    let det = j11 * j22 - j12 * j21;
    let inv = 1.0 / det;
    let phys = |d: [f64; 2]| [(j22 * d[0] - j12 * d[1]) * inv, (-j21 * d[0] + j11 * d[1]) * inv];
    let mut dn = [[0.0; 2]; 6];
    for k in 0..6 {
        dn[k] = phys(d[k]);
    }
    Geo { n, dn, l: [1.0 - xi - eta, xi, eta], dl: [phys([-1.0, -1.0]), phys([1.0, 0.0]), phys([0.0, 1.0])], det, x }
}

/// A point on a quadratic edge [start, middle, end] at s in [0, 1]: the point, its tangent
/// (d/ds; its length is the stretch) and the three edge shape functions.
pub fn edge_geo(a: [f64; 2], m: [f64; 2], b: [f64; 2], s: f64) -> ([f64; 2], [f64; 2], [f64; 3]) {
    let n = [(1.0 - s) * (1.0 - 2.0 * s), 4.0 * s * (1.0 - s), s * (2.0 * s - 1.0)];
    let dn = [4.0 * s - 3.0, 4.0 - 8.0 * s, 4.0 * s - 1.0];
    ([n[0] * a[0] + n[1] * m[0] + n[2] * b[0], n[0] * a[1] + n[1] * m[1] + n[2] * b[1]], [dn[0] * a[0] + dn[1] * m[0] + dn[2] * b[0], dn[0] * a[1] + dn[1] * m[1] + dn[2] * b[1]], n)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_quadrature_is_exact_to_degree_five() {
        // ∫ ξ^a η^b over the reference triangle = a! b! / (a + b + 2)!
        let fact = |n: u32| (1..=n).map(|k| k as f64).product::<f64>();
        for a in 0..=5u32 {
            for b in 0..=(5 - a) {
                let got: f64 = QP.iter().map(|q| q.0.powi(a as i32) * q.1.powi(b as i32) * q.2).sum();
                let want = fact(a) * fact(b) / fact(a + b + 2);
                assert!((got - want).abs() < 1e-15, "{a} {b}: {got} {want}");
            }
        }
        assert!((G3.iter().map(|g| g.0.powi(5) * g.1).sum::<f64>() - 1.0 / 6.0).abs() < 1e-15);
    }
    #[test]
    fn shape_functions_are_one_at_their_node_and_reproduce_quadratics() {
        for (i, r) in NODES_REF.iter().enumerate() {
            let (n, _) = shape6(r.0, r.1);
            for (j, v) in n.iter().enumerate() {
                assert!((v - if i == j { 1.0 } else { 0.0 }).abs() < 1e-15);
            }
        }
        // A curved element: f = 1 + 2x − y + x² − 3xy is reproduced, with its gradient, when the
        // element is straight; on any element the functions sum to one and their gradients to zero.
        let straight = [[0.2, 0.1], [1.3, 0.4], [0.5, 1.1], [0.75, 0.25], [0.9, 0.75], [0.35, 0.6]];
        let f = |p: [f64; 2]| 1.0 + 2.0 * p[0] - p[1] + p[0] * p[0] - 3.0 * p[0] * p[1];
        let g = geo(&straight, 0.3, 0.2);
        let fx: f64 = (0..6).map(|k| g.n[k] * f(straight[k])).sum();
        let dfx: f64 = (0..6).map(|k| g.dn[k][0] * f(straight[k])).sum();
        let dfy: f64 = (0..6).map(|k| g.dn[k][1] * f(straight[k])).sum();
        assert!((fx - f(g.x)).abs() < 1e-14 && (dfx - (2.0 + 2.0 * g.x[0] - 3.0 * g.x[1])).abs() < 1e-13 && (dfy - (-1.0 - 3.0 * g.x[0])).abs() < 1e-13);
        let mut curved = straight;
        curved[4] = [1.0, 0.85];
        let g = geo(&curved, 0.3, 0.2);
        assert!((g.n.iter().sum::<f64>() - 1.0).abs() < 1e-15 && g.dn.iter().map(|d| d[0]).sum::<f64>().abs() < 1e-14 && (g.l.iter().sum::<f64>() - 1.0).abs() < 1e-15);
        // The linear functions reproduce x on a straight element.
        let g = geo(&straight, 0.3, 0.2);
        assert!(((0..3).map(|k| g.dl[k][0] * straight[k][0]).sum::<f64>() - 1.0).abs() < 1e-14);
        // Area of the straight element from the quadrature.
        let area: f64 = QP.iter().map(|q| geo(&straight, q.0, q.1).det * q.2).sum();
        assert!((area - 0.5 * ((1.3 - 0.2) * (1.1 - 0.1) - (0.5 - 0.2) * (0.4 - 0.1))).abs() < 1e-14);
    }
}
