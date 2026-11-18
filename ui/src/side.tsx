// The sidebar (FAMILY §6): one icon and label per view, Setup first, groups split by rules. The first
// five views show the 2D view with their ribbon tab; Values, Plots, Checks and Verification are
// pages. Also the step-by-step Setup panel and those pages.
import { For, Show, JSX, createEffect, createResource, createSignal, on } from "solid-js";
import { sel, out, M, view, setView, View, PAGES, tab, setTab, setSel, setupOpen, setSetupOpen, setupStep, setSetupStep, act, call, toast, fmt, toU, ulabel, running, Tab, setBottom, setBottomOpen } from "./store";
import { C, cmd } from "./cmds";
import { Icon, Num } from "./ui";
import { Help } from "./help";
import { num, Fluid, invoke } from "./api";
import { issues, runChecks, genMesh, solve, mesh, summary, probeData, exportLine, exportProbes } from "./sim";
import type { ValueRow, Report, Sample } from "./sim";
import { ValuesTable, valuesTsv, LineChart, QUANT, ChecksPanel, VerTable, verSummary, verificationPages, checkCount, checkBad } from "./simui";
import { setReportPages } from "./print";
import { setBackstage } from "./store";

type Item = [View | "setup", string, string, string];
/** id, icon, label, what it is for (the tooltip). */
const SIDE: (Item | "-")[] = [
  ["setup", "setup", "Setup", "Set up a model step by step"], "-",
  ["geometry", "rect", "Geometry", "Draw the flow region"], ["physics", "fluid", "Physics", "Fluid, heat and boundaries"], ["mesh", "mesh", "Mesh", "Mesh the region and read its quality"], "-",
  ["solve", "solve", "Solve", "Solve and follow the residuals"], ["results", "contour", "Results", "Contours, streamlines, probes"], "-",
  ["values", "values", "Values", "Flow rates, pressure drops, forces, heat"], ["plots", "plots", "Plots", "Probes and plots along lines"], "-",
  ["checks", "wizard", "Checks", "What is missing before a solve"], ["verification", "verify", "Verify", "How the solver compares with known answers"],
];
/** The tree section each 2D view selects. */
const SECTION: Partial<Record<View, "geometry" | "fluid" | "mesh" | "solver" | "results">> = { geometry: "geometry", physics: "fluid", mesh: "mesh", solve: "solver", results: "results" };

/** Open a view: the 2D views also bring their ribbon tab and tree section. */
export const openView = (v: View) => {
  setView(v);
  const s = SECTION[v];
  // With Setup open the selection stays, so the Setup steps stay in view.
  if (s) { setTab(v as Tab); if (!setupOpen()) setSel({ kind: s }); }
  if (v === "checks") runChecks(false);
};

/** Sidebar › Setup: open it at the model, or close it; when properties cover it, show it again. */
const toggleSetup = () => {
  if (setupOpen() && sel().kind !== "model") return setSel({ kind: "model" });
  setSel({ kind: "model" });
  setSetupOpen(!setupOpen());
};

export const Side = () => {
  // A ribbon tab of a 2D view (or a click in the tree) brings its view back.
  createEffect(on(tab, t => { if (["geometry", "physics", "mesh", "solve", "results"].includes(t)) setView(t as View); }, { defer: true }));
  const badge = (v: Item[0]) => v === "checks" && checkCount() ? String(checkCount()) : "";
  return (
    <nav class="side">
      <For each={SIDE}>{x => x === "-" ? <hr /> :
        <button classList={{ on: x[0] === "setup" ? setupOpen() : view() === x[0], bad: x[0] === "checks" && checkBad() }} onClick={() => x[0] === "setup" ? toggleSetup() : openView(x[0])} title={`${x[2]}: ${x[3]}`}>
          <Icon n={x[1]} /><span>{x[2]}</span><Show when={badge(x[0])}><i class="sbadge">{badge(x[0])}</i></Show>
        </button>}</For>
    </nav>
  );
};

// ---------------------------------------------------------------- the pages
const Page = (p: { title: string; help: string; lead: string; tools?: JSX.Element; children: JSX.Element }) => (
  <div class="pgview">
    <header><div><h1>{p.title}<Help k={p.help} /></h1><p>{p.lead}</p></div><span class="grow" />{p.tools}</header>
    <div class="pgbody">{p.children}</div>
  </div>
);
/** An empty page says what the view is for and offers the first action (FAMILY §7). */
const Empty = (p: { icon: string; title: string; text: string; children?: JSX.Element }) => (
  <div class="pgempty"><Icon n={p.icon} size={34} /><b>{p.title}</b><span>{p.text}</span><div>{p.children}</div></div>
);
const solveBtn = () => <button class="btn pri" disabled={running()} onClick={() => { openView("solve"); solve(); }}>Solve</button>;

const ValuesPage = () => {
  const key = () => out()?.has_results ? `${out()!.revision}|${out()!.m.units}|${summary()?.history.length ?? 0}` : null;
  const [rows] = createResource(key, () => call<ValueRow[]>("values"));
  const copy = () => { const v = rows(); if (v) navigator.clipboard?.writeText(valuesTsv(v)).then(() => toast("Copied: paste it into any spreadsheet.", true)).catch(() => toast("The clipboard is not available here.")); };
  return <Page title="Values" help="values" lead={`The numbers read off the solution, per the model's depth (${fmt("len", M().physics.depth)}). Forces and wall heat flows come from the balance of the solved equations at the wall.`}
    tools={<Show when={rows()}><button class="btn" onClick={copy}>Copy as a table</button></Show>}>
    <Show when={out()?.has_results} fallback={<Empty icon="values" title="No results yet" text="Solve the model to read its flow rates, pressure drops, forces, heat flows and balances here.">{solveBtn()}</Empty>}>
      <Show when={rows()}><ValuesTable rows={rows()!} /></Show>
    </Show>
  </Page>;
};

/** One plot line: its chart, with the quantity to plot. */
const LineCard = (p: { index: number }) => {
  const l = () => M().lines[p.index];
  const key = () => out()?.has_results ? `${out()!.revision}|${p.index}|${JSON.stringify(l())}` : null;
  const [pts] = createResource(key, () => call<[number, Sample][]>("line", { index: p.index }));
  const [q, setQ] = createSignal<keyof Sample>(M().physics.heat && M().view.field === "temperature" ? "temperature" : "speed");
  const meta = () => QUANT.find(x => x[0] === q())!;
  const data = () => (pts() ?? []).filter(x => x[1].inside && x[1][q()] !== null).map(x => ({ x: toU("len", x[0]), y: toU(meta()[2], x[1][q()] as number) }));
  return <section class="pgcard">
    <h3>{l()?.name || `Line ${p.index + 1}`}<span class="muted">from ({num(toU("len", l()?.a[0] ?? 0), 4)}, {num(toU("len", l()?.a[1] ?? 0), 4)}) to ({num(toU("len", l()?.b[0] ?? 0), 4)}, {num(toU("len", l()?.b[1] ?? 0), 4)}) {ulabel("len")}</span>
      <span class="grow" /><select onChange={e => setQ(e.currentTarget.value as keyof Sample)}><For each={QUANT.filter(x => x[0] !== "temperature" || M().physics.heat)}>{x => <option value={x[0]} selected={x[0] === q()}>{x[1]}</option>}</For></select>
      <button class="btn sm" onClick={() => exportLine(p.index)}>CSV</button></h3>
    <LineChart cls="big" legend={false} name={`${l()?.name || "line"} ${meta()[1]}`} x={`Distance along the line (${ulabel("len")})`} y={`${meta()[1]}${ulabel(meta()[2]) ? ` (${ulabel(meta()[2])})` : ""}`} series={() => [{ label: meta()[1], data: data() }]} />
  </section>;
};
const PlotsPage = () => (
  <Page title="Plots" help="probes" lead="Values at the probes, and the solution along the lines drawn across the flow."
    tools={<><button class="btn" onClick={() => { openView("results"); cmd("probe"); }}>Add a probe</button><button class="btn" onClick={() => { openView("results"); cmd("lineplot"); }}>Draw a line</button></>}>
    <Show when={out()?.has_results} fallback={<Empty icon="plots" title="No results yet" text="Solve the model, then place probes and draw lines in the Results view: their values and plots appear here.">{solveBtn()}</Empty>}>
      <Show when={M().probes.length || M().lines.length} fallback={<Empty icon="probe" title="No probes or lines" text="Results › Probe: click points in the flow. Results › Line Plot: click the two ends of a line.">
        <button class="btn pri" onClick={() => { openView("results"); cmd("probe"); }}>Add a probe</button><button class="btn" onClick={() => { openView("results"); cmd("lineplot"); }}>Draw a line</button></Empty>}>
        <Show when={M().probes.length}>
          <section class="pgcard"><h3>Probes<span class="grow" /><button class="btn sm" onClick={exportProbes}>CSV</button></h3>
            <table class="tbl"><thead><tr><th>Probe</th><th class="n">x</th><th class="n">y</th><th class="n">Speed</th><th class="n">Pressure</th><Show when={M().physics.heat}><th class="n">Temperature</th></Show><th class="n">Vorticity</th></tr></thead>
              <tbody><For each={probeData()}>{([name, s]) => <tr><td>{name}</td><td class="n">{fmt("len", s.x)}</td><td class="n">{fmt("len", s.y)}</td>
                <Show when={s.inside} fallback={<td class="muted" colSpan={M().physics.heat ? 4 : 3}>outside the fluid</td>}><td class="n">{fmt("speed", s.speed)}</td><td class="n">{fmt("pressure", s.pressure)}</td><Show when={M().physics.heat}><td class="n">{fmt("temp", s.temperature ?? NaN)}</td></Show><td class="n">{fmt("vorticity", s.vorticity)}</td></Show></tr>}</For></tbody></table>
          </section>
        </Show>
        <For each={M().lines}>{(_, i) => <LineCard index={i()} />}</For>
      </Show>
    </Show>
  </Page>
);

const ChecksPage = () => {
  const n = () => out()!.numbers;
  return <Page title="Checks" help="checks" lead="What Datum Flow checks before it solves. An error stops the solve and says how to fix it; a warning is worth reading." tools={<button class="btn pri" onClick={() => runChecks(false)}>Check again</button>}>
    <section class="pgcard"><h3>What kind of flow this is<Help k="numbers" /></h3>
      <div class="pgnums">
        <div><small>Reynolds number</small><b>{n().speed > 0 ? num(n().re, 3) : "—"}</b></div>
        <div><small>Rayleigh number</small><b>{M().physics.buoyancy ? num(n().ra, 3) : "—"}</b></div>
        <div><small>Prandtl number</small><b>{num(n().pr, 3)}</b></div>
        <div><small>Reference speed</small><b>{fmt("speed", n().speed, 3)}</b></div>
        <div><small>Reference length</small><b>{fmt("len", n().length, 3)}</b></div>
      </div>
      <p classList={{ pgwarn: !!out()!.turbulent, pgok: !out()!.turbulent }}>{out()!.turbulent ?? (n().through ? "Flow through the region: laminar at this Reynolds number." : "A closed region: laminar at these numbers.")}</p>
    </section>
    <section class="pgcard"><h3>The setup</h3><Show when={issues()} fallback={<p class="muted">Not checked yet.</p>}><ChecksPanel /></Show></section>
  </Page>;
};

const VerificationPage = () => {
  const [r] = createResource(() => call<Report>("verification"));
  const s = () => r() ? verSummary(r()!) : { pass: 0, fail: 0 };
  return <Page title="Verification" help="verification" lead="The cases this version was tested on before release: problems whose answers are known. Any FAIL blocks a release. Click a row for its source."
    tools={<Show when={r()}><button class="btn" onClick={() => { setReportPages(verificationPages(r()!)); setBackstage("print"); }}>Print or save as PDF…</button></Show>}>
    <Show when={r()}>
      <div class="vhead"><b class="chip big" classList={{ ok: s().fail === 0 && s().pass > 0, bad: s().fail > 0 }}>{s().fail ? `${s().fail} FAIL` : s().pass ? "ALL PASS" : "NO CHECKS"}</b>
        <div><b>Datum Flow {r()!.version}: {s().pass} of {r()!.checks.length} checks pass</b><span class="muted">Run on {r()!.date}{r()!.machine ? ` on ${r()!.machine}` : ""}{r()!.seconds ? `, ${num(r()!.seconds, 3)} s in all` : ""}.</span></div></div>
      <Show when={r()!.checks.length} fallback={<p class="inote">This record has no checks yet.</p>}><VerTable r={r()!} /></Show>
      <Show when={r()!.not_verified.length}><h4 class="vtier">Not verified in this version</h4><ul class="vnot"><For each={r()!.not_verified}>{x => <li>{x}</li>}</For></ul></Show>
    </Show>
  </Page>;
};

/** The page of the open view, or nothing when it is a 2D view. */
export const ViewPage = () => (
  <Show when={PAGES.includes(view())}>
    <Show when={view() === "values"}><ValuesPage /></Show>
    <Show when={view() === "plots"}><PlotsPage /></Show>
    <Show when={view() === "checks"}><ChecksPage /></Show>
    <Show when={view() === "verification"}><VerificationPage /></Show>
  </Show>
);

// ---------------------------------------------------------------- the Setup panel
const PRESETS: [string, string][] = [["water", "Water"], ["air", "Air"], ["oil", "Light oil"]];
const TEMPLATES: [string, string, string][] = [["channel", "Channel", "inlet"], ["cavity-100", "Cavity", "lid"], ["step", "Step", "poly"], ["cylinder", "Cylinder", "hcircle"], ["heated-1e4", "Heated box", "heat"]];

type Step = { t: string; icon: string; done: () => boolean; body: () => JSX.Element };
const STEPS: Step[] = [
  { t: "Name and units", icon: "model", done: () => M().name !== "Untitled", body: () => <>
    <p>Give the model a name, and choose the units it is shown in. The file always keeps SI values, so you can switch at any time.</p>
    <div class="form"><label>Name</label><input value={M().name} onChange={e => act({ op: "set_name", name: e.currentTarget.value })} />
      <label>Units</label><div class="seg"><button classList={{ on: M().units === "SI" }} onClick={() => C.units("SI")}>SI</button><button classList={{ on: M().units === "US" }} onClick={() => C.units("US")}>US</button></div></div>
  </> },
  { t: "Geometry", icon: "rect", done: () => M().shapes.length > 0 && !out()!.geometry_problem, body: () => <>
    <p>Draw the flow region: rectangles, circles and polygons that add to it, and shapes that cut bodies out of it. Or start from a template, which comes with its boundaries set.</p>
    <div class="stpl"><For each={TEMPLATES}>{([id, l, i]) => <button onClick={() => C.template(id)}><Icon n={i} /><span>{l}</span></button>}</For></div>
    <div class="srow"><button class="btn" onClick={() => { openView("geometry"); C.tool("rect"); }}>Rectangle</button><button class="btn" onClick={() => { openView("geometry"); C.tool("circle"); }}>Circle</button><button class="btn" onClick={() => { openView("geometry"); C.tool("poly"); }}>Polygon</button><button class="btn" onClick={() => cmd("import_dxf")}>Import DXF…</button></div>
    <Show when={M().shapes.length}><p class={out()!.geometry_problem ? "pgwarn" : "pgok"}>{out()!.geometry_problem ?? `${M().shapes.length} shape${M().shapes.length === 1 ? "" : "s"}; the region is ${fmt("len", out()!.bounds![2] - out()!.bounds![0], 3)} by ${fmt("len", out()!.bounds![3] - out()!.bounds![1], 3)}.`}</p></Show>
  </> },
  { t: "Fluid", icon: "fluid", done: () => true, body: () => {
    const load = async (name: string, t: number) => { try { await act({ op: "set_fluid", fluid: await invoke<Fluid>("fluid_preset", { name, t }) }); } catch (e) { toast(String(e)); } };
    return <>
      <p>Choose the fluid and the temperature its properties are taken at. For another fluid, use Custom in Physics › Fluid and type its values.</p>
      <div class="form"><label>Fluid</label><select onChange={e => load(e.currentTarget.value, M().fluid.at)}><For each={PRESETS}>{([k, l]) => <option value={k} selected={k === M().fluid.preset}>{l}</option>}</For><Show when={M().fluid.preset === "custom"}><option value="custom" selected>{M().fluid.name}</option></Show></select>
        <label>At</label><Num kind="temp" value={M().fluid.at} disabled={M().fluid.preset === "custom"} set={v => load(M().fluid.preset, v)} /></div>
      <p class="muted">Density {fmt("density", M().fluid.rho, 4)} · viscosity {fmt("viscosity", M().fluid.mu, 3)}. {M().fluid.source}</p>
    </>;
  } },
  { t: "Heat", icon: "heat", done: () => true, body: () => <>
    <p>Turn heat on to solve the temperature too, and buoyancy for natural convection (warm fluid rises). Leave both off for flow alone.</p>
    <label class="chk"><input type="checkbox" checked={M().physics.heat} onChange={e => C.heat(e.currentTarget.checked)} /> Heat: solve the temperature</label>
    <label class="chk"><input type="checkbox" checked={M().physics.buoyancy} onChange={e => C.buoyancy(e.currentTarget.checked)} /> Buoyancy: warm fluid rises</label>
  </> },
  { t: "Boundaries", icon: "bc", done: () => out()!.edges.length > 0 && out()!.edges.every(e => e.boundary), body: () => {
    const free = () => out()!.edges.filter(e => !e.boundary).length;
    return <>
      <p>Say what every edge is: where the fluid comes in, where it leaves, the walls, and any symmetry line. Add one, then click its edges in the view.</p>
      <div class="srow"><button class="btn" onClick={C.inlet}>Inlet</button><button class="btn" onClick={C.outlet}>Outlet</button><button class="btn" onClick={C.wall}>Wall</button><button class="btn" onClick={C.symmetry}>Symmetry</button><button class="btn" disabled={!free()} onClick={C.wallRest}>Wall the rest</button></div>
      <p class={free() ? "pgwarn" : "pgok"}>{!out()!.edges.length ? "Draw the geometry first." : free() ? `${free()} edge${free() === 1 ? " has" : "s have"} no boundary yet (drawn dashed in red).` : `Every edge has a boundary: ${M().boundaries.map(b => b.name).join(", ")}.`}</p>
      <Show when={M().physics.heat}><p class="muted">Heat is on: give the inlets a temperature, and the walls a temperature, a heat flux or insulation, in Physics.</p></Show>
    </>;
  } },
  { t: "Mesh", icon: "mesh", done: () => !!out()!.has_mesh, body: () => <>
    <p>Datum Flow meshes the region in triangles, finer at walls, corners and wakes. Generate it to see the mesh and its quality; Solve also meshes by itself when needed.</p>
    <div class="srow"><button class="btn pri" disabled={!M().shapes.length} onClick={() => { openView("mesh"); genMesh(); }}>Generate the mesh</button></div>
    <Show when={out()!.has_mesh && mesh()}><p class="pgok">{mesh()!.quality.elements.toLocaleString()} elements, {mesh()!.quality.unknowns.toLocaleString()} unknowns; smallest angle {num(mesh()!.quality.min_angle, 3)}°.</p></Show>
  </> },
  { t: "Check and solve", icon: "solve", done: () => !!out()!.has_results && !!summary()?.converged, body: () => <>
    <p>The check says what is still missing. When it is ready, solve: the residuals fall as it runs, and the results open when it is done.</p>
    <div class="srow"><button class="btn" onClick={() => { setBottom("checks"); setBottomOpen(true); runChecks(); }}>Check</button><button class="btn pri" disabled={running() || !M().shapes.length} onClick={() => { openView("solve"); solve(); }}>Solve</button></div>
    <Show when={issues()?.length}><p classList={{ pgwarn: checkBad(), pgok: !checkBad() }}>{issues()![0].level === "ok" ? issues()![0].text : `${issues()!.filter(i => i.level === "error").length} to fix, ${issues()!.filter(i => i.level === "warning").length} to read: see Checks below.`}</p></Show>
    <Show when={summary()}><p class={summary()!.converged ? "pgok" : "pgwarn"}>{summary()!.message}</p></Show>
  </> },
];

export const SetupPanel = () => {
  const i = () => Math.min(setupStep(), STEPS.length - 1);
  const go = (k: number) => setSetupStep(Math.max(0, Math.min(STEPS.length - 1, k)));
  createEffect(on(i, k => { const v = (["geometry", "geometry", "physics", "physics", "physics", "mesh", "solve"] as View[])[k]; if (PAGES.includes(view())) openView(v); }, { defer: true }));
  return (
    <aside class="setup">
      <header><b>Setup</b><Help k="setup" /><span class="grow" /><button class="x" title="Close (reopen it from Setup in the sidebar)" onClick={() => setSetupOpen(false)}>×</button></header>
      <ol class="ssteps"><For each={STEPS}>{(s, k) => <li classList={{ on: k() === i(), done: s.done() }} onClick={() => go(k())}><i>{s.done() && k() !== i() ? "✓" : k() + 1}</i><span>{s.t}</span></li>}</For></ol>
      <div class="sbody"><h3><Icon n={STEPS[i()].icon} />{STEPS[i()].t}</h3>{STEPS[i()].body()}</div>
      <footer><button class="btn" disabled={i() === 0} onClick={() => go(i() - 1)}>Back</button><span class="grow" />
        <Show when={i() < STEPS.length - 1} fallback={<button class="btn pri" onClick={() => setSetupOpen(false)}>Done</button>}><button class="btn pri" onClick={() => go(i() + 1)}>Next</button></Show></footer>
    </aside>
  );
};
