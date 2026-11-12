// File › Print: settings on the left, a live page preview on the right. The page (header, the
// view as drawn, a summary of the model, footer) is laid out here at real size and printed with
// the WebView's print dialog, or written straight to a PDF. Other parts add report pages with
// `setReportPages`.
import { For, Show, JSX, createSignal, createMemo, onCleanup } from "solid-js";
import { Portal } from "solid-js/web";
import { createStore } from "solid-js/store";
import { invoke, num } from "./api";
import { out, opts, toast, fmt, KIND_NAME } from "./store";
import { Pick, Check } from "./ui";
import { Help } from "./help";
import { viewPicture } from "./canvas";

/** Paper sizes in millimetres (portrait). */
export const PAPERS: [string, number, number][] = [["A4", 210, 297], ["A3", 297, 420], ["A5", 148, 210], ["Letter", 215.9, 279.4], ["Legal", 215.9, 355.6], ["Tabloid", 279.4, 431.8]];
const MM = 96 / 25.4;
type Slots = [string, string, string];
export type PageSetup = { paper: string; landscape: boolean; margins: [number, number, number, number]; header: Slots; footer: Slots; picture: boolean; summary: boolean; notes: boolean };
export const CODES: [string, string][] = [["&[Model]", "Model name"], ["&[File]", "File name"], ["&[Date]", "Today's date"], ["&[Page]", "Page number"], ["&[Pages]", "Total pages"], ["&[Company]", "Company (Options)"], ["&[Version]", "Datum Flow version"]];
const [ps, setPs] = createStore<PageSetup>({ paper: "A4", landscape: true, margins: [12, 12, 12, 12], header: ["&[Company]", "&[Model]", "&[Date]"], footer: ["Datum Flow &[Version]", "", "Page &[Page] of &[Pages]"], picture: true, summary: true, notes: true });
let seeded = false;

/** Extra pages after the first (the PDF report's pages): each is the body of one page. */
const [reportPages, setReportPagesRaw] = createSignal<(() => JSX.Element)[]>([]);
export const setReportPages = (pages: (() => JSX.Element)[]) => setReportPagesRaw(pages);

const expand = (s: string, page: number, pages: number) => {
  const o = out();
  return s.replaceAll("&[Model]", o?.m.name ?? "").replaceAll("&[File]", o?.path ? o.file : "").replaceAll("&[Date]", new Date().toLocaleDateString(undefined, { day: "2-digit", month: "short", year: "numeric" }))
    .replaceAll("&[Page]", String(page)).replaceAll("&[Pages]", String(pages)).replaceAll("&[Company]", opts().company).replaceAll("&[Version]", o?.version ?? "");
};
const geometry = () => {
  const p = PAPERS.find(x => x[0] === ps.paper) ?? PAPERS[0], pw = ps.landscape ? Math.max(p[1], p[2]) : Math.min(p[1], p[2]), ph = ps.landscape ? Math.min(p[1], p[2]) : Math.max(p[1], p[2]);
  return { pw, ph, w: pw * MM, h: ph * MM };
};

const Summary = () => {
  const o = () => out()!, m = () => o().m, n = () => o().numbers;
  return (
    <div class="psum">
      <table><tbody>
        <tr><th>Fluid</th><td>{m().fluid.name} at {fmt("temp", m().fluid.at)}: density {fmt("density", m().fluid.rho)}, viscosity {fmt("viscosity", m().fluid.mu)}</td></tr>
        <tr><th>Physics</th><td>Steady laminar flow in 2D{m().physics.heat ? ", with heat" : ""}{m().physics.buoyancy ? " and buoyancy" : ""}</td></tr>
        <tr><th>Numbers</th><td>Reynolds {n().speed > 0 ? num(n().re, 3) : "—"}{m().physics.buoyancy ? ` · Rayleigh ${num(n().ra, 3)}` : ""} · Prandtl {num(n().pr, 3)} (speed {fmt("speed", n().speed)}, length {fmt("len", n().length)})</td></tr>
        <For each={m().boundaries}>{b => <tr><th>{b.name}</th><td>{KIND_NAME[b.kind.type]}
          {b.kind.type === "inlet" ? `, mean speed ${fmt("speed", b.kind.speed)}, ${b.kind.profile === "parabolic" ? "fully developed" : "uniform"}` : b.kind.type === "outlet" ? `, pressure ${fmt("pressure", b.kind.pressure)}` : b.kind.type === "wall" && (b.kind.u[0] || b.kind.u[1]) ? `, moving at ${fmt("speed", b.kind.u[0])}, ${fmt("speed", b.kind.u[1])}` : ""}
          {m().physics.heat && b.heat.type === "temperature" ? `, ${fmt("temp", b.heat.t)}` : m().physics.heat && b.heat.type === "flux" ? `, heat flux ${fmt("flux", b.heat.q)}` : ""} · {b.edges.length} edge{b.edges.length === 1 ? "" : "s"}</td></tr>}</For>
        <tr><th>Mesh</th><td>{m().mesh.size ? `Element size ${fmt("len", m().mesh.size)}` : "Automatic element size"}{o().has_mesh ? "" : " (not generated yet)"}</td></tr>
        <tr><th>Results</th><td>{o().has_results ? "Solved" : "Not solved yet"}</td></tr>
      </tbody></table>
      <Show when={ps.notes && m().notes}><p>{m().notes}</p></Show>
    </div>
  );
};

/** Every page, at real size. */
const Pages = (p: { picture: string | null }) => {
  const G = createMemo(geometry);
  const bodies = (): (() => JSX.Element)[] => [
    () => <><Show when={ps.picture && p.picture}><div class="ppic"><img src={p.picture!} alt="" /></div></Show><Show when={ps.summary}><Summary /></Show></>,
    ...reportPages(),
  ];
  return <For each={bodies()}>{(body, i) => (
    <div class="ppage" style={{ width: `${G().w}px`, height: `${G().h}px` }}>
      <div class="pin" style={{ left: `${ps.margins[3] * MM}px`, top: `${ps.margins[0] * MM}px`, width: `${G().w - (ps.margins[1] + ps.margins[3]) * MM}px`, height: `${G().h - (ps.margins[0] + ps.margins[2]) * MM}px` }}>
        <div class="phead"><For each={[0, 1, 2]}>{k => <div class={`ps${k}`}>{expand(ps.header[k], i() + 1, bodies().length)}</div>}</For></div>
        <div class="pbody">{body()}</div>
        <div class="pfoot"><For each={[0, 1, 2]}>{k => <div class={`ps${k}`}>{expand(ps.footer[k], i() + 1, bodies().length)}</div>}</For></div>
      </div>
    </div>
  )}</For>;
};

const [printing, setPrinting] = createSignal<string | null | false>(false);
/** Lays the pages out for the printer; returns how to clean up. */
const startJob = () => {
  if (!out()) { toast("Open a model first."); return null; }
  const G = geometry(), css = document.createElement("style");
  css.textContent = `@page { size: ${G.pw}mm ${G.ph}mm; margin: 0 }`;
  document.head.append(css);
  document.documentElement.classList.add("printing");
  setPrinting(viewPicture());
  const end = () => { document.documentElement.classList.remove("printing"); css.remove(); setPrinting(false); window.removeEventListener("afterprint", end); };
  return { G, end };
};
const settle = async () => {
  await new Promise(r => setTimeout(r, 120));
  await document.fonts?.ready;
  await Promise.all([...document.querySelectorAll<HTMLImageElement>("#printroot img")].map(i => i.decode().catch(() => {})));
  await Promise.race([new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))), new Promise(r => setTimeout(r, 300))]);
};
/** Print…: the print window, for a printer. */
export const printNow = async () => { const job = startJob(); if (!job) return; await settle(); window.addEventListener("afterprint", job.end); window.print(); };
/** Save as PDF: the pages written straight to a file at the paper size (Windows); where that is
 *  not available, the print window opens instead. */
export const savePdf = async () => {
  const job = startJob(); if (!job) return;
  await settle();
  try {
    const path = await invoke<string | null>("save_pdf", { name: out()!.m.name || "Model", wMm: job.G.pw, hMm: job.G.ph, margins: [0, 0, 0, 0] });
    job.end();
    if (path) toast(`Saved ${path}`, true);
  } catch (e) {
    if (String(e).includes("native-pdf-unavailable")) { window.addEventListener("afterprint", job.end); window.print(); return; }
    job.end(); toast(`The PDF could not be saved: ${e}`);
  }
};
/** The pages for the printer, outside the app (shown only while printing). */
export const PrintRoot = () => <Show when={printing() !== false}><Portal mount={document.getElementById("printroot")!}><Pages picture={printing() as string | null} /></Portal></Show>;

const SlotsEdit = (p: { k: "header" | "footer"; label: string }) => (
  <div class="pslots"><b>{p.label}</b>
    <For each={["Left", "Center", "Right"]}>{(l, i) => <label><span>{l}</span><input value={ps[p.k][i()]} onInput={e => setPs(p.k, i(), e.currentTarget.value)} /></label>}</For>
  </div>
);

export const PrintPage = () => {
  if (!seeded) { seeded = true; setPs({ paper: opts().paper || "A4", landscape: opts().landscape !== false }); }
  const [zoom, setZoom] = createSignal(0.62);
  const picture = createMemo(() => viewPicture());
  const G = createMemo(geometry);
  onCleanup(() => {});
  return (
    <div class="printpage">
      <div class="pset">
        <h1>Print<Help k="print" /></h1>
        <div class="pbtns"><button class="btn pri big" onClick={printNow}>Print…</button><button class="btn big" style={{ "margin-top": "8px" }} onClick={savePdf}>Save as PDF</button></div>
        <h4>Paper</h4>
        <div class="prow"><label><span>Size</span><Pick value={ps.paper} set={v => setPs("paper", v)} options={PAPERS.map(p => [p[0], `${p[0]} (${p[1]} × ${p[2]} mm)`] as [string, string])} /></label>
          <label><span>Orientation</span><Pick value={ps.landscape ? "1" : "0"} set={v => setPs("landscape", v === "1")} options={[["1", "Landscape"], ["0", "Portrait"]]} /></label></div>
        <div class="prow"><For each={["Top", "Right", "Bottom", "Left"]}>{(l, i) => <label><span>{l} (mm)</span><input type="number" min="0" max="60" value={ps.margins[i()]} onChange={e => setPs("margins", i(), Math.max(0, Math.min(60, +e.currentTarget.value || 0)))} /></label>}</For></div>
        <h4>On the page</h4>
        <div class="ptab"><Check label="The view as it is drawn now" value={ps.picture} set={v => setPs("picture", v)} /><Check label="A summary of the model" value={ps.summary} set={v => setPs("summary", v)} /><Check label="The model's notes" value={ps.notes} set={v => setPs("notes", v)} /></div>
        <h4>Header and footer</h4>
        <SlotsEdit k="header" label="Header" /><SlotsEdit k="footer" label="Footer" />
        <p class="pcodes muted">Codes: <For each={CODES}>{([c, d]) => <code title={d} onClick={() => navigator.clipboard?.writeText(c).then(() => toast(`Copied ${c}`, true)).catch(() => {})}>{c}</code>}</For></p>
      </div>
      <div class="pprev">
        <div class="pzoom"><span class="muted">{reportPages().length + 1} page{reportPages().length ? "s" : ""} · {ps.paper} {ps.landscape ? "landscape" : "portrait"}</span><button class="btn sm" onClick={() => setZoom(Math.max(0.2, zoom() - 0.1))}>−</button><span class="muted">{Math.round(zoom() * 100)}%</span><button class="btn sm" onClick={() => setZoom(Math.min(2, zoom() + 0.1))}>+</button></div>
        <Show when={out()} fallback={<p class="muted">Open a model first.</p>}>
          <div class="pscale" style={{ width: `${G().w * zoom()}px`, height: `${(G().h + 22) * (reportPages().length + 1) * zoom()}px` }}><div style={{ transform: `scale(${zoom()})`, "transform-origin": "0 0", display: "flex", "flex-direction": "column", gap: "22px" }}><Pages picture={picture()} /></div></div>
        </Show>
      </div>
    </div>
  );
};
