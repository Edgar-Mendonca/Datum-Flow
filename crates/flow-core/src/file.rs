//! The `.dflow` file: versioned JSON. Reading is forgiving: a damaged file opens as far as it
//! can, and the notes say what was left out; a file that cannot be read at all gives a clear error.

use crate::model::*;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// The largest file read (bytes).
pub const MAX_BYTES: usize = 64 * 1024 * 1024;

/// Write the model as a `.dflow` document.
pub fn write(m: &Model) -> String {
    write_with(m, None)
}

/// Write the model, and its mesh and results when given (see `flow_cli::Engine::to_file`).
pub fn write_with(m: &Model, results: Option<&Value>) -> String {
    let mut v = serde_json::json!({ "app": "Datum Flow", "format": FORMAT, "version": env!("CARGO_PKG_VERSION"), "model": m });
    if let Some(r) = results {
        v["results"] = r.clone();
    }
    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
}

fn take<T: DeserializeOwned + Default>(o: &serde_json::Map<String, Value>, key: &str, what: &str, notes: &mut Vec<String>) -> T {
    match o.get(key) {
        None | Some(Value::Null) => T::default(),
        Some(v) => serde_json::from_value(v.clone()).unwrap_or_else(|_| {
            notes.push(format!("{what} could not be read and was reset."));
            T::default()
        }),
    }
}
fn list<T: DeserializeOwned>(o: &serde_json::Map<String, Value>, key: &str, what: &str, notes: &mut Vec<String>) -> Vec<T> {
    match o.get(key) {
        None | Some(Value::Null) => vec![],
        Some(Value::Array(a)) => {
            let out: Vec<T> = a.iter().take(100_000).filter_map(|v| serde_json::from_value(v.clone()).ok()).collect();
            if out.len() != a.len() {
                notes.push(format!("{} of the {what} could not be read and were left out.", a.len() - out.len()));
            }
            out
        }
        Some(_) => {
            notes.push(format!("The {what} could not be read and were left out."));
            vec![]
        }
    }
}

/// Older formats are brought up to date here, one step per version.
fn migrate(_from: u32, _model: &mut serde_json::Map<String, Value>, _notes: &mut [String]) {
    // Format 1 is the first; nothing to do yet.
}

/// Read a `.dflow` document. Returns the model and notes about anything that had to be left out
/// or reset. An error means nothing could be read.
pub fn read(text: &str) -> Result<(Model, Vec<String>), String> {
    read_all(text).map(|(m, notes, _)| (m, notes))
}

/// Read a `.dflow` document with the stored mesh and results, if it has them (still to be
/// checked against the model by whoever uses them).
pub fn read_all(text: &str) -> Result<(Model, Vec<String>, Option<Value>), String> {
    if text.len() > MAX_BYTES {
        return Err("This file is too large to be a Datum Flow model.".into());
    }
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| {
        if text.trim().is_empty() { "This file is empty.".to_string() } else { format!("This file is damaged and cannot be read (line {}): it is not complete, or not a Datum Flow model.", e.line()) }
    })?;
    let root = v.as_object().ok_or("This is not a Datum Flow model.")?;
    if root.get("app").and_then(|a| a.as_str()) != Some("Datum Flow") {
        return Err("This is not a Datum Flow model.".into());
    }
    let format = root.get("format").and_then(|f| f.as_u64()).unwrap_or(0) as u32;
    if format == 0 {
        return Err("This file does not say which format it is; it cannot be read.".into());
    }
    let mut notes = vec![];
    if format > FORMAT {
        notes.push("This file was written by a newer version of Datum Flow; anything this version does not know was left out.".to_string());
    }
    let mut o = root.get("model").and_then(|m| m.as_object()).cloned().ok_or("This file has no model in it.")?;
    if format < FORMAT {
        migrate(format, &mut o, &mut notes);
    }
    let d = Model::default();
    let mut m = Model {
        name: o.get("name").and_then(|n| n.as_str()).map(|s| s.chars().take(80).collect()).unwrap_or(d.name),
        notes: o.get("notes").and_then(|n| n.as_str()).map(|s| s.chars().take(4000).collect()).unwrap_or_default(),
        units: take(&o, "units", "The unit system", &mut notes),
        grid: o.get("grid").and_then(|g| g.as_f64()).unwrap_or(d.grid),
        shapes: list(&o, "shapes", "shapes", &mut notes),
        fluid: take(&o, "fluid", "The fluid", &mut notes),
        physics: take(&o, "physics", "The physics settings", &mut notes),
        boundaries: list(&o, "boundaries", "boundaries", &mut notes),
        mesh: take(&o, "mesh", "The mesh settings", &mut notes),
        solver: take(&o, "solver", "The solver settings", &mut notes),
        monitors: list(&o, "monitors", "monitors", &mut notes),
        probes: list(&o, "probes", "probes", &mut notes),
        lines: list(&o, "lines", "plot lines", &mut notes),
        view: take(&o, "view", "The view settings", &mut notes),
    };
    notes.extend(m.normalize());
    let results = root.get("results").filter(|r| r.is_object()).cloned();
    Ok((m, notes, results))
}
