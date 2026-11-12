// The bottom panel: Residuals and Monitors (plotted while a solve runs), Checks (what to fix
// before solving) and Messages (what Datum Flow told you). The first three get their content
// from the engine; other parts fill them through `setPanel`.
import { For, Show, JSX, createSignal } from "solid-js";
import { bottom, setBottom, bottomOpen, setBottomOpen, messages, setMessages, BottomTab, out, setSel, setTab } from "./store";
import { Icon } from "./ui";
import { Help } from "./help";
import { cmd } from "./cmds";
import { checkCount, checkBad } from "./simui";

const TABS: [BottomTab, string, string][] = [["residuals", "Residuals", "resid"], ["monitors", "Monitors", "monitor"], ["checks", "Checks", "wizard"], ["messages", "Messages", "messages"]];
/** What a tab shows instead of its empty state (the engine's parts set these). */
const [panels, setPanels] = createSignal<Partial<Record<BottomTab, () => JSX.Element>>>({});
export const setPanel = (t: BottomTab, body: (() => JSX.Element) | null) => setPanels(p => { const n = { ...p }; if (body) n[t] = body; else delete n[t]; return n; });

const Empty = (p: { icon: string; title: string; text: string; children?: JSX.Element }) => <div class="bempty"><Icon n={p.icon} size={26} /><div><b>{p.title}</b><span>{p.text}</span></div>{p.children}</div>;

/** The checks the window can read straight off the model, until the engine's full list is wired. */
const QuickChecks = () => {
  const o = () => out()!;
  const free = () => o().edges.filter(e => !e.boundary).length;
  const items = (): [boolean, string, () => void][] => [
    [!o().geometry_problem, o().geometry_problem ?? "The drawing can be meshed.", () => { setSel({ kind: "geometry" }); setTab("geometry"); }],
    [free() === 0, free() ? `${free()} edge${free() > 1 ? "s have" : " has"} no boundary.` : "Every edge has a boundary.", () => { setSel({ kind: "boundaries" }); setTab("physics"); }],
    [!o().turbulent || o().m.solver.allow_turbulent, o().turbulent ?? "The flow should be laminar.", () => { setSel({ kind: "solver" }); setTab("solve"); }],
  ];
  return <div class="bchecks">
    <For each={items()}>{([ok, text, go]) => <button class="bcheck" classList={{ ok, bad: !ok }} onClick={go}><i>{ok ? "✓" : "!"}</i><span>{text}</span></button>}</For>
    <p class="muted">These three are read straight off the model. <button class="btn sm" onClick={() => cmd("check")}>Run all the checks</button></p>
  </div>;
};

export const Bottom = () => (
  <section class="bottom" classList={{ shut: !bottomOpen() }}>
    <header>
      <For each={TABS}>{([k, l, i]) => <button classList={{ on: bottomOpen() && bottom() === k }} onClick={() => { setBottom(k); setBottomOpen(true); }}><Icon n={i} />{l}{k === "messages" && messages().some(m => m.kind !== "info") && <b class="dot" />}{k === "checks" && checkCount() > 0 && <b class="badge" classList={{ bad: checkBad() }}>{checkCount()}</b>}</button>}</For>
      <Help k="bottom" /><span class="grow" />
      <Show when={bottomOpen() && bottom() === "messages" && messages().length}><button class="btn sm" onClick={() => setMessages([])}>Clear</button></Show>
      <button class="fold" title={bottomOpen() ? "Hide the panel" : "Show the panel"} onClick={() => setBottomOpen(!bottomOpen())}>{bottomOpen() ? "▾" : "▴"}</button>
    </header>
    <Show when={bottomOpen()}>
      <div class="bbody">
        <Show when={panels()[bottom()]} keyed fallback={<>
          <Show when={bottom() === "residuals"}><Empty icon="resid" title="No solve yet" text="While a solve runs, the residuals of momentum, continuity and energy are plotted here. They should fall by many powers of ten." /></Show>
          <Show when={bottom() === "monitors"}><Empty icon="monitor" title="No monitors plotted yet" text="Monitors are numbers watched during a solve, such as the drag on a body or the outlet temperature. Add them under Results; they are plotted here as the solve runs." /></Show>
          <Show when={bottom() === "checks"}><Show when={out()} fallback={<Empty icon="wizard" title="No model" text="Open or start a model first." />}><QuickChecks /></Show></Show>
          <Show when={bottom() === "messages"}>
            <Show when={messages().length} fallback={<Empty icon="messages" title="No messages" text="Notes about opened files, warnings and errors are kept here." />}>
              <table class="tbl msgs"><tbody><For each={[...messages()].reverse()}>{m => <tr classList={{ warn: m.kind === "warn", err: m.kind === "error" }}><td class="muted">{m.at}</td><td>{m.kind === "error" ? "Error" : m.kind === "warn" ? "Note" : "Info"}</td><td>{m.text}</td></tr>}</For></tbody></table>
            </Show>
          </Show>
        </>}>{body => body()}</Show>
      </div>
    </Show>
  </section>
);
