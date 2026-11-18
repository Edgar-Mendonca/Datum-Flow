//! The command-line tool: run a `.dflow` without the window. `run` takes the arguments and
//! returns the exit code and the text to print, so it can be tested.

use crate::Engine;
use flow_core::units::show;
use flow_core::{examples, file};
use std::fmt::Write;

pub const USAGE: &str = "Datum Flow command line

  flow-cli solve <model.dflow> [--save <file.dflow>] [--vtu <file>] [--probes <file.csv>] [--lines <prefix>] [--json]
        Mesh, check and solve a model; print the residuals and the values. A file that keeps
        results starts from them. --save writes the model with its results, to open solved.
        --vtu writes the fields for a viewer such as ParaView, --probes the probes as CSV,
        --lines one CSV per plot line (<prefix>-1.csv, …), --json prints the values as JSON.
  flow-cli check <model.dflow>      Mesh the model and list what the setup check finds.
  flow-cli mesh <model.dflow>       Mesh the model and print the quality report.
  flow-cli example <id> [<file>]    Write a built-in example as a .dflow (ids: flow-cli examples).
  flow-cli examples                 List the built-in examples.
  flow-cli verify                   Print the verification record shipped with this version.
  flow-cli version

Exit codes: 0 done, 1 not converged or a check failed, 2 the command or the file could not be read.";

fn load(path: &str) -> Result<(flow_core::Model, String, Option<serde_json::Value>), String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("Could not open {path}: {e}"))?;
    if meta.len() > file::MAX_BYTES as u64 {
        return Err("This file is too large to be a Datum Flow model.".into());
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("Could not open {path}: {e}"))?;
    let (m, notes, results) = file::read_all(&text)?;
    Ok((m, notes.iter().map(|n| format!("note: {n}\n")).collect(), results))
}

fn write(path: &str, text: &str) -> Result<(), String> {
    // Whole or not at all: a temporary file beside the target, then a rename.
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, path)).map_err(|e| format!("Could not write {path}: {e}"))
}

pub fn run(args: &[String]) -> (i32, String) {
    match go(args) {
        Ok(r) => r,
        Err(e) => (2, format!("{e}\n")),
    }
}

fn go(args: &[String]) -> Result<(i32, String), String> {
    let mut o = String::new();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let need_file = || args.get(1).filter(|a| !a.starts_with("--")).cloned().ok_or_else(|| format!("Which model? Usage:\n\n{USAGE}"));
    match cmd {
        "version" | "--version" | "-V" => Ok((0, format!("Datum Flow {}\n", env!("CARGO_PKG_VERSION")))),
        "examples" => {
            for (id, title, about) in examples::EXAMPLES {
                let _ = writeln!(o, "{id:<12} {title}: {about}");
            }
            Ok((0, o))
        }
        "example" => {
            let id = args.get(1).ok_or("Which example? See: flow-cli examples")?;
            let m = examples::example(id).ok_or_else(|| format!("There is no example \"{id}\". See: flow-cli examples"))?;
            match args.get(2) {
                Some(path) => { write(path, &file::write(&m))?; Ok((0, format!("Wrote {path}\n"))) }
                None => Ok((0, file::write(&m) + "\n")),
            }
        }
        "verify" => {
            let r = flow_verify::report::shipped();
            let _ = writeln!(o, "Datum Flow {} verification record, run on {} ({}).", r.version, r.date, r.machine);
            for c in &r.checks {
                let _ = writeln!(o, "{} {} | {} | reference {} | result {} | error {:.1e} | tolerance {:.1e}", if c.pass { "PASS" } else { "FAIL" }, c.case, c.quantity, c.reference, c.result, c.error, c.tolerance);
            }
            let _ = writeln!(o, "{} of {} checks pass. Full report: docs/VERIFICATION.md, or Help › Verification in the app.", r.passed(), r.checks.len());
            Ok((if r.failed() == 0 && !r.checks.is_empty() { 0 } else { 1 }, o))
        }
        "mesh" | "check" | "solve" => {
            let (m, notes, results) = load(&need_file()?)?;
            o.push_str(&notes);
            let mut e = Engine::default();
            // Results kept in the file: the solve starts from them.
            if let Some(r) = results.filter(|_| cmd == "solve") {
                match e.from_file(&m, &r, 1) {
                    Ok(()) => o.push_str("note: starting from the results kept in the file.\n"),
                    Err(err) => { let _ = writeln!(o, "note: {err}"); }
                }
            }
            let view = if e.has_mesh() { e.mesh_view().ok_or("There is no mesh.")? } else { e.mesh(&m, 1)? };
            let q = &view.quality;
            let _ = writeln!(o, "Mesh: {} elements, {} nodes, {} unknowns; smallest angle {:.1}°; largest size ratio between neighbours {:.2}; element size {}.", q.elements, q.nodes, q.unknowns, q.min_angle, q.max_ratio, show(m.units, "len", view.size, 3));
            if cmd == "mesh" {
                let _ = writeln!(o, "Angles: {} over 30°, {} at 20–30°, {} at 10–20°, {} under 10°. Edge lengths {} to {}.", q.angle_bands[0], q.angle_bands[1], q.angle_bands[2], q.angle_bands[3], show(m.units, "len", q.edges[0], 3), show(m.units, "len", q.edges[2], 3));
                return Ok((0, o));
            }
            let issues = e.check(&m);
            for i in &issues {
                let _ = writeln!(o, "{}: {}{}", i.level, i.text, if i.fix.is_empty() { String::new() } else { format!(" {}", i.fix) });
            }
            let blocked = issues.iter().any(|i| i.level == "error");
            if cmd == "check" || blocked {
                return Ok((blocked as i32, o));
            }
            let mut lines = String::new();
            let s = e.solve_now(&m, 1, &mut |p| {
                let r = &p.row;
                let _ = writeln!(lines, "{:>4} {:<6} load {:>4.0}%  momentum {:.2e}  continuity {:.2e}{}", r.iter, r.kind, r.load * 100.0, r.momentum, r.continuity, if m.physics.heat { format!("  energy {:.2e}", r.energy) } else { String::new() });
                true
            })?;
            o.push_str(&lines);
            let _ = writeln!(o, "{} ({} unknowns, {:.1} s, {}).", s.message, s.stats.unknowns, s.stats.seconds, s.stats.linear_solver);
            for a in &s.advice {
                let _ = writeln!(o, "try: {a}");
            }
            let values = e.values(&m)?;
            if args.iter().any(|a| a == "--json") {
                let _ = writeln!(o, "{}", serde_json::json!({ "converged": s.converged, "iterations": s.history.len(), "values": values }));
            } else {
                let mut group = "";
                for v in &values {
                    if v.group != group {
                        group = v.group;
                        let _ = writeln!(o, "\n{group}");
                    }
                    let _ = writeln!(o, "  {:<52} {}{}", v.name, show(m.units, v.kind, v.value, 6), if v.note.is_empty() { String::new() } else { format!("   ({})", v.note) });
                }
            }
            if let Some(path) = opt("--vtu") {
                write(&path, &e.vtu()?)?;
                let _ = writeln!(o, "Wrote {path}");
            }
            if let Some(path) = opt("--save") {
                write(&path, &file::write_with(&m, e.to_file(&m, 1).as_ref()))?;
                let _ = writeln!(o, "Wrote {path} (the model with its results)");
            }
            if let Some(path) = opt("--probes") {
                write(&path, &e.probes_csv(&m)?)?;
                let _ = writeln!(o, "Wrote {path}");
            }
            if let Some(prefix) = opt("--lines") {
                for i in 0..m.lines.len() {
                    let path = format!("{prefix}-{}.csv", i + 1);
                    write(&path, &e.line_csv(&m, i)?)?;
                    let _ = writeln!(o, "Wrote {path}");
                }
            }
            Ok((if s.converged { 0 } else { 1 }, o))
        }
        "help" | "--help" | "-h" => Ok((0, format!("{USAGE}\n"))),
        other => Err(format!("\"{other}\" is not a command.\n\n{USAGE}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sh(a: &[&str]) -> (i32, String) {
        run(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
    #[test]
    fn the_command_line_solves_a_model_and_writes_its_files() {
        let dir = std::env::temp_dir().join(format!("datum-flow-cli-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = |n: &str| dir.join(n).to_string_lossy().into_owned();
        assert_eq!(sh(&["example", "channel", &p("channel.dflow")]).0, 0);
        let (code, out) = sh(&["solve", &p("channel.dflow"), "--vtu", &p("out.vtu"), "--lines", &p("line"), "--probes", &p("probes.csv")]);
        assert!(code == 0 && out.contains("ok: Ready to solve.") && out.contains("Converged in") && out.contains("Pressure drop") && out.contains("0.6012 Pa") && out.contains("mass conservation"), "{out}");
        assert!(std::fs::read_to_string(p("out.vtu")).unwrap().contains("UnstructuredGrid") && std::fs::read_to_string(p("line-1.csv")).unwrap().lines().count() == 52 && std::path::Path::new(&p("probes.csv")).exists());
        // --save keeps the results; solving that file again starts from them.
        assert_eq!(sh(&["solve", &p("channel.dflow"), "--save", &p("solved.dflow")]).0, 0);
        let (_, _, kept) = file::read_all(&std::fs::read_to_string(p("solved.dflow")).unwrap()).unwrap();
        assert!(kept.is_some());
        let (code, out) = sh(&["solve", &p("solved.dflow")]);
        assert!(code == 0 && out.contains("starting from the results kept in the file") && out.contains("Converged in 0 iterations"), "{out}");
        let (code, out) = sh(&["solve", &p("channel.dflow"), "--json"]);
        assert!(code == 0 && out.contains("\"converged\":true"));
        assert!(sh(&["mesh", &p("channel.dflow")]).1.contains("Angles:"));
        // A setup that cannot be solved: exit code 1 and the reason.
        let mut m = examples::channel();
        m.boundaries[1].edges.clear();
        std::fs::write(p("bad.dflow"), file::write(&m)).unwrap();
        let (code, out) = sh(&["check", &p("bad.dflow")]);
        assert!(code == 1 && out.contains("error: 1 edge has no boundary") && out.contains("no outlet"), "{out}");
        assert_eq!(sh(&["solve", &p("bad.dflow")]).0, 1);
        // Files that are missing or damaged, and commands that do not exist: exit code 2, in words.
        std::fs::write(p("cut.dflow"), &file::write(&m)[..200]).unwrap();
        assert!(sh(&["solve", &p("cut.dflow")]) == (2, "This file is damaged and cannot be read (line 9): it is not complete, or not a Datum Flow model.\n".into()) || sh(&["solve", &p("cut.dflow")]).1.contains("damaged"));
        assert!(sh(&["solve", &p("nope.dflow")]).1.starts_with("Could not open") && sh(&["solve"]).0 == 2 && sh(&["fly"]).0 == 2 && sh(&["example", "nope"]).0 == 2);
        assert!(sh(&[]).1.contains("flow-cli solve") && sh(&["examples"]).1.contains("cylinder") && sh(&["version"]).1.starts_with("Datum Flow "));
        let (code, out) = sh(&["verify"]);
        assert!(code == 0 && out.contains("PASS Cylinder in a channel, Re 20 | Drag coefficient") && out.contains("checks pass"), "{out}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
