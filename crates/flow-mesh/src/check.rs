//! The checks made before solving (the boundary wizard): each finding says what is wrong and how
//! to fix it, in plain words. Any error stops the solve.

use crate::mesh::Mesh;
use flow_core::model::*;
use flow_core::numbers::numbers;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Issue {
    /// "error" (the solve is refused), "warning" or "ok".
    pub level: &'static str,
    pub text: String,
    /// What to do about it.
    pub fix: String,
    /// The boundary it is about (0 = the model as a whole).
    pub boundary: u32,
}

fn issue(level: &'static str, text: impl Into<String>, fix: &str, boundary: u32) -> Issue {
    Issue { level, text: text.into(), fix: fix.into(), boundary }
}

/// Whether any finding refuses the solve.
pub fn blocked(issues: &[Issue]) -> bool {
    issues.iter().any(|i| i.level == "error")
}

/// Check the model, and its mesh if there is one (with a mesh, only edges that really bound the
/// fluid are asked for a boundary).
pub fn check(m: &Model, mesh: Option<&Mesh>) -> Vec<Issue> {
    let mut out = vec![];
    if let Err(e) = m.check_geometry() {
        out.push(issue("error", e, "Fix the drawing in the Geometry tab.", 0));
        return out;
    }
    // Edges without a boundary.
    let free: Vec<EdgeRef> = match mesh {
        Some(g) => {
            let mut v: Vec<EdgeRef> = g.edges.iter().filter(|e| e.boundary == 0).map(|e| EdgeRef { shape: e.shape, edge: e.edge }).collect();
            v.sort();
            v.dedup();
            v
        }
        None => m.all_edges().into_iter().filter(|e| m.boundary_of(*e).is_none()).collect(),
    };
    if !free.is_empty() {
        let names: Vec<String> = free.iter().take(4).map(|e| format!("\"{}\"", m.edge_name(*e))).collect();
        let more = if free.len() > 4 { format!(" and {} more", free.len() - 4) } else { String::new() };
        out.push(issue("error", format!("{} {} no boundary: {}{more}.", free.len(), if free.len() == 1 { "edge has" } else { "edges have" }, names.join(", ")),
            "Click each edge and say what it is (inlet, outlet, wall or symmetry), or use Physics › Wall the Rest.", 0));
    }
    let used = |b: &&Boundary| !b.edges.is_empty();
    let inlets: Vec<&Boundary> = m.boundaries.iter().filter(used).filter(|b| matches!(b.kind, BcKind::Inlet { .. })).collect();
    let outlets: Vec<&Boundary> = m.boundaries.iter().filter(used).filter(|b| matches!(b.kind, BcKind::Outlet { .. })).collect();
    if !inlets.is_empty() && outlets.is_empty() {
        out.push(issue("error", "There is no outlet: the fluid that comes in has nowhere to go.", "Make one edge a pressure outlet (Physics › Outlet).", 0));
    }
    let n = numbers(m);
    let moving = m.boundaries.iter().filter(used).any(|b| matches!(b.kind, BcKind::Wall { u } if u != [0.0, 0.0]));
    let pushed = inlets.iter().any(|b| matches!(b.kind, BcKind::Inlet { speed, .. } if speed > 0.0));
    let pressures: Vec<f64> = outlets.iter().filter_map(|b| if let BcKind::Outlet { pressure } = b.kind { Some(pressure) } else { None }).collect();
    let pressure_driven = pressures.iter().any(|p| *p != pressures[0]);
    let buoyant = m.physics.heat && m.physics.buoyancy && n.ra > 0.0;
    if free.is_empty() && !moving && !pushed && !pressure_driven && !buoyant {
        out.push(issue("error", "Nothing drives the flow: no inlet speed, no moving wall, no pressure difference and no buoyancy.", "Give an inlet a speed, move a wall, or turn on heat and buoyancy with a temperature difference.", 0));
    }
    for b in m.boundaries.iter().filter(|b| b.edges.is_empty()) {
        out.push(issue("warning", format!("\"{}\" is not on any edge.", b.name), "Pick its edges in the view, or delete it.", b.id));
    }
    for b in m.boundaries.iter().filter(used) {
        let shapes: Vec<Option<&Shape>> = b.edges.iter().map(|e| m.shape(e.shape)).collect();
        match &b.kind {
            BcKind::Inlet { speed, profile } => {
                if *speed == 0.0 {
                    out.push(issue("warning", format!("Inlet \"{}\" has zero speed.", b.name), "Give it a speed, or make it a wall.", b.id));
                }
                if *profile == Profile::Parabolic && !(b.edges.len() == 1 && shapes[0].is_some_and(|s| s.geom.straight(b.edges[0].edge as usize))) {
                    out.push(issue("error", format!("Inlet \"{}\" has a fully developed profile but is not one straight edge.", b.name), "Use one straight edge for it, or choose a uniform profile.", b.id));
                }
                if m.physics.heat && !matches!(b.heat, HeatBc::Temperature { .. }) {
                    out.push(issue("error", format!("Inlet \"{}\" needs a temperature: heat is on, and the fluid coming in must have one.", b.name), "Set the inlet's temperature in its Heat settings.", b.id));
                }
            }
            BcKind::Symmetry => {
                let aligned = b.edges.iter().zip(&shapes).all(|(e, s)| s.is_some_and(|s| {
                    let (p, q) = s.geom.edge_ends(e.edge as usize);
                    let (dx, dy) = ((q[0] - p[0]).abs(), (q[1] - p[1]).abs());
                    s.geom.straight(e.edge as usize) && (dx <= 1e-9 * dy || dy <= 1e-9 * dx)
                }));
                if !aligned {
                    out.push(issue("error", format!("Symmetry \"{}\" is on an edge that is curved or sloping.", b.name), "In 0.1 a symmetry or slip edge must be straight and horizontal or vertical.", b.id));
                }
            }
            _ => {}
        }
        if !m.physics.heat && b.heat != HeatBc::Insulated {
            out.push(issue("error", format!("\"{}\" has a temperature or heat flux set, but heat is off.", b.name), "Turn heat on (Physics › Heat), or set the boundary back to insulated.", b.id));
        }
    }
    if m.physics.heat {
        let fixed = m.boundaries.iter().filter(used).any(|b| matches!(b.heat, HeatBc::Temperature { .. }));
        if !fixed {
            out.push(issue("error", "Heat is on but no boundary has a temperature, so the temperature level is not set.", "Give at least one boundary a temperature.", 0));
        }
        if m.physics.buoyancy {
            if m.physics.gravity == [0.0, 0.0] {
                out.push(issue("error", "Buoyancy is on but gravity is zero.", "Set gravity in Physics (9.81 m/s² downwards on Earth).", 0));
            }
            if m.fluid.beta == 0.0 {
                out.push(issue("warning", "Buoyancy is on but the fluid's expansion coefficient is zero, so warm fluid will not rise.", "Set the expansion coefficient of the fluid.", 0));
            }
        }
    }
    if let Some(t) = n.turbulent() {
        if m.solver.allow_turbulent {
            out.push(issue("warning", t, "\"Solve anyway\" is on: the laminar answer may not be what really happens.", 0));
        } else {
            out.push(issue("error", t, "Lower the speed or the size, or tick Solve › Solve Anyway to see the laminar answer.", 0));
        }
    } else if n.through && n.re > 2300.0 {
        out.push(issue("warning", format!("Re ≈ {:.0}: this flow is close to turbulent and may not settle to a steady answer.", n.re), "Treat the result with care; a finer mesh helps the solver.", 0));
    } else if n.ra > 1e8 {
        out.push(issue("warning", format!("Ra ≈ {:.1e}: this natural convection is close to turbulent and may not settle to a steady answer.", n.ra), "Treat the result with care; refine the mesh at the walls.", 0));
    }
    // A mesh too coarse for the speed: the element Reynolds number.
    if let Some(g) = mesh {
        let cell = m.fluid.rho * n.speed * g.quality.edges[1] / m.fluid.mu;
        if cell > 60.0 {
            out.push(issue("warning", format!("The mesh is coarse for this speed (about {cell:.0} for the Reynolds number of one element).", ), "If the solve struggles or the result looks wavy, use a smaller element size or more refinement at walls.", 0));
        }
        if g.quality.min_angle < 12.0 {
            out.push(issue("warning", format!("The mesh has thin triangles (smallest angle {:.0}°).", g.quality.min_angle), "Look at Mesh › Quality; a smaller size near the worst place usually fixes it.", 0));
        }
    }
    if !blocked(&out) {
        out.insert(0, issue("ok", if mesh.is_some() { "Ready to solve." } else { "The setup is complete." }, "", 0));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::examples::*;

    fn errors(m: &Model) -> Vec<String> {
        check(m, None).into_iter().filter(|i| i.level == "error").map(|i| i.text).collect()
    }

    #[test]
    fn every_example_is_ready_to_solve() {
        for (id, title, _) in EXAMPLES {
            if id == "empty" { continue; }
            let m = example(id).unwrap();
            let g = crate::mesh(&m).unwrap();
            let c = check(&m, Some(&g));
            assert!(c[0].level == "ok" && c[0].text == "Ready to solve.", "{title}: {c:?}");
            assert!(c.iter().all(|i| i.level != "error"), "{title}: {c:?}");
        }
    }

    #[test]
    fn the_wizard_refuses_and_says_why() {
        // No outlet.
        let mut m = channel();
        m.boundaries[1].kind = BcKind::Wall { u: [0.0; 2] };
        assert!(errors(&m).iter().any(|e| e.contains("no outlet")));
        // An edge with no boundary, named.
        let mut m = channel();
        m.boundaries[2].edges.pop();
        let e = errors(&m);
        assert!(e.iter().any(|e| e.contains("1 edge has no boundary") && e.contains("\"Top\"")), "{e:?}");
        // With a mesh, an edge buried inside the region is not asked for.
        let mut m = channel();
        m.shapes.push(Shape { id: 9, name: "Extra".into(), geom: Geom::rect(0.02, 0.002, 0.03, 0.008), edges: vec![String::new(); 4], ..Default::default() });
        assert!(errors(&m).iter().any(|e| e.contains("4 edges have no boundary")));
        let g = crate::mesh(&m).unwrap();
        assert!(!blocked(&check(&m, Some(&g))));
        // Temperatures set but heat off.
        let mut m = heated_cavity(1e4);
        m.physics.heat = false;
        m.physics.buoyancy = false;
        let e = errors(&m);
        assert!(e.iter().any(|e| e.contains("heat is off")) && e.iter().any(|e| e.contains("Nothing drives")), "{e:?}");
        // Probably turbulent, in the brief's words; "solve anyway" turns it into a warning.
        let mut m = channel();
        m.boundaries[0].kind = BcKind::Inlet { speed: 1.2, profile: Profile::Parabolic };
        assert!(errors(&m).iter().any(|e| e.starts_with("Re ≈ 12000: this flow is probably turbulent; 0.1 solves laminar flow only.")));
        m.solver.allow_turbulent = true;
        assert!(errors(&m).is_empty() && check(&m, None).iter().any(|i| i.level == "warning" && i.text.contains("probably turbulent")));
        // Heat on with no temperature anywhere; an inlet without one.
        let mut m = heatsink();
        m.boundaries[0].heat = HeatBc::Insulated;
        assert!(errors(&m).iter().any(|e| e.contains("Inlet \"Inlet\" needs a temperature")));
        m.boundaries[2].heat = HeatBc::Insulated;
        assert!(errors(&m).iter().any(|e| e.contains("no boundary has a temperature")));
        // A parabolic inlet over two edges; a sloping symmetry line; zero gravity with buoyancy.
        let mut m = step(100.0);
        m.boundaries[0].edges.push(EdgeRef { shape: 1, edge: 4 });
        m.boundaries[2].edges.retain(|e| e.edge != 4);
        assert!(errors(&m).iter().any(|e| e.contains("not one straight edge")));
        let mut m = cylinder();
        m.boundaries[3].kind = BcKind::Symmetry;
        assert!(errors(&m).iter().any(|e| e.contains("curved or sloping")));
        let mut m = heated_cavity(1e4);
        m.physics.gravity = [0.0, 0.0];
        assert!(errors(&m).iter().any(|e| e.contains("gravity is zero")));
        // A broken drawing is the only thing reported.
        let mut m = cavity(100.0);
        m.shapes.clear();
        let c = check(&m, None);
        assert!(c.len() == 1 && c[0].level == "error" && c[0].text.contains("nothing to mesh"));
    }
}
