// Datum Flow window: layout (ribbon, sidebar, model tree or Setup, 2D view or a page, inspector,
// bottom panel, status bar),
// keyboard shortcuts, start-up and recovery.
import { render } from "solid-js/web";
import { For, Show, onMount } from "solid-js";
import "@fontsource/ibm-plex-sans/latin-400.css";
import "@fontsource/ibm-plex-sans/latin-500.css";
import "@fontsource/ibm-plex-sans/latin-600.css";
import "@fontsource/ibm-plex-sans/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "@fontsource/ibm-plex-mono/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-600.css";
import "./styles.css";
import { invoke, listen, Out, Options, num } from "./api";
import { out, setOut, backstage, setBackstage, setOpts, opts, dialogs, call, openDialog, toast, setThemeRaw, applyTheme, treeOpen, inspOpen, setupOpen, view, PAGES, setSnapOn, setShowGrid, tool, setSel, sel, setTab, toU, ulabel, loadOut, M } from "./store";
import { C } from "./cmds";
import { TitleBar, Ribbon, palette } from "./ribbon";
import { Canvas, xf, cursor, viewSize } from "./canvas";
import { Tree } from "./tree";
import { Inspector } from "./inspector";
import { Bottom } from "./bottom";
import { Backstage, update, checkUpdate } from "./backstage";
import { PrintRoot } from "./print";
import { Menu, Toast, Dialog, askYes } from "./ui";
import * as H from "./help";
import { initSim } from "./sim";
import { initDraw } from "./draw";
import { initSimUi, SimOverlay, HoverValues } from "./simui";
import { Side, SetupPanel, ViewPage } from "./side";

const TOOL_NAME: Record<string, string> = { select: "Select", pan: "Pan", rect: "Rectangle", circle: "Circle", poly: "Polygon", pick: "Pick edges" };

const Status = () => {
  const n = () => out()?.numbers;
  // The zoom as a share of the size that fits the model in the view.
  const pct = () => { const b = out()?.bounds, s = viewSize(); if (!b || !(b[2] > b[0])) return 100; const k0 = Math.min((s.w - 90) / (b[2] - b[0]), (s.h - 90) / (b[3] - b[1])); return Math.round(100 * xf().k / k0); };
  return (
    <footer class="status">
      <span classList={{ late: !!out()?.geometry_problem && !!out()?.m.shapes.length }}>{out()?.geometry_problem && out()!.m.shapes.length ? "Check the drawing" : "Ready"}</span>
      <Show when={out()}>
        <button class="mode" title="Switch the units shown: SI or US" onClick={() => C.units(M().units === "SI" ? "US" : "SI")}>Units: {M().units}</button>
        <button class="mode" title="The numbers that say what kind of flow this is" classList={{ late: !!out()!.turbulent }} onClick={() => { setSel({ kind: "model" }); setTab("check"); }}>
          Re {n()!.speed > 0 ? num(n()!.re, 3) : "—"}{M().physics.buoyancy ? ` · Ra ${num(n()!.ra, 3)}` : ""} · Pr {num(n()!.pr, 3)}</button>
        <span class="muted">{M().fluid.name}{M().physics.heat ? (M().physics.buoyancy ? " · heat, buoyancy" : " · heat") : ""}</span>
        <Show when={tool() !== "select"}><span class="muted">Tool: {TOOL_NAME[tool()]}</span></Show>
      </Show>
      <span class="grow" />
      <Show when={out()?.has_results}><HoverValues /></Show>
      <Show when={cursor()}>{c => <span class="muted coords">x {num(toU("len", c()[0]), 4)} · y {num(toU("len", c()[1]), 4)} {ulabel("len")}</span>}</Show>
      <button onClick={C.zoomOut} title="Zoom Out">−</button><button class="zpct" title="Fit the model in the view (F)" onClick={C.fit}>{pct()}%</button><button onClick={C.zoomIn} title="Zoom In">+</button>
    </footer>
  );
};

/** Leaving with unsaved changes: save first, leave anyway, or stay. */
const confirmClose = () => openDialog(() => (
  <Dialog title="Save your changes?" okText="Save" ok={async () => { const o = await loadOut("save", { path: null, asNew: false }); if (!o) return false; await invoke("quit"); }}
    extra={<button class="btn" onClick={() => invoke("quit")}>Don't save</button>}>
    <p>“{out()?.m.name}” has changes that are not saved.</p>
  </Dialog>
));

/** Anything picked in the tree or the view shows its properties in the Setup panel's place, until
 *  Back to Setup. */
const picked = () => sel().kind !== "model";

const App = () => {
  initSim(); initDraw(); initSimUi();
  onMount(async () => {
    try {
      const o = await invoke<Options>("options"); setOpts(o); setThemeRaw(o.theme || "auto"); setSnapOn(o.snap); setShowGrid(o.grid); applyTheme();
      setOut(await invoke<Out>("state"));
      const start = await invoke<string | null>("startup_file");
      if (start) await loadOut("open", { path: start }); else if (!opts().openLast) setBackstage("home");
      const rec = await invoke<{ name: string; saved: string } | null>("recovery");
      if (rec) openDialog(() => (
        <Dialog title="Recover unsaved work?" okText="Recover" cancel="Discard" ok={async () => { const r = await call<Out>("recover", { keep: true }); if (r) { setOut(r); setBackstage(null); } }}>
          <p>Datum Flow closed before “{rec.name}” was saved. A copy from {rec.saved} was kept.</p>
        </Dialog>
      ));
      checkUpdate(true);
      setInterval(() => { if (opts().autosave > 0) invoke("autosave").catch(() => {}); }, Math.max(1, opts().autosave) * 60000);
      listen("close-requested", confirmClose);
      // No browser menu (Back, Reload…) outside text fields: our own menus open where they apply.
      document.addEventListener("contextmenu", e => { if (!(e.target as HTMLElement).closest?.("input, textarea, [contenteditable]")) e.preventDefault(); });
    } catch (e) { toast(String(e)); }
  });
  const keys = (e: KeyboardEvent) => {
    const ctrl = e.ctrlKey || e.metaKey, k = e.key.toLowerCase(), typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
    if (dialogs().length) return;
    if ((ctrl && k === "k") || (e.altKey && k === "q")) { e.preventDefault(); palette(); }
    else if (ctrl && k === "s") { e.preventDefault(); C.save(e.shiftKey); }
    else if (ctrl && k === "o") { e.preventDefault(); C.open(); }
    else if (ctrl && k === "n") { e.preventDefault(); setBackstage("home"); }
    else if (ctrl && k === "p") { e.preventDefault(); C.print(); }
    else if (ctrl && k === "z" && !typing) { e.preventDefault(); C.undo(); }
    else if (ctrl && k === "y" && !typing) { e.preventDefault(); C.redo(); }
    else if (e.key === "F1") { e.preventDefault(); H.shortcuts(); }
    else if (e.key === "Escape" && backstage() && out()) setBackstage(null);
  };
  return (
    <div class="app" ref={() => document.addEventListener("keydown", keys)}>
      <TitleBar update={update()?.available ? update()!.latest : null} />
      <Ribbon />
      <main class="work">
        <Show when={out()}>
          <Side />
          <Show when={treeOpen()}><Tree /></Show>
          {/* The 2D view stays mounted under a page, so it keeps its size, zoom and layers. */}
          <div class="stage">
            <div class="centre"><div class="viewwrap"><Canvas /><SimOverlay /></div><Bottom /></div>
            {/* Setup sits where the properties are while it is open, as in Datum Plan. */}
            <Show when={setupOpen() && !picked()} fallback={<Show when={inspOpen() || setupOpen()}><Inspector /></Show>}><SetupPanel /></Show>
            <Show when={PAGES.includes(view())}><ViewPage /></Show>
          </div>
        </Show>
      </main>
      <Status />
      <Show when={backstage()}><Backstage /></Show>
      <For each={dialogs()}>{d => d()}</For>
      <Menu /><Toast /><PrintRoot />
    </div>
  );
};

render(() => <App />, document.getElementById("app")!);
export { askYes };
