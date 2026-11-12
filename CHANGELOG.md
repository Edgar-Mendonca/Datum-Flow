# Changelog

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
