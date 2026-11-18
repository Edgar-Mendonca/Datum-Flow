//! The model: what the user draws and sets. Everything is in SI base units (m, kg, s, K, Pa, W);
//! the window converts for display only (`units.rs`).

use serde::{Deserialize, Serialize};

/// The version of the `.dflow` format this build writes.
pub const FORMAT: u32 = 1;
/// Standard gravity (m/s²).
pub const G0: f64 = 9.806_65;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Units {
    #[default]
    SI,
    US,
}

/// An outline: a closed polygon whose sides may be arcs (`bulge` = tan of a quarter of the swept
/// angle, positive to the left of the side), or a circle. Counter-clockwise or clockwise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Geom {
    Poly {
        pts: Vec<[f64; 2]>,
        #[serde(default)]
        bulge: Vec<f64>,
    },
    Circle { c: [f64; 2], r: f64 },
}

/// Which side of the line a→b a point is on (positive: left).
fn side(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

/// The point halfway along the arc of a bulged side.
pub fn arc_mid(p: [f64; 2], q: [f64; 2], bulge: f64) -> [f64; 2] {
    if bulge == 0.0 {
        return [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
    }
    let (c, r, a0, th) = arc(p, q, bulge);
    [c[0] + r * (a0 + th / 2.0).cos(), c[1] + r * (a0 + th / 2.0).sin()]
}

/// The bulge of the arc from p to q that passes through t; 0 when t is on the chord's line.
/// None when no sensible arc exists (t at an end, or the arc would be almost a full circle).
pub fn bulge_through(p: [f64; 2], q: [f64; 2], t: [f64; 2]) -> Option<f64> {
    let chord = (q[0] - p[0]).hypot(q[1] - p[1]);
    if chord.is_nan() || chord <= 0.0 || [p, q].iter().any(|e| (t[0] - e[0]).hypot(t[1] - e[1]) < 1e-9 * chord) {
        return None;
    }
    let d = 2.0 * side(p, q, t);
    if d.abs() < 1e-12 * chord * chord {
        return Some(0.0);
    }
    // The circle through the three points.
    let (a2, b2, c2) = (p[0] * p[0] + p[1] * p[1], q[0] * q[0] + q[1] * q[1], t[0] * t[0] + t[1] * t[1]);
    let det = 2.0 * (p[0] * (q[1] - t[1]) + q[0] * (t[1] - p[1]) + t[0] * (p[1] - q[1]));
    let c = [(a2 * (q[1] - t[1]) + b2 * (t[1] - p[1]) + c2 * (p[1] - q[1])) / det, (a2 * (t[0] - q[0]) + b2 * (p[0] - t[0]) + c2 * (q[0] - p[0])) / det];
    let ang = |x: [f64; 2]| (x[1] - c[1]).atan2(x[0] - c[0]);
    let tau = std::f64::consts::TAU;
    let s = (ang(q) - ang(p)).rem_euclid(tau);
    let st = (ang(t) - ang(p)).rem_euclid(tau);
    // Counter-clockwise from p to q if t lies on that way round, else clockwise.
    let sweep = if st < s { s } else { s - tau };
    if sweep.abs() > 350f64.to_radians() {
        return None;
    }
    // The sign that puts the arc on t's side of the chord (checked, not assumed).
    let b = (sweep.abs() / 4.0).tan();
    Some(if side(p, q, arc_mid(p, q, b)).signum() == side(p, q, t).signum() { b } else { -b })
}

/// The arc of a bulged side p→q: centre, radius, start angle and swept angle (signed).
pub fn arc(p: [f64; 2], q: [f64; 2], bulge: f64) -> ([f64; 2], f64, f64, f64) {
    let th = 4.0 * bulge.atan();
    let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
    let c = (dx * dx + dy * dy).sqrt();
    let r = c / (2.0 * (th / 2.0).sin()).abs();
    // Centre: from the chord's midpoint, along the chord's left normal by the signed distance.
    let d = c * 0.5 / (th / 2.0).tan();
    let (mx, my) = ((p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0);
    let (nx, ny) = (-dy / c, dx / c);
    let cen = [mx + nx * d, my + ny * d];
    let a0 = (p[1] - cen[1]).atan2(p[0] - cen[0]);
    (cen, r, a0, th)
}

impl Geom {
    pub fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Geom {
        let (a, b) = (x0.min(x1), x0.max(x1));
        let (c, d) = (y0.min(y1), y0.max(y1));
        Geom::Poly { pts: vec![[a, c], [b, c], [b, d], [a, d]], bulge: vec![0.0; 4] }
    }
    pub fn poly(pts: &[[f64; 2]]) -> Geom {
        Geom::Poly { pts: pts.to_vec(), bulge: vec![0.0; pts.len()] }
    }
    /// Number of edges a boundary can be put on.
    pub fn edges(&self) -> usize {
        match self {
            Geom::Poly { pts, .. } => pts.len(),
            Geom::Circle { .. } => 1,
        }
    }
    /// Signed area (positive counter-clockwise), arcs included.
    pub fn area(&self) -> f64 {
        match self {
            Geom::Circle { r, .. } => std::f64::consts::PI * r * r,
            Geom::Poly { pts, bulge } => {
                let n = pts.len();
                let mut a = 0.0;
                for i in 0..n {
                    let (p, q) = (pts[i], pts[(i + 1) % n]);
                    a += 0.5 * (p[0] * q[1] - q[0] * p[1]);
                    let b = bulge.get(i).copied().unwrap_or(0.0);
                    if b != 0.0 {
                        // The circular segment between the chord and the arc.
                        let th = 4.0 * b.atan();
                        let c = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2)).sqrt();
                        let r = c / (2.0 * (th / 2.0).sin().abs());
                        a += 0.5 * r * r * (th - th.sin());
                    }
                }
                a
            }
        }
    }
    /// The outline as points, arcs split so no piece is longer than `seg` (and at least `min`
    /// pieces per full circle). Edge i starts at index starts[i].
    pub fn outline(&self, seg: f64, min: usize) -> (Vec<[f64; 2]>, Vec<usize>) {
        match self {
            Geom::Circle { c, r } => {
                let n = ((std::f64::consts::TAU * r / seg).ceil() as usize).clamp(min.max(8), 4096);
                ((0..n).map(|i| { let t = i as f64 / n as f64 * std::f64::consts::TAU; [c[0] + r * t.cos(), c[1] + r * t.sin()] }).collect(), vec![0])
            }
            Geom::Poly { pts, bulge } => {
                let n = pts.len();
                let (mut out, mut starts) = (vec![], vec![]);
                for i in 0..n {
                    starts.push(out.len());
                    let (p, q) = (pts[i], pts[(i + 1) % n]);
                    out.push(p);
                    let b = bulge.get(i).copied().unwrap_or(0.0);
                    if b != 0.0 {
                        let (c, r, a0, th) = arc(p, q, b);
                        let k = ((r * th.abs() / seg).ceil() as usize).max(((th.abs() / std::f64::consts::TAU) * min as f64).ceil() as usize).clamp(2, 4096);
                        for j in 1..k {
                            let t = a0 + th * j as f64 / k as f64;
                            out.push([c[0] + r * t.cos(), c[1] + r * t.sin()]);
                        }
                    }
                }
                (out, starts)
            }
        }
    }
    /// [x0, y0, x1, y1] of the outline.
    pub fn bounds(&self) -> [f64; 4] {
        match self {
            Geom::Circle { c, r } => [c[0] - r, c[1] - r, c[0] + r, c[1] + r],
            Geom::Poly { .. } => {
                let (o, _) = self.outline(f64::INFINITY, 32);
                o.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, p| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])])
            }
        }
    }
    /// The two ends of edge `i` (a circle's one edge starts and ends at its rightmost point).
    pub fn edge_ends(&self, i: usize) -> ([f64; 2], [f64; 2]) {
        match self {
            Geom::Circle { c, r } => ([c[0] + r, c[1]], [c[0] + r, c[1]]),
            Geom::Poly { pts, .. } => (pts[i % pts.len()], pts[(i + 1) % pts.len()]),
        }
    }
    /// Whether edge `i` is a straight line.
    pub fn straight(&self, i: usize) -> bool {
        match self {
            Geom::Circle { .. } => false,
            Geom::Poly { bulge, .. } => bulge.get(i).copied().unwrap_or(0.0) == 0.0,
        }
    }
    /// What is wrong with the outline, in plain words; None when it can be meshed.
    pub fn problem(&self) -> Option<String> {
        let fin = |v: f64| v.is_finite() && v.abs() < 1e9;
        match self {
            Geom::Circle { c, r } => {
                if !(fin(c[0]) && fin(c[1]) && fin(*r)) { return Some("a circle has a value that is not a number".into()); }
                if *r <= 0.0 { return Some("a circle needs a radius above zero".into()); }
            }
            Geom::Poly { pts, bulge } => {
                if pts.len() < 3 { return Some("a polygon needs at least three corners".into()); }
                if pts.len() > 20_000 { return Some("a polygon has too many corners (over 20 000)".into()); }
                if !pts.iter().all(|p| fin(p[0]) && fin(p[1])) || !bulge.iter().all(|b| b.is_finite() && b.abs() < 1e3) { return Some("a polygon has a value that is not a number".into()); }
                let b = self.bounds();
                let size = (b[2] - b[0]).hypot(b[3] - b[1]);
                let n = pts.len();
                for i in 0..n {
                    let (p, q) = (pts[i], pts[(i + 1) % n]);
                    if (q[0] - p[0]).hypot(q[1] - p[1]) <= 1e-9 * size { return Some(format!("side {} has zero length (two corners at the same place)", i + 1)); }
                }
                if self.area().abs() <= 1e-12 * size * size { return Some("the outline encloses no area".into()); }
                // Sides must not cross each other, arcs included (split into short straight pieces).
                if n <= 2000 {
                    let (o, starts) = self.outline(size / 200.0, 64);
                    if o.len() <= 6000 {
                        let side_of = |k: usize| starts.iter().rposition(|s0| *s0 <= k).unwrap_or(0);
                        let m = o.len();
                        let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                        for i in 0..m {
                            for j in i + 2..m {
                                if i == 0 && j == m - 1 { continue; }
                                let (a, b2, c, d) = (o[i], o[(i + 1) % m], o[j], o[(j + 1) % m]);
                                // Quick reject on the pieces' boxes.
                                if a[0].max(b2[0]) < c[0].min(d[0]) || c[0].max(d[0]) < a[0].min(b2[0]) || a[1].max(b2[1]) < c[1].min(d[1]) || c[1].max(d[1]) < a[1].min(b2[1]) { continue; }
                                let (d1, d2, d3, d4) = (cross(c, d, a), cross(c, d, b2), cross(a, b2, c), cross(a, b2, d));
                                if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
                                    let (si, sj) = (side_of(i), side_of(j));
                                    return Some(if si == sj { format!("side {} crosses itself", si + 1) } else { format!("sides {} and {} cross each other", si + 1, sj + 1) });
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shape {
    pub id: u32,
    pub name: String,
    /// A hole is cut out of the flow region (a body in the flow); other shapes add to it.
    pub hole: bool,
    pub geom: Geom,
    /// A name for every edge (shown on the view and in the boundary lists).
    pub edges: Vec<String>,
}
impl Default for Shape {
    fn default() -> Self {
        Shape { id: 0, name: String::new(), hole: false, geom: Geom::rect(0.0, 0.0, 1.0, 1.0), edges: vec![] }
    }
}

/// One edge of one shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EdgeRef {
    pub shape: u32,
    pub edge: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fluid {
    /// "water", "air", "oil" or "custom".
    pub preset: String,
    pub name: String,
    /// The temperature the properties are taken at (K).
    pub at: f64,
    /// Density (kg/m³).
    pub rho: f64,
    /// Dynamic viscosity (Pa·s).
    pub mu: f64,
    /// Thermal conductivity (W/m·K).
    pub k: f64,
    /// Specific heat at constant pressure (J/kg·K).
    pub cp: f64,
    /// Volumetric thermal expansion coefficient (1/K).
    pub beta: f64,
    /// Where the values come from.
    pub source: String,
}
impl Default for Fluid {
    fn default() -> Self {
        crate::fluid::preset("water", 293.15).expect("water")
    }
}
impl Fluid {
    /// Kinematic viscosity ν = μ/ρ (m²/s).
    pub fn nu(&self) -> f64 { self.mu / self.rho }
    /// Thermal diffusivity α = k/(ρ cp) (m²/s).
    pub fn alpha(&self) -> f64 { self.k / (self.rho * self.cp) }
    /// Prandtl number ν/α.
    pub fn pr(&self) -> f64 { self.nu() / self.alpha() }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Physics {
    /// Solve the temperature too.
    pub heat: bool,
    /// Warm fluid rises (the Boussinesq body force); needs `heat`.
    pub buoyancy: bool,
    /// Gravity (m/s², x and y).
    pub gravity: [f64; 2],
    /// The temperature at which the fluid has no buoyancy (K); also the starting temperature.
    pub t_ref: f64,
    /// The length used for the Reynolds and Rayleigh numbers (m); 0 = automatic.
    pub length: f64,
    /// Depth of the 2D model into the page (m), for flow rates, forces and heat flows.
    pub depth: f64,
}
impl Default for Physics {
    fn default() -> Self {
        Physics { heat: false, buoyancy: false, gravity: [0.0, -G0], t_ref: 293.15, length: 0.0, depth: 1.0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    #[default]
    Uniform,
    /// Fully developed between two walls: a parabola with the given mean speed.
    Parabolic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BcKind {
    /// Flow comes in across the edge at a mean speed (m/s).
    Inlet { speed: f64, #[serde(default)] profile: Profile },
    /// Flow leaves freely at a pressure (Pa, relative).
    Outlet { #[serde(default)] pressure: f64 },
    /// A wall the fluid sticks to; `u` is the wall's own velocity (m/s; zero for a fixed wall, the lid speed for a moving one).
    Wall { #[serde(default)] u: [f64; 2] },
    /// No flow across, free to slide along (a symmetry line or a slip wall).
    Symmetry,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HeatBc {
    /// No heat crosses the edge.
    #[default]
    Insulated,
    /// The edge is held at a temperature (K).
    Temperature { t: f64 },
    /// Heat enters the fluid through the edge (W/m²; negative leaves).
    Flux { q: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Boundary {
    pub id: u32,
    pub name: String,
    pub edges: Vec<EdgeRef>,
    pub kind: BcKind,
    pub heat: HeatBc,
}
impl Default for Boundary {
    fn default() -> Self {
        Boundary { id: 0, name: String::new(), edges: vec![], kind: BcKind::Wall { u: [0.0; 2] }, heat: HeatBc::Insulated }
    }
}

/// Where a local mesh size applies.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Target {
    Edge { shape: u32, edge: u32 },
    Point { x: f64, y: f64 },
    Box { x0: f64, y0: f64, x1: f64, y1: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Refine {
    pub at: Target,
    /// Element size there (m).
    pub size: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MeshSettings {
    /// Global element size (m); 0 = automatic.
    pub size: f64,
    /// Element size at walls as a share of the global size (1 = no refinement).
    pub wall: f64,
    /// Element size at sharp inside corners as a share of the global size.
    pub corner: f64,
    /// Element size in the wake behind bodies as a share of the global size.
    pub wake: f64,
    /// How fast elements grow away from a refined place (size gained per distance).
    pub growth: f64,
    /// Smallest angle the mesher aims for (degrees).
    pub min_angle: f64,
    pub refine: Vec<Refine>,
}
impl Default for MeshSettings {
    fn default() -> Self {
        MeshSettings { size: 0.0, wall: 0.5, corner: 0.25, wake: 0.5, growth: 0.2, min_angle: 25.0, refine: vec![] }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Continuation {
    /// Solve directly; raise the Reynolds or Rayleigh number in steps only if that fails.
    #[default]
    Auto,
    /// Always raise it in steps.
    Always,
    Off,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SolverSettings {
    /// Most iterations of one solve (each continuation step gets this many).
    pub max_iter: u32,
    /// Stop when the residual has fallen by this factor.
    pub tol: f64,
    /// Picard (simple) iterations before Newton takes over.
    pub picard: u32,
    pub continuation: Continuation,
    /// Grad-div stabilisation weight (0 = off; helps strong natural convection).
    pub grad_div: f64,
    /// Solve even when the Reynolds or Rayleigh number says the flow is probably turbulent.
    pub allow_turbulent: bool,
}
impl Default for SolverSettings {
    fn default() -> Self {
        SolverSettings { max_iter: 40, tol: 1e-8, picard: 3, continuation: Continuation::Auto, grad_div: 0.0, allow_turbulent: false }
    }
}

/// A number watched while solving and listed in the results.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MonitorKind {
    /// Force on a boundary along the flow (N).
    Drag { boundary: u32 },
    /// Force on a boundary across the flow (N).
    Lift { boundary: u32 },
    /// Volume flow through a boundary (m³/s).
    FlowRate { boundary: u32 },
    /// Mean pressure on `from` minus mean pressure on `to` (Pa).
    PressureDrop { from: u32, to: u32 },
    /// Flow-weighted mean temperature on a boundary (K).
    MeanTemperature { boundary: u32 },
    /// Heat passing from a boundary into the fluid (W).
    HeatFlow { boundary: u32 },
    /// Average Nusselt number on a boundary.
    Nusselt { boundary: u32 },
    /// Speed at a point (m/s).
    Speed { x: f64, y: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Monitor {
    pub name: String,
    pub kind: MonitorKind,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Probe {
    pub name: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Line {
    pub name: String,
    pub a: [f64; 2],
    pub b: [f64; 2],
    /// Number of points along it.
    pub n: u32,
}
impl Default for Line {
    fn default() -> Self {
        Line { name: String::new(), a: [0.0; 2], b: [1.0, 0.0], n: 101 }
    }
}

/// What the results view shows; saved with the file so it reopens the same.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct View {
    /// "speed", "pressure", "temperature", "vorticity", "u", "v", "stream" or "none".
    pub field: String,
    /// "viridis", "inferno", "cividis" or "diverging".
    pub map: String,
    pub arrows: bool,
    pub streamlines: bool,
    pub mesh: bool,
    /// Number of colour bands (0 = smooth).
    pub bands: u32,
    /// Colour ranges set by hand, by field (SI units: low, high); fields not listed are automatic.
    pub ranges: std::collections::BTreeMap<String, [f64; 2]>,
}
impl Default for View {
    fn default() -> Self {
        View { field: "speed".into(), map: "viridis".into(), arrows: false, streamlines: true, mesh: false, bands: 0, ranges: Default::default() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Model {
    pub name: String,
    /// A line or two about the case (examples say what to look for).
    pub notes: String,
    pub units: Units,
    /// Snap grid of the drawing (m).
    pub grid: f64,
    pub shapes: Vec<Shape>,
    pub fluid: Fluid,
    pub physics: Physics,
    pub boundaries: Vec<Boundary>,
    pub mesh: MeshSettings,
    pub solver: SolverSettings,
    pub monitors: Vec<Monitor>,
    pub probes: Vec<Probe>,
    pub lines: Vec<Line>,
    pub view: View,
}
impl Default for Model {
    fn default() -> Self {
        Model {
            name: "Untitled".into(), notes: String::new(), units: Units::SI, grid: 0.01, shapes: vec![], fluid: Fluid::default(), physics: Physics::default(),
            boundaries: vec![], mesh: MeshSettings::default(), solver: SolverSettings::default(), monitors: vec![], probes: vec![], lines: vec![], view: View::default(),
        }
    }
}

impl Model {
    pub fn shape(&self, id: u32) -> Option<&Shape> { self.shapes.iter().find(|s| s.id == id) }
    pub fn boundary(&self, id: u32) -> Option<&Boundary> { self.boundaries.iter().find(|b| b.id == id) }
    /// The boundary an edge belongs to.
    pub fn boundary_of(&self, e: EdgeRef) -> Option<&Boundary> { self.boundaries.iter().find(|b| b.edges.contains(&e)) }
    /// The next free id (shapes and boundaries share one counter).
    pub fn next_id(&self) -> u32 {
        self.shapes.iter().map(|s| s.id).chain(self.boundaries.iter().map(|b| b.id)).max().unwrap_or(0).saturating_add(1)
    }
    /// [x0, y0, x1, y1] of the flow region (the shapes that add to it).
    pub fn bounds(&self) -> Option<[f64; 4]> {
        let mut it = self.shapes.iter().filter(|s| !s.hole).map(|s| s.geom.bounds());
        let first = it.next()?;
        Some(it.fold(first, |b, c| [b[0].min(c[0]), b[1].min(c[1]), b[2].max(c[2]), b[3].max(c[3])]))
    }
    /// Every edge of every shape.
    pub fn all_edges(&self) -> Vec<EdgeRef> {
        self.shapes.iter().flat_map(|s| (0..s.geom.edges() as u32).map(move |e| EdgeRef { shape: s.id, edge: e })).collect()
    }
    /// The name shown for an edge: its own name, or "Shape · side n".
    pub fn edge_name(&self, e: EdgeRef) -> String {
        match self.shape(e.shape) {
            Some(s) => match s.edges.get(e.edge as usize).filter(|n| !n.is_empty()) {
                Some(n) => n.clone(),
                None if s.geom.edges() == 1 => s.name.clone(),
                None => format!("{} · side {}", s.name, e.edge + 1),
            },
            None => format!("edge {}/{}", e.shape, e.edge),
        }
    }
    /// Check the drawing as a whole; the message says what to fix.
    pub fn check_geometry(&self) -> Result<(), String> {
        if !self.shapes.iter().any(|s| !s.hole) {
            return Err("There is nothing to mesh: draw the flow region first (Geometry › Rectangle, Circle or Polygon, or pick a template).".into());
        }
        for s in &self.shapes {
            if let Some(p) = s.geom.problem() {
                return Err(format!("\"{}\" cannot be used: {p}.", s.name));
            }
        }
        Ok(())
    }
    /// Make every value safe to work with: non-finite numbers and impossible settings go back to
    /// their defaults, references to things that no longer exist are dropped. Returns what changed.
    pub fn normalize(&mut self) -> Vec<String> {
        let mut notes = vec![];
        let d = Model::default();
        let fix = |v: &mut f64, lo: f64, hi: f64, def: f64, what: &str, notes: &mut Vec<String>| {
            if !v.is_finite() || *v < lo || *v > hi {
                notes.push(format!("{what} was out of range and was reset."));
                *v = def;
            }
        };
        fix(&mut self.grid, 1e-9, 1e6, d.grid, "The snap grid", &mut notes);
        let f = &mut self.fluid;
        let df = Fluid::default();
        fix(&mut f.rho, 1e-6, 1e6, df.rho, "The fluid's density", &mut notes);
        fix(&mut f.mu, 1e-12, 1e6, df.mu, "The fluid's viscosity", &mut notes);
        fix(&mut f.k, 1e-9, 1e6, df.k, "The fluid's conductivity", &mut notes);
        fix(&mut f.cp, 1e-6, 1e9, df.cp, "The fluid's specific heat", &mut notes);
        fix(&mut f.beta, -1.0, 1.0, df.beta, "The fluid's expansion coefficient", &mut notes);
        fix(&mut f.at, 1.0, 1e5, df.at, "The fluid's temperature", &mut notes);
        let p = &mut self.physics;
        fix(&mut p.gravity[0], -1e12, 1e12, 0.0, "Gravity", &mut notes);
        fix(&mut p.gravity[1], -1e12, 1e12, -G0, "Gravity", &mut notes);
        fix(&mut p.t_ref, 1.0, 1e5, 293.15, "The reference temperature", &mut notes);
        fix(&mut p.length, 0.0, 1e6, 0.0, "The reference length", &mut notes);
        fix(&mut p.depth, 1e-9, 1e6, 1.0, "The depth", &mut notes);
        if p.buoyancy && !p.heat {
            p.buoyancy = false;
        }
        let m = &mut self.mesh;
        fix(&mut m.size, 0.0, 1e6, 0.0, "The element size", &mut notes);
        fix(&mut m.wall, 0.01, 1.0, 0.5, "The wall refinement", &mut notes);
        fix(&mut m.corner, 0.01, 1.0, 0.25, "The corner refinement", &mut notes);
        fix(&mut m.wake, 0.01, 1.0, 0.5, "The wake refinement", &mut notes);
        fix(&mut m.growth, 0.02, 2.0, 0.2, "The mesh growth rate", &mut notes);
        fix(&mut m.min_angle, 5.0, 33.0, 25.0, "The smallest mesh angle", &mut notes);
        let before = m.refine.len();
        m.refine.retain(|r| r.size.is_finite() && r.size > 0.0 && match &r.at {
            Target::Edge { .. } => true,
            Target::Point { x, y } => x.is_finite() && y.is_finite(),
            Target::Box { x0, y0, x1, y1 } => [x0, y0, x1, y1].iter().all(|v| v.is_finite()),
        });
        if m.refine.len() != before {
            notes.push("Local mesh sizes with impossible values were removed.".into());
        }
        let s = &mut self.solver;
        if s.max_iter == 0 || s.max_iter > 10_000 {
            s.max_iter = 40;
            notes.push("The iteration limit was reset.".into());
        }
        s.picard = s.picard.min(1000);
        fix(&mut s.tol, 1e-14, 1e-1, 1e-8, "The convergence tolerance", &mut notes);
        fix(&mut s.grad_div, 0.0, 1e9, 0.0, "The grad-div weight", &mut notes);
        // Shapes: unique ids, an edge name per edge, no broken outlines kept silently.
        let mut seen = std::collections::HashSet::new();
        let before = self.shapes.len();
        self.shapes.retain(|s| s.id != 0 && seen.insert(s.id));
        if self.shapes.len() != before {
            notes.push("Shapes without a proper id were removed.".into());
        }
        for s in &mut self.shapes {
            if let Geom::Poly { pts, bulge } = &mut s.geom {
                bulge.resize(pts.len(), 0.0);
                for b in bulge.iter_mut() {
                    if !b.is_finite() || b.abs() > 1e3 { *b = 0.0; }
                }
            }
            s.edges.resize(s.geom.edges(), String::new());
            if s.name.is_empty() { s.name = format!("Shape {}", s.id); }
        }
        let before = self.shapes.len();
        self.shapes.retain(|s| s.geom.problem().is_none());
        if self.shapes.len() != before {
            notes.push(format!("{} shape(s) with a broken outline were removed.", before - self.shapes.len()));
        }
        // Boundaries: unique ids, each edge in one boundary only, only edges that exist.
        let shapes = &self.shapes;
        let exists = |e: &EdgeRef| shapes.iter().any(|s| s.id == e.shape && (e.edge as usize) < s.geom.edges());
        let mut used = std::collections::HashSet::new();
        let mut ids = std::collections::HashSet::new();
        let mut dropped = false;
        self.boundaries.retain(|b| b.id != 0 && !seen.contains(&b.id) && ids.insert(b.id));
        for b in &mut self.boundaries {
            let n = b.edges.len();
            b.edges.retain(|e| exists(e) && used.insert(*e));
            dropped |= b.edges.len() != n;
            match &mut b.kind {
                BcKind::Inlet { speed, .. } => if !speed.is_finite() || speed.abs() > 1e6 { *speed = 0.0; dropped = true; },
                BcKind::Outlet { pressure } => if !pressure.is_finite() || pressure.abs() > 1e12 { *pressure = 0.0; dropped = true; },
                BcKind::Wall { u } => for v in u.iter_mut() { if !v.is_finite() || v.abs() > 1e6 { *v = 0.0; dropped = true; } },
                BcKind::Symmetry => {}
            }
            match &mut b.heat {
                HeatBc::Temperature { t } => if !t.is_finite() || *t <= 0.0 || *t > 1e5 { b.heat = HeatBc::Insulated; dropped = true; },
                HeatBc::Flux { q } => if !q.is_finite() || q.abs() > 1e12 { b.heat = HeatBc::Insulated; dropped = true; },
                HeatBc::Insulated => {}
            }
            if b.name.is_empty() { b.name = format!("Boundary {}", b.id); }
        }
        if dropped {
            notes.push("Boundary settings that pointed at missing edges or had impossible values were reset.".into());
        }
        let bids: std::collections::HashSet<u32> = self.boundaries.iter().map(|b| b.id).collect();
        let before = self.monitors.len();
        self.monitors.retain(|m| match &m.kind {
            MonitorKind::Drag { boundary } | MonitorKind::Lift { boundary } | MonitorKind::FlowRate { boundary } | MonitorKind::MeanTemperature { boundary } | MonitorKind::HeatFlow { boundary } | MonitorKind::Nusselt { boundary } => bids.contains(boundary),
            MonitorKind::PressureDrop { from, to } => bids.contains(from) && bids.contains(to),
            MonitorKind::Speed { x, y } => x.is_finite() && y.is_finite(),
        });
        if self.monitors.len() != before {
            notes.push("Monitors of boundaries that no longer exist were removed.".into());
        }
        self.probes.retain(|p| p.x.is_finite() && p.y.is_finite());
        self.lines.retain(|l| l.a.iter().chain(l.b.iter()).all(|v| v.is_finite()));
        for l in &mut self.lines { l.n = l.n.clamp(2, 5000); }
        self.view.bands = self.view.bands.min(64);
        let before = self.view.ranges.len();
        self.view.ranges.retain(|k, r| k.len() < 40 && r[0].is_finite() && r[1].is_finite() && r[1] > r[0]);
        if self.view.ranges.len() != before {
            notes.push("Colour ranges with impossible values were set back to automatic.".into());
        }
        notes
    }
}
