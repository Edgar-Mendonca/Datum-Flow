# Datum Flow: verification report

Version **0.1.3**, run on 2026-11-18 (4 threads, linux x86_64; 473 s). **48 of 48 checks pass**.

Every release of Datum Flow solves the problems below, whose answers are known, and compares. Any FAIL blocks the release. The same record is in the app under Help › Verification, and the cases are the test suite of `crates/flow-verify` (`cargo test -p flow-verify --release -- --include-ignored`).

## How to read it

- **Exact solutions** must be reproduced to round-off (1e-8): they lie in the space of the elements.
- **Order of accuracy:** the error must fall at the method's formal rate as the mesh is halved (3 for velocity and temperature, 2 for pressure, within ±0.1), by the method of manufactured solutions (Salari & Knupp 2000) and on Kovasznay's exact solution (1948).
- **Benchmarks** are published high-accuracy results. Each quantity is computed on three meshes (refinement ratio 1.5); the error of the finest must be within twice its Grid Convergence Index (Celik et al. 2008: safety factor 1.25 on the observed order), or within the reference's own stated accuracy where that is wider. Where the three values do not converge steadily the order cannot be established, and the band is three times the largest change between meshes instead.
- **Error** is relative to the reference, except where marked *abs* (orders of accuracy and values whose reference is zero).
- **Validation** compares with laboratory measurements, labelled as such: the backward-facing step of Armaly et al. (1983), within a stated band, since the paper gives no uncertainty.
- **Reference** says how each published number was confirmed: "original paper" (the source names its table or figure) or "secondary sources" (not yet read from the original paper).

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
| Lid-driven cavity, Re 1000 | Stream function at the centre of the main eddy | -0.1189366 | -0.1189355 | 9.5e-6 | 5.5e-4 | **PASS** | Botella & Peyret (1998), spectral solution N = 160, Table 6 *(original paper)* |
| Lid-driven cavity, Re 1000 | Main eddy centre, x | 0.5308 | 0.5308911 | 9.1e-5 abs | 2.0e-3 abs | **PASS** | Botella & Peyret (1998), spectral solution N = 160, Table 6 *(original paper)* |
| Lid-driven cavity, Re 1000 | Main eddy centre, y | 0.5652 | 0.565268 | 6.8e-5 abs | 2.0e-3 abs | **PASS** | Botella & Peyret (1998), spectral solution N = 160, Table 6 *(original paper)* |
| Lid-driven cavity, Re 1000 | u on the vertical centre line at y = 0.9531 | 0.4723329 | 0.4723297 | 6.9e-6 | 6.7e-4 | **PASS** | Botella & Peyret (1998), spectral solution N = 160, Table 9 *(original paper)* |
| Lid-driven cavity, Re 1000 | u on the vertical centre line at y = 0.5 | -0.0620561 | -0.06205929 | 5.1e-5 | 2.6e-4 | **PASS** | Botella & Peyret (1998), spectral solution N = 160, Table 9 *(original paper)* |
| Lid-driven cavity, Re 1000 | u on the vertical centre line at y = 0.1719 | -0.3885691 | -0.3885884 | 5.0e-5 | 7.5e-4 | **PASS** | Botella & Peyret (1998), spectral solution N = 160, Table 9 *(original paper)* |
| Lid-driven cavity, Re 1000 | v on the horizontal centre line at x = 0.9063 | -0.5264392 | -0.5263847 | 1.0e-4 | 2.9e-3 | **PASS** | Botella & Peyret (1998), spectral solution N = 160, Table 10 *(original paper)* |
| Lid-driven cavity, Re 1000 | v on the horizontal centre line at x = 0.5 | 0.0257995 | 0.02579621 | 1.3e-4 | 2.3e-3 | **PASS** | Botella & Peyret (1998), spectral solution N = 160, Table 10 *(original paper)* |
| Lid-driven cavity, Re 1000 | v on the horizontal centre line at x = 0.1563 | 0.3769189 | 0.3768954 | 6.2e-5 | 2.3e-3 | **PASS** | Botella & Peyret (1998), spectral solution N = 160, Table 10 *(original paper)* |
| Heated cavity, Ra 1e3 | Average Nusselt number | 1.118 | 1.11779 | 1.9e-4 | 1.0e-3 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e3 | Largest horizontal velocity on the vertical mid-plane (α/L) | 3.649 | 3.649515 | 1.4e-4 | 1.0e-3 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e3 | Largest vertical velocity on the horizontal mid-plane (α/L) | 3.697 | 3.697506 | 1.4e-4 | 1.0e-3 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e3 | Heat out through the cool wall, over heat in through the warm wall | 1 | 1 | 1.6e-8 | 1.0e-4 | **PASS** | exact (energy is conserved) |
| Heated cavity, Ra 1e4 | Average Nusselt number | 2.243 | 2.244811 | 8.1e-4 | 2.0e-3 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e4 | Largest horizontal velocity on the vertical mid-plane (α/L) | 16.178 | 16.18366 | 3.5e-4 | 2.0e-3 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e4 | Largest vertical velocity on the horizontal mid-plane (α/L) | 19.617 | 19.62809 | 5.7e-4 | 2.0e-3 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e4 | Heat out through the cool wall, over heat in through the warm wall | 1 | 0.9999998 | 2.0e-7 | 1.0e-4 | **PASS** | exact (energy is conserved) |
| Heated cavity, Ra 1e5 | Average Nusselt number | 4.519 | 4.521583 | 5.7e-4 | 3.0e-3 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e5 | Largest horizontal velocity on the vertical mid-plane (α/L) | 34.73 | 34.74351 | 3.9e-4 | 1.2e-2 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e5 | Largest vertical velocity on the horizontal mid-plane (α/L) | 68.59 | 68.64006 | 7.3e-4 | 2.2e-2 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e5 | Heat out through the cool wall, over heat in through the warm wall | 1 | 0.9999997 | 2.6e-7 | 1.0e-4 | **PASS** | exact (energy is conserved) |
| Heated cavity, Ra 1e6 | Average Nusselt number | 8.8 | 8.825118 | 2.9e-3 | 1.0e-2 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e6 | Largest horizontal velocity on the vertical mid-plane (α/L) | 64.63 | 64.83114 | 3.1e-3 | 1.0e-2 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e6 | Largest vertical velocity on the horizontal mid-plane (α/L) | 219.36 | 220.5354 | 5.4e-3 | 1.0e-2 | **PASS** | de Vahl Davis (1983), bench mark solution, Table V *(original paper)* |
| Heated cavity, Ra 1e6 | Heat out through the cool wall, over heat in through the warm wall | 1 | 1.000001 | 6.3e-7 | 1.0e-4 | **PASS** | exact (energy is conserved) |

Notes:

- **Cylinder in a channel, Re 20: Drag coefficient.** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 3.15e-4. Tolerance: twice that band. Coefficient = 2 F / (ρ U² D) with the mean inlet speed. Meshes, coarse to fine (element size → value): 0.0175 → 5.580078; 0.013 → 5.579493; 0.00945 → 5.579581.
- **Cylinder in a channel, Re 20: Lift coefficient.** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 1.95e-2. Tolerance: twice that band.  Meshes, coarse to fine (element size → value): 0.0175 → 0.01063344; 0.013 → 0.0106157; 0.00945 → 0.01068514.
- **Cylinder in a channel, Re 20: Pressure difference, front to back of the cylinder (Pa).** Grid Convergence Index of the fine mesh 1.85e-4 (apparent order 13.43, extrapolated value 0.1174886). Tolerance: twice that band.  Meshes, coarse to fine (element size → value): 0.0175 → 0.1183825; 0.013 → 0.1175216; 0.00945 → 0.117506.
- **Cylinder in a channel, Re 20: Net flow through all boundaries, over the flow in.** On the fine mesh.
- **Lid-driven cavity, Re 1000: Stream function at the centre of the main eddy.** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 2.73e-4. Tolerance: twice that band. Their lid moves in −x (u = −1 on the lid): x, u and the stream function are mirrored here for a lid moving in +x; v keeps its sign. Their table gives +0.1189366 at x = 0.4692. Meshes, coarse to fine (element size → value): 0.0177 → -0.1189279; 0.0124 → -0.1189387; 0.00887 → -0.1189355.
- **Lid-driven cavity, Re 1000: Main eddy centre, x.** Their x = 0.4692, mirrored. Position on the fine mesh; tolerance 0.002 of the side (the reference gives four figures; the stream function is flat near its minimum).
- **Lid-driven cavity, Re 1000: u on the vertical centre line at y = 0.9531.** Grid Convergence Index of the fine mesh 3.36e-4 (apparent order 7.04, extrapolated value 0.4722025). Tolerance: twice that band. Their lid moves in −x (u = −1 on the lid): x, u and the stream function are mirrored here for a lid moving in +x; v keeps its sign. Meshes, coarse to fine (element size → value): 0.0177 → 0.4725727; 0.0124 → 0.4725474; 0.00887 → 0.4723297.
- **Lid-driven cavity, Re 1000: u on the vertical centre line at y = 0.5.** Grid Convergence Index of the fine mesh 1.29e-4 (apparent order 6.32, extrapolated value -0.06205287). Tolerance: twice that band. Their lid moves in −x (u = −1 on the lid): x, u and the stream function are mirrored here for a lid moving in +x; v keeps its sign. Meshes, coarse to fine (element size → value): 0.0177 → -0.06217724; 0.0124 → -0.06207028; 0.00887 → -0.06205929.
- **Lid-driven cavity, Re 1000: u on the vertical centre line at y = 0.1719.** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 3.76e-4. Tolerance: twice that band. Their lid moves in −x (u = −1 on the lid): x, u and the stream function are mirrored here for a lid moving in +x; v keeps its sign. Meshes, coarse to fine (element size → value): 0.0177 → -0.3885498; 0.0124 → -0.3885396; 0.00887 → -0.3885884.
- **Lid-driven cavity, Re 1000: v on the horizontal centre line at x = 0.9063.** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 1.45e-3. Tolerance: twice that band. Their lid moves in −x (u = −1 on the lid): x, u and the stream function are mirrored here for a lid moving in +x; v keeps its sign. Their x = 0.0937. Meshes, coarse to fine (element size → value): 0.0177 → -0.5265582; 0.0124 → -0.5263038; 0.00887 → -0.5263847.
- **Lid-driven cavity, Re 1000: v on the horizontal centre line at x = 0.5.** Grid Convergence Index of the fine mesh 1.17e-3 (apparent order 8.24, extrapolated value 0.02582031). Tolerance: twice that band. Their lid moves in −x (u = −1 on the lid): x, u and the stream function are mirrored here for a lid moving in +x; v keeps its sign. Their x = 0.5000. Meshes, coarse to fine (element size → value): 0.0177 → 0.02575164; 0.0124 → 0.02575493; 0.00887 → 0.02579621.
- **Lid-driven cavity, Re 1000: v on the horizontal centre line at x = 0.1563.** Grid Convergence Index of the fine mesh 1.15e-3 (apparent order 0.60, extrapolated value 0.3772413). Tolerance: twice that band. Their lid moves in −x (u = −1 on the lid): x, u and the stream function are mirrored here for a lid moving in +x; v keeps its sign. Their x = 0.8437. Meshes, coarse to fine (element size → value): 0.0177 → 0.3767173; 0.0124 → 0.3768186; 0.00887 → 0.3768954.
- **Heated cavity, Ra 1e3: Average Nusselt number.** Grid Convergence Index of the fine mesh 3.47e-7 (apparent order 4.99, extrapolated value 1.11779). Tolerance: the reference's own accuracy (0.1%), which is wider than twice that band. The reference is his average over the cavity; here it is the heat through the warm wall, which is the same number for a steady flow (his own wall value differs from his average by less than his error). His estimated error at this Rayleigh number is 0.1% (his Table VII). Meshes, coarse to fine (element size → value): 0.0259 → 1.117786; 0.0187 → 1.117789; 0.0133 → 1.11779.
- **Heated cavity, Ra 1e3: Largest horizontal velocity on the vertical mid-plane (α/L).** Grid Convergence Index of the fine mesh 1.67e-4 (apparent order 1.52, extrapolated value 3.650002). Tolerance: the reference's own accuracy (0.1%), which is wider than twice that band. His estimated error at this Rayleigh number is 0.1% (his Table VII). Meshes, coarse to fine (element size → value): 0.0259 → 3.648657; 0.0187 → 3.649182; 0.0133 → 3.649515.
- **Heated cavity, Ra 1e3: Largest vertical velocity on the horizontal mid-plane (α/L).** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 4.65e-4. Tolerance: the reference's own accuracy (0.1%), which is wider than twice that band. His estimated error at this Rayleigh number is 0.1% (his Table VII). Meshes, coarse to fine (element size → value): 0.0259 → 3.696967; 0.0187 → 3.697541; 0.0133 → 3.697506.
- **Heated cavity, Ra 1e3: Heat out through the cool wall, over heat in through the warm wall.** On the fine mesh.
- **Heated cavity, Ra 1e4: Average Nusselt number.** Grid Convergence Index of the fine mesh 3.81e-6 (apparent order 4.04, extrapolated value 2.244818). Tolerance: the reference's own accuracy (0.2%), which is wider than twice that band. The reference is his average over the cavity; here it is the heat through the warm wall, which is the same number for a steady flow (his own wall value differs from his average by less than his error). His estimated error at this Rayleigh number is 0.2% (his Table VII). Meshes, coarse to fine (element size → value): 0.0259 → 2.244754; 0.0187 → 2.244799; 0.0133 → 2.244811.
- **Heated cavity, Ra 1e4: Largest horizontal velocity on the vertical mid-plane (α/L).** Grid Convergence Index of the fine mesh 3.38e-5 (apparent order 6.63, extrapolated value 16.18322). Tolerance: the reference's own accuracy (0.2%), which is wider than twice that band. His estimated error at this Rayleigh number is 0.2% (his Table VII). Meshes, coarse to fine (element size → value): 0.0259 → 16.19118; 0.0187 → 16.18445; 0.0133 → 16.18366.
- **Heated cavity, Ra 1e4: Largest vertical velocity on the horizontal mid-plane (α/L).** Grid Convergence Index of the fine mesh 1.75e-5 (apparent order 6.27, extrapolated value 19.62837). Tolerance: the reference's own accuracy (0.2%), which is wider than twice that band. His estimated error at this Rayleigh number is 0.2% (his Table VII). Meshes, coarse to fine (element size → value): 0.0259 → 19.62385; 0.0187 → 19.6276; 0.0133 → 19.62809.
- **Heated cavity, Ra 1e4: Heat out through the cool wall, over heat in through the warm wall.** On the fine mesh.
- **Heated cavity, Ra 1e5: Average Nusselt number.** Grid Convergence Index of the fine mesh 9.62e-6 (apparent order 6.12, extrapolated value 4.521618). Tolerance: the reference's own accuracy (0.3%), which is wider than twice that band. The reference is his average over the cavity; here it is the heat through the warm wall, which is the same number for a steady flow (his own wall value differs from his average by less than his error). His estimated error at this Rayleigh number is 0.3% (his Table VII). Meshes, coarse to fine (element size → value): 0.0259 → 4.521067; 0.0187 → 4.52152; 0.0133 → 4.521583.
- **Heated cavity, Ra 1e5: Largest horizontal velocity on the vertical mid-plane (α/L).** Grid Convergence Index of the fine mesh 5.97e-3 (apparent order 0.30, extrapolated value 34.90938). Tolerance: twice that band. His estimated error at this Rayleigh number is 0.3% (his Table VII). Meshes, coarse to fine (element size → value): 0.0259 → 34.67986; 0.0187 → 34.71243; 0.0133 → 34.74351.
- **Heated cavity, Ra 1e5: Largest vertical velocity on the horizontal mid-plane (α/L).** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 1.11e-2. Tolerance: twice that band. His estimated error at this Rayleigh number is 0.3% (his Table VII). Meshes, coarse to fine (element size → value): 0.0259 → 68.53756; 0.0187 → 68.38592; 0.0133 → 68.64006.
- **Heated cavity, Ra 1e5: Heat out through the cool wall, over heat in through the warm wall.** On the fine mesh.
- **Heated cavity, Ra 1e6: Average Nusselt number.** Grid Convergence Index of the fine mesh 2.05e-5 (apparent order 2.31, extrapolated value 8.825263). Tolerance: the reference's own accuracy (1.0%), which is wider than twice that band. The reference is his average over the cavity; here it is the heat through the warm wall, which is the same number for a steady flow (his own wall value differs from his average by less than his error). His estimated error at this Rayleigh number is 1% (his Table VII). Meshes, coarse to fine (element size → value): 0.016 → 8.824686; 0.0118 → 8.824977; 0.00879 → 8.825118.
- **Heated cavity, Ra 1e6: Largest horizontal velocity on the vertical mid-plane (α/L).** The three meshes do not converge steadily, so the band is 3 × the largest change between meshes: 3.93e-3. Tolerance: the reference's own accuracy (1.0%), which is wider than twice that band. His estimated error at this Rayleigh number is 1% (his Table VII). Meshes, coarse to fine (element size → value): 0.016 → 64.88613; 0.0118 → 64.80112; 0.00879 → 64.83114.
- **Heated cavity, Ra 1e6: Largest vertical velocity on the horizontal mid-plane (α/L).** Grid Convergence Index of the fine mesh 4.11e-5 (apparent order 11.77, extrapolated value 220.5282). Tolerance: the reference's own accuracy (1.0%), which is wider than twice that band. His estimated error at this Rayleigh number is 1% (his Table VII). Meshes, coarse to fine (element size → value): 0.016 → 220.9167; 0.0118 → 220.5457; 0.00879 → 220.5354.
- **Heated cavity, Ra 1e6: Heat out through the cool wall, over heat in through the warm wall.** On the fine mesh.

## 4. Validation against experiment

| Case | Quantity | Reference | Datum Flow | Error | Tolerance | | Source |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| Backward-facing step, Re 100 | Reattachment length, in step heights | 3.05 | 2.858633 | 6.3e-2 | 1.0e-1 | **PASS** | Armaly, Durst, Pereira & Schönung (1983), measured, Figure 14 *(original paper (read off the figure))* |
| Backward-facing step, Re 150 | Reattachment length, in step heights | 4.05 | 3.912538 | 3.4e-2 | 1.0e-1 | **PASS** | Armaly, Durst, Pereira & Schönung (1983), measured, Figure 14 *(original paper (read off the figure))* |
| Backward-facing step, Re 300 | Reattachment length, in step heights | 6.62 | 6.642319 | 3.4e-3 | 1.0e-1 | **PASS** | Armaly, Durst, Pereira & Schönung (1983), measured, Figure 14 *(original paper (read off the figure))* |
| Backward-facing step, Re 389 | Reattachment length, in step heights | 8.61 | 8.004619 | 7.0e-2 | 1.0e-1 | **PASS** | Armaly, Durst, Pereira & Schönung (1983), measured, Figure 14 *(original paper (read off the figure))* |

Notes:

- **Backward-facing step, Re 100: Reattachment length, in step heights.** Validation against an experiment. The paper gives no uncertainty for these lengths, so the 10% band is a judgement, set after the first comparison: it covers reading the figure (±0.05 step heights), and the small departures from two-dimensional flow the authors report as Re nears 400. At Re 389 their own 2D computation (Figure 14) gives about 8.05, beside 8.00 here.
- **Backward-facing step, Re 150: Reattachment length, in step heights.** Validation against an experiment. The paper gives no uncertainty for these lengths, so the 10% band is a judgement, set after the first comparison: it covers reading the figure (±0.05 step heights), and the small departures from two-dimensional flow the authors report as Re nears 400. At Re 389 their own 2D computation (Figure 14) gives about 8.05, beside 8.00 here.
- **Backward-facing step, Re 300: Reattachment length, in step heights.** Validation against an experiment. The paper gives no uncertainty for these lengths, so the 10% band is a judgement, set after the first comparison: it covers reading the figure (±0.05 step heights), and the small departures from two-dimensional flow the authors report as Re nears 400. At Re 389 their own 2D computation (Figure 14) gives about 8.05, beside 8.00 here.
- **Backward-facing step, Re 389: Reattachment length, in step heights.** Validation against an experiment. The paper gives no uncertainty for these lengths, so the 10% band is a judgement, set after the first comparison: it covers reading the figure (±0.05 step heights), and the small departures from two-dimensional flow the authors report as Re nears 400. At Re 389 their own 2D computation (Figure 14) gives about 8.05, beside 8.00 here.

## 5. Regression

| Case | Quantity | Reference | Datum Flow | Error | Tolerance | | Source |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| Compared with 0.1.2 | Values unchanged since the last release, of those compared | 35 | 35 | 0 abs | 3.5e1 abs | **PASS** | Datum Flow 0.1.2 |

Notes:

- **Compared with 0.1.2: Values unchanged since the last release, of those compared.** Each value that changed is listed below with its old and new value.

## Not verified

- The cylinder benchmark's reference values (Schäfer & Turek 1996) are still from secondary sources: the benchmark authors' own code and results pages. The pages of the paper that define the case and give its results (549–566) have not been checked.
- Kovasznay's exact solution (1948) is checked for consistency (it satisfies the equations) and against later papers that quote it, not against the 1948 paper.
- The backward-facing step is compared with measurements below Re 400 only. Above that the authors found the experimental flow three-dimensional, which a 2D solver cannot reproduce; the paper states no uncertainty for its reattachment lengths.
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
