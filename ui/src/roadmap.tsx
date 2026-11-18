// File › Roadmap: where Datum Flow is heading. The list is in docs/site.json (also read by the
// Datum family site), one line per three-number release.
import { For } from "solid-js";
import { C } from "./cmds";

type Stage = { v: string; title: string; when: string; items: string[] };
const FALLBACK: Stage[] = [{ v: "1.0.0", title: "The roadmap", when: "Goal", items: ["The full roadmap is in docs/site.json and on the family site."] }];
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
