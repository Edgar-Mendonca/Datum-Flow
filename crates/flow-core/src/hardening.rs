//! The hardening standard (FAMILY §11) for the model: failed edits change nothing, random and
//! hostile documents never panic and keep their invariants, damaged files open or explain.

use crate::doc::{Doc, Op};
use crate::examples::{example, EXAMPLES};
use crate::file;
use crate::model::*;

/// A small generator (no extra crate): xorshift.
pub(crate) struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn f(&mut self) -> f64 { (self.next() >> 11) as f64 / (1u64 << 53) as f64 }
    pub fn pick(&mut self, n: usize) -> usize { (self.next() % n.max(1) as u64) as usize }
    /// A value that is sometimes impossible on purpose.
    pub fn hostile(&mut self) -> f64 {
        match self.pick(12) {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => f64::NEG_INFINITY,
            3 => 0.0,
            4 => -1.0,
            5 => 1e308,
            6 => -1e308,
            7 => 1e-320,
            _ => (self.f() - 0.3) * 10.0,
        }
    }
}

fn invariants(m: &Model) {
    let mut ids = std::collections::HashSet::new();
    assert!(m.shapes.iter().all(|s| s.id != 0 && ids.insert(s.id) && s.geom.problem().is_none() && s.edges.len() == s.geom.edges()));
    assert!(m.boundaries.iter().all(|b| b.id != 0 && ids.insert(b.id)));
    let all = m.all_edges();
    let mut used = std::collections::HashSet::new();
    assert!(m.boundaries.iter().flat_map(|b| &b.edges).all(|e| all.contains(e) && used.insert(*e)), "an edge is in two boundaries or does not exist");
    assert!(m.fluid.rho > 0.0 && m.fluid.mu > 0.0 && m.fluid.k > 0.0 && m.fluid.cp > 0.0 && m.physics.depth > 0.0 && m.solver.tol > 0.0 && m.solver.max_iter > 0);
    assert!(!m.physics.buoyancy || m.physics.heat);
    let text = file::write(m);
    assert!(!text.contains("null"), "a non-finite number reached the file");
    let (back, _) = file::read(&text).expect("a written model reads back");
    assert_eq!(&back, m);
}

fn random_op(r: &mut Rng, m: &Model) -> Op {
    let h = |r: &mut Rng| r.hostile();
    let sid = m.shapes.get(r.pick(m.shapes.len())).map(|s| s.id).unwrap_or(r.pick(5) as u32);
    let bid = m.boundaries.get(r.pick(m.boundaries.len())).map(|b| b.id).unwrap_or(r.pick(5) as u32);
    let geom = |r: &mut Rng| match r.pick(4) {
        0 => Geom::Circle { c: [h(r), h(r)], r: h(r) },
        1 => Geom::rect(h(r), h(r), h(r), h(r)),
        2 => Geom::Poly { pts: (0..r.pick(7)).map(|_| [h(r), h(r)]).collect(), bulge: (0..r.pick(4)).map(|_| h(r)).collect() },
        _ => { let (x, y, w) = (r.f() * 4.0, r.f() * 4.0, 0.2 + r.f()); Geom::rect(x, y, x + w, y + w) }
    };
    let edges = |r: &mut Rng| (0..r.pick(4)).map(|_| EdgeRef { shape: sid, edge: r.pick(6) as u32 }).collect::<Vec<_>>();
    match r.pick(20) {
        0 => Op::SetName { name: if r.pick(3) == 0 { String::new() } else { "Case \u{0} α".into() } },
        1 => Op::SetUnits { units: if r.pick(2) == 0 { Units::SI } else { Units::US } },
        2 => Op::SetGrid { grid: h(r) },
        3 | 4 => Op::AddShape { name: String::new(), hole: r.pick(3) == 0, geom: geom(r), edges: vec![] },
        5 => Op::SetGeom { id: sid, geom: geom(r) },
        6 => Op::MoveShape { id: sid, dx: h(r), dy: h(r) },
        7 => Op::RemoveShape { id: sid },
        8 => Op::NameEdge { shape: sid, edge: r.pick(6) as u32, name: "Edge".into() },
        9 => Op::SetFluid { fluid: Fluid { rho: h(r), mu: h(r), k: h(r), cp: h(r), beta: h(r), at: h(r), ..Fluid::default() } },
        10 => Op::SetPhysics { physics: Physics { heat: r.pick(2) == 0, buoyancy: r.pick(2) == 0, gravity: [h(r), h(r)], t_ref: h(r), length: h(r), depth: h(r) } },
        11 | 12 => Op::SetBoundary { boundary: Boundary {
            id: if r.pick(2) == 0 { 0 } else { bid }, name: String::new(), edges: edges(r),
            kind: match r.pick(4) { 0 => BcKind::Inlet { speed: h(r), profile: Profile::Parabolic }, 1 => BcKind::Outlet { pressure: h(r) }, 2 => BcKind::Wall { u: [h(r), h(r)] }, _ => BcKind::Symmetry },
            heat: match r.pick(3) { 0 => HeatBc::Temperature { t: h(r) }, 1 => HeatBc::Flux { q: h(r) }, _ => HeatBc::Insulated },
        } },
        13 => Op::RemoveBoundary { id: bid },
        14 => Op::WallTheRest,
        15 => Op::SetMesh { mesh: MeshSettings { size: h(r), wall: h(r), corner: h(r), wake: h(r), growth: h(r), min_angle: h(r), refine: vec![Refine { at: Target::Point { x: h(r), y: h(r) }, size: h(r) }] } },
        16 => Op::SetSolver { solver: SolverSettings { max_iter: r.pick(3) as u32 * 1_000_000, tol: h(r), picard: r.next() as u32, grad_div: h(r), ..Default::default() } },
        17 => Op::SetMonitors { monitors: vec![Monitor { name: "m".into(), kind: MonitorKind::PressureDrop { from: bid, to: r.pick(9) as u32 } }, Monitor { name: "s".into(), kind: MonitorKind::Speed { x: h(r), y: h(r) } }] },
        18 => Op::SetProbes { probes: vec![Probe { name: "p".into(), x: h(r), y: h(r) }] },
        _ => Op::SetLines { lines: vec![Line { name: "l".into(), a: [h(r), h(r)], b: [h(r), h(r)], n: r.next() as u32 }] },
    }
}

#[test]
fn a_failed_edit_leaves_the_document_exactly_as_it_was() {
    let mut d = Doc::new(example("cylinder").unwrap());
    let before = d.m.clone();
    let bad = [
        Op::SetName { name: "  ".into() },
        Op::AddShape { name: "x".into(), hole: false, geom: Geom::Circle { c: [0.0, 0.0], r: -1.0 }, edges: vec![] },
        Op::AddShape { name: "x".into(), hole: false, geom: Geom::poly(&[[0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 1.0]]), edges: vec![] },
        Op::AddShape { name: "x".into(), hole: false, geom: Geom::poly(&[[0.0, 0.0], [0.0, 0.0], [1.0, 0.0]]), edges: vec![] },
        Op::SetGeom { id: 99, geom: Geom::rect(0.0, 0.0, 1.0, 1.0) },
        Op::RemoveShape { id: 99 },
        Op::SetFluid { fluid: Fluid { mu: 0.0, ..Fluid::default() } },
        Op::SetFluid { fluid: Fluid { rho: f64::NAN, ..Fluid::default() } },
        Op::SetBoundary { boundary: Boundary { id: 77, ..Default::default() } },
        Op::SetBoundary { boundary: Boundary { kind: BcKind::Inlet { speed: -1.0, profile: Profile::Uniform }, ..Default::default() } },
        Op::RemoveBoundary { id: 77 },
        Op::WallTheRest,
        Op::SetMesh { mesh: MeshSettings { size: -1.0, ..Default::default() } },
        Op::SetSolver { solver: SolverSettings { max_iter: 0, ..Default::default() } },
        Op::SetLines { lines: vec![Line { a: [1.0, 1.0], b: [1.0, 1.0], ..Default::default() }] },
    ];
    for op in bad {
        let text = format!("{op:?}");
        assert!(d.apply(op).is_err(), "{text} should fail");
        assert!(d.m == before && !d.can_undo() && d.revision == 0, "{text} changed the document");
    }
}

#[test]
fn edits_undo_and_redo_exactly() {
    let mut d = Doc::new(example("channel").unwrap());
    let start = d.m.clone();
    d.apply(Op::AddShape { name: "Post".into(), hole: true, geom: Geom::Circle { c: [0.05, 0.005], r: 0.002 }, edges: vec![] }).unwrap();
    let post = d.m.shapes.last().unwrap().id;
    assert!(d.m.shapes.last().unwrap().edges.len() == 1 && post == 5);
    d.apply(Op::SetBoundary { boundary: Boundary { name: "Post".into(), edges: vec![EdgeRef { shape: post, edge: 0 }], ..Default::default() } }).unwrap();
    // Moving an edge to another boundary takes it out of the first.
    d.apply(Op::SetBoundary { boundary: Boundary { id: 4, name: "Plates".into(), edges: vec![EdgeRef { shape: 1, edge: 0 }, EdgeRef { shape: 1, edge: 2 }, EdgeRef { shape: post, edge: 0 }], ..Default::default() } }).unwrap();
    assert!(d.m.boundaries.last().unwrap().edges.is_empty() && d.m.boundary(4).unwrap().edges.len() == 3);
    let end = d.m.clone();
    // Removing the shape drops its edge from the boundary.
    d.apply(Op::RemoveShape { id: post }).unwrap();
    assert_eq!(d.m.boundary(4).unwrap().edges.len(), 2);
    assert!(d.undo() && d.m == end);
    while d.undo() {}
    assert_eq!(d.m, start);
    while d.redo() {}
    assert_eq!(d.m.shapes.len(), 1);
    // An edit that changes nothing adds no undo step.
    let r = d.revision;
    d.apply(Op::SetUnits { units: Units::SI }).unwrap();
    assert_eq!(d.revision, r);
}

#[test]
fn random_and_hostile_edits_never_panic_and_keep_the_invariants() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    for round in 0..40 {
        let mut d = Doc::new(example(EXAMPLES[round % EXAMPLES.len()].0).unwrap());
        for _ in 0..150 {
            let op = random_op(&mut r, &d.m);
            let before = d.m.clone();
            if d.apply(op).is_err() {
                assert!(d.m == before);
            }
            if r.pick(9) == 0 { d.undo(); }
            if r.pick(13) == 0 { d.redo(); }
        }
        invariants(&d.m);
    }
}

#[test]
fn hostile_models_are_made_safe_on_loading() {
    // Every number impossible, every reference dangling.
    let bad = f64::NAN;
    let mut m = Model {
        grid: bad,
        shapes: vec![
            Shape { id: 1, geom: Geom::Circle { c: [bad, 0.0], r: 1.0 }, ..Default::default() },
            Shape { id: 1, ..Default::default() },
            Shape { id: 0, ..Default::default() },
            Shape { id: 2, geom: Geom::Poly { pts: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]], bulge: vec![f64::INFINITY] }, edges: vec!["a".into(); 9], ..Default::default() },
        ],
        fluid: Fluid { rho: -1.0, mu: bad, k: 0.0, cp: f64::INFINITY, beta: 1e9, at: -5.0, ..Fluid::default() },
        physics: Physics { heat: false, buoyancy: true, gravity: [bad, f64::INFINITY], t_ref: 0.0, length: -1.0, depth: 0.0 },
        boundaries: vec![
            Boundary { id: 2, ..Default::default() },
            Boundary { id: 5, edges: vec![EdgeRef { shape: 2, edge: 0 }, EdgeRef { shape: 2, edge: 0 }, EdgeRef { shape: 9, edge: 0 }, EdgeRef { shape: 2, edge: 77 }], kind: BcKind::Inlet { speed: bad, profile: Profile::Uniform }, heat: HeatBc::Temperature { t: -3.0 }, ..Default::default() },
            Boundary { id: 6, edges: vec![EdgeRef { shape: 2, edge: 0 }], kind: BcKind::Wall { u: [bad, 1e300] }, heat: HeatBc::Flux { q: bad }, ..Default::default() },
        ],
        mesh: MeshSettings { size: bad, wall: 0.0, corner: 9.0, wake: bad, growth: 0.0, min_angle: 89.0, refine: vec![Refine { at: Target::Point { x: bad, y: 0.0 }, size: 1.0 }, Refine { at: Target::Point { x: 0.0, y: 0.0 }, size: -1.0 }] },
        solver: SolverSettings { max_iter: 0, tol: 0.0, picard: u32::MAX, grad_div: bad, ..Default::default() },
        monitors: vec![Monitor { name: "x".into(), kind: MonitorKind::Drag { boundary: 99 } }],
        probes: vec![Probe { name: "p".into(), x: bad, y: 0.0 }],
        lines: vec![Line { n: 0, ..Default::default() }, Line { a: [bad, 0.0], ..Default::default() }],
        ..Default::default()
    };
    let notes = m.normalize();
    assert!(notes.len() >= 8, "{notes:?}");
    invariants(&m);
    assert!(m.shapes.len() == 1 && m.boundaries.len() == 2 && m.boundaries[0].edges.len() == 1 && m.boundaries[1].edges.is_empty() && m.monitors.is_empty() && m.lines.len() == 1 && m.lines[0].n == 2);
    // Normalising again changes nothing more.
    let again = m.clone();
    assert!(m.normalize().is_empty() && m == again);
}

#[test]
fn damaged_files_open_as_far_as_possible_or_explain() {
    let good = file::write(&example("cylinder").unwrap());
    // Cut short anywhere: a clear error, never a panic.
    let mut r = Rng(7);
    for cut in (0..good.len()).step_by(37) {
        if let Some(part) = good.get(..cut) {
            match file::read(part) {
                Ok((m, _)) => invariants(&m),
                Err(e) => assert!(e.contains("damaged") || e.contains("empty") || e.contains("not a Datum Flow"), "{e}"),
            }
        }
    }
    // Garbled bytes.
    for _ in 0..300 {
        let mut b = good.clone().into_bytes();
        for _ in 0..1 + r.pick(6) {
            let i = r.pick(b.len());
            b[i] = b"{}[]\":,0x9-e \n"[r.pick(14)];
        }
        if let Ok(s) = String::from_utf8(b) {
            if let Ok((m, _)) = file::read(&s) { invariants(&m); }
        }
    }
    // Not ours, wrong shapes of JSON, a newer format.
    for (text, want) in [("", "empty"), ("[1,2]", "not a Datum Flow"), ("{\"app\":\"Other\"}", "not a Datum Flow"), ("{\"app\":\"Datum Flow\"}", "format"), ("{\"app\":\"Datum Flow\",\"format\":1}", "no model"), ("{\"app\":\"Datum Flow\",\"format\":1,\"model\":7}", "no model")] {
        let e = file::read(text).unwrap_err();
        assert!(e.contains(want), "{text}: {e}");
    }
    // One broken shape and a broken fluid: the rest opens, and the notes say what was left out.
    let mut v: serde_json::Value = serde_json::from_str(&good).unwrap();
    v["model"]["shapes"][1] = serde_json::json!({ "id": "two" });
    v["model"]["fluid"] = serde_json::json!([1, 2, 3]);
    v["model"]["boundaries"] = serde_json::json!("none");
    v["format"] = serde_json::json!(99);
    let (m, notes) = file::read(&v.to_string()).unwrap();
    invariants(&m);
    assert!(m.shapes.len() == 1 && m.fluid == Fluid::default() && m.boundaries.is_empty());
    assert!(notes.iter().any(|n| n.contains("1 of the shapes")) && notes.iter().any(|n| n.contains("fluid")) && notes.iter().any(|n| n.contains("newer version")) && notes.iter().any(|n| n.contains("boundaries")), "{notes:?}");
    // A byte-order mark in front is fine.
    assert!(file::read(&format!("\u{feff}{good}")).is_ok());
    // Deeply nested rubbish does not overflow the stack.
    let deep = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    assert!(file::read(&deep).is_err());
}

#[test]
fn a_large_model_stays_quick() {
    // 400 shapes with a boundary each: every edit copies and checks the model.
    let mut d = Doc::new(Model::default());
    let t = std::time::Instant::now();
    for i in 0..400 {
        let x = (i % 20) as f64 * 2.0;
        let y = (i / 20) as f64 * 2.0;
        d.apply(Op::AddShape { name: String::new(), hole: i > 0, geom: Geom::rect(x, y, x + 1.0, y + 1.0), edges: vec![] }).unwrap();
        let id = d.m.shapes.last().unwrap().id;
        d.apply(Op::SetBoundary { boundary: Boundary { edges: (0..4).map(|e| EdgeRef { shape: id, edge: e }).collect(), ..Default::default() } }).unwrap();
    }
    let text = file::write(&d.m);
    let (back, notes) = file::read(&text).unwrap();
    assert!(back == d.m && notes.is_empty());
    let limit = if cfg!(debug_assertions) { 20.0 } else { 5.0 };
    assert!(t.elapsed().as_secs_f64() < limit, "{:?}", t.elapsed());
}
