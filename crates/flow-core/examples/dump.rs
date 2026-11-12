//! For the window's tests (tests_ui/run.cjs).
//!
//! `cargo run -p flow-core --example dump` prints the state the window is sent for every example
//! (the fixture). With `--bridge` it stays open and answers the window's commands one per line,
//! `{"id":1,"cmd":"apply","args":{"op":{…}}}` → `{"id":1,"ok":{…}}` or `{"id":1,"err":"…"}`, so
//! the tests drive the real model and its edits, without the desktop shell.

use flow_core::doc::{Doc, Op};
use flow_core::{examples, file, fluid, wire};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// What the shell adds to the engine's `Out` (src-tauri/src/main.rs `Out`), kept in step by hand.
fn state(doc: &Doc, path: &Option<String>, saved: u64, notes: &[String]) -> Value {
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
    o.insert("has_mesh".into(), json!(false));
    o.insert("has_results".into(), json!(false));
    v
}

fn bridge() {
    let (mut doc, mut path, mut saved, mut notes) = (Doc::new(examples::example("empty").expect("empty")), None::<String>, 0u64, Vec::<String>::new());
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(req) = serde_json::from_str::<Value>(&line) else { continue };
        let (id, cmd, args) = (req["id"].clone(), req["cmd"].as_str().unwrap_or(""), &req["args"]);
        let res: Result<Value, String> = (|| match cmd {
            "state" => Ok(state(&doc, &path, saved, &notes)),
            "apply" => {
                let op: Op = serde_json::from_value(args["op"].clone()).map_err(|e| format!("That edit could not be read: {e}"))?;
                doc.apply(op)?;
                Ok(state(&doc, &path, saved, &notes))
            }
            "undo" => { doc.undo(); Ok(state(&doc, &path, saved, &notes)) }
            "redo" => { doc.redo(); Ok(state(&doc, &path, saved, &notes)) }
            "new_doc" => {
                let m = examples::example(args["id"].as_str().unwrap_or("")).ok_or("That example does not exist.")?;
                doc = Doc::new(m);
                (path, saved, notes) = (None, 0, vec![]);
                Ok(state(&doc, &path, saved, &notes))
            }
            // The tests hand over the file's text instead of a path on disk.
            "open" => {
                let (m, n) = file::read(args["text"].as_str().ok_or("No file was given.")?)?;
                doc = Doc::new(m);
                (path, saved, notes) = (Some(args["path"].as_str().unwrap_or("C:/Models/Opened.dflow").to_string()), 0, n);
                Ok(state(&doc, &path, saved, &notes))
            }
            "save" => {
                path = Some(path.clone().unwrap_or_else(|| format!("C:/Models/{}.dflow", doc.m.name)));
                saved = doc.revision;
                Ok(state(&doc, &path, saved, &notes))
            }
            "file_text" => Ok(json!(file::write(&doc.m))),
            "fluid_preset" => fluid::preset(args["name"].as_str().unwrap_or(""), args["t"].as_f64().unwrap_or(293.15)).map(|f| json!(f)).ok_or_else(|| "That fluid is not built in.".to_string()),
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
    let states: serde_json::Map<String, Value> = examples::EXAMPLES.iter().map(|(id, _, _)| (id.to_string(), state(&Doc::new(examples::example(id).expect("an example")), &None, 0, &[]))).collect();
    println!("{}", json!({ "states": states }));
}
