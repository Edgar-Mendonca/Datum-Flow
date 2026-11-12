// Title bar, quick access toolbar, Tell Me and the ribbon.
import { For, Show, JSX, createSignal, createMemo, onMount } from "solid-js";
import { act, out, tab, setTab, setBackstage, openMenu, Item, openDialog, closeDialog, theme, tool, asHole, M, sel, showGrid, setShowGrid, showNames, setShowNames, snapOn, setSnapOn, treeOpen, setTreeOpen, inspOpen, setInspOpen, bottomOpen, setBottomOpen, Tab, opts, saveOpts } from "./store";
import { C, ALL_CMDS, cmd } from "./cmds";
import { Icon } from "./ui";
import { Help } from "./help";
import * as H from "./help";

/** A ribbon button: big (icon above the label) or small; `dd` opens a menu; both make a split button. */
const B = (p: { i: string; l: string; run?: (e: MouseEvent) => void; big?: boolean; on?: boolean; off?: boolean; dd?: (e: MouseEvent) => void; title?: string }) => p.big && p.run && p.dd ? (
  <div class="rbw big splitbig" classList={{ on: p.on }}>
    <button class="rb top" title={p.title ?? p.l} disabled={p.off} onClick={e => p.run!(e)}><Icon n={p.i} /></button>
    <button class="rb bot" title={`${p.l} options`} onClick={e => p.dd!(e)}><span>{p.l}</span><b class="car">▾</b></button>
  </div>
) : (
  <div class="rbw" classList={{ big: p.big, sp: !p.big && !!p.run && !!p.dd }}>
    <button class="rb" classList={{ big: p.big, on: p.on }} disabled={p.off} title={p.title ?? p.l} onClick={e => (p.run ?? p.dd)?.(e)}><Icon n={p.i} />{p.big ? <span>{p.l}{p.dd && !p.run ? " ▾" : ""}</span> : <span>{p.l}</span>}{!p.big && p.dd && !p.run && <b class="car">▾</b>}</button>
    {p.run && p.dd && <button class="rb sarr" title={`${p.l} options`} onClick={e => p.dd!(e)}>▾</button>}
  </div>
);
const G = (p: { l: string; h?: string; children: JSX.Element }) => {
  let g!: HTMLDivElement;
  // Folded (a narrow window): one button with the group's name opens its commands, named, below it.
  const open = (e: MouseEvent) => {
    const was = g.classList.contains("open");
    document.querySelectorAll(".ribbon .grp.open").forEach(x => x.classList.remove("open"));
    if (was) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect(), items = g.querySelector<HTMLElement>(".items")!, btn = e.currentTarget;
    g.classList.add("open");
    items.style.left = `${Math.max(4, Math.min(r.left, innerWidth - items.offsetWidth - 8))}px`;
    items.style.top = `${r.bottom + 2}px`;
    const close = (ev: MouseEvent) => { if (!items.contains(ev.target as Node) && ev.target !== btn) { g.classList.remove("open"); removeEventListener("mousedown", close, true); } };
    addEventListener("mousedown", close, true);
  };
  const ran = (e: MouseEvent) => { if (g.classList.contains("open") && (e.target as Element).closest("button")) setTimeout(() => g.classList.remove("open"), 0); };
  return <div class="grp" ref={g}><div class="items" onClick={ran}>{p.children}</div><button class="gfold" title={p.l} onClick={open}><span>{p.l}</span> ▾</button><div class="cap">{p.l}{p.h && <Help k={p.h} />}</div></div>;
};
const S = (p: { children: JSX.Element }) => <div class="stack">{p.children}</div>;
const Chk = (p: { l: string; v: boolean; set: (v: boolean) => void; title?: string }) => <label class="chk" title={p.title}><input type="checkbox" checked={p.v} onChange={e => p.set(e.currentTarget.checked)} />{p.l}</label>;

/** Tell Me / command search (Ctrl+K, Alt+Q). */
export const palette = () => openDialog(() => {
  const [q, setQ] = createSignal(""), [k, setK] = createSignal(0);
  const list = createMemo(() => { const s = q().toLowerCase(); return ALL_CMDS.filter(c => c[0].toLowerCase().includes(s)).slice(0, 14); });
  const go = (i: number) => { const c = list()[i]; if (c) { closeDialog(); setTimeout(() => c[1]()); } };
  let input!: HTMLInputElement;
  onMount(() => input.focus());
  return (
    <div class="ov high" onMouseDown={e => e.target === e.currentTarget && closeDialog()}>
      <div class="pal">
        <input ref={input} placeholder="Tell me what you want to do" value={q()} onInput={e => { setQ(e.currentTarget.value); setK(0); }}
          onKeyDown={e => { if (e.key === "ArrowDown") { e.preventDefault(); setK(Math.min(list().length - 1, k() + 1)); } else if (e.key === "ArrowUp") { e.preventDefault(); setK(Math.max(0, k() - 1)); } else if (e.key === "Enter") { e.preventDefault(); go(k()); } else if (e.key === "Escape") closeDialog(); }} />
        <div class="pl"><For each={list()}>{(c, i) => <button classList={{ on: i() === k() }} onMouseEnter={() => setK(i())} onClick={() => go(i())}><span>{c[0]}</span>{c[2] && <kbd>{c[2]}</kbd>}</button>}</For></div>
      </div>
    </div>
  );
});

const TABS: [Tab, string][] = [["home", "Home"], ["geometry", "Geometry"], ["physics", "Physics"], ["mesh", "Mesh"], ["solve", "Solve"], ["results", "Results"], ["check", "Check"], ["view", "View"], ["help", "Help"]];
export const TitleBar = (p: { update: string | null }) => (
  <header class="top">
    <div class="bar">
      <div class="qat">
        <img src="logo.svg" alt="" class="logo" />
        <button title="Save (Ctrl+S)" onClick={() => C.save()}><Icon n="save" /></button>
        <button title="Undo (Ctrl+Z)" disabled={!out()?.can_undo} onClick={C.undo}><Icon n="undo" /></button>
        <button title="Redo (Ctrl+Y)" disabled={!out()?.can_redo} onClick={C.redo}><Icon n="redo" /></button>
        <button title="Print (Ctrl+P)" onClick={C.print}><Icon n="print" /></button>
      </div>
      <div class="title">{out() ? `${out()!.path ? out()!.file : out()!.m.name}${out()!.dirty ? " •" : ""} — Datum Flow` : "Datum Flow"}</div>
      <button class="search" onClick={palette} title="Tell me what you want to do (Alt+Q)"><Icon n="search" /><span>Tell me what you want to do</span><kbd>Ctrl+K</kbd></button>
      <Show when={p.update}><button class="upd" onClick={() => setBackstage("about")}>Update {p.update} available</button></Show>
      <button class="theme" title="Theme: Auto follows Windows" onClick={C.theme}>Theme · {theme()}</button>
    </div>
    <nav class="tabs">
      <button class="file" onClick={() => setBackstage("home")}>File</button>
      <For each={TABS}>{([k, l]) => <button classList={{ on: tab() === k }} onClick={() => setTab(k)}>{l}</button>}</For>
    </nav>
  </header>
);

/** Fits the ribbon to the window instead of scrolling sideways: tighter spacing, then groups folded from the right. */
const fitRibbon = (el: HTMLElement) => {
  const fit = () => {
    const gs = [...el.querySelectorAll<HTMLElement>(":scope > .grp")];
    el.dataset.fit = "0"; gs.forEach(g => delete g.dataset.fit);
    const over = () => el.scrollWidth > el.clientWidth + 1;
    if (!over()) return;
    el.dataset.fit = "1";
    // Whole groups fold from the right into a button with the group's name, its commands named below it.
    for (let i = gs.length - 1; i >= 0 && over(); i--) gs[i].dataset.fit = "4";
  };
  new ResizeObserver(fit).observe(el);
  new MutationObserver(() => requestAnimationFrame(fit)).observe(el, { childList: true, subtree: false });
};

const examplesMenu = (): Item[] => (out()?.examples ?? []).filter(e => e.id !== "empty").map(e => ({ label: e.title, run: () => C.newModel(e.id) }));
const templatesMenu = (): Item[] => (out()?.templates ?? []).map(t => ({ label: t.title, icon: "template", run: () => C.template(t.id) }));
const TPL_ICON: Record<string, string> = { channel: "inlet", "cavity-100": "lid", step: "poly", cylinder: "hcircle", "heated-1e4": "heat" };
const fieldMenu = (): Item[] => [["speed", "Speed"], ["pressure", "Pressure"], ["temperature", "Temperature"], ["vorticity", "Vorticity"]].map(([k, l]) => ({ label: l, on: out()?.m.view.field === k, off: k === "temperature" && !out()?.m.physics.heat, run: () => cmd("field", k) }));

const Draw = () => <>
  <G l="Draw" h="draw"><B big i="rect" l="Rectangle" on={tool() === "rect"} run={() => C.tool("rect")} /><B big i="circle" l="Circle" on={tool() === "circle"} run={() => C.tool("circle")} /><B big i="poly" l="Polygon" on={tool() === "poly"} run={() => C.tool("poly")} /></G>
  <G l="Combine" h="combine"><S><B i="union" l="Add" on={!asHole()} run={() => C.hole(false)} title="New shapes are part of the flow region" /><B i="subtract" l="Subtract" on={asHole()} run={() => C.hole(true)} title="New shapes are cut out of the flow region (a body in the flow)" /></S></G>
</>;
const Steps = () => (
  <G l="Steps" h="steps"><B big i="rect" l="Geometry" run={() => C.go("geometry", "geometry")} /><B big i="fluid" l="Fluid" run={() => C.go("fluid", "physics")} /><B big i="bc" l="Boundaries" run={() => C.go("boundaries", "physics")} />
    <B big i="mesh" l="Mesh" run={() => C.go("mesh", "mesh")} /><B big i="solve" l="Solve" run={() => C.go("solver", "solve")} /><B big i="contour" l="Results" run={() => C.go("results", "results")} /></G>
);

export const Ribbon = () => (
  <section class="ribbon" ref={fitRibbon}>
    <Show when={tab() === "home"}>
      <G l="Model" h="file"><B big i="new" l="New" run={() => C.newModel("empty")} dd={e => openMenu(e, [{ label: "Blank model", icon: "new", run: () => C.newModel("empty") }, "-", ...templatesMenu()])} /><B big i="folder" l="Examples" dd={e => openMenu(e, examplesMenu())} title="Finished cases to open, solve and change" />
        <S><B i="open" l="Open" run={() => C.open()} /><B i="save" l="Save" run={() => C.save()} /><B i="print" l="Print" run={C.print} /></S></G>
      <Steps />
      <G l="Run" h="solve"><B big i="mesh" l="Generate Mesh" run={() => cmd("mesh")} /><B big i="solve" l="Solve" run={() => cmd("solve")} /><S><B i="stop" l="Stop" run={() => cmd("stop")} /><B i="wizard" l="Checks" run={() => { C.panel("checks"); cmd("check"); }} /></S></G>
      <G l="Units" h="units"><S><B i="units" l="SI" on={out()?.m.units === "SI"} run={() => C.units("SI")} title="Metres, m/s, pascals, °C" /><B i="units" l="US" on={out()?.m.units === "US"} run={() => C.units("US")} title="Feet, ft/s, psi, °F" /></S></G>
    </Show>
    <Show when={tab() === "geometry"}>
      <G l="Select" h="edit"><B big i="select" l="Select" on={tool() === "select"} run={() => C.tool("select")} /><S><B i="pan" l="Pan" on={tool() === "pan"} run={() => C.tool("pan")} /><B i="del" l="Delete" run={C.del} off={sel().kind !== "shape"} /></S></G>
      <Draw />
      <G l="Templates" h="templates"><For each={out()?.templates ?? []}>{t => <B big i={TPL_ICON[t.id] ?? "template"} l={t.title} title={t.about} run={() => C.template(t.id)} />}</For></G>
      <G l="Import" h="importdxf"><B big i="dxf" l="Import DXF" run={() => cmd("import_dxf")} title="Outlines of lines and arcs from a DXF drawing" /></G>
      <G l="Grid" h="grid"><S><Chk l="Snap to grid" v={snapOn()} set={v => { setSnapOn(v); saveOpts({ ...opts(), snap: v }); }} /><Chk l="Show grid" v={showGrid()} set={v => { setShowGrid(v); saveOpts({ ...opts(), grid: v }); }} /><B i="grid" l="Grid step…" run={() => C.go("geometry")} /></S></G>
    </Show>
    <Show when={tab() === "physics"}>
      <G l="Fluid" h="fluid"><B big i="fluid" l="Fluid" run={() => C.go("fluid")} title={out()?.m.fluid.name} /></G>
      <G l="Heat" h="heat"><B big i="heat" l="Heat" on={!!out()?.m.physics.heat} run={() => { C.heat(!M().physics.heat); C.go("physics"); }} title="Solve the temperature too" /><B big i="gravity" l="Buoyancy" on={!!out()?.m.physics.buoyancy} run={() => { C.buoyancy(!M().physics.buoyancy); C.go("physics"); }} title="Warm fluid rises: natural convection" />
        <S><B i="temp" l="Heat & gravity…" run={() => C.go("physics")} /></S></G>
      <G l="Boundaries" h="boundaries"><B big i="inlet" l="Inlet" run={C.inlet} title="Flow comes in at a speed" /><B big i="outlet" l="Outlet" run={C.outlet} title="Flow leaves at a pressure" /><B big i="wall" l="Wall" run={C.wall} title="The fluid sticks to it; fixed or moving" /><B big i="sym" l="Symmetry" run={C.symmetry} title="No flow across, free to slide along" />
        <S><B i="bc" l="All boundaries" run={() => C.go("boundaries")} /><B i="wall" l="Wall the Rest" run={C.wallRest} title="Put every edge that is still free on a fixed wall" /></S></G>
      <G l="Check" h="checks"><B big i="wizard" l="Checks" run={() => { C.panel("checks"); cmd("check"); }} title="What must be right before a solve" /></G>
    </Show>
    <Show when={tab() === "mesh"}>
      <G l="Mesh" h="meshgen"><B big i="mesh" l="Generate" run={() => cmd("mesh")} /><B big i="quality" l="Quality" run={() => cmd("mesh_quality")} title="Element count, worst angle and size ratio, coloured on the view" /></G>
      <G l="Size" h="meshsize"><B big i="refine" l="Size & Refinement" run={() => C.go("mesh")} /><S><B i="wall" l="At walls…" run={() => C.go("mesh")} /><B i="point" l="Local sizes…" run={() => C.go("mesh")} /></S></G>
      <G l="Show" h="show"><S><Chk l="Mesh lines" v={!!out()?.m.view.mesh} set={v => { if (out()) act({ op: "set_view", view: { ...M().view, mesh: v } }); }} /></S></G>
    </Show>
    <Show when={tab() === "solve"}>
      <G l="Run" h="solve"><B big i="solve" l="Solve" run={() => cmd("solve")} /><B big i="stop" l="Stop" run={() => cmd("stop")} /><B big i="resume" l="Resume" run={() => cmd("resume")} title="Carry on from where the last solve stopped" /></G>
      <G l="Settings" h="solversettings"><B big i="opts" l="Solver Settings" run={() => C.go("solver")} /></G>
      <G l="Watch" h="bottom"><B big i="resid" l="Residuals" run={() => C.panel("residuals")} /><B big i="monitor" l="Monitors" run={() => C.panel("monitors")} /><S><B i="messages" l="Messages" run={() => C.panel("messages")} /><B i="wizard" l="Checks" run={() => { C.panel("checks"); cmd("check"); }} /></S></G>
    </Show>
    <Show when={tab() === "results"}>
      <G l="Colour by" h="resultsfield"><B big i="contour" l="Field" dd={e => openMenu(e, fieldMenu())} /><S><B i="speed" l="Speed" run={() => cmd("field", "speed")} /><B i="pressure" l="Pressure" run={() => cmd("field", "pressure")} /><B i="temp" l="Temperature" off={!out()?.m.physics.heat} run={() => cmd("field", "temperature")} /></S><S><B i="vort" l="Vorticity" run={() => cmd("field", "vorticity")} /><B i="cmap" l="Colours…" run={() => C.go("results")} /></S></G>
      <G l="On top" h="resultsfield"><B big i="stream" l="Streamlines" on={!!out()?.m.view.streamlines && !!out()?.has_results} run={() => cmd("streamlines")} /><B big i="arrows" l="Arrows" on={!!out()?.m.view.arrows && !!out()?.has_results} run={() => cmd("arrows")} /></G>
      <G l="Measure" h="probes"><B big i="probe" l="Probe" run={() => cmd("probe")} title="Read the values at a point" /><B big i="lineplot" l="Line Plot" run={() => cmd("lineplot")} title="Graph a value along a line you draw" /><B big i="values" l="Values" run={() => cmd("values")} title="Flow rate, pressure drop, drag and lift, heat flow, Nusselt number" /></G>
      <G l="Export" h="export"><B big i="report" l="PDF Report" run={() => cmd("report")} /><S><B i="image" l="Picture (PNG)" run={C.picture} /><B i="csv" l="CSV" run={() => cmd("export_csv")} /><B i="export" l="VTU" run={() => cmd("export_vtu")} /></S></G>
    </Show>
    <Show when={tab() === "check"}>
      <G l="Before solving" h="checks"><B big i="wizard" l="Checks" run={() => { C.panel("checks"); cmd("check"); }} /><S><B i="bc" l="Boundaries" run={() => C.go("boundaries")} /><B i="quality" l="Mesh quality" run={() => cmd("mesh_quality")} /></S></G>
      <G l="Trust" h="verification"><B big i="verify" l="Verification Report" run={() => cmd("verification")} title="The cases with known answers this version was tested on" /></G>
      <G l="Flow regime" h="numbers"><B big i="info" l="Re, Ra, Pr" run={() => C.go("model")} title="The numbers that say what kind of flow this is" /></G>
    </Show>
    <Show when={tab() === "view"}>
      <G l="Zoom" h="zoom"><B big i="fit" l="Fit" run={C.fit} title="Show the whole model (F)" /><S><B i="zin" l="Zoom In" run={C.zoomIn} /><B i="zout" l="Zoom Out" run={C.zoomOut} /><B i="pan" l="Pan" on={tool() === "pan"} run={() => C.tool("pan")} /></S></G>
      <G l="Show" h="show"><S><Chk l="Grid" v={showGrid()} set={v => { setShowGrid(v); saveOpts({ ...opts(), grid: v }); }} /><Chk l="Edge names" v={showNames()} set={setShowNames} /></S>
        <S><Chk l="Model tree" v={treeOpen()} set={setTreeOpen} /><Chk l="Properties" v={inspOpen()} set={setInspOpen} /><Chk l="Bottom panel" v={bottomOpen()} set={setBottomOpen} /></S></G>
      <G l="Panels" h="bottom"><S><B i="resid" l="Residuals" run={() => C.panel("residuals")} /><B i="monitor" l="Monitors" run={() => C.panel("monitors")} /></S><S><B i="wizard" l="Checks" run={() => C.panel("checks")} /><B i="messages" l="Messages" run={() => C.panel("messages")} /></S></G>
      <G l="Window" h="theme"><B big i="theme" l={`Theme: ${theme()}`} run={C.theme} /><S><B i="units" l="SI units" on={out()?.m.units === "SI"} run={() => C.units("SI")} /><B i="units" l="US units" on={out()?.m.units === "US"} run={() => C.units("US")} /></S></G>
    </Show>
    <Show when={tab() === "help"}>
      <G l="Learn"><B big i="report" l="Getting Started" run={H.gettingStarted} /><B big i="list" l="Glossary" run={H.glossary} title="Simulation terms in plain words" /><B big i="keys" l="Shortcuts" run={H.shortcuts} /><S><B i="search" l="Tell Me" run={palette} /><B i="help" l="What Is This Tool?" run={H.allHelp} /></S></G>
      <G l="Trust" h="verification"><B big i="verify" l="Verification" run={() => cmd("verification")} title="The cases with known answers this version was tested on" /></G>
      <G l="Datum Flow on GitHub"><B big i="github" l="GitHub" title="The project and its code" run={() => C.link()} /><B big i="tag" l="Releases" title="Every version and its change log" run={() => C.link("/releases")} /><B big i="bug" l="Report an Issue" run={() => C.link("/issues/new")} /></G>
      <G l="Updates" h="update"><B big i="upd" l="Check for Updates" run={() => setBackstage("about")} /><B big i="roadmap" l="Roadmap" run={() => setBackstage("roadmap")} /></G>
    </Show>
  </section>
);
