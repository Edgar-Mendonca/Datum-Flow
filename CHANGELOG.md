# Changelog

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
