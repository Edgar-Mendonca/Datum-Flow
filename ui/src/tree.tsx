// The model tree: everything in the model in the order it is set up. A click selects a part; the
// inspector on the right shows it.
import { For, Show } from "solid-js";
import { out, sel, setSel, setTab, Sel, KIND_NAME, fmt, pickFor } from "./store";
import { Icon } from "./ui";
import { Help } from "./help";
import { num } from "./api";

const same = (a: Sel, b: Sel) => a.kind === b.kind && (a as any).id === (b as any).id && (a as any).shape === (b as any).shape && (a as any).edge === (b as any).edge;

export const Tree = () => {
  const o = () => out()!;
  const Row = (p: { s: Sel; icon: string; label: string; note?: string; level?: number; bad?: boolean; tab?: Parameters<typeof setTab>[0]; head?: boolean }) => (
    <button class="ti" classList={{ on: same(sel(), p.s) || (p.s.kind === "boundary" && pickFor() === (p.s as any).id), head: p.head, bad: p.bad }} style={{ "padding-left": `${8 + (p.level ?? 0) * 14}px` }}
      onClick={() => { setSel(p.s); if (p.tab) setTab(p.tab); }} title={p.label}>
      <Icon n={p.icon} /><span>{p.label}</span>{p.note && <small>{p.note}</small>}
    </button>
  );
  const free = () => o().edges.filter(e => !e.boundary).length;
  return (
    <aside class="tree">
      <header><b>Model</b><Help k="tree" /></header>
      <div class="tlist">
        <Row s={{ kind: "model" }} icon="model" label={o().m.name} head />
        <Row s={{ kind: "geometry" }} icon="rect" label="Geometry" note={o().m.shapes.length ? String(o().m.shapes.length) : "empty"} bad={!!o().geometry_problem && o().m.shapes.length > 0} tab="geometry" head />
        <For each={o().m.shapes}>{s => <Row s={{ kind: "shape", id: s.id }} icon={s.hole ? (s.geom.type === "circle" ? "hcircle" : "hrect") : s.geom.type === "circle" ? "circle" : s.geom.type === "poly" && s.geom.pts.length === 4 ? "rect" : "poly"} label={s.name} note={s.hole ? "cut out" : undefined} level={1} tab="geometry" />}</For>
        <Row s={{ kind: "fluid" }} icon="fluid" label="Fluid" note={o().m.fluid.name} tab="physics" head />
        <Row s={{ kind: "physics" }} icon={o().m.physics.heat ? "heat" : "gravity"} label="Heat and gravity" note={o().m.physics.heat ? (o().m.physics.buoyancy ? "heat, buoyancy" : "heat") : "flow only"} level={1} tab="physics" />
        <Row s={{ kind: "boundaries" }} icon="bc" label="Boundaries" note={free() ? `${free()} edge${free() > 1 ? "s" : ""} free` : String(o().m.boundaries.length)} bad={free() > 0 && o().m.shapes.length > 0} tab="physics" head />
        <For each={o().m.boundaries}>{b => <Row s={{ kind: "boundary", id: b.id }} icon={b.kind.type === "inlet" ? "inlet" : b.kind.type === "outlet" ? "outlet" : b.kind.type === "symmetry" ? "sym" : "wall"} label={b.name}
          note={b.kind.type === "inlet" ? fmt("speed", b.kind.speed, 3) : b.kind.type === "wall" && (b.kind.u[0] || b.kind.u[1]) ? "moving" : b.edges.length ? KIND_NAME[b.kind.type].toLowerCase() : "no edges"} bad={!b.edges.length} level={1} tab="physics" />}</For>
        <Row s={{ kind: "mesh" }} icon="mesh" label="Mesh" note={o().has_mesh ? "ready" : o().m.mesh.size ? fmt("len", o().m.mesh.size, 3) : "auto size"} tab="mesh" head />
        <Row s={{ kind: "solver" }} icon="solve" label="Solver" note={`${o().m.solver.max_iter} it. · ${num(o().m.solver.tol, 2)}`} tab="solve" head />
        <Row s={{ kind: "results" }} icon="contour" label="Results" note={o().has_results ? "solved" : "not solved"} tab="results" head />
        <Show when={o().m.monitors.length}><For each={o().m.monitors}>{m => <div class="ti leaf" style={{ "padding-left": "22px" }}><Icon n="monitor" /><span>{m.name}</span></div>}</For></Show>
      </div>
    </aside>
  );
};
