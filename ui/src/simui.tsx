// The engine's results as panels and dialogs: the checks list, the residual and monitor charts,
// the mesh quality report, the legend over the view, values, line plots, the verification page
// and the pages of the PDF report.
import { For, Show, JSX, createSignal, createEffect, createMemo, onMount, onCleanup, untrack } from "solid-js";
import { Chart, LineController, LineElement, PointElement, LinearScale, LogarithmicScale, Tooltip, Legend, Filler } from "chart.js";
import { num } from "./api";
import { out, M, call, toast, openDialog, closeDialog, setSel, setTab, setBackstage, backstage, toU, ulabel, fmt, themeTick, KIND_NAME } from "./store";
import { Dialog, Icon } from "./ui";
import { Help } from "./help";
import { wire } from "./cmds";
import { fit, viewPicture, cursor } from "./canvas";
import { setPanel } from "./bottom";
import { setReportPages } from "./print";
import { mesh, showQuality, setShowQuality, issues, runChecks, running, rows, monValues, monNames, summary, field, placing, lineStart, hoverSample, hoverAt, ui, exportProbes, exportLine, stop, Issue, Sample, ValueRow, Report, Check, Summary, ResRow } from "./sim";
import { lut, fieldRange, QUALITY, meshPicture } from "./draw";

Chart.register(LineController, LineElement, PointElement, LinearScale, LogarithmicScale, Tooltip, Legend, Filler);
// For the UI tests: how many points each series of a chart holds.
(window as unknown as { __chartData: (el: HTMLCanvasElement) => number[] | null }).__chartData = el => Chart.getChart(el)?.data.datasets.map(d => d.data.length) ?? null;
/** The family's series colours (Okabe and Ito: safe for colour-blind readers). */
export const SERIES = ["#0072B2", "#E69F00", "#009E73", "#D55E00", "#CC79A7", "#56B4E9", "#F0E442", "#000000"];
const css = (n: string) => getComputedStyle(document.documentElement).getPropertyValue(n).trim() || "#888";

type Series = { label: string; data: { x: number; y: number }[]; colour?: string };
/** A line chart in the family style: white plot, thin axes, light grid, crosshair tooltip. */
const makeChart = (cv: HTMLCanvasElement, o: { x: string; y: string; log?: boolean; legend?: boolean; print?: boolean }) => {
  const ink = o.print ? "#3B4859" : css("--text2"), grid = o.print ? "#E6E8E5" : css("--line2"), axis = o.print ? "#B9BFC2" : css("--line3");
  const scale = (title: string) => ({ title: { display: !!title, text: title, color: ink, font: { family: "IBM Plex Mono", size: 10 } }, ticks: { color: ink, font: { family: "IBM Plex Mono", size: 10 }, maxTicksLimit: 8 }, grid: { color: grid, lineWidth: 1 }, border: { color: axis } });
  return new Chart(cv, {
    type: "line", data: { datasets: [] },
    options: {
      responsive: true, maintainAspectRatio: false, animation: false, parsing: false, normalized: true,
      interaction: { mode: "nearest", axis: "x", intersect: false },
      elements: { point: { radius: 0, hoverRadius: 3 }, line: { borderWidth: 1.6, tension: 0 } },
      scales: { x: { type: "linear", ...scale(o.x) }, y: (o.log
        ? { type: "logarithmic", ...scale(o.y), ticks: { ...scale(o.y).ticks, maxTicksLimit: 12, callback: (v: number | string) => { const e = Math.log10(Number(v)); return Math.abs(e - Math.round(e)) < 1e-9 ? `1e${Math.round(e)}` : ""; } } }
        : { type: "linear", ...scale(o.y), ticks: { ...scale(o.y).ticks, callback: (v: number | string) => num(Number(v), 4) } }) as never },
      plugins: { legend: { display: o.legend !== false, position: "top", align: "end", labels: { color: ink, boxWidth: 14, boxHeight: 2, font: { family: "IBM Plex Sans", size: 11 } } }, tooltip: { callbacks: { label: c => `${c.dataset.label}: ${num(c.parsed.y ?? NaN, 4)}` } } },
    },
  });
};
const setSeries = (c: Chart, s: Series[]) => { c.data.datasets = s.map((d, i) => ({ label: d.label, data: d.data, borderColor: d.colour ?? SERIES[i % SERIES.length], backgroundColor: d.colour ?? SERIES[i % SERIES.length] })); c.update("none"); };
/** A chart that follows its series; a PNG button saves it. */
export const LineChart = (p: { series: () => Series[]; x: string; y: string; log?: boolean; legend?: boolean; name: string; cls?: string }) => {
  let cv!: HTMLCanvasElement, chart: Chart | null = null;
  // Made again when the theme changes (its axes and legend take the theme's colours), then fed its series.
  onMount(() => {
    createEffect(() => { themeTick(); chart?.destroy(); chart = makeChart(cv, p); setSeries(chart, untrack(p.series)); });
    createEffect(() => { const s = p.series(); if (chart) setSeries(chart, s); });
  });
  onCleanup(() => chart?.destroy());
  const png = async () => { const path = await call<string | null>("save_file", { name: `${M().name} ${p.name}.png`, data: chartPicture(p.series(), p) }); if (path) toast(`Saved ${path}`, true); };
  return <div class={`schart ${p.cls ?? ""}`}><canvas ref={cv} /><button class="btn sm spng" title="Save this chart as a picture" onClick={png}>PNG</button></div>;
};
/** A chart drawn off screen on white, as a PNG data URL (for saving and for the report). */
const chartPicture = (series: Series[], o: { x: string; y: string; log?: boolean; legend?: boolean }, w = 1100, h = 420) => {
  const host = document.createElement("div"); host.style.cssText = `position:fixed;left:-9999px;top:0;width:${w}px;height:${h}px;background:#fff`;
  const cv = document.createElement("canvas"); host.append(cv); document.body.append(host);
  const c = makeChart(cv, { ...o, print: true });
  c.options.plugins!.legend!.labels!.color = "#3B4859";
  setSeries(c, series);
  const flat = document.createElement("canvas"); flat.width = cv.width; flat.height = cv.height;
  const ctx = flat.getContext("2d")!; ctx.fillStyle = "#fff"; ctx.fillRect(0, 0, flat.width, flat.height); ctx.drawImage(cv, 0, 0);
  const url = flat.toDataURL("image/png"); c.destroy(); host.remove();
  return url;
};

// ---------------------------------------------------------------- residuals and monitors
const residualSeries = (r: ResRow[], heat: boolean): Series[] => {
  const pts = (k: "momentum" | "continuity" | "energy") => r.map((x, i) => ({ x: i, y: Math.max(x[k], 1e-12) }));
  return [{ label: "Momentum", data: pts("momentum") }, { label: "Continuity", data: pts("continuity") }, ...(heat ? [{ label: "Energy", data: pts("energy") }] : [])];
};
const RunLine = () => {
  const last = () => rows()[rows().length - 1];
  return <div class="srun" classList={{ on: running() }}>
    <Show when={running()} fallback={<Show when={summary()}>{s => <><b classList={{ ok: s().converged, bad: !s().converged && !s().stopped }}>{s().converged ? "Converged" : s().stopped ? "Stopped" : "Not converged"}</b><span>{s().message}</span><span class="muted">{s().stats.unknowns.toLocaleString()} unknowns · {num(s().stats.seconds, 3)} s · {s().stats.linear_solver}</span></>}</Show>}>
      <i class="spin" /><b>Solving</b>
      <Show when={last()} fallback={<span>Getting the mesh and the equations ready…</span>}>{l => <span>Iteration {rows().length - 1} · {l().kind}{l().load < 1 ? ` · at ${Math.round(l().load * 100)}% of the full flow (continuation)` : ""} · residual {num(Math.max(l().momentum, l().continuity, l().energy), 2)}</span>}</Show>
      <button class="btn sm" onClick={stop}>Stop</button>
    </Show>
  </div>;
};
const ResidualPanel = () => <div class="spanel"><RunLine /><LineChart name="residuals" x="Iteration" y="" log series={() => residualSeries(rows(), !!M().physics.heat)} /></div>;
/** Monitors: what unit each is shown in (known for sure once a solve has finished). */
const KIND_OF: Record<string, string> = { drag: "force", lift: "force", flow_rate: "flow", pressure_drop: "pressure", mean_temperature: "temp", heat_flow: "power", nusselt: "ratio", speed: "speed" };
const monKind = (i: number) => summary()?.monitor_kinds[i] ?? KIND_OF[M().monitors[i]?.kind.type ?? ""] ?? "ratio";
const MonitorPanel = () => <div class="spanel"><RunLine />
  <div class="smons"><For each={monNames()}>{(name, i) => {
    const last = () => { const v = monValues(); return v.length ? v[v.length - 1][i()] : NaN; };
    return <div class="smon"><header><b>{name}</b><span>{fmt(monKind(i()), last(), 5)}</span></header>
      <LineChart name={name} x="Iteration" y={ulabel(monKind(i()))} legend={false} series={() => [{ label: name, data: monValues().map((v, k) => ({ x: k, y: toU(monKind(i()), v[i()]) })).filter(p => isFinite(p.y)) }]} /></div>;
  }}</For></div></div>;

// ---------------------------------------------------------------- checks
const goIssue = (i: Issue) => { if (i.boundary) { setSel({ kind: "boundary", id: i.boundary }); setTab("physics"); } else if (/mesh|triangle/i.test(i.text)) { setSel({ kind: "mesh" }); setTab("mesh"); } else if (/turbulent|Re ≈|Ra ≈/.test(i.text)) { setSel({ kind: "solver" }); setTab("solve"); } else if (/heat|temperature|gravity|buoyancy/i.test(i.text)) { setSel({ kind: "physics" }); setTab("physics"); } else { setSel({ kind: "boundaries" }); setTab("physics"); } };
export const ChecksPanel = () => (
  <div class="schecks">
    <For each={issues() ?? []}>{i => <button class={`scheck ${i.level}`} onClick={() => goIssue(i)}><i>{i.level === "ok" ? "✓" : "!"}</i><div><b>{i.text}</b><Show when={i.fix}><span>{i.fix}</span></Show></div></button>}</For>
    <p class="muted">{out()?.has_mesh ? "Checked against the mesh." : "Checked without a mesh: generate it for the full list."} <button class="btn sm" onClick={() => runChecks()}>Check again</button></p>
  </div>
);
/** Errors and warnings found by the last check, for the badge on the Checks tab. */
export const checkCount = () => (issues() ?? []).filter(i => i.level !== "ok").length;
export const checkBad = () => (issues() ?? []).some(i => i.level === "error");

// ---------------------------------------------------------------- mesh quality (in the inspector)
export const MeshQuality = () => (
  <Show when={mesh()} fallback={<section class="isec"><h3>Quality<Help k="meshquality" /></h3><p class="muted">No mesh yet. Generate makes the triangles and reports how good they are.</p></section>}>{m => {
    const q = () => m().quality, total = () => Math.max(1, q().elements);
    const bands = () => [q().angle_bands[0], q().angle_bands[1], q().angle_bands[2] + q().angle_bands[3]];
    return <section class="isec squal"><h3>Quality<Help k="meshquality" /><span class="grow" /><label class="chk"><input type="checkbox" checked={showQuality()} onChange={e => setShowQuality(e.currentTarget.checked)} />Colour by quality</label></h3>
      <table class="sq"><tbody>
        <tr><th>Elements</th><td>{q().elements.toLocaleString()}</td></tr>
        <tr><th>Nodes</th><td>{q().nodes.toLocaleString()}</td></tr>
        <tr><th>Unknowns</th><td>{q().unknowns.toLocaleString()}{M().physics.heat ? ` + ${q().nodes.toLocaleString()} temperatures` : ""}</td></tr>
        <tr><th>Smallest angle</th><td classList={{ warn: q().min_angle < 20 }}>{num(q().min_angle, 3)}°</td></tr>
        <tr><th>Size ratio</th><td title="The largest ratio of the sizes of two neighbouring elements">{num(q().max_ratio, 3)} at most</td></tr>
        <tr><th>Edge length</th><td>{fmt("len", q().edges[0], 3)} to {fmt("len", q().edges[2], 3)}, mean {fmt("len", q().edges[1], 3)}</td></tr>
        <tr><th>Made in</th><td>{num(m().seconds * 1000, 3)} ms</td></tr>
      </tbody></table>
      <div class="sqbar" title="Elements by their smallest angle"><For each={bands()}>{(n, i) => <i style={{ width: `${100 * n / total()}%`, background: QUALITY[i()][1] }} />}</For></div>
      <div class="sqleg"><For each={QUALITY}>{([l, c], i) => <span><i style={{ background: c }} />{l}: {bands()[i()].toLocaleString()}</span>}</For></div>
      <h4>The five thinnest elements</h4>
      <For each={q().worst}>{w => <div class="irow"><Icon n="quality" /><span>{num(w.angle, 3)}° at {num(toU("len", w.at[0]), 3)}, {num(toU("len", w.at[1]), 3)} {ulabel("len")}</span><button class="btn sm" onClick={() => { const r = 6 * q().edges[1]; setShowQuality(true); fit([w.at[0] - r, w.at[1] - r, w.at[0] + r, w.at[1] + r]); }}>Show</button></div>}</For>
    </section>;
  }}</Show>
);

// ---------------------------------------------------------------- over the view: legend, hints, status
const Legend2 = () => {
  let cv!: HTMLCanvasElement;
  const range = createMemo(() => { const f = field(); return f ? fieldRange(f, M().view.map) : [0, 1]; });
  createEffect(() => {
    const f = field(); if (!f || !cv) return;
    const t = lut(M().view.map, M().view.bands), ctx = cv.getContext("2d")!;
    for (let y = 0; y < cv.height; y++) { const i = 3 * Math.round(255 * (1 - y / (cv.height - 1))); ctx.fillStyle = `rgb(${t[i]},${t[i + 1]},${t[i + 2]})`; ctx.fillRect(0, y, cv.width, 1); }
  });
  const ticks = () => { const [lo, hi] = range(); return [1, 0.75, 0.5, 0.25, 0].map(s => toU(field()!.kind, lo + (hi - lo) * s)); };
  return <div class="sleg"><b>{field()!.name}<small>{ulabel(field()!.kind)}</small></b><div><canvas ref={cv} width="14" height="150" /><ul><For each={ticks()}>{t => <li>{num(t, 4)}</li>}</For></ul></div></div>;
};
export const SimOverlay = () => (
  <div class="sover">
    <Show when={out()?.has_results && field() && !showQuality()}><Legend2 /></Show>
    <Show when={mesh() && showQuality()}><div class="sleg sq2"><b>Smallest angle</b><For each={QUALITY}>{([l, c, w]) => <span><i style={{ background: c }} />{l} ({w})</span>}</For></div></Show>
    <Show when={placing()}><div class="shint">{placing() === "probe" ? "Click where you want to read the values. Esc when done." : lineStart() ? "Click the other end of the line. Esc cancels." : "Click the first end of the line to plot along. Esc cancels."}</div></Show>
    <Show when={running()}><div class="sbusy"><i class="spin" />Solving · iteration {Math.max(0, rows().length - 1)}<button class="btn sm" onClick={stop}>Stop</button></div></Show>
  </div>
);
/** The values under the pointer, for the status bar. */
export const HoverValues = () => {
  createEffect(() => hoverAt(placing() ? null : cursor()));
  return <Show when={hoverSample()}>{s => <span class="shover">{fmt("speed", s().speed, 4)} · p {fmt("pressure", s().pressure, 4)}<Show when={M().physics.heat && s().temperature !== null}> · T {fmt("temp", s().temperature as number, 5)}</Show></span>}</Show>;
};

// ---------------------------------------------------------------- dialogs
export const valuesTsv = (v: ValueRow[]) => ["Group\tValue\tNumber\tUnit\tNote", ...v.map(r => [r.group, r.name, toU(r.kind, r.value), ulabel(r.kind), r.note].join("\t"))].join("\n");
export const ValuesTable = (p: { rows: ValueRow[] }) => {
  const groups = () => [...new Set(p.rows.map(r => r.group))];
  return <table class="tbl svals"><tbody><For each={groups()}>{g => <>
    <tr class="sgrp"><th colSpan={3}>{g}</th></tr>
    <For each={p.rows.filter(r => r.group === g)}>{r => <tr><td>{r.name}</td><td class="n">{num(toU(r.kind, r.value), 5)} <i>{ulabel(r.kind)}</i></td><td class="muted">{r.note}</td></tr>}</For>
  </>}</For></tbody></table>;
};
export const valuesDialog = async () => {
  if (!out()?.has_results) return toast("There are no results yet: solve the model first (Solve › Solve).");
  const v = await call<ValueRow[]>("values");
  if (!v) return;
  openDialog(() => <Dialog title="Values" wide help="values" extra={<button class="btn" onClick={() => navigator.clipboard?.writeText(valuesTsv(v)).then(() => toast("Copied: paste it into any spreadsheet.", true)).catch(() => toast("The clipboard is not available here."))}>Copy as a table</button>}>
    <p class="muted">Per metre of depth × the model's depth ({fmt("len", M().physics.depth)}). Forces and wall heat flows come from the balance of the solved equations at the wall.</p>
    <ValuesTable rows={v} />
  </Dialog>);
};

export const QUANT: [keyof Sample, string, string][] = [["speed", "Speed", "speed"], ["u", "Velocity x", "speed"], ["v", "Velocity y", "speed"], ["pressure", "Pressure", "pressure"], ["temperature", "Temperature", "temp"], ["vorticity", "Vorticity", "vorticity"]];
export const linePlot = async (index: number) => {
  const pts = await call<[number, Sample][]>("line", { index });
  if (!pts) return;
  const [q, setQ] = createSignal<keyof Sample>(M().physics.heat && M().view.field === "temperature" ? "temperature" : M().view.field === "pressure" ? "pressure" : "speed");
  const l = () => M().lines[index];
  openDialog(() => {
    const meta = () => QUANT.find(x => x[0] === q())!;
    const data = () => pts.filter(p => p[1].inside && p[1][q()] !== null).map(p => ({ x: toU("len", p[0]), y: toU(meta()[2], p[1][q()] as number) }));
    return <Dialog title={`Along ${l()?.name || `line ${index + 1}`}`} wide help="probes" extra={<><button class="btn" onClick={() => exportLine(index)}>Save as CSV</button></>}>
      <div class="slinehead"><label>Plot <select onChange={e => setQ(e.currentTarget.value as keyof Sample)}><For each={QUANT.filter(x => x[0] !== "temperature" || M().physics.heat)}>{x => <option value={x[0]} selected={x[0] === q()}>{x[1]}</option>}</For></select></label>
        <span class="muted">{pts.length} points from ({num(toU("len", l()?.a[0] ?? 0), 4)}, {num(toU("len", l()?.a[1] ?? 0), 4)}) to ({num(toU("len", l()?.b[0] ?? 0), 4)}, {num(toU("len", l()?.b[1] ?? 0), 4)}) {ulabel("len")}{pts.some(p => !p[1].inside) ? " · points outside the fluid are left out" : ""}</span></div>
      <LineChart cls="big" name={`${l()?.name || "line"} ${meta()[1]}`} legend={false} x={`Distance along the line (${ulabel("len")})`} y={`${meta()[1]}${ulabel(meta()[2]) ? ` (${ulabel(meta()[2])})` : ""}`} series={() => [{ label: meta()[1], data: data() }]} />
    </Dialog>;
  });
};
const csvDialog = () => {
  if (!out()?.has_results) return toast("There are no results yet: solve the model first (Solve › Solve).");
  if (!M().probes.length && !M().lines.length) return toast("Place a probe or draw a plot line first (Results › Probe, Line Plot): the CSV holds their values.");
  openDialog(() => <Dialog title="Save as CSV" help="export">
    <p class="muted">A table in the units shown ({M().units}), ready for any spreadsheet or for Datum Plot.</p>
    <div class="scsv">
      <Show when={M().probes.length}><button class="btn" onClick={() => { closeDialog(); exportProbes(); }}>The {M().probes.length} probe{M().probes.length === 1 ? "" : "s"}</button></Show>
      <For each={M().lines}>{(l, i) => <button class="btn" onClick={() => { closeDialog(); exportLine(i()); }}>Along {l.name || `line ${i() + 1}`}</button>}</For>
    </div>
  </Dialog>);
};

// ---------------------------------------------------------------- verification
const TIERS = ["Exact solutions", "Order of accuracy", "Benchmarks", "Validation against experiment", "Regression"];
const TIER_ABOUT = ["Flows whose answer is known in closed form. The solver must reproduce them.", "As the mesh is refined, the error must fall at the rate the method promises.", "Published high-accuracy results from other researchers.", "Measurements from a laboratory, with their uncertainty.", "This version's results against the previous version's."];
const pct = (c: Check, v: number) => (c.absolute ? num(v, 3) : `${num(100 * v, 3)}%`);
export const VerTable = (p: { r: Report; print?: boolean }) => {
  const [open, setOpen] = createSignal<string | null>(null);
  return <For each={TIERS}>{(t, i) => {
    const list = () => p.r.checks.filter(c => c.tier === i() + 1);
    return <Show when={list().length}>
      <h4 class="vtier">{i() + 1} · {t}<small>{TIER_ABOUT[i()]}</small><span class="grow" /><b classList={{ ok: list().every(c => c.pass), bad: list().some(c => !c.pass) }}>{list().filter(c => c.pass).length} of {list().length} pass</b></h4>
      <table class="tbl vtab"><thead><tr><th>Case</th><th>Quantity</th><th class="n">Reference</th><th class="n">Datum Flow</th><th class="n">Error</th><th class="n">Allowed</th><th /></tr></thead><tbody>
        <For each={list()}>{c => { const key = `${c.case}|${c.quantity}`; return <>
          <tr classList={{ link: !p.print }} onClick={() => !p.print && setOpen(open() === key ? null : key)}><td>{c.case}</td><td>{c.quantity}</td><td class="n">{num(c.reference, 7)}</td><td class="n">{num(c.result, 7)}</td><td class="n">{pct(c, c.error)}</td><td class="n">{pct(c, c.tolerance)}</td><td><b class="chip" classList={{ ok: c.pass, bad: !c.pass }}>{c.pass ? "PASS" : "FAIL"}</b></td></tr>
          <Show when={open() === key || p.print}><tr class="vmore"><td colSpan={7}>
            <span><b>Source:</b> {c.source}{c.confirmed && c.confirmed !== "exact" ? <i class="muted"> · {c.confirmed === "secondary sources" ? "reference value taken from secondary sources, not yet checked against the original paper" : `reference value checked against the ${c.confirmed}`}</i> : ""}</span>
            <Show when={c.note}><span>{c.note}</span></Show>
            <Show when={c.steps.length && !p.print}><span><b>Mesh by mesh</b> (element size → value): <For each={c.steps}>{(s, k) => <>{k() ? " · " : ""}{num(s[0], 3)} → {num(s[1], 7)}</>}</For></span></Show>
          </td></tr></Show>
        </>; }}</For>
      </tbody></table>
    </Show>;
  }}</For>;
};
export const verSummary = (r: Report) => { const pass = r.checks.filter(c => c.pass).length; return { pass, fail: r.checks.length - pass }; };
export const verification = async () => {
  const r = await call<Report>("verification");
  if (!r) return;
  const s = verSummary(r);
  openDialog(() => <Dialog title="Verification" wide help="verification" extra={<button class="btn" onClick={() => { closeDialog(); setReportPages(verificationPages(r)); setBackstage("print"); }}>Print or save as PDF…</button>}>
    <div class="vhead"><b class="chip big" classList={{ ok: s.fail === 0 && s.pass > 0, bad: s.fail > 0 }}>{s.fail ? `${s.fail} FAIL` : s.pass ? "ALL PASS" : "NO CHECKS"}</b>
      <div><b>Datum Flow {r.version}: {s.pass} of {r.checks.length} checks pass</b><span class="muted">Run on {r.date}{r.machine ? ` on ${r.machine}` : ""}{r.seconds ? `, ${num(r.seconds, 3)} s in all` : ""}. These are the cases this version was tested on before release; any FAIL blocks a release. Click a row for its source.</span></div></div>
    <Show when={r.checks.length} fallback={<p class="inote">This record has no checks yet.</p>}><VerTable r={r} /></Show>
    <Show when={r.not_verified.length}><h4 class="vtier">Not verified in this version</h4><ul class="vnot"><For each={r.not_verified}>{n => <li>{n}</li>}</For></ul></Show>
  </Dialog>);
};
/** The verification record as report pages (about 14 rows a page). */
export const verificationPages = (r: Report): (() => JSX.Element)[] => {
  const s = verSummary(r), per = 9, pages: (() => JSX.Element)[] = [];
  for (let i = 0; i < Math.max(1, r.checks.length); i += per) {
    const part = { ...r, checks: r.checks.slice(i, i + per) }, first = i === 0, last = i + per >= r.checks.length;
    pages.push(() => <div class="rpage">
      <Show when={first}><h2>Verification of Datum Flow {r.version}</h2><p>{s.pass} of {r.checks.length} checks pass{s.fail ? `; ${s.fail} FAIL` : ""}. Run on {r.date}{r.machine ? ` on ${r.machine}` : ""}.</p></Show>
      <VerTable r={part} print />
      <Show when={last && r.not_verified.length}><h4 class="vtier">Not verified in this version</h4><ul class="vnot"><For each={r.not_verified}>{n => <li>{n}</li>}</For></ul></Show>
    </div>);
  }
  return pages;
};

// ---------------------------------------------------------------- the PDF report
const report = async () => {
  const o = out();
  if (!o) return;
  const [vals, ver] = await Promise.all([o.has_results ? call<ValueRow[]>("values") : Promise.resolve(undefined), call<Report>("verification")]);
  const m = o.m, f = m.fluid, n = o.numbers, s = summary(), q = mesh()?.quality;
  const meshPng = meshPicture(), viewPng = viewPicture(), resPng = s ? chartPicture(residualSeries(s.history, m.physics.heat), { x: "Iteration", y: "Residual (scaled)", log: true }) : null;
  const fd = field(), range = fd ? fieldRange(fd, m.view.map) : null;
  const vs = ver ? verSummary(ver) : null;
  const pages: (() => JSX.Element)[] = [
    () => <div class="rpage"><h2>1 · Inputs</h2>
      <table class="rtab"><tbody>
        <tr><th>Fluid</th><td>{f.name} at {fmt("temp", f.at)}</td></tr>
        <tr><th>Density</th><td>{fmt("density", f.rho, 5)}</td></tr><tr><th>Viscosity</th><td>{fmt("viscosity", f.mu, 5)} (kinematic {fmt("kinvisc", f.mu / f.rho, 4)})</td></tr>
        <Show when={m.physics.heat}><tr><th>Conductivity</th><td>{fmt("cond", f.k, 5)}</td></tr><tr><th>Specific heat</th><td>{fmt("cp", f.cp, 5)}</td></tr><tr><th>Expansion</th><td>{fmt("expansion", f.beta, 4)}</td></tr></Show>
        <tr><th>Source</th><td>{f.source}</td></tr>
        <tr><th>Physics</th><td>Steady laminar flow in 2D{m.physics.heat ? ", with heat" : ""}{m.physics.buoyancy ? ` and buoyancy (gravity ${fmt("accel", m.physics.gravity[0], 4)}, ${fmt("accel", m.physics.gravity[1], 4)}; reference ${fmt("temp", m.physics.t_ref)})` : ""}; depth {fmt("len", m.physics.depth)}</td></tr>
        <tr><th>Numbers</th><td>Reynolds {n.speed > 0 ? num(n.re, 4) : "—"}{m.physics.buoyancy ? ` · Rayleigh ${num(n.ra, 4)}` : ""} · Prandtl {num(n.pr, 4)} (speed {fmt("speed", n.speed)}, length {fmt("len", n.length)}{n.dt ? `, temperature difference ${fmt("dtemp", n.dt)}` : ""})</td></tr>
      </tbody></table>
      <h3>Boundaries</h3>
      <table class="rtab cols"><thead><tr><th>Name</th><th>Kind</th><th>Flow</th><th>Heat</th><th>Edges</th></tr></thead><tbody><For each={m.boundaries}>{b => <tr><td>{b.name}</td><td>{KIND_NAME[b.kind.type]}</td>
        <td>{b.kind.type === "inlet" ? `${fmt("speed", b.kind.speed)} mean, ${b.kind.profile === "parabolic" ? "fully developed" : "uniform"}` : b.kind.type === "outlet" ? `pressure ${fmt("pressure", b.kind.pressure)}` : b.kind.type === "wall" ? (b.kind.u[0] || b.kind.u[1] ? `moving at ${fmt("speed", b.kind.u[0])}, ${fmt("speed", b.kind.u[1])}` : "fixed, no slip") : "no flow across"}</td>
        <td>{!m.physics.heat ? "—" : b.heat.type === "temperature" ? fmt("temp", b.heat.t) : b.heat.type === "flux" ? fmt("flux", b.heat.q) : "insulated"}</td><td>{b.edges.map(e => o.edges.find(x => x.shape === e.shape && x.edge === e.edge)?.name ?? "").join(", ")}</td></tr>}</For></tbody></table>
      <Show when={m.notes}><p class="rnote">{m.notes}</p></Show></div>,
    () => <div class="rpage"><h2>2 · Mesh</h2>
      <Show when={q} fallback={<p>No mesh has been generated.</p>}>
        <table class="rtab"><tbody><tr><th>Elements</th><td>{q!.elements.toLocaleString()} six-node triangles, {q!.nodes.toLocaleString()} nodes, {q!.unknowns.toLocaleString()} flow unknowns</td></tr>
          <tr><th>Size</th><td>{m.mesh.size ? fmt("len", m.mesh.size) : `automatic (${fmt("len", mesh()!.size, 3)})`}; edges {fmt("len", q!.edges[0], 3)} to {fmt("len", q!.edges[2], 3)}; walls × {num(m.mesh.wall, 2)}, corners × {num(m.mesh.corner, 2)}, wakes × {num(m.mesh.wake, 2)}; {m.mesh.refine.length} local size{m.mesh.refine.length === 1 ? "" : "s"}</td></tr>
          <tr><th>Quality</th><td>Smallest angle {num(q!.min_angle, 3)}°; {num(100 * q!.good, 3)}% of elements have every angle over 30°; neighbouring elements differ in size by at most {num(q!.max_ratio, 3)}</td></tr></tbody></table>
        <Show when={meshPng}><div class="ppic"><img src={meshPng!} alt="" /></div></Show>
      </Show></div>,
    () => <div class="rpage"><h2>3 · Convergence</h2>
      <Show when={s} fallback={<p>The model has not been solved.</p>}>
        <table class="rtab"><tbody><tr><th>Outcome</th><td><b>{s!.converged ? "Converged" : s!.stopped ? "Stopped before it converged" : "Did not converge"}</b>. {s!.message}</td></tr>
          <tr><th>Solve</th><td>{s!.history.length - 1} iterations{s!.stats.steps ? ` in ${s!.stats.steps} continuation steps` : ""}; {s!.stats.unknowns.toLocaleString()} unknowns; {num(s!.stats.seconds, 3)} s; {s!.stats.linear_solver}; tolerance {num(m.solver.tol, 2)}</td></tr>
          <tr><th>Last residuals</th><td>momentum {num(s!.history[s!.history.length - 1].momentum, 2)}, continuity {num(s!.history[s!.history.length - 1].continuity, 2)}{m.physics.heat ? `, energy ${num(s!.history[s!.history.length - 1].energy, 2)}` : ""} (scaled by their largest value)</td></tr></tbody></table>
        <div class="ppic"><img src={resPng!} alt="" /></div>
      </Show></div>,
    () => <div class="rpage"><h2>4 · Results</h2>
      <Show when={o.has_results && viewPng} fallback={<p>There are no results.</p>}>
        <Show when={fd && range}><p>{fd!.name} from {fmt(fd!.kind, range![0], 4)} to {fmt(fd!.kind, range![1], 4)} ({m.view.map} colours){m.view.streamlines ? ", with streamlines" : ""}{m.view.arrows ? " and velocity arrows" : ""}.</p></Show>
        <div class="ppic"><img src={viewPng!} alt="" /></div>
      </Show></div>,
    ...(vals ? Array.from({ length: Math.ceil(vals.length / 22) }, (_, i) => () => <div class="rpage"><h2>5 · Values{i ? " (continued)" : ""}</h2><ValuesTable rows={vals.slice(22 * i, 22 * i + 22)} /></div>) : []),
    () => <div class="rpage"><h2>{vals ? 6 : 5} · Verification of this version</h2>
      <Show when={ver && vs} fallback={<p>The verification record could not be read.</p>}>
        <p>These results were computed with Datum Flow {ver!.version}. Before release that version was tested on cases with known answers: <b>{vs!.pass} of {ver!.checks.length} checks pass{vs!.fail ? `, ${vs!.fail} FAIL` : ""}</b> (run on {ver!.date}).</p>
        <table class="rtab cols"><thead><tr><th>Tier</th><th>Checks</th><th>Pass</th></tr></thead><tbody><For each={TIERS}>{(t, i) => { const l = ver!.checks.filter(c => c.tier === i() + 1); return <Show when={l.length}><tr><td>{t}</td><td>{l.length}</td><td>{l.filter(c => c.pass).length}</td></tr></Show>; }}</For></tbody></table>
        <p>Full report: Help › Verification.</p>
        <p class="rnote">A verified solver does not make every model right: the answer also depends on the mesh, the boundaries chosen and whether the real flow is steady and laminar. Compare with a finer mesh before relying on a number.</p>
      </Show></div>,
  ];
  setReportPages(pages);
  setBackstage("print");
  toast("The report's pages follow the first page in the preview. Print, or Save as PDF.", true);
};

/** Register the panels, dialogs and commands. Called once, inside the app. */
export const initSimUi = () => {
  ui.advice = (s: Summary) => openDialog(() => <Dialog title="The solve did not converge" help="solve" okText="Solver settings" ok={() => { setSel({ kind: "solver" }); setTab("solve"); }}>
    <p>{s.message}</p><Show when={s.advice.length}><h4>What to try</h4><ul class="sadv"><For each={s.advice}>{a => <li>{a}</li>}</For></ul></Show>
    <p class="muted">The view shows the last state the solver reached, so you can see where it struggles.</p></Dialog>);
  ui.refused = (i: Issue) => openDialog(() => <Dialog title="Not ready to solve" help="checks" okText={i.boundary ? "Show it" : "OK"} ok={() => goIssue(i)}>
    <p><b>{i.text}</b></p><p>{i.fix}</p><p class="muted">The Checks tab below lists everything found.</p></Dialog>);
  ui.linePlot = linePlot;
  // The report's pages belong to one visit of File › Print: a plain Print shows the view's page only.
  createEffect(() => { if (backstage() !== "print") setReportPages([]); });
  wire("values", valuesDialog); wire("export_csv", csvDialog); wire("verification", verification); wire("report", report);
  // The bottom panel's tabs take their content from the engine once there is something to show.
  const hasRes = createMemo(() => rows().length > 0 || running()), hasMon = createMemo(() => monNames().length > 0 && (monValues().length > 0 || running())), hasChecks = createMemo(() => !!issues());
  createEffect(() => setPanel("residuals", hasRes() ? () => <ResidualPanel /> : null));
  createEffect(() => setPanel("monitors", hasMon() ? () => <MonitorPanel /> : null));
  createEffect(() => setPanel("checks", hasChecks() ? () => <ChecksPanel /> : null));
};
