//! The results against exact answers: channel flow has them all in closed form.

use crate::values::*;
use crate::*;
use flow_core::examples;
use flow_core::model::*;
use flow_solve::{solve, Setup, SparseLu};

#[test]
fn channel_flow_gives_the_exact_profile_pressure_drop_and_forces() {
    let m = examples::channel();
    let mesh = flow_mesh::mesh(&m).unwrap();
    let s = Setup::from_model(&m, &mesh).unwrap();
    let r = solve(&mesh, &s, &m.solver, None, &mut SparseLu::default(), &mut |_| true).unwrap();
    assert!(r.converged, "{}", r.message);
    let v = Values::new(&m, &mesh, &r.state, &r.residual, r.dofs);
    let loc = Locator::new(&mesh);
    let (h, l, um, mu) = (0.01, 0.1, 0.05, m.fluid.mu);
    // The parabola, at any point: u = 6 U (y/H)(1 − y/H).
    for p in [[0.05, 0.005], [0.03, 0.002], [0.08, 0.0085]] {
        let got = sample(&mesh, &r.state, &loc, p);
        let want = 6.0 * um * (p[1] / h) * (1.0 - p[1] / h);
        assert!((got.u - want).abs() < 1e-9 && got.v.abs() < 1e-9 && got.inside, "{got:?} {want}");
        // Vorticity −du/dy.
        assert!((got.vorticity + 6.0 * um / h * (1.0 - 2.0 * p[1] / h)).abs() < 1e-6);
    }
    assert!(!sample(&mesh, &r.state, &loc, [0.2, 0.0]).inside);
    // Flow rate U H (per metre of depth), in at the inlet and out at the outlet.
    assert!((v.flow_rate(3) - um * h).abs() < 1e-12 && (v.flow_rate(2) + um * h).abs() < 1e-12);
    let (net, inflow) = v.balance();
    assert!(net.abs() < 1e-12 * inflow && (inflow - um * h).abs() < 1e-12, "{net} {inflow}");
    // Pressure drop 12 μ U L / H².
    let dp = 12.0 * mu * um * l / (h * h);
    let (got, kind) = monitor(&v, &loc, &MonitorKind::PressureDrop { from: 2, to: 3 });
    assert!((got - dp).abs() < 1e-8 * dp && kind == "pressure", "{got} {dp}");
    // Friction on the two plates balances the pressure drop: F = Δp H, along the flow.
    let f = v.drag_lift(4);
    assert!((f[0] - dp * h).abs() < 1e-8 * dp * h && f[1].abs() < 1e-8 * dp * h, "{f:?} {}", dp * h);
    assert_eq!(v.flow_dir(), [1.0, 0.0]);
    // The stream function runs from 0 on the bottom plate to the flow rate on the top one (it is
    // a cubic here, so its quadratic elements are close, not exact).
    let psi = stream_function(&mesh, &r.state, &mut SparseLu::default()).unwrap();
    let top = mesh.edges.iter().find(|e| e.edge == 2).unwrap().n[1] as usize;
    let bottom = mesh.edges.iter().find(|e| e.edge == 0).unwrap().n[1] as usize;
    assert!((psi[top] - psi[bottom] - um * h).abs() < 1e-3 * um * h && psi[bottom].abs() < 1e-3 * um * h, "{} {}", psi[top], psi[bottom]);
    assert_eq!(fields::stream_levels(&psi, 9).len(), 9);
    // Fields by name.
    assert!(nodal(&mesh, &r.state, "speed", None).unwrap().len() == mesh.nodes.len() && nodal(&mesh, &r.state, "temperature", None).is_none() && nodal(&mesh, &r.state, "stream", None).is_none() && nodal(&mesh, &r.state, "nope", None).is_none());
    // Line plot across the channel: 51 points, the maximum 1.5 U in the middle.
    let pts = along(&mesh, &r.state, &loc, &m.lines[0]);
    assert!(pts.len() == 51 && (pts[25].1.u - 1.5 * um).abs() < 1e-9 && (pts[50].0 - h).abs() < 1e-15);
    // Arrows only inside the fluid.
    let a = arrows(&mesh, &r.state, &loc, [0.0, 0.0, 0.1, 0.02], 200);
    assert!(a.len() > 60 && a.iter().all(|p| p[1] < 0.01 && p[2] > 0.0));
    // The files.
    let vtu = export::vtu(&mesh, &r.state);
    assert!(vtu.contains(&format!("NumberOfPoints=\"{}\" NumberOfCells=\"{}\"", mesh.nodes.len(), mesh.tris.len())) && vtu.contains("Name=\"velocity\"") && !vtu.contains("temperature") && vtu.ends_with("</VTKFile>\n"));
    let csv = export::line_csv(Units::SI, false, &pts);
    assert!(csv.starts_with("distance [m],x [m],y [m],u [m/s],v [m/s],speed [m/s],pressure [Pa],vorticity [1/s]\n") && csv.lines().count() == 52);
    let csv = export::probes_csv(Units::US, false, &[("=cmd()".into(), pts[25].1), ("a,b".into(), sample(&mesh, &r.state, &loc, [9.0, 9.0]))]);
    assert!(csv.contains("'=cmd()") && csv.contains("\"a,b\"") && csv.contains("u [ft/s]"), "{csv}");
    assert_eq!(export::cell("-12.5"), "-12.5");
}

#[test]
fn heat_flows_balance_in_the_heat_sink() {
    let m = examples::heatsink();
    let mesh = flow_mesh::mesh(&m).unwrap();
    let s = Setup::from_model(&m, &mesh).unwrap();
    let r = solve(&mesh, &s, &m.solver, None, &mut SparseLu::default(), &mut |_| true).unwrap();
    assert!(r.converged, "{}", r.message);
    let v = Values::new(&m, &mesh, &r.state, &r.residual, r.dofs);
    // Mass: what comes in goes out.
    let (net, inflow) = v.balance();
    assert!(net.abs() < 1e-9 * inflow, "{net} {inflow}");
    // Heat: what the fin gives the air is what the air carries away (conduction through the
    // inlet and outlet is small but not zero, hence the 1%).
    let (conducted, carried) = v.heat_balance();
    let fin = v.heat_flow(4);
    assert!(fin > 0.0 && (carried - fin).abs() < 0.01 * fin, "fin {fin} W, carried {carried} W, conducted {conducted} W");
    // The air leaves warmer than it came in and cooler than the fin.
    let out = v.mean_temperature(3);
    assert!(out > 293.15 + 1.0 && out < 333.15, "{out}");
    // Energy from the mixing-cup temperature: ṁ cp (T_out − T_in) = heat from the fin.
    let mdot = m.fluid.rho * v.flow_rate(3);
    assert!((mdot * m.fluid.cp * (out - 293.15) - fin).abs() < 0.01 * fin);
    let loc = Locator::new(&mesh);
    let (dp, _) = monitor(&v, &loc, &MonitorKind::PressureDrop { from: 2, to: 3 });
    assert!(dp > 0.0 && dp < 100.0, "{dp}");
    eprintln!("heat sink: fin {fin:.3} W/m, outlet {:.2} °C, Δp {dp:.3} Pa, {} iterations", out - 273.15, r.history.len());
    assert!(export::vtu(&mesh, &r.state).contains("Name=\"temperature\""));
}
