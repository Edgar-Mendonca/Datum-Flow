//! Templates and examples: complete models, ready to mesh and solve. The benchmark cases use a
//! plain "benchmark fluid" so their Reynolds or Rayleigh number is exact.

use crate::fluid;
use crate::model::*;

/// (id, title, one line) of every example, in menu order.
pub const EXAMPLES: [(&str, &str, &str); 11] = [
    ("cavity-100", "Lid-driven cavity, Re 100", "A square box whose lid slides: one large eddy."),
    ("cavity-1000", "Lid-driven cavity, Re 1000", "The same box at ten times the speed: corner eddies appear."),
    ("channel", "Channel flow", "Water between two plates: the exact parabolic profile and pressure drop."),
    ("cylinder", "Flow past a cylinder, Re 20", "The standard benchmark: drag, lift and pressure difference."),
    ("step", "Backward-facing step", "Air over a step: how far the recirculation reaches."),
    ("heated-1e3", "Heated cavity, Ra 10³", "Natural convection between a warm and a cool wall: almost pure conduction."),
    ("heated-1e4", "Heated cavity, Ra 10⁴", "Natural convection: one roll, Nusselt number 2.24."),
    ("heated-1e5", "Heated cavity, Ra 10⁵", "Natural convection: boundary layers form on the walls."),
    ("heated-1e6", "Heated cavity, Ra 10⁶", "Natural convection: thin boundary layers and a layered core."),
    ("heatsink", "Heat-sink fin channel", "Air between two warm fins: pressure drop and heat removed."),
    ("empty", "Empty model", "A blank drawing."),
];

/// The templates of the Geometry tab: (id, title).
pub const TEMPLATES: [(&str, &str); 5] = [("channel", "Channel"), ("cavity-100", "Cavity"), ("step", "Step"), ("cylinder", "Cylinder in a channel"), ("heated-1e4", "Heated box")];

fn rect(id: u32, name: &str, x0: f64, y0: f64, x1: f64, y1: f64) -> Shape {
    Shape { id, name: name.into(), hole: false, geom: Geom::rect(x0, y0, x1, y1), edges: vec!["Bottom".into(), "Right".into(), "Top".into(), "Left".into()] }
}
fn bc(id: u32, name: &str, shape: u32, edges: &[u32], kind: BcKind, heat: HeatBc) -> Boundary {
    Boundary { id, name: name.into(), edges: edges.iter().map(|e| EdgeRef { shape, edge: *e }).collect(), kind, heat }
}
fn wall() -> BcKind { BcKind::Wall { u: [0.0; 2] } }
fn mon(name: &str, kind: MonitorKind) -> Monitor { Monitor { name: name.into(), kind } }

/// A square cavity of side 1 m whose top wall moves at 1 m/s; the fluid sets the Reynolds number.
pub fn cavity(re: f64) -> Model {
    Model {
        name: format!("Lid-driven cavity, Re {re:.0}"),
        notes: "A closed square box; the lid slides to the right at 1 m/s and drags the fluid round. Look at the streamlines: one main eddy, and small ones in the bottom corners as the Reynolds number rises.".into(),
        grid: 0.05,
        shapes: vec![rect(1, "Cavity", 0.0, 0.0, 1.0, 1.0)],
        fluid: Fluid { name: format!("Benchmark fluid (Re {re:.0})"), source: "Chosen so the Reynolds number is exact: density 1, viscosity 1/Re.".into(), ..fluid::custom("", 1.0, 1.0 / re, 1.0, 1.0, 0.0) },
        boundaries: vec![bc(2, "Lid", 1, &[2], BcKind::Wall { u: [1.0, 0.0] }, HeatBc::Insulated), bc(3, "Walls", 1, &[0, 1, 3], wall(), HeatBc::Insulated)],
        mesh: MeshSettings { size: if re > 500.0 { 0.025 } else { 0.04 }, wall: 0.5, ..Default::default() },
        monitors: vec![mon("Speed at the centre", MonitorKind::Speed { x: 0.5, y: 0.5 })],
        lines: vec![Line { name: "Vertical centre line".into(), a: [0.5, 0.0], b: [0.5, 1.0], n: 101 }, Line { name: "Horizontal centre line".into(), a: [0.0, 0.5], b: [1.0, 0.5], n: 101 }],
        ..Default::default()
    }
}

/// Water between two plates 10 mm apart and 100 mm long, mean speed 0.05 m/s (Re 500).
pub fn channel() -> Model {
    Model {
        name: "Channel flow".into(),
        notes: "Water flows between two flat plates with a fully developed (parabolic) profile. The exact answer is known: the profile stays parabolic and the pressure falls by 12 μ U L / H².".into(),
        grid: 0.005,
        shapes: vec![rect(1, "Channel", 0.0, 0.0, 0.1, 0.01)],
        fluid: fluid::preset("water", 293.15).expect("water"),
        boundaries: vec![
            bc(2, "Inlet", 1, &[3], BcKind::Inlet { speed: 0.05, profile: Profile::Parabolic }, HeatBc::Insulated),
            bc(3, "Outlet", 1, &[1], BcKind::Outlet { pressure: 0.0 }, HeatBc::Insulated),
            bc(4, "Plates", 1, &[0, 2], wall(), HeatBc::Insulated),
        ],
        mesh: MeshSettings { size: 0.002, wall: 1.0, ..Default::default() },
        monitors: vec![mon("Pressure drop", MonitorKind::PressureDrop { from: 2, to: 3 }), mon("Flow rate out", MonitorKind::FlowRate { boundary: 3 })],
        lines: vec![Line { name: "Across the channel".into(), a: [0.05, 0.0], b: [0.05, 0.01], n: 51 }],
        ..Default::default()
    }
}

/// The benchmark of Schäfer & Turek (1996), case 2D-1: a cylinder slightly off the centre line of
/// a channel, Reynolds number 20 on the cylinder diameter and the mean inlet speed.
pub fn cylinder() -> Model {
    Model {
        name: "Flow past a cylinder, Re 20".into(),
        notes: "The standard benchmark of Schäfer & Turek (1996), case 2D-1. A cylinder of diameter 0.1 m sits slightly below the centre line of a channel, so there is a small lift. Published values: drag coefficient 5.5795, lift coefficient 0.010619, pressure difference front to back 0.11752 Pa. Coefficients are the force × 500 here (2 F / ρ U² D with U = 0.2 m/s).".into(),
        grid: 0.05,
        shapes: vec![
            rect(1, "Channel", 0.0, 0.0, 2.2, 0.41),
            Shape { id: 2, name: "Cylinder".into(), hole: true, geom: Geom::Circle { c: [0.2, 0.2], r: 0.05 }, edges: vec!["Cylinder".into()] },
        ],
        fluid: Fluid { name: "Benchmark fluid (ν = 0.001 m²/s)".into(), source: "Schäfer & Turek (1996): density 1 kg/m³, kinematic viscosity 0.001 m²/s.".into(), ..fluid::custom("", 1.0, 0.001, 1.0, 1.0, 0.0) },
        physics: Physics { length: 0.1, ..Default::default() },
        boundaries: vec![
            bc(3, "Inlet", 1, &[3], BcKind::Inlet { speed: 0.2, profile: Profile::Parabolic }, HeatBc::Insulated),
            bc(4, "Outlet", 1, &[1], BcKind::Outlet { pressure: 0.0 }, HeatBc::Insulated),
            bc(5, "Channel walls", 1, &[0, 2], wall(), HeatBc::Insulated),
            bc(6, "Cylinder", 2, &[0], wall(), HeatBc::Insulated),
        ],
        mesh: MeshSettings { size: 0.03, wall: 0.6, wake: 0.5, refine: vec![Refine { at: Target::Edge { shape: 2, edge: 0 }, size: 0.004 }], ..Default::default() },
        monitors: vec![mon("Drag on the cylinder", MonitorKind::Drag { boundary: 6 }), mon("Lift on the cylinder", MonitorKind::Lift { boundary: 6 })],
        probes: vec![Probe { name: "Front of the cylinder".into(), x: 0.15, y: 0.2 }, Probe { name: "Back of the cylinder".into(), x: 0.25, y: 0.2 }],
        lines: vec![Line { name: "Wake centre line".into(), a: [0.25, 0.2], b: [2.2, 0.2], n: 201 }],
        ..Default::default()
    }
}

/// The step of Armaly et al. (1983): inlet channel 5.2 mm high, step 4.9 mm, air.
/// `re` is their Reynolds number: mean inlet speed × twice the inlet height / ν.
pub fn step(re: f64) -> Model {
    let (h, s) = (0.0052, 0.0049);
    let air = fluid::preset("air", 293.15).expect("air");
    let speed = re * air.nu() / (2.0 * h);
    let (up, down) = (4.0 * s, 30.0 * s);
    Model {
        name: format!("Backward-facing step, Re {re:.0}"),
        notes: "Air flows from a narrow channel over a step into a wider one (the geometry of Armaly et al., 1983). Behind the step the flow turns back on itself; the streamlines show where it reattaches to the bottom wall. The Reynolds number uses the mean inlet speed and twice the inlet height.".into(),
        grid: 0.001,
        shapes: vec![Shape {
            id: 1, name: "Step channel".into(), hole: false,
            geom: Geom::poly(&[[-up, s], [0.0, s], [0.0, 0.0], [down, 0.0], [down, s + h], [-up, s + h]]),
            edges: vec!["Inlet floor".into(), "Step face".into(), "Floor".into(), "Outlet".into(), "Roof".into(), "Inlet".into()],
        }],
        fluid: air,
        physics: Physics { length: 2.0 * h, ..Default::default() },
        boundaries: vec![
            bc(2, "Inlet", 1, &[5], BcKind::Inlet { speed, profile: Profile::Parabolic }, HeatBc::Insulated),
            bc(3, "Outlet", 1, &[3], BcKind::Outlet { pressure: 0.0 }, HeatBc::Insulated),
            bc(4, "Walls", 1, &[0, 1, 2, 4], wall(), HeatBc::Insulated),
        ],
        mesh: MeshSettings { size: 0.0012, wall: 0.6, corner: 0.25, ..Default::default() },
        monitors: vec![mon("Pressure drop", MonitorKind::PressureDrop { from: 2, to: 3 })],
        lines: vec![Line { name: "Just above the floor".into(), a: [0.0, 0.0001], b: [down, 0.0001], n: 301 }],
        ..Default::default()
    }
}

/// The differentially heated square cavity of de Vahl Davis (1983): left wall warm, right wall
/// cool, top and bottom insulated, Prandtl number 0.71. The fluid and gravity are chosen so the
/// side is 1 m, the temperature difference 1 K and the Rayleigh number exactly `ra`.
pub fn heated_cavity(ra: f64) -> Model {
    let t0 = 293.15;
    let exp = ra.log10().round();
    Model {
        name: format!("Heated cavity, Ra 1e{exp:.0}"),
        notes: "A closed square box with a warm left wall and a cool right wall (de Vahl Davis, 1983). Warm fluid rises along the left wall and sinks along the right one. The Nusselt number says how much more heat crosses the box than by conduction alone: 1.118, 2.243, 4.519 and 8.800 at Rayleigh numbers 10³ to 10⁶.".into(),
        grid: 0.05,
        shapes: vec![rect(1, "Cavity", 0.0, 0.0, 1.0, 1.0)],
        fluid: Fluid { name: "Benchmark fluid (Pr = 0.71)".into(), source: "Chosen so the Prandtl number is 0.71 and the Rayleigh number equals gravity ÷ 0.71: density 1, viscosity 0.71, conductivity 1, specific heat 1, expansion 1.".into(), ..fluid::custom("", 1.0, 0.71, 1.0, 1.0, 1.0) },
        physics: Physics { heat: true, buoyancy: true, gravity: [0.0, -ra * 0.71], t_ref: t0, length: 1.0, depth: 1.0 },
        boundaries: vec![
            bc(2, "Warm wall", 1, &[3], wall(), HeatBc::Temperature { t: t0 + 0.5 }),
            bc(3, "Cool wall", 1, &[1], wall(), HeatBc::Temperature { t: t0 - 0.5 }),
            bc(4, "Insulated walls", 1, &[0, 2], wall(), HeatBc::Insulated),
        ],
        mesh: MeshSettings { size: if ra > 2e5 { 0.03 } else { 0.04 }, wall: if ra > 2e5 { 0.3 } else { 0.5 }, ..Default::default() },
        monitors: vec![mon("Nusselt number, warm wall", MonitorKind::Nusselt { boundary: 2 }), mon("Heat into the fluid", MonitorKind::HeatFlow { boundary: 2 })],
        lines: vec![Line { name: "Mid-height".into(), a: [0.0, 0.5], b: [1.0, 0.5], n: 101 }],
        view: View { field: "temperature".into(), map: "inferno".into(), ..Default::default() },
        ..Default::default()
    }
}

/// One passage of a plate-fin heat sink, seen from above: air at 20 °C enters at 1 m/s and passes
/// between two fins held at 60 °C. Half a passage is modelled, with symmetry lines.
pub fn heatsink() -> Model {
    let (l, gap, fin, up, down) = (0.05, 0.002, 0.001, 0.01, 0.015);
    let top = gap + fin;
    Model {
        name: "Heat-sink fin channel".into(),
        notes: "Half of one passage between two fins of a heat sink, seen from above. The bottom edge is the middle of the passage and the top edges are the middle of the fin's row (symmetry lines). Air at 20 °C enters at 1 m/s; the fin is at 60 °C. Read the pressure drop and the heat removed (per metre of fin height) in Results › Values.".into(),
        grid: 0.001,
        shapes: vec![Shape {
            id: 1, name: "Passage".into(), hole: false,
            geom: Geom::poly(&[[-up, 0.0], [l + down, 0.0], [l + down, top], [l, top], [l, gap], [0.0, gap], [0.0, top], [-up, top]]),
            edges: vec!["Mid-passage".into(), "Outlet".into(), "Behind the fin".into(), "Fin back".into(), "Fin side".into(), "Fin front".into(), "Ahead of the fin".into(), "Inlet".into()],
        }],
        fluid: fluid::preset("air", 293.15).expect("air"),
        physics: Physics { heat: true, buoyancy: false, length: 2.0 * gap, ..Default::default() },
        boundaries: vec![
            bc(2, "Inlet", 1, &[7], BcKind::Inlet { speed: 1.0, profile: Profile::Uniform }, HeatBc::Temperature { t: 293.15 }),
            bc(3, "Outlet", 1, &[1], BcKind::Outlet { pressure: 0.0 }, HeatBc::Insulated),
            bc(4, "Fin", 1, &[3, 4, 5], wall(), HeatBc::Temperature { t: 333.15 }),
            bc(5, "Symmetry", 1, &[0, 2, 6], BcKind::Symmetry, HeatBc::Insulated),
        ],
        mesh: MeshSettings { size: 0.0005, wall: 0.4, corner: 0.3, ..Default::default() },
        monitors: vec![
            mon("Pressure drop", MonitorKind::PressureDrop { from: 2, to: 3 }),
            mon("Heat removed from the fin", MonitorKind::HeatFlow { boundary: 4 }),
            mon("Outlet temperature", MonitorKind::MeanTemperature { boundary: 3 }),
        ],
        view: View { field: "temperature".into(), map: "inferno".into(), ..Default::default() },
        ..Default::default()
    }
}

/// An example or template by its id.
pub fn example(id: &str) -> Option<Model> {
    let mut m = match id {
        "cavity-100" => cavity(100.0),
        "cavity-1000" => cavity(1000.0),
        "channel" => channel(),
        "cylinder" => cylinder(),
        "step" => step(100.0),
        "heated-1e3" => heated_cavity(1e3),
        "heated-1e4" => heated_cavity(1e4),
        "heated-1e5" => heated_cavity(1e5),
        "heated-1e6" => heated_cavity(1e6),
        "heatsink" => heatsink(),
        "empty" => Model::default(),
        _ => return None,
    };
    m.normalize();
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_example_is_sound_and_fully_bounded() {
        for (id, title, _) in EXAMPLES {
            let m = example(id).unwrap_or_else(|| panic!("{id}"));
            let mut n = m.clone();
            assert!(n.normalize().is_empty() && n == m, "{id} changes when normalised");
            if id == "empty" { continue; }
            assert!(m.check_geometry().is_ok(), "{title}");
            assert!(m.all_edges().iter().all(|e| m.boundary_of(*e).is_some()), "{title}: an edge has no boundary");
            // The file round-trips exactly.
            let (back, notes) = crate::file::read(&crate::file::write(&m)).unwrap();
            assert!(notes.is_empty() && back == m, "{title}: {notes:?}");
        }
        assert!(TEMPLATES.iter().all(|(id, _)| example(id).is_some()) && example("nope").is_none());
    }
    #[test]
    fn the_benchmark_numbers_are_exact() {
        let c = cylinder();
        // Re = U D / ν = 0.2 × 0.1 / 0.001.
        assert!((0.2 * 0.1 / c.fluid.nu() - 20.0).abs() < 1e-12);
        let h = heated_cavity(1e5);
        // Ra = g β ΔT L³ / (ν α), Pr = 0.71.
        assert!((h.physics.gravity[1].abs() * h.fluid.beta / (h.fluid.nu() * h.fluid.alpha()) - 1e5).abs() < 1e-6 && (h.fluid.pr() - 0.71).abs() < 1e-12);
        let s = step(100.0);
        let BcKind::Inlet { speed, .. } = s.boundaries[0].kind else { panic!() };
        assert!((speed * 0.0104 / s.fluid.nu() - 100.0).abs() < 1e-9);
    }
}
