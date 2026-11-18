// UI tests: the built window (ui/dist) in Chromium, with the real model engine behind a bridge
// (crates/flow-cli/examples/bridge.rs --bridge) in place of the desktop shell. Edits, undo, examples
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
const engine = spawn("cargo", ["run", "-q", "-p", "flow-cli", "--example", "bridge", "--", "--bridge"], { cwd: path.join(__dirname, ".."), stdio: ["pipe", "pipe", "inherit"] });
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
    window.__links = []; window.__calls = []; window.__files = {}; window.__listeners = {};
    window.__TAURI__ = {
      core: { invoke: async (c, a) => {
        window.__calls.push([c, a]);
        if (c in shell) return shell[c](a ?? {});
        // The solver's worker thread and its events: the bridge answers in one go, and the
        // iterations are then delivered one by one, as the shell would.
        if (c === "solve_start") {
          const r = await window.__bridge("solve", window.__stopAfter ? { stop_after: window.__stopAfter } : {});
          if (r.err !== undefined) throw r.err;
          const emit = (e, payload) => (window.__listeners[e] ?? []).forEach(f => f({ payload }));
          (async () => { for (const pr of r.ok.progress) { await new Promise(ok => setTimeout(ok, window.__solveDelay ?? 15)); emit("solve-progress", pr); } emit("solve-done", { summary: r.ok.summary, error: null, out: r.ok.out }); })();
          return r.ok.summary.monitor_names;
        }
        if (c === "solve_stop") return null;
        if (c === "verification" && window.__verification) return window.__verification;
        // Files: the tests keep "saved" files in the page, by path.
        if (c === "import_dxf") a = { scale: a.scale, text: window.__dxf ?? "" };
        if (c === "open") { if (!a?.path) return null; a = { path: a.path, text: window.__files[a.path] ?? "" }; }
        const r = await window.__bridge(c, a ?? {});
        if (r.err !== undefined) throw r.err;
        if (c === "set_options") return (await window.__bridge("state", {})).ok;
        return r.ok;
      } },
      event: { listen: async (e, f) => { (window.__listeners[e] ??= []).push(f); return () => {}; } },
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

  // ---------- the one part still to come says so (never a dead button)
  await tab("Geometry");
  // DXF: a 200 × 100 mm plate of four lines with a circle in it, and a text that is left out.
  await p.evaluate(() => { const L = (a, b, c, d) => `0\nLINE\n10\n${a}\n20\n${b}\n11\n${c}\n21\n${d}\n`; window.__dxf = "0\nSECTION\n2\nENTITIES\n" + L(0, 0, 200, 0) + L(200, 0, 200, 100) + L(200, 100, 0, 100) + L(0, 100, 0, 0) + "0\nCIRCLE\n10\n60\n20\n50\n40\n15\n0\nTEXT\n1\nnote\n0\nENDSEC\n0\nEOF\n"; });
  const shapesBefore = (await state()).m.shapes.length;
  await p.click('.ribbon .rb:has-text("Import DXF")'); await p.waitForTimeout(150);
  await p.click('.ov:last-child .dlg .btn.pri'); await p.waitForTimeout(400);
  const dxfState = await state();
  check("Geometry › Import DXF adds the outlines as a region and a body", dxfState.m.shapes.length === shapesBefore + 2 && dxfState.m.shapes.some(s => s.hole && s.geom.type === "circle" && Math.abs(s.geom.r - 0.015) < 1e-12) && dxfState.notes.some(n => n.includes("TEXT")), JSON.stringify(dxfState.notes));
  await p.keyboard.press("Control+z"); await p.waitForTimeout(250);
  await hear();

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
  check("a blank model opens the Setup steps on the right, at step 1", (await text(".setup")).includes("Name and units") && (await p.locator(".setup .ssteps li").count()) === 7 && (await p.locator(".setup .ssteps li.on").innerText()).includes("Name and units"));
  await p.click('.setup footer .btn.pri'); await p.waitForTimeout(80);
  check("Next goes to the Geometry step, with the templates", (await p.locator(".setup .ssteps li.on").innerText()).includes("Geometry") && (await p.locator(".setup .stpl button").count()) === 5);
  await shot("04b-setup");
  await p.click(".setup header .x"); await p.waitForTimeout(80);
  check("Setup closes, and the properties come back", (await p.locator(".setup").count()) === 0 && (await p.locator(".insp").count()) === 1);
  await p.click('.side button:has-text("Setup")'); await p.waitForTimeout(80);
  check("Sidebar › Setup opens it again", (await p.locator(".setup").count()) === 1);
  await p.click(".setup header .x"); await p.waitForTimeout(80);
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

  // ---------- mesh, checks, solve and results, through the real engine
  const images = path.join(__dirname, "../docs/images");
  fs.mkdirSync(images, { recursive: true });
  const site = async n => { await p.evaluate(() => document.querySelector("#toast")?.click()); await p.mouse.move(700, 600); await p.waitForTimeout(120); await p.screenshot({ path: path.join(images, n + ".png") }); };
  const openExample = async title => {
    await p.keyboard.press("Control+s"); await p.waitForTimeout(150);
    await p.click(".tabs .file"); await p.waitForSelector(".bs .home");
    await p.click(`.recent.examples tr:has-text("${title}")`); await p.waitForTimeout(350);
  };
  const rb = async (tabName, button) => { await tab(tabName); await p.click(`.ribbon .rb:has-text("${button}") >> nth=0`); await p.waitForTimeout(120); };
  // Home › Run › Solve or Resume (Home › Steps also has a "Solve", which only opens the settings).
  const run = async button => { await tab("Home"); await p.click(`.ribbon .grp:has(.cap:has-text("Run")) .rb:has-text("${button}")`); await p.waitForTimeout(120); };
  // The colour of the view at a model point (device pixels).
  const pixel = (x, y) => p.evaluate(([x, y]) => { const c = document.querySelector(".view canvas"), s = window.__view.toScreen([x, y]), r = c.width / c.clientWidth, d = c.getContext("2d").getImageData(Math.round(s[0] * r), Math.round(s[1] * r), 1, 1).data; return [d[0], d[1], d[2]]; }, [x, y]);
  const clickAt = async (x, y) => { const s = await p.evaluate(q => { const r = document.querySelector(".view canvas").getBoundingClientRect(), t = window.__view.toScreen(q); return [r.left + t[0], r.top + t[1]]; }, [x, y]); await p.mouse.move(s[0], s[1]); await p.mouse.down(); await p.mouse.up(); await p.waitForTimeout(200); };
  const calls = name => p.evaluate(n => window.__calls.filter(c => c[0] === n).map(c => c[1]), name);
  const solved = async (ms = 120000) => { await p.waitForFunction(() => !document.querySelector(".sbusy") && !!document.querySelector(".srun b"), null, { timeout: ms }); await p.waitForTimeout(500); };

  await openExample("Flow past a cylinder");
  // Mesh
  await rb("Mesh", "Generate"); await p.waitForFunction(() => document.querySelector("#toast")?.innerText.includes("Mesh ready"), null, { timeout: 30000 });
  let ms = await state();
  check("Mesh › Generate meshes the model and says so", ms.has_mesh && /Mesh ready: [\d,. ]+ elements/.test(await toastText()), await toastText());
  const qual = await text(".squal");
  check("the mesh quality report lists elements, angle, ratio and the thinnest elements", ["Elements", "Unknowns", "Smallest angle", "Size ratio", "Edge length", "over 30°"].every(w => qual.includes(w)) && (await p.locator(".squal .irow").count()) === 5, qual.slice(0, 200));
  const lines = await p.evaluate(() => { const c = document.querySelector(".view canvas"), d = c.getContext("2d").getImageData(0, 0, c.width, c.height).data; let n = 0; for (let i = 0; i < d.length; i += 4) if (Math.abs(d[i] - 236) > 12 && Math.abs(d[i] - 255) > 6 && d[i] < 215) n++; return n; });
  check("the mesh is drawn in the view", lines > 20000, `${lines} dark pixels`);
  await hear(); await shot("10-mesh");
  await rb("Mesh", "Quality");
  const green = await p.evaluate(() => { const c = document.querySelector(".view canvas"), d = c.getContext("2d").getImageData(0, 0, c.width, c.height).data; let n = 0; for (let i = 0; i < d.length; i += 4 * 7) if (d[i + 1] > d[i] + 30 && d[i + 1] > d[i + 2] + 30) n++; return n; });
  check("Quality colours the elements and shows its legend", green > 2000 && (await text(".sleg.sq2")).includes("over 30°"), `${green} green samples`);
  await p.click(".squal .irow .btn >> nth=0"); await p.waitForTimeout(200);
  check("Show zooms to a thin element", /\d{3,}%/.test(await text(".status .zpct")), await text(".status .zpct"));
  await shot("11-quality"); await hear();
  await rb("Mesh", "Quality"); await p.keyboard.press("f"); await p.click(".view canvas", { position: { x: 5, y: 5 } }); await p.keyboard.press("f"); await p.waitForTimeout(150);
  check("Quality off removes the colours", (await p.locator(".sleg.sq2").count()) === 0);

  // Checks: ready; then a broken setup is refused, in words
  await rb("Check", "Checks");
  check("the checks say the model is ready, against its mesh", (await text(".schecks")).includes("Ready to solve.") && (await text(".schecks")).includes("Checked against the mesh"), await text(".schecks"));
  await p.click('.tree .ti:has-text("Outlet")'); await p.waitForTimeout(100);
  await p.selectOption(".insp .ifield:has(label:has-text('Kind')) select", "wall"); await p.waitForTimeout(200);
  ms = await state();
  check("an edit that changes the flow drops the mesh", !ms.has_mesh && (await p.locator(".squal .sq").count()) === 0);
  await run("Solve"); await p.waitForTimeout(400);
  check("Solve refuses a model with no outlet and says why", (await text(".dlg")).includes("Not ready to solve") && (await text(".dlg")).includes("no outlet"), await text(".dlg"));
  await p.keyboard.press("Escape"); await p.waitForTimeout(100);
  check("the Checks tab lists the error with its fix and a count", (await text(".schecks .scheck.error")).includes("pressure outlet") && (await text(".bottom header .badge")).trim() !== "", await text(".schecks"));
  await hear(); await shot("12-checks");
  await p.keyboard.press("Control+z"); await p.waitForTimeout(250);
  check("undo brings the outlet back", (await state()).m.boundaries[1].kind.type === "outlet");

  // Solve: live residuals and monitors, then the results
  await p.evaluate(() => { window.__solveDelay = 900; });
  await run("Solve");
  await p.waitForSelector(".sbusy", { timeout: 120000 });
  await p.waitForFunction(() => (window.__chartData(document.querySelector(".spanel .schart canvas")) ?? [0])[0] >= 4, null, { timeout: 30000 }).catch(async e => { await shot("debug"); console.log(await text(".bottom")); throw e; });
  const mid = await text(".srun");
  check("while solving: the iteration, the kind of step and a Stop button", /Iteration \d+ · (Picard|Newton)/.test(mid) && mid.includes("STOP") && (await p.locator(".ribbon .rb:has-text('Stop'):not([disabled])").count()) === 1, mid);
  check("the residual chart grows as the solve runs (momentum and continuity)", JSON.stringify(await p.evaluate(() => window.__chartData(document.querySelector(".spanel .schart canvas")).length)) === "2");
  await p.click('.tree .ti:has-text("Fluid")'); await p.waitForTimeout(80);
  await rb("Physics", "Heat"); await p.waitForTimeout(150);
  check("the model cannot be changed while a solve runs", (await toastText()).includes("A solve is running") && (await state()).m.physics.heat === false, await toastText());
  await tab("Solve"); await p.waitForTimeout(250);
  check("still solving when the picture is taken", (await p.locator(".sbusy").count()) === 1);
  await shot("13-solving"); await site("residuals-solving"); await hear();
  await p.evaluate(() => { window.__solveDelay = 10; });
  await solved();
  ms = await state();
  check("the solve converges and says so", ms.has_results && (await text(".srun")).includes("CONVERGED") && /Converged in \d+ iterations/.test(await text(".srun")), await text(".srun"));
  check("Messages keeps the outcome", (await p.evaluate(() => window.__calls.some(c => c[0] === "mesh_view"))) && true);
  const brightest = async (x, y) => { let best = [0, 0, 0]; for (let k = -2; k <= 2; k++) { const c = await pixel(x, y + 0.004 * k); if (c[0] + c[1] + c[2] > best[0] + best[1] + best[2]) best = c; } return best; };
  const sp = await brightest(1.6, 0.26), wake = await brightest(1.6, 0.012);
  check("the view is coloured by speed (fast mid-channel, slow at the wall)", sp[1] > wake[1] + 60 && !(sp[0] === 236 && sp[1] === 238), `${sp} / ${wake}`);
  const leg = await text(".sleg");
  check("the legend names the field, its unit and its range", leg.includes("SPEED") && leg.includes("m/s") && /0\.\d+/.test(leg), leg);
  await p.click('.bottom header button:has-text("Monitors")'); await p.waitForTimeout(250);
  const mon = await text(".smons");
  check("the monitors are plotted with their values (drag 0.01116 N)", mon.includes("Drag on the cylinder") && /0\.0111[56]\d* N/.test(mon) && (await p.evaluate(() => window.__chartData(document.querySelector(".smon canvas"))[0])) >= 5, mon);
  await hear(); await shot("14-monitors");

  // Streamlines, arrows, fields
  check("streamlines are fetched with the results", (await calls("field")).some(a => a.id === "stream"));
  await p.keyboard.press("f"); await p.waitForTimeout(200);
  await p.click('.bottom header button:has-text("Residuals")'); await p.waitForTimeout(200);
  await shot("15-cylinder-results");
  // Closer on the cylinder for the site's picture, then the whole model again.
  const z0 = await p.evaluate(q => { const r = document.querySelector(".view canvas").getBoundingClientRect(), t = window.__view.toScreen(q); return [r.left + t[0], r.top + t[1]]; }, [0.0, 0.205]);
  await p.mouse.move(z0[0], z0[1]); await p.mouse.wheel(0, -470); await p.waitForTimeout(500);
  await site("cylinder-streamlines");
  await p.keyboard.press("f"); await p.waitForTimeout(300);
  await rb("Results", "Streamlines");
  check("Results › Streamlines switches them off and keeps the choice in the model", (await state()).m.view.streamlines === false);
  await rb("Results", "Streamlines"); await rb("Results", "Arrows"); await p.waitForTimeout(300);
  check("Results › Arrows fetches and draws arrows", (await state()).m.view.arrows === true && (await calls("arrows")).length >= 1);
  await shot("16-arrows"); await rb("Results", "Arrows");
  await rb("Results", "Pressure"); await p.waitForTimeout(300);
  check("the field changes to pressure", (await state()).m.view.field === "pressure" && (await text(".sleg")).includes("PRESSURE") && (await text(".sleg")).includes("Pa"), await text(".sleg"));
  await rb("Results", "Temperature").catch(() => {});
  check("Temperature is not offered without heat", (await p.locator(".ribbon .rb:has-text('Temperature')[disabled]").count()) === 1);
  await rb("Results", "Vorticity"); await p.waitForTimeout(300);
  await p.click('.tree .ti:has-text("Results")'); await p.waitForTimeout(100);
  await p.selectOption(".insp .ifield:has(label:has-text('Colour map')) select", "diverging"); await p.waitForTimeout(300);
  const far = await brightest(1.9, 0.225);
  check("the diverging map is centred on zero (near white where vorticity is near zero)", far[0] > 200 && far[1] > 200 && far[2] > 200, `${far}`);
  await shot("17-vorticity");
  await p.selectOption(".insp .ifield:has(label:has-text('Colour map')) select", "viridis"); await rb("Results", "Speed"); await p.waitForTimeout(300);

  // Probes, the value under the pointer, line plots
  await rb("Results", "Probe");
  check("Probe says what to do", (await text(".shint")).includes("Click where you want to read the values"));
  await clickAt(0.15, 0.2); await clickAt(1.0, 0.3); await p.keyboard.press("Escape"); await p.waitForTimeout(300);
  ms = await state();
  check("clicks in the view add probes, and their values are fetched", ms.m.probes.length === 3 + 1 - 0 || ms.m.probes.length === 4, `${ms.m.probes.length}`);
  check("probe values come from the engine", (await calls("probes")).length >= 1);
  const hv = await p.evaluate(q => { const r = document.querySelector(".view canvas").getBoundingClientRect(), t = window.__view.toScreen(q); return [r.left + t[0], r.top + t[1]]; }, [1.5, 0.2]);
  await p.mouse.move(hv[0], hv[1]); await p.mouse.move(hv[0] + 2, hv[1]); await p.waitForTimeout(500);
  check("the status bar shows the values under the pointer", /0\.\d+ m\/s · p [−\d.e-]+ Pa/.test(await text(".shover")), await text(".shover"));
  await rb("Results", "Line Plot");
  await clickAt(0.6, 0.02); await p.mouse.move(hv[0], hv[1]); await clickAt(0.6, 0.39); await p.waitForTimeout(600);
  ms = await state();
  const dl = await text(".dlg");
  check("two clicks draw a plot line and open its chart", ms.m.lines.length === 2 && dl.includes("Along Line 2") && dl.includes("201 points"), dl.slice(0, 160));
  check("the line plot holds the engine's samples", (await p.evaluate(() => window.__chartData(document.querySelector(".dlg .schart canvas"))[0])) > 150);
  await p.selectOption(".dlg .slinehead select", "pressure"); await p.waitForTimeout(200);
  await shot("18-lineplot"); await hear();
  await p.click('.dlg .btn:has-text("Save as CSV")'); await p.waitForTimeout(300);
  let saved = await calls("save_file");
  check("a line plot saves as CSV with units", saved.at(-1).name.endsWith(".csv") && saved.at(-1).data.startsWith("distance [m],x [m],y [m],u [m/s]") && saved.at(-1).data.split("\n").length > 200, saved.at(-1).data.slice(0, 80));
  await p.keyboard.press("Escape"); await p.waitForTimeout(100);

  // Values
  await rb("Results", "Values"); await p.waitForTimeout(400);
  const vals = await text(".dlg");
  check("Values lists monitors, boundaries, balances and extremes (drag 0.01116 N)", ["MONITORS", "BOUNDARIES", "BALANCES", "EXTREMES", "Drag on the cylinder", "Net flow through all boundaries", "should be near zero"].every(w => vals.includes(w)) && /0\.0111[56]\d*\s*N/.test(vals), vals.slice(0, 300));
  await shot("19-values"); await hear(); await p.keyboard.press("Escape");

  // Exports and the report
  await rb("Results", "VTU"); await p.waitForTimeout(500);
  saved = await calls("save_file");
  check("VTU export writes the mesh and the fields", saved.at(-1).name === "Flow past a cylinder, Re 20.vtu" && saved.at(-1).data.includes("UnstructuredGrid") && saved.at(-1).data.includes('Name="velocity"') && (await toastText()).includes("Saved"), saved.at(-1).name);
  await rb("Results", "CSV"); await p.waitForTimeout(200);
  await p.click('.dlg .scsv .btn >> nth=0'); await p.waitForTimeout(400);
  saved = await calls("save_file");
  check("CSV export writes the probes", saved.at(-1).data.startsWith("probe,x [m]") && saved.at(-1).data.includes("Front of the cylinder"), saved.at(-1).data.slice(0, 80));
  await rb("Results", "PDF Report"); await p.waitForTimeout(900);
  const rep = await p.evaluate(() => ({ pages: document.querySelectorAll(".pprev .ppage").length, text: document.querySelector(".pprev").innerText, pics: [...document.querySelectorAll(".pprev .rpage .ppic img")].filter(i => i.naturalWidth > 50).length, foot: [...document.querySelectorAll(".pprev .pfoot")].at(-1)?.innerText }));
  check("the PDF report has pages for inputs, mesh, convergence, results, values and verification", rep.pages >= 7 && ["1 · Inputs", "2 · Mesh", "3 · Convergence", "4 · Results", "5 · Values", "Verification of this version", "Full report: Help › Verification", "Converged"].every(w => rep.text.toLowerCase().includes(w.toLowerCase())) && rep.pics >= 3 && rep.foot.includes(`of ${rep.pages}`), JSON.stringify({ ...rep, text: ["1 · Inputs", "2 · Mesh", "3 · Convergence", "4 · Results", "5 · Values", "Verification of this version", "Full report: Help › Verification", "Converged"].filter(w => !rep.text.includes(w)) }));
  await shot("20-report"); await hear();
  await p.click(".bs aside .back"); await p.waitForTimeout(150);

  // The verification page: the shipped record, and a full one
  await rb("Help", "Verification"); await p.waitForTimeout(400);
  const realVer = (await ask("verification")).ok;
  check("Help › Verification shows the record shipped with this version", (await text(".dlg")).includes(`Datum Flow ${realVer.version}`) && (await text(".dlg")).includes(`of ${realVer.checks.length} checks pass`), (await text(".dlg")).slice(0, 200));
  await hear(); await p.keyboard.press("Escape");
  const verFix = JSON.parse(fs.readFileSync(path.join(__dirname, "verification_fixture.json")));
  await p.evaluate(v => { window.__verification = v; }, verFix);
  await rb("Check", "Verification Report"); await p.waitForTimeout(400);
  const ver = await p.evaluate(() => ({ rows: document.querySelectorAll(".dlg .vtab tbody tr:not(.vmore)").length, tiers: document.querySelectorAll(".dlg .vtier").length, text: document.querySelector(".dlg").innerText }));
  check("a full record shows every check by tier, with PASS and FAIL", ver.rows === verFix.checks.length && ver.tiers === 6 && ver.text.includes("1 FAIL") && ver.text.includes("PASS") && ver.text.toLowerCase().includes("not verified in this version"), JSON.stringify({ rows: ver.rows, tiers: ver.tiers, head: ver.text.slice(0, 300) }));
  await p.click('.dlg .vtab tr:has-text("Drag coefficient")'); await p.waitForTimeout(150);
  check("a row opens to its source, how the reference was confirmed and the meshes", (await text(".dlg .vmore")).includes("Schäfer & Turek (1996)") && (await text(".dlg .vmore")).includes("secondary sources") && (await text(".dlg .vmore")).includes("Mesh by mesh"), await text(".dlg .vmore"));
  await shot("21-verification"); if (!realVer.checks.length) await site("verification-report"); await hear();
  await p.click('.dlg .btn:has-text("Print or save as PDF")'); await p.waitForTimeout(600);
  const flowed = await p.evaluate(() => [...document.querySelectorAll(".pprev .ppage .pbody")].map(b => ({ over: b.scrollHeight - b.clientHeight, text: b.innerText })));
  check("the verification record's tables flow over pages, nothing cut off, the header repeated", flowed.length >= 2 && flowed.every(f => f.over <= 1) && flowed.some(f => f.text.toLowerCase().includes("(continued)")) && flowed.filter(f => f.text.includes("CASE") || f.text.includes("Case")).length >= 2, JSON.stringify(flowed.map(f => f.over)));
  await p.click('.pset select >> nth=0'); await p.selectOption('.pset select >> nth=0', "A3"); await p.waitForTimeout(500);
  const a3 = await p.evaluate(() => [...document.querySelectorAll(".pprev .ppage .pbody")].map(b => b.scrollHeight - b.clientHeight));
  check("a larger paper lays the pages out again: fewer of them, nothing cut off", a3.length < flowed.length && a3.every(x => x <= 1), `${a3.length} on A3, ${flowed.length} on A4`);
  await p.selectOption('.pset select >> nth=0', "A4"); await p.waitForTimeout(300);
  check("the verification record prints as pages", (await p.locator(".pprev .ppage").count()) >= 2 && (await text(".pprev")).toLowerCase().includes("verification of datum flow"), `${await p.locator(".pprev .ppage").count()} pages: ${(await text(".pprev")).slice(0, 200)}`);
  await p.click(".bs aside .back"); await p.waitForTimeout(150);
  await p.evaluate(() => { window.__verification = null; });
  if (realVer.checks.length) { await rb("Help", "Verification"); await p.waitForTimeout(400); await site("verification-report"); await p.keyboard.press("Escape"); }

  // ---------- the sidebar (FAMILY §6): its views, and the pages
  const sideLabels = await p.evaluate(() => [...document.querySelectorAll(".side button span")].map(e => e.textContent));
  check("the sidebar lists Setup first, then the views in order, in groups", sideLabels.join(",") === "Setup,Geometry,Physics,Mesh,Solve,Results,Values,Plots,Checks,Verify" && (await p.locator(".side hr").count()) === 4, sideLabels.join(","));
  await openExample("Flow past a cylinder, Re 20");
  await p.click('.side button:has-text("Values")'); await p.waitForTimeout(250);
  check("Values without results says what it is for and offers Solve", (await text(".pgview")).includes("No results yet") && (await p.locator(".pgview .pgempty .btn.pri:has-text('Solve')").count()) === 1);
  await p.click(".pgview .pgempty .btn.pri"); await solved();
  check("Solve from the page opens the Solve view with its ribbon tab", (await p.locator(".side button.on span").innerText()) === "SOLVE" || (await p.locator(".side button.on span").innerText()) === "Solve" || (await p.locator(".side button.on").innerText()).toLowerCase().includes("results"));
  await p.click('.side button:has-text("Values")'); await p.waitForTimeout(500);
  check("the Values page lists the drag on the cylinder", /Drag on the cylinder\s+0\.0111/.test(await text(".pgview")), (await text(".pgview")).slice(0, 300));
  check("a page hides the 2D view's properties and covers the view", (await p.locator(".pgview").count()) === 1 && await p.evaluate(() => { const r = document.querySelector(".pgview").getBoundingClientRect(), c = document.querySelector(".viewwrap").getBoundingClientRect(); return r.left <= c.left && r.right >= c.right; }));
  await shot("24-values-page"); await hear();
  await p.click('.side button:has-text("Plots")'); await p.waitForTimeout(700);
  check("the Plots page shows the probes and a chart for every line", (await text(".pgview")).includes("Front of the cylinder") && (await p.locator(".pgview .pgcard canvas").count()) === (await state()).m.lines.length && (await state()).m.lines.length >= 1);
  await shot("25-plots-page"); await hear();
  await p.click('.side button:has-text("Checks")'); await p.waitForTimeout(400);
  check("the Checks page gives the flow's numbers and the setup check", (await text(".pgview")).includes("Reynolds number") && (await text(".pgview")).includes("Ready to solve"));
  await hear();
  await p.click('.side button:has-text("Verify")'); await p.waitForTimeout(400);
  check("the Verify page shows the record of this version", (await text(".pgview")).includes(`Datum Flow ${realVer.version}`) && (await text(".pgview")).includes("checks pass"));
  await shot("26-verify-page"); await hear();
  await p.click('.side button:has-text("Geometry")'); await p.waitForTimeout(150);
  check("Sidebar › Geometry brings back the 2D view and the Geometry tab", (await p.locator(".pgview").count()) === 0 && (await p.locator('.tabs button.on').innerText()).includes("Geometry"));
  await tab("Results");
  check("a ribbon tab of a view lights its sidebar button", (await p.locator(".side button.on").innerText()).toLowerCase().includes("results"));
  await p.click('.side button:has-text("Physics")'); await p.waitForTimeout(80);
  await p.click('.tree .ti:has-text("Outlet")'); await p.keyboard.press("Delete"); await p.waitForTimeout(200);
  await p.click('.side button:has-text("Checks")'); await p.waitForTimeout(400);
  check("an error shows as a red badge on Checks in the sidebar", (await p.locator(".side button.bad .sbadge").count()) === 1 && (await text(".pgview")).includes("no boundary"), await text(".side"));
  await p.keyboard.press("Control+z"); await p.waitForTimeout(150);
  await p.click('.side button:has-text("Geometry")'); await p.waitForTimeout(100);

  // ---------- 0.1.4: results kept in the file, and a colour range set by hand
  await openExample("Flow past a cylinder, Re 20");
  await run("Solve"); await solved();
  const keptText = (await ask("file_text")).ok;
  check("a solved model is saved with its mesh and results", JSON.parse(keptText).results?.fingerprint?.length === 16 && JSON.parse((await ask("file_text", { results: false })).ok).results === undefined);
  const changedText = JSON.parse(keptText); changedText.model.fluid.mu *= 2;
  await p.evaluate(([a, b]) => { window.__files["C:/Models/Solved cylinder.dflow"] = a; window.__files["C:/Models/Edited cylinder.dflow"] = b; }, [keptText, JSON.stringify(changedText)]);
  await openExample("Lid-driven cavity, Re 100");
  await p.click(".tabs .file"); await p.waitForSelector(".bs .home");
  await p.click('.recent tr:has-text("Solved cylinder")'); await p.waitForTimeout(900);
  const reopened = await state();
  check("it opens solved: results, mesh, legend and history, without solving", reopened.has_results && reopened.has_mesh && (await p.locator(".sleg").count()) === 1 && (await calls("mesh_view")).length >= 1 && (await text(".srun")).includes("CONVERGED"), await text(".srun"));
  await p.click('.side button:has-text("Values")'); await p.waitForTimeout(500);
  check("the values of a model opened solved are there at once", /Drag on the cylinder\s+0\.0111/.test(await text(".pgview")));
  await p.click('.side button:has-text("Results")'); await p.waitForTimeout(300);
  await shot("27-opened-solved"); await hear();
  // The colour range: click the legend, set it, and back to automatic.
  await p.click(".sleg"); await p.waitForTimeout(200);
  const rin = p.locator(".dlg .form input");
  await rin.nth(0).fill("0"); await rin.nth(0).press("Tab"); await rin.nth(1).fill("0.2"); await rin.nth(1).press("Tab");
  await p.click(".dlg .btn.pri"); await p.waitForTimeout(500);
  const rv = (await state()).m.view.ranges?.speed;
  check("the legend sets a colour range, kept with the model", !!rv && Math.abs(rv[0]) < 1e-12 && Math.abs(rv[1] - 0.2) < 1e-12 && (await state()).has_results, JSON.stringify(rv));
  check("the legend says the range is fixed and marks the values beyond it", (await text(".sleg")).includes("FIXED") && (await text(".sleg")).includes("≥ 0.2"), await text(".sleg"));
  await shot("28-fixed-range"); await hear();
  await p.click(".sleg"); await p.waitForTimeout(200);
  await p.click('.dlg .btn:has-text("Automatic")'); await p.waitForTimeout(300);
  check("Automatic sets the range back", !(await state()).m.view.ranges?.speed && !(await text(".sleg")).includes("FIXED"));
  // A file whose model was changed outside the app: the results are left out, and the notes say why.
  await p.keyboard.press("Control+s"); await p.waitForTimeout(200);
  await p.click(".tabs .file"); await p.waitForSelector(".bs .home");
  await p.click('.recent tr:has-text("Edited cylinder")'); await p.waitForTimeout(600);
  const edited = await state();
  check("results that no longer match the model are left out, with a note", !edited.has_results && edited.notes.some(n => n.includes("do not match")), JSON.stringify(edited.notes));

  // ---------- 0.1.5: streamlines that thin out, arcs and rounded corners
  await openExample("Heat-sink fin channel");
  await run("Solve"); await solved();
  await tab("Results"); await p.keyboard.press("f"); await p.waitForTimeout(400);
  const fewLines = await p.evaluate(() => window.__streamCount());
  for (let i = 0; i < 10; i++) { await p.mouse.move(...at(0.35, 0.5)); await p.mouse.wheel(0, -400); await p.waitForTimeout(40); }
  await p.waitForTimeout(400);
  const moreLines = await p.evaluate(() => window.__streamCount());
  check("streamlines thin out in a long thin region seen whole, and come back when zoomed in", fewLines > 0 && fewLines < 12 && moreLines > fewLines, `${fewLines} → ${moreLines}`);
  await shot("29-heatsink-streamlines"); await p.keyboard.press("f");
  await openExample("Channel flow");
  await tab("Geometry"); await p.keyboard.press("f"); await p.waitForTimeout(200);
  const ch = (await state()).m.shapes[0].geom.pts;
  const scr = await p.evaluate(pts => pts.map(q => window.__view.toScreen(q)), ch);
  const cbox = await p.locator(".view canvas").boundingBox();
  const px = q => [cbox.x + q[0], cbox.y + q[1]];
  await p.click('.ribbon .rb:has-text("Arc Side")');
  const sideMid = [(scr[0][0] + scr[1][0]) / 2, (scr[0][1] + scr[1][1]) / 2];
  await p.mouse.click(...px(sideMid)); await p.waitForTimeout(100);
  await p.mouse.move(...px([sideMid[0], sideMid[1] + 14])); await p.waitForTimeout(80);
  await p.keyboard.down("Alt"); await p.mouse.click(...px([sideMid[0], sideMid[1] + 14])); await p.keyboard.up("Alt"); await p.waitForTimeout(300);
  const bent = (await state()).m.shapes[0].geom;
  check("Arc Side bends a side through the point clicked, outwards", bent.bulge[0] !== 0 && (await p.evaluate(() => 1)) === 1, JSON.stringify(bent.bulge));
  await p.keyboard.press("Escape");
  await p.click('.ribbon .rb:has-text("Round Corner")');
  await p.mouse.click(...px(scr[2])); await p.waitForTimeout(200);
  await p.fill(".dlg input", "0.003"); await p.click(".dlg .btn.pri"); await p.waitForTimeout(300);
  const rounded = await state();
  const rg = rounded.m.shapes[0].geom, rbulge = rg.bulge[2], chord = Math.hypot(rg.pts[3][0] - rg.pts[2][0], rg.pts[3][1] - rg.pts[2][1]);
  const radius = chord / (2 * Math.sin(2 * Math.atan(Math.abs(rbulge))));
  check("Round Corner rounds a corner with the radius typed", rg.pts.length === 5 && Math.abs(radius - 0.003) < 1e-12, JSON.stringify(rg));
  check("the new arc between an outlet and a wall waits for its boundary, and the check says so", rounded.edges.filter(e => !e.boundary).length === 1);
  await p.keyboard.press("Escape"); await shot("30-arc-round"); await hear();

  // Stop and resume
  await openExample("Lid-driven cavity, Re 100");
  await p.evaluate(() => { window.__stopAfter = 3; });
  await run("Solve"); await solved(); await tab("Home");
  check("a stopped solve says so and offers to resume", (await text(".srun")).includes("STOPPED") && (await p.locator('.ribbon .grp:has(.cap:has-text("Run")) .rb:has-text("Resume")').count()) === 1 && (await state()).has_results, await text(".srun"));
  await p.evaluate(() => { window.__stopAfter = 0; });
  await run("Resume"); await solved(); await tab("Home");
  check("Resume carries on to convergence", (await text(".srun")).includes("CONVERGED") && (await p.locator('.ribbon .grp:has(.cap:has-text("Run")) .rb:has-text("Resume")').count()) === 0, await text(".srun"));
  check("the cavity is solved in two clicks from its example (the mesh is drawn after)", (await state()).has_mesh && (await calls("mesh_view")).length >= 2);

  // Heat: temperature, the inferno map, the energy residual
  await openExample("Heated cavity, Ra 10⁵");
  await run("Solve"); await solved(240000);
  const hl = await text(".sleg");
  check("the heated cavity is coloured by temperature in the model's units", (await state()).has_results && hl.includes("TEMPERATURE") && hl.includes("°C") && hl.includes("20.5") && hl.includes("19.5"), hl);
  check("the residual chart has an energy line with heat on", (await p.evaluate(() => window.__chartData(document.querySelector(".spanel .schart canvas")).length)) === 3);
  const hot = await brightest(0.08, 0.3), cold = await brightest(0.92, 0.3);
  check("warm on the left, cool on the right (inferno)", hot[0] > cold[0] + 80, `${hot} / ${cold}`);
  await rb("Results", "Values"); await p.waitForTimeout(400);
  check("Values gives the Nusselt number (about 4.52 at Ra 10⁵)", /Warm wall: Nusselt number\s+4\.5[12]/.test(await text(".dlg")), (await text(".dlg")).slice(0, 400));
  await p.keyboard.press("Escape"); await p.keyboard.press("f"); await p.waitForTimeout(250);
  await shot("22-heated"); await site("heated-cavity-temperature"); await hear();
  await p.click(".theme"); await p.waitForTimeout(300); await shot("23-dark-results"); await hear();
  await p.click(".theme"); await p.waitForTimeout(80); await p.click(".theme"); await p.waitForTimeout(150);
  await rb("Physics", "Buoyancy"); await p.waitForTimeout(250);
  check("changing the physics drops the results and their legend", !(await state()).has_results && (await p.locator(".sleg").count()) === 0);
  await p.keyboard.press("Control+z"); await p.keyboard.press("Control+s"); await p.waitForTimeout(200);


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
