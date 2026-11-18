// File (backstage): home with templates, examples and recent models, export, print, options,
// roadmap and updates.
import { For, Show, createSignal, createResource, onMount, JSX } from "solid-js";
import { PAPERS, PrintPage } from "./print";
import { RoadmapPage } from "./roadmap";
import { Options } from "./api";
import { backstage, setBackstage, call, out, opts, setOpts, toast, setThemeRaw, applyTheme, setOut, setSnapOn, setShowGrid } from "./store";
import { C, cmd, GH } from "./cmds";
import * as H from "./help";
import { Icon, Text, Pick, Check } from "./ui";
import { Help } from "./help";
import { listen, invoke, Out } from "./api";

type Info = { current: string; latest: string | null; beta: boolean; available: boolean; notes: string; published: string; url: string | null; sums: string | null; size: number; page: string };
export const [update, setUpdate] = createSignal<Info | null>(null);
export const [checking, setChecking] = createSignal(false), [checkedAt, setCheckedAt] = createSignal("");
export const checkUpdate = async (quiet: boolean) => {
  setChecking(true);
  try { setUpdate(await invoke<Info>("check_update")); setCheckedAt(new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })); if (!quiet) toast(update()?.available ? `Version ${update()!.latest} is available.` : "You have the latest version.", true); }
  catch (e) { if (!quiet) toast(`Could not check for updates: ${e}`); }
  finally { setChecking(false); }
};
/** Links to the project on GitHub: code, releases (change log) and issues (report a problem). */
export const GitHubLinks = () => (
  <div class="ghlinks">
    <For each={[["", "GitHub", "The project and its code"], ["/releases", "Releases", "Every version and its change log"], ["/issues/new", "Report an issue", "Tell us about a bug or an idea"]]}>{([path, l, t]) =>
      <button class="btn" title={t} onClick={() => call("open_link", { url: GH + path })}><svg class="ic"><use href={path === "/issues/new" ? "#i-bug" : path ? "#i-tag" : "#i-github"} /></svg>{l}</button>}</For>
  </div>
);
/** Release notes: headings, bullets and bold, enough for the changelog. */
const md = (s: string) => s.split("\n").map(l => {
  const e = l.replace(/[&<>]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!).replace(/\*\*(.+?)\*\*/g, "<b>$1</b>").replace(/`(.+?)`/g, "<code>$1</code>");
  return l.startsWith("## ") ? `<h3>${e.slice(3)}</h3>` : l.startsWith("- ") ? `<li>${e.slice(2)}</li>` : e.trim() ? `<p>${e}</p>` : "";
}).join("");

/** A small drawing of a template: the region, its body and the way the flow goes. */
const Mini = (p: { id: string }) => {
  const a = "var(--blue)", f = "var(--pale)", s = { fill: f, stroke: "var(--text2)", "stroke-width": "1.4" };
  const arrow = (x: number, y: number, l = 14) => <path d={`M${x} ${y}h${l}m-4 -3l4 3l-4 3`} fill="none" stroke={a} stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />;
  return (
    <svg viewBox="0 0 120 70" preserveAspectRatio="xMidYMid meet">
      {p.id === "channel" ? <><rect x="10" y="24" width="100" height="22" {...s} />{arrow(16, 30)}{arrow(16, 35, 20)}{arrow(16, 40)}</>
        : p.id.startsWith("cavity") ? <><rect x="34" y="10" width="52" height="52" {...s} /><path d="M40 10h40m-5 -3l5 3l-5 3" fill="none" stroke={a} stroke-width="1.6" stroke-linecap="round" /><ellipse cx="60" cy="38" rx="15" ry="13" fill="none" stroke={a} stroke-width="1.2" /></>
        : p.id === "step" ? <><path d="M10 14h100v42H40V34H10z" {...s} />{arrow(14, 24)}<path d="M44 48a7 5 0 1 0 14 0a7 5 0 1 0 -14 0" fill="none" stroke={a} stroke-width="1.2" /></>
        : p.id === "cylinder" ? <><rect x="8" y="20" width="104" height="30" {...s} /><circle cx="32" cy="34.5" r="6" fill="var(--bg2)" stroke="var(--text2)" stroke-width="1.4" />{arrow(12, 27, 10)}{arrow(12, 43, 10)}<path d="M40 32c14 -5 30 4 66 1M40 37c14 5 30 -4 66 -1" fill="none" stroke={a} stroke-width="1.1" /></>
        : p.id.startsWith("heated") ? <><rect x="34" y="10" width="52" height="52" {...s} /><path d="M34 10v52" stroke="#C0392B" stroke-width="3.5" /><path d="M86 10v52" stroke={a} stroke-width="3.5" /><path d="M44 50V22h32v28" fill="none" stroke="var(--text2)" stroke-width="1.1" stroke-dasharray="3 3" /></>
        : p.id === "heatsink" ? <><path d="M8 20h104v30H8z" {...s} /><path d="M30 20h56v8H30zM30 42h56v8H30z" fill="#F3B9B4" stroke="#C0392B" stroke-width="1.2" />{arrow(12, 35, 12)}{arrow(90, 35, 12)}</>
        : <><rect x="24" y="14" width="72" height="42" fill="none" stroke="var(--line3)" stroke-width="1.4" stroke-dasharray="4 4" /><path d="M60 27v16M52 35h16" stroke={a} stroke-width="1.8" stroke-linecap="round" /></>}
    </svg>
  );
};

const Home = () => {
  const [recent] = createResource(() => call<{ path: string; name: string; modified: string }[]>("recent"));
  const ex = () => (out()?.examples ?? []).filter(e => e.id !== "empty");
  return (
    <div class="home">
      <div>
        <h1>Good {new Date().getHours() < 12 ? "morning" : new Date().getHours() < 18 ? "afternoon" : "evening"}</h1>
        <h2>New</h2>
        <div class="tiles">
          <button class="tile" onClick={() => C.newModel("empty")}><div class="pic"><Mini id="empty" /></div><b>Blank model</b><span>An empty drawing: start with a rectangle, a circle or a polygon.</span></button>
          <For each={out()?.templates ?? []}>{t => <button class="tile" onClick={() => C.template(t.id)}><div class="pic"><Mini id={t.id} /><small>template</small></div><b>{t.title}</b><span>{t.about}</span></button>}</For>
          <button class="tile" onClick={() => setBackstage("roadmap")}><div class="pic"><svg class="ic"><use href="#i-roadmap" /></svg></div><b>Roadmap</b><span>Where Datum Flow is heading, release by release.</span></button>
        </div>
        <h2 id="examples">Examples</h2>
        <table class="recent examples"><tbody>
          <For each={ex()}>{e => <tr class="f" onClick={() => C.newModel(e.id)}><td class="epic"><Mini id={e.id} /></td><td><b>{e.title}</b><div class="p">{e.about}</div></td></tr>}</For>
        </tbody></table>
        <h2>Recent</h2>
        <table class="recent"><tbody>
          <For each={recent() ?? []} fallback={<tr><td class="muted">Models you open or save appear here.</td></tr>}>{r => <tr class="f" onClick={() => C.open(r.path)}><td><b>{r.name}</b><div class="p">{r.path}</div></td><td class="p">{r.modified}</td></tr>}</For>
        </tbody></table>
      </div>
      <UpdateCard />
    </div>
  );
};

const UpdateCard = () => {
  const [pct, setPct] = createSignal<number | null>(null);
  onMount(() => { listen("update-progress", (p: number) => setPct(p)); if (!update()) checkUpdate(true); });
  const install = async () => { const u = update(); if (!u?.url) return; setPct(0); await call("install_update", { url: u.url, sums: u.sums }); setPct(null); };
  return (
    <div class="card">
      <div class="brand"><img src="logo.svg" alt="" /><div><b>Datum Flow</b><span class="muted">Version {out()?.version}{opts().beta ? " · beta channel" : ""}</span></div></div>
      <Show when={update()} fallback={<><span class="pill">{checking() ? "Checking for updates…" : "Not checked yet"}</span><button class="btn wide" disabled={checking()} onClick={() => checkUpdate(false)}>{checking() ? <><i class="spin" /> Checking…</> : "Check for updates"}</button><GitHubLinks /></>}>{u => <>
        <Show when={u().available} fallback={<span class="pill ok">You have the latest version</span>}><span class="pill warn">Version {u().latest}{u().beta ? " (beta)" : ""} is ready</span></Show>
        <Show when={u().available && u().url}><button class="btn pri wide" disabled={pct() !== null} onClick={install}>{pct() === null ? `Update now (${Math.max(1, Math.round(u().size / 1048576))} MB)` : `Downloading… ${pct()}%`}</button></Show>
        <Show when={pct() !== null}><div class="prog"><i style={{ width: `${pct()}%` }} /></div></Show>
        <button class="btn wide" disabled={checking()} onClick={() => checkUpdate(false)}>{checking() ? <><i class="spin" /> Checking…</> : "Check again"}</button>
        <Show when={checkedAt() && !checking()}><span class="muted small">Checked at {checkedAt()}</span></Show>
        <GitHubLinks />
        <div class="notes"><h4>What's new{u().latest ? ` in ${u().latest}` : ""}</h4><div innerHTML={md(u().notes || "Release notes will appear here.")} /></div>
      </>}</Show>
    </div>
  );
};

const Export = () => (
  <div><h1>Export<Help k="export" /></h1>
    <div class="tiles">
      <button class="tile" onClick={() => { setBackstage(null); C.picture(); }}><div class="pic"><svg class="ic"><use href="#i-image" /></svg><small>.png</small></div><b>Picture of the view</b><span>The view as it is drawn now, as a PNG.</span></button>
      <button class="tile" onClick={() => setBackstage("print")}><div class="pic"><svg class="ic"><use href="#i-print" /></svg><small>.pdf</small></div><b>Print or save as PDF</b><span>The view and a summary of the model on paper.</span></button>
      <button class="tile" onClick={() => cmd("report")}><div class="pic"><svg class="ic"><use href="#i-report" /></svg><small>.pdf</small></div><b>PDF report</b><span>Inputs, mesh, convergence and results, with the verification status of this version.</span></button>
      <button class="tile" onClick={() => cmd("export_vtu")}><div class="pic"><svg class="ic"><use href="#i-export" /></svg><small>.vtu</small></div><b>VTU</b><span>The mesh and the fields, for free 3D viewers.</span></button>
      <button class="tile" onClick={() => cmd("export_csv")}><div class="pic"><svg class="ic"><use href="#i-csv" /></svg><small>.csv</small></div><b>CSV</b><span>Probes and line plots as a table.</span></button>
    </div>
  </div>
);

/** One option: name and a one-line explanation on the left, the control on the right. */
const Opt = (p: { l: string; hint?: string; children: JSX.Element }) => <div class="opt"><div><b>{p.l}</b>{p.hint && <small>{p.hint}</small>}</div><div class="ctl">{p.children}</div></div>;
const Sec = (p: { t: string; about: string; children: JSX.Element }) => <section class="osec"><h3>{p.t}</h3><p class="muted">{p.about}</p>{p.children}</section>;

const OptionsPage = () => {
  const [o, setO] = createSignal<Options>({ ...opts() });
  const save = async () => { const r = await call<Out>("set_options", { opts: o() }); if (r) { setOpts(o()); setOut(r); setSnapOn(o().snap); setShowGrid(o().grid); toast("Options saved.", true); } };
  const put = <K extends keyof Options>(k: K, v: Options[K]) => setO({ ...o(), [k]: v });
  return (
    <div class="options"><h1>Options<Help k="options" /></h1>
      <p class="muted">Settings for Datum Flow on this computer. Defaults for new models apply when a model is created; an open model keeps its own.</p>
      <Sec t="General" about="How the window looks.">
        <Opt l="Theme" hint="Auto follows the Windows light or dark setting."><Pick value={o().theme || "auto"} set={v => { put("theme", v); setThemeRaw(v); applyTheme(); }} options={[["auto", "Auto (follow Windows)"], ["light", "Light"], ["dark", "Dark"]]} /></Opt>
        <Opt l="Snap to the grid" hint="Points you click or drag in the drawing land on grid points. Hold Alt to place one freely."><Check label="Snap while drawing" value={o().snap} set={v => put("snap", v)} /></Opt>
        <Opt l="Grid" hint="Draw the grid lines behind the model."><Check label="Show the grid" value={o().grid} set={v => put("grid", v)} /></Opt>
      </Sec>
      <Sec t="Start-up and saving" about="What happens when Datum Flow starts, and how your work is protected.">
        <Opt l="When Datum Flow starts" hint="Open the File page, or reopen the model you used last."><Pick value={o().openLast ? "last" : "home"} set={v => put("openLast", v === "last")} options={[["home", "Show the File page"], ["last", "Reopen the last model"]]} /></Opt>
        <Opt l="Keep results in the file" hint="Save the mesh and the solution with the model, so it opens solved. The file grows (about 1 MB for 25 000 unknowns)."><Pick value={o().keepResults ? "yes" : "no"} set={v => put("keepResults", v === "yes")} options={[["yes", "Yes: open models solved"], ["no", "No: keep files small"]]} /></Opt>
        <Opt l="Recovery copy every" hint="A copy is kept while you work, so a crash or power cut loses little. 0 turns it off."><span class="inline"><input type="number" min="0" style={{ width: "70px" }} value={o().autosave} onChange={e => put("autosave", Math.max(0, Math.round(+e.currentTarget.value || 0)))} /> minutes</span></Opt>
      </Sec>
      <Sec t="New models" about="Defaults given to every new model. Each model keeps its own settings afterwards.">
        <Opt l="Units" hint="SI: metres, m/s, pascals, °C. US: feet, ft/s, psi, °F. A model can be switched at any time."><Pick value={o().units} set={v => put("units", v as "SI" | "US")} options={[["SI", "SI"], ["US", "US"]]} /></Opt>
        <Opt l="Colour map" hint="The colours of results. All three read evenly from low to high, also in grey print."><Pick value={o().map} set={v => put("map", v)} options={[["viridis", "Viridis"], ["inferno", "Inferno"], ["cividis", "Cividis"]]} /></Opt>
      </Sec>
      <Sec t="Printing and branding" about="Your company name goes in the header of printed pages and reports.">
        <Opt l="Paper" hint="Default paper for new printouts."><span class="inline"><Pick value={o().paper || "A4"} set={v => put("paper", v)} options={PAPERS.map(p => p[0])} /><Pick value={o().landscape === false ? "0" : "1"} set={v => put("landscape", v === "1")} options={[["1", "Landscape"], ["0", "Portrait"]]} /></span></Opt>
        <Opt l="Company" hint="Shown on printouts and reports (the code &[Company])."><Text value={o().company} set={v => put("company", v)} /></Opt>
      </Sec>
      <Sec t="Updates" about="Datum Flow checks GitHub for new versions when it starts. Updates install from File › Updates.">
        <Opt l="Beta versions" hint="Get new features early. They may have bugs."><Check label="Get beta versions" value={o().beta} set={v => put("beta", v)} /></Opt>
      </Sec>
      <p class="osave"><button class="btn pri" onClick={save}>Save options</button><span class="muted">Datum Flow keeps everything on this computer. Its only connection is the update check to GitHub.</span></p>
    </div>
  );
};

export const Backstage = () => (
  <div class="bs">
    <aside>
      <button class="back" onClick={() => out() && setBackstage(null)} title="Back">←</button>
      <For each={[["home", "home", "Home"], ["new", "new", "New"], ["open", "open", "Open"], ["save", "save", "Save"], ["saveas", "save", "Save As"], ["export", "export", "Export"], ["print", "print", "Print"], ["roadmap", "roadmap", "Roadmap"]]}>{([k, i, l]) =>
        <button classList={{ on: backstage() === k }} onClick={() => {
          if (k === "open") C.open(); else if (k === "save") C.save(); else if (k === "saveas") C.save(true);
          else if (k === "new") C.newModel("empty");
          else setBackstage(k as "home" | "export" | "print" | "roadmap");
        }}><Icon n={i} size={22} />{l}</button>}</For>
      <span class="grow" />
      <button classList={{ on: backstage() === "options" }} onClick={() => setBackstage("options")}><Icon n="opts" size={22} />Options</button>
      <button classList={{ on: backstage() === "about" }} onClick={() => setBackstage("about")}><Icon n="upd" size={22} />Updates</button>
    </aside>
    <section class="bsmain">
      <Show when={backstage() === "home"}><Home /></Show>
      <Show when={backstage() === "export"}><Export /></Show>
      <Show when={backstage() === "options"}><OptionsPage /></Show>
      <Show when={backstage() === "print"}><PrintPage /></Show>
      <Show when={backstage() === "roadmap"}><RoadmapPage /></Show>
      <Show when={backstage() === "about"}><div class="about"><h1>Updates<Help k="update" /></h1><UpdateCard /><p><button class="btn" onClick={H.shortcuts}>Keyboard shortcuts</button> <button class="btn" onClick={() => C.site()}>Datum Flow on the Datum site</button></p></div></Show>
    </section>
  </div>
);
