//! Finding the element that holds a point, and the point's place inside it, quickly: elements are
//! binned on a grid by their bounding boxes.

use flow_mesh::Mesh;
use flow_solve::fe::geo;

pub struct Locator {
    x0: f64,
    y0: f64,
    cell: f64,
    nx: usize,
    ny: usize,
    bins: Vec<Vec<u32>>,
}

/// An element and the reference coordinates (ξ, η) of a point in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub element: usize,
    pub xi: f64,
    pub eta: f64,
}

fn element(mesh: &Mesh, e: usize) -> [[f64; 2]; 6] {
    mesh.tris[e].map(|n| mesh.nodes[n as usize])
}

impl Locator {
    pub fn new(mesh: &Mesh) -> Locator {
        let b = mesh.nodes.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, p| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])]);
        let (w, h) = ((b[2] - b[0]).max(1e-300), (b[3] - b[1]).max(1e-300));
        // About two elements per bin.
        let cell = (w * h / (mesh.tris.len().max(1) as f64 / 2.0)).sqrt().max(w.max(h) / 2000.0);
        let (nx, ny) = ((w / cell).ceil() as usize + 1, (h / cell).ceil() as usize + 1);
        let mut bins = vec![vec![]; nx * ny];
        for e in 0..mesh.tris.len() {
            let xy = element(mesh, e);
            let eb = xy.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, p| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])]);
            let (i0, j0) = (((eb[0] - b[0]) / cell) as usize, ((eb[1] - b[1]) / cell) as usize);
            let (i1, j1) = ((((eb[2] - b[0]) / cell) as usize).min(nx - 1), (((eb[3] - b[1]) / cell) as usize).min(ny - 1));
            for j in j0..=j1 {
                for i in i0..=i1 {
                    bins[j * nx + i].push(e as u32);
                }
            }
        }
        Locator { x0: b[0], y0: b[1], cell, nx, ny, bins }
    }

    /// The element holding `p`, or None when the point is outside the fluid.
    pub fn find(&self, mesh: &Mesh, p: [f64; 2]) -> Option<Place> {
        if !(p[0].is_finite() && p[1].is_finite()) {
            return None;
        }
        let (fi, fj) = ((p[0] - self.x0) / self.cell, (p[1] - self.y0) / self.cell);
        if fi < -1e-9 || fj < -1e-9 || fi > self.nx as f64 || fj > self.ny as f64 {
            return None;
        }
        let (i, j) = ((fi.max(0.0) as usize).min(self.nx - 1), (fj.max(0.0) as usize).min(self.ny - 1));
        let mut best: Option<(f64, Place)> = None;
        for &e in &self.bins[j * self.nx + i] {
            let xy = element(mesh, e as usize);
            // Start from the straight triangle, then correct for curved sides by Newton.
            let (a, b, c) = (xy[0], xy[1], xy[2]);
            let det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
            let mut xi = ((p[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (p[1] - a[1])) / det;
            let mut eta = ((b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1])) / det;
            for _ in 0..8 {
                let g = geo(&xy, xi, eta);
                let (rx, ry) = (p[0] - g.x[0], p[1] - g.x[1]);
                if rx.abs() + ry.abs() < 1e-13 * (det.abs().sqrt() + 1e-300) {
                    break;
                }
                // dξ = dN/dx · r for the linear part of the map: use the corner functions' gradients.
                xi += g.dl[1][0] * rx + g.dl[1][1] * ry;
                eta += g.dl[2][0] * rx + g.dl[2][1] * ry;
            }
            // How far outside the element the point is (0 inside).
            let out = (-xi).max(-eta).max(xi + eta - 1.0).max(0.0);
            if out <= 1e-9 {
                return Some(Place { element: e as usize, xi, eta });
            }
            if best.is_none_or(|b| out < b.0) {
                best = Some((out, Place { element: e as usize, xi, eta }));
            }
        }
        // A point a hair outside (on a wall, after rounding) belongs to the nearest element.
        best.filter(|b| b.0 < 1e-6).map(|b| b.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn points_are_found_in_straight_and_curved_elements() {
        let m = flow_mesh::mesh(&flow_core::examples::cylinder()).unwrap();
        let l = Locator::new(&m);
        for p in [[0.15, 0.2], [0.25, 0.2], [1.1, 0.205], [0.0, 0.0], [2.2, 0.41], [0.2, 0.2501], [0.2 + 0.05 * 0.6, 0.2 + 0.05 * 0.8]] {
            let at = l.find(&m, p).unwrap_or_else(|| panic!("{p:?} not found"));
            let g = geo(&element(&m, at.element), at.xi, at.eta);
            assert!((g.x[0] - p[0]).abs() < 1e-9 && (g.x[1] - p[1]).abs() < 1e-9, "{p:?} → {:?}", g.x);
        }
        // Inside the cylinder and outside the channel there is no fluid.
        assert!(l.find(&m, [0.2, 0.2]).is_none() && l.find(&m, [0.21, 0.23]).is_none() && l.find(&m, [3.0, 0.2]).is_none() && l.find(&m, [1.0, -0.01]).is_none() && l.find(&m, [f64::NAN, 0.0]).is_none());
    }
}
