# Datum Flow: project memory

> **Paste this file at the start of each new session**, or just open the repo: `CLAUDE.md` tells Claude to read it.
> It records what exists, why, and what comes next. Update it at the end of every session.

| | |
| --- | --- |
| **Product** | Datum Flow: fluid-flow simulation (CFD) on the desktop. Draw or load a geometry, choose a fluid, set inlets, outlets and walls, mesh, solve, and see velocity, pressure and temperature, with pressure drop, flow rate, drag and lift, heat transfer. Every release proves its answers in a built-in verification report |
| **Tagline** | "Fluid flow simulation: draw it, set the flow, see and check the results." |
| **Current version** | 0.1.4 (2026-11-18) |
| **Repo** | `Edgar-Mendonca/Datum-Flow` · branch `claude/magical-galileo-k8g6qh` (the first branch pushed) |
| **Stack** | Rust engine (`crates/`, own solver), Tauri 2, SolidJS + strict TypeScript; CPU only (threads), no GPU in 0.x |
| **Identifier** | `com.edgarmendonca.datumflow` · files `.dflow` (JSON, `format` 1) · installer `DatumFlow-x.y.z-setup.exe` + `SHA256SUMS.txt` · publisher "Datum" |
| **Accent** | Cobalt `#2F45B5` (`--blue2` `#25389A`, `--sky` `#9AA8E6`, `--pale` `#ECEEFA`, `--sel` `#DEE3F7`, `--sel2` `#C3CAF0`; dark `#8C9CF0`; File sidebar `#1A2560`). Results colour maps are separate from the accent |
| **Logo** | `docs/brand/datum-flow-icon.svg` (final, from the hub's `brand/`): a venturi with three streamlines and the amber diamond in the throat |
| **Brief** | `Edgar-Mendonca/Datum` › `briefs/DATUM-FLOW.md` (hub session 46, 2026-11-12): every decision made up front |
| **Reference repos** | `Edgar-Mendonca/Datum` (FAMILY, MEMORY, brain), `Edgar-Mendonca/Datum-Mesh` (closest relative: T6 elements, spade mesher, faer, verification report), `Edgar-Mendonca/Datum-Plan` (the shell), `Edgar-Mendonca/Datum-Calc` (units), `Edgar-Mendonca/Datum-Plot` (chart style) |

## Purpose

To give engineers, researchers and students trustworthy fluid-flow simulation without a specialist setup: a free app that meshes, solves and shows the flow, and proves its own answers.

- **For:** mechanical, process, HVAC and energy engineers; researchers; students and lecturers.
- **Different because:** trust you can see (a verification report in every release; mass conservation and setup checks on every case), free and open (MIT), simple (draw, set, solve, read), honest about its limits (laminar only in 0.1, and it says so before solving).
- **In the family:** Datum Flow is the fluids app. Datum Mesh keeps solids and heat conduction. They meet later: flow loads into Mesh, and conjugate heat transfer.

## 1. Architecture

The engine is six pure-Rust crates; the shell and the window only show what they compute.

```
flow-core ─▶ flow-mesh ─▶ flow-solve ─▶ flow-post ─▶ flow-verify
 (model)      (mesh,        (assembly,    (fields,     (cases, report)
              checks)       Newton)       integrals)        │
     └────────────┴─────────────┴────────────┴──────────────┴─▶ src-tauri (commands) ─▶ ui (window)
                                                              └─▶ flow-cli
```

- **Method:** Taylor–Hood finite elements on six-node triangles (velocity and temperature on six nodes, pressure on three corners; curved sides on arcs). One coupled system for velocity, pressure and temperature. Momentum in the Laplace form (−μ∇²u), so a pressure outlet is the natural "do nothing" condition and fully developed flow leaves undisturbed. Boussinesq buoyancy.
- **Nonlinear solve:** `picard` Picard iterations, then Newton with a backtracking line search on scaled residuals; on failure, continuation: boundary speeds, outlet pressures and gravity are scaled from 10% up to 100% in adaptive steps, each from the last answer.
- **Convergence:** each block of the residual (momentum, continuity, energy) is scaled by its largest value so far, starting from the fluid at rest; continuity is never scaled by less than (largest speed) × (lumped areas)/L, so a flow starting from rest is not judged against round-off.
- **Linear solve:** sparse LU with partial pivoting (faer) behind the `LinearSolver` trait; the pattern is built once from node adjacency and the symbolic analysis reused.
- **Forces and wall heat flows:** the residual at fixed unknowns is the reaction (traction, heat flux) on that node. Summed over a boundary's nodes it gives the force or heat flow to far better accuracy than differentiated fields. Corner nodes shared with another boundary are integrated directly instead, from the element's gradients.
- **Streamlines:** level lines of the stream function ψ, found by a finite-element solve (the function whose gradient best fits (−v, u)).
- **Units:** SI in the engine and the file; `flow_core::units` converts for display.

### Files

| File | What it does |
| --- | --- |
| `crates/flow-core/src/model.rs` | The model: shapes (polygons with arc sides, circles), fluid, physics, boundaries, mesh and solver settings, monitors, probes, lines, view; `normalize` makes any loaded model safe |
| `crates/flow-core/src/doc.rs` | `Doc` and `Op`: every edit, applied to a copy and kept only if sound; undo and redo |
| `crates/flow-core/src/file.rs` | The `.dflow` file: versioned JSON, forgiving reader with notes, migrations |
| `crates/flow-core/src/fluid.rs` | Water, air and a light oil at a temperature, each with its source |
| `crates/flow-core/src/units.rs` | Display units (SI, US) by kind of quantity; number formatting |
| `crates/flow-core/src/numbers.rs` | Reynolds, Rayleigh and Prandtl numbers; the turbulence limits |
| `crates/flow-core/src/examples.rs` | The examples and templates as complete models |
| `crates/flow-core/src/wire.rs` | What the window is sent after every action |
| `crates/flow-core/src/hardening.rs` | Hardening tests: failed edits, random and hostile edits, damaged files, speed |
| `crates/flow-cli/src/engine.rs` | The session the window and the command line share: mesh, check, solve (as a job for a worker thread), fields, values, exports |
| `crates/flow-mesh/src/mesh.rs` | The mesher: graded edge points, constrained Delaunay with refinement (spade), fluid triangles, boundary tagging, midside nodes on arcs, quality report |
| `crates/flow-mesh/src/size.rs` | The size field: finer at walls, sharp corners, wakes and asked-for places, growing away from them |
| `crates/flow-mesh/src/check.rs` | The checks before solving (the boundary wizard) |
| `crates/flow-solve/src/fe.rs` | Shape functions, quadrature, element geometry |
| `crates/flow-solve/src/setup.rs` | The discrete problem: fixed unknowns and boundary loads from a model, or by hand for verification |
| `crates/flow-solve/src/assemble.rs` | Residual and Jacobian (Picard or Newton); the equations are written at the top |
| `crates/flow-solve/src/sparse.rs` | Sparse matrix, `LinearSolver`, the faer LU |
| `crates/flow-solve/src/solve.rs` | Picard, Newton with line search, continuation, residual history, stop |
| `crates/flow-solve/src/advice.rs` | Plain advice when a solve does not converge |
| `crates/flow-post/src/fields.rs` | Nodal fields: speed, pressure, temperature, vorticity, stream function |
| `crates/flow-post/src/locate.rs` | Finding the element that holds a point |
| `crates/flow-post/src/values.rs` | Probes, line plots, arrows; flow rate, pressure drop, forces, heat flow, Nusselt number, mass and heat balance |
| `crates/flow-post/src/export.rs` | VTU and CSV |
| `crates/flow-core/src/pack.rs` | Exact, compact storage of long arrays in the file (base64), and the fingerprint that ties stored results to their model |
| `crates/flow-core/src/dxf.rs` | Outlines from a DXF drawing: lines, arcs, circles, polylines |
| `crates/flow-verify/src/cases.rs` | The verification cases: exact solutions, manufactured solution, Kovasznay, cylinder, cavity, heated cavity, step |
| `crates/flow-verify/src/gci.rs` | The Grid Convergence Index (Celik et al. 2008) |
| `crates/flow-verify/src/report.rs`, `report.json`, `src/bin/report.rs` | The record's shape, the record shipped with the app, and the program that runs every case and writes it with `docs/VERIFICATION.md` and `docs/verification/<version>.{md,json}` |
| `crates/flow-verify/src/tests.rs` | The cases as tests; the full suite is `#[ignore]` (nightly and release) |
| `crates/flow-cli/src/cli.rs`, `main.rs` | `flow-cli`: solve, check, mesh, example, verify from the command line |
| `crates/flow-cli/examples/bridge.rs` | The UI tests' fixture and line-protocol bridge to the real engine |
| `src-tauri/src/main.rs` | The shell: commands, `guard()`, atomic saves, recovery, options |
| `src-tauri/src/update.rs`, `pdf.rs` | The updater (`REPO = "Edgar-Mendonca/Datum-Flow"`); writing PDFs |
| `ui/src/store.ts`, `api.ts`, `cmds.ts` | State, the commands and their types, the command table (and the stubs not yet wired) |
| `ui/src/ribbon.tsx`, `backstage.tsx`, `ui.tsx`, `help.tsx`, `roadmap.tsx`, `print.tsx` | The family window: ribbon, File page, dialogs, help cards, roadmap, printing |
| `ui/src/canvas.tsx` | The 2D view: shapes, grid, edges by boundary, drawing tools, pan and zoom, layers for mesh and results |
| `ui/src/side.tsx` | The sidebar of views, the step-by-step Setup panel, and the Values, Plots, Checks and Verify pages |
| `ui/src/tree.tsx`, `inspector.tsx`, `bottom.tsx` | Model tree, inspector, bottom panel (Residuals, Monitors, Checks, Messages) |
| `tests_ui/run.cjs`, `realapp.sh` | Playwright checks of the window against the real engine; screenshots of the real app |
| `tools/commercial-names.txt` | Product names that must not appear in public words |
| `.github/workflows/ci.yml`, `nightly.yml`, `release.yml` | Fast checks on push; heavy checks nightly; the release with build provenance |

### Release flow

Bump the version in the root `Cargo.toml`, add the CHANGELOG section, regenerate the verification report, run every check, push, then run `release.yml` by hand. It runs the full verification suite in release mode, builds the installer, writes `SHA256SUMS.txt`, attests provenance, tags and publishes.

### How we work

1. The user reviews a release and sends screenshots.
2. Reproduce first.
3. Big asks get a plan file and questions; build after the answers.
4. Engine first (with a test), then the UI.
5. Check every change in the real app (Xvfb screenshots, `tests_ui/realapp.sh`).
6. Add a UI test for every bug.
7. Run all checks: clippy `-D warnings`, `cargo test` (with the hardening tests and verification cases), the UI type-check and UI tests, `cargo deny check`.
8. Release.
9. Report in plain language: what changed, what was checked, what is not done.

## 2. Decisions

| Version | Decision | Why |
| --- | --- | --- |
| 0.1.0 | **Datum Flow is the family's fluids app**; Datum Mesh keeps solids and conduction; the old "pipe pressure drop" idea of this name is retired to Calc and Schema | The brief (hub session 46) |
| 0.1.0 | **Own solver in Rust inside the app**; GPL solvers only ever as separate optional programs (0.6) | No mature CFD code can be built into an MIT app |
| 0.1.0 | **Taylor–Hood P2/P1, coupled, Picard then Newton, sparse direct LU behind `LinearSolver`** | Stable without tricks; the brief |
| 0.1.0 | **Momentum in the Laplace form** (−μ∇²u, not the full stress divergence) | The pressure outlet is then the plain "do nothing" condition: fully developed flow leaves exactly, and the cylinder benchmark is defined this way. On a fixed wall the two forms give the same force |
| 0.1.0 | **Six crates** (`flow-core`, `-mesh`, `-solve`, `-post`, `-verify`, `-cli`), each depending only on those before it | The brief §4; the solver knows nothing of files or windows |
| 0.1.0 | **Boundaries hold lists of edges**; an edge with no boundary refuses the solve; "Wall the Rest" fills the gaps | One setting covers many edges; nothing is assumed silently |
| 0.1.0 | **Edits are `Op`s applied to a copy**; undo keeps whole models | A failed edit changes nothing (hardening standard); models are small |
| 0.1.0 | **Loading clamps** every value (`normalize`) and reads a damaged file part by part, with notes | Hardening standard; gravity may be very large (benchmarks set the Rayleigh number with it) |
| 0.1.0 | **Units: a small table by kind of quantity** in the engine, not Calc's whole unit algebra | Flow shows fixed kinds of quantity; the full engine is not needed. The window never converts by itself |
| 0.1.0 | **Fluid properties:** water from rounded handbook values of the IAPWS formulations (10 °C steps, interpolated); air as an ideal gas with Sutherland's law; oil as a typical ISO VG 10 mineral oil with ASTM D341 viscosity | Each value has a stated source; the table values were keyed from memory of standard handbooks and should be checked against a printed table before 1.0 |
| 0.1.0 | **The turbulence check blocks with an override**: Re > 4000 with an inlet, Re > 10 000 in a closed box, Ra > 10⁹; "Solve anyway" makes it a warning | The brief asks for a refusal in plain words; a laminar answer can still be instructive |
| 0.1.0 | **A fully developed inlet is one straight edge; its speed is the mean. Symmetry edges must be straight and axis-aligned** | Simplest sensible rules for 0.1; sloping slip edges need rotated unknowns |
| 0.1.0 | **Where boundaries meet, fixed walls win, then moving walls, then inlets, then symmetry** | A lid's corners belong to the side walls (no leak); an inlet's corners to the walls |
| 0.1.0 | **Mesh size field on a background grid**; refinement at walls (×0.5), corners that stick into the flow and lid corners (×0.25), wakes behind bodies (×0.5), growth 0.2 | Fast to look up; the brief's "walls, corners and the wake" |
| 0.1.0 | **Forces and wall heat flows from reactions**, with shared corner nodes integrated directly | The cylinder's drag is right to five digits on the example mesh; next to an inlet the plain reaction would include the inlet's pressure |
| 0.1.0 | **Drag is along the oncoming flow** (the mean direction through the inlets, else +x); lift is to its left | Works for any orientation |
| 0.1.0 | **Residuals are scaled per block by their largest value so far**, with a physical floor for continuity | A cavity heated from rest has no continuity residual to start with; without the floor the solver chased round-off |
| 0.1.0 | **Kovasznay flow is checked by its convergence rate**, not at round-off | Its solution is exponential: no polynomial element is exact for it |
| 0.1.0 | **The verification report is computed at release time and shipped** | The full suite takes minutes |
| 0.1.0 | **One shared build budget:** CI on `main` and pull requests only, concurrency cancel, docs ignored; heavy work nightly and at release; no macOS | FAMILY §9 |
| 0.1.1 | **One engine session (`flow_cli::Engine`) for the shell, the command line and the UI tests** | They cannot disagree; the UI tests drive the real mesher and solver through a line-protocol bridge |
| 0.1.1 | **Solving runs on a worker thread as a `Job`** that owns copies of the model, mesh and start state; its results are kept only if the model has not changed meanwhile | The window stays live; Stop is a flag read after every iteration |
| 0.1.1 | **Results live in memory, not in the `.dflow` file** | Examples solve in seconds; the file stays small and readable. Solve again after opening |
| 0.1.1 | **Benchmark tolerance: twice the Grid Convergence Index of the fine mesh, or the reference's stated accuracy when wider; when the three meshes do not converge steadily, three times the largest change between meshes** | The brief's rule, made usable on unstructured meshes: the cylinder's drag is so accurate that mesh-to-mesh noise exceeds its error, and the index then has no order to stand on (the safety factor of 3 is the one recommended when the order is unknown) |
| 0.1.1 | **The benchmark meshes are a coarser series than the examples'** (sizes ÷ 0.525, 0.79, 1.18 for the cylinder; ÷ 0.6, 0.9, 1.35 for the cavities) | Three refinements with ratio 1.5 that run in minutes; the error bands come from the data |
| 0.1.1 | **de Vahl Davis's Nusselt numbers are compared within his stated 1%** | His 1983 values are less accurate than this solver at Ra 10⁶ (8.800 against 8.825, which later studies also find); the band says so instead of hiding it |
| 0.1.1 | **The backward-facing step has no verdict** | Armaly's reattachment lengths are in a figure that could not be read; the computed lengths (2.85, 4.87, 8.00 step heights at Re 100, 200, 389) are reported under "Not verified" |
| 0.1.1 | **The stream-function minimum is searched inside elements**, not only at nodes | The node value alone was 4e-5 short of the published cavity value |
| 0.1.1 | **The brief's steps 2 to 6 ship as one release, 0.1.1** | They were finished together; the build budget says release in batches, and version numbers may not jump |
| 0.1.2 | **The family sidebar (66 px, icon + mono label, Setup first, groups split by rules), with the model tree kept beside it** | The user asked for the left pane of every Datum app; Plot has the same pair (sidebar + navigator), and the brief asked for the tree |
| 0.1.2 | **Ten views:** Geometry, Physics, Mesh, Solve, Results show the 2D view (each brings its ribbon tab, and a ribbon tab lights its view); Values, Plots, Checks, Verify are pages laid over the 2D view, which stays mounted | Pages need the width; the view keeps its zoom and layers underneath |
| 0.1.2 | **Setup is a step-by-step panel on the right, in the properties' place** (as in Datum Plan), opened by itself for a new blank model; picking anything shows its properties with "Back to Setup" | Datum Plan's pattern; the properties are still needed to type exact sizes while setting up |
| 0.1.2 | **The roadmap is written one three-number release per line**, from 0.1.3 to 1.0.0 (site.json, this file, File › Roadmap) | The user asked for the versions broken down to three numbers; small numbered releases (FAMILY §9) |
| 0.1.3 | **Reference values checked against the papers the user supplied**: Botella & Peyret (1998, Tables 6, 9, 10), de Vahl Davis (1983, Tables V and VII), Armaly et al. (1983). Each row's source names its table or figure; `confirmed` is "original paper" | The brief: values must be re-checked against the original papers before they gate a release |
| 0.1.3 | **The heated cavity's tolerance is de Vahl Davis's own estimated error per Rayleigh number** (0.1, 0.2, 0.3, 1%, his Table VII), not a flat 1%; his mid-plane velocity maxima are added | Read in the paper; the Nusselt numbers used are his cavity averages (Nu-bar), and his wall value Nu₀ differs from them by less than his error |
| 0.1.3 | **Armaly's measured reattachment lengths were digitised from Figure 14**, calibrated against its tick marks (3.05, 4.05, 6.62, 8.61 step heights at Re 100, 150, 300, 389); only Re < 400, where the authors found the flow two-dimensional | The numbers appear only in figures. Figure 14 is labelled x₃/S but below Re 400 only the primary eddy exists, and its values match Figure 4's x₁ |
| 0.1.3 | **The step's validation band is 10%, a judgement stated in the report** | The paper states no uncertainty. The band was set after the first comparison (6.3% at Re 100, 7.0% at Re 389); Armaly's own 2D computation gives about 8.05 at Re 389, beside 8.00 here, so the gap there is the experiment's three-dimensionality, as the authors say |
| 0.1.3 | **The cylinder (Schäfer & Turek 1996) stays "secondary sources"** | The file supplied held pages 547–548 only (the introduction); the definitions and results are on pages 549–566 |
| 0.1.3 | **The OpenAI "Finite Time Blowup for Navier–Stokes" paper (dated September 2026, supplied by the user) changes nothing in Flow** | It is a pure existence proof for 3D flow with an engineered force; it gives no computable flow or method, and 2D flows cannot blow up. At most a page in the teaching pack (0.9.0), once Flow has 3D and if the result is accepted |
| 0.1.4 | **Results are kept in the `.dflow` file** (option, on by default): the mesh and the solution under `results`, long arrays as little-endian bytes in base64 (`flow_core::pack`), exact and about a third the size of decimal text; the residual history and monitors as plain JSON | A model should open solved. Exact storage means the values, forces and fields read back bit for bit (tested) |
| 0.1.4 | **Stored results are used only if the model's fingerprint matches** (FNV-1a of the drawing, fluid, physics, boundaries, mesh and solver settings; not the view, probes or name); a stored mesh is checked (numbering, orientation, finite values) and its quality worked out again | Results must never be shown for a model they were not solved from; changing how results are shown keeps them |
| 0.1.4 | **Recovery copies do not keep results** | They are written every few minutes; results can be solved again |
| 0.1.4 | **Colour ranges are kept per field in the view** (`view.ranges`, SI), values beyond take the end colours, the legend marks them; the legend itself is the button | One range per quantity, so switching fields keeps each one's range; comparing models needs the same colours |
| 0.1.0 | **The window shell was built by a parallel worker session from Datum Plan's shell**, with a bridge so UI tests drive the real engine | Speed; the engine and the shell meet only at `wire::Out` and `Op` |

## 3. Roadmap

Every release is one small three-number version (FAMILY §9); each line below is one release.

| Version | Scope | State |
| --- | --- | --- |
| 0.1.0 | Skeleton: window, drawing, fluid, boundaries, files, updater, CI, release with provenance | ✅ |
| 0.1.1 | Mesh, checks, solve, results, heat and buoyancy, examples, verification report, exports, DXF, `flow-cli` | ✅ |
| 0.1.2 | The family sidebar (Setup first, then Geometry, Physics, Mesh, Solve, Results, Values, Plots, Checks, Verify), the step-by-step Setup panel, the Values, Plots, Checks and Verify pages; the roadmap in three-number releases | ✅ |
| 0.1.3 | **Reference values from the papers:** the cavity and heated-cavity values checked against the original papers (the cylinder's still to do: its paper's later pages); the backward-facing step compared with the measurements of Armaly et al. (1983); the heated cavity's velocity maxima | ✅ |
| 0.1.4 | **Results kept with the model:** results saved in the .dflow file, so a model opens solved; a manual range for the colour legend; `flow-cli --save` | ✅ |
| 0.1.5 | **Drawing and the report:** streamlines thin out where the region is drawn small (in a long thin region such as the heat sink they hide the colours at full view); a drawn-arc tool and rounded corners; contours shaded with the elements' true curved values; long tables flow over pages in the PDF report | ⬜ |
| 0.1.6 | **Guide and tutorial:** help › Guide: a printable user guide with pictures of the real window; help › Tutorial: one example that uses every view, with a note on each | ⬜ |
| 0.2.0 | **Time-dependent flow:** time stepping, with the step size chosen for accuracy; verified against flows that start from rest and have an exact answer | ⬜ |
| 0.2.1 | **Vortex shedding:** the cylinder in a channel at Re 100: the benchmark's largest drag and lift and its Strouhal number in the verification report | ⬜ |
| 0.2.2 | **Faster flows:** stabilisation for flows where convection dominates, so coarser meshes stay smooth | ⬜ |
| 0.2.3 | **Animations:** play the flow through time; monitors plotted over time; save frames as pictures | ⬜ |
| 0.2.4 | **Transient heat:** heating and cooling over time, with natural convection | ⬜ |
| 0.3.0 | **Boundary layers:** thin layers of elements along walls, with a wall-distance helper | ⬜ |
| 0.3.1 | **Pipes and nozzles:** axisymmetric flow, verified against the exact flow in a round pipe | ⬜ |
| 0.3.2 | **Heat through solids:** solid parts beside the fluid, heat passing through both | ⬜ |
| 0.3.3 | **Heat-sink cases:** fins with their own conduction, and their benchmarks | ⬜ |
| 0.4.0 | **Iterative solvers:** iterative linear solvers with preconditioners, for models too big for the direct one | ⬜ |
| 0.4.1 | **Bigger models:** assembly on every core; meshes of a million unknowns | ⬜ |
| 0.4.2 | **Parameter sweeps:** run one model over a range of speeds, temperatures or sizes, with a table of the results | ⬜ |
| 0.5.0 | **The 3D decision:** 3D finite elements or finite volumes, chosen with measurements and written up | ⬜ |
| 0.5.1 | **3D meshing:** tetrahedral meshes of 3D regions | ⬜ |
| 0.5.2 | **First 3D cases:** 3D cavity and 3D cylinder benchmarks | ⬜ |
| 0.6.0 | **External solver: SU2:** write SU2 cases, run SU2 as a separate program you install, read the results back | ⬜ |
| 0.6.1 | **External solver: OpenFOAM:** the same for OpenFOAM | ⬜ |
| 0.7.0 | **Turbulence: one equation:** wall distance and the Spalart–Allmaras model, verified on published cases | ⬜ |
| 0.7.1 | **Turbulence: two equations:** the k-ω SST model, verified on published cases | ⬜ |
| 0.7.2 | **Turbulence checks:** flat plate, bump and step cases in the verification report | ⬜ |
| 0.8.0 | **STEP import:** 3D and 2D outlines from STEP files | ⬜ |
| 0.8.1 | **Family links in:** geometry from Datum Layout; line data from Datum Schema | ⬜ |
| 0.8.2 | **Family links out:** pressures and temperatures into Datum Mesh | ⬜ |
| 0.9.0 | **Teaching pack:** guided lessons on the classic cases, each with its theory and reference data | ⬜ |
| 0.9.1 | **What went wrong:** a mode that shows the common mistakes in a setup and what they do to the answer | ⬜ |
| 1.0.0 | **Version 1:** full verification and validation report, user guide, signed installer | ⬜ |

## 4. Session log

### 2026-11-12: Session 1 (the brief, the engine, 0.1.0)
- Read the hub (FAMILY, MEMORY, status, brand, brain) and the reference apps; wrote `CLAUDE.md`, this file and `docs/PLAN-0.1.md`.
- Engine: `flow-core` (model, ops, file, fluids, units, examples, hardening tests), `flow-mesh` (mesher with refinement, quality, checks), `flow-solve` (Taylor–Hood assembly, sparse LU, Picard and Newton, continuation), `flow-post` (fields, stream function, probes, integrals, VTU, CSV).
- First measurements, before any tuning: cylinder benchmark drag coefficient 5.57957 (published 5.57954) on the example mesh in 4 s; cavity Re 1000 stream-function minimum −0.118930 (published −0.118937); heated cavity Nusselt 1.118, 2.245, 4.522, 8.825 at Ra 10³ to 10⁶ (published 1.118, 2.243, 4.519, 8.800). 190 000 unknowns with heat solve in two minutes on four cores.
- The window shell (ribbon, File page, Options, updater, drawing, fluid and boundary editing, 77 UI checks) from Datum Plan's shell.
- Released 0.1.0.

### 2026-11-12: Session 1, continued (0.1.1: mesh, solve, results, verification)
- `flow_cli::Engine`: one session for the shell, the command line and the UI tests (mesh, check, solve as a job on a worker thread, fields, values, exports). The shell's commands and the tests' bridge both call it.
- `flow-verify`: exact solutions (Couette, Poiseuille, conduction) at round-off; orders of accuracy 2.97, 2.04, 2.99 (manufactured solution with flow, heat and buoyancy) and 3.00, 2.04 (Kovasznay); benchmarks with Grid Convergence Index bands: cylinder (drag 5.57958 against 5.57954), cavity Re 1000 (stream function −0.1189355 against −0.1189366, eddy centre and six centre-line velocities), heated cavity Ra 10³ to 10⁶ (Nusselt 1.1178, 2.2448, 4.5216, 8.8251). 35 checks plus a regression row against 0.1.0, all pass; the record is built into the app and written to `docs/VERIFICATION.md`.
- A research pass tried to confirm every reference value: the session's network could reach no original paper, so all are marked "secondary sources" (the cylinder values were read in the benchmark authors' own code; the cavity tables agree across four independent transcriptions). Armaly's step lengths could not be confirmed at all, so the step has no verdict.
- Fixed while testing: residual scaling for a fluid starting at rest (the heated cavity stalled against round-off); wall forces next to an inlet (corner nodes are now integrated directly); loading clamped gravity and broke the Ra 10⁵ and 10⁶ examples.
- Measured: every example solves in under 25 s (most in under 5 s) and conserves mass to round-off; 259 000 unknowns solve in 53 s on four cores; mesh independence of the examples' key numbers within 1%.
- The window's mesh, checks, solve, results, values, exports and verification views (a second parallel worker session): 119 UI checks against the real engine. The real app was run under Xvfb: the cylinder solves through the worker thread and its events.
- DXF import (lines, arcs, circles, polylines), `flow-cli`, PDF report, four screenshots for the site.
- Released 0.1.1. The brief's steps 2 to 6 shipped together (see Decisions).
- **Cut or not done:** nothing from the brief's cut list was cut (DXF import, the heat-sink example, VTU export and vorticity contours are all in). Not done: the guide and Help › Tutorial (FAMILY §10, not in the brief); multi-document tabs; a drawn-arc tool (arcs are typed as a bulge); Windows/macOS test jobs in CI (the build budget asks for Linux only; Windows runs nightly and at release).
- **2-minute demo:** File › Examples › Flow past a cylinder, Re 20 → Home › Solve (about 5 s: watch the residuals fall) → Results: speed contours with streamlines; Results › Values: "Drag on the cylinder" 0.01116 N (× 500 = drag coefficient 5.58) → Help › Verification: 36 of 36 checks pass, the cylinder row among them.

### 2026-11-18: Session 2, continued (0.1.4: models open solved; colour ranges)
- Engine: `flow_core::pack` (exact base64 arrays, the fingerprint), `file::read_all`/`write_with`, `flow_mesh::rebuild`, `Engine::to_file`/`from_file`; the shell keeps results on Save and restores them on Open (option), the bridge and `flow-cli --save` likewise.
- Window: models arrive with their mesh and history; the colour range dialog (legend click, properties row); help.
- Tests: an exact round trip of the heat sink (values, fields, history, mesh angles), refusal of changed or damaged results, hostile ranges on loading, the command line's --save and resume; 7 new UI checks.
- Released 0.1.4.

### 2026-11-18: Session 2, continued (0.1.3: the reference values from the papers)
- The user supplied Botella & Peyret (1998), de Vahl Davis (1983), Armaly et al. (1983) and the first two pages of Schäfer & Turek (1996). Every cavity value matched the original tables; the heated cavity's Nusselt numbers are his cavity averages; Armaly's geometry and Reynolds number match the Step example.
- Armaly's measured lengths were digitised from Figure 14 by calibrating pixels against the tick marks. A mesh and inlet-length study showed the computed lengths converged to 0.3% (2.86, 3.91, 6.64, 8.00), and found a bug in reading the reattachment point on fine meshes (a corner eddy), fixed.
- New rows: mid-plane velocity maxima of the heated cavity (8), the step against experiment (4). 48 of 48 pass.
- The user also supplied a paper claiming finite-time blowup for 3D Navier–Stokes (see Decisions): read, reported, no change to the plan.
- Released 0.1.3.

### 2026-11-18: Session 2 (0.1.2: the sidebar, Setup, the roadmap in three-number releases)
- The user confirmed 0.1.1 works on their computer (the first run of the Windows installer and app), and asked for the versions broken down to three numbers and for the left pane of every Datum app, both in 0.1.2.
- The sidebar (from Datum Plan and Datum Plot), the Setup panel (from Datum Plan's pattern) and four pages; 14 new UI checks (133 in all).
- The roadmap: one line per release from 0.1.3 to 1.0.0.
- Released 0.1.2. Still owed for 0.1.3: the reference values against the original papers, which needs the papers or network access to them.

## 5. Known limits

- 2D, steady, laminar only. No turbulence model: the setup check refuses flows that are probably turbulent.
- Symmetry and slip edges must be straight and horizontal or vertical.
- The direct solver's memory grows faster than the mesh: about 200 000 unknowns is the practical size in 0.1.
- No stabilisation for strongly convection-dominated flow on coarse meshes (SUPG comes in 0.2): the check warns when the mesh is coarse for the speed.
- Contours are drawn as four flat-shaded triangles per element (the values are quadratic); drawing speed above about 8 000 elements has not been timed.
- The PDF report puts each section on fixed pages: a very long boundaries table or notes would overflow. Arc sides are typed as a bulge value; there is no drawn-arc tool.
- No guide PDF and no Help › Tutorial yet; one document at a time.
- The cylinder benchmark's reference values and Kovasznay's solution are still from secondary sources (see `docs/VERIFICATION.md`); the other benchmarks were checked against their original papers in 0.1.3.
- The Windows installer, the PDF writer and the update install are compile-checked for Windows here but first run for real in the release workflow.
