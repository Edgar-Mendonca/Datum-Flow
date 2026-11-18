// The inspector: the values of whatever is selected in the tree or the view. Every change is one
// edit sent to the engine (`act`); the window never works a result out itself.
import { For, Show, Switch, Match, JSX, createSignal, createMemo } from "solid-js";
import { Geom, Pt, Shape, Boundary, Fluid, MeshSettings, SolverSettings, Physics, Target, Monitor, MonitorKind, View, num, invoke } from "./api";
import { out, M, sel, setSel, act, fmt, toU, fromU, ulabel, toast, setTool, tool, pickFor, setPickFor, boundaryOf, shapeOf, edgeInfo, KIND_NAME, sameEdge, snapOn, setSnapOn, opts, saveOpts, setupOpen } from "./store";
import { Icon, Num, Pick, Check, Text, Seg, askText } from "./ui";
import { Help } from "./help";
import { C, cmd } from "./cmds";
import { edgeCount } from "./canvas";
import { running, summary, arrowCount, setArrowCount } from "./sim";
import { MeshQuality, linePlot, askRange } from "./simui";
/** The kind of unit of each field, for showing a range set by hand. */
const FKIND: Record<string, string> = { speed: "speed", u: "speed", v: "speed", pressure: "pressure", temperature: "temp", vorticity: "vorticity", stream: "stream" };

const Sec = (p: { t: string; h?: string; children: JSX.Element; tools?: JSX.Element }) => <section class="isec"><h3>{p.t}{p.h && <Help k={p.h} />}<span class="grow" />{p.tools}</h3>{p.children}</section>;
/** A label on the left, a control on the right. */
const F = (p: { l: string; hint?: string; children: JSX.Element }) => <div class="ifield" title={p.hint}><label>{p.l}</label><div>{p.children}</div></div>;
const Note = (p: { children: JSX.Element; kind?: "warn" | "err" | "ok" }) => <p class="inote" classList={{ warn: p.kind === "warn", err: p.kind === "err", ok: p.kind === "ok" }}>{p.children}</p>;

// ---------------------------------------------------------------- model

const ModelCard = () => {
  const n = () => out()!.numbers;
  return <>
    <Sec t="Model" h="file">
      <F l="Name"><Text value={M().name} set={v => act({ op: "set_name", name: v })} /></F>
      <F l="Units"><Seg options={[["SI", "SI"], ["US", "US"]]} value={M().units} set={v => C.units(v as "SI" | "US")} /></F>
      <F l="Notes"><textarea rows="5" value={M().notes} onChange={e => act({ op: "set_notes", notes: e.currentTarget.value })} placeholder="What this case is, and what to look for." /></F>
    </Sec>
    <Sec t="What kind of flow" h="numbers">
      <table class="tbl facts"><tbody>
        <tr><td>Reynolds number</td><td class="r">{n().speed > 0 ? num(n().re, 3) : "—"}</td></tr>
        <Show when={M().physics.buoyancy}><tr><td>Rayleigh number</td><td class="r">{num(n().ra, 3)}</td></tr></Show>
        <tr><td>Prandtl number</td><td class="r">{num(n().pr, 3)}</td></tr>
        <tr><td>Reference speed</td><td class="r">{fmt("speed", n().speed)}</td></tr>
        <tr><td>Reference length</td><td class="r">{fmt("len", n().length)}</td></tr>
        <Show when={M().physics.heat}><tr><td>Temperature difference</td><td class="r">{fmt("dtemp", n().dt)}</td></tr></Show>
      </tbody></table>
      <Show when={out()!.turbulent}><Note kind="warn">{out()!.turbulent}</Note></Show>
      <Show when={!out()!.turbulent && n().speed > 0}><Note kind="ok">{n().through ? "Flow through the region" : "A closed region"}: laminar at this Reynolds number.</Note></Show>
    </Sec>
  </>;
};

// ---------------------------------------------------------------- geometry

const GeometryCard = () => (
  <>
    <Sec t="Geometry" h="draw" tools={<button class="btn sm" onClick={() => C.fit()}>Fit</button>}>
      <Show when={out()!.geometry_problem && M().shapes.length}><Note kind="err">{out()!.geometry_problem}</Note></Show>
      <Show when={out()!.bounds}>{b => <F l="Size"><span class="ro">{fmt("len", b()[2] - b()[0])} × {fmt("len", b()[3] - b()[1])}</span></F>}</Show>
      <F l="Grid step" hint="Points you click or drag land on the grid while Snap is on."><Num kind="len" value={M().grid} set={v => act({ op: "set_grid", grid: v })} min={1e-9} /></F>
      <F l="Snap"><Check label="Snap to the grid" value={snapOn()} set={v => { setSnapOn(v); saveOpts({ ...opts(), snap: v }); }} /></F>
    </Sec>
    <Sec t="Shapes">
      <For each={M().shapes} fallback={<p class="muted">Nothing drawn yet. Use Geometry › Rectangle, Circle or Polygon, or a template.</p>}>{s =>
        <button class="irow" onClick={() => setSel({ kind: "shape", id: s.id })}><Icon n={s.geom.type === "circle" ? "circle" : "poly"} /><span>{s.name}</span><small>{s.hole ? "cut out" : "adds"}</small></button>}</For>
    </Sec>
  </>
);

/** A polygon that is a rectangle with sides along the axes: [x0, y0, x1, y1]. */
const asRect = (g: Geom): [number, number, number, number] | null => {
  if (g.type !== "poly" || g.pts.length !== 4 || g.bulge.some(b => b)) return null;
  const xs = [...new Set(g.pts.map(p => p[0]))], ys = [...new Set(g.pts.map(p => p[1]))];
  if (xs.length !== 2 || ys.length !== 2) return null;
  const [a, b, c, d] = g.pts;
  if (!((a[1] === b[1] && b[0] === c[0] && c[1] === d[1] && d[0] === a[0]) || (a[0] === b[0] && b[1] === c[1] && c[0] === d[0] && d[1] === a[1]))) return null;
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
};
const rectGeom = (x0: number, y0: number, x1: number, y1: number): Geom => ({ type: "poly", pts: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]], bulge: [0, 0, 0, 0] });

const ShapeCard = (p: { s: Shape }) => {
  const s = () => p.s, g = () => p.s.geom;
  const setGeom = (geom: Geom) => act({ op: "set_geom", id: s().id, geom });
  const r = createMemo(() => asRect(g()));
  const edges = () => out()!.edges.filter(e => e.shape === s().id);
  const setPt = (i: number, k: 0 | 1, v: number) => { const gg = g(); if (gg.type === "poly") setGeom({ ...gg, pts: gg.pts.map((q, j) => (j === i ? (k === 0 ? [v, q[1]] : [q[0], v]) as Pt : q)) }); };
  const setBulge = (i: number, v: number) => { const gg = g(); if (gg.type === "poly") setGeom({ ...gg, bulge: gg.pts.map((_, j) => (j === i ? v : gg.bulge[j] ?? 0)) }); };
  const addPt = (i: number) => { const gg = g(); if (gg.type !== "poly") return; const a = gg.pts[i], b = gg.pts[(i + 1) % gg.pts.length]; const pts = [...gg.pts]; pts.splice(i + 1, 0, [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2]); const bulge = gg.pts.map((_, j) => gg.bulge[j] ?? 0); bulge[i] = 0; bulge.splice(i + 1, 0, 0); setGeom({ type: "poly", pts, bulge }); };
  const delPt = (i: number) => { const gg = g(); if (gg.type !== "poly") return; if (gg.pts.length <= 3) return toast("A polygon needs at least three corners."); setGeom({ type: "poly", pts: gg.pts.filter((_, j) => j !== i), bulge: gg.pts.map((_, j) => gg.bulge[j] ?? 0).filter((_, j) => j !== i) }); };
  return <>
    <Sec t="Shape" h="edit" tools={<button class="btn sm" title="Delete this shape (Del)" onClick={() => act({ op: "remove_shape", id: s().id })}>Delete</button>}>
      <F l="Name"><Text value={s().name} set={v => act({ op: "set_shape", id: s().id, name: v, hole: s().hole })} /></F>
      <F l="Role" hint="Add: part of the flow region. Subtract: a body cut out of it."><Seg options={[["add", "Add"], ["sub", "Subtract"]]} value={s().hole ? "sub" : "add"} set={v => act({ op: "set_shape", id: s().id, name: s().name, hole: v === "sub" })} /></F>
    </Sec>
    <Switch>
      <Match when={g().type === "circle" && (g() as Extract<Geom, { type: "circle" }>)}>{c => (
        <Sec t="Circle">
          <F l="Centre x"><Num kind="len" value={c().c[0]} set={v => setGeom({ ...c(), c: [v, c().c[1]] })} /></F>
          <F l="Centre y"><Num kind="len" value={c().c[1]} set={v => setGeom({ ...c(), c: [c().c[0], v] })} /></F>
          <F l="Radius"><Num kind="len" value={c().r} min={1e-12} set={v => setGeom({ ...c(), r: v })} /></F>
          <F l="Diameter"><Num kind="len" value={2 * c().r} min={1e-12} set={v => setGeom({ ...c(), r: v / 2 })} /></F>
        </Sec>)}</Match>
      <Match when={r()}>{b => (
        <Sec t="Rectangle">
          <F l="Left x"><Num kind="len" value={b()[0]} set={v => setGeom(rectGeom(v, b()[1], v + (b()[2] - b()[0]), b()[3]))} /></F>
          <F l="Bottom y"><Num kind="len" value={b()[1]} set={v => setGeom(rectGeom(b()[0], v, b()[2], v + (b()[3] - b()[1])))} /></F>
          <F l="Width"><Num kind="len" value={b()[2] - b()[0]} min={1e-12} set={v => setGeom(rectGeom(b()[0], b()[1], b()[0] + v, b()[3]))} /></F>
          <F l="Height"><Num kind="len" value={b()[3] - b()[1]} min={1e-12} set={v => setGeom(rectGeom(b()[0], b()[1], b()[2], b()[1] + v))} /></F>
        </Sec>)}</Match>
    </Switch>
    <Show when={g().type === "poly" && (g() as Extract<Geom, { type: "poly" }>)}>{pg => (
      <Sec t="Corners" h="draw">
        <table class="tbl pts"><thead><tr><th>#</th><th>x ({ulabel("len")})</th><th>y ({ulabel("len")})</th><th title="The side from this corner to the next: 0 straight, 1 a half circle to the left, −1 to the right">Arc</th><th /></tr></thead>
          <tbody><For each={pg().pts}>{(q, i) => <tr>
            <td class="muted">{i() + 1}</td>
            <td><Num kind="len" value={q[0]} set={v => setPt(i(), 0, v)} /></td>
            <td><Num kind="len" value={q[1]} set={v => setPt(i(), 1, v)} /></td>
            <td><Num value={pg().bulge[i()] ?? 0} set={v => setBulge(i(), v)} sig={4} title="Bulge of the side to the next corner" /></td>
            <td class="acts"><button title="Add a corner after this one" onClick={() => addPt(i())}>+</button><button title="Remove this corner" onClick={() => delPt(i())}>×</button></td>
          </tr>}</For></tbody></table>
      </Sec>)}</Show>
    <Sec t="Edges" h="boundaries">
      <table class="tbl"><thead><tr><th>Name</th><th>Length</th><th>Boundary</th></tr></thead>
        <tbody><For each={edges()}>{e => <tr class="link" classList={{ sel: sel().kind === "edge" && sameEdge(sel() as any, e) }} onClick={() => setSel({ kind: "edge", shape: e.shape, edge: e.edge })}>
          <td><input value={s().edges[e.edge] ?? ""} placeholder={e.name} onClick={ev => ev.stopPropagation()} onChange={ev => act({ op: "name_edge", shape: e.shape, edge: e.edge, name: ev.currentTarget.value })} /></td>
          <td class="r">{fmt("len", e.length, 3)}</td>
          <td classList={{ late: !e.boundary }}>{e.boundary ? boundaryOf(e.boundary)?.name : "none"}</td></tr>}</For></tbody></table>
    </Sec>
  </>;
};

const EdgeCard = (p: { shape: number; edge: number }) => {
  const s = () => shapeOf(p.shape)!, info = () => edgeInfo(p)!;
  const g = () => s().geom;
  const put = (id: number) => {
    const ref = { shape: p.shape, edge: p.edge }, cur = info().boundary ? boundaryOf(info().boundary) : undefined;
    if (id === 0) { if (cur) act({ op: "set_boundary", boundary: { ...cur, edges: cur.edges.filter(x => !sameEdge(x, ref)) } }); return; }
    const b = boundaryOf(id); if (b) act({ op: "set_boundary", boundary: { ...b, edges: [...b.edges.filter(x => !sameEdge(x, ref)), ref] } });
  };
  return <Show when={s() && info()}>
    <Sec t="Edge" h="boundaries" tools={<button class="btn sm" onClick={() => setSel({ kind: "shape", id: p.shape })}>Its shape</button>}>
      <F l="Name"><Text value={s().edges[p.edge] ?? ""} placeholder={info().name} set={v => act({ op: "name_edge", shape: p.shape, edge: p.edge, name: v })} /></F>
      <F l="Of"><span class="ro">{s().name}{edgeCount(g()) > 1 ? ` · side ${p.edge + 1}` : ""}</span></F>
      <F l="Length"><span class="ro">{fmt("len", info().length)}</span></F>
      <Show when={g().type === "poly"}><F l="Arc" hint="0 straight; 1 a half circle to the left of the side; 0.414 a quarter circle; negative bulges to the right.">
        <Num value={(g() as Extract<Geom, { type: "poly" }>).bulge[p.edge] ?? 0} sig={4} set={v => { const gg = g() as Extract<Geom, { type: "poly" }>; act({ op: "set_geom", id: p.shape, geom: { ...gg, bulge: gg.pts.map((_, j) => (j === p.edge ? v : gg.bulge[j] ?? 0)) } }); }} /></F></Show>
      <F l="Boundary"><select onChange={e => put(+e.currentTarget.value)}>
        <option value="0" selected={!info().boundary}>None yet</option>
        <For each={M().boundaries}>{b => <option value={b.id} selected={b.id === info().boundary}>{b.name} ({KIND_NAME[b.kind.type]})</option>}</For></select></F>
      <Show when={!info().boundary}><Note kind="warn">This edge has no boundary. Pick one above, or add one under Physics › Boundaries.</Note></Show>
      <Show when={info().boundary}><p><button class="btn sm" onClick={() => setSel({ kind: "boundary", id: info().boundary })}>Open its boundary</button></p></Show>
    </Sec>
  </Show>;
};

// ---------------------------------------------------------------- fluid and physics

const PRESETS: [string, string][] = [["water", "Water"], ["air", "Air"], ["oil", "Light oil (ISO VG 10)"], ["custom", "Custom"]];
const FluidCard = () => {
  const f = () => M().fluid, custom = () => f().preset === "custom";
  const load = async (preset: string, t: number) => {
    if (preset === "custom") return act({ op: "set_fluid", fluid: { ...f(), preset: "custom", name: f().preset === "custom" ? f().name : `${f().name} (edited)`, source: "Values typed by the user." } });
    try { act({ op: "set_fluid", fluid: await invoke<Fluid>("fluid_preset", { name: preset, t }) }); } catch (e) { toast(String(e)); }
  };
  const put = (k: keyof Fluid, v: number) => act({ op: "set_fluid", fluid: { ...f(), [k]: v } });
  const P = (p: { l: string; k: "rho" | "mu" | "k" | "cp" | "beta"; kind: string }) => <F l={p.l}><Num kind={p.kind} value={f()[p.k]} disabled={!custom()} set={v => put(p.k, v)} /></F>;
  return <>
    <Sec t="Fluid" h="fluid">
      <F l="Fluid"><select onChange={e => load(e.currentTarget.value, f().at)}><For each={PRESETS}>{([k, l]) => <option value={k} selected={k === f().preset}>{l}</option>}</For></select></F>
      <Show when={custom()}><F l="Name"><Text value={f().name} set={v => act({ op: "set_fluid", fluid: { ...f(), name: v } })} /></F></Show>
      <Show when={!custom()}><F l="At temperature" hint="The properties are taken at this temperature and stay constant during a solve."><Num kind="temp" value={f().at} set={v => load(f().preset, v)} /></F></Show>
    </Sec>
    <Sec t="Properties">
      <P l="Density" k="rho" kind="density" />
      <P l="Viscosity" k="mu" kind="viscosity" />
      <P l="Conductivity" k="k" kind="cond" />
      <P l="Specific heat" k="cp" kind="cp" />
      <P l="Expansion" k="beta" kind="expansion" />
      <Show when={!custom()}><p><button class="btn sm" onClick={() => load("custom", f().at)}>Edit the values</button></p></Show>
      <table class="tbl facts"><tbody>
        <tr><td>Kinematic viscosity</td><td class="r">{fmt("kinvisc", f().mu / f().rho)}</td></tr>
        <tr><td>Thermal diffusivity</td><td class="r">{fmt("kinvisc", f().k / (f().rho * f().cp))}</td></tr>
        <tr><td>Prandtl number</td><td class="r">{num(out()!.numbers.pr, 3)}</td></tr>
      </tbody></table>
    </Sec>
    <Sec t="Source"><p class="src">{f().source}</p></Sec>
  </>;
};

const PhysicsCard = () => {
  const p = () => M().physics, put = (x: Partial<Physics>) => act({ op: "set_physics", physics: { ...p(), ...x } });
  return <>
    <Sec t="Heat" h="heat">
      <F l="Heat"><Check label="Solve the temperature" value={p().heat} set={v => C.heat(v)} /></F>
      <F l="Buoyancy"><Check label="Warm fluid rises" value={p().buoyancy} set={v => C.buoyancy(v)} /></F>
      <Show when={p().heat}><F l="Reference temp." hint="The temperature at which the fluid neither rises nor sinks; also where the solve starts."><Num kind="temp" value={p().t_ref} set={v => put({ t_ref: v })} /></F></Show>
      <Show when={p().heat && !M().boundaries.some(b => b.heat.type !== "insulated")}><Note kind="warn">Heat is on, but no boundary has a temperature or a heat flux yet.</Note></Show>
    </Sec>
    <Sec t="Gravity">
      <F l="Gravity x"><Num kind="accel" value={p().gravity[0]} set={v => put({ gravity: [v, p().gravity[1]] })} /></F>
      <F l="Gravity y"><Num kind="accel" value={p().gravity[1]} set={v => put({ gravity: [p().gravity[0], v] })} /></F>
      <p class="muted">Gravity only acts through buoyancy: without it the pressure shown is the pressure that moves the fluid.</p>
    </Sec>
    <Sec t="Reference" h="numbers">
      <F l="Length" hint="The length the Reynolds and Rayleigh numbers are built on. 0: automatic (the inlet's width, or the region's smaller side)."><Num kind="len" value={p().length} min={0} set={v => put({ length: v })} /></F>
      <F l="Depth" hint="How deep the 2D model is into the page, for flow rates, forces and heat flows."><Num kind="len" value={p().depth} min={1e-9} set={v => put({ depth: v })} /></F>
      <p class="muted">Automatic length now: {fmt("len", out()!.numbers.length)}.</p>
    </Sec>
  </>;
};

// ---------------------------------------------------------------- boundaries

const BoundariesCard = () => {
  const free = () => out()!.edges.filter(e => !e.boundary);
  return <>
    <Sec t="Boundaries" h="boundaries">
      <div class="ibtns"><button class="btn sm" onClick={C.inlet}><Icon n="inlet" />Inlet</button><button class="btn sm" onClick={C.outlet}><Icon n="outlet" />Outlet</button><button class="btn sm" onClick={C.wall}><Icon n="wall" />Wall</button><button class="btn sm" onClick={C.symmetry}><Icon n="sym" />Symmetry</button></div>
      <For each={M().boundaries} fallback={<p class="muted">No boundaries yet. Add an inlet, an outlet and walls, then click the edges each applies to.</p>}>{b =>
        <button class="irow" classList={{ bad: !b.edges.length }} onClick={() => setSel({ kind: "boundary", id: b.id })}><Icon n={b.kind.type === "inlet" ? "inlet" : b.kind.type === "outlet" ? "outlet" : b.kind.type === "symmetry" ? "sym" : "wall"} /><span>{b.name}</span><small>{KIND_NAME[b.kind.type]} · {b.edges.length} edge{b.edges.length === 1 ? "" : "s"}</small></button>}</For>
    </Sec>
    <Sec t="Edges without a boundary">
      <Show when={free().length} fallback={<Note kind="ok">Every edge has a boundary.</Note>}>
        <Note kind="warn">{free().length} edge{free().length > 1 ? "s have" : " has"} no boundary yet. They are drawn dashed in red.</Note>
        <For each={free().slice(0, 12)}>{e => <button class="irow" onClick={() => setSel({ kind: "edge", shape: e.shape, edge: e.edge })}><Icon n="point" /><span>{e.name}</span><small>{fmt("len", e.length, 3)}</small></button>}</For>
        <p><button class="btn sm" onClick={C.wallRest}>Wall the rest</button></p>
      </Show>
    </Sec>
  </>;
};

const BoundaryCard = (p: { b: Boundary }) => {
  const b = () => p.b, put = (x: Partial<Boundary>) => act({ op: "set_boundary", boundary: { ...b(), ...x } });
  const k = () => b().kind, h = () => b().heat;
  const setKind = (t: string) => put({ kind: t === "inlet" ? { type: "inlet", speed: 1, profile: "uniform" } : t === "outlet" ? { type: "outlet", pressure: 0 } : t === "wall" ? { type: "wall", u: [0, 0] } : { type: "symmetry" } });
  const picking = () => tool() === "pick" && pickFor() === b().id;
  return <>
    <Sec t="Boundary" h="boundaries" tools={<button class="btn sm" title="Delete this boundary (Del)" onClick={() => act({ op: "remove_boundary", id: b().id })}>Delete</button>}>
      <F l="Name"><Text value={b().name} set={v => put({ name: v })} /></F>
      <F l="Kind"><select onChange={e => setKind(e.currentTarget.value)}><For each={Object.entries(KIND_NAME)}>{([key, l]) => <option value={key} selected={key === k().type}>{l}</option>}</For></select></F>
      <Switch>
        <Match when={k().type === "inlet" && (k() as Extract<Boundary["kind"], { type: "inlet" }>)}>{x => <>
          <F l="Mean speed"><Num kind="speed" value={x().speed} min={0} set={v => put({ kind: { ...x(), speed: v } })} /></F>
          <F l="Profile" hint="Uniform: the same speed across the inlet. Fully developed: a parabola between two walls, with this mean speed."><Seg options={[["uniform", "Uniform"], ["parabolic", "Developed"]]} value={x().profile} set={v => put({ kind: { ...x(), profile: v as "uniform" | "parabolic" } })} /></F>
        </>}</Match>
        <Match when={k().type === "outlet" && (k() as Extract<Boundary["kind"], { type: "outlet" }>)}>{x =>
          <F l="Pressure" hint="Relative pressure at the outlet; every pressure shown is measured from it."><Num kind="pressure" value={x().pressure} set={v => put({ kind: { ...x(), pressure: v } })} /></F>}</Match>
        <Match when={k().type === "wall" && (k() as Extract<Boundary["kind"], { type: "wall" }>)}>{x => <>
          <F l="Wall speed x" hint="Zero for a fixed wall. A moving wall (a lid, a belt) drags the fluid along."><Num kind="speed" value={x().u[0]} set={v => put({ kind: { ...x(), u: [v, x().u[1]] } })} /></F>
          <F l="Wall speed y"><Num kind="speed" value={x().u[1]} set={v => put({ kind: { ...x(), u: [x().u[0], v] } })} /></F>
        </>}</Match>
        <Match when={k().type === "symmetry"}><p class="muted">No flow crosses this line and the fluid slides along it freely. In 0.1 symmetry lines must run along x or y.</p></Match>
      </Switch>
    </Sec>
    <Show when={M().physics.heat} fallback={<Show when={h().type !== "insulated"}><Sec t="Heat"><Note kind="warn">This boundary has a {h().type === "temperature" ? "temperature" : "heat flux"}, but Heat is off (Physics › Heat).</Note><p><button class="btn sm" onClick={() => put({ heat: { type: "insulated" } })}>Remove it</button> <button class="btn sm" onClick={() => C.heat(true)}>Turn Heat on</button></p></Sec></Show>}>
      <Sec t="Heat" h="heat">
        <F l="Heat"><select onChange={e => { const t = e.currentTarget.value; put({ heat: t === "temperature" ? { type: "temperature", t: M().physics.t_ref } : t === "flux" ? { type: "flux", q: 0 } : { type: "insulated" } }); }}>
          <option value="insulated" selected={h().type === "insulated"}>{k().type === "outlet" ? "Leaves with the flow" : "Insulated"}</option><option value="temperature" selected={h().type === "temperature"}>Temperature</option><option value="flux" selected={h().type === "flux"}>Heat flux</option></select></F>
        <Show when={h().type === "temperature"}><F l="Temperature"><Num kind="temp" value={(h() as { t: number }).t} set={v => put({ heat: { type: "temperature", t: v } })} /></F></Show>
        <Show when={h().type === "flux"}><F l="Heat flux" hint="Heat entering the fluid through this boundary; negative leaves."><Num kind="flux" value={(h() as { q: number }).q} set={v => put({ heat: { type: "flux", q: v } })} /></F></Show>
      </Sec>
    </Show>
    <Sec t="Edges" tools={<button class="btn sm" classList={{ pri: picking() }} onClick={() => { if (picking()) { setTool("select"); setPickFor(null); } else { setPickFor(b().id); setTool("pick"); } }}>{picking() ? "Done" : "Pick edges"}</button>}>
      <For each={b().edges} fallback={<Note kind="warn">No edges yet: press Pick edges, then click them in the view.</Note>}>{e =>
        <div class="irow"><Icon n="point" /><span onClick={() => setSel({ kind: "edge", shape: e.shape, edge: e.edge })}>{edgeInfo(e)?.name ?? "edge"}</span><small>{fmt("len", edgeInfo(e)?.length ?? 0, 3)}</small>
          <button class="x" title="Take this edge off the boundary" onClick={() => put({ edges: b().edges.filter(x => !sameEdge(x, e)) })}>×</button></div>}</For>
    </Sec>
  </>;
};

// ---------------------------------------------------------------- mesh and solver

/** A slider for a share between 0.05 and 1 (1 = no refinement). */
const Share = (p: { value: number; set: (v: number) => void }) => <span class="share"><input type="range" min="5" max="100" step="5" value={Math.round(p.value * 100)} onChange={e => p.set(+e.currentTarget.value / 100)} /><b>{p.value >= 1 ? "off" : `× ${num(p.value, 2)}`}</b></span>;
const MeshCard = () => {
  const m = () => M().mesh, put = (x: Partial<MeshSettings>) => act({ op: "set_mesh", mesh: { ...m(), ...x } });
  // The slider runs from 1/200 to 1/5 of the region's larger side, on a log scale.
  const side = () => { const b = out()!.bounds; return b ? Math.max(b[2] - b[0], b[3] - b[1]) : 1; };
  const lo = () => side() / 200, hi = () => side() / 5;
  const pos = () => (m().size > 0 ? Math.round(100 * Math.log(Math.min(hi(), Math.max(lo(), m().size)) / lo()) / Math.log(hi() / lo())) : 50);
  const fromPos = (v: number) => { const s = lo() * Math.pow(hi() / lo(), v / 100); return +s.toPrecision(2); };
  const [kind, setKind] = createSignal<"edge" | "point" | "box">("edge");
  const addLocal = () => {
    const b = out()!.bounds ?? [0, 0, 1, 1], size = (m().size || side() / 25) / 4, s0 = sel();
    const at: Target | null = kind() === "edge" ? (s0.kind === "edge" ? { type: "edge", shape: s0.shape, edge: s0.edge } : null) : kind() === "point" ? { type: "point", x: (b[0] + b[2]) / 2, y: (b[1] + b[3]) / 2 } : { type: "box", x0: b[0] + (b[2] - b[0]) * 0.4, y0: b[1] + (b[3] - b[1]) * 0.4, x1: b[0] + (b[2] - b[0]) * 0.6, y1: b[1] + (b[3] - b[1]) * 0.6 };
    if (!at) return toast("Select an edge in the view first, then add its local size.");
    put({ refine: [...m().refine, { at, size: +size.toPrecision(2) }] });
  };
  const setR = (i: number, at: Target, size: number) => put({ refine: m().refine.map((r, j) => (j === i ? { at, size } : r)) });
  return <>
    <Sec t="Mesh size" h="meshsize" tools={<button class="btn sm pri" onClick={() => cmd("mesh")}>Generate</button>}>
      <F l="Size"><Check label="Automatic" value={m().size === 0} set={v => put({ size: v ? 0 : fromPos(50) })} /></F>
      <Show when={m().size > 0}>
        <F l="Element size"><Num kind="len" value={m().size} min={1e-12} set={v => put({ size: v })} /></F>
        <div class="slider"><span>fine</span><input type="range" min="0" max="100" value={pos()} onChange={e => put({ size: fromPos(+e.currentTarget.value) })} /><span>coarse</span></div>
      </Show>
    </Sec>
    <Sec t="Refine where the flow changes" h="meshsize">
      <F l="At walls" hint="Element size at walls as a share of the general size."><Share value={m().wall} set={v => put({ wall: v })} /></F>
      <F l="At corners" hint="Element size at sharp inside corners, where the flow turns hardest."><Share value={m().corner} set={v => put({ corner: v })} /></F>
      <F l="In the wake" hint="Element size behind bodies in the flow."><Share value={m().wake} set={v => put({ wake: v })} /></F>
      <F l="Growth" hint="How fast elements grow away from a refined place: size gained per distance."><Num value={m().growth} min={0.02} sig={3} set={v => put({ growth: v })} /></F>
      <F l="Smallest angle" hint="The mesher aims for no angle below this (degrees)."><Num value={m().min_angle} min={5} sig={3} set={v => put({ min_angle: v })} /></F>
    </Sec>
    <Sec t="Local sizes" h="meshsize" tools={<><select class="mini" onChange={e => setKind(e.currentTarget.value as "edge" | "point" | "box")}><option value="edge">On the selected edge</option><option value="point">Near a point</option><option value="box">In a box</option></select><button class="btn sm" onClick={addLocal}>Add</button></>}>
      <For each={m().refine} fallback={<p class="muted">None. A local size makes the mesh finer on one edge, near a point or in a box.</p>}>{(r, i) => <div class="local">
        <div class="lhead"><b>{r.at.type === "edge" ? (edgeInfo(r.at)?.name ?? "Edge (gone)") : r.at.type === "point" ? "Near a point" : "In a box"}</b><Num kind="len" value={r.size} min={1e-12} width={70} set={v => setR(i(), r.at, v)} /><button class="x" title="Remove this local size" onClick={() => put({ refine: m().refine.filter((_, j) => j !== i()) })}>×</button></div>
        <Show when={r.at.type === "point" && (r.at as Extract<Target, { type: "point" }>)}>{a => <div class="lxy"><span>x</span><Num kind="len" value={a().x} width={70} set={v => setR(i(), { ...a(), x: v }, r.size)} /><span>y</span><Num kind="len" value={a().y} width={70} set={v => setR(i(), { ...a(), y: v }, r.size)} /></div>}</Show>
        <Show when={r.at.type === "box" && (r.at as Extract<Target, { type: "box" }>)}>{a => <>
          <div class="lxy"><span>x</span><Num kind="len" value={a().x0} width={70} set={v => setR(i(), { ...a(), x0: v }, r.size)} /><span>to</span><Num kind="len" value={a().x1} width={70} set={v => setR(i(), { ...a(), x1: v }, r.size)} /></div>
          <div class="lxy"><span>y</span><Num kind="len" value={a().y0} width={70} set={v => setR(i(), { ...a(), y0: v }, r.size)} /><span>to</span><Num kind="len" value={a().y1} width={70} set={v => setR(i(), { ...a(), y1: v }, r.size)} /></div></>}</Show>
      </div>}</For>
    </Sec>
    <MeshQuality />
  </>;
};

const SolverCard = () => {
  const s = () => M().solver, put = (x: Partial<SolverSettings>) => act({ op: "set_solver", solver: { ...s(), ...x } });
  return <>
    <Sec t="Solver" h="solversettings" tools={<Show when={running()} fallback={<button class="btn sm pri" onClick={() => cmd("solve")}>{summary()?.stopped ? "Resume" : "Solve"}</button>}><button class="btn sm" onClick={() => cmd("stop")}>Stop</button></Show>}>
      <F l="Iterations" hint="The most steps of one solve (each continuation step gets this many)."><Num value={s().max_iter} min={1} sig={5} set={v => put({ max_iter: Math.round(v) })} /></F>
      <F l="Tolerance" hint="Stop when the residual has fallen by this factor."><Num value={s().tol} min={1e-14} sig={3} set={v => put({ tol: v })} /></F>
      <F l="Picard steps" hint="Simple, robust steps before the fast (Newton) steps take over."><Num value={s().picard} min={0} sig={4} set={v => put({ picard: Math.round(v) })} /></F>
      <F l="Continuation" hint="Raise the Reynolds or Rayleigh number step by step from an easy case."><Pick value={s().continuation} set={v => put({ continuation: v as SolverSettings["continuation"] })} options={[["auto", "Only if needed"], ["always", "Always"], ["off", "Never"]]} /></F>
      <F l="Grad-div" hint="Stabilisation weight; 0 is off. It helps strong natural convection."><Num value={s().grad_div} min={0} sig={3} set={v => put({ grad_div: v })} /></F>
    </Sec>
    <Sec t="Laminar flow only" h="numbers">
      <Show when={out()!.turbulent} fallback={<Note kind="ok">The Reynolds{M().physics.buoyancy ? " and Rayleigh numbers say" : " number says"} this flow should be laminar.</Note>}><Note kind="warn">{out()!.turbulent}</Note></Show>
      <F l="Anyway"><Check label="Solve even if probably turbulent" value={s().allow_turbulent} set={v => put({ allow_turbulent: v })} /></F>
    </Sec>
  </>;
};

// ---------------------------------------------------------------- results setup

const FIELDS: [string, string][] = [["speed", "Speed"], ["pressure", "Pressure"], ["temperature", "Temperature"], ["vorticity", "Vorticity"], ["u", "Velocity x"], ["v", "Velocity y"], ["stream", "Stream function"], ["none", "None"]];
const MAPS: [string, string][] = [["viridis", "Viridis"], ["inferno", "Inferno"], ["cividis", "Cividis"], ["diverging", "Blue to red (signed values)"]];
const MON: [MonitorKind["type"], string][] = [["drag", "Drag on"], ["lift", "Lift on"], ["flow_rate", "Flow rate through"], ["pressure_drop", "Pressure drop from"], ["mean_temperature", "Mean temperature at"], ["heat_flow", "Heat flow from"], ["nusselt", "Nusselt number on"]];
const ResultsCard = () => {
  const v = () => M().view, put = (x: Partial<View>) => act({ op: "set_view", view: { ...v(), ...x } });
  const [kind, setKind] = createSignal<MonitorKind["type"]>("drag");
  const [bid, setBid] = createSignal(0), [bid2, setBid2] = createSignal(0);
  const add = () => {
    const first = M().boundaries[0]?.id ?? 0, a = bid() || first, b2 = bid2() || M().boundaries[1]?.id || first;
    if (!a) return toast("Add a boundary first: monitors watch a value on a boundary.");
    const k = kind(), name = (id: number) => boundaryOf(id)?.name ?? "";
    const mk: Monitor = k === "pressure_drop" ? { name: `Pressure drop, ${name(a)} to ${name(b2)}`, kind: { type: k, from: a, to: b2 } } : { name: `${MON.find(x => x[0] === k)![1]} ${name(a)}`, kind: { type: k, boundary: a } as MonitorKind };
    act({ op: "set_monitors", monitors: [...M().monitors, mk] });
  };
  return <>
    <Sec t="Results" h="resultsfield">
      <Show when={!out()!.has_results}><Note>Not solved yet: Solve › Solve meshes and solves. What you choose here is kept with the model.</Note></Show>
      <F l="Colour by"><Pick value={v().field} set={x => put({ field: x })} options={FIELDS.filter(f => f[0] !== "temperature" || M().physics.heat)} /></F>
      <F l="Colour map" hint="Viridis, Inferno and Cividis read evenly from low to high, also in grey print and for colour-blind readers."><Pick value={v().map} set={x => put({ map: x })} options={MAPS} /></F>
      <F l="Bands" hint="Steps of colour instead of a smooth shade; 0 is smooth."><Num value={v().bands} min={0} sig={3} set={x => put({ bands: Math.round(x) })} /></F>
      <F l="Range" hint="Automatic follows the field; a range set by hand is kept with the model, so several models can be compared in the same colours."><div class="irange"><span>{v().ranges?.[v().field] ? `${fmt(FKIND[v().field] ?? "ratio", v().ranges![v().field][0], 4)} to ${fmt(FKIND[v().field] ?? "ratio", v().ranges![v().field][1], 4)}` : "Automatic"}</span>
        <button class="btn sm" disabled={!out()!.has_results} onClick={askRange}>Set…</button><Show when={v().ranges?.[v().field]}><button class="btn sm" onClick={() => { const r = { ...v().ranges }; delete r[v().field]; put({ ranges: r }); }}>Auto</button></Show></div></F>
      <F l="Show"><div class="checks"><Check label="Streamlines" value={v().streamlines} set={x => put({ streamlines: x })} /><Check label="Arrows" value={v().arrows} set={x => put({ arrows: x })} /><Check label="Mesh" value={v().mesh} set={x => put({ mesh: x })} /></div></F>
      <Show when={v().arrows}><F l="Arrows" hint="About how many arrows are drawn over the region."><div class="slider"><span>few</span><input type="range" min="100" max="3000" step="100" value={arrowCount()} onChange={e => setArrowCount(+e.currentTarget.value)} /><span>many</span></div></F></Show>
      <Show when={out()!.has_results}><div class="ibtns"><button class="btn sm" onClick={() => cmd("values")}>Values…</button><button class="btn sm" onClick={() => cmd("probe")}>Probe</button><button class="btn sm" onClick={() => cmd("lineplot")}>Line plot</button></div></Show>
    </Sec>
    <Sec t="Monitors" h="values">
      <For each={M().monitors} fallback={<p class="muted">None. A monitor is a number watched while solving and listed with the results.</p>}>{(m, i) =>
        <div class="irow"><Icon n="monitor" /><span>{m.name}</span><button class="x" title="Remove this monitor" onClick={() => act({ op: "set_monitors", monitors: M().monitors.filter((_, j) => j !== i()) })}>×</button></div>}</For>
      <div class="madd"><select onChange={e => setKind(e.currentTarget.value as MonitorKind["type"])}><For each={MON}>{([k, l]) => <option value={k}>{l}</option>}</For></select>
        <select onChange={e => setBid(+e.currentTarget.value)}><For each={M().boundaries}>{b => <option value={b.id}>{b.name}</option>}</For></select>
        <Show when={kind() === "pressure_drop"}><span>to</span><select onChange={e => setBid2(+e.currentTarget.value)}><For each={M().boundaries}>{(b, i) => <option value={b.id} selected={i() === 1}>{b.name}</option>}</For></select></Show>
        <button class="btn sm" onClick={add}>Add</button></div>
    </Sec>
    <Sec t="Probes and lines" h="probes">
      <For each={M().probes}>{(pr, i) => <div class="irow"><Icon n="probe" /><span title="Double-click to rename" onDblClick={async () => { const n = await askText("Rename the probe", "Name", pr.name); if (n !== null) act({ op: "set_probes", probes: M().probes.map((q, j) => (j === i() ? { ...q, name: n } : q)) }); }}>{pr.name || `Probe ${i() + 1}`}</span><small>{num(toU("len", pr.x), 4)}, {num(toU("len", pr.y), 4)} {ulabel("len")}</small><button class="x" title="Remove this probe" onClick={() => act({ op: "set_probes", probes: M().probes.filter((_, j) => j !== i()) })}>×</button></div>}</For>
      <For each={M().lines}>{(l, i) => <div class="irow"><Icon n="lineplot" /><span title="Click to plot; double-click to rename" onClick={() => out()!.has_results && linePlot(i())} onDblClick={async () => { const n = await askText("Rename the line", "Name", l.name); if (n !== null) act({ op: "set_lines", lines: M().lines.map((q, j) => (j === i() ? { ...q, name: n } : q)) }); }}>{l.name || `Line ${i() + 1}`}</span><small>{l.n} points</small><Show when={out()!.has_results}><button class="btn sm" onClick={() => linePlot(i())}>Plot</button></Show><button class="x" title="Remove this line" onClick={() => act({ op: "set_lines", lines: M().lines.filter((_, j) => j !== i()) })}>×</button></div>}</For>
      <Show when={!M().probes.length && !M().lines.length}><p class="muted">None yet. Results › Probe and Line Plot place them in the view.</p></Show>
    </Sec>
  </>;
};

// ---------------------------------------------------------------- the panel

export const Inspector = () => {
  const s = sel;
  return (
    <aside class="insp">
      <header><b>Properties</b><Help k="inspector" /><Show when={setupOpen()}><span class="grow" /><button class="btn sm" title="Back to the Setup steps" onClick={() => setSel({ kind: "model" })}>Back to Setup</button></Show></header>
      <div class="ibody">
        <Switch fallback={<ModelCard />}>
          <Match when={s().kind === "geometry"}><GeometryCard /></Match>
          <Match when={s().kind === "shape" && shapeOf((s() as { id: number }).id)}>{x => <ShapeCard s={x()} />}</Match>
          <Match when={s().kind === "edge" && (s() as { shape: number; edge: number })}>{x => <EdgeCard shape={x().shape} edge={x().edge} />}</Match>
          <Match when={s().kind === "fluid"}><FluidCard /></Match>
          <Match when={s().kind === "physics"}><PhysicsCard /></Match>
          <Match when={s().kind === "boundaries"}><BoundariesCard /></Match>
          <Match when={s().kind === "boundary" && boundaryOf((s() as { id: number }).id)}>{x => <BoundaryCard b={x()} />}</Match>
          <Match when={s().kind === "mesh"}><MeshCard /></Match>
          <Match when={s().kind === "solver"}><SolverCard /></Match>
          <Match when={s().kind === "results"}><ResultsCard /></Match>
        </Switch>
      </div>
    </aside>
  );
};
export { fromU };
