// The 2D view: the drawing (shapes, holes cut out, edges coloured by their boundary), the snap
// grid, and the drawing tools. One world→screen transform (`xf`) is shared with everything that
// draws into the view: other parts add `layers` (the mesh, results, probes) and are drawn in order.
import { createSignal, createEffect, onMount, onCleanup, Show } from "solid-js";
import { Geom, Pt, Shape, EdgeRef, Boundary, num } from "./api";
import { out, act, sel, setSel, tool, setTool, asHole, pickFor, setPickFor, showGrid, showNames, snapOn, themeTick, openMenu, Item, boundaryOf, toU, ulabel, toast, sameEdge, KIND_NAME } from "./store";

// ---------- the transform (world metres, y up → screen pixels, y down)
/** The world point at the middle of the view, and pixels per metre. */
export const [xf, setXf] = createSignal({ cx: 0.5, cy: 0.5, k: 400 });
/** The view's size in CSS pixels. */
export const [viewSize, setViewSize] = createSignal({ w: 800, h: 500 });
export const toScreen = (p: Pt | readonly [number, number]): Pt => { const t = xf(), s = viewSize(); return [s.w / 2 + (p[0] - t.cx) * t.k, s.h / 2 - (p[1] - t.cy) * t.k]; };
export const toWorld = (sx: number, sy: number): Pt => { const t = xf(), s = viewSize(); return [t.cx + (sx - s.w / 2) / t.k, t.cy - (sy - s.h / 2) / t.k]; };
/** The pointer's place in the model (m), for the status bar; null when it is outside the view. */
export const [cursor, setCursor] = createSignal<Pt | null>(null);

/** Show a box of the model (m) with a margin; the whole model when none is given. */
export const fit = (box?: [number, number, number, number] | null) => {
  const b = box ?? out()?.bounds ?? allBounds();
  const s = viewSize();
  if (!b || !(b[2] > b[0]) || !(b[3] > b[1])) return setXf({ cx: 0.5, cy: 0.5, k: Math.min(s.w, s.h) * 0.8 });
  const k = Math.min((s.w - 90) / (b[2] - b[0]), (s.h - 90) / (b[3] - b[1]));
  setXf({ cx: (b[0] + b[2]) / 2, cy: (b[1] + b[3]) / 2, k: Math.max(1e-6, k) });
};
/** Zoom by a factor about a screen point (the middle of the view when none is given). */
export const zoomBy = (f: number, sx?: number, sy?: number) => {
  const s = viewSize(), x = sx ?? s.w / 2, y = sy ?? s.h / 2, w = toWorld(x, y), t = xf();
  const k = Math.min(1e9, Math.max(1e-3, t.k * f));
  setXf({ k, cx: w[0] - (x - s.w / 2) / k, cy: w[1] + (y - s.h / 2) / k });
};

// ---------- layers: what other parts draw into the view
export interface Env { toScreen: typeof toScreen; toWorld: typeof toWorld; k: number; w: number; h: number; css: (name: string) => string }
export interface Layer {
  id: string;
  /** Below 0: under the drawing's outlines (filled results). Above 0: over them (mesh lines, probes). */
  z: number;
  draw: (ctx: CanvasRenderingContext2D, env: Env) => void;
}
const [layerList, setLayerList] = createSignal<Layer[]>([]);
const [tick, setTick] = createSignal(0);
/** Add or replace a layer; it is drawn on every repaint until removed. */
export const setLayer = (l: Layer) => setLayerList(v => [...v.filter(x => x.id !== l.id), l].sort((a, b) => a.z - b.z));
export const removeLayer = (id: string) => setLayerList(v => v.filter(x => x.id !== id));
/** Ask for a repaint (a layer's data changed). */
export const redraw = () => setTick(t => t + 1);
/** Draw the region's fill (off when filled results cover it). */
export const [regionFill, setRegionFill] = createSignal(true);
/** A part that wants the next clicks in the view (placing probes, drawing a plot line): return
 *  true when the click was used. Null gives the view back to the drawing tools. */
export type ClickHook = (world: Pt, e: MouseEvent, phase: "down" | "move" | "up") => boolean;
let clickHook: ClickHook | null = null;
export const setClickHook = (h: ClickHook | null) => { clickHook = h; };
/** The view as a PNG data URL (for Export › Picture and printing). */
export const viewPicture = (): string | null => canvasEl?.toDataURL("image/png") ?? null;
let canvasEl: HTMLCanvasElement | null = null;

// ---------- geometry for drawing
/** The arc of a bulged side p→q: centre, radius, start angle and swept angle. */
const arc = (p: Pt, q: Pt, bulge: number) => {
  const th = 4 * Math.atan(bulge), dx = q[0] - p[0], dy = q[1] - p[1], c = Math.hypot(dx, dy);
  const r = c / Math.abs(2 * Math.sin(th / 2)), d = c * 0.5 / Math.tan(th / 2);
  const cen: Pt = [(p[0] + q[0]) / 2 - dy / c * d, (p[1] + q[1]) / 2 + dx / c * d];
  return { cen, r, a0: Math.atan2(p[1] - cen[1], p[0] - cen[0]), th };
};
/** Points along edge `i` of an outline (world), enough for a smooth curve. */
export const edgePoints = (g: Geom, i: number): Pt[] => {
  if (g.type === "circle") return Array.from({ length: 73 }, (_, k) => [g.c[0] + g.r * Math.cos(k / 72 * 2 * Math.PI), g.c[1] + g.r * Math.sin(k / 72 * 2 * Math.PI)] as Pt);
  const n = g.pts.length, p = g.pts[i % n], q = g.pts[(i + 1) % n], b = g.bulge[i] ?? 0;
  if (!b || !isFinite(b)) return [p, q];
  const a = arc(p, q, b), m = Math.max(4, Math.ceil(Math.abs(a.th) / (2 * Math.PI) * 72));
  return Array.from({ length: m + 1 }, (_, k) => k === 0 ? p : k === m ? q : [a.cen[0] + a.r * Math.cos(a.a0 + a.th * k / m), a.cen[1] + a.r * Math.sin(a.a0 + a.th * k / m)] as Pt);
};
export const edgeCount = (g: Geom) => (g.type === "circle" ? 1 : g.pts.length);
const outline = (g: Geom): Pt[] => Array.from({ length: edgeCount(g) }, (_, i) => edgePoints(g, i).slice(0, -1)).flat();
const inside = (poly: Pt[], p: Pt) => {
  let c = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const a = poly[i], b = poly[j];
    if ((a[1] > p[1]) !== (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]) c = !c;
  }
  return c;
};
const allBounds = (): [number, number, number, number] | null => {
  const pts = (out()?.m.shapes ?? []).flatMap(s => outline(s.geom));
  return pts.length ? [Math.min(...pts.map(p => p[0])), Math.min(...pts.map(p => p[1])), Math.max(...pts.map(p => p[0])), Math.max(...pts.map(p => p[1]))] : null;
};
const distSeg = (p: Pt, a: Pt, b: Pt) => {
  const dx = b[0] - a[0], dy = b[1] - a[1], l2 = dx * dx + dy * dy, t = l2 ? Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2)) : 0;
  return Math.hypot(a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
};
/** A length in the model's display unit, for the size readout while drawing. */
const len = (v: number) => `${num(toU("len", v), 4)} ${ulabel("len")}`;

/** The colour of an edge: by the kind of its boundary; red when it has none. */
const edgeColour = (b: Boundary | undefined, css: (n: string) => string) =>
  !b ? css("--crit") : b.kind.type === "inlet" ? css("--blue") : b.kind.type === "outlet" ? css("--amber") : b.kind.type === "symmetry" ? css("--green") : css("--text");

export const Canvas = () => {
  let wrap!: HTMLDivElement, cv!: HTMLCanvasElement;
  const fills = document.createElement("canvas");
  // What is being drawn or dragged, in world coordinates.
  const [draft, setDraft] = createSignal<{ kind: "rect"; a: Pt; b: Pt } | { kind: "circle"; a: Pt; b: Pt } | { kind: "poly"; pts: Pt[]; at: Pt } | null>(null);
  const [drag, setDrag] = createSignal<{ id: number; dx: number; dy: number } | { id: number; vertex: number; at: Pt } | null>(null);
  const [hover, setHover] = createSignal<EdgeRef | null>(null);
  const [space, setSpace] = createSignal(false);
  let cssCache: Record<string, string> = {}, cssTick = -1;
  const css = (name: string) => {
    if (cssTick !== themeTick()) { cssCache = {}; cssTick = themeTick(); }
    return (cssCache[name] ??= getComputedStyle(document.documentElement).getPropertyValue(name).trim() || "#888");
  };
  const snap = (p: Pt, e?: MouseEvent): Pt => {
    const g = out()?.m.grid ?? 0;
    return snapOn() && g > 0 && !e?.altKey ? [Math.round(p[0] / g) * g, Math.round(p[1] / g) * g] : p;
  };
  const at = (e: MouseEvent): Pt => { const r = cv.getBoundingClientRect(); return toWorld(e.clientX - r.left, e.clientY - r.top); };

  /** The edge or shape under a world point (edges first, within a few pixels). */
  const hit = (w: Pt): { edge?: EdgeRef; shape?: Shape } => {
    const m = out()?.m; if (!m) return {};
    const tol = 6 / xf().k;
    let best: { e: EdgeRef; d: number } | null = null;
    for (const s of m.shapes) for (let i = 0; i < edgeCount(s.geom); i++) {
      const pts = edgePoints(s.geom, i);
      for (let k = 0; k + 1 < pts.length; k++) { const d = distSeg(w, pts[k], pts[k + 1]); if (d < tol && (!best || d < best.d)) best = { e: { shape: s.id, edge: i }, d }; }
    }
    if (best) return { edge: best.e, shape: m.shapes.find(s => s.id === best!.e.shape) };
    // Bodies (holes) lie on top of the region they are cut from; the last drawn wins.
    const under = [...m.shapes].reverse().filter(s => inside(outline(s.geom), w));
    return { shape: under.find(s => s.hole) ?? under[0] };
  };

  const paint = () => {
    const o = out(), size = viewSize(), dpr = window.devicePixelRatio || 1, t = xf();
    tick(); themeTick(); showGrid(); showNames(); sel(); hover(); draft(); drag(); regionFill(); pickFor(); tool();
    const ls = layerList();
    if (cv.width !== Math.round(size.w * dpr) || cv.height !== Math.round(size.h * dpr)) { cv.width = fills.width = Math.round(size.w * dpr); cv.height = fills.height = Math.round(size.h * dpr); }
    const ctx = cv.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = css("--bg2"); ctx.fillRect(0, 0, size.w, size.h);
    const env: Env = { toScreen, toWorld, k: t.k, w: size.w, h: size.h, css };
    const path = (c: CanvasRenderingContext2D, pts: Pt[], close = false, off: Pt = [0, 0]) => {
      pts.forEach((p, i) => { const s = toScreen([p[0] + off[0], p[1] + off[1]]); if (i) c.lineTo(s[0], s[1]); else c.moveTo(s[0], s[1]); });
      if (close) c.closePath();
    };
    const moved = (s: Shape): Pt => { const d = drag(); return d && "dx" in d && d.id === s.id ? [d.dx, d.dy] : [0, 0]; };
    const geomOf = (s: Shape): Geom => {
      const d = drag();
      if (d && "vertex" in d && d.id === s.id && s.geom.type === "poly") return { ...s.geom, pts: s.geom.pts.map((p, i) => (i === d.vertex ? d.at : p)) };
      return s.geom;
    };
    // The grid: its step doubled until lines are at least 9 px apart.
    if (showGrid() && o) {
      let g = o.m.grid > 0 ? o.m.grid : 0.01;
      while (g * t.k < 9) g *= g * t.k < 1 ? 10 : (String(Math.round(g / 10 ** Math.floor(Math.log10(g)))) === "2" ? 2.5 : 2);
      const a = toWorld(0, size.h), b = toWorld(size.w, 0);
      ctx.strokeStyle = css("--line2"); ctx.lineWidth = 1; ctx.beginPath();
      for (let x = Math.ceil(a[0] / g) * g; x <= b[0]; x += g) { const sx = Math.round(toScreen([x, 0])[0]) + 0.5; ctx.moveTo(sx, 0); ctx.lineTo(sx, size.h); }
      for (let y = Math.ceil(a[1] / g) * g; y <= b[1]; y += g) { const sy = Math.round(toScreen([0, y])[1]) + 0.5; ctx.moveTo(0, sy); ctx.lineTo(size.w, sy); }
      ctx.stroke();
      // The axes through the origin.
      const o0 = toScreen([0, 0]);
      ctx.strokeStyle = css("--line3"); ctx.beginPath(); ctx.moveTo(Math.round(o0[0]) + 0.5, 0); ctx.lineTo(Math.round(o0[0]) + 0.5, size.h); ctx.moveTo(0, Math.round(o0[1]) + 0.5); ctx.lineTo(size.w, Math.round(o0[1]) + 0.5); ctx.stroke();
    }
    if (o) {
      // The flow region: adding shapes filled, subtracting shapes cut out (on a spare canvas, so
      // the grid shows through the holes).
      if (regionFill()) {
        const f = fills.getContext("2d")!;
        f.setTransform(dpr, 0, 0, dpr, 0, 0); f.clearRect(0, 0, size.w, size.h);
        f.globalCompositeOperation = "source-over"; f.fillStyle = css("--pale");
        for (const s of o.m.shapes) if (!s.hole) { f.beginPath(); path(f, outline(geomOf(s)), true, moved(s)); f.fill(); }
        f.globalCompositeOperation = "destination-out";
        for (const s of o.m.shapes) if (s.hole) { f.beginPath(); path(f, outline(geomOf(s)), true, moved(s)); f.fill(); }
        ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.drawImage(fills, 0, 0); ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      }
      for (const l of ls) if (l.z < 0) { ctx.save(); l.draw(ctx, env); ctx.restore(); }
      // Edges, coloured by their boundary.
      const s0 = sel(), picking = tool() === "pick" ? pickFor() : null;
      ctx.lineJoin = "round"; ctx.lineCap = "round"; ctx.font = `500 10px ${css("--mono")}`; ctx.textBaseline = "middle";
      const labels: { x: number; y: number; text: string; colour: string }[] = [];
      for (const s of o.m.shapes) {
        const g = geomOf(s), off = moved(s);
        for (let i = 0; i < edgeCount(g); i++) {
          const info = o.edges.find(e => e.shape === s.id && e.edge === i), b = info?.boundary ? boundaryOf(info.boundary) : undefined;
          const pts = edgePoints(g, i), colour = edgeColour(b, css), ref = { shape: s.id, edge: i };
          const on = (s0.kind === "edge" && sameEdge(s0, ref)) || (s0.kind === "boundary" && b?.id === s0.id) || (picking !== null && b?.id === picking);
          const hot = hover() && sameEdge(hover()!, ref);
          if (on || hot) { ctx.strokeStyle = css(on ? "--sel2" : "--sel"); ctx.lineWidth = 9; ctx.setLineDash([]); ctx.beginPath(); path(ctx, pts, false, off); ctx.stroke(); }
          ctx.strokeStyle = colour; ctx.lineWidth = s0.kind === "shape" && s0.id === s.id ? 2.6 : 1.8;
          ctx.setLineDash(!b ? [6, 4] : b.kind.type === "symmetry" ? [10, 4, 2, 4] : []);
          ctx.beginPath(); path(ctx, pts, false, off); ctx.stroke(); ctx.setLineDash([]);
          const mid = pts[Math.floor(pts.length / 2)], prev = pts[Math.max(0, Math.floor(pts.length / 2) - 1)];
          const a = toScreen([mid[0] + off[0], mid[1] + off[1]]), p0 = toScreen([prev[0] + off[0], prev[1] + off[1]]);
          const m2: Pt = pts.length === 2 ? [(a[0] + p0[0]) / 2, (a[1] + p0[1]) / 2] : a;
          const screenLen = pts.slice(1).reduce((acc, p, k) => acc + Math.hypot(p[0] - pts[k][0], p[1] - pts[k][1]), 0) * t.k;
          const text = b?.name ?? info?.name ?? "";
          if (showNames() && text && screenLen > ctx.measureText(text).width + 24) labels.push({ x: m2[0], y: m2[1], text, colour });
          // Inlets and outlets: small arrows across the edge, into or out of the region.
          if (b && (b.kind.type === "inlet" || b.kind.type === "outlet") && pts.length === 2 && screenLen > 40) {
            const q0 = toScreen(pts[0]), q1 = toScreen(pts[1]), ex = (q1[0] - q0[0]) / screenLen, ey = (q1[1] - q0[1]) / screenLen;
            // The side the region is on: test a point a little to the left of the edge.
            const wl: Pt = [(pts[0][0] + pts[1][0]) / 2 - (pts[1][1] - pts[0][1]) * 1e-3, (pts[0][1] + pts[1][1]) / 2 + (pts[1][0] - pts[0][0]) * 1e-3];
            const inLeft = o.m.shapes.some(x => !x.hole && inside(outline(x.geom), wl)) && !o.m.shapes.some(x => x.hole && inside(outline(x.geom), wl));
            // On screen y points down, so the left normal of the world edge is (ey, −ex) here.
            const sgn = (inLeft ? 1 : -1) * (b.kind.type === "inlet" ? 1 : -1), nx = ey * sgn, ny = -ex * sgn;
            const count = Math.max(2, Math.min(7, Math.floor(screenLen / 34)));
            ctx.strokeStyle = colour; ctx.lineWidth = 1.4; ctx.beginPath();
            for (let j = 0; j < count; j++) {
              const f2 = (j + 0.5) / count, bx = q0[0] + (q1[0] - q0[0]) * f2, by = q0[1] + (q1[1] - q0[1]) * f2;
              // n points into the region for an inlet and out of it for an outlet; the arrow sits outside the region.
              const tail = b.kind.type === "inlet" ? [bx - nx * 16, by - ny * 16] : [bx + nx * 3, by + ny * 3], tip = b.kind.type === "inlet" ? [bx - nx * 3, by - ny * 3] : [bx + nx * 16, by + ny * 16];
              const ux = (tip[0] - tail[0]), uy = (tip[1] - tail[1]), ul = Math.hypot(ux, uy) || 1;
              ctx.moveTo(tail[0], tail[1]); ctx.lineTo(tip[0], tip[1]);
              ctx.moveTo(tip[0] - (ux / ul) * 5 - (uy / ul) * 3, tip[1] - (uy / ul) * 5 + (ux / ul) * 3); ctx.lineTo(tip[0], tip[1]); ctx.lineTo(tip[0] - (ux / ul) * 5 + (uy / ul) * 3, tip[1] - (uy / ul) * 5 - (ux / ul) * 3);
            }
            ctx.stroke();
          }
        }
        // Corner handles of the selected polygon.
        if (s0.kind === "shape" && s0.id === s.id && g.type === "poly" && tool() === "select") {
          ctx.fillStyle = css("--bg2"); ctx.strokeStyle = css("--blue"); ctx.lineWidth = 1.5;
          for (const p of g.pts) { const q = toScreen([p[0] + off[0], p[1] + off[1]]); ctx.beginPath(); ctx.rect(Math.round(q[0]) - 3.5, Math.round(q[1]) - 3.5, 7, 7); ctx.fill(); ctx.stroke(); }
        }
      }
      for (const l of ls) if (l.z >= 0) { ctx.save(); l.draw(ctx, env); ctx.restore(); }
      for (const l of labels) {
        const w = ctx.measureText(l.text).width + 8;
        ctx.fillStyle = css("--bg2"); ctx.globalAlpha = 0.88; ctx.fillRect(l.x - w / 2, l.y - 8, w, 16); ctx.globalAlpha = 1;
        ctx.fillStyle = l.colour; ctx.textAlign = "center"; ctx.fillText(l.text.toUpperCase(), l.x, l.y + 0.5);
      }
    }
    // What is being drawn.
    const d = draft();
    if (d) {
      ctx.strokeStyle = css("--blue"); ctx.fillStyle = css("--sel"); ctx.lineWidth = 1.5; ctx.setLineDash(asHole() ? [5, 4] : []); ctx.globalAlpha = 0.9;
      ctx.beginPath();
      let note = "";
      if (d.kind === "rect") { path(ctx, [d.a, [d.b[0], d.a[1]], d.b, [d.a[0], d.b[1]]], true); note = `${len(Math.abs(d.b[0] - d.a[0]))} × ${len(Math.abs(d.b[1] - d.a[1]))}`; }
      else if (d.kind === "circle") { const c = toScreen(d.a), r = Math.hypot(d.b[0] - d.a[0], d.b[1] - d.a[1]); ctx.arc(c[0], c[1], r * t.k, 0, 2 * Math.PI); note = `radius ${len(r)}`; }
      else { path(ctx, [...d.pts, d.at]); note = d.pts.length ? len(Math.hypot(d.at[0] - d.pts[d.pts.length - 1][0], d.at[1] - d.pts[d.pts.length - 1][1])) : ""; }
      if (d.kind !== "poly") { ctx.globalAlpha = 0.35; ctx.fill(); ctx.globalAlpha = 0.9; }
      ctx.stroke(); ctx.setLineDash([]); ctx.globalAlpha = 1;
      if (d.kind === "poly") { ctx.fillStyle = css("--blue"); for (const p of d.pts) { const q = toScreen(p); ctx.fillRect(q[0] - 2.5, q[1] - 2.5, 5, 5); } }
      const tip = toScreen(d.kind === "poly" ? d.at : d.b);
      if (note) { ctx.font = `500 11px ${css("--mono")}`; ctx.textAlign = "left"; ctx.fillStyle = css("--text"); ctx.fillText(note, tip[0] + 12, tip[1] - 12); }
    }
  };

  // ---------- pointer
  const finishPoly = async () => {
    const d = draft(); setDraft(null);
    if (!d || d.kind !== "poly") return;
    const pts = d.pts.filter((p, i) => i === 0 || Math.hypot(p[0] - d.pts[i - 1][0], p[1] - d.pts[i - 1][1]) > 1e-12);
    if (pts.length < 3) return toast("A polygon needs at least three corners.");
    await addShape({ type: "poly", pts, bulge: pts.map(() => 0) });
  };
  const addShape = async (geom: Geom) => {
    if (await act({ op: "add_shape", name: "", hole: asHole(), geom })) { const s = out()!.m.shapes; setSel({ kind: "shape", id: s[s.length - 1].id }); setTool("select"); }
  };
  const togglePick = async (e: EdgeRef) => {
    const b = pickFor() !== null ? boundaryOf(pickFor()!) : undefined;
    if (!b) return;
    const has = b.edges.some(x => sameEdge(x, e));
    await act({ op: "set_boundary", boundary: { ...b, edges: has ? b.edges.filter(x => !sameEdge(x, e)) : [...b.edges, e] } });
  };
  let pan: { x: number; y: number; cx: number; cy: number } | null = null, press: { w: Pt; x: number; y: number; shape?: Shape; vertex?: number } | null = null;
  const down = (e: MouseEvent) => {
    wrap.focus();
    const w = at(e);
    if (e.button === 1 || e.button === 2 || tool() === "pan" || space()) { e.preventDefault(); pan = { x: e.clientX, y: e.clientY, cx: xf().cx, cy: xf().cy }; return; }
    if (e.button !== 0 || !out()) return;
    if (clickHook?.(w, e, "down")) return;
    const p = snap(w, e), tl = tool();
    if (tl === "rect") return setDraft({ kind: "rect", a: p, b: p });
    if (tl === "circle") return setDraft({ kind: "circle", a: p, b: p });
    if (tl === "poly") { const d = draft(); return setDraft(d && d.kind === "poly" ? { kind: "poly", pts: [...d.pts, p], at: p } : { kind: "poly", pts: [p], at: p }); }
    const h = hit(w);
    if (tl === "pick") { if (h.edge) togglePick(h.edge); return; }
    // A corner handle of the selected polygon.
    const s0 = sel(), cur = s0.kind === "shape" ? out()!.m.shapes.find(s => s.id === s0.id) : undefined;
    if (cur?.geom.type === "poly") {
      const v = cur.geom.pts.findIndex(q => { const a = toScreen(q), b = toScreen(w); return Math.hypot(a[0] - b[0], a[1] - b[1]) < 7; });
      if (v >= 0) { press = { w, x: e.clientX, y: e.clientY, shape: cur, vertex: v }; return; }
    }
    if (h.edge) { setSel({ kind: "edge", ...h.edge }); press = { w, x: e.clientX, y: e.clientY, shape: h.shape }; return; }
    if (h.shape) { setSel({ kind: "shape", id: h.shape.id }); press = { w, x: e.clientX, y: e.clientY, shape: h.shape }; return; }
    setSel({ kind: "geometry" });
  };
  const move = (e: MouseEvent) => {
    const r = cv.getBoundingClientRect(), inView = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    const w = at(e);
    setCursor(inView ? w : null);
    if (pan) { const t = xf(); setXf({ k: t.k, cx: pan.cx - (e.clientX - pan.x) / t.k, cy: pan.cy + (e.clientY - pan.y) / t.k }); return; }
    if (clickHook?.(w, e, "move")) return;
    const d = draft(), p = snap(w, e);
    if (d && d.kind !== "poly" && e.buttons & 1) return setDraft({ ...d, b: p });
    if (d && d.kind === "poly") return setDraft({ ...d, at: p });
    if (press?.shape && e.buttons & 1) {
      if (Math.hypot(e.clientX - press.x, e.clientY - press.y) < 4 && !drag()) return;
      if (press.vertex !== undefined) return setDrag({ id: press.shape.id, vertex: press.vertex, at: p });
      const g = out()?.m.grid ?? 0, dx = w[0] - press.w[0], dy = w[1] - press.w[1], q = (v: number) => (snapOn() && g > 0 && !e.altKey ? Math.round(v / g) * g : v);
      return setDrag({ id: press.shape.id, dx: q(dx), dy: q(dy) });
    }
    if (inView && (tool() === "select" || tool() === "pick")) { const h = hit(w).edge ?? null; if (JSON.stringify(h) !== JSON.stringify(hover())) setHover(h); } else if (hover()) setHover(null);
  };
  const up = async (e: MouseEvent) => {
    if (pan) { pan = null; return; }
    if (clickHook?.(at(e), e, "up")) return;
    const d = draft();
    if (d && d.kind === "rect") { setDraft(null); if (Math.abs(d.b[0] - d.a[0]) > 0 && Math.abs(d.b[1] - d.a[1]) > 0) await addShape({ type: "poly", pts: [[Math.min(d.a[0], d.b[0]), Math.min(d.a[1], d.b[1])], [Math.max(d.a[0], d.b[0]), Math.min(d.a[1], d.b[1])], [Math.max(d.a[0], d.b[0]), Math.max(d.a[1], d.b[1])], [Math.min(d.a[0], d.b[0]), Math.max(d.a[1], d.b[1])]], bulge: [0, 0, 0, 0] }); }
    if (d && d.kind === "circle") { setDraft(null); const r = Math.hypot(d.b[0] - d.a[0], d.b[1] - d.a[1]); if (r > 0) await addShape({ type: "circle", c: d.a, r }); }
    const g = drag(); press = null;
    if (g) {
      setDrag(null);
      if ("vertex" in g) { const s = out()?.m.shapes.find(x => x.id === g.id); if (s?.geom.type === "poly") await act({ op: "set_geom", id: g.id, geom: { ...s.geom, pts: s.geom.pts.map((p, i) => (i === g.vertex ? g.at : p)) } }); }
      else if (g.dx || g.dy) { await act({ op: "move_shape", id: g.id, dx: g.dx, dy: g.dy }); setSel({ kind: "shape", id: g.id }); }
    }
  };
  const wheel = (e: WheelEvent) => { e.preventDefault(); const r = cv.getBoundingClientRect(); zoomBy(Math.exp(-e.deltaY * 0.0015), e.clientX - r.left, e.clientY - r.top); };
  const context = (e: MouseEvent) => {
    e.preventDefault();
    if (pan || !out()) return;
    const h = hit(at(e)), o = out()!;
    const items: Item[] = [];
    if (h.edge) {
      const ref = h.edge, cur = o.edges.find(x => sameEdge(x, ref))?.boundary;
      setSel({ kind: "edge", ...ref });
      items.push({ label: "Put on boundary", sub: o.m.boundaries.length ? o.m.boundaries.map(b => ({ label: `${b.name} (${KIND_NAME[b.kind.type]})`, on: b.id === cur, run: () => act({ op: "set_boundary", boundary: { ...b, edges: [...b.edges.filter(x => !sameEdge(x, ref)), ref] } }) })) : [{ label: "No boundaries yet: Physics › Boundaries", off: true }] });
      if (cur) items.push({ label: "Take off its boundary", run: () => { const b = boundaryOf(cur)!; act({ op: "set_boundary", boundary: { ...b, edges: b.edges.filter(x => !sameEdge(x, ref)) } }); } });
      items.push("-");
    }
    if (h.shape) {
      const s = h.shape;
      if (!h.edge) setSel({ kind: "shape", id: s.id });
      items.push({ label: s.hole ? "Add to the flow region" : "Subtract from the flow region", icon: s.hole ? "union" : "subtract", run: () => act({ op: "set_shape", id: s.id, name: s.name, hole: !s.hole }) });
      items.push({ label: `Delete ${s.name}`, icon: "del", keys: "Del", run: () => act({ op: "remove_shape", id: s.id }) });
      items.push("-");
    }
    items.push({ label: "Fit", icon: "fit", keys: "F", run: () => fit() });
    openMenu(e, items);
  };
  const key = (e: KeyboardEvent) => {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;
    if (document.querySelector(".ov, .bs")) return;
    if (e.key === " ") { setSpace(e.type === "keydown"); if (e.type === "keydown") e.preventDefault(); return; }
    if (e.type !== "keydown") return;
    if (e.key === "Escape") { if (draft()) setDraft(null); else if (tool() !== "select") { setTool("select"); setPickFor(null); } }
    else if (e.key === "Enter" && draft()?.kind === "poly") finishPoly();
    else if (e.key === "Backspace" && draft()?.kind === "poly") { const d = draft() as { kind: "poly"; pts: Pt[]; at: Pt }; setDraft(d.pts.length > 1 ? { ...d, pts: d.pts.slice(0, -1) } : null); }
    else if ((e.key === "f" || e.key === "F") && !e.ctrlKey && !e.metaKey && !e.altKey) fit();
    else if (e.key === "Delete") { const s = sel(); if (s.kind === "shape") act({ op: "remove_shape", id: s.id }); else if (s.kind === "boundary") act({ op: "remove_boundary", id: s.id }); }
  };

  onMount(() => {
    canvasEl = cv;
    // For the UI tests: where a model point is on screen.
    (window as unknown as { __view: object }).__view = { toScreen, toWorld };
    const ro = new ResizeObserver(() => { const r = wrap.getBoundingClientRect(); if (r.width > 0 && r.height > 0) setViewSize({ w: Math.round(r.width), h: Math.round(r.height) }); });
    ro.observe(wrap);
    addEventListener("mousemove", move); addEventListener("mouseup", up); addEventListener("keydown", key); addEventListener("keyup", key);
    cv.addEventListener("wheel", wheel, { passive: false });
    onCleanup(() => { ro.disconnect(); removeEventListener("mousemove", move); removeEventListener("mouseup", up); removeEventListener("keydown", key); removeEventListener("keyup", key); canvasEl = null; });
  });
  createEffect(paint);
  // A drawing tool put away drops what was half drawn.
  createEffect(() => { if (tool() !== "poly" && draft()?.kind === "poly") setDraft(null); });
  // A new model (another file, an example) is shown whole.
  let shown = "";
  createEffect(() => { const o = out(), s = viewSize(); const k = o ? `${o.path}|${o.m.name}|${s.w > 0}` : ""; if (o && k !== shown) { shown = k; queueMicrotask(() => fit()); } });

  return (
    <div class="view" ref={wrap} tabindex="0" classList={{ panning: tool() === "pan" || space(), drawing: ["rect", "circle", "poly"].includes(tool()), picking: tool() === "pick" }}>
      <canvas ref={cv} onMouseDown={down} onDblClick={() => draft()?.kind === "poly" && finishPoly()} onContextMenu={context} />
      <Show when={out() && !out()!.m.shapes.length && !draft() && !["rect", "circle", "poly"].includes(tool())}>
        <div class="vempty"><b>Draw the flow region</b><span>Start with Geometry › Rectangle, Circle or Polygon, pick a template, or open an example from File › Examples.</span>
          <div><button class="btn pri" onClick={() => setTool("rect")}>Draw a rectangle</button></div></div>
      </Show>
      <Show when={tool() !== "select" && tool() !== "pan"}>
        <div class="vhint">{tool() === "pick" ? `Click the edges of “${boundaryOf(pickFor() ?? -1)?.name ?? "the boundary"}”. Click again to take one off. Esc when done.`
          : tool() === "poly" ? "Click each corner. Enter or a double-click closes the polygon; Backspace takes a corner back; Esc cancels."
          : tool() === "rect" ? "Press, drag and release to draw a rectangle. Esc cancels." : "Press at the centre, drag to the radius and release. Esc cancels."}{asHole() && tool() !== "pick" ? " Subtract is on: the shape will be cut out of the flow region." : ""}</div>
      </Show>
    </div>
  );
};
