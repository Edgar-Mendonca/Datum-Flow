//! For the window's tests (tests_ui/run.cjs).
//!
//! `cargo run -p flow-cli --example bridge` prints the state the window is sent for every example
//! (the fixture). With `--bridge` it stays open and answers the window's commands one per line,
//! `{"id":1,"cmd":"apply","args":{"op":{…}}}` → `{"id":1,"ok":{…}}` or `{"id":1,"err":"…"}`, so
//! the tests drive the real engine (edits, meshing, solving, results), without the desktop shell.

use flow_cli::Engine;
use flow_core::doc::{keeps_results, Doc, Op};
use flow_core::{examples, file, fluid, wire};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// What the shell adds to the engine's `Out` (src-tauri/src/main.rs `Out`), kept in step by hand.
fn state(doc: &Doc, path: &Option<String>, saved: u64, notes: &[String], eng: &Engine) -> Value {
    let mut v = serde_json::to_value(wire::out(doc)).expect("state");
    let named = |id: &str, title: &str, about: &str| json!({ "id": id, "title": title, "about": about });
    let o = v.as_object_mut().expect("an object");
    o.insert("path".into(), json!(path));
    o.insert("file".into(), json!(path.as_ref().and_then(|p| p.rsplit(['/', '\\']).next()).map(|f| f.trim_end_matches(".dflow").to_string()).unwrap_or_else(|| doc.m.name.clone())));
    o.insert("dirty".into(), json!(doc.revision != saved));
    o.insert("notes".into(), json!(notes));
    o.insert("version".into(), json!(env!("CARGO_PKG_VERSION")));
    o.insert("examples".into(), examples::EXAMPLES.iter().map(|(id, t, a)| named(id, t, a)).collect());
    o.insert("templates".into(), examples::TEMPLATES.iter().map(|(id, t)| named(id, t, examples::EXAMPLES.iter().find(|e| e.0 == *id).map(|e| e.2).unwrap_or(""))).collect());
    o.insert("has_mesh".into(), json!(eng.has_mesh() && eng.revision == doc.revision));
    o.insert("has_results".into(), json!(eng.has_results() && eng.revision == doc.revision));
    v
}

fn bridge() {
    let (mut doc, mut path, mut saved, mut notes) = (Doc::new(examples::example("empty").expect("empty")), None::<String>, 0u64, Vec::<String>::new());
    let mut eng = Engine::default();
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(req) = serde_json::from_str::<Value>(&line) else { continue };
        let (id, cmd, args) = (req["id"].clone(), req["cmd"].as_str().unwrap_or(""), &req["args"]);
        let res: Result<Value, String> = (|| match cmd {
            "state" => Ok(state(&doc, &path, saved, &notes, &eng)),
            "apply" => {
                let op: Op = serde_json::from_value(args["op"].clone()).map_err(|e| format!("That edit could not be read: {e}"))?;
                let (keep, before) = (keeps_results(&op), doc.revision);
                doc.apply(op)?;
                if doc.revision != before {
                    if keep && eng.revision == before { eng.revision = doc.revision } else if !keep { eng.invalidate() }
                }
                Ok(state(&doc, &path, saved, &notes, &eng))
            }
            "undo" => { doc.undo(); eng.invalidate(); Ok(state(&doc, &path, saved, &notes, &eng)) }
            "redo" => { doc.redo(); eng.invalidate(); Ok(state(&doc, &path, saved, &notes, &eng)) }
            "new_doc" => {
                let m = examples::example(args["id"].as_str().unwrap_or("")).ok_or("That example does not exist.")?;
                doc = Doc::new(m);
                eng.invalidate();
                (path, saved, notes) = (None, 0, vec![]);
                Ok(state(&doc, &path, saved, &notes, &eng))
            }
            // The tests hand over the file's text instead of a path on disk.
            "open" => {
                let (m, mut n, results) = file::read_all(args["text"].as_str().ok_or("No file was given.")?)?;
                doc = Doc::new(m);
                eng.invalidate();
                if let Some(r) = results {
                    if let Err(e) = eng.from_file(&doc.m, &r, doc.revision) { n.push(e); }
                }
                (path, saved, notes) = (Some(args["path"].as_str().unwrap_or("C:/Models/Opened.dflow").to_string()), 0, n);
                Ok(state(&doc, &path, saved, &notes, &eng))
            }
            "save" => {
                path = Some(path.clone().unwrap_or_else(|| format!("C:/Models/{}.dflow", doc.m.name)));
                saved = doc.revision;
                Ok(state(&doc, &path, saved, &notes, &eng))
            }
            "file_text" => Ok(json!(file::write_with(&doc.m, if args["results"].as_bool() == Some(false) { None } else { eng.to_file(&doc.m, doc.revision) }.as_ref()))),
            "fluid_preset" => fluid::preset(args["name"].as_str().unwrap_or(""), args["t"].as_f64().unwrap_or(293.15)).map(|f| json!(f)).ok_or_else(|| "That fluid is not built in.".to_string()),
            "import_dxf" => {
                let (outlines, n) = flow_core::dxf::read(args["text"].as_str().ok_or("No drawing was given.")?, args["scale"].as_f64().unwrap_or(1.0))?;
                doc.apply(Op::AddShapes { shapes: outlines.into_iter().map(|o| (o.name, o.hole, o.geom)).collect() })?;
                eng.invalidate();
                notes = n.into_iter().map(|x| format!("DXF import: {x}")).collect();
                Ok(state(&doc, &path, saved, &notes, &eng))
            }
            "mesh" => eng.mesh(&doc.m, doc.revision).map(|v| json!(v)),
            "mesh_view" => Ok(json!(if eng.revision == doc.revision { eng.mesh_view() } else { None })),
            "check" => Ok(json!(if eng.revision == doc.revision { eng.check(&doc.m) } else { flow_mesh::check(&doc.m, None) })),
            // The window's worker thread and its events, in one answer: every iteration, then the end.
            "solve" => {
                let stop_after = args["stop_after"].as_u64().unwrap_or(u64::MAX);
                let mut progress = vec![];
                let summary = eng.solve_now(&doc.m, doc.revision, &mut |p| { progress.push(json!(p)); (progress.len() as u64) < stop_after })?;
                Ok(json!({ "progress": progress, "summary": summary, "out": state(&doc, &path, saved, &notes, &eng) }))
            }
            "summary" => Ok(json!(if eng.revision == doc.revision { eng.summary() } else { None })),
            "field" => eng.field(args["id"].as_str().unwrap_or(""), args["lines"].as_u64().unwrap_or(24) as usize).map(|v| json!(v)),
            "arrows" => eng.arrows(&doc.m, args["count"].as_u64().unwrap_or(400) as usize).map(|v| json!(v)),
            "probe" => eng.probe(args["x"].as_f64().unwrap_or(f64::NAN), args["y"].as_f64().unwrap_or(f64::NAN)).map(|v| json!(v)),
            "probes" => eng.probes(&doc.m).map(|v| json!(v)),
            "line" => eng.line(&doc.m, args["index"].as_u64().unwrap_or(0) as usize).map(|v| json!(v)),
            "values" => eng.values(&doc.m).map(|v| json!(v)),
            "export_text" => match args["what"].as_str().unwrap_or("") {
                "vtu" => eng.vtu(),
                "probes" => eng.probes_csv(&doc.m),
                "line" => eng.line_csv(&doc.m, args["index"].as_u64().unwrap_or(0) as usize),
                _ => Err("That export does not exist.".into()),
            }.map(|v| json!(v)),
            // The record shipped with this version, read from the verification crate's folder.
            "verification" => std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../flow-verify/report.json")).map_err(|e| e.to_string()).and_then(|t| serde_json::from_str::<Value>(&t).map_err(|e| e.to_string())),
            _ => Err(format!("unknown command {cmd}")),
        })();
        let reply = match res { Ok(v) => json!({ "id": id, "ok": v }), Err(e) => json!({ "id": id, "err": e }) };
        let _ = writeln!(out, "{reply}");
        let _ = out.flush();
    }
}

fn main() {
    if std::env::args().any(|a| a == "--bridge") {
        return bridge();
    }
    let states: serde_json::Map<String, Value> = examples::EXAMPLES.iter().map(|(id, _, _)| (id.to_string(), state(&Doc::new(examples::example(id).expect("an example")), &None, 0, &[], &Engine::default()))).collect();
    println!("{}", json!({ "states": states }));
}
