// The engine's side of the window: the mesh, the checks, the running solve and the results the
// shell holds for the open model, as signals, and the commands that ask for them. Nothing is
// calculated here: every number comes from the engine (crates/flow-cli/src/engine.rs).
import { createSignal, createEffect, createMemo, on, untrack } from "solid-js";
import { listen, Out, Pt, View } from "./api";
import { out, setOut, call, act, toast, say, M, setBottom, setBottomOpen, setSel, setTab, setTool, setPickFor, running, setRunning } from "./store";
export { running };
import { wire } from "./cmds";
import { setClickHook, redraw } from "./canvas";

export interface Quality { elements: number; nodes: number; unknowns: number; min_angle: number; good: number; angle_bands: number[]; max_ratio: number; edges: number[]; worst: { element: number; at: Pt; angle: number }[] }
export interface MeshView { nodes: number[]; nv: number; tris: number[]; edges: number[]; angle: number[]; quality: Quality; size: number; seconds: number }
export interface NodalField { id: string; name: string; kind: string; values: (number | null)[]; min: number; max: number; levels: number[] }
export interface ResRow { iter: number; kind: string; load: number; momentum: number; continuity: number; energy: number }
export interface Summary { converged: boolean; stopped: boolean; message: string; advice: string[]; stats: { unknowns: number; nonzeros: number; seconds: number; linear_solver: string; steps: number }; history: ResRow[]; monitor_names: string[]; monitor_kinds: string[]; monitors: number[][] }
export interface Issue { level: "error" | "warning" | "ok"; text: string; fix: string; boundary: number }
export interface Sample { x: number; y: number; u: number; v: number; speed: number; pressure: number; temperature: number | null; vorticity: number; inside: boolean }
export interface ValueRow { group: string; name: string; value: number; kind: string; note: string }
export interface Check { tier: number; case: string; quantity: string; reference: number; result: number; error: number; tolerance: number; absolute: boolean; pass: boolean; source: string; confirmed: string; note: string; steps: [number, number][] }
export interface Report { version: string; date: string; checks: Check[]; not_verified: string[]; seconds: number; machine: string }

export const [mesh, setMesh] = createSignal<MeshView | null>(null);
/** Colour the mesh by element quality. */
export const [showQuality, setShowQuality] = createSignal(false);
export const [issues, setIssues] = createSignal<Issue[] | null>(null);
/** The field the view is coloured by, the stream function (for streamlines) and the arrows. */
export const [field, setField] = createSignal<NodalField | null>(null);
export const [stream, setStream] = createSignal<NodalField | null>(null);
export const [arrowData, setArrowData] = createSignal<number[][]>([]);
export const [arrowCount, setArrowCount] = createSignal(500);
export const [probeData, setProbeData] = createSignal<[string, Sample][]>([]);
/** A solve is running; its residual rows and monitor values so far. */
export const [rows, setRows] = createSignal<ResRow[]>([]);
export const [monValues, setMonValues] = createSignal<number[][]>([]);
export const [monNames, setMonNames] = createSignal<string[]>([]);
export const [summary, setSummary] = createSignal<Summary | null>(null);
/** The values under the pointer, for the status bar. */
export const [hoverSample, setHoverSample] = createSignal<Sample | null>(null);
/** What the next clicks in the view place: a probe, or the two ends of a plot line. */
export const [placing, setPlacing] = createSignal<"probe" | "line" | null>(null);
export const [lineStart, setLineStart] = createSignal<Pt | null>(null);
export const [lineEnd, setLineEnd] = createSignal<Pt | null>(null);

/** Hooks the dialogs register (they live in simui.tsx, which imports this file). */
export const ui: { advice?: (s: Summary) => void; refused?: (i: Issue) => void; linePlot?: (index: number) => void } = {};

const refreshOut = async () => { const o = await call<Out>("state"); if (o) setOut(o); };
const needResults = () => { if (out()?.has_results) return true; toast("There are no results yet: solve the model first (Solve › Solve)."); return false; };

// ---------- mesh
export const genMesh = async () => {
  if (!out()) return;
  if (running()) return toast("A solve is running: stop it before meshing again.");
  const v = await call<MeshView>("mesh");
  if (!v) return;
  setMesh(v); await refreshOut();
  setSel({ kind: "mesh" });
  toast(`Mesh ready: ${v.quality.elements.toLocaleString()} elements, smallest angle ${v.quality.min_angle.toFixed(0)}°.`, true);
  if (issues()) runChecks(false);
};
export const toggleQuality = async () => {
  if (!mesh()) { await genMesh(); if (!mesh()) return; }
  setShowQuality(!showQuality()); setSel({ kind: "mesh" }); setTab("mesh"); redraw();
};

// ---------- checks
export const runChecks = async (show = true) => {
  const list = await call<Issue[]>("check");
  if (list) setIssues(list);
  if (show) { setBottom("checks"); setBottomOpen(true); }
  return list ?? [];
};

// ---------- solve
export const solve = async () => {
  if (!out()) return;
  if (running()) return toast("A solve is already running.");
  const list = await runChecks(false);
  const bad = list.find(i => i.level === "error");
  if (bad) { setBottom("checks"); setBottomOpen(true); ui.refused?.(bad); return; }
  setRows([]); setMonValues([]); setMonNames(M().monitors.map(m => m.name));
  setTool("select"); setPickFor(null);
  try {
    setRunning(true);
    const names = await window.__TAURI__!.core.invoke("solve_start");
    if (Array.isArray(names)) setMonNames(names);
    setBottom("residuals"); setBottomOpen(true);
  } catch (e) { setRunning(false); toast(String(e)); }
};
export const stop = () => { if (!running()) return toast("No solve is running."); call("solve_stop"); };

const onProgress = (p: { row: ResRow; monitors: number[] }) => { setRows(r => [...r, p.row]); setMonValues(v => [...v, p.monitors]); };
const onDone = async (d: { summary: Summary | null; error: string | null; out: Out }) => {
  setRunning(false);
  setOut(d.out);
  if (d.error) { toast(d.error); return; }
  const s = d.summary!;
  setSummary(s); setRows(s.history); setMonValues(s.monitors); setMonNames(s.monitor_names);
  say(`${s.message} ${s.stats.unknowns.toLocaleString()} unknowns, ${s.stats.seconds.toFixed(1)} s.`, s.converged ? "info" : "warn");
  if (s.converged) toast(s.message, true);
  else if (s.stopped) toast(s.message, true);
  else ui.advice?.(s);
  const v = await call<MeshView | null>("mesh_view");
  if (v) setMesh(v);
  await loadResults(true);
  setSel({ kind: "results" }); setTab("results");
};

// ---------- results
let loaded = "";
/** Fetch what the view shows (field, streamlines, arrows, probes) when it is not already here. */
const loadResults = async (force = false) => {
  const o = out();
  if (!o?.has_results) return;
  const v = o.m.view, key = `${o.revision}|${v.field}|${v.streamlines}|${v.arrows}|${arrowCount()}|${JSON.stringify(o.m.probes)}`;
  if (!force && key === loaded) return;
  const was = force ? "" : loaded;
  loaded = key;
  const part = (i: number) => was.split("|")[i];
  const id = v.field === "temperature" && !o.m.physics.heat ? "speed" : v.field;
  if (id === "none") setField(null);
  else if (force || field()?.id !== id) { const f = await call<NodalField>("field", { id, lines: 28 }); setField(f ?? null); }
  if (!v.streamlines) setStream(null);
  else if (force || !stream()) { const f = await call<NodalField>("field", { id: "stream", lines: 28 }); setStream(f ?? null); }
  if (!v.arrows) setArrowData([]);
  else if (force || !arrowData().length || part(4) !== String(arrowCount())) setArrowData((await call<number[][]>("arrows", { count: arrowCount() })) ?? []);
  if (!o.m.probes.length) setProbeData([]);
  else if (force || part(5) !== JSON.stringify(o.m.probes)) setProbeData((await call<[string, Sample][]>("probes")) ?? []);
  redraw();
};
const clearResults = () => { loaded = ""; setField(null); setStream(null); setArrowData([]); setProbeData([]); setHoverSample(null); };

export const setView = (x: Partial<View>) => act({ op: "set_view", view: { ...M().view, ...x } });
const pickField = (id: unknown) => { if (!needResults()) return; if (id === "temperature" && !M().physics.heat) return toast("This model has no temperature: turn heat on in Physics and solve again."); setView({ field: String(id) }); };

// ---------- probes and plot lines
export const place = (what: "probe" | "line") => {
  if (!needResults()) return;
  setTool("select"); setPickFor(null); setLineStart(null); setLineEnd(null);
  setPlacing(placing() === what ? null : what);
};
const hook = (w: Pt, _e: MouseEvent, phase: "down" | "move" | "up") => {
  const mode = placing();
  if (!mode) return false;
  if (phase === "move") { if (mode === "line" && lineStart()) { setLineEnd(w); redraw(); } return false; }
  if (phase !== "down") return true;
  if (mode === "probe") act({ op: "set_probes", probes: [...M().probes, { name: `Probe ${M().probes.length + 1}`, x: w[0], y: w[1] }] });
  else if (!lineStart()) { setLineStart(w); setLineEnd(w); }
  else {
    const a = lineStart()!; setLineStart(null); setLineEnd(null); setPlacing(null);
    if (Math.hypot(w[0] - a[0], w[1] - a[1]) > 0) act({ op: "set_lines", lines: [...M().lines, { name: `Line ${M().lines.length + 1}`, a, b: w, n: 201 }] }).then(ok => { if (ok) ui.linePlot?.(M().lines.length - 1); });
  }
  return true;
};

// ---------- exports
const saveText = async (name: string, data: string | undefined) => { if (data === undefined) return; const p = await call<string | null>("save_file", { name, data }); if (p) toast(`Saved ${p}`, true); };
export const exportVtu = async () => { if (needResults()) await saveText(`${M().name}.vtu`, await call<string>("export_text", { what: "vtu" })); };
export const exportProbes = async () => { if (needResults()) await saveText(`${M().name} probes.csv`, await call<string>("export_text", { what: "probes" })); };
export const exportLine = async (index: number) => { if (needResults()) await saveText(`${M().name} ${M().lines[index]?.name || `line ${index + 1}`}.csv`, await call<string>("export_text", { what: "line", index })); };

/** Start the engine's part of the window: events, commands and keeping the view in step with the
 *  model. Called once, inside the app. */
export const initSim = () => {
  listen("solve-progress", onProgress);
  listen("solve-done", onDone);
  wire("mesh", genMesh); wire("mesh_quality", toggleQuality); wire("check", () => runChecks()); wire("solve", solve); wire("resume", solve); wire("stop", stop);
  wire("field", pickField);
  wire("streamlines", () => { if (needResults()) setView({ streamlines: !M().view.streamlines }); });
  wire("arrows", () => { if (needResults()) setView({ arrows: !M().view.arrows }); });
  wire("probe", () => place("probe")); wire("lineplot", () => place("line"));
  wire("export_vtu", exportVtu);
  addEventListener("keydown", e => { if (e.key === "Escape" && placing()) { setPlacing(null); setLineStart(null); setLineEnd(null); redraw(); } });
  createEffect(() => setClickHook(placing() ? hook : null));
  // The engine drops the mesh and the results when an edit makes them out of date.
  const kept = createMemo(() => `${!!out()?.has_mesh}|${!!out()?.has_results}|${out()?.path}|${out()?.m.name}`);
  createEffect(on(kept, () => {
    const hasMesh = !!out()?.has_mesh, hasResults = !!out()?.has_results;
    if (!hasMesh) { setMesh(null); setShowQuality(false); }
    if (!hasResults) { clearResults(); if (!untrack(running)) setSummary(null); }
    if (!hasMesh && !untrack(running)) { setRows([]); setMonValues([]); }
    setPlacing(null);
    // A model that already has them (opened with its results, or the window was reloaded): fetch what is there.
    if (hasMesh && !untrack(running)) call<MeshView | null>("mesh_view").then(v => v && setMesh(v));
    if (hasResults && !untrack(running)) call<Summary | null>("summary").then(s => { if (s) { setSummary(s); setRows(s.history); setMonValues(s.monitors); setMonNames(s.monitor_names); } });
    redraw();
  }));
  // Checks on show follow the model: every edit runs them again; another model starts clean.
  const edited = createMemo(() => `${out()?.revision}|${out()?.path}|${out()?.m.name}|${!!out()?.has_mesh}`);
  let lastModel = "";
  createEffect(on(edited, () => {
    const model = `${out()?.path}|${out()?.m.name}`;
    if (model !== lastModel) { lastModel = model; setIssues(null); }
    else if (untrack(issues) && !untrack(running)) runChecks(false);
  }, { defer: true }));
  createEffect(() => { const o = out(); arrowCount(); if (o?.has_results) { JSON.stringify(o.m.view); JSON.stringify(o.m.probes); loadResults(); } });
};

/** The value under the pointer, asked for at most ten times a second. */
let hoverBusy = false, hoverNext: Pt | null = null;
export const hoverAt = (w: Pt | null) => {
  if (!w || !out()?.has_results) { if (hoverSample()) setHoverSample(null); return; }
  hoverNext = w;
  if (hoverBusy) return;
  hoverBusy = true;
  setTimeout(async () => {
    const p = hoverNext; hoverBusy = false;
    if (!p || !out()?.has_results) return;
    try { const s = await window.__TAURI__!.core.invoke("probe", { x: p[0], y: p[1] }); setHoverSample(s?.inside ? s : null); } catch { setHoverSample(null); }
  }, 100);
};
