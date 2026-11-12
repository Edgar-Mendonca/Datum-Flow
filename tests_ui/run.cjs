// UI tests: the built window (ui/dist) in Chromium, with the real model engine behind a bridge
// (crates/flow-core/examples/dump.rs --bridge) in place of the desktop shell. Edits, undo, examples
// and fluids are the engine's own; only the shell's parts (files, options, updates) are mocked.
// Run: npm run test:ui (dumps tests_ui/fixture.json, builds the window, runs this).
const { chromium } = require("playwright");
const { spawn } = require("child_process");
const fs = require("fs"), http = require("http"), path = require("path"), readline = require("readline");

const root = path.join(__dirname, "../ui/dist"), fix = JSON.parse(fs.readFileSync(path.join(__dirname, "fixture.json")));
// FAMILY §13: the window never names a commercial product; whole words, any case.
const NAMES = fs.readFileSync(path.join(__dirname, "../tools/commercial-names.txt"), "utf8").split("\n").map(l => l.trim().toLowerCase()).filter(l => l && !l.startsWith("#"));
const named = text => { const t = text.toLowerCase(); return NAMES.filter(n => new RegExp(`(^|[^\\p{L}\\p{N}])${n.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}($|[^\\p{L}\\p{N}])`, "u").test(t)); };
const types = { ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".woff2": "font/woff2", ".woff": "font/woff", ".html": "text/html" };
const srv = http.createServer((q, r) => {
  const f = path.join(root, q.url === "/" ? "index.html" : decodeURIComponent(q.url.split("?")[0]));
  fs.readFile(f, (e, b) => { if (e) { r.writeHead(404); return r.end(); } r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" }); r.end(b); });
});
let failed = 0;
const check = (name, ok, info = "") => { console.log(`${ok ? "ok  " : "FAIL"} ${name}${ok ? "" : "  " + info}`); if (!ok) failed++; };
const shots = path.join(__dirname, "shots");

// ---------- the engine bridge: one request per line, answers matched by id
const engine = spawn("cargo", ["run", "-q", "-p", "flow-core", "--example", "dump", "--", "--bridge"], { cwd: path.join(__dirname, ".."), stdio: ["pipe", "pipe", "inherit"] });
const waiting = new Map(); let nextId = 1;
readline.createInterface({ input: engine.stdout }).on("line", l => { try { const m = JSON.parse(l); waiting.get(m.id)?.(m); waiting.delete(m.id); } catch { /* not ours */ } });
const ask = (cmd, args) => new Promise(res => { const id = nextId++; waiting.set(id, res); engine.stdin.write(JSON.stringify({ id, cmd, args: args ?? {} }) + "\n"); });

(async () => {
  await new Promise(ok => srv.listen(0, ok));
  const url = `http://localhost:${srv.address().port}/`;
  const b = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {});
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on("pageerror", e => errors.push(e.message));
  p.on("console", m => { if (m.type() === "error") errors.push(m.text()); });
  await p.exposeFunction("__bridge", ask);
  await p.addInitScript(() => {
    const opts = { autosave: 0, beta: false, theme: "light", openLast: false, units: "SI", snap: true, grid: true, map: "viridis", paper: "A4", landscape: true, company: "Test Works" };
    const shell = { options: () => opts, set_options: a => (Object.assign(opts, a.opts), null), startup_file: () => null, recovery: () => null, recent: () => Object.keys(window.__files).map(f => ({ path: f, name: f.split("/").pop().replace(".dflow", ""), modified: "12-Nov-26 10:00" })), version: () => "test", autosave: () => null, quit: () => null,
      check_update: () => ({ current: "0.1.0", latest: "0.1.0", beta: false, available: false, notes: "## 0.1.0\n- The first release.", published: "2026-11-12", url: null, sums: null, size: 0, page: "" }),
      open_link: a => { window.__links.push(a.url); return null; }, save_file: a => `C:/Exports/${a.name}`, save_pdf: () => { throw "native-pdf-unavailable"; } };
    window.__links = []; window.__calls = []; window.__files = {};
    window.__TAURI__ = {
      core: { invoke: async (c, a) => {
        window.__calls.push([c, a]);
        if (c in shell) return shell[c](a ?? {});
        // Files: the tests keep "saved" files in the page, by path.
        if (c === "open") { if (!a?.path) return null; a = { path: a.path, text: window.__files[a.path] ?? "" }; }
        const r = await window.__bridge(c, a ?? {});
        if (r.err !== undefined) throw r.err;
        if (c === "set_options") return (await window.__bridge("state", {})).ok;
        return r.ok;
      } },
      event: { listen: async () => () => {} },
    };
    window.print = () => { window.__printed = (window.__printed ?? 0) + 1; };
  });
  fs.mkdirSync(shots, { recursive: true });
  const shot = n => p.screenshot({ path: path.join(shots, n + ".png") });
  const text = sel => p.evaluate(s => document.querySelector(s)?.innerText ?? "", sel);
  const state = async () => (await ask("state")).ok;
  const toastText = () => text("#toast");
  const tab = async name => { await p.click(`.tabs button:has-text("${name}")`); await p.waitForTimeout(80); };
  const spoken = []; // every text the window showed, for the names check
  const hear = async () => spoken.push(await p.evaluate(() => document.body.innerText + " " + [...document.querySelectorAll("[title]")].map(e => e.getAttribute("title")).join(" ") + " " + [...document.querySelectorAll("[placeholder]")].map(e => e.getAttribute("placeholder")).join(" ")));

  await p.goto(url);
  await p.waitForSelector(".bs .home");
  await p.waitForTimeout(300);

  // ---------- start: the File page
  check("starts on the File page with templates and examples", await p.locator(".bs .tiles .tile").count() >= 6 && await p.locator(".recent.examples tr").count() === Object.keys(fix.states).length - 1);
  check("the update card says the version is current", (await text(".bs .card")).includes("YOU HAVE THE LATEST VERSION") || (await text(".bs .card")).toLowerCase().includes("latest version"));
  await hear(); await shot("01-file-home");

  // ---------- an example loads, as the engine sends it
  await p.click('.recent.examples tr:has-text("Flow past a cylinder")');
  await p.waitForSelector(".view canvas");
  await p.waitForTimeout(250);
  const st = await state();
  check("the cylinder example is the engine's own model", JSON.stringify(st.m) === JSON.stringify(fix.states.cylinder.m));
  const treeText = await text(".tree");
  check("the tree lists its shapes and boundaries", ["Channel", "Cylinder", "Inlet", "Outlet", "Channel walls", "Fluid", "Mesh", "Solver", "Results"].every(t => treeText.includes(t)), treeText);
  check("the status bar shows the Reynolds number", /Re 20\b/.test(await text(".status")), await text(".status"));
  check("the title names the model", (await text(".title")).includes("Flow past a cylinder"));
  // The view really draws: the canvas holds more than one colour, and the pale region fill.
  const colours = await p.evaluate(() => { const c = document.querySelector(".view canvas"), d = c.getContext("2d").getImageData(0, 0, c.width, c.height).data, s = new Set(); for (let i = 0; i < d.length; i += 4 * 97) s.add(`${d[i]},${d[i + 1]},${d[i + 2]}`); return [...s]; });
  check("the view draws the model", colours.length > 4 && colours.includes("236,238,250"), `${colours.length} colours`);
  await hear(); await shot("02-cylinder");

  // ---------- every ribbon tab opens, fits, and explains itself
  for (const name of ["Home", "Geometry", "Physics", "Mesh", "Solve", "Results", "Check", "View", "Help"]) {
    await tab(name);
    const r = await p.evaluate(() => { const el = document.querySelector(".ribbon"); return { groups: el.querySelectorAll(".grp").length, over: el.scrollWidth - el.clientWidth, on: document.querySelector(".tabs button.on")?.innerText }; });
    check(`ribbon tab ${name} opens with its groups and does not scroll sideways`, r.on === name && r.groups >= 3 && r.over <= 1, JSON.stringify(r));
    await hear();
    if (name === "Geometry" || name === "Physics" || name === "Results") await shot(`03-ribbon-${name.toLowerCase()}`);
  }
  // A narrow window folds groups instead of scrolling.
  await p.setViewportSize({ width: 980, height: 700 }); await tab("Results"); await p.waitForTimeout(200);
  const narrow = await p.evaluate(() => { const el = document.querySelector(".ribbon"); return { over: el.scrollWidth - el.clientWidth, folded: el.querySelectorAll('.grp[data-fit="4"]').length }; });
  check("a narrow window folds ribbon groups instead of scrolling", narrow.over <= 1, JSON.stringify(narrow));
  await p.setViewportSize({ width: 1440, height: 900 }); await p.waitForTimeout(150);

  // ---------- parts that arrive later say so (never a dead button)
  for (const [tabName, button, word] of [["Mesh", "Generate", "mesh"], ["Solve", "Solve", "Solving"], ["Results", "Streamlines", "Streamlines"], ["Check", "Verification Report", "verification"], ["Geometry", "Import DXF", "DXF"]]) {
    await tab(tabName);
    await p.click(`.ribbon .rb:has-text("${button}")`);
    await p.waitForTimeout(80);
    const t = await toastText();
    check(`${tabName} › ${button} says it is coming`, t.includes("coming in 0.1.x") && t.toLowerCase().includes(word.toLowerCase()), t);
    await hear();
  }

  // ---------- ? cards
  await tab("Physics");
  await p.click('.ribbon .grp:has(.cap:has-text("Boundaries")) .help');
  check("a ? card opens and explains boundaries", (await text(".dlg")).includes("Every edge needs one"));
  await hear(); await p.keyboard.press("Escape");
  const helps = await p.evaluate(() => [...document.querySelectorAll(".help")].length);
  check("the window has ? cards beside its tools", helps >= 6, String(helps));

  // ---------- the inspector edits through the engine: fluid, units, undo
  await p.click('.tree .ti:has-text("Fluid")');
  await p.selectOption(".insp .isec select", "air");
  await p.waitForTimeout(150);
  let s2 = await state();
  check("choosing Air loads the engine's values and their source", Math.abs(s2.m.fluid.rho - 1.204) < 0.01 && (await text(".insp")).includes("Sutherland"), String(s2.m.fluid.rho));
  check("the Reynolds number follows the fluid", !/Re 20\b/.test(await text(".status")));
  await p.keyboard.press("Control+z"); await p.waitForTimeout(120);
  check("Undo restores the fluid", (await state()).m.fluid.rho === 1 && /Re 20\b/.test(await text(".status")));
  await p.click('.status .mode:has-text("Units")'); await p.waitForTimeout(120);
  check("the units switch converts what is shown (the model stays in SI)", (await state()).m.units === "US" && (await text(".status")).includes("Units: US") && (await state()).m.shapes[0].geom.pts[1][0] === 2.2);
  await p.click('.tree .ti:has-text("Channel") >> nth=0'); await p.waitForTimeout(80);
  check("lengths are shown in feet", (await text(".insp")).includes("ft") && (await p.inputValue(".insp .isec:has(h3:has-text('Rectangle')) .ifield:has(label:has-text('Width')) input")).startsWith("7.21"), await p.inputValue(".insp .isec:has(h3:has-text('Rectangle')) .ifield:has(label:has-text('Width')) input"));
  await p.click('.status .mode:has-text("Units")'); await p.waitForTimeout(120);
  // Typing an exact size.
  const w = p.locator(".insp .isec:has(h3:has-text('Rectangle')) .ifield:has(label:has-text('Width')) input");
  await w.fill("2,5"); await w.press("Enter"); await p.waitForTimeout(150);
  check("typing a width changes the rectangle (a comma is read as a decimal point)", (await state()).m.shapes[0].geom.pts[1][0] === 2.5);
  await w.fill("abc"); await w.press("Enter"); await p.waitForTimeout(100);
  check("a value that is not a number is refused and the box goes back", (await toastText()).includes("not a number") && (await w.inputValue()) === "2.5" && (await state()).m.shapes[0].geom.pts[1][0] === 2.5);
  await hear();

  // ---------- an engine error is shown, and the model is as it was
  await p.click('.tree .ti:not(.head):has-text("Cylinder") >> nth=0'); await p.waitForTimeout(80);
  const rad = p.locator(".insp .ifield:has(label:has-text('Radius')) input");
  await rad.fill("0"); await rad.press("Enter"); await p.waitForTimeout(120);
  check("an impossible radius is refused", (await state()).m.shapes[1].geom.r === 0.05, await toastText());

  // ---------- a blank model: draw, bound, check
  await p.keyboard.press("Control+n"); await p.waitForSelector(".bs .home");
  await p.click('.bs .tile:has-text("Blank model")'); await p.waitForTimeout(200);
  check("leaving a changed model asks first", (await text(".dlg")).includes("not saved"));
  await hear(); await p.click('.dlg .btn.pri'); await p.waitForTimeout(250);
  check("a blank model shows what to do first", (await text(".vempty")).includes("Draw the flow region") && (await state()).m.shapes.length === 0);
  await shot("04-empty"); await hear();
  await tab("Geometry");
  await p.click('.ribbon .rb:has-text("Rectangle")');
  const box = await p.locator(".view").boundingBox();
  const at = (fx, fy) => [box.x + box.width * fx, box.y + box.height * fy];
  await p.mouse.move(...at(0.3, 0.7)); await p.mouse.down(); await p.mouse.move(...at(0.5, 0.5), { steps: 4 }); await p.mouse.move(...at(0.7, 0.4), { steps: 4 }); await p.mouse.up();
  await p.waitForTimeout(250);
  let s3 = await state();
  check("dragging draws a rectangle, snapped to the grid", s3.m.shapes.length === 1 && s3.m.shapes[0].geom.pts.every(q => q.every(v => Math.abs(v / s3.m.grid - Math.round(v / s3.m.grid)) < 1e-9)), JSON.stringify(s3.m.shapes[0]?.geom));
  check("the new shape is selected and listed", (await text(".insp")).includes("RECTANGLE") || (await text(".insp")).includes("Rectangle"));
  // Subtract a circle from it.
  await p.click('.ribbon .rb:has-text("Subtract")'); await p.click('.ribbon .rb:has-text("Circle")');
  await p.mouse.move(...at(0.5, 0.5)); await p.mouse.down(); await p.mouse.move(...at(0.53, 0.5), { steps: 3 }); await p.mouse.up(); await p.waitForTimeout(250);
  s3 = await state();
  check("Subtract cuts a circle out of the region", s3.m.shapes.length === 2 && s3.m.shapes[1].hole === true && s3.m.shapes[1].geom.type === "circle");
  // A polygon by clicks, closed with Enter.
  await p.click('.ribbon .rb:has-text("Add")'); await p.click('.ribbon .rb:has-text("Polygon")');
  for (const [x, y] of [[0.75, 0.7], [0.85, 0.7], [0.8, 0.55]]) { await p.mouse.click(...at(x, y)); await p.waitForTimeout(40); }
  await p.keyboard.press("Enter"); await p.waitForTimeout(250);
  s3 = await state();
  check("clicks and Enter make a polygon", s3.m.shapes.length === 3 && s3.m.shapes[2].geom.pts.length === 3);
  await p.keyboard.press("Delete"); await p.waitForTimeout(150);
  check("Del removes the selected shape", (await state()).m.shapes.length === 2);
  await shot("05-drawn");
  // Boundaries: five edges are free, drawn dashed; an inlet by picking an edge, then wall the rest.
  await tab("Physics");
  check("free edges are counted in the tree", (await text(".tree")).includes("5 edges free"));
  await p.click('.ribbon .rb:has-text("Inlet")'); await p.waitForTimeout(150);
  check("a new inlet starts picking its edges", (await text(".vhint")).includes("Click the edges") && (await state()).m.boundaries.length === 1);
  // The middle of the rectangle's left edge, from the view's own transform.
  const left = await p.evaluate(g => { const [a, d] = [g.pts[0], g.pts[3]]; return window.__view.toScreen([(a[0] + d[0]) / 2, (a[1] + d[1]) / 2]); }, s3.m.shapes[0].geom);
  await p.mouse.click(box.x + left[0], box.y + left[1]); await p.waitForTimeout(200);
  s3 = await state();
  check("clicking an edge puts it on the boundary", s3.m.boundaries[0].edges.length === 1 && s3.m.boundaries[0].edges[0].shape === s3.m.shapes[0].id, JSON.stringify(s3.m.boundaries[0].edges));
  await p.keyboard.press("Escape");
  const speed = p.locator(".insp .ifield:has(label:has-text('Mean speed')) input");
  await speed.fill("0.2"); await speed.press("Enter"); await p.waitForTimeout(120);
  check("the inlet speed is set", (await state()).m.boundaries[0].kind.speed === 0.2);
  await p.click('.ribbon .rb:has-text("Wall the Rest")'); await p.waitForTimeout(200);
  s3 = await state();
  check("Wall the Rest leaves no edge without a boundary", s3.edges.every(e => e.boundary !== 0) && s3.m.boundaries.length === 2);
  await p.click('.bottom header button:has-text("Checks")'); await p.waitForTimeout(80);
  check("the Checks panel says every edge has a boundary", (await text(".bottom")).includes("Every edge has a boundary"));
  await hear(); await shot("06-boundaries");
  // Heat: a temperature without Heat is flagged; with Heat on it can be set.
  await tab("Physics"); await p.click('.ribbon .rb:has-text("Heat") >> nth=0'); await p.waitForTimeout(150);
  check("Heat turns on", (await state()).m.physics.heat === true);
  await p.click('.tree .ti:has-text("Walls")'); await p.waitForTimeout(80);
  await p.selectOption(".insp .isec:has(h3:has-text('Heat')) select", "temperature"); await p.waitForTimeout(120);
  check("a wall can be given a temperature", (await state()).m.boundaries[1].heat.type === "temperature");
  // Mesh and solver forms.
  await p.click('.tree .ti:has-text("Mesh")'); await p.waitForTimeout(80);
  await p.click(".insp .ifield:has(label:has-text('Size')) input[type=checkbox]"); await p.waitForTimeout(150);
  check("the mesh size can be set by hand", (await state()).m.mesh.size > 0);
  await p.click('.tree .ti:has-text("Solver")'); await p.waitForTimeout(80);
  const it = p.locator(".insp .ifield:has(label:has-text('Iterations')) input");
  await it.fill("60"); await it.press("Enter"); await p.waitForTimeout(120);
  check("the solver settings are edited", (await state()).m.solver.max_iter === 60);
  await p.click('.tree .ti:has-text("Results")'); await p.waitForTimeout(80);
  await p.click('.insp .madd .btn'); await p.waitForTimeout(120);
  check("a monitor is added", (await state()).m.monitors.length === 1);
  await hear();

  // ---------- save, then a damaged file: it opens as far as it can and the notes say what was lost
  await p.keyboard.press("Control+s"); await p.waitForTimeout(200);
  check("Save reports where the file went and clears the mark", (await toastText()).includes(".dflow") && !(await text(".title")).includes("•"), await text(".title"));
  const good = JSON.parse((await ask("file_text")).ok);
  good.model.shapes[1] = { id: "two" }; good.model.fluid = [1, 2, 3];
  await p.evaluate(t => { window.__files["C:/Models/Damaged.dflow"] = t; }, JSON.stringify(good));
  await p.click(".tabs .file"); await p.waitForSelector(".bs .home");
  await p.click('.recent tr:has-text("Damaged")'); await p.waitForTimeout(300);
  const opened = await state();
  check("a damaged file opens as far as possible", opened.m.shapes.length === 1 && opened.notes.some(n => n.includes("shapes")) && opened.notes.some(n => n.includes("fluid")), JSON.stringify(opened.notes));
  check("and Messages says what was left out", (await text(".bottom")).includes("could not be read") && (await text(".title")).includes("Damaged"), await text(".bottom"));
  await hear(); await shot("06b-messages");

  // ---------- Tell Me
  await p.keyboard.press("Control+k"); await p.keyboard.type("glossary"); await p.keyboard.press("Enter"); await p.waitForTimeout(150);
  check("Tell Me finds and opens the Glossary", (await text(".dlg")).includes("Reynolds number"));
  await p.fill(".gsearch", "nusselt"); await p.waitForTimeout(80);
  check("the Glossary searches", (await p.locator(".glossary dt").count()) === 1);
  await p.fill(".gsearch", ""); await hear(); await p.keyboard.press("Escape");
  await tab("Help");
  for (const [button, word] of [["Getting Started", "Draw the geometry"], ["Shortcuts", "Ctrl+K"], ["What Is This Tool?", "Model tree"]]) {
    await p.click(`.ribbon .rb:has-text("${button}")`); await p.waitForTimeout(100);
    check(`Help › ${button} opens`, (await text(".dlg")).toLowerCase().includes(word.toLowerCase())); await hear(); await p.keyboard.press("Escape");
  }
  await p.click('.ribbon .rb:has-text("Report an Issue")'); await p.waitForTimeout(50);
  check("GitHub links go only to the project's pages", (await p.evaluate(() => window.__links)).every(u => u.startsWith("https://github.com/Edgar-Mendonca/Datum-Flow")) && (await p.evaluate(() => window.__links.length)) === 1);

  // ---------- themes
  await p.click(".theme"); await p.waitForTimeout(100); // light → dark
  const dark = await p.evaluate(() => ({ theme: document.documentElement.dataset.theme, bg: getComputedStyle(document.body).backgroundColor }));
  check("the theme button switches to dark", dark.theme === "dark" && dark.bg === "rgb(11, 17, 25)", JSON.stringify(dark));
  await p.click(".tabs .file"); await p.waitForSelector(".bs .home");
  await p.click('.recent.examples tr:has-text("Heated cavity, Ra 10⁴")'); await p.waitForTimeout(300);
  check("natural convection shows its Rayleigh number", /Ra 10000\b/.test(await text(".status")), await text(".status"));
  await shot("07-dark-heated"); await hear();
  await p.click(".theme"); await p.waitForTimeout(80); await p.click(".theme"); await p.waitForTimeout(80); // auto → light
  check("and back to light", (await p.evaluate(() => document.documentElement.dataset.theme)) === "light");

  // ---------- the File page: every page opens
  await p.click(".tabs .file"); await p.waitForSelector(".bs .home");
  for (const [side, word] of [["Export", "PDF report"], ["Print", "Save as PDF"], ["Roadmap", "Transient"], ["Options", "Recovery copy"], ["Updates", "Datum Flow"]]) {
    await p.click(`.bs aside button:has-text("${side}")`); await p.waitForTimeout(150);
    check(`File › ${side} opens`, (await text(".bsmain")).toLowerCase().includes(word.toLowerCase()), (await text(".bsmain")).slice(0, 80)); await hear();
    if (side === "Print" || side === "Options" || side === "Roadmap") await shot(`08-file-${side.toLowerCase()}`);
    if (side === "Print") {
      const page = await p.evaluate(() => { const g = document.querySelector(".pprev .ppage"); return g ? { w: g.offsetWidth, h: g.offsetHeight, img: !!g.querySelector(".ppic img")?.naturalWidth, head: g.querySelector(".phead")?.innerText, foot: g.querySelector(".pfoot")?.innerText } : null; });
      check("the print preview shows an A4 landscape page with the view, a header and a footer", !!page && Math.abs(page.w / page.h - 297 / 210) < 0.01 && page.img && page.head.includes("Heated cavity") && page.head.includes("Test Works") && page.foot.includes("Page 1 of 1"), JSON.stringify(page));
      await p.click('.pset .btn.pri'); await p.waitForTimeout(600);
      check("Print lays the pages out and opens the print window", (await p.evaluate(() => window.__printed)) === 1);
      await p.evaluate(() => window.dispatchEvent(new Event("afterprint")));
    }
    if (side === "Roadmap") check("the roadmap lists the releases to 1.0", (await p.locator(".rmstage").count()) >= 8);
  }
  await p.click(".bs aside .back"); await p.waitForTimeout(100);

  // ---------- public words
  const all = spoken.join("\n");
  check("the window names no commercial product", named(all).length === 0, named(all).join(", "));
  const src = ["../ui/src", "../src-tauri/src", "../ui/index.html", "../NOTICE", "../SECURITY.md", "../CONTRIBUTING.md"].flatMap(f => { const full = path.join(__dirname, f); return fs.statSync(full).isDirectory() ? fs.readdirSync(full).map(x => path.join(full, x)) : [full]; });
  const bad = src.flatMap(f => named(fs.readFileSync(f, "utf8")).map(n => `${path.basename(f)}: ${n}`));
  check("the window's source names no commercial product", bad.length === 0, bad.join(", "));

  check("no errors in the window", errors.length === 0, errors.slice(0, 5).join(" | "));
  await b.close(); srv.close(); engine.stdin.end(); engine.kill();
  console.log(failed ? `\n${failed} check(s) failed` : "\nAll UI checks passed");
  process.exit(failed ? 1 : 0);
})().catch(e => { console.error(e); engine.kill(); process.exit(1); });
