//! Meshing the flow region: the outlines split by the local size, a constrained Delaunay
//! triangulation refined to a minimum angle and the wanted sizes (spade, MIT or Apache-2.0),
//! triangles kept where there is fluid, and six-node triangles with their midside nodes on the
//! true arcs. Every boundary edge of the mesh knows which edge of which shape it lies on, and so
//! which boundary it belongs to.

use crate::size::SizeField;
use flow_core::model::{arc, BcKind, EdgeRef, Geom, Model, Target};
use serde::Serialize;
use spade::{AngleLimit, ConstrainedDelaunayTriangulation, Point2, RefinementParameters, Triangulation};
use std::collections::HashMap;

/// One element edge on the outline of the flow region, the fluid on its left.
#[derive(Clone, Debug, Serialize)]
pub struct BEdge {
    pub shape: u32,
    pub edge: u32,
    /// The boundary it belongs to (0 = none).
    pub boundary: u32,
    /// [start, middle, end] nodes.
    pub n: [u32; 3],
}

/// One of a mesh's worst elements: its number, middle, smallest angle and neighbour size ratio.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Worst {
    pub element: usize,
    pub at: [f64; 2],
    pub angle: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Quality {
    pub elements: usize,
    pub nodes: usize,
    /// Unknowns of a flow solve on this mesh (two velocities per node, one pressure per corner).
    pub unknowns: usize,
    /// Smallest angle of any triangle (degrees).
    pub min_angle: f64,
    /// Share of triangles with every angle over 30°.
    pub good: f64,
    /// How many elements have their smallest angle over 30°, 20–30°, 10–20° and under 10°.
    pub angle_bands: [usize; 4],
    /// Largest ratio of the sizes of two neighbouring elements (1 = equal).
    pub max_ratio: f64,
    /// Edge lengths: smallest, mean, largest (m).
    pub edges: [f64; 3],
    /// The five worst elements by angle, with where they are.
    pub worst: Vec<Worst>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Mesh {
    /// Corner nodes first (`nv` of them), then the midside nodes.
    pub nodes: Vec<[f64; 2]>,
    pub nv: usize,
    /// Corners counter-clockwise, then the midside nodes of edges 0–1, 1–2, 2–0.
    pub tris: Vec<[u32; 6]>,
    pub edges: Vec<BEdge>,
    /// The global element size used (m).
    pub size: f64,
    /// Smallest angle of every element (degrees), for colouring the mesh by quality.
    pub angle: Vec<f32>,
    pub quality: Quality,
}

/// A piece of the outline between two inserted points: on which shape edge, and its arc if curved.
#[derive(Clone, Copy)]
struct Seg {
    shape: u32,
    edge: u32,
    a: [f64; 2],
    b: [f64; 2],
    arc: Option<([f64; 2], f64)>,
}

/// The largest number of triangles a mesh may have.
pub const MAX_ELEMENTS: usize = 200_000;

fn dist_seg(p: [f64; 2], s: &Seg) -> f64 {
    if let Some((c, r)) = s.arc {
        let d = ((p[0] - c[0]).hypot(p[1] - c[1]) - r).abs();
        let chord = (s.b[0] - s.a[0]).hypot(s.b[1] - s.a[1]);
        let to_ends = (p[0] - s.a[0]).hypot(p[1] - s.a[1]).max((p[0] - s.b[0]).hypot(p[1] - s.b[1]));
        return if to_ends <= chord * 1.0001 + 1e-15 { d } else { f64::INFINITY };
    }
    let (dx, dy) = (s.b[0] - s.a[0], s.b[1] - s.a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((p[0] - s.a[0]) * dx + (p[1] - s.a[1]) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    (s.a[0] + t * dx - p[0]).hypot(s.a[1] + t * dy - p[1])
}

pub(crate) fn inside(poly: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut c = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            c = !c;
        }
        j = i;
    }
    c
}

/// The automatic element size: about 2 000 triangles over the region, and at least eight
/// elements across its smaller side.
pub fn auto_size(m: &Model) -> f64 {
    let area: f64 = m.shapes.iter().map(|s| if s.hole { -s.geom.area().abs() } else { s.geom.area().abs() }).sum::<f64>().max(0.0);
    let by_area = if area > 0.0 { (area / (0.43 * 2000.0)).sqrt() } else { f64::INFINITY };
    m.bounds().map(|b| ((b[2] - b[0]).min(b[3] - b[1]) / 8.0).min(by_area)).filter(|h| h.is_finite() && *h > 0.0).unwrap_or(0.01)
}

/// The global element size a model will be meshed with.
pub fn global_size(m: &Model) -> f64 {
    if m.mesh.size > 0.0 { m.mesh.size } else { auto_size(m) }
}

/// A point on edge `i` of a shape at fraction `t` of its length, with the arc if it has one.
fn edge_at(g: &Geom, i: usize, t: f64) -> [f64; 2] {
    match g {
        Geom::Circle { c, r } => { let a = t * std::f64::consts::TAU; [c[0] + r * a.cos(), c[1] + r * a.sin()] }
        Geom::Poly { pts, bulge } => {
            let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
            let b = bulge.get(i).copied().unwrap_or(0.0);
            if b == 0.0 { return [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]; }
            let (cen, r, a0, th) = arc(p, q, b);
            let a = a0 + th * t;
            [cen[0] + r * a.cos(), cen[1] + r * a.sin()]
        }
    }
}
fn edge_arc(g: &Geom, i: usize) -> Option<([f64; 2], f64, f64)> {
    match g {
        Geom::Circle { c, r } => Some((*c, *r, std::f64::consts::TAU)),
        Geom::Poly { pts, bulge } => {
            let b = bulge.get(i).copied().unwrap_or(0.0);
            if b == 0.0 { return None; }
            let (cen, r, _, th) = arc(pts[i], pts[(i + 1) % pts.len()], b);
            Some((cen, r, th.abs()))
        }
    }
}

/// Points along edge `i`, spaced by the size wanted there, with the ends exactly as drawn.
fn edge_points(g: &Geom, i: usize, f: &SizeField) -> Vec<[f64; 2]> {
    let len = flow_core::numbers::edge_length(g, i);
    let steps = ((len / (f.min * 0.25)).ceil() as usize).clamp(8, 40_000);
    // The running count of elements along the edge: ∫ ds / size.
    let mut cum = Vec::with_capacity(steps + 1);
    cum.push(0.0);
    for k in 0..steps {
        let t = (k as f64 + 0.5) / steps as f64;
        cum.push(cum[k] + len / steps as f64 / f.at(edge_at(g, i, t)).max(1e-300));
    }
    let total = cum[steps];
    // At least 32 pieces per full turn, so holes and fillets stay round.
    let least = edge_arc(g, i).map(|a| ((a.2 / std::f64::consts::TAU * 32.0).ceil() as usize).max(2)).unwrap_or(1);
    let n = (total.round() as usize).clamp(least, 20_000);
    let mut out = Vec::with_capacity(n + 1);
    let mut k = 0;
    for j in 0..=n {
        let want = total * j as f64 / n as f64;
        while k + 1 < steps && cum[k + 1] < want { k += 1; }
        let t = (k as f64 + ((want - cum[k]) / (cum[k + 1] - cum[k]).max(1e-300)).clamp(0.0, 1.0)) / steps as f64;
        out.push(edge_at(g, i, t));
    }
    let (a, b) = g.edge_ends(i);
    out[0] = a;
    out[n] = b;
    out
}

/// Where the mesh should be finer than the global size `h`: asked-for places, walls, sharp
/// corners that stick into the flow, corners where a moving wall meets a fixed one, and wakes.
fn size_field(m: &Model, h: f64, bounds: [f64; 4]) -> SizeField {
    let ms = &m.mesh;
    let finest = ms.refine.iter().map(|r| r.size).fold(h * ms.wall.min(ms.corner).min(ms.wake).min(1.0), f64::min).max(h * 1e-3);
    let mut f = SizeField::new(bounds, h, finest);
    let along = |f: &mut SizeField, g: &Geom, e: usize, size: f64| {
        let n = if g.straight(e) { 1 } else { 64 };
        for k in 0..n {
            f.segment(edge_at(g, e, k as f64 / n as f64), edge_at(g, e, (k + 1) as f64 / n as f64), size);
        }
    };
    for r in &ms.refine {
        let size = r.size.max(h * 1e-3);
        match r.at {
            Target::Edge { shape, edge } => if let Some(s) = m.shape(shape).filter(|s| (edge as usize) < s.geom.edges()) { along(&mut f, &s.geom, edge as usize, size) },
            Target::Point { x, y } => f.point([x, y], size),
            Target::Box { x0, y0, x1, y1 } => f.rect([x0, y0, x1, y1], size),
        }
    }
    let wall_u = |e: EdgeRef| m.boundary_of(e).and_then(|b| if let BcKind::Wall { u } = b.kind { Some(u) } else { None });
    for s in &m.shapes {
        for e in 0..s.geom.edges() {
            if ms.wall < 1.0 && wall_u(EdgeRef { shape: s.id, edge: e as u32 }).is_some() {
                along(&mut f, &s.geom, e, h * ms.wall);
            }
        }
        if let Geom::Poly { pts, .. } = &s.geom {
            let n = pts.len();
            let turn = s.geom.area().signum() * if s.hole { -1.0 } else { 1.0 };
            for i in 0..n {
                let (a, p, b) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
                let (u, v) = ([p[0] - a[0], p[1] - a[1]], [b[0] - p[0], b[1] - p[1]]);
                let cross = (u[0] * v[1] - u[1] * v[0]) / (u[0].hypot(u[1]) * v[0].hypot(v[1])).max(1e-300);
                // The corner sticks into the fluid (a step's edge, a fin's corner).
                let sharp = cross * turn < -0.34;
                // A moving wall meets a wall moving differently (the lid's corners).
                let (w0, w1) = (wall_u(EdgeRef { shape: s.id, edge: ((i + n - 1) % n) as u32 }), wall_u(EdgeRef { shape: s.id, edge: i as u32 }));
                let lid = matches!((w0, w1), (Some(x), Some(y)) if x != y);
                if ms.corner < 1.0 && (sharp || lid) {
                    f.point(p, h * ms.corner);
                }
            }
        }
    }
    // Wakes: behind each body, along the line from the inlets to the outlets.
    let centre = |want_inlet: bool| -> Option<[f64; 2]> {
        let pts: Vec<[f64; 2]> = m.boundaries.iter().filter(|b| matches!(b.kind, BcKind::Inlet { .. }) == want_inlet && matches!(b.kind, BcKind::Inlet { .. } | BcKind::Outlet { .. }))
            .flat_map(|b| b.edges.iter()).filter_map(|e| m.shape(e.shape).filter(|s| (e.edge as usize) < s.geom.edges()).map(|s| edge_at(&s.geom, e.edge as usize, 0.5))).collect();
        if pts.is_empty() { None } else { Some([pts.iter().map(|p| p[0]).sum::<f64>() / pts.len() as f64, pts.iter().map(|p| p[1]).sum::<f64>() / pts.len() as f64]) }
    };
    if let (true, Some(a), Some(b)) = (ms.wake < 1.0, centre(true), centre(false)) {
        let d = [b[0] - a[0], b[1] - a[1]];
        for s in m.shapes.iter().filter(|s| s.hole) {
            let bb = s.geom.bounds();
            let (w, t) = (bb[2] - bb[0], bb[3] - bb[1]);
            let reach = 5.0 * w.max(t);
            let mut r = [bb[0] - 0.25 * w, bb[1] - 0.25 * t, bb[2] + 0.25 * w, bb[3] + 0.25 * t];
            if d[0].abs() >= d[1].abs() {
                if d[0] > 0.0 { r[2] += reach } else { r[0] -= reach }
            } else if d[1] > 0.0 { r[3] += reach } else { r[1] -= reach }
            f.rect([r[0].max(bounds[0]), r[1].max(bounds[1]), r[2].min(bounds[2]), r[3].min(bounds[3])], h * ms.wake);
        }
    }
    f.spread(ms.growth);
    f
}

/// Mesh the model's flow region.
pub fn mesh(m: &Model) -> Result<Mesh, String> {
    m.check_geometry()?;
    let bounds = m.bounds().ok_or("The flow region is empty.")?;
    let diag = (bounds[2] - bounds[0]).hypot(bounds[3] - bounds[1]);
    let h = global_size(m);
    if !(h.is_finite() && h > diag * 1e-5) {
        return Err("The element size is too small for this drawing. Use a larger size, and refine only where it matters.".into());
    }
    let area: f64 = m.shapes.iter().filter(|s| !s.hole).map(|s| s.geom.area().abs()).sum();
    if area / (h * h * 0.43) > MAX_ELEMENTS as f64 {
        return Err(format!("The element size is too small for this region: it would make over {MAX_ELEMENTS} triangles. Use a larger size, and refine only where it matters."));
    }
    let field = size_field(m, h, bounds);
    let mut cdt: ConstrainedDelaunayTriangulation<Point2<f64>> = ConstrainedDelaunayTriangulation::new();
    let mut segs: Vec<Seg> = vec![];
    let ins = |cdt: &mut ConstrainedDelaunayTriangulation<Point2<f64>>, p: [f64; 2]| cdt.insert(Point2::new(p[0], p[1])).map_err(|_| "A point of the outline could not be meshed: check the drawing for values that are not numbers.".to_string());
    for s in &m.shapes {
        for e in 0..s.geom.edges() {
            let pts = edge_points(&s.geom, e, &field);
            let arc = edge_arc(&s.geom, e).map(|a| (a.0, a.1));
            let mut prev = ins(&mut cdt, pts[0])?;
            for w in pts.windows(2) {
                let next = ins(&mut cdt, w[1])?;
                if prev != next {
                    cdt.add_constraint_and_split(prev, next, |p| p);
                }
                segs.push(Seg { shape: s.id, edge: e as u32, a: w[0], b: w[1], arc });
                prev = next;
            }
            if segs.len() > 4 * MAX_ELEMENTS {
                return Err("The outline needs too many points at this element size. Use a larger size.".into());
            }
        }
    }
    // A frame round everything, so the hull is not made of near-collinear points of a sloping
    // edge (their slivers can never be refined); the triangles between frame and region are dropped.
    let pad = 2.0 * h;
    for p in [[bounds[0] - pad, bounds[1] - pad], [bounds[2] + pad, bounds[1] - pad], [bounds[2] + pad, bounds[3] + pad], [bounds[0] - pad, bounds[3] + pad]] {
        ins(&mut cdt, p)?;
    }
    let max_area = h * h * 3f64.sqrt() / 4.0 * 1.3;
    let hmin = field.min;
    let params = || RefinementParameters::<f64>::new()
        .with_angle_limit(AngleLimit::from_deg(m.mesh.min_angle.clamp(5.0, 33.0)))
        .with_max_allowed_area(max_area)
        .with_min_required_area(hmin * hmin * 1e-4)
        .with_max_additional_vertices(2 * MAX_ELEMENTS)
        .exclude_outer_faces(false);
    let mut res = cdt.refine(params());
    let fine = (hmin / 4.0).min(diag / 400.0).max(diag * 1e-6);
    let solid_out: Vec<Vec<[f64; 2]>> = m.shapes.iter().filter(|s| !s.hole).map(|s| s.geom.outline(fine, 64).0).collect();
    let holes_out: Vec<Vec<[f64; 2]>> = m.shapes.iter().filter(|s| s.hole).map(|s| s.geom.outline(fine, 64).0).collect();
    let fluid = |c: [f64; 2]| solid_out.iter().any(|o| inside(o, c)) && !holes_out.iter().any(|o| inside(o, c));
    // Triangles too large for the size wanted at their centre get a point there, and the mesh is
    // refined again for shape, until all fit.
    if hmin < h {
        for _ in 0..60 {
            let mut add = vec![];
            for f in cdt.inner_faces() {
                let p = f.vertices().map(|x| { let q = x.position(); [q.x, q.y] });
                let l = (0..3).map(|i| (p[(i + 1) % 3][0] - p[i][0]).hypot(p[(i + 1) % 3][1] - p[i][1])).fold(0.0, f64::max);
                if l <= 1.3 * hmin {
                    continue;
                }
                let c = [(p[0][0] + p[1][0] + p[2][0]) / 3.0, (p[0][1] + p[1][1] + p[2][1]) / 3.0];
                if l > 1.3 * field.at(c) && fluid(c) {
                    add.push(c);
                }
            }
            if add.is_empty() {
                break;
            }
            if cdt.num_vertices() + add.len() > MAX_ELEMENTS {
                return Err(format!("The local element sizes would make over {MAX_ELEMENTS} triangles. Use larger sizes or less refinement."));
            }
            for p in add {
                ins(&mut cdt, p)?;
            }
            res = cdt.refine(params());
            if !res.refinement_complete {
                break;
            }
        }
    }
    // Keep the triangles that are fluid (checked at the centroid; every face is well shaped).
    let mut map = vec![u32::MAX; cdt.num_vertices()];
    let mut nodes: Vec<[f64; 2]> = vec![];
    let mut corners: Vec<[u32; 3]> = vec![];
    for f in cdt.inner_faces() {
        let v = f.vertices();
        let p = v.map(|x| { let q = x.position(); [q.x, q.y] });
        let a2 = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
        if a2.abs() < 1e-12 * h * h {
            continue;
        }
        let c = [(p[0][0] + p[1][0] + p[2][0]) / 3.0, (p[0][1] + p[1][1] + p[2][1]) / 3.0];
        if !fluid(c) {
            continue;
        }
        let mut t = [0u32; 3];
        for (k, x) in v.iter().enumerate() {
            let i = x.fix().index();
            if map[i] == u32::MAX {
                map[i] = nodes.len() as u32;
                nodes.push(p[k]);
            }
            t[k] = map[i];
        }
        if a2 < 0.0 { t.swap(1, 2); }
        corners.push(t);
    }
    if corners.is_empty() {
        return Err("The mesh has no triangles: check that the bodies cut out do not cover the whole flow region.".into());
    }
    if corners.len() > MAX_ELEMENTS {
        return Err(format!("The mesh would have over {MAX_ELEMENTS} triangles. Use a larger element size or less refinement."));
    }
    if !res.refinement_complete {
        return Err("The mesh needed more triangles than allowed. Use a larger element size or a smaller minimum angle.".into());
    }
    // The fluid must be in one piece: join triangles across shared edges and count the groups.
    let mut owner: HashMap<(u32, u32), (u32, usize)> = HashMap::with_capacity(corners.len() * 2);
    let mut group: Vec<usize> = (0..corners.len()).collect();
    fn find(g: &mut [usize], mut i: usize) -> usize {
        while g[i] != i { g[i] = g[g[i]]; i = g[i]; }
        i
    }
    let mut pairs: Vec<(usize, usize)> = vec![];
    for (k, t) in corners.iter().enumerate() {
        for j in 0..3 {
            let (a, b) = (t[j], t[(j + 1) % 3]);
            let e = owner.entry((a.min(b), a.max(b))).or_insert((0, k));
            e.0 += 1;
            if e.0 == 2 {
                pairs.push((e.1, k));
                let (x, y) = (find(&mut group, e.1), find(&mut group, k));
                group[x] = y;
            }
        }
    }
    let pieces = (0..corners.len()).filter(|&i| find(&mut group, i) == i).count();
    if pieces > 1 {
        return Err(format!("The flow region is in {pieces} separate pieces. Join them (shapes must overlap or touch along an edge) or delete the ones not needed."));
    }
    // Boundary edges: element edges used once, tagged with the outline piece they lie on.
    let cell = (h / 2.0).max(diag * 1e-6);
    let key = |p: [f64; 2]| ((p[0] / cell).floor() as i64, (p[1] / cell).floor() as i64);
    let mut bins: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, s) in segs.iter().enumerate() {
        let (k0, k1) = (key(s.a), key(s.b));
        for x in k0.0.min(k1.0) - 1..=k0.0.max(k1.0) + 1 {
            for y in k0.1.min(k1.1) - 1..=k0.1.max(k1.1) + 1 {
                bins.entry((x, y)).or_default().push(i);
            }
        }
    }
    let nearest = |p: [f64; 2]| -> Option<usize> {
        bins.get(&key(p))?.iter().map(|&i| (i, dist_seg(p, &segs[i]))).filter(|x| x.1 <= diag * 1e-6 + cell * 0.05).min_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0)
    };
    let mut bedges: Vec<(u32, u32, Option<usize>)> = vec![];
    for (&(a, b), &(c, k)) in &owner {
        if c != 1 {
            continue;
        }
        // Keep the element's direction (counter-clockwise round the fluid).
        let t = &corners[k];
        let (a, b) = if (0..3).any(|j| t[j] == a && t[(j + 1) % 3] == b) { (a, b) } else { (b, a) };
        let mid = [(nodes[a as usize][0] + nodes[b as usize][0]) / 2.0, (nodes[a as usize][1] + nodes[b as usize][1]) / 2.0];
        bedges.push((a, b, nearest(mid).or_else(|| nearest(nodes[a as usize]))));
    }
    bedges.sort_by_key(|x| (x.2.map(|s| (segs[s].shape, segs[s].edge)), x.0, x.1));
    // Points that refinement put on an arc's chord go onto the arc itself.
    let before = nodes.clone();
    for &(a, b, s) in &bedges {
        if let Some((c, r)) = s.and_then(|s| segs[s].arc) {
            for n in [a, b] {
                let p = nodes[n as usize];
                let d = (p[0] - c[0]).hypot(p[1] - c[1]);
                if d > 0.0 && (d - r).abs() > 1e-12 * r {
                    nodes[n as usize] = [c[0] + (p[0] - c[0]) * r / d, c[1] + (p[1] - c[1]) * r / d];
                }
            }
        }
    }
    let area2 = |t: &[u32; 3], nodes: &[[f64; 2]]| { let p = [nodes[t[0] as usize], nodes[t[1] as usize], nodes[t[2] as usize]]; (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]) };
    for _ in 0..10 {
        let folded: Vec<u32> = corners.iter().filter(|t| area2(t, &nodes) <= 0.0).flat_map(|t| t.iter().copied()).collect();
        if folded.is_empty() {
            break;
        }
        for n in folded {
            nodes[n as usize] = before[n as usize];
        }
    }
    // Midside nodes, on the arc where the edge lies on one.
    let nv = nodes.len();
    let arc_of: HashMap<(u32, u32), ([f64; 2], f64)> = bedges.iter().filter_map(|&(a, b, s)| s.and_then(|s| segs[s].arc).map(|x| ((a.min(b), a.max(b)), x))).collect();
    let mut mids: HashMap<(u32, u32), u32> = HashMap::with_capacity(corners.len() * 2);
    let mut tris: Vec<[u32; 6]> = Vec::with_capacity(corners.len());
    for t in &corners {
        let mut six = [t[0], t[1], t[2], 0, 0, 0];
        for j in 0..3 {
            let (a, b) = (t[j], t[(j + 1) % 3]);
            six[3 + j] = *mids.entry((a.min(b), a.max(b))).or_insert_with(|| {
                let (p, q) = (nodes[a as usize], nodes[b as usize]);
                let mut c = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
                if let Some((o, r)) = arc_of.get(&(a.min(b), a.max(b))) {
                    let d = (c[0] - o[0]).hypot(c[1] - o[1]);
                    if d > 0.0 {
                        c = [o[0] + (c[0] - o[0]) * r / d, o[1] + (c[1] - o[1]) * r / d];
                    }
                }
                nodes.push(c);
                (nodes.len() - 1) as u32
            });
        }
        tris.push(six);
    }
    let edges: Vec<BEdge> = bedges.iter().map(|&(a, b, s)| {
        let (shape, edge) = s.map(|s| (segs[s].shape, segs[s].edge)).unwrap_or((0, 0));
        BEdge { shape, edge, boundary: m.boundary_of(EdgeRef { shape, edge }).map(|x| x.id).unwrap_or(0), n: [a, mids[&(a.min(b), a.max(b))], b] }
    }).collect();
    let (quality, angle) = quality(&nodes, nv, &tris, &pairs);
    Ok(Mesh { nodes, nv, tris, edges, size: h, angle, quality })
}

fn quality(nodes: &[[f64; 2]], nv: usize, tris: &[[u32; 6]], pairs: &[(usize, usize)]) -> (Quality, Vec<f32>) {
    let mut q = Quality { elements: tris.len(), nodes: nodes.len(), unknowns: 2 * nodes.len() + nv, min_angle: 180.0, max_ratio: 1.0, edges: [f64::INFINITY, 0.0, 0.0], ..Default::default() };
    let mut angles = Vec::with_capacity(tris.len());
    let mut areas = Vec::with_capacity(tris.len());
    let (mut good, mut sum) = (0usize, 0.0);
    for (k, t) in tris.iter().enumerate() {
        let p = [nodes[t[0] as usize], nodes[t[1] as usize], nodes[t[2] as usize]];
        let l: Vec<f64> = (0..3).map(|i| (p[(i + 1) % 3][0] - p[i][0]).hypot(p[(i + 1) % 3][1] - p[i][1])).collect();
        let area = 0.5 * ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1])).abs();
        // The smallest angle faces the shortest side: sin = 2 area / (product of the other two).
        let (lo, hi) = (l.iter().cloned().fold(f64::INFINITY, f64::min), l.iter().cloned().fold(0.0, f64::max));
        let mid = l[0] + l[1] + l[2] - lo - hi;
        let a = (2.0 * area / (mid * hi).max(1e-300)).clamp(0.0, 1.0).asin().to_degrees();
        angles.push(a as f32);
        areas.push(area);
        q.min_angle = q.min_angle.min(a);
        good += (a > 30.0) as usize;
        q.angle_bands[if a > 30.0 { 0 } else if a > 20.0 { 1 } else if a > 10.0 { 2 } else { 3 }] += 1;
        q.edges[0] = q.edges[0].min(lo);
        q.edges[2] = q.edges[2].max(hi);
        sum += l[0] + l[1] + l[2];
        let w = Worst { element: k, at: [(p[0][0] + p[1][0] + p[2][0]) / 3.0, (p[0][1] + p[1][1] + p[2][1]) / 3.0], angle: a };
        let at = q.worst.iter().position(|x| x.angle > a).unwrap_or(q.worst.len());
        if at < 5 {
            q.worst.insert(at, w);
            q.worst.truncate(5);
        }
    }
    for &(a, b) in pairs {
        let r = (areas[a] / areas[b].max(1e-300)).sqrt();
        q.max_ratio = q.max_ratio.max(r.max(1.0 / r.max(1e-300)));
    }
    q.good = good as f64 / tris.len().max(1) as f64;
    q.edges[1] = sum / (3 * tris.len()).max(1) as f64;
    (q, angles)
}

impl Mesh {
    /// Area of the meshed region from its straight-sided triangles (m²).
    pub fn area(&self) -> f64 {
        self.tris.iter().map(|t| { let p = [self.nodes[t[0] as usize], self.nodes[t[1] as usize], self.nodes[t[2] as usize]]; 0.5 * ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1])) }).sum()
    }
    /// A regular mesh of a rectangle, n × m squares each cut in two (benchmarks and tests);
    /// every boundary edge is tagged shape 1, edge 0 bottom, 1 right, 2 top, 3 left, and given
    /// the boundary id from `ids` in that order.
    pub fn grid(x0: f64, y0: f64, x1: f64, y1: f64, n: usize, m: usize, ids: [u32; 4]) -> Mesh {
        let (n, m) = (n.max(1), m.max(1));
        let mut nodes = vec![];
        for j in 0..=m {
            for i in 0..=n {
                nodes.push([x0 + (x1 - x0) * i as f64 / n as f64, y0 + (y1 - y0) * j as f64 / m as f64]);
            }
        }
        let nv = nodes.len();
        let v = |i: usize, j: usize| (j * (n + 1) + i) as u32;
        let mut corners = vec![];
        for j in 0..m {
            for i in 0..n {
                // Alternate the diagonal so the mesh has no preferred direction.
                if (i + j) % 2 == 0 {
                    corners.push([v(i, j), v(i + 1, j), v(i + 1, j + 1)]);
                    corners.push([v(i, j), v(i + 1, j + 1), v(i, j + 1)]);
                } else {
                    corners.push([v(i, j), v(i + 1, j), v(i, j + 1)]);
                    corners.push([v(i + 1, j), v(i + 1, j + 1), v(i, j + 1)]);
                }
            }
        }
        let mut mids: HashMap<(u32, u32), u32> = HashMap::new();
        let mut count: HashMap<(u32, u32), (u32, u32, u32)> = HashMap::new();
        let mut tris = vec![];
        let mut pairs_owner: HashMap<(u32, u32), usize> = HashMap::new();
        let mut pairs = vec![];
        for (k, t) in corners.iter().enumerate() {
            let mut six = [t[0], t[1], t[2], 0, 0, 0];
            for j in 0..3 {
                let (a, b) = (t[j], t[(j + 1) % 3]);
                let key = (a.min(b), a.max(b));
                six[3 + j] = *mids.entry(key).or_insert_with(|| {
                    let (p, q) = (nodes[a as usize], nodes[b as usize]);
                    nodes.push([(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0]);
                    (nodes.len() - 1) as u32
                });
                let c = count.entry(key).or_insert((0, a, b));
                c.0 += 1;
                if let Some(o) = pairs_owner.insert(key, k) { pairs.push((o, k)); }
            }
            tris.push(six);
        }
        let tol = 1e-9 * ((x1 - x0).abs() + (y1 - y0).abs());
        let mut edges: Vec<BEdge> = count.iter().filter(|(_, c)| c.0 == 1).map(|(key, &(_, a, b))| {
            let (p, q) = (nodes[a as usize], nodes[b as usize]);
            let edge = if (p[1] - y0).abs() < tol && (q[1] - y0).abs() < tol { 0 } else if (p[0] - x1).abs() < tol && (q[0] - x1).abs() < tol { 1 } else if (p[1] - y1).abs() < tol && (q[1] - y1).abs() < tol { 2 } else { 3 };
            BEdge { shape: 1, edge, boundary: ids[edge as usize], n: [a, mids[key], b] }
        }).collect();
        edges.sort_by_key(|e| (e.edge, e.n[0]));
        let (quality, angle) = quality(&nodes, nv, &tris, &pairs);
        Mesh { nodes, nv, tris, edges, size: ((x1 - x0) / n as f64).abs(), angle, quality }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::examples::*;
    use flow_core::model::*;

    fn sound(m: &Mesh) {
        assert!(m.nv > 0 && m.nodes.len() > m.nv && m.angle.len() == m.tris.len());
        for t in &m.tris {
            assert!(t[..3].iter().all(|n| (*n as usize) < m.nv) && t[3..].iter().all(|n| (*n as usize) >= m.nv && (*n as usize) < m.nodes.len()));
            let p = [m.nodes[t[0] as usize], m.nodes[t[1] as usize], m.nodes[t[2] as usize]];
            assert!((p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]) > 0.0, "a triangle is folded or clockwise");
        }
        // The boundary is closed: every node on it starts one edge and ends one.
        let mut deg: HashMap<u32, i32> = HashMap::new();
        for e in &m.edges {
            *deg.entry(e.n[0]).or_default() += 1;
            *deg.entry(e.n[2]).or_default() -= 1;
        }
        assert!(deg.values().all(|d| *d == 0), "the boundary is not closed");
    }

    #[test]
    fn a_square_meshes_whole_with_good_triangles() {
        let m = mesh(&cavity(100.0)).unwrap();
        sound(&m);
        assert!((m.area() - 1.0).abs() < 1e-9, "{}", m.area());
        assert!(m.quality.min_angle > 24.0 && m.quality.good > 0.8, "{:?}", m.quality);
        assert!(m.edges.iter().all(|e| e.boundary == 2 || e.boundary == 3) && m.edges.iter().any(|e| e.boundary == 2));
        // The lid's edges run right to left (counter-clockwise round the fluid) along y = 1.
        assert!(m.edges.iter().filter(|e| e.boundary == 2).all(|e| m.nodes[e.n[0] as usize][1] == 1.0 && m.nodes[e.n[0] as usize][0] > m.nodes[e.n[2] as usize][0]));
        assert_eq!(m.quality.unknowns, 2 * m.nodes.len() + m.nv);
    }

    #[test]
    fn the_cylinder_is_cut_out_round_and_refined() {
        let model = cylinder();
        let m = mesh(&model).unwrap();
        sound(&m);
        let want = 2.2 * 0.41 - std::f64::consts::PI * 0.05 * 0.05;
        assert!((m.area() - want).abs() < 2e-4 * want, "{} {want}", m.area());
        // Every node on the cylinder, midside nodes too, is on the circle.
        let on: Vec<&BEdge> = m.edges.iter().filter(|e| e.boundary == 6).collect();
        assert!(on.len() >= 60, "{}", on.len());
        for e in &on {
            for n in e.n {
                let p = m.nodes[n as usize];
                assert!(((p[0] - 0.2).hypot(p[1] - 0.2) - 0.05).abs() < 1e-12);
            }
        }
        // Elements are small at the cylinder and large far from it.
        let len = |e: &BEdge| { let (a, b) = (m.nodes[e.n[0] as usize], m.nodes[e.n[2] as usize]); (a[0] - b[0]).hypot(a[1] - b[1]) };
        let at_cyl = on.iter().map(|e| len(e)).fold(0.0, f64::max);
        let at_out = m.edges.iter().filter(|e| e.boundary == 4).map(len).fold(0.0, f64::max);
        assert!(at_cyl < 0.0065 && at_out > 0.015, "{at_cyl} {at_out}");
        assert!(m.quality.max_ratio < 6.0 && m.quality.min_angle > 20.0, "{:?}", m.quality);
    }

    #[test]
    fn every_example_meshes_with_every_edge_on_a_boundary() {
        for (id, title, _) in EXAMPLES {
            if id == "empty" { continue; }
            let model = example(id).unwrap();
            let t = std::time::Instant::now();
            let m = mesh(&model).unwrap_or_else(|e| panic!("{title}: {e}"));
            sound(&m);
            assert!(m.edges.iter().all(|e| e.boundary != 0), "{title}");
            assert!(m.quality.min_angle > 15.0 && m.quality.elements > 300 && m.quality.elements < 40_000, "{title}: {:?}", m.quality);
            let area: f64 = model.shapes.iter().map(|s| if s.hole { -s.geom.area().abs() } else { s.geom.area().abs() }).sum();
            assert!((m.area() - area).abs() < 1e-3 * area, "{title}");
            eprintln!("{title}: {} elements, {} unknowns, min angle {:.1}°, ratio {:.2}, {:.0} ms", m.quality.elements, m.quality.unknowns, m.quality.min_angle, m.quality.max_ratio, t.elapsed().as_secs_f64() * 1e3);
        }
    }

    #[test]
    fn corners_and_walls_are_refined() {
        let model = step(100.0);
        let m = mesh(&model).unwrap();
        // The smallest elements sit at the step's corner (0, S).
        let near = m.tris.iter().filter(|t| { let p = m.nodes[t[0] as usize]; (p[0]).hypot(p[1] - 0.0049) < 0.0006 }).count();
        let far = m.tris.iter().filter(|t| { let p = m.nodes[t[0] as usize]; (p[0] - 0.1).hypot(p[1] - 0.005) < 0.0006 }).count();
        assert!(near > 3 * far.max(1), "{near} {far}");
        // No refinement at all gives far fewer elements.
        let mut plain = model.clone();
        plain.mesh = MeshSettings { size: model.mesh.size, wall: 1.0, corner: 1.0, wake: 1.0, ..Default::default() };
        assert!(mesh(&plain).unwrap().tris.len() * 3 < m.tris.len() * 2);
    }

    #[test]
    fn bad_drawings_are_reported_not_panicked_on() {
        let mut m = Model::default();
        assert!(mesh(&m).unwrap_err().contains("nothing to mesh"));
        // Two regions that do not touch.
        m.shapes = vec![Shape { id: 1, name: "A".into(), geom: Geom::rect(0.0, 0.0, 1.0, 1.0), ..Default::default() }, Shape { id: 2, name: "B".into(), geom: Geom::rect(2.0, 0.0, 3.0, 1.0), ..Default::default() }];
        assert!(mesh(&m).unwrap_err().contains("2 separate pieces"));
        // A body that covers everything.
        m.shapes[1] = Shape { id: 2, name: "B".into(), hole: true, geom: Geom::rect(-1.0, -1.0, 3.0, 3.0), ..Default::default() };
        assert!(mesh(&m).unwrap_err().contains("no triangles"));
        // A size far too small.
        m.shapes.truncate(1);
        m.mesh.size = 1e-5;
        assert!(mesh(&m).unwrap_err().contains("too small"));
        // Overlapping regions join into one; a body crossing the outline is cut cleanly.
        m.mesh.size = 0.0;
        m.shapes.push(Shape { id: 2, name: "B".into(), geom: Geom::rect(0.5, 0.5, 2.0, 0.8), ..Default::default() });
        m.shapes.push(Shape { id: 3, name: "C".into(), hole: true, geom: Geom::Circle { c: [1.0, 0.2], r: 0.3 }, ..Default::default() });
        let g = mesh(&m).unwrap();
        sound(&g);
        assert!(g.area() > 1.0 && g.area() < 1.45);
        // A broken outline.
        m.shapes[0].geom = Geom::poly(&[[0.0, 0.0], [2.0, 2.0], [2.0, 0.0], [0.0, 1.0]]);
        assert!(mesh(&m).unwrap_err().contains("cross"));
    }

    #[test]
    fn the_regular_grid_is_tagged_and_sound() {
        let g = Mesh::grid(0.0, 0.0, 2.0, 1.0, 4, 2, [10, 11, 12, 13]);
        sound(&g);
        assert!(g.tris.len() == 16 && g.nv == 15 && (g.area() - 2.0).abs() < 1e-12);
        assert_eq!([0, 1, 2, 3].map(|e| g.edges.iter().filter(|x| x.edge == e).count()), [4, 2, 4, 2]);
        assert!(g.edges.iter().all(|e| e.boundary == 10 + e.edge));
    }

    #[test]
    fn hostile_and_random_drawings_never_panic() {
        // A small generator (no extra crate): xorshift.
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = move || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };
        let mut f = move || (next() >> 11) as f64 / (1u64 << 53) as f64;
        let (mut ok, mut refused) = (0, 0);
        for round in 0..60 {
            let mut m = Model::default();
            let shapes = 1 + (f() * 4.0) as usize;
            for i in 0..shapes {
                let (x, y) = (f() * 3.0 - 0.5, f() * 3.0 - 0.5);
                let geom = match (f() * 6.0) as usize {
                    0 => Geom::Circle { c: [x, y], r: 10f64.powf(f() * 8.0 - 6.0) },
                    1 => Geom::rect(x, y, x + 10f64.powf(f() * 6.0 - 4.0), y + f()),
                    2 => Geom::Poly { pts: (0..3 + (f() * 6.0) as usize).map(|_| [f() * 2.0, f() * 2.0]).collect(), bulge: (0..9).map(|_| if f() < 0.3 { f() * 4.0 - 2.0 } else { 0.0 }).collect() },
                    // Needle-thin, and touching another shape exactly along an edge.
                    3 => Geom::rect(x, y, x + 1.0, y + 1e-7),
                    4 => Geom::rect(1.0, 0.0, 2.0, 1.0),
                    _ => Geom::rect(0.0, 0.0, 1.0, 1.0),
                };
                m.shapes.push(Shape { id: i as u32 + 1, name: format!("S{i}"), hole: i > 0 && f() < 0.4, geom, edges: vec![] });
            }
            m.mesh = MeshSettings { size: if f() < 0.3 { 10f64.powf(f() * 4.0 - 3.0) } else { 0.0 }, wall: 0.01 + f(), corner: 0.01 + f(), wake: 0.01 + f(), growth: 0.02 + f(), min_angle: 5.0 + f() * 28.0,
                refine: if f() < 0.3 { vec![Refine { at: Target::Point { x: f() * 2.0, y: f() * 2.0 }, size: 10f64.powf(f() * 3.0 - 3.0) }, Refine { at: Target::Edge { shape: 1, edge: 77 }, size: 0.01 }, Refine { at: Target::Box { x0: 5.0, y0: 5.0, x1: -5.0, y1: -5.0 }, size: 0.05 }] } else { vec![] } };
            m.normalize();
            // Make it a through-flow now and then, so walls and wakes are refined too.
            if round % 3 == 0 {
                let edges = m.all_edges();
                if edges.len() >= 3 {
                    m.boundaries = vec![
                        Boundary { id: 900, name: "In".into(), edges: vec![edges[0]], kind: BcKind::Inlet { speed: 1.0, profile: Profile::Uniform }, ..Default::default() },
                        Boundary { id: 901, name: "Out".into(), edges: vec![edges[1]], kind: BcKind::Outlet { pressure: 0.0 }, ..Default::default() },
                        Boundary { id: 902, name: "Walls".into(), edges: edges[2..].to_vec(), ..Default::default() },
                    ];
                }
            }
            match mesh(&m) {
                Ok(g) => {
                    sound(&g);
                    let _ = crate::check(&m, Some(&g));
                    ok += 1;
                }
                Err(e) => {
                    assert!(!e.is_empty() && e.ends_with('.'), "{e}");
                    let _ = crate::check(&m, None);
                    refused += 1;
                }
            }
        }
        assert!(ok > 8 && refused > 2, "{ok} meshed, {refused} refused");
    }
}
