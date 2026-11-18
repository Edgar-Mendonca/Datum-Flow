# Datum Flow: verification report

Version **0.1.2**, run on 2026-11-18 (4 threads, linux x86_64; 446 s). **36 of 36 checks pass**.

Every release of Datum Flow solves the problems below, whose answers are known, and compares. Any FAIL blocks the release. The same record is in the app under Help › Verification, and the cases are the test suite of `crates/flow-verify` (`cargo test -p flow-verify --release -- --include-ignored`).

## How to read it

- **Exact solutions** must be reproduced to round-off (1e-8): they lie in the space of the elements.
- **Order of accuracy:** the error must fall at the method's formal rate as the mesh is halved (3 for velocity and temperature, 2 for pressure, within ±0.1), by the method of manufactured solutions (Salari & Knupp 2000) and on Kovasznay's exact solution (1948).
- **Benchmarks** are published high-accuracy results. Each quantity is computed on three meshes (refinement ratio 1.5); the error of the finest must be within twice its Grid Convergence Index (Celik et al. 2008: safety factor 1.25 on the observed order), or within the reference's own stated accuracy where that is wider. Where the three values do not converge steadily the order cannot be established, and the band is three times the largest change between meshes instead.
- **Error** is relative to the reference, except where marked *abs* (orders of accuracy and values whose reference is zero).
- **Reference** says how each published number was confirmed. "Secondary sources" means it was not read from the original paper.

## 1. Exact solutions

| Case | Quantity | Reference | Datum Flow | Error | Tolerance | | Source |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| Couette flow | Largest error of the velocity, over the plate speed | 0 | 1.873501e-15 | 1.9e-15 abs | 1.0e-8 abs | **PASS** | exact (u = U y / H) |
| Couette flow | Largest pressure (it should be uniform), over ρU² | 0 | 2.151165e-15 | 2.2e-15 abs | 1.0e-8 abs | **PASS** | exact (p = constant) |
| Plane Poiseuille flow | Largest error of the velocity, over the mean speed | 0 | 7.689485e-14 | 7.7e-14 abs | 1.0e-8 abs | **PASS** | exact (u = 6 U (y/H)(1 − y/H), Δp = 12 μ U L / H²) |
| Plane Poiseuille flow | Pressure drop inlet to outlet (Pa) | 0.6012 | 0.6012 | 1.1e-15 | 1.0e-8 | **PASS** | exact (u = 6 U (y/H)(1 − y/H), Δp = 12 μ U L / H²) |
| Plane Poiseuille flow | Friction force on the plates (N per metre depth) | 0.006012 | 0.006012 | 1.3e-15 | 1.0e-8 | **PASS** | exact (it balances the pressure drop: Δp H) |
| Plane Poiseuille flow | Net flow through all boundaries, over the flow in | 0 | 2.168404e-16 | 2.2e-16 abs | 1.0e-10 abs | **PASS** | exact (mass is conserved) |
| Conduction between two walls | Largest error of the temperature, over the temperature difference | 0 | 1.136868e-13 | 1.1e-13 abs | 1.0e-8 abs | **PASS** | exact (T falls in a straight line) |
| Conduction between two walls | Nusselt number on the warm wall | 1 | 1 | 1.9e-13 | 1.0e-8 | **PASS** | exact (Nu = 1 without flow) |
| Kovasznay flow, Re 40 | Velocity error (L2 norm) on the finest mesh | 0 | 4.536736e-5 | 4.5e-5 abs | 1.0e-4 abs | **PASS** | Exact solution of Kovasznay (1948) *(secondary sources)* |

Notes:

- **Couette flow: Largest error of the velocity, over the plate speed.** Re 100 on the gap; a regular mesh of 64 elements.
- **Plane Poiseuille flow: Largest error of the velocity, over the mean speed.** The Channel example: water, Re 500, automatic mesh, fully developed inlet, pressure outlet.
- **Plane Poiseuille flow: Friction force on the plates (N per metre depth).** From the reactions at the wall nodes.
- **Conduction between two walls: Largest error of the temperature, over the temperature difference.** The Heated cavity example with buoyancy off.
- **Conduction between two walls: Nusselt number on the warm wall.** From the reactions of the energy equation.
- **Kovasznay flow, Re 40: Velocity error (L2 norm) on the finest mesh.** Exact, but not a polynomial: the error is small, not round-off. The formula was checked for consistency (it satisfies continuity) and against later papers that quote it.

## 2. Order of accuracy

| Case | Quantity | Reference | Datum Flow | Error | Tolerance | | Source |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| Kovasznay flow, Re 40 | Observed order of accuracy: velocity (L2 norm) | 3 | 3.00284 | 2.8e-3 abs | 1.0e-1 abs | **PASS** | Exact solution of Kovasznay (1948); formal orders 3 and 2 |
| Kovasznay flow, Re 40 | Observed order of accuracy: pressure (L2 norm) | 2 | 2.040704 | 4.1e-2 abs | 1.0e-1 abs | **PASS** | Exact solution of Kovasznay (1948); formal orders 3 and 2 |
| Manufactured solution: flow, heat and buoyancy | Observed order of accuracy: velocity (L2 norm) | 3 | 2.969699 | 3.0e-2 abs | 1.0e-1 abs | **PASS** | Method of manufactured solutions (Salari & Knupp 2000); formal orders 3, 2, 3 |
| Manufactured solution: flow, heat and buoyancy | Observed order of accuracy: pressure (L2 norm) | 2 | 2.037439 | 3.7e-2 abs | 1.0e-1 abs | **PASS** | Method of manufactured solutions (Salari & Knupp 2000); formal orders 3, 2, 3 |
| Manufactured solution: flow, heat and buoyancy | Observed order of accuracy: temperature (L2 norm) | 3 | 2.988451 | 1.2e-2 abs | 1.0e-1 abs | **PASS** | Method of manufactured solutions (Salari & Knupp 2000); formal orders 3, 2, 3 |

Notes:

- **Kovasznay flow, Re 40: Observed order of accuracy: velocity (L2 norm).** An exact solution of the full equations. It is exponential, so it is checked by its rate of convergence rather than at round-off. Meshes, coarse to fine (element size → value): 0.125 → 0.002916046; 0.0625 → 0.000363654; 0.0312 → 4.536736e-5.
- **Kovasznay flow, Re 40: Observed order of accuracy: pressure (L2 norm).** An exact solution of the full equations. It is exponential, so it is checked by its rate of convergence rather than at round-off. Meshes, coarse to fine (element size → value): 0.125 → 0.002490333; 0.0625 → 0.00053637; 0.0312 → 0.0001303621.
- **Manufactured solution: flow, heat and buoyancy: Observed order of accuracy: velocity (L2 norm).** Regular meshes of the unit square, each half the size of the last; the order is taken from the two finest. Meshes, coarse to fine (element size → value): 0.125 → 0.003640752; 0.0625 → 0.0004706877; 0.0312 → 6.008477e-5.
- **Manufactured solution: flow, heat and buoyancy: Observed order of accuracy: pressure (L2 norm).** Regular meshes of the unit square, each half the size of the last; the order is taken from the two finest. Meshes, coarse to fine (element size → value): 0.125 → 0.01039228; 0.0625 → 0.002303076; 0.0312 → 0.0005610195.
- **Manufactured solution: flow, heat and buoyancy: Observed order of accuracy: temperature (L2 norm).** Regular meshes of the unit square, each half the size of the last; the order is taken from the two finest. Meshes, coarse to fine (element size → value): 0.125 → 0.0005724902; 0.0625 → 7.295222e-5; 0.0312 → 9.192319e-6.

## 3. Benchmarks

| Case | Quantity | Reference | Datum Flow | Error | Tolerance | | Source |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| Cylinder in a channel, Re 20 | Drag coefficient | 5.57953523384 | 5.579581 | 8.1e-6 | 6.3e-4 | **PASS** | Schäfer & Turek (1996), case 2D-1; reference values from the benchmark's results page *(secondary sources)* |
| Cylinder in a channel, Re 20 | Lift coefficient | 0.010618948146 | 0.01068514 | 6.2e-3 | 3.9e-2 | **PASS** | Schäfer & Turek (1996), case 2D-1; reference values from the benchmark's results page *(secondary sources)* |
| Cylinder in a channel, Re 20 | Pressure difference, front to back of the cylinder (Pa) | 0.11752016697 | 0.117506 | 1.2e-4 | 3.7e-4 | **PASS** | Schäfer & Turek (1996), case 2D-1; reference values from the benchmark's results page *(secondary sources)* |
| Cylinder in a channel, Re 20 | Net flow through all boundaries, over the flow in | 0 | 8.462066e-16 | 8.5e-16 abs | 1.0e-10 abs | **PASS** | exact (mass is conserved) |
| Lid-driven cavity, Re 1000 | Stream function at the centre of the main eddy | -0.1189366 | -0.1189355 | 9.5e-6 | 5.5e-4 | **PASS** | Botella & Peyret (1998), spectral solution *(secondary sources)* |
| Lid-driven cavity, Re 1000 | Main eddy centre, x | 0.5308 | 0.5308911 | 9.1e-5 abs | 2.0e-3 abs | **PASS** | Botella & Peyret (1998), spectral solution *(secondary sources)* |
| Lid-driven cavity, Re 1000 | Main eddy centre, y | 0.5652 | 0.565268 | 6.8e-5 abs | 2.0e-3 abs | **PASS** | Botella & Peyret (1998), spectral solution *(secondary sources)* |
| Lid-driven cavity, Re 1000 | u on the vertical centre line at y = 0.9531 | 0.4723329 | 0.4723297 | 6.9e-6 | 6.7e-4 | **PASS** | Botella & Peyret (1998), spectral solution *(secondary sources)* |
| Lid-driven cavity, Re 1000 | u on the vertical centre line at y = 0.5 | -0.0620561 | -0.06205929 | 5.1e-5 | 2.6e-4 | **PASS** | Botella & Peyret (1998), spectral solution *(secondary sources)* |
| Lid-driven cavity, Re 1000 | u on the vertical centre line at y = 0.1719 | -0.3885691 | -0.3885884 | 5.0e-5 | 7.5e-4 | **PASS** | Botella & Peyret (1998), spectral solution *(secondary sources)* |
| Lid-driven cavity, Re 1000 | v on the horizontal centre line at x = 0.9063 | -0.5264392 | -0.5263847 | 1.0e-4 | 2.9e-3 | **PASS** | Botella & Peyret (1998), spectral solution *(secondary sources)* |
| Lid-driven cavity, Re 1000 | v on the horizontal centre line at x = 0.5 | 0.0257995 | 0.02579621 | 1.3e-4 | 2.3e-3 | **PASS** | Botella & Peyret (1998), spectral solution *(secondary sources)* |
| Lid-driven cavity, Re 1000 | v on the horizontal centre line at x = 0.1563 | 0.3769189 | 0.3768954 | 6.2e-5 | 2.3e-3 | **PASS** | Botella & Peyret (1998), spectral solution *(secondary sources)* |
| Heated cavity, Ra 1e3 | Average Nusselt number | 1.118 | 1.11779 | 1.9e-4 | 1.0e-2 | **PASS** | de Vahl Davis (1983), benchmark solution *(secondary sources)* |
| Heated cavity, Ra 1e3 | Heat out through the cool wall, over heat in through the warm wall | 1 | 1 | 1.6e-8 | 1.0e-4 | **PASS** | exact (energy is conserved) |
| Heated cavity, Ra 1e4 | Average Nusselt number | 2.243 | 2.244811 | 8.1e-4 | 1.0e-2 | **PASS** | de Vahl Davis (1983), benchmark solution *(secondary sources)* |
| Heated cavity, Ra 1e4 | Heat out through the cool wall, over heat in through the warm wall | 1 | 0.9999998 | 2.0e-7 | 1.0e-4 | **PASS** | exact (energy is conserved) |
| Heated cavity, Ra 1e5 | Average Nusselt number | 4.519 | 4.521583 | 5.7e-4 | 1.0e-2 | **PASS** | de Vahl Davis (1983), benchmark solution *(secondary sources)* |
| Heated cavity, Ra 1e5 | Heat out through the cool wall, over heat in through the warm wall | 1 | 0.9999997 | 2.6e-7 | 1.0e-4 | **PASS** | exact (energy is conserved) |
| Heated cavity, Ra 1e6 | Average Nusselt number | 8.8 | 8.825118 | 2.9e-3 | 1.0e-2 | **PASS** | de Vahl Davis (1983), benchmark solution *(secondary sources)* |
| Heated cavity, Ra 1e6 | Heat out through the cool wall, over heat in through the warm wall | 1 | 1.000001 | 6.3e-7 | 1.0e-4 | **PASS** | exact (energy is conserved) |

Notes:

- **Cylinder in a channel, Re 20: Drag coefficient.** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 3.15e-4. Tolerance: twice that band. Coefficient = 2 F / (ρ U² D) with the mean inlet speed. Meshes, coarse to fine (element size → value): 0.0175 → 5.580078; 0.013 → 5.579493; 0.00945 → 5.579581.
- **Cylinder in a channel, Re 20: Lift coefficient.** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 1.95e-2. Tolerance: twice that band.  Meshes, coarse to fine (element size → value): 0.0175 → 0.01063344; 0.013 → 0.0106157; 0.00945 → 0.01068514.
- **Cylinder in a channel, Re 20: Pressure difference, front to back of the cylinder (Pa).** Grid Convergence Index of the fine mesh 1.85e-4 (apparent order 13.43, extrapolated value 0.1174886). Tolerance: twice that band.  Meshes, coarse to fine (element size → value): 0.0175 → 0.1183825; 0.013 → 0.1175216; 0.00945 → 0.117506.
- **Cylinder in a channel, Re 20: Net flow through all boundaries, over the flow in.** On the fine mesh.
- **Lid-driven cavity, Re 1000: Stream function at the centre of the main eddy.** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 2.73e-4. Tolerance: twice that band. Their lid moves in −x; values are mirrored here for a lid moving in +x. Meshes, coarse to fine (element size → value): 0.0177 → -0.1189279; 0.0124 → -0.1189387; 0.00887 → -0.1189355.
- **Lid-driven cavity, Re 1000: Main eddy centre, x.** Position on the fine mesh; tolerance 0.002 of the side (the reference gives four figures; the stream function is flat near its minimum).
- **Lid-driven cavity, Re 1000: u on the vertical centre line at y = 0.9531.** Grid Convergence Index of the fine mesh 3.36e-4 (apparent order 7.04, extrapolated value 0.4722025). Tolerance: twice that band. Their lid moves in −x; values are mirrored here for a lid moving in +x. Meshes, coarse to fine (element size → value): 0.0177 → 0.4725727; 0.0124 → 0.4725474; 0.00887 → 0.4723297.
- **Lid-driven cavity, Re 1000: u on the vertical centre line at y = 0.5.** Grid Convergence Index of the fine mesh 1.29e-4 (apparent order 6.32, extrapolated value -0.06205287). Tolerance: twice that band. Their lid moves in −x; values are mirrored here for a lid moving in +x. Meshes, coarse to fine (element size → value): 0.0177 → -0.06217724; 0.0124 → -0.06207028; 0.00887 → -0.06205929.
- **Lid-driven cavity, Re 1000: u on the vertical centre line at y = 0.1719.** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 3.76e-4. Tolerance: twice that band. Their lid moves in −x; values are mirrored here for a lid moving in +x. Meshes, coarse to fine (element size → value): 0.0177 → -0.3885498; 0.0124 → -0.3885396; 0.00887 → -0.3885884.
- **Lid-driven cavity, Re 1000: v on the horizontal centre line at x = 0.9063.** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 1.45e-3. Tolerance: twice that band. Their lid moves in −x; values are mirrored here for a lid moving in +x. Meshes, coarse to fine (element size → value): 0.0177 → -0.5265582; 0.0124 → -0.5263038; 0.00887 → -0.5263847.
- **Lid-driven cavity, Re 1000: v on the horizontal centre line at x = 0.5.** Grid Convergence Index of the fine mesh 1.17e-3 (apparent order 8.24, extrapolated value 0.02582031). Tolerance: twice that band. Their lid moves in −x; values are mirrored here for a lid moving in +x. Meshes, coarse to fine (element size → value): 0.0177 → 0.02575164; 0.0124 → 0.02575493; 0.00887 → 0.02579621.
- **Lid-driven cavity, Re 1000: v on the horizontal centre line at x = 0.1563.** Grid Convergence Index of the fine mesh 1.15e-3 (apparent order 0.60, extrapolated value 0.3772413). Tolerance: twice that band. Their lid moves in −x; values are mirrored here for a lid moving in +x. Meshes, coarse to fine (element size → value): 0.0177 → 0.3767173; 0.0124 → 0.3768186; 0.00887 → 0.3768954.
- **Heated cavity, Ra 1e3: Average Nusselt number.** Grid Convergence Index of the fine mesh 3.47e-7 (apparent order 4.99, extrapolated value 1.11779). Tolerance: the reference's own accuracy (1.0%), which is wider than twice that band. The reference is his average over the cavity; here it is the heat through the warm wall, which is the same number for a steady flow. He states his solution is accurate to better than 1% at the highest Rayleigh number. Meshes, coarse to fine (element size → value): 0.0259 → 1.117786; 0.0187 → 1.117789; 0.0133 → 1.11779.
- **Heated cavity, Ra 1e3: Heat out through the cool wall, over heat in through the warm wall.** On the fine mesh.
- **Heated cavity, Ra 1e4: Average Nusselt number.** Grid Convergence Index of the fine mesh 3.81e-6 (apparent order 4.04, extrapolated value 2.244818). Tolerance: the reference's own accuracy (1.0%), which is wider than twice that band. The reference is his average over the cavity; here it is the heat through the warm wall, which is the same number for a steady flow. He states his solution is accurate to better than 1% at the highest Rayleigh number. Meshes, coarse to fine (element size → value): 0.0259 → 2.244754; 0.0187 → 2.244799; 0.0133 → 2.244811.
- **Heated cavity, Ra 1e4: Heat out through the cool wall, over heat in through the warm wall.** On the fine mesh.
- **Heated cavity, Ra 1e5: Average Nusselt number.** Grid Convergence Index of the fine mesh 9.62e-6 (apparent order 6.12, extrapolated value 4.521618). Tolerance: the reference's own accuracy (1.0%), which is wider than twice that band. The reference is his average over the cavity; here it is the heat through the warm wall, which is the same number for a steady flow. He states his solution is accurate to better than 1% at the highest Rayleigh number. Meshes, coarse to fine (element size → value): 0.0259 → 4.521067; 0.0187 → 4.52152; 0.0133 → 4.521583.
- **Heated cavity, Ra 1e5: Heat out through the cool wall, over heat in through the warm wall.** On the fine mesh.
- **Heated cavity, Ra 1e6: Average Nusselt number.** Grid Convergence Index of the fine mesh 2.05e-5 (apparent order 2.31, extrapolated value 8.825263). Tolerance: the reference's own accuracy (1.0%), which is wider than twice that band. The reference is his average over the cavity; here it is the heat through the warm wall, which is the same number for a steady flow. He states his solution is accurate to better than 1% at the highest Rayleigh number. Meshes, coarse to fine (element size → value): 0.016 → 8.824686; 0.0118 → 8.824977; 0.00879 → 8.825118.
- **Heated cavity, Ra 1e6: Heat out through the cool wall, over heat in through the warm wall.** On the fine mesh.

## 5. Regression

| Case | Quantity | Reference | Datum Flow | Error | Tolerance | | Source |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| Compared with 0.1.1 | Values unchanged since the last release, of those compared | 35 | 35 | 0 abs | 3.5e1 abs | **PASS** | Datum Flow 0.1.1 |

Notes:

- **Compared with 0.1.1: Values unchanged since the last release, of those compared.** Each value that changed is listed below with its old and new value.

## Not verified

- Validation against experiment: for the backward-facing step of Armaly et al. (1983) Datum Flow computes reattachment at 2.85 step heights at Re 100, 4.87 step heights at Re 200, 8.00 step heights at Re 389. The measured lengths are given in a figure of the paper, which could not be opened while this record was made, so no verdict is given. Values quoted for it elsewhere are about 3, 5 and 8 step heights.
- Reference values were keyed in from secondary sources (the benchmark authors' own code and results pages as quoted by others, and independent transcriptions that agree with each other), not read from the original papers. They are marked "secondary sources" until checked against the papers.
- Flows that are turbulent, time-dependent, compressible or three-dimensional: this version does not solve them.
- Symmetry and slip edges, heat-flux boundaries, moving walls other than a lid, and the light-oil and US-unit paths have unit tests but no benchmark row.
- The Reynolds and Rayleigh limits of the turbulence warning are rules of thumb, not verified transition points.

## Sources

- M. Schäfer, S. Turek, F. Durst, E. Krause, R. Rannacher (1996), "Benchmark computations of laminar flow around a cylinder", in *Flow Simulation with High-Performance Computers II*, Notes on Numerical Fluid Mechanics, Vieweg, 547–566. Reference values for case 2D-1 from the benchmark's results pages (TU Dortmund).
- O. Botella, R. Peyret (1998), "Benchmark spectral results on the lid-driven cavity flow", *Computers & Fluids* 27(4), 421–433.
- G. de Vahl Davis (1983), "Natural convection of air in a square cavity: a bench mark numerical solution", *International Journal for Numerical Methods in Fluids* 3, 249–264.
- B. F. Armaly, F. Durst, J. C. F. Pereira, B. Schönung (1983), "Experimental and theoretical investigation of backward-facing step flow", *Journal of Fluid Mechanics* 127, 473–496.
- L. I. G. Kovasznay (1948), "Laminar flow behind a two-dimensional grid", *Mathematical Proceedings of the Cambridge Philosophical Society* 44(1), 58–62.
- K. Salari, P. Knupp (2000), *Code Verification by the Method of Manufactured Solutions*, Sandia National Laboratories report SAND2000-1444.
- I. B. Celik, U. Ghia, P. J. Roache, C. J. Freitas, H. Coleman, P. E. Raad (2008), "Procedure for estimation and reporting of uncertainty due to discretization in CFD applications", *Journal of Fluids Engineering* 130(7), 078001.
