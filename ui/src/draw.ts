// What the engine's results look like in the 2D view: mesh lines, element quality, filled
// contours, streamlines, arrows, probes and plot lines, as layers of the canvas. Drawing only:
// every value drawn was worked out by the engine.
import { createEffect } from "solid-js";
import { Pt } from "./api";
import { out, M, toU, ulabel, themeTick } from "./store";
import { setLayer, removeLayer, setRegionFill, redraw, xf, viewSize, Env } from "./canvas";
import { mesh, showQuality, field, stream, arrowData, probeData, placing, lineStart, lineEnd, NodalField, MeshView } from "./sim";
import { num } from "./api";

// ---------- colour maps
// Viridis, Inferno (S. van der Walt, N. Smith) and Cividis (J. Nuñez, C. Anderton, R. Renslow)
// were released to the public domain (CC0) by their authors; these are ten samples of each,
// interpolated. The diverging map is plain blue to white to red.
const STOPS: Record<string, string[]> = {
  viridis: ["#440154", "#482878", "#3e4989", "#31688e", "#26828e", "#1f9e89", "#35b779", "#6ece58", "#b5de2b", "#fde725"],
  inferno: ["#000004", "#1b0c41", "#4a0c6b", "#781c6d", "#a52c60", "#cf4446", "#ed6925", "#fb9b06", "#f7d13d", "#fcffa4"],
  cividis: ["#00224e", "#123570", "#3b496c", "#575d6d", "#707173", "#8a8678", "#a59c74", "#c3b369", "#e1cc55", "#fee838"],
  diverging: ["#1d4f9c", "#4f7fc4", "#9db9e2", "#f4f4f2", "#e8a79c", "#cf5b4c", "#a8261c"],
};
const hex = (h: string) => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];
const luts: Record<string, Uint8Array> = {};
/** 256 colours of a map (r, g, b each); `bands` > 0 gives that many flat steps. */
export const lut = (map: string, bands = 0): Uint8Array => {
  const key = `${map}|${bands}`;
  if (luts[key]) return luts[key];
  const st = (STOPS[map] ?? STOPS.viridis).map(hex), t = new Uint8Array(768);
  for (let i = 0; i < 256; i++) {
    let f = i / 255;
    if (bands > 0) f = Math.min(bands - 1, Math.floor(f * bands)) / Math.max(1, bands - 1);
    const x = f * (st.length - 1), k = Math.min(st.length - 2, Math.floor(x)), a = x - k;
    for (let c = 0; c < 3; c++) t[3 * i + c] = Math.round(st[k][c] + (st[k + 1][c] - st[k][c]) * a);
  }
  return (luts[key] = t);
};
export const cssColour = (map: string, f: number) => { const t = lut(map), i = 3 * Math.max(0, Math.min(255, Math.round(f * 255))); return `rgb(${t[i]},${t[i + 1]},${t[i + 2]})`; };
/** The range a field is coloured over: its own, or symmetric about zero for the diverging map. */
/** The colour range of a field: the one set by hand (Results › Range), else from its values. */
export const fieldRange = (f: NodalField, map: string): [number, number] => {
  const fixed = out()?.m.view.ranges?.[f.id];
  return fixed && fixed[1] > fixed[0] ? [fixed[0], fixed[1]] : autoRange(f, map);
};
/** The automatic colour range: the field's own, symmetric about zero for the diverging map. */
export const autoRange = (f: NodalField, map: string): [number, number] => {
  if (map === "diverging") { const m = Math.max(Math.abs(f.min), Math.abs(f.max)) || 1; return [-m, m]; }
  return f.max > f.min ? [f.min, f.max] : [f.min - 0.5, f.min + 0.5];
};

// Each six-node triangle is drawn as four straight ones through its midside nodes.
const SUB = [[0, 3, 5], [3, 1, 4], [5, 4, 2], [3, 4, 5]];

// ---------- filled contours: rasterised into a picture, redone after pans and zooms settle
let pic: { cv: HTMLCanvasElement; key: string; cx: number; cy: number; k: number; w: number; h: number } | null = null, picTimer = 0;
const raster = (m: MeshView, f: NodalField, map: string, bands: number, t: { cx: number; cy: number; k: number }, w: number, h: number) => {
  // At most about three million pixels: sharp on dense screens without slowing large windows.
  const scale = Math.min(window.devicePixelRatio || 1, Math.sqrt(3e6 / Math.max(1, w * h)));
  const W = Math.max(1, Math.round(w * scale)), H = Math.max(1, Math.round(h * scale));
  const cv = document.createElement("canvas"); cv.width = W; cv.height = H;
  const ctx = cv.getContext("2d")!, img = ctx.createImageData(W, H), px = new Uint32Array(img.data.buffer);
  const table = lut(map, bands), [lo, hi] = fieldRange(f, map), inv = 255 / (hi - lo);
  const col = new Uint32Array(256);
  for (let i = 0; i < 256; i++) col[i] = (255 << 24) | (table[3 * i + 2] << 16) | (table[3 * i + 1] << 8) | table[3 * i];
  const n = m.nodes, v = f.values, ks = t.k * scale, ox = W / 2 - t.cx * ks, oy = H / 2 + t.cy * ks;
  const sx = new Float32Array(6), sy = new Float32Array(6), sv = new Float32Array(6);
  for (let e = 0; e < m.tris.length; e += 6) {
    let ok = true, x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity;
    for (let a = 0; a < 6; a++) {
      const i = m.tris[e + a], val = v[i];
      if (val === null || val !== val) { ok = false; break; }
      sx[a] = ox + n[2 * i] * ks; sy[a] = oy - n[2 * i + 1] * ks; sv[a] = (val - lo) * inv;
      if (sx[a] < x0) x0 = sx[a]; if (sx[a] > x1) x1 = sx[a]; if (sy[a] < y0) y0 = sy[a]; if (sy[a] > y1) y1 = sy[a];
    }
    if (!ok || x1 < 0 || y1 < 0 || x0 > W || y0 > H) continue;
    for (const s of SUB) {
      const ax = sx[s[0]], ay = sy[s[0]], bx = sx[s[1]], by = sy[s[1]], cx = sx[s[2]], cy = sy[s[2]];
      const det = (bx - ax) * (cy - ay) - (cx - ax) * (by - ay);
      if (det === 0) continue;
      const id = 1 / det, va = sv[s[0]], vb = sv[s[1]], vc = sv[s[2]];
      const px0 = Math.max(0, Math.floor(Math.min(ax, bx, cx))), px1 = Math.min(W - 1, Math.ceil(Math.max(ax, bx, cx)));
      const py0 = Math.max(0, Math.floor(Math.min(ay, by, cy))), py1 = Math.min(H - 1, Math.ceil(Math.max(ay, by, cy)));
      // A little slack so neighbouring triangles leave no gaps.
      const eps = -0.5 * Math.abs(id) * (Math.abs(bx - ax) + Math.abs(cx - ax) + Math.abs(by - ay) + Math.abs(cy - ay)) * 0.5 - 1e-6;
      for (let y = py0; y <= py1; y++) {
        const fy = y + 0.5 - ay;
        let row = y * W + px0;
        for (let x = px0; x <= px1; x++, row++) {
          const fx = x + 0.5 - ax;
          const l1 = (fx * (cy - ay) - (cx - ax) * fy) * id, l2 = ((bx - ax) * fy - fx * (by - ay)) * id, l0 = 1 - l1 - l2;
          if (l0 < eps || l1 < eps || l2 < eps) continue;
          const c = l0 * va + l1 * vb + l2 * vc;
          px[row] = col[c <= 0 ? 0 : c >= 255 ? 255 : c | 0];
        }
      }
    }
  }
  ctx.putImageData(img, 0, 0);
  return cv;
};
const contourKey = (f: NodalField, m: MeshView) => `${f.id}|${f.min}|${f.max}|${f.values.length}|${m.tris.length}|${M().view.map}|${M().view.bands}|${fieldRange(f, M().view.map).join(",")}`;
const drawContours = (ctx: CanvasRenderingContext2D, env: Env) => {
  const f = field(), m = mesh();
  if (!f || !m || f.values.length * 2 !== m.nodes.length) return;
  const t = xf(), key = contourKey(f, m);
  const fresh = pic && pic.key === key && pic.cx === t.cx && pic.cy === t.cy && pic.k === t.k && pic.w === env.w && pic.h === env.h;
  if (!fresh) {
    const make = () => { const tt = xf(), s = viewSize(), ff = field(), mm = mesh(); if (!ff || !mm || ff.values.length * 2 !== mm.nodes.length) return; pic = { cv: raster(mm, ff, M().view.map, M().view.bands, tt, s.w, s.h), key: contourKey(ff, mm), cx: tt.cx, cy: tt.cy, k: tt.k, w: s.w, h: s.h }; redraw(); };
    clearTimeout(picTimer);
    // The first picture and a changed field are drawn at once; while panning the old one is stretched.
    if (!pic || pic.key !== key) { pic = { cv: raster(m, f, M().view.map, M().view.bands, t, env.w, env.h), key, cx: t.cx, cy: t.cy, k: t.k, w: env.w, h: env.h }; }
    else picTimer = window.setTimeout(make, 70);
  }
  if (!pic) return;
  // Where the picture's corners are now.
  const r = t.k / pic.k, x = env.w / 2 + ((pic.cx - pic.w / 2 / pic.k) - t.cx) * t.k, y = env.h / 2 - ((pic.cy + pic.h / 2 / pic.k) - t.cy) * t.k;
  ctx.imageSmoothingEnabled = r !== 1;
  ctx.drawImage(pic.cv, x, y, pic.w * r, pic.h * r);
};

// ---------- mesh lines and quality
let edgeCache: { of: MeshView; segs: Float32Array } | null = null;
const meshEdges = (m: MeshView) => {
  if (edgeCache?.of === m) return edgeCache.segs;
  const seen = new Set<number>(), out: number[] = [], n = m.nodes, nn = n.length / 2;
  for (let e = 0; e < m.tris.length; e += 6) for (let j = 0; j < 3; j++) {
    const a = m.tris[e + j], b = m.tris[e + (j + 1) % 3], key = Math.min(a, b) * nn + Math.max(a, b);
    if (seen.has(key)) continue;
    seen.add(key); out.push(n[2 * a], n[2 * a + 1], n[2 * b], n[2 * b + 1]);
  }
  edgeCache = { of: m, segs: new Float32Array(out) };
  return edgeCache.segs;
};
const strokeSegs = (ctx: CanvasRenderingContext2D, env: Env, segs: Float32Array) => {
  const t = xf(), ox = env.w / 2 - t.cx * t.k, oy = env.h / 2 + t.cy * t.k;
  ctx.beginPath();
  for (let i = 0; i < segs.length; i += 4) {
    const x0 = ox + segs[i] * t.k, y0 = oy - segs[i + 1] * t.k, x1 = ox + segs[i + 2] * t.k, y1 = oy - segs[i + 3] * t.k;
    if ((x0 < 0 && x1 < 0) || (y0 < 0 && y1 < 0) || (x0 > env.w && x1 > env.w) || (y0 > env.h && y1 > env.h)) continue;
    ctx.moveTo(x0, y0); ctx.lineTo(x1, y1);
  }
  ctx.stroke();
};
export const QUALITY: [string, string, string][] = [["over 30°", "#2E9D57", "good"], ["20° to 30°", "#E0A21B", "fair"], ["under 20°", "#C0392B", "poor"]];
const drawQuality = (ctx: CanvasRenderingContext2D, env: Env) => {
  const m = mesh(); if (!m) return;
  const t = xf(), ox = env.w / 2 - t.cx * t.k, oy = env.h / 2 + t.cy * t.k, n = m.nodes;
  for (let band = 0; band < 3; band++) {
    ctx.beginPath();
    for (let e = 0, k = 0; e < m.tris.length; e += 6, k++) {
      const a = m.angle[k], b = a > 30 ? 0 : a > 20 ? 1 : 2;
      if (b !== band) continue;
      for (let j = 0; j < 3; j++) { const i = m.tris[e + j], x = ox + n[2 * i] * t.k, y = oy - n[2 * i + 1] * t.k; if (j) ctx.lineTo(x, y); else ctx.moveTo(x, y); }
      ctx.closePath();
    }
    ctx.globalAlpha = band === 0 ? 0.45 : 0.85; ctx.fillStyle = QUALITY[band][1]; ctx.fill();
  }
  ctx.globalAlpha = 1;
};
const drawMesh = (ctx: CanvasRenderingContext2D, env: Env) => {
  const m = mesh(); if (!m) return;
  const over = !!field() && !showQuality();
  ctx.lineWidth = 0.6; ctx.globalAlpha = over ? 0.35 : 0.75;
  ctx.strokeStyle = over ? "#101820" : env.css("--muted");
  strokeSegs(ctx, env, meshEdges(m));
  ctx.globalAlpha = 1;
};

// ---------- streamlines: the level lines of the stream function, found once per field
let lineCache: { of: NodalField; segs: Float32Array } | null = null;
/** Levels for `count` streamlines, spread evenly over the region's area (not over its nodes,
 *  which crowd where the mesh is fine): slow eddies get lines too. */
const streamLevels = (m: MeshView, f: NodalField, count: number) => {
  const n = m.nodes, v = f.values, parts: [number, number][] = [];
  let total = 0;
  for (let e = 0; e < m.tris.length; e += 6) {
    const i = m.tris[e], j = m.tris[e + 1], k = m.tris[e + 2], a = v[i], b = v[j], c = v[k];
    if (a === null || b === null || c === null) continue;
    const area = Math.abs((n[2 * j] - n[2 * i]) * (n[2 * k + 1] - n[2 * i + 1]) - (n[2 * k] - n[2 * i]) * (n[2 * j + 1] - n[2 * i + 1]));
    parts.push([(a + b + c) / 3, area]); total += area;
  }
  parts.sort((x, y) => x[0] - y[0]);
  const out: number[] = [];
  let run = 0, next = 1;
  for (const [val, area] of parts) {
    run += area;
    while (next <= count && run >= total * next / (count + 1)) { if (!out.length || Math.abs(val - out[out.length - 1]) > 1e-12 * (Math.abs(val) + Math.abs(f.max - f.min))) out.push(val); next++; }
  }
  return out;
};
const streamSegs = (m: MeshView, f: NodalField) => {
  if (lineCache?.of === f) return lineCache.segs;
  const out: number[] = [], n = m.nodes, v = f.values, lv = streamLevels(m, f, 26);
  const cut = (i: number, j: number, c: number) => { const a = v[i] as number, b = v[j] as number, s = (c - a) / (b - a); out.push(n[2 * i] + (n[2 * j] - n[2 * i]) * s, n[2 * i + 1] + (n[2 * j + 1] - n[2 * i + 1]) * s); };
  for (let e = 0; e < m.tris.length; e += 6) for (const s of SUB) {
    const i = m.tris[e + s[0]], j = m.tris[e + s[1]], k = m.tris[e + s[2]], a = v[i], b = v[j], c = v[k];
    if (a === null || b === null || c === null) continue;
    const lo = Math.min(a, b, c), hi = Math.max(a, b, c);
    if (lo === hi) continue;
    for (const l of lv) {
      if (l < lo || l > hi) continue;
      const pa = a > l, pb = b > l, pc = c > l;
      if (pa === pb && pb === pc) continue;
      if (pa !== pb) cut(i, j, l);
      if (pb !== pc) cut(j, k, l);
      if (pc !== pa) cut(k, i, l);
    }
  }
  lineCache = { of: f, segs: new Float32Array(out) };
  return lineCache.segs;
};
const drawStream = (ctx: CanvasRenderingContext2D, env: Env) => {
  const m = mesh(), f = stream();
  if (!m || !f || f.values.length * 2 !== m.nodes.length) return;
  const filled = !!field();
  // Over colours: dark lines with a light edge, readable on every map. On the bare region: ink.
  if (filled) { ctx.lineWidth = 2.2; ctx.strokeStyle = "rgba(255,255,255,.45)"; strokeSegs(ctx, env, streamSegs(m, f)); }
  ctx.lineWidth = 1; ctx.strokeStyle = filled ? "rgba(12,20,32,.85)" : env.css("--text2");
  strokeSegs(ctx, env, streamSegs(m, f));
};

// ---------- arrows
const drawArrows = (ctx: CanvasRenderingContext2D, env: Env) => {
  const a = arrowData(), b = out()?.bounds; if (!a.length || !b) return;
  let umax = 0; for (const p of a) umax = Math.max(umax, Math.hypot(p[2], p[3]));
  if (!(umax > 0)) return;
  // The longest arrow spans about one cell of the arrows' grid.
  const cell = Math.sqrt((b[2] - b[0]) * (b[3] - b[1]) / a.length) * env.k, len = Math.max(6, cell * 0.95) / umax;
  const filled = !!field();
  ctx.strokeStyle = ctx.fillStyle = filled ? "rgba(255,255,255,.92)" : env.css("--text"); ctx.lineWidth = 1.1; ctx.lineCap = "round";
  ctx.beginPath();
  for (const p of a) {
    const s = env.toScreen([p[0], p[1]]), dx = p[2] * len, dy = -p[3] * len, l = Math.hypot(dx, dy);
    if (l < 1.5 || s[0] < -20 || s[1] < -20 || s[0] > env.w + 20 || s[1] > env.h + 20) continue;
    const x0 = s[0] - dx / 2, y0 = s[1] - dy / 2, x1 = s[0] + dx / 2, y1 = s[1] + dy / 2, h = Math.min(5, l * 0.4), ux = dx / l, uy = dy / l;
    ctx.moveTo(x0, y0); ctx.lineTo(x1, y1);
    ctx.moveTo(x1 - ux * h - uy * h * 0.55, y1 - uy * h + ux * h * 0.55); ctx.lineTo(x1, y1); ctx.lineTo(x1 - ux * h + uy * h * 0.55, y1 - uy * h - ux * h * 0.55);
  }
  ctx.stroke();
};

// ---------- probes and plot lines
const tag = (ctx: CanvasRenderingContext2D, env: Env, x: number, y: number, lines: string[]) => {
  ctx.font = `500 10px ${env.css("--mono")}`; ctx.textBaseline = "middle"; ctx.textAlign = "left";
  const w = Math.max(...lines.map(l => ctx.measureText(l).width)) + 10, h = 13 * lines.length + 5;
  ctx.fillStyle = env.css("--bg2"); ctx.globalAlpha = 0.92; ctx.fillRect(x, y - h / 2, w, h); ctx.globalAlpha = 1;
  ctx.strokeStyle = env.css("--line3"); ctx.lineWidth = 1; ctx.strokeRect(Math.round(x) + 0.5, Math.round(y - h / 2) + 0.5, Math.round(w), Math.round(h));
  ctx.fillStyle = env.css("--text");
  lines.forEach((l, i) => ctx.fillText(l, x + 5, y - h / 2 + 9 + 13 * i));
};
const val = (kind: string, v: number, sig = 4) => `${num(toU(kind, v), sig)} ${ulabel(kind)}`.trim();
const drawMarks = (ctx: CanvasRenderingContext2D, env: Env) => {
  const o = out(); if (!o) return;
  const lines = o.m.lines.map(l => [l.a, l.b, l.name] as [Pt, Pt, string]);
  if (placing() === "line" && lineStart() && lineEnd()) lines.push([lineStart()!, lineEnd()!, ""]);
  for (const [a, b, name] of lines) {
    const p = env.toScreen(a), q = env.toScreen(b);
    ctx.strokeStyle = "rgba(255,255,255,.8)"; ctx.lineWidth = 3.4; ctx.beginPath(); ctx.moveTo(p[0], p[1]); ctx.lineTo(q[0], q[1]); ctx.stroke();
    ctx.strokeStyle = "#D55E00"; ctx.lineWidth = 1.6; ctx.setLineDash([7, 4]); ctx.beginPath(); ctx.moveTo(p[0], p[1]); ctx.lineTo(q[0], q[1]); ctx.stroke(); ctx.setLineDash([]);
    ctx.fillStyle = "#D55E00"; for (const s of [p, q]) ctx.fillRect(s[0] - 3, s[1] - 3, 6, 6);
    if (name) tag(ctx, env, p[0] + (q[0] - p[0]) * 0.62, p[1] + (q[1] - p[1]) * 0.62 - 14, [name]);
  }
  const data = probeData();
  o.m.probes.forEach((pr, i) => {
    const s = env.toScreen([pr.x, pr.y]), d = data[i]?.[1];
    ctx.fillStyle = "#fff"; ctx.strokeStyle = "#0C1A2B"; ctx.lineWidth = 1.6;
    ctx.beginPath(); ctx.arc(s[0], s[1], 4.5, 0, 2 * Math.PI); ctx.fill(); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(s[0] - 8, s[1]); ctx.lineTo(s[0] + 8, s[1]); ctx.moveTo(s[0], s[1] - 8); ctx.lineTo(s[0], s[1] + 8); ctx.stroke();
    const text = [pr.name || `Probe ${i + 1}`];
    if (d && o.has_results) {
      if (!d.inside) text.push("outside the fluid");
      else { text.push(val("speed", d.speed), `p ${val("pressure", d.pressure)}`); if (d.temperature !== null && d.temperature === d.temperature && o.m.physics.heat) text.push(`T ${val("temp", d.temperature)}`); }
    }
    // A probe with another close on its right puts its label on the left, so the two do not cover each other.
    const crowded = o.m.probes.some((q, j) => j !== i && q.x > pr.x && (q.x - pr.x) * env.k < 170 && Math.abs(q.y - pr.y) * env.k < 44);
    ctx.font = `500 10px ${env.css("--mono")}`;
    const wide = Math.max(...text.map(l => ctx.measureText(l).width)) + 10;
    tag(ctx, env, crowded ? s[0] - 11 - wide : s[0] + 11, s[1], text);
  });
};

/** Keep the view's layers in step with what the engine has sent. Called once, inside the app. */
export const initDraw = () => {
  createEffect(() => {
    const o = out(), m = mesh(), f = field(), st = stream(), q = showQuality();
    themeTick(); arrowData(); probeData(); placing();
    const results = !!o?.has_results && !!m;
    if (results && f && !q) setLayer({ id: "contours", z: -3, draw: drawContours }); else { removeLayer("contours"); pic = null; }
    if (m && q) setLayer({ id: "quality", z: -2, draw: drawQuality }); else removeLayer("quality");
    // Mesh lines: asked for, or all there is to see before a solve.
    if (m && (q || o?.m.view.mesh || !results)) setLayer({ id: "mesh", z: -1, draw: drawMesh }); else removeLayer("mesh");
    if (results && st && o!.m.view.streamlines && !q) setLayer({ id: "stream", z: 1, draw: drawStream }); else removeLayer("stream");
    if (results && o!.m.view.arrows && !q) setLayer({ id: "arrows", z: 2, draw: drawArrows }); else removeLayer("arrows");
    if (o && (o.m.probes.length || o.m.lines.length || placing())) setLayer({ id: "marks", z: 5, draw: drawMarks }); else removeLayer("marks");
    setRegionFill(!(results && f && !q) && !(m && q));
    redraw();
  });
};

/** A picture of the mesh alone, fitted to w × h pixels, for the report. */
export const meshPicture = (w = 1400, h = 700): string | null => {
  const m = mesh(), b = out()?.bounds; if (!m || !b) return null;
  const cv = document.createElement("canvas"); cv.width = w; cv.height = h;
  const ctx = cv.getContext("2d")!, k = Math.min((w - 20) / (b[2] - b[0]), (h - 20) / (b[3] - b[1])), ox = w / 2 - (b[0] + b[2]) / 2 * k, oy = h / 2 + (b[1] + b[3]) / 2 * k, segs = meshEdges(m);
  ctx.fillStyle = "#fff"; ctx.fillRect(0, 0, w, h); ctx.strokeStyle = "#3B4859"; ctx.lineWidth = 0.6; ctx.beginPath();
  for (let i = 0; i < segs.length; i += 4) { ctx.moveTo(ox + segs[i] * k, oy - segs[i + 1] * k); ctx.lineTo(ox + segs[i + 2] * k, oy - segs[i + 3] * k); }
  ctx.stroke();
  return cv.toDataURL("image/png");
};
