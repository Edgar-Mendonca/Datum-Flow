//! Files written from a solution: VTU (the VTK unstructured-grid format, read by free viewers
//! such as ParaView) and CSV tables of probes and line plots.

use crate::values::Sample;
use flow_core::model::Units;
use flow_core::units::{to_display, unit};
use flow_mesh::Mesh;
use flow_solve::State;
use std::fmt::Write;

/// The mesh and the solution as a VTU document: quadratic triangles with velocity, pressure,
/// speed and (with heat) temperature at the points. SI units.
pub fn vtu(mesh: &Mesh, s: &State) -> String {
    let (nn, ne) = (mesh.nodes.len(), mesh.tris.len());
    let p = crate::fields::pressure(mesh, s);
    let mut o = String::with_capacity(nn * 120 + ne * 60);
    o.push_str("<?xml version=\"1.0\"?>\n<VTKFile type=\"UnstructuredGrid\" version=\"0.1\" byte_order=\"LittleEndian\">\n<!-- Written by Datum Flow. SI units: m, m/s, Pa, K. -->\n<UnstructuredGrid>\n");
    let _ = writeln!(o, "<Piece NumberOfPoints=\"{nn}\" NumberOfCells=\"{ne}\">");
    o.push_str("<Points>\n<DataArray type=\"Float64\" NumberOfComponents=\"3\" format=\"ascii\">\n");
    for x in &mesh.nodes {
        let _ = writeln!(o, "{:e} {:e} 0", x[0], x[1]);
    }
    o.push_str("</DataArray>\n</Points>\n<Cells>\n<DataArray type=\"Int32\" Name=\"connectivity\" format=\"ascii\">\n");
    for t in &mesh.tris {
        let _ = writeln!(o, "{} {} {} {} {} {}", t[0], t[1], t[2], t[3], t[4], t[5]);
    }
    o.push_str("</DataArray>\n<DataArray type=\"Int32\" Name=\"offsets\" format=\"ascii\">\n");
    for k in 1..=ne {
        let _ = writeln!(o, "{}", 6 * k);
    }
    // Cell type 22: the quadratic triangle.
    o.push_str("</DataArray>\n<DataArray type=\"UInt8\" Name=\"types\" format=\"ascii\">\n");
    for _ in 0..ne {
        o.push_str("22\n");
    }
    o.push_str("</DataArray>\n</Cells>\n<PointData Scalars=\"speed\" Vectors=\"velocity\">\n<DataArray type=\"Float64\" Name=\"velocity\" NumberOfComponents=\"3\" format=\"ascii\">\n");
    let fin = |v: f64| if v.is_finite() { v } else { 0.0 };
    for u in &s.u {
        let _ = writeln!(o, "{:e} {:e} 0", fin(u[0]), fin(u[1]));
    }
    o.push_str("</DataArray>\n");
    let mut scalar = |name: &str, v: &mut dyn Iterator<Item = f64>| {
        let _ = writeln!(o, "<DataArray type=\"Float64\" Name=\"{name}\" format=\"ascii\">");
        for x in v {
            let _ = writeln!(o, "{:e}", fin(x));
        }
        o.push_str("</DataArray>\n");
    };
    scalar("speed", &mut s.u.iter().map(|u| u[0].hypot(u[1])));
    scalar("pressure", &mut p.iter().copied());
    if !s.t.is_empty() {
        scalar("temperature", &mut s.t.iter().copied());
    }
    o.push_str("</PointData>\n</Piece>\n</UnstructuredGrid>\n</VTKFile>\n");
    o
}

/// A text cell for a CSV file: quoted when needed, and never starting a formula.
pub fn cell(text: &str) -> String {
    let t = text.trim();
    let risky = t.starts_with(['=', '+', '-', '@']) && t.parse::<f64>().is_err();
    let t = if risky { format!("'{t}") } else { t.to_string() };
    if t.contains([',', '"', '\n', '\r']) { format!("\"{}\"", t.replace('"', "\"\"")) } else { t }
}

fn num(v: f64) -> String {
    if v.is_finite() { format!("{v}") } else { String::new() }
}

fn columns(units: Units, heat: bool, first: &[(&str, &str)]) -> (String, Vec<&'static str>) {
    let mut kinds = vec!["len", "len", "speed", "speed", "speed", "pressure", "vorticity"];
    let mut names = vec!["x", "y", "u", "v", "speed", "pressure", "vorticity"];
    if heat {
        kinds.push("temp");
        names.push("temperature");
    }
    let head: Vec<String> = first.iter().map(|(n, k)| (n.to_string(), *k)).chain(names.iter().zip(&kinds).map(|(n, k)| (n.to_string(), *k)))
        .map(|(n, k)| { let l = unit(units, k).label; if k.is_empty() || l.is_empty() { n } else { format!("{n} [{l}]") } }).collect();
    (head.join(","), kinds)
}

fn row(units: Units, s: &Sample, heat: bool) -> String {
    let c = |k: &str, v: f64| num(to_display(units, k, v));
    let mut r = vec![c("len", s.x), c("len", s.y), c("speed", s.u), c("speed", s.v), c("speed", s.speed), c("pressure", s.pressure), c("vorticity", s.vorticity)];
    if heat {
        r.push(c("temp", s.temperature));
    }
    r.join(",")
}

/// Probes as a CSV table in the model's display units.
pub fn probes_csv(units: Units, heat: bool, probes: &[(String, Sample)]) -> String {
    let (head, _) = columns(units, heat, &[("probe", "")]);
    let mut o = head + "\n";
    for (name, s) in probes {
        let _ = writeln!(o, "{},{}", cell(name), row(units, s, heat));
    }
    o
}

/// A line plot as a CSV table in the model's display units.
pub fn line_csv(units: Units, heat: bool, points: &[(f64, Sample)]) -> String {
    let (head, _) = columns(units, heat, &[("distance", "len")]);
    let mut o = head + "\n";
    for (d, s) in points {
        let _ = writeln!(o, "{},{}", num(to_display(units, "len", *d)), row(units, s, heat));
    }
    o
}
