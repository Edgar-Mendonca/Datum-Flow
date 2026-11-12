//! Datum Flow's desktop shell: the window's commands, files, options, recovery and the updater.
//! Everything that calculates lives in the engine crates; this file only passes it on.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf;
mod update;

use flow_core::doc::{keeps_results, Doc, Op};
use flow_core::model::{Fluid, Model, Units};
use flow_core::{examples, file, fluid, wire};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_dialog::DialogExt;

const GITHUB: &str = "https://github.com/Edgar-Mendonca/Datum-Flow";
const SITE: &str = "https://edgar-mendonca.github.io/Datum";

// ---------------------------------------------------------------- state

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Options {
    /// Minutes between recovery saves; 0 turns them off.
    autosave: u32,
    beta: bool,
    /// auto, light or dark.
    theme: String,
    open_last: bool,
    /// Display units for new models: SI or US.
    units: String,
    snap: bool,
    grid: bool,
    /// Colour map new models start with: viridis, inferno or cividis.
    map: String,
    paper: String,
    landscape: bool,
    company: String,
}

impl Default for Options {
    fn default() -> Self {
        Self { autosave: 5, beta: false, theme: "auto".into(), open_last: false, units: "SI".into(), snap: true, grid: true, map: "viridis".into(), paper: "A4".into(), landscape: true, company: String::new() }
    }
}

/// What the engine has worked out for the open model. The mesh and the results are kept here,
/// beside the document, and dropped when an edit makes them out of date.
#[derive(Default)]
struct Engine {
    // ENGINE: the mesher's output goes here, e.g. `mesh: Option<flow_mesh::Mesh>`.
    // ENGINE: the solver's output goes here, e.g. `results: Option<flow_solve::Solution>`.
    /// The document revision the mesh and results belong to.
    revision: u64,
}

impl Engine {
    /// The model changed in a way that makes the mesh and the results out of date.
    fn invalidate(&mut self) {
        *self = Engine::default();
    }
    fn has_mesh(&self) -> bool {
        false // ENGINE: self.mesh.is_some()
    }
    fn has_results(&self) -> bool {
        false // ENGINE: self.results.is_some()
    }
}

struct App {
    doc: Doc,
    path: Option<PathBuf>,
    /// The revision that is on disk; None for a model never saved.
    saved: Option<u64>,
    /// What a damaged file lost or changed when it was opened (shown under Messages).
    notes: Vec<String>,
    opts: Options,
    engine: Engine,
}

impl App {
    fn new(opts: Options) -> App {
        let mut a = App { doc: Doc::new(Model::default()), path: None, saved: None, notes: vec![], opts, engine: Engine::default() };
        a.load(Model::default(), None, vec![]);
        a
    }
    /// Replace the open model: a new, opened or recovered one.
    fn load(&mut self, mut m: Model, path: Option<PathBuf>, notes: Vec<String>) {
        if path.is_none() && m.shapes.is_empty() {
            m.units = if self.opts.units == "US" { Units::US } else { Units::SI };
            if ["viridis", "inferno", "cividis"].contains(&self.opts.map.as_str()) {
                m.view.map = self.opts.map.clone();
            }
        }
        self.doc = Doc::new(m);
        // A model just loaded (a file, an example, a blank drawing) has nothing to lose yet.
        self.saved = Some(self.doc.revision);
        self.path = path;
        self.notes = notes;
        self.engine.invalidate();
    }
    fn dirty(&self) -> bool {
        self.saved != Some(self.doc.revision)
    }
}

struct St(Mutex<App>);

impl St {
    /// A command that panicked must not lock every later one out, so a poisoned lock is reused.
    fn lock(&self) -> std::sync::MutexGuard<'_, App> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

type R<T> = Result<T, String>;

/// Runs an action so that an internal error (a panic) becomes an error message instead of
/// closing the app. Edits work on a copy of the model, so the model is left as it was.
fn guard<T>(f: impl FnOnce() -> R<T>) -> R<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| {
        Err("Datum Flow met an internal error and stopped that action; your model is as it was. Details are in crash.log; please report it (Help › Report an Issue).".into())
    })
}

#[derive(Serialize)]
struct Named {
    id: &'static str,
    title: &'static str,
    about: &'static str,
}

/// What the window gets after every action: the engine's view of the model (`wire::Out`) plus
/// what the shell owns.
#[derive(Serialize)]
struct Out {
    #[serde(flatten)]
    out: wire::Out,
    path: Option<String>,
    /// The file's name without its folder and extension, or the model's name.
    file: String,
    dirty: bool,
    notes: Vec<String>,
    version: &'static str,
    examples: Vec<Named>,
    templates: Vec<Named>,
    has_mesh: bool,
    has_results: bool,
}

fn out(a: &App) -> Out {
    Out {
        out: wire::out(&a.doc),
        path: a.path.as_ref().map(|p| p.display().to_string()),
        file: a.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| a.doc.m.name.clone()),
        dirty: a.dirty(),
        notes: a.notes.clone(),
        version: env!("CARGO_PKG_VERSION"),
        examples: examples::EXAMPLES.iter().map(|(id, title, about)| Named { id, title, about }).collect(),
        templates: examples::TEMPLATES.iter().map(|(id, title)| Named { id, title, about: examples::EXAMPLES.iter().find(|e| e.0 == *id).map(|e| e.2).unwrap_or("") }).collect(),
        has_mesh: a.engine.has_mesh() && a.engine.revision == a.doc.revision,
        has_results: a.engine.has_results() && a.engine.revision == a.doc.revision,
    }
}

fn dir(app: &AppHandle, data: bool) -> Option<PathBuf> {
    let d = if data { app.path().app_data_dir() } else { app.path().app_config_dir() }.ok()?;
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

// ---------------------------------------------------------------- the model and its edits

#[tauri::command]
fn state(st: State<St>) -> Out {
    out(&st.lock())
}

#[tauri::command]
fn apply(st: State<St>, op: Op) -> R<Out> {
    let mut a = st.lock();
    guard(|| {
        let keep = keeps_results(&op);
        let before = a.doc.revision;
        a.doc.apply(op)?;
        if a.doc.revision != before {
            if keep {
                // The mesh and results still describe this model: they follow it to its new revision.
                if a.engine.revision == before {
                    a.engine.revision = a.doc.revision;
                }
            } else {
                a.engine.invalidate();
            }
        }
        Ok(out(&a))
    })
}

#[tauri::command]
fn undo(st: State<St>) -> Out {
    let mut a = st.lock();
    if a.doc.undo() {
        a.engine.invalidate();
    }
    out(&a)
}

#[tauri::command]
fn redo(st: State<St>) -> Out {
    let mut a = st.lock();
    if a.doc.redo() {
        a.engine.invalidate();
    }
    out(&a)
}

/// A new model from an example or template id ("empty" for a blank drawing).
#[tauri::command]
fn new_doc(app: AppHandle, st: State<St>, id: String) -> R<Out> {
    let mut a = st.lock();
    guard(|| {
        let m = examples::example(&id).ok_or("That example does not exist.")?;
        a.load(m, None, vec![]);
        Ok(out(&a))
    })
    .inspect(|_| clear_recovery(&app))
}

/// A built-in fluid at a temperature (K), for the Physics inspector.
#[tauri::command]
fn fluid_preset(name: String, t: f64) -> R<Fluid> {
    fluid::preset(&name, t).ok_or_else(|| "That fluid is not built in.".into())
}

// ENGINE: `mesh`, `check`, `solve_start` / `solve_stop` (a worker thread that emits
// "solve-progress" events), `results`, `report`, `export_*` and `verification` go here. They read
// `a.doc.m`, keep what they make in `a.engine` with `a.engine.revision = a.doc.revision`, and
// return plain serialisable values. Register them in `main()` below.

// ---------------------------------------------------------------- files

fn pick(app: &AppHandle, name: &str, exts: &[&str]) -> Option<PathBuf> {
    app.dialog().file().add_filter(name, exts).blocking_pick_file().and_then(|f| f.into_path().ok())
}

/// Read a text file, refusing anything too large to be a model.
fn read_limited(path: &Path) -> R<String> {
    let len = std::fs::metadata(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?.len();
    if len > file::MAX_BYTES as u64 {
        return Err("This file is too large to be a Datum Flow model.".into());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[tauri::command]
async fn open(app: AppHandle, st: State<'_, St>, path: Option<String>) -> R<Option<Out>> {
    let Some(path) = path.map(PathBuf::from).or_else(|| pick(&app, "Datum Flow model", &["dflow"])) else { return Ok(None) };
    let text = read_limited(&path)?;
    let mut a = st.lock();
    guard(|| {
        let (m, notes) = file::read(&text)?;
        a.load(m, Some(path.clone()), notes);
        Ok(())
    })?;
    remember(&app, &path);
    clear_recovery(&app);
    Ok(Some(out(&a)))
}

/// Save the model: to `path` when given, to its own file, or (never saved, or `as_new`) where the
/// person chooses. None when they cancelled.
#[tauri::command]
async fn save(app: AppHandle, st: State<'_, St>, path: Option<String>, as_new: Option<bool>) -> R<Option<Out>> {
    let (known, name) = {
        let a = st.lock();
        (path.map(PathBuf::from).or_else(|| a.path.clone().filter(|_| as_new != Some(true))), a.doc.m.name.clone())
    };
    let path = match known {
        Some(p) => p,
        None => {
            let safe: String = name.chars().filter(|c| !"\\/:*?\"<>|".contains(*c)).collect();
            match app.dialog().file().add_filter("Datum Flow model", &["dflow"]).set_file_name(format!("{}.dflow", safe.trim())).blocking_save_file() {
                Some(f) => f.into_path().map_err(|e| e.to_string())?,
                None => return Ok(None),
            }
        }
    };
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("dflow")) {
        return Err("Datum Flow models are saved as .dflow files.".into());
    }
    let mut a = st.lock();
    write_safely(&path, file::write(&a.doc.m).as_bytes()).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
    a.saved = Some(a.doc.revision);
    a.path = Some(path.clone());
    remember(&app, &path);
    clear_recovery(&app);
    Ok(Some(out(&a)))
}

/// Text that a spreadsheet would run as a formula gets an apostrophe in front (CSV exports).
// ENGINE: the CSV exports of probes and line plots pass every text cell through this.
#[allow(dead_code)]
pub fn csv_safe(cell: &str) -> String {
    let risky = cell.starts_with(['=', '+', '-', '@']) && cell.parse::<f64>().is_err();
    let cell = if risky { format!("'{cell}") } else { cell.to_string() };
    if cell.contains([',', '"', '\n']) { format!("\"{}\"", cell.replace('"', "\"\"")) } else { cell }
}

/// Saves a picture or a file the window made (a PNG data URL, CSV or other text) where the
/// person chooses. Only export file types; returns the path, None when cancelled.
#[tauri::command]
async fn save_file(app: AppHandle, name: String, data: String) -> R<Option<String>> {
    let name: String = name.chars().filter(|c| !"\\/:*?\"<>|".contains(*c)).collect();
    let ext = name.rsplit('.').next().unwrap_or("txt").to_lowercase();
    if !["png", "csv", "vtu", "txt", "svg", "md"].contains(&ext.as_str()) {
        return Err("That kind of file cannot be saved from here.".into());
    }
    let Some(path) = app.dialog().file().add_filter(ext.to_uppercase(), &[&ext]).set_file_name(&name).blocking_save_file().and_then(|f| f.into_path().ok()) else { return Ok(None) };
    let bytes = match data.strip_prefix("data:").and_then(|d| d.split_once(";base64,")) {
        Some((_, b64)) => base64_decode(b64)?,
        None => data.into_bytes(),
    };
    write_safely(&path, &bytes).map_err(|e| format!("Could not save {}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

/// File › Print › Save as PDF: asks where, then writes the pages the window has laid out for
/// printing straight to that file (Windows); returns the path, None when cancelled.
#[tauri::command]
async fn save_pdf(app: AppHandle, window: tauri::WebviewWindow, name: String, w_mm: f64, h_mm: f64, margins: [f64; 4]) -> R<Option<String>> {
    if cfg!(not(windows)) {
        return Err("native-pdf-unavailable".into());
    }
    let name: String = name.chars().filter(|c| !"\\/:*?\"<>|".contains(*c)).collect();
    let Some(path) = app.dialog().file().add_filter("PDF", &["pdf"]).set_file_name(format!("{}.pdf", name.trim())).blocking_save_file().and_then(|f| f.into_path().ok()) else { return Ok(None) };
    pdf::write(&window, &path, w_mm, h_mm, margins.map(|m| m.clamp(0.0, 60.0))).await?;
    Ok(Some(path.display().to_string()))
}

/// Write a file so a crash or a full disk never leaves it half written: write a temporary file
/// beside it, flush it to disk, then rename it over the old one.
fn write_safely(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension(format!("{}.saving", path.extension().and_then(|e| e.to_str()).unwrap_or("tmp")));
    let res = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

fn base64_decode(s: &str) -> R<Vec<u8>> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        acc = (acc << 6 | val(c).ok_or("The picture data is damaged.")? as u32) & 0x00FF_FFFF;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- options, recovery, recent

fn options_file(app: &AppHandle) -> Option<PathBuf> {
    dir(app, false).map(|d| d.join("options.json"))
}

#[tauri::command]
fn options(st: State<St>) -> Options {
    st.lock().opts.clone()
}

#[tauri::command]
fn set_options(app: AppHandle, st: State<St>, opts: Options) -> Out {
    if let Some(f) = options_file(&app) {
        let _ = write_safely(&f, serde_json::to_string(&opts).unwrap_or_default().as_bytes());
    }
    let mut a = st.lock();
    a.opts = opts;
    out(&a)
}

#[derive(Serialize, Deserialize)]
struct Recovery {
    name: String,
    path: Option<String>,
    saved: String,
}

fn recovery_files(app: &AppHandle) -> Option<(PathBuf, PathBuf)> {
    dir(app, true).map(|d| (d.join("recovery.dflow"), d.join("recovery.json")))
}

fn clear_recovery(app: &AppHandle) {
    if let Some((a, b)) = recovery_files(app) {
        let _ = std::fs::remove_file(a);
        let _ = std::fs::remove_file(b);
    }
}

#[tauri::command]
fn autosave(app: AppHandle, st: State<St>) {
    let a = st.lock();
    if let (true, Some((file, meta))) = (a.dirty(), recovery_files(&app)) {
        let info = Recovery { name: a.doc.m.name.clone(), path: a.path.as_ref().map(|p| p.display().to_string()), saved: chrono::Local::now().format("%d-%b-%y %H:%M").to_string() };
        let _ = write_safely(&file, file::write(&a.doc.m).as_bytes());
        let _ = write_safely(&meta, serde_json::to_string(&info).unwrap_or_default().as_bytes());
    }
}

#[tauri::command]
fn recovery(app: AppHandle) -> Option<Recovery> {
    let (file, meta) = recovery_files(&app)?;
    file.exists().then_some(())?;
    serde_json::from_str(&read_limited(&meta).ok()?).ok()
}

#[tauri::command]
fn recover(app: AppHandle, st: State<St>, keep: bool) -> R<Option<Out>> {
    let info = recovery(app.clone());
    let files = recovery_files(&app);
    if !keep {
        clear_recovery(&app);
        return Ok(None);
    }
    let (file, _) = files.ok_or("No recovered model was found.")?;
    let text = read_limited(&file)?;
    let mut a = st.lock();
    guard(|| {
        let (m, notes) = file::read(&text)?;
        a.load(m, info.and_then(|i| i.path).map(PathBuf::from), notes);
        // Recovered work is not on disk yet.
        a.saved = None;
        Ok(Some(out(&a)))
    })
}

fn recent_paths(app: &AppHandle) -> Vec<String> {
    options_file(app).map(|f| f.with_file_name("recent.json")).and_then(|f| read_limited(&f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn remember(app: &AppHandle, path: &Path) {
    let p = path.display().to_string();
    let mut v = recent_paths(app);
    v.retain(|x| *x != p);
    v.insert(0, p);
    v.truncate(12);
    if let Some(f) = options_file(app).map(|f| f.with_file_name("recent.json")) {
        let _ = write_safely(&f, serde_json::to_string(&v).unwrap_or_default().as_bytes());
    }
}

#[derive(Serialize)]
struct Recent {
    path: String,
    name: String,
    modified: String,
}

#[tauri::command]
fn recent(app: AppHandle) -> Vec<Recent> {
    recent_paths(&app)
        .into_iter()
        .filter_map(|p| {
            let m = std::fs::metadata(&p).ok()?.modified().ok()?;
            let modified = chrono::DateTime::<chrono::Local>::from(m).format("%d-%b-%y %H:%M").to_string();
            let name = Path::new(&p).file_stem()?.to_string_lossy().into_owned();
            Some(Recent { path: p, name, modified })
        })
        .collect()
}

/// The file given on the command line (a double-clicked .dflow), or the last one when asked.
#[tauri::command]
fn startup_file(app: AppHandle, st: State<St>) -> Option<String> {
    let last = || if st.lock().opts.open_last { recent_paths(&app).into_iter().find(|p| Path::new(p).exists()) } else { None };
    std::env::args().skip(1).find(|a| a.to_lowercase().ends_with(".dflow")).or_else(last)
}

/// Whether a link may be opened: the project's own GitHub pages and the Datum site only (an exact
/// prefix, then plain URL characters).
fn link_allowed(url: &str) -> bool {
    [GITHUB, SITE].iter().any(|base| url.strip_prefix(base).is_some_and(|rest| (rest.is_empty() || rest.starts_with('/')) && rest.chars().all(|c| c.is_ascii_alphanumeric() || "/-_.#?=&".contains(c)) && !rest.contains("..")))
}

/// Opens one of the project's pages in the browser (only those).
#[tauri::command]
fn open_link(url: String) -> R<()> {
    if !link_allowed(&url) {
        return Err("Only the Datum Flow pages on GitHub and the Datum site can be opened.".into());
    }
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(&url).spawn();
    #[cfg(not(windows))]
    let r = std::process::Command::new("xdg-open").arg(&url).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
fn quit(app: AppHandle) {
    clear_recovery(&app);
    app.exit(0);
}

#[tauri::command]
async fn check_update(st: State<'_, St>) -> R<update::Info> {
    let beta = st.lock().opts.beta;
    tauri::async_runtime::spawn_blocking(move || update::check(beta)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn install_update(app: AppHandle, url: String, sums: Option<String>) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || update::install(&app, &url, sums.as_deref())).await.map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Anything that ever panics is written to crash.log (version, place, message) for an issue report.
            if let Some(log) = dir(app.handle(), true).map(|d| d.join("crash.log")) {
                let default = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |info| {
                    use std::io::Write;
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log) {
                        let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
                        let what = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
                        let _ = writeln!(f, "{} Datum Flow {} panicked at {at}: {what}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), env!("CARGO_PKG_VERSION"));
                    }
                    default(info);
                }));
            }
            let opts: Options = options_file(app.handle()).and_then(|f| read_limited(&f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            app.manage(St(Mutex::new(App::new(opts))));
            Ok(())
        })
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                if w.state::<St>().lock().dirty() {
                    api.prevent_close();
                    let _ = w.emit("close-requested", ());
                } else {
                    clear_recovery(w.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            state, apply, undo, redo, new_doc, fluid_preset, open, save, save_file, save_pdf, options, set_options, autosave, recovery, recover, recent, startup_file, open_link, version, quit, check_update, install_update
        ])
        .run(tauri::generate_context!())
        .expect("Datum Flow failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saving_replaces_the_file_whole_and_leaves_nothing_behind() {
        let dir = std::env::temp_dir().join(format!("datum-flow-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("model.dflow");
        write_safely(&f, b"first, and longer than the second").unwrap();
        write_safely(&f, b"second").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"second");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "no temporary file left");
        // A folder that does not exist: an error, and nothing written.
        assert!(write_safely(&dir.join("no/such/model.dflow"), b"x").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_the_projects_own_pages_open() {
        for ok in [GITHUB, "https://github.com/Edgar-Mendonca/Datum-Flow/releases", "https://github.com/Edgar-Mendonca/Datum-Flow/issues/new", "https://edgar-mendonca.github.io/Datum/apps/flow"] {
            assert!(link_allowed(ok), "{ok}");
        }
        for bad in ["https://github.com/Edgar-Mendonca/Datum-Flow.evil.example/x", "https://github.com/Edgar-Mendonca/Datum-Flow/../../other", "https://example.com", "file:///etc/passwd", "https://github.com/Edgar-Mendonca/Datum-Flow/ --flag", "https://edgar-mendonca.github.io/Datumx"] {
            assert!(!link_allowed(bad), "{bad}");
        }
    }

    #[test]
    fn a_model_is_dirty_only_after_a_real_change_and_clean_after_loading() {
        let mut a = App::new(Options::default());
        assert!(!a.dirty());
        a.doc.apply(Op::SetName { name: "Duct".into() }).unwrap();
        assert!(a.dirty());
        a.saved = Some(a.doc.revision);
        assert!(!a.dirty());
        a.doc.undo();
        assert!(a.dirty());
        // A model just loaded has nothing to lose: an example, or an opened file.
        a.load(examples::example("cylinder").unwrap(), None, vec![]);
        assert!(!a.dirty() && a.path.is_none());
        a.load(examples::example("cylinder").unwrap(), Some(PathBuf::from("x.dflow")), vec!["note".into()]);
        assert!(!a.dirty() && out(&a).notes.len() == 1 && out(&a).file == "x");
        // New models take the unit system from Options; examples keep their own.
        a.opts.units = "US".into();
        a.load(Model::default(), None, vec![]);
        assert!(a.doc.m.units == Units::US && !a.dirty());
    }

    #[test]
    fn what_the_window_gets_has_the_engines_fields_and_the_shells() {
        let mut a = App::new(Options::default());
        a.load(examples::example("cylinder").unwrap(), None, vec![]);
        let v = serde_json::to_value(out(&a)).unwrap();
        for key in ["m", "units", "numbers", "edges", "bounds", "can_undo", "path", "file", "dirty", "notes", "version", "examples", "templates", "has_mesh", "has_results"] {
            assert!(v.get(key).is_some(), "{key} is missing");
        }
        assert_eq!(v["examples"].as_array().unwrap().len(), examples::EXAMPLES.len());
        assert_eq!(v["templates"].as_array().unwrap().len(), examples::TEMPLATES.len());
    }

    #[test]
    fn exported_text_is_safe_for_spreadsheets_and_pictures_decode() {
        assert_eq!(csv_safe("=1+1"), "'=1+1");
        assert_eq!(csv_safe("-12.5"), "-12.5");
        assert_eq!(csv_safe("a,b"), "\"a,b\"");
        assert_eq!(csv_safe("@x \"y\""), "\"'@x \"\"y\"\"\"");
        assert_eq!(base64_decode("SGVsbG8=").unwrap(), b"Hello");
        assert!(base64_decode("SGV$bG8=").is_err());
    }

    #[test]
    fn a_panic_in_an_action_becomes_a_message() {
        let r: R<()> = guard(|| panic!("boom"));
        assert!(r.unwrap_err().contains("internal error"));
    }
}
