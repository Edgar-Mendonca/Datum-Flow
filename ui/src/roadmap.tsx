// File › Roadmap: where Datum Flow is heading. The list is in docs/site.json (also read by the
// Datum family site); until that file exists, a short list from the plan is shown.
import { For } from "solid-js";
import { C } from "./cmds";

type Stage = { v: string; title: string; when: string; items: string[] };
const FALLBACK: Stage[] = [
  { v: "0.1", title: "2D steady laminar flow", when: "Next", items: ["Draw the geometry, set the flow, mesh and solve", "Heat transfer and natural convection", "Contours, streamlines, probes and integrated values", "A verification report with every release"] },
  { v: "0.2", title: "Transient", when: "Soon", items: ["Time-dependent flow: vortex shedding behind a cylinder", "Stabilisation for convection-dominated flow", "Animations and monitors over time"] },
  { v: "0.3", title: "Walls and solids", when: "Soon", items: ["Boundary-layer meshing with a wall-distance helper", "Heat through solid parts and the fluid together", "Axisymmetric flow: pipes and nozzles"] },
  { v: "0.4", title: "Bigger models", when: "Later", items: ["Iterative solvers with preconditioners", "Parallel assembly and larger meshes", "Parameter sweeps"] },
  { v: "0.5", title: "The 3D decision", when: "Later", items: ["First 3D laminar cases", "3D meshing"] },
  { v: "0.6", title: "External solvers (optional)", when: "Later", items: ["Write case files for SU2 and OpenFOAM, run them as separate programs you install, and read their results back"] },
  { v: "0.7", title: "Turbulence", when: "Later", items: ["Wall distance, then one-equation and two-equation models, each verified against published cases"] },
  { v: "0.8", title: "CAD and family links", when: "Later", items: ["STEP import", "Geometry from Datum Layout, line data from Datum Schema, pressures and temperatures into Datum Mesh"] },
  { v: "0.9", title: "Teaching pack", when: "Later", items: ["Guided lessons on the classic cases, each with its theory and reference data"] },
  { v: "1.0", title: "Complete", when: "Goal", items: ["Full verification and validation report", "User guide", "Signed installer"] },
];
// Present at build time or not: a missing docs/site.json must not stop the build.
const found = import.meta.glob("../../docs/site.json", { eager: true }) as Record<string, { default?: { roadmap?: Stage[] }; roadmap?: Stage[] }>;
const site = Object.values(found)[0];
export const ROADMAP: Stage[] = (site?.default?.roadmap ?? site?.roadmap ?? []).length ? (site.default?.roadmap ?? site.roadmap)! : FALLBACK;

export const RoadmapPage = () => (
  <div class="roadmap">
    <h1>Roadmap</h1>
    <p class="muted">What Datum Flow will do, and when. Releases are small and arrive through File › Updates. Ideas and bug reports are welcome on GitHub.</p>
    <p><button class="btn" onClick={() => C.link("/issues/new")}>Suggest a feature or report an issue</button> <button class="btn" onClick={() => C.link("/releases")}>Every release so far</button></p>
    <div class="rmlist">
      <For each={ROADMAP}>{s => (
        <section class="rmstage" classList={{ next: s.when === "Next" }}>
          <header><span class="rmv">{s.v}</span><b>{s.title}</b><span class="pill">{s.when}</span></header>
          <ul><For each={s.items}>{i => <li>{i}</li>}</For></ul>
        </section>
      )}</For>
    </div>
  </div>
);
