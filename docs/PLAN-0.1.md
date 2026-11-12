# Datum Flow 0.1: plan

Written 2026-11-12 from the brief (`briefs/DATUM-FLOW.md` in the hub, session 46). Every decision in the brief is made; this file says how 0.1 is built and in what order. Open points are decided here and recorded in `docs/MEMORY.md` (Decisions).

## What 0.1 is

2D, steady, laminar flow with heat and natural convection, on the desktop: draw the region, choose the fluid, say what each edge is, mesh, solve, and read velocity, pressure and temperature with the numbers engineers need. A verification report from the first release.

## How it is built

| Part | Choice |
| --- | --- |
| Engine | Six crates in `crates/`: `flow-core` (model, units, file, undo), `flow-mesh` (mesher, quality, checks before solving), `flow-solve` (assembly, Picard and Newton, continuation), `flow-post` (fields, probes, integrals, exports), `flow-verify` (the cases and the report), `flow-cli` |
| Element | Taylor–Hood: six-node triangles, velocity and temperature on all six nodes, pressure on the three corners; curved sides on arcs |
| Equations | One coupled system for velocity, pressure and temperature; momentum in the Laplace form, so a pressure outlet is the plain "do nothing" condition |
| Nonlinear solve | A few Picard iterations, then Newton with a backtracking line search; continuation in the driving (boundary speeds, gravity) when that fails or when asked |
| Linear solve | Sparse direct LU (faer) behind `LinearSolver`; the symbolic analysis is reused between iterations |
| Forces and heat flows | From the reactions of the discrete equations at fixed unknowns, not from differentiated fields |
| Streamlines | Level lines of the stream function, itself a small finite-element solve |
| Window | The family shell (from Datum Plan), model tree, 2D view, inspector, bottom panel (Residuals, Monitors, Checks, Messages) |

## Order of work

| Step | Work | Release |
| --- | --- | --- |
| 1 | Skeleton: shell, Cobalt accent, logo, updater, lean CI, release workflow with provenance; a model saves and opens; geometry, fluid and boundary editing | 0.1.0 |
| 2 | Mesher in the window with refinement and the quality report; the checks before solving | 0.1.1 |
| 3 | Solver in the window: live residuals, stop and resume; exact-solution and order-of-accuracy tests; cavity and channel examples | 0.1.2 |
| 4 | Results: contours, arrows, streamlines, probes, line plots, integrated values; the cylinder benchmark | 0.1.3 |
| 5 | Heat and buoyancy; the heated cavity benchmark; advice when a solve fails | 0.1.4 |
| 6 | Examples complete; the verification report in the app and in `docs/VERIFICATION.md`; PDF report; VTU and CSV; help cards; screenshots, README, `docs/site.json` | 0.1.5 |

## If time runs short

Cut in this order: DXF import, the heat-sink example, VTU export, vorticity contours. Never cut the verification report, the boundary checks, mass conservation, or the cylinder and cavity benchmarks.

## Decided here (not in the brief)

- **Boundaries hold edges; edges do not hold boundaries.** A boundary is a named thing (Inlet, Walls) with a list of edges, so one setting covers many edges. An edge with no boundary refuses the solve.
- **Symmetry and slip edges must be straight and horizontal or vertical in 0.1.** Sloping ones need rotated unknowns; later.
- **A fully developed inlet is one straight edge**, and its speed is the mean speed (the peak is 1.5 times it).
- **The turbulence check blocks, with an override.** Above Re 4000 with an inlet, Re 10 000 in a closed box, or Ra 10⁹, the solve is refused in plain words; "Solve anyway" turns it into a warning.
- **Kovasznay flow is checked by its rate of convergence**, not at round-off: its solution is exponential, so no polynomial element reproduces it exactly. Couette and Poiseuille are checked at round-off.
- **The verification report is computed at release time and shipped with the app** (it takes minutes); the app shows that record, and can run the quick cases again on the user's computer.

## What happened (2026-11-12)

0.1.0 shipped as step 1. Steps 2 to 6 were finished together and shipped as one release, 0.1.1: the build budget asks for releases in batches, and version numbers may not jump. Nothing on the cut list was cut.
