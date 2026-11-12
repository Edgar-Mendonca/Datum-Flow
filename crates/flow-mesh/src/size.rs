//! The element size wanted at every place: the global size, made smaller at walls, sharp corners,
//! wakes and wherever the user asked, and growing gently away from those places. Kept on a
//! background grid so looking it up is quick.

pub struct SizeField {
    x0: f64,
    y0: f64,
    cell: f64,
    nx: usize,
    ny: usize,
    v: Vec<f64>,
    /// The smallest size anywhere.
    pub min: f64,
}

impl SizeField {
    /// A field of the global size `h` over the box, on a grid of at most about 300 cells a side.
    pub fn new(bounds: [f64; 4], h: f64, finest: f64) -> SizeField {
        let (w, t) = ((bounds[2] - bounds[0]).max(1e-300), (bounds[3] - bounds[1]).max(1e-300));
        let cell = (finest * 0.7).max(w.max(t) / 300.0);
        let nx = (w / cell).ceil() as usize + 3;
        let ny = (t / cell).ceil() as usize + 3;
        SizeField { x0: bounds[0] - cell, y0: bounds[1] - cell, cell, nx, ny, v: vec![h; nx * ny], min: h }
    }
    fn ij(&self, p: [f64; 2]) -> (usize, usize) {
        let i = (((p[0] - self.x0) / self.cell).floor().max(0.0) as usize).min(self.nx - 1);
        let j = (((p[1] - self.y0) / self.cell).floor().max(0.0) as usize).min(self.ny - 1);
        (i, j)
    }
    /// Ask for `size` at a point.
    pub fn point(&mut self, p: [f64; 2], size: f64) {
        let (i, j) = self.ij(p);
        let k = j * self.nx + i;
        self.v[k] = self.v[k].min(size);
        self.min = self.min.min(size);
    }
    /// Ask for `size` along a line.
    pub fn segment(&mut self, a: [f64; 2], b: [f64; 2], size: f64) {
        let n = ((b[0] - a[0]).hypot(b[1] - a[1]) / (self.cell * 0.5)).ceil().clamp(1.0, 1e6) as usize;
        for k in 0..=n {
            let t = k as f64 / n as f64;
            self.point([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t], size);
        }
    }
    /// Ask for `size` everywhere in a box.
    pub fn rect(&mut self, b: [f64; 4], size: f64) {
        let (i0, j0) = self.ij([b[0].min(b[2]), b[1].min(b[3])]);
        let (i1, j1) = self.ij([b[0].max(b[2]), b[1].max(b[3])]);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let k = j * self.nx + i;
                self.v[k] = self.v[k].min(size);
            }
        }
        self.min = self.min.min(size);
    }
    /// Let sizes grow away from where they were asked for: no more than `growth` per distance.
    pub fn spread(&mut self, growth: f64) {
        let (nx, ny) = (self.nx as isize, self.ny as isize);
        let (d1, d2) = (growth * self.cell, growth * self.cell * std::f64::consts::SQRT_2);
        for _ in 0..2 {
            for pass in 0..2 {
                let js: Box<dyn Iterator<Item = isize>> = if pass == 0 { Box::new(0..ny) } else { Box::new((0..ny).rev()) };
                for j in js {
                    let is: Box<dyn Iterator<Item = isize>> = if pass == 0 { Box::new(0..nx) } else { Box::new((0..nx).rev()) };
                    for i in is {
                        let s = if pass == 0 { -1 } else { 1 };
                        let mut best = self.v[(j * nx + i) as usize];
                        for (di, dj, d) in [(s, 0, d1), (0, s, d1), (s, s, d2), (-s, s, d2)] {
                            let (a, b) = (i + di, j + dj);
                            if a >= 0 && a < nx && b >= 0 && b < ny {
                                best = best.min(self.v[(b * nx + a) as usize] + d);
                            }
                        }
                        self.v[(j * nx + i) as usize] = best;
                    }
                }
            }
        }
    }
    /// The size wanted at a point (bilinear between grid cells).
    pub fn at(&self, p: [f64; 2]) -> f64 {
        let fx = ((p[0] - self.x0) / self.cell - 0.5).clamp(0.0, (self.nx - 1) as f64);
        let fy = ((p[1] - self.y0) / self.cell - 0.5).clamp(0.0, (self.ny - 1) as f64);
        let (i, j) = ((fx.floor() as usize).min(self.nx - 2), (fy.floor() as usize).min(self.ny - 2));
        let (tx, ty) = (fx - i as f64, fy - j as f64);
        let g = |a: usize, b: usize| self.v[b * self.nx + a];
        (g(i, j) * (1.0 - tx) + g(i + 1, j) * tx) * (1.0 - ty) + (g(i, j + 1) * (1.0 - tx) + g(i + 1, j + 1) * tx) * ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sizes_grow_away_from_a_refined_point_at_the_set_rate() {
        let mut f = SizeField::new([0.0, 0.0, 10.0, 4.0], 1.0, 0.1);
        f.point([2.0, 2.0], 0.1);
        f.spread(0.2);
        assert!((f.at([2.0, 2.0]) - 0.1).abs() < 0.03, "{}", f.at([2.0, 2.0]));
        // 2 away: 0.1 + 0.2 × 2 = 0.5 (the grid's diagonal steps make it a little larger).
        let s = f.at([4.0, 2.0]);
        assert!(s > 0.45 && s < 0.6, "{s}");
        let s = f.at([3.4, 3.4]);
        assert!(s > 0.45 && s < 0.62, "{s}");
        // Far away the global size is back.
        assert!((f.at([9.0, 2.0]) - 1.0).abs() < 1e-12 && f.min == 0.1);
        // Outside the box it holds the edge value instead of failing.
        assert!(f.at([-50.0, 99.0]).is_finite());
    }
}
