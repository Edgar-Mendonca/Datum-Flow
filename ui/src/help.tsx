// "?" help: a small button beside a tool; a click opens a short card saying what the tool is for
// and how to use it. One place for every explanation, so the whole app teaches itself. Also
// Getting Started, the Glossary and the Shortcuts.
import { For, Show, createSignal } from "solid-js";
import { openDialog } from "./store";
import { Dialog } from "./ui";

type Topic = { t: string; what: string; how?: string[]; tip?: string };
export const HELP: Record<string, Topic> = {
  file: { t: "Model", what: "A Datum Flow model is one .dflow file: the drawing, the fluid, the boundaries and the mesh and solver settings. It stays on your computer.",
    how: ["New starts a blank drawing; Examples opens a finished case to try.", "Save (Ctrl+S) writes the file; a recovery copy is kept while you work (File › Options)."] },
  side: { t: "Sidebar", what: "The views of Datum Flow, in the order a simulation is built: Setup, then Geometry, Physics and Mesh, then Solve and Results, then the pages of numbers: Values, Plots, Checks and Verification.",
    how: ["Click a view to open it. The first five show the 2D view with their ribbon tab; the others are pages.", "Setup opens a step-by-step panel on the right, in place of the properties; it opens by itself for a new blank model."] },
  setup: { t: "Setup", what: "A step-by-step guide through a new model: name and units, geometry, fluid, heat, boundaries, mesh, then check and solve. Each step says what it is for and ticks itself when it is done.",
    how: ["Back and Next move between the steps, or click a step's number.", "Skip any step: everything can also be set in the ribbon and on the right.", "Close it with ×; reopen it from Setup in the sidebar."] },
  plots: { t: "Plots", what: "The values at every probe in a table, and a chart for every line drawn across the flow, with the quantity to plot and a CSV of each." },
  steps: { t: "The steps", what: "A simulation is built left to right along the tabs: draw the geometry, choose the fluid and set the boundaries, mesh, solve, then read the results and check them.",
    how: ["The sidebar on the left opens each step as a view, and Setup walks through them in order; the model tree beside it lists the parts of the model.", "Check › Checks says what is still missing before a solve."] },
  draw: { t: "Draw", what: "The flow region is drawn from rectangles, circles and polygons. Shapes that add make the region; shapes that subtract cut bodies and holes out of it.",
    how: ["Rectangle and Circle: press, drag, release. Polygon: click each corner; Enter or a double-click closes it, Esc cancels.", "Type exact sizes on the right after drawing.", "A side becomes an arc when you give it a bulge on the right: 1 is a half circle, 0.414 a quarter."],
    tip: "The flow region is everything inside the adding shapes and outside the subtracting ones." },
  combine: { t: "Add and subtract", what: "Add makes the next shapes part of the flow region. Subtract makes them bodies in the flow: a cylinder in a channel, a fin, a baffle.",
    how: ["Pick Add or Subtract, then draw.", "Change a shape later with the Add / Subtract switch on the right."] },
  edit: { t: "Edit", what: "Move, rename and delete shapes, and name their edges.",
    how: ["Select: click a shape or an edge. Drag a selected shape to move it; it snaps to the grid.", "Delete removes the selected shape (Del).", "Name every edge you will put a boundary on: the name shows in the view and in the lists."] },
  templates: { t: "Templates", what: "Ready-made drawings to start from: a channel, a cavity, a step, a cylinder in a channel and a heated box. Each comes with its boundaries set, to change as you need.",
    how: ["Click a template: it replaces the open model (you are asked first if there are unsaved changes)."] },
  importdxf: { t: "Import DXF", what: "Brings in outlines from a DXF drawing: closed loops of lines and arcs become shapes." },
  grid: { t: "Grid and snap", what: "The grid is a drawing aid: with Snap on, points you click or drag land on grid points.", how: ["Set the grid step on the right (Geometry).", "Hold Alt while drawing to place a point freely."] },
  fluid: { t: "Fluid", what: "The fluid's properties: density, viscosity, thermal conductivity, specific heat and expansion coefficient, taken at one temperature.",
    how: ["Pick Water, Air or Light oil and the temperature: the values and where they come from are shown.", "Custom lets you type every value, for another fluid or a data sheet."],
    tip: "Properties are constant during a solve. Choose the temperature in the middle of the range you expect." },
  heat: { t: "Heat", what: "With Heat on, the temperature of the fluid is solved too. With Buoyancy on, warm fluid rises (natural convection).",
    how: ["Heat on: give boundaries a temperature or a heat flux, or leave them insulated.", "Buoyancy needs gravity and the reference temperature: the temperature at which the fluid neither rises nor sinks."] },
  boundaries: { t: "Boundaries", what: "A boundary says what happens at edges of the flow region: flow comes in, flow leaves, a wall, or a symmetry line. Every edge needs one.",
    how: ["Add an Inlet, Outlet, Wall or Symmetry, then click its edges in the view.", "Inlet: the mean speed, uniform or fully developed (a parabola between two walls).", "Outlet: the pressure there, usually 0 (pressures are relative to it).", "Wall: the fluid sticks to it. A moving wall has its own velocity (the lid of a cavity).", "Symmetry: no flow across, free to slide along.", "Wall the Rest puts every edge that is still free on a fixed wall."],
    tip: "Edges without a boundary are drawn dashed in red." },
  numbers: { t: "Reynolds, Rayleigh and Prandtl", what: "Three numbers that say what kind of flow this is. Reynolds compares inertia with viscosity; Rayleigh says how strongly buoyancy drives the flow; Prandtl compares how fast momentum and heat spread.",
    how: ["They are worked out from the fluid, the fastest inlet or moving wall, the temperature difference and the reference length.", "Set the reference length under Physics when the automatic one (the inlet's width, or the region's smaller side) is not the one your case is known by."],
    tip: "0.1 solves laminar flow. When the numbers say the flow is probably turbulent, Datum Flow says so before solving." },
  meshsize: { t: "Mesh size", what: "The mesh divides the flow region into small triangles; the solution is worked out on them. Smaller triangles give a more accurate answer and take longer.",
    how: ["Size: the general triangle size (Auto picks a sensible one).", "Walls, Corners and Wake make the mesh finer where the flow changes fastest, as a share of the general size.", "Local sizes set the size on one edge, near a point or in a box."],
    tip: "Solve on two mesh sizes: if the numbers you care about barely change, the mesh is fine enough." },
  meshgen: { t: "Generate the mesh", what: "Makes the mesh from the drawing and the size settings, and reports its quality: how many triangles, the worst angle and how fast sizes change." },
  meshquality: { t: "Mesh quality", what: "How good the triangles are. Thin triangles and sudden jumps in size make the answer less accurate and the solve harder.",
    how: ["Smallest angle: above 30° is good, 20° to 30° fair, below 20° poor.", "Size ratio: how much two neighbouring elements differ in size; a few times is normal where the mesh is refined.", "Colour by quality paints every element green, amber or red in the view; Show zooms to one of the thinnest.", "To improve a poor place: a smaller size there (Local sizes), or a smaller general size."] },
  solve: { t: "Solve", what: "Works out the steady flow (and temperature) on the mesh. It repeats until the equations are satisfied: the residuals fall as it goes.",
    how: ["Solve starts; Stop ends it at once and keeps the last state; Resume carries on from there.", "The Residuals and Monitors panels at the bottom plot the progress."] },
  solversettings: { t: "Solver settings", what: "How long and how hard the solver tries.",
    how: ["Iterations: the most steps of one solve.", "Tolerance: stop when the residual has fallen by this factor.", "Picard steps: simple, robust steps before the fast (Newton) steps take over.", "Continuation: for hard cases, the speed or heating is raised step by step from an easy case.", "Grad-div: a stabilisation that helps strong natural convection."] },
  resultsfield: { t: "What to show", what: "Colours the flow region by speed, pressure, temperature or vorticity, with arrows and streamlines on top.",
    how: ["Pick the field and the colour map.", "Bands draws steps of colour instead of a smooth shade."] },
  probes: { t: "Probes and line plots", what: "A probe reads the values at a point. A line plot graphs a value along a line you draw." },
  values: { t: "Values", what: "The numbers engineers need: flow rate through an edge, pressure drop between two edges, drag and lift on a body, wall heat flow and the Nusselt number." },
  export: { t: "Export", what: "Results as files for other tools: VTU for 3D viewers, CSV of probes and line plots, a PNG of the view, and a PDF report." },
  checks: { t: "Checks", what: "What must be right before a solve: every edge has a boundary, there is a way out for the flow, temperatures are set only with Heat on, and the flow is laminar.",
    how: ["Each line says what is wrong and where; click it to go there."] },
  verification: { t: "Verification", what: "Every release of Datum Flow solves a set of cases with known answers and shows the comparison: computed against reference, the error, the tolerance, and pass or fail." },
  show: { t: "Show", what: "What the view draws besides the model: the grid, edge names, and the panels around the view." },
  zoom: { t: "Zoom", what: "Move around the drawing.", how: ["Wheel: zoom at the pointer.", "Drag with the middle or right button, or hold Space and drag: pan.", "Fit (F) shows the whole model."] },
  theme: { t: "Theme", what: "Light, dark, or Auto, which follows the Windows setting." },
  units: { t: "Units", what: "SI (m, m/s, Pa, °C) or US (ft, ft/s, psi, °F), per model. Switching converts every number shown; the model itself does not change." },
  tree: { t: "Model tree", what: "Everything in the model, in the order you set it up: Geometry, Fluid, Boundaries, Mesh, Solver, Results. Click a line to see and change it on the right." },
  inspector: { t: "Properties", what: "The values of what is selected in the tree or the view. Changes apply as you type them; Undo (Ctrl+Z) takes them back." },
  bottom: { t: "Bottom panel", what: "Residuals and Monitors plot a solve as it runs. Checks lists what to fix before solving. Messages keeps what Datum Flow told you." },
  print: { t: "Print", what: "Prints the view, or saves it as a PDF, with a header and footer.", how: ["Choose the paper, orientation and margins on the left; the page on the right is what prints.", "Header and footer boxes take codes such as &[Model], &[Date] and &[Page]."] },
  options: { t: "Options", what: "Settings for Datum Flow on this computer: the theme, what happens at start-up, the recovery copy, defaults for new models, printing, and updates." },
  update: { t: "Updates", what: "Datum Flow checks GitHub for a new version when it starts. An update downloads, is checked against its published checksum, installs and restarts the app." },
};

/** The small "?" beside a tool. */
export const Help = (p: { k: string }) => <button class="help" title={`What is ${HELP[p.k]?.t ?? "this"}?`} onClick={e => { e.stopPropagation(); helpCard(p.k); }}>?</button>;

export const helpCard = (k: string) => openDialog(() => {
  const h = HELP[k];
  return (
    <Dialog title={h?.t ?? "Help"}>
      <div class="helpcard"><p>{h?.what}</p>
        <Show when={h?.how}><h4>How to use it</h4><ul><For each={h!.how}>{x => <li>{x}</li>}</For></ul></Show>
        <Show when={h?.tip}><p class="muted">{h!.tip}</p></Show></div>
    </Dialog>
  );
});

/** Help › What Is This Tool?: every card in one list. */
export const allHelp = () => openDialog(() => (
  <Dialog title="What is this tool?" wide>
    <div class="helpall"><For each={Object.entries(HELP)}>{([, h]) => <section><h4>{h.t}</h4><p>{h.what}</p><Show when={h.how}><ul><For each={h.how}>{x => <li>{x}</li>}</For></ul></Show></section>}</For></div>
  </Dialog>
));

const STEPS: [string, string[]][] = [
  ["1 · Draw the geometry", ["Start from a template (Geometry › Templates) or File › Examples, or draw: Geometry › Rectangle, Circle or Polygon.", "The flow region is what the adding shapes cover; Subtract cuts bodies out of it (a cylinder, a fin).", "Type exact coordinates on the right. Name the edges you will use."]],
  ["2 · Set the flow", ["Physics › Fluid: water, air, a light oil, or your own values.", "Physics › Boundaries: an inlet with its speed, an outlet, walls, symmetry lines. Click the edges each one applies to.", "For heat: turn Heat on and give boundaries a temperature or a heat flux; turn Buoyancy on for natural convection.", "The status bar shows the Reynolds number. Datum Flow 0.1 solves steady laminar flow in 2D."]],
  ["3 · Mesh and solve", ["Mesh › Generate makes the triangles and reports their quality. Refine near walls, corners and wakes.", "Solve › Solve starts. Watch the residuals fall in the bottom panel; Stop and Resume work at any time.", "If it does not converge, the message says what to try: a finer mesh somewhere, or raising the speed in steps."]],
  ["4 · See and check the results", ["Results: contours of speed, pressure, temperature and vorticity, arrows and streamlines, probes and line plots.", "Results › Values: flow rate, pressure drop, drag and lift, heat flow and Nusselt number.", "Check: the checks before a solve, and Help › Verification: the cases with known answers this version was tested on."]],
];
export const gettingStarted = () => openDialog(() => (
  <Dialog title="Getting started" wide>
    <div class="helpall start">
      <p>Datum Flow simulates fluid flow in two dimensions: you draw the region the fluid moves in, say what the fluid is and what happens at the edges, and it works out the velocity, pressure and temperature everywhere. Coming from another simulation tool? The steps are the ones you know, in plain words, left to right along the tabs.</p>
      <For each={STEPS}>{([t, items]) => <section><h4>{t}</h4><ul><For each={items}>{x => <li>{x}</li>}</For></ul></section>}</For>
      <p class="muted">A quick first run: File › Examples › Flow past a cylinder, then Mesh › Generate and Solve › Solve.</p>
    </div>
  </Dialog>
));

export const GLOSSARY: [string, string][] = [
  ["Grid Convergence Index", "A way to say how much an answer still depends on the mesh: solve on three meshes, each finer than the last, and see how the value changes. The index is an error band for the finest mesh (Celik and others, 2008). The verification report uses it."],
  ["Mass balance", "What flows in must flow out. Datum Flow adds up the flow through every boundary; the sum should be next to nothing beside the flow coming in. It is listed under Results › Values."],
  ["Newton iteration", "The fast way the solver closes in on the answer: each step uses how the equations change with the flow, so the error roughly squares every step. It needs a fair starting point, which a few Picard steps give."],
  ["Stream function", "A quantity whose level lines are the streamlines. The difference between two lines is the flow passing between them."],
  ["Boundary condition", "What you tell the solver about an edge of the flow region: flow comes in here at this speed, leaves there, this is a wall. Every edge needs one."],
  ["Boussinesq approximation", "A way to model natural convection: the density is taken as constant except in the buoyancy force, which grows with the temperature difference. Good for small temperature differences."],
  ["Buoyancy", "Warm fluid is lighter and rises; cool fluid sinks. With Buoyancy on, temperature differences drive the flow."],
  ["CFD", "Computational fluid dynamics: working out how a fluid moves by solving its equations on a mesh."],
  ["Continuation", "Reaching a hard case in steps: solve an easy one (slow flow, weak heating) and raise the speed or heating bit by bit, each solve starting from the last."],
  ["Contour", "A picture of a value over the flow region, drawn in colours: where the speed is high, where the pressure is low."],
  ["Convergence", "The solver repeats its calculation, and each round should change the answer less. When the residuals have fallen far enough, the solve has converged and the answer can be used."],
  ["Drag and lift", "The force of the flow on a body: drag along the flow, lift across it."],
  ["Element", "One triangle of the mesh. Datum Flow's triangles have six nodes, so they can follow curved edges and curved flow."],
  ["Flow rate", "How much fluid passes through an edge each second (per metre of depth in 2D)."],
  ["Fully developed flow", "Flow far along a channel, where the velocity profile no longer changes: a parabola between two walls when laminar."],
  ["Grad-div stabilisation", "A small extra term that keeps the computed flow free of spurious sources. It helps strong natural convection."],
  ["Heat flux", "Heat passing through a surface per second and per unit area (W/m²)."],
  ["Inlet", "An edge where fluid enters at a known speed."],
  ["Laminar flow", "Smooth, layered flow, at low Reynolds numbers. Datum Flow 0.1 solves laminar flow."],
  ["Mesh", "The flow region divided into small triangles. The solution is worked out on them; a finer mesh gives a more accurate answer."],
  ["Mesh independence", "The answer should not depend on the mesh: solve on a finer mesh and compare. If the numbers barely change, the mesh is fine enough."],
  ["Monitor", "A number watched while the solver runs, such as the drag on a body. When it stops changing, the answer has settled."],
  ["Natural convection", "Flow driven only by temperature differences: air rising from a warm wall."],
  ["Newton's method", "A fast way to solve the flow equations once the answer is roughly known: each step roughly squares the accuracy."],
  ["No-slip wall", "At a solid wall the fluid moves with the wall: it does not slide along it."],
  ["Nusselt number", "How much more heat crosses a fluid layer than conduction alone would carry. 1 means still fluid; larger means the flow carries heat."],
  ["Outlet", "An edge where fluid leaves freely, at a known pressure."],
  ["Picard iteration", "A simple, robust way to solve the flow equations: each step uses the last step's velocity. Slower than Newton, but it starts from anywhere."],
  ["Prandtl number", "A property of the fluid: how fast momentum spreads compared with heat. About 0.7 for air, 7 for water."],
  ["Pressure drop", "The pressure lost between two places, here between two edges: what a pump or fan has to overcome."],
  ["Probe", "A point where you read the values."],
  ["Rayleigh number", "How strongly buoyancy drives the flow compared with what damps it. Low: heat is conducted. High: convection, then turbulence."],
  ["Recirculation", "A region where the flow turns back on itself, such as behind a step or a body."],
  ["Residual", "How far the current answer is from satisfying the equations. The solver drives it down; a fall of many powers of ten means convergence."],
  ["Reynolds number", "Inertia compared with viscosity: speed × length × density ÷ viscosity. Low: smooth laminar flow. High: turbulent flow."],
  ["Steady flow", "Flow that does not change with time."],
  ["Streamline", "A line the fluid follows. Close together: fast flow. Closed loops: an eddy."],
  ["Symmetry", "An edge the flow does not cross and slides along freely: the middle line of a symmetric case, so only half needs solving."],
  ["Turbulent flow", "Irregular, swirling flow at high Reynolds numbers. It needs a turbulence model, which arrives in a later version."],
  ["Verification", "Proof that the solver solves its equations correctly: cases with known answers, solved and compared."],
  ["Viscosity", "How strongly a fluid resists being sheared: honey has a high viscosity, air a low one."],
  ["Vorticity", "How fast the fluid spins locally. High near walls and in eddies."],
];
export const glossary = () => openDialog(() => {
  const [q, setQ] = createSignal("");
  const list = () => [...GLOSSARY].sort((a, b) => a[0].localeCompare(b[0])).filter(([t, d]) => (t + " " + d).toLowerCase().includes(q().toLowerCase()));
  return (
    <Dialog title="Glossary" wide>
      <input class="gsearch" placeholder="Search the terms" value={q()} onInput={e => setQ(e.currentTarget.value)} />
      <dl class="glossary"><For each={list()} fallback={<p class="muted">No term matches.</p>}>{([t, d]) => <><dt>{t}</dt><dd>{d}</dd></>}</For></dl>
    </Dialog>
  );
});

export const SHORTCUTS: [string, string][] = [
  ["Ctrl+N", "New model (the File page)"], ["Ctrl+O", "Open"], ["Ctrl+S", "Save"], ["Ctrl+Shift+S", "Save As"], ["Ctrl+P", "Print"],
  ["Ctrl+Z", "Undo"], ["Ctrl+Y", "Redo"], ["Ctrl+K", "Tell me what you want to do"], ["F1", "Shortcuts"],
  ["F", "Fit the model in the view"], ["Wheel", "Zoom at the pointer"], ["Space + drag", "Pan (also the middle or right button)"],
  ["Esc", "Cancel the drawing, back to Select"], ["Enter", "Close the polygon being drawn"], ["Del", "Delete the selected shape or boundary"], ["Alt", "While drawing: do not snap to the grid"],
];
export const shortcuts = () => openDialog(() => (
  <Dialog title="Keyboard shortcuts">
    <table class="tbl keys"><tbody><For each={SHORTCUTS}>{([k, d]) => <tr><td><kbd>{k}</kbd></td><td>{d}</td></tr>}</For></tbody></table>
  </Dialog>
));
