// Commands: one place for everything the ribbon, the shortcuts and Tell Me can run. Commands
// whose engine part is not there yet go through `cmd(name)`, which says so in a toast.
import { call, loadOut, out, setBackstage, setTab, setSel, setTool, setPickFor, setAsHole, toast, act, opts, saveOpts, setBottom, setBottomOpen, openDialog, closeDialog, setTheme, theme, tool, sel, asHole, Tab, running } from "./store";
import { BcKind, Boundary, Out } from "./api";
import { fit, zoomBy, viewPicture } from "./canvas";
import { askYes } from "./ui";
import * as H from "./help";

export const GH = "https://github.com/Edgar-Mendonca/Datum-Flow";
export const SITE = "https://edgar-mendonca.github.io/Datum/apps/flow";

/** The parts that arrive in the next 0.1.x releases. Each name is wired to the engine in one
 *  place: replace its entry here (or call the engine from `cmd`) when the part lands. */
export const STUBS: Record<string, string> = {
  import_dxf: "DXF import",
};
/** Engine hooks: the main parts replace a stub by registering the real command here. */
const wired: Record<string, (arg?: unknown) => void> = {};
export const wire = (name: string, run: (arg?: unknown) => void) => { wired[name] = run; };
export const isWired = (name: string) => name in wired;
/** Run a command by name: the real one when it is wired, else a toast that says when it comes. */
export const cmd = (name: string, arg?: unknown) => {
  if (wired[name]) return wired[name](arg);
  toast(`${STUBS[name] ?? "This"}: coming in 0.1.x. See File › Roadmap.`, true);
};

/** Before the open model is replaced: ask when there are unsaved changes. */
const sure = async (what: string) => !out()?.dirty || (await askYes("Unsaved changes", `“${out()!.m.name}” has changes that are not saved. ${what} without saving them?`, what));
const newBoundary = async (kind: BcKind) => {
  const b: Boundary = { id: 0, name: "", edges: [], kind, heat: { type: "insulated" } };
  if (!(await act({ op: "set_boundary", boundary: b }))) return;
  const list = out()!.m.boundaries, made = list[list.length - 1];
  setSel({ kind: "boundary", id: made.id }); setPickFor(made.id); setTool("pick"); setTab("physics");
};

export const C = {
  // file
  newModel: async (id = "empty") => { if (await sure("Start a new model")) { const o = await loadOut("new_doc", { id }); if (o) { setBackstage(null); setSel({ kind: "model" }); setTool("select"); setTab(id === "empty" ? "geometry" : "home"); } } },
  open: async (path?: string) => { if (await sure("Open another model")) { const o = await loadOut("open", { path: path ?? null }); if (o) { setBackstage(null); setSel({ kind: "model" }); setTool("select"); } } },
  save: async (asNew = false) => { const o = await loadOut("save", { path: null, asNew }); if (o) toast(`Saved ${(o as Out).path}`, true); },
  print: () => setBackstage("print"),
  picture: async () => { const data = viewPicture(); if (!data) return toast("Open a model first."); const p = await call<string | null>("save_file", { name: `${out()?.m.name ?? "View"}.png`, data }); if (p) toast(`Saved ${p}`, true); },
  undo: () => (running() ? toast("A solve is running: stop it before undoing.") : loadOut("undo")),
  redo: () => (running() ? toast("A solve is running: stop it before redoing.") : loadOut("redo")),
  // geometry
  tool: (t: "select" | "pan" | "rect" | "circle" | "poly") => { setPickFor(null); setTool(tool() === t && t !== "select" ? "select" : t); },
  hole: (v: boolean) => setAsHole(v),
  del: () => { const s = sel(); if (s.kind === "shape") act({ op: "remove_shape", id: s.id }); else if (s.kind === "boundary") act({ op: "remove_boundary", id: s.id }); else toast("Select a shape or a boundary first."); },
  template: (id: string) => C.newModel(id),
  // physics
  inlet: () => newBoundary({ type: "inlet", speed: 1, profile: "uniform" }),
  outlet: () => newBoundary({ type: "outlet", pressure: 0 }),
  wall: () => newBoundary({ type: "wall", u: [0, 0] }),
  symmetry: () => newBoundary({ type: "symmetry" }),
  wallRest: async () => { if (await act({ op: "wall_the_rest" })) { const l = out()!.m.boundaries; setSel({ kind: "boundary", id: l[l.length - 1].id }); toast("Every free edge is now a fixed wall.", true); } },
  heat: (on: boolean) => act({ op: "set_physics", physics: { ...out()!.m.physics, heat: on, buoyancy: on && out()!.m.physics.buoyancy } }),
  buoyancy: (on: boolean) => act({ op: "set_physics", physics: { ...out()!.m.physics, heat: on || out()!.m.physics.heat, buoyancy: on } }),
  units: (u: "SI" | "US") => act({ op: "set_units", units: u }),
  // view
  go: (kind: "model" | "geometry" | "fluid" | "physics" | "boundaries" | "mesh" | "solver" | "results", t?: Tab) => { setSel({ kind }); if (t) setTab(t); },
  fit: () => fit(),
  zoomIn: () => zoomBy(1.25),
  zoomOut: () => zoomBy(0.8),
  panel: (t: "residuals" | "monitors" | "checks" | "messages") => { setBottom(t); setBottomOpen(true); },
  theme: () => setTheme(theme() === "auto" ? "light" : theme() === "light" ? "dark" : "auto"),
  snap: (on: boolean) => saveOpts({ ...opts(), snap: on }),
  // help
  link: (path = "") => call("open_link", { url: GH + path }),
  site: () => call("open_link", { url: SITE }),
};

/** Tell Me (Ctrl+K): every command by name. */
export const ALL_CMDS: [string, () => void, string?][] = [
  ["New model", () => setBackstage("home"), "Ctrl+N"], ["Open…", () => C.open(), "Ctrl+O"], ["Save", () => C.save(), "Ctrl+S"], ["Save As…", () => C.save(true), "Ctrl+Shift+S"], ["Print…", C.print, "Ctrl+P"],
  ["Examples", () => setBackstage("home")], ["Options", () => setBackstage("options")], ["Check for updates", () => setBackstage("about")], ["Roadmap", () => setBackstage("roadmap")],
  ["Undo", C.undo, "Ctrl+Z"], ["Redo", C.redo, "Ctrl+Y"],
  ["Draw a rectangle", () => { setTab("geometry"); C.tool("rect"); }], ["Draw a circle", () => { setTab("geometry"); C.tool("circle"); }], ["Draw a polygon", () => { setTab("geometry"); C.tool("poly"); }],
  ["Add to the flow region (new shapes)", () => C.hole(false)], ["Subtract from the flow region (new shapes)", () => C.hole(true)], ["Delete the selection", C.del, "Del"], ["Import DXF…", () => cmd("import_dxf")],
  ["Template: Channel", () => C.template("channel")], ["Template: Cavity", () => C.template("cavity-100")], ["Template: Step", () => C.template("step")], ["Template: Cylinder in a channel", () => C.template("cylinder")], ["Template: Heated box", () => C.template("heated-1e4")],
  ["Fluid", () => C.go("fluid", "physics")], ["Heat and buoyancy", () => C.go("physics", "physics")], ["Boundaries", () => C.go("boundaries", "physics")],
  ["New inlet", C.inlet], ["New outlet", C.outlet], ["New wall", C.wall], ["New symmetry line", C.symmetry], ["Wall the rest", C.wallRest],
  ["Units: SI", () => C.units("SI")], ["Units: US", () => C.units("US")],
  ["Mesh settings", () => C.go("mesh", "mesh")], ["Generate the mesh", () => cmd("mesh")], ["Solver settings", () => C.go("solver", "solve")], ["Solve", () => cmd("solve")], ["Stop the solve", () => cmd("stop")],
  ["Results", () => C.go("results", "results")], ["Resume the solve", () => cmd("resume")], ["Mesh quality", () => cmd("mesh_quality")],
  ["Colour by speed", () => cmd("field", "speed")], ["Colour by pressure", () => cmd("field", "pressure")], ["Colour by temperature", () => cmd("field", "temperature")], ["Colour by vorticity", () => cmd("field", "vorticity")],
  ["Streamlines on or off", () => cmd("streamlines")], ["Velocity arrows on or off", () => cmd("arrows")], ["Place a probe", () => cmd("probe")], ["Draw a plot line", () => cmd("lineplot")], ["Values: flow rate, pressure drop, forces, heat", () => cmd("values")],
  ["PDF report", () => cmd("report")], ["Export VTU", () => cmd("export_vtu")], ["Export CSV", () => cmd("export_csv")], ["Checks before a solve", () => { C.panel("checks"); cmd("check"); }], ["Verification report", () => cmd("verification")],
  ["Fit the view", C.fit, "F"], ["Zoom in", C.zoomIn], ["Zoom out", C.zoomOut], ["Theme: next (Auto, Light, Dark)", C.theme],
  ["Residuals panel", () => C.panel("residuals")], ["Monitors panel", () => C.panel("monitors")], ["Checks panel", () => C.panel("checks")], ["Messages panel", () => C.panel("messages")],
  ["Save a picture of the view (PNG)", C.picture],
  ["Getting started", H.gettingStarted], ["Glossary", H.glossary], ["Keyboard shortcuts", H.shortcuts, "F1"], ["What is this tool?", H.allHelp],
  ["Datum Flow on GitHub", () => C.link()], ["Releases", () => C.link("/releases")], ["Report an issue", () => C.link("/issues/new")],
];
export { openDialog, closeDialog, asHole };
