// Calls into the Rust shell and the shapes it returns (they mirror crates/flow-core/src/model.rs
// and wire.rs; the file and the engine are in SI units, the window converts with `out.units`).
declare global {
  interface Window { __TAURI__?: { core: { invoke: (cmd: string, args?: object) => Promise<any> }; event: { listen: (e: string, f: (x: { payload: any }) => void) => Promise<() => void> } } }
}

export const invoke = <T = any>(cmd: string, args?: object): Promise<T> =>
  window.__TAURI__ ? window.__TAURI__.core.invoke(cmd, args) : Promise.reject("Datum Flow must run inside its desktop window.");
export const listen = (e: string, f: (p: any) => void) => window.__TAURI__?.event.listen(e, x => f(x.payload));

export type Pt = [number, number];
export type Geom = { type: "poly"; pts: Pt[]; bulge: number[] } | { type: "circle"; c: Pt; r: number };
export interface Shape { id: number; name: string; hole: boolean; geom: Geom; edges: string[] }
export interface EdgeRef { shape: number; edge: number }
export interface Fluid { preset: string; name: string; at: number; rho: number; mu: number; k: number; cp: number; beta: number; source: string }
export interface Physics { heat: boolean; buoyancy: boolean; gravity: Pt; t_ref: number; length: number; depth: number }
export type BcKind = { type: "inlet"; speed: number; profile: "uniform" | "parabolic" } | { type: "outlet"; pressure: number } | { type: "wall"; u: Pt } | { type: "symmetry" };
export type HeatBc = { type: "insulated" } | { type: "temperature"; t: number } | { type: "flux"; q: number };
export interface Boundary { id: number; name: string; edges: EdgeRef[]; kind: BcKind; heat: HeatBc }
export type Target = { type: "edge"; shape: number; edge: number } | { type: "point"; x: number; y: number } | { type: "box"; x0: number; y0: number; x1: number; y1: number };
export interface Refine { at: Target; size: number }
export interface MeshSettings { size: number; wall: number; corner: number; wake: number; growth: number; min_angle: number; refine: Refine[] }
export interface SolverSettings { max_iter: number; tol: number; picard: number; continuation: "auto" | "always" | "off"; grad_div: number; allow_turbulent: boolean }
export type MonitorKind =
  | { type: "drag" | "lift" | "flow_rate" | "mean_temperature" | "heat_flow" | "nusselt"; boundary: number }
  | { type: "pressure_drop"; from: number; to: number } | { type: "speed"; x: number; y: number };
export interface Monitor { name: string; kind: MonitorKind }
export interface Probe { name: string; x: number; y: number }
export interface Line { name: string; a: Pt; b: Pt; n: number }
export interface View { field: string; map: string; arrows: boolean; streamlines: boolean; mesh: boolean; bands: number; ranges?: Record<string, [number, number]> }
export interface Model {
  name: string; notes: string; units: "SI" | "US"; grid: number; shapes: Shape[]; fluid: Fluid; physics: Physics; boundaries: Boundary[];
  mesh: MeshSettings; solver: SolverSettings; monitors: Monitor[]; probes: Probe[]; lines: Line[]; view: View;
}
export interface Unit { factor: number; offset: number; label: string }
export interface Numbers { speed: number; length: number; dt: number; re: number; ra: number; pr: number; through: boolean }
export interface EdgeInfo { shape: number; edge: number; name: string; boundary: number; length: number }
export interface Named { id: string; title: string; about: string }
/** What the window gets after every action. */
export interface Out {
  m: Model; can_undo: boolean; can_redo: boolean; revision: number; units: Record<string, Unit>; numbers: Numbers; turbulent: string | null;
  edges: EdgeInfo[]; geometry_problem: string | null; bounds: [number, number, number, number] | null;
  path: string | null; file: string; dirty: boolean; notes: string[]; version: string; examples: Named[]; templates: Named[]; has_mesh: boolean; has_results: boolean;
}
/** One edit (crates/flow-core/src/doc.rs `Op`). */
export type Op =
  | { op: "set_name"; name: string } | { op: "set_notes"; notes: string } | { op: "set_units"; units: "SI" | "US" } | { op: "set_grid"; grid: number }
  | { op: "add_shape"; name: string; hole: boolean; geom: Geom; edges?: string[] } | { op: "set_geom"; id: number; geom: Geom } | { op: "set_shape"; id: number; name: string; hole: boolean }
  | { op: "move_shape"; id: number; dx: number; dy: number } | { op: "remove_shape"; id: number } | { op: "name_edge"; shape: number; edge: number; name: string }
  | { op: "set_fluid"; fluid: Fluid } | { op: "set_physics"; physics: Physics } | { op: "set_boundary"; boundary: Boundary } | { op: "remove_boundary"; id: number } | { op: "wall_the_rest" }
  | { op: "set_mesh"; mesh: MeshSettings } | { op: "set_solver"; solver: SolverSettings } | { op: "set_monitors"; monitors: Monitor[] } | { op: "set_probes"; probes: Probe[] }
  | { op: "set_lines"; lines: Line[] } | { op: "set_view"; view: View } | { op: "replace"; model: Model };
export interface Options { autosave: number; beta: boolean; theme: string; openLast: boolean; units: "SI" | "US"; snap: boolean; grid: boolean; map: string; paper: string; landscape: boolean; company: string; keepResults: boolean }

/** A number typed by a person: "−" and "," are understood ("1,5" is 1.5, "1,200" is 1200); empty
 * or not a number gives null. */
export const parseNum = (s: string): number | null => {
  let t = s.trim().replace(/[−–]/g, "-").replace(/\s/g, "");
  if (!t) return null;
  if (t.includes(",")) t = t.includes(".") || /^-?\d{1,3}(,\d{3})+$/.test(t) ? t.replace(/,/g, "") : t.replace(",", ".");
  const v = Number(t);
  return Number.isFinite(v) ? v : null;
};

/** Numbers for people: `sig` significant digits, no trailing zeros, a true minus sign. */
export const num = (v: number, sig = 4) => {
  if (!isFinite(v)) return "—";
  if (v === 0) return "0";
  const a = Math.abs(v);
  if (a >= 1e7 || a < 1e-4) return v.toExponential(Math.max(0, sig - 1)).replace(/\.?0+e/, "e").replace("-", "−").replace("e−", "e-");
  return String(+v.toPrecision(sig)).replace("-", "−");
};
