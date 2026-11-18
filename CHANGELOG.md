# Changelog

## 0.1.5 — 2026-11-18

Arcs and rounded corners, smoother colours, readable streamlines, and reports whose tables flow over pages.

- **Arc Side.** Geometry › Arc Side: click a side of a polygon or rectangle, then click where the arc should pass. A point on the side's line makes it straight again.
- **Round Corner.** Geometry › Round Corner: click a corner and type the radius. The arc meets both sides smoothly; where the two sides share a boundary the arc joins it, and otherwise the setup check asks you to give it one.
- **Outlines checked with their arcs.** An arc that would cross another side is refused at once, with the reason, instead of failing later in the mesher.
- **Smoother colours.** Contours now follow each element's curved (quadratic) values instead of four flat pieces per element.
- **Readable streamlines.** Streamlines that would crowd closer than a few pixels are left out, so in a long thin region seen whole (the heat sink) they no longer hide the colours. Zoom in and they come back.
- **Reports that flow.** The PDF report and the printed verification record lay themselves out on the paper chosen: tables split between rows, carry their header onto the next page, say "(continued)", and nothing is cut off. A larger paper or smaller margins give fewer pages.
- Shallow arcs are drawn smoothly in the view.

## 0.1.4 — 2026-11-18

Models open solved, and the colours of results can be fixed by hand.

- **Results kept with the model.** Saving keeps the mesh and the solution in the .dflow file, so a model opens solved: contours, streamlines, probes, values, the residual and monitor history, all without solving again. Solving again starts from the kept answer, so it takes one step. The numbers are stored exactly; a model of 25 000 unknowns makes a file of about 1 MB.
- **Only results that still fit.** If a file's model was changed outside Datum Flow, its results are left out, and Messages says why. The results never belong to another model.
- **Your choice.** File › Options › "Keep results in the file" turns it off for small files.
- **Colour range by hand.** Click the legend, or Results › Range in the properties: set the low and high values the colours run between, for each field. The range is kept with the model, so several models or meshes can be compared in the same colours. Values beyond it take the end colours, and the legend marks them with ≥ and ≤. Automatic sets it back.
- **Command line.** `flow-cli solve model.dflow --save solved.dflow` writes the model with its results, to open solved in the window. Solving a file that keeps results starts from them.

## 0.1.3 — 2026-11-18

The verification report now checks its reference values against the original papers, and the backward-facing step is compared with the experiment.

- **Reference values from the papers.** The lid-driven cavity's values were checked against Botella & Peyret (1998), Tables 6, 9 and 10, and the heated cavity's against de Vahl Davis (1983), Table V. Every row says which table its number comes from.
- **Tighter bands for the heated cavity.** de Vahl Davis gives the error of his own solution: 0.1%, 0.2%, 0.3% and 1% at Rayleigh numbers 10³ to 10⁶ (his Table VII). Datum Flow's Nusselt numbers are compared within those, instead of a flat 1%.
- **New checks.** The largest horizontal and vertical velocities on the cavity's mid-planes at each Rayleigh number, also from de Vahl Davis's Table V.
- **Validation against an experiment.** The backward-facing step is compared with the reattachment lengths measured by Armaly et al. (1983) at Reynolds numbers 100, 150, 300 and 389 (read off their Figure 14). The paper gives no uncertainty for these lengths, so the report says plainly that the 10% band used is a judgement.
- **Fixed:** on fine meshes, the step's reattachment length could be read at a small corner eddy at the foot of the step instead of where the main eddy ends.
- 48 of 48 checks pass. Still from secondary sources: the cylinder benchmark (the pages of the paper with its definitions and results have not been checked) and Kovasznay's exact solution.

## 0.1.2 — 2026-11-18

The Datum sidebar: every view one click away, and a step-by-step Setup for new models.

- **Sidebar.** As in every Datum app: one icon and label per view, Setup first, then Geometry, Physics and Mesh, then Solve and Results, then the pages of numbers: Values, Plots, Checks and Verify. The five views of the model show the 2D view with their ribbon tab; choosing a ribbon tab lights its view in the sidebar. The model tree stays beside it.
- **Setup.** A step-by-step panel on the right: name and units, geometry (or a template), fluid, heat, boundaries, mesh, then check and solve. Each step says what it is for and ticks itself when it is done. It opens by itself for a new blank model; reopen it from Setup in the sidebar. Picking a shape, an edge or a boundary shows its properties, with a button back to the steps.
- **Values page.** The values table on a full page: monitors, flow through each inlet and outlet, pressure drops, forces, heat flows, Nusselt numbers, the mass and heat balances and the extremes. Copy it as a table.
- **Plots page.** The probes in one table, and a chart for every line drawn across the flow, each with the quantity to plot and its CSV.
- **Checks page.** What kind of flow this is (Reynolds, Rayleigh and Prandtl numbers, with the speed and length they come from) and the full setup check. The sidebar shows a count of what needs attention, in red when something stops the solve.
- **Verify page.** The verification record of this version on a full page, as in Help › Verification.
- **Roadmap.** File › Roadmap lists the next releases one by one, each a small three-number version, up to 1.0.0.

## 0.1.1 — 2026-11-12

Mesh, solve and see the results: Datum Flow now simulates 2D steady laminar flow with heat and natural convection, and proves its answers.

- **Mesh.** Mesh › Generate makes the mesh, finer at walls, at corners that stick into the flow and in the wake of bodies, with a size slider and local sizes. The quality report gives the element count, smallest angle and size ratio, and colours the elements on the view.
- **Check before solving.** The Checks tab lists what is missing and how to fix it. Solving is refused, with the reason, when there is no outlet, an edge has no boundary, temperatures are set with heat off, or the flow is probably turbulent ("Re ≈ 12000: this flow is probably turbulent; 0.1 solves laminar flow only.").
- **Solve.** Steady laminar flow, with heat and buoyancy when turned on. Residuals and monitors (drag, pressure drop, outlet temperature and more) are plotted while it runs. Stop at any time and resume. When a solve does not converge, Datum Flow says what to try, and raises the Reynolds or Rayleigh number in steps by itself when a direct solve fails.
- **Results.** Contours of speed, pressure, temperature, vorticity and velocity; streamlines and arrows; colour maps that read well in print (viridis, inferno, cividis, and a blue-white-red map for signed values). Click to place probes, draw a line to plot along it, and read the values table: flow rate through each inlet and outlet, pressure drop, force on each wall, heat flow, Nusselt number, and the mass and heat balances.
- **Examples.** File › Examples: lid-driven cavity (Re 100 and 1000), channel flow, flow past a cylinder (Re 20), backward-facing step, heated cavity (Ra 10³ to 10⁶) and a heat-sink fin channel. Each solves in seconds.
- **Verification.** Help › Verification shows the record of this version: exact solutions reproduced to round-off, errors falling at the method's formal rate, and published benchmarks (cylinder in a channel, lid-driven cavity, heated cavity). 36 of 36 checks pass; the same record is in `docs/VERIFICATION.md`, with what is not verified said plainly.
- **Exports.** A PDF report (inputs, mesh, convergence, results, verification status), VTU for free viewers such as ParaView, CSV of probes and line plots, PNG of the view.
- **Import DXF.** Closed outlines of lines, arcs, circles and polylines become the flow region and the bodies in it.
- **Command line.** `flow-cli solve model.dflow` meshes, checks and solves a model without the window, and writes VTU and CSV files.
- Results are kept while the model is open and are not saved in the file: solve again after opening.

## 0.1.0 — 2026-11-12

The first release: the Datum Flow window, with drawing, fluids and boundaries. Meshing, solving and results follow in 0.1.1 to 0.1.5.

- **The window.** The Datum family's ribbon (Home, Geometry, Physics, Mesh, Solve, Results, Check, View, Help), File page, Options, printing, light and dark themes, and an updater that installs new versions from GitHub.
- **Draw the flow region.** Rectangles, circles and polygons, with arc sides; add shapes or cut them out as bodies in the flow. Type exact coordinates, snap to a grid, name every edge.
- **Templates and examples.** Channel, cavity, step, cylinder in a channel and heated box, and the six worked examples under File › Examples.
- **Choose the fluid.** Water, air or a light oil at a chosen temperature, each property with its unit and its source, or your own values. SI or US units, switchable at any time.
- **Say what each edge is.** Inlets (uniform or fully developed), pressure outlets, fixed and moving walls, symmetry lines, with temperatures and heat fluxes. Edges still without a boundary are drawn dashed in red.
- **Reynolds, Rayleigh and Prandtl numbers** in the status bar as you edit, with a plain warning when the flow is probably turbulent.
- **Files.** `.dflow` models save and open; a damaged file opens as far as it can and says what was left out. Undo and redo for every edit, autosave and recovery.
- Buttons for meshing, solving and results are in place and say which version brings them.
