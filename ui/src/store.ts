// The window's state: what the shell last sent, what is selected, which tool is active, and the
// small helpers every part uses (calls, toasts, menus, dialogs, units).
import { createSignal, JSX } from "solid-js";
import { invoke, Out, Op, Options, Model, EdgeRef, num } from "./api";

export const [out, setOutRaw] = createSignal<Out | null>(null);
/** The open model. Only call where a model is known to be loaded. */
export const M = (): Model => out()!.m;

// ---------- theme and options
export const [theme, setThemeRaw] = createSignal("auto");
const darkMq = window.matchMedia?.("(prefers-color-scheme: dark)");
/** Bumps when the colours change, so canvases redraw. */
export const [themeTick, setThemeTick] = createSignal(0);
export const applyTheme = () => { document.documentElement.dataset.theme = theme() === "auto" ? (darkMq?.matches ? "dark" : "light") : theme(); setThemeTick(t => t + 1); };
darkMq?.addEventListener("change", () => theme() === "auto" && applyTheme());
export const [opts, setOpts] = createSignal<Options>({ autosave: 5, beta: false, theme: "auto", openLast: false, units: "SI", snap: true, grid: true, map: "viridis", paper: "A4", landscape: true, company: "", keepResults: true });
export const saveOpts = (o: Options) => { setOpts(o); invoke("set_options", { opts: o }).catch(() => {}); };
export const setTheme = (t: string) => { setThemeRaw(t); applyTheme(); saveOpts({ ...opts(), theme: t }); };

// ---------- where we are
export type Tab = "home" | "geometry" | "physics" | "mesh" | "solve" | "results" | "check" | "view" | "help";
export const [tab, setTab] = createSignal<Tab>("home");
/** The views of the sidebar (FAMILY §6): the first five show the 2D view, the others a page. */
export type View = "geometry" | "physics" | "mesh" | "solve" | "results" | "values" | "plots" | "checks" | "verification";
export const [view, setView] = createSignal<View>("geometry");
export const PAGES: View[] = ["values", "plots", "checks", "verification"];
/** The step-by-step Setup panel (sidebar › Setup), shown first for a new blank model. */
export const [setupOpen, setSetupOpen] = createSignal(false);
export const [setupStep, setSetupStep] = createSignal(0);
export type Page = "home" | "export" | "print" | "options" | "roadmap" | "about";
export const [backstage, setBackstage] = createSignal<Page | null>(null);
export type BottomTab = "residuals" | "monitors" | "checks" | "messages";
export const [bottom, setBottom] = createSignal<BottomTab>("messages");
export const [bottomOpen, setBottomOpen] = createSignal(true);
export const [treeOpen, setTreeOpen] = createSignal(true);
export const [inspOpen, setInspOpen] = createSignal(true);

/** What is selected: a tree section, a shape, an edge of a shape or a boundary. */
export type Sel =
  | { kind: "model" } | { kind: "geometry" } | { kind: "fluid" } | { kind: "physics" } | { kind: "boundaries" } | { kind: "mesh" } | { kind: "solver" } | { kind: "results" }
  | { kind: "shape"; id: number } | { kind: "edge"; shape: number; edge: number } | { kind: "boundary"; id: number };
export const [sel, setSel] = createSignal<Sel>({ kind: "model" });

/** The active tool of the 2D view. */
export type Tool = "select" | "pan" | "rect" | "circle" | "poly" | "pick";
export const [tool, setTool] = createSignal<Tool>("select");
/** New shapes are cut out of the flow region (a body) instead of added to it. */
export const [asHole, setAsHole] = createSignal(false);
/** While "pick" is active: the boundary that clicked edges join. */
export const [pickFor, setPickFor] = createSignal<number | null>(null);
/** View switches that are not saved with the model. */
export const [showGrid, setShowGrid] = createSignal(true);
export const [showNames, setShowNames] = createSignal(true);
export const [snapOn, setSnapOn] = createSignal(true);

// ---------- messages (bottom panel)
export type Message = { at: string; text: string; kind: "info" | "warn" | "error" };
export const [messages, setMessages] = createSignal<Message[]>([]);
export const say = (text: string, kind: Message["kind"] = "info") =>
  setMessages(m => [...m.slice(-199), { at: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" }), text, kind }]);

// ---------- toasts, menus, dialogs
export const [msg, setMsg] = createSignal<{ text: string; ok: boolean } | null>(null);
let timer = 0;
export const toast = (text: string, ok = false) => { setMsg({ text, ok }); clearTimeout(timer); timer = window.setTimeout(() => setMsg(null), ok ? 3000 : 7000); if (!ok) say(text, "error"); };
export type Item = { label: string; run?: () => void; keys?: string; off?: boolean; on?: boolean; sub?: Item[]; icon?: string } | "-";
export const [menu, setMenu] = createSignal<{ x: number; y: number; items: Item[] } | null>(null);
export const openMenu = (e: MouseEvent | { x: number; y: number }, items: Item[]) => {
  if ("currentTarget" in e && e.currentTarget instanceof HTMLElement && e.type === "click") { const r = e.currentTarget.getBoundingClientRect(); setMenu({ x: r.left, y: r.bottom + 2, items }); }
  else if ("clientX" in e) { e.preventDefault?.(); setMenu({ x: e.clientX, y: e.clientY, items }); }
  else setMenu({ x: (e as { x: number }).x, y: (e as { y: number }).y, items });
};
export const [dialogs, setDialogs] = createSignal<(() => JSX.Element)[]>([]);
export const openDialog = (d: () => JSX.Element) => setDialogs(v => [...v, d]);
export const closeDialog = () => setDialogs(v => v.slice(0, -1));

// ---------- calls into the shell
/** Takes what the shell sent: the new state, and its notes into Messages. */
export const setOut = (o: Out) => {
  const before = out();
  setOutRaw(o);
  if (o.notes.length && (before?.path !== o.path || before?.notes.join("\n") !== o.notes.join("\n"))) { o.notes.forEach(n => say(n, "warn")); setBottom("messages"); setBottomOpen(true); }
  const s = sel();
  // A selection whose thing is gone falls back to its section.
  if (s.kind === "shape" && !o.m.shapes.some(x => x.id === s.id)) setSel({ kind: "geometry" });
  if (s.kind === "edge" && !o.edges.some(x => x.shape === s.shape && x.edge === s.edge)) setSel({ kind: "geometry" });
  if (s.kind === "boundary" && !o.m.boundaries.some(x => x.id === s.id)) setSel({ kind: "boundaries" });
  if (pickFor() !== null && !o.m.boundaries.some(x => x.id === pickFor())) { setPickFor(null); if (tool() === "pick") setTool("select"); }
};
/** A shell call whose failure is shown as a toast; undefined when it failed. */
export const call = async <T = any>(cmd: string, args?: object): Promise<T | undefined> => {
  try { return await invoke<T>(cmd, args); } catch (e) { toast(String(e)); return undefined; }
};
/** A call that answers with the new state. Null when the person cancelled, undefined on an error. */
export const loadOut = async (cmd: string, args?: object): Promise<Out | null | undefined> => {
  const o = await call<Out | null>(cmd, args);
  if (o) setOut(o);
  return o;
};
/** A solve is running on the engine's worker thread. */
export const [running, setRunning] = createSignal(false);
/** Edits that leave the mesh and the results as they are; the others wait until a solve ends. */
const LIGHT = ["set_view", "set_probes", "set_lines", "set_units", "set_name", "set_notes", "set_grid", "name_edge"];
/** One edit of the model. True when it was applied. */
export const act = async (op: Op): Promise<boolean> => {
  if (running() && !LIGHT.includes(op.op)) { toast("A solve is running: stop it (Solve › Stop) before changing the model."); return false; }
  return !!(await loadOut("apply", { op }));
};

// ---------- units: the engine works in SI; `out.units` says how each kind of quantity is shown
export const unit = (kind: string) => out()?.units[kind] ?? { factor: 1, offset: 0, label: "" };
/** SI value → display value. */
export const toU = (kind: string, v: number) => { const u = unit(kind); return v * u.factor + u.offset; };
/** Display value → SI value. */
export const fromU = (kind: string, v: number) => { const u = unit(kind); return (v - u.offset) / u.factor; };
export const ulabel = (kind: string) => unit(kind).label;
/** A value with its unit, for labels. */
export const fmt = (kind: string, v: number, sig = 4) => `${num(toU(kind, v), sig)}${ulabel(kind) ? " " + ulabel(kind) : ""}`;

// ---------- small lookups
export const shapeOf = (id: number) => out()?.m.shapes.find(s => s.id === id);
export const boundaryOf = (id: number) => out()?.m.boundaries.find(b => b.id === id);
export const edgeInfo = (e: EdgeRef) => out()?.edges.find(x => x.shape === e.shape && x.edge === e.edge);
export const sameEdge = (a: EdgeRef, b: EdgeRef) => a.shape === b.shape && a.edge === b.edge;
export const KIND_NAME: Record<string, string> = { inlet: "Inlet", outlet: "Outlet", wall: "Wall", symmetry: "Symmetry" };
