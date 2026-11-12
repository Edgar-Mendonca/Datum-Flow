//! The verification report: every check as one row, with a verdict. The record shipped with a
//! release is `report.json` beside this crate (written by `cargo run --release -p flow-verify
//! --bin report`), built into the app and shown in Help › Verification.

use serde::{Deserialize, Serialize};

/// The tiers of the report, in order.
pub const TIERS: [&str; 5] = ["Exact solutions", "Order of accuracy", "Benchmarks", "Validation against experiment", "Regression"];

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Check {
    /// 1 exact solutions, 2 order of accuracy, 3 benchmarks, 4 validation, 5 regression.
    pub tier: u8,
    pub case: String,
    pub quantity: String,
    pub reference: f64,
    pub result: f64,
    /// |result − reference|, relative to the reference unless `absolute`.
    pub error: f64,
    pub tolerance: f64,
    /// The error and tolerance are absolute differences (orders of accuracy, values near zero).
    pub absolute: bool,
    pub pass: bool,
    /// Where the reference comes from: author and year, or "exact".
    pub source: String,
    /// How the reference value was confirmed: "exact", "original paper", or "secondary sources".
    pub confirmed: String,
    /// How the tolerance was set, and anything else worth saying.
    pub note: String,
    /// Mesh convergence, coarsest first: element size and value.
    pub steps: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Report {
    pub version: String,
    /// The day the cases were run (yyyy-mm-dd).
    pub date: String,
    pub checks: Vec<Check>,
    /// What this version does not verify, said plainly.
    pub not_verified: Vec<String>,
    /// Seconds the whole suite took, and on what.
    pub seconds: f64,
    pub machine: String,
}

impl Report {
    pub fn passed(&self) -> usize { self.checks.iter().filter(|c| c.pass).count() }
    pub fn failed(&self) -> usize { self.checks.len() - self.passed() }
}

/// The record shipped with this build, as JSON text.
pub fn bundled() -> &'static str {
    include_str!("../report.json")
}

/// The record shipped with this build.
pub fn shipped() -> Report {
    serde_json::from_str(bundled()).unwrap_or_default()
}

/// What no row of the report covers, said plainly. `step` is the computed reattachment lengths.
pub fn not_verified(step: &str) -> Vec<String> {
    vec![
        format!("Validation against experiment: for the backward-facing step of Armaly et al. (1983) Datum Flow computes reattachment at {step}. The measured lengths are given in a figure of the paper, which could not be opened while this record was made, so no verdict is given. Values quoted for it elsewhere are about 3, 5 and 8 step heights."),
        "Reference values were keyed in from secondary sources (the benchmark authors' own code and results pages as quoted by others, and independent transcriptions that agree with each other), not read from the original papers. They are marked \"secondary sources\" until checked against the papers.".into(),
        "Flows that are turbulent, time-dependent, compressible or three-dimensional: this version does not solve them.".into(),
        "Symmetry and slip edges, heat-flux boundaries, moving walls other than a lid, and the light-oil and US-unit paths have unit tests but no benchmark row.".into(),
        "The Reynolds and Rayleigh limits of the turbulence warning are rules of thumb, not verified transition points.".into(),
    ]
}

/// Compare with the record of the release before: every value that moved gets a row, and one row
/// counts the values that did not.
pub fn regress(prev: &Report, checks: &[Check]) -> Vec<Check> {
    let mut rows = vec![];
    let (mut same, mut compared) = (0usize, 0usize);
    for c in checks.iter().filter(|c| c.tier < 5) {
        let Some(p) = prev.checks.iter().find(|p| p.tier == c.tier && p.case == c.case && p.quantity == c.quantity) else { continue };
        compared += 1;
        let scale = p.result.abs().max(c.reference.abs()).max(c.tolerance);
        if (c.result - p.result).abs() <= 1e-6 * scale {
            same += 1;
            continue;
        }
        rows.push(Check {
            tier: 5, case: format!("Changed since {}", prev.version), quantity: format!("{}: {}", c.case, c.quantity), reference: p.result, result: c.result,
            error: (c.result - p.result).abs() / p.result.abs().max(1e-300), tolerance: c.tolerance, absolute: false, pass: c.pass, source: format!("Datum Flow {}", prev.version), confirmed: "exact".into(),
            note: "The value differs from the last release's. It passes if the new value is still within its own tolerance.".into(), steps: vec![],
        });
    }
    rows.insert(0, Check {
        tier: 5, case: format!("Compared with {}", prev.version), quantity: "Values unchanged since the last release, of those compared".into(), reference: compared as f64, result: same as f64,
        error: (compared - same) as f64, tolerance: compared as f64, absolute: true, pass: true, source: format!("Datum Flow {}", prev.version), confirmed: "exact".into(),
        note: "Each value that changed is listed below with its old and new value.".into(), steps: vec![],
    });
    rows
}

fn n(v: f64) -> String {
    flow_core::units::num(v, 7)
}
fn e(v: f64) -> String {
    if v == 0.0 { "0".into() } else { format!("{v:.1e}") }
}

/// The record as a Markdown document (docs/VERIFICATION.md).
pub fn markdown(r: &Report) -> String {
    let mut o = String::new();
    o.push_str("# Datum Flow: verification report\n\n");
    o.push_str(&format!("Version **{}**, run on {} ({}; {:.0} s). **{} of {} checks pass**{}.\n\n", r.version, r.date, r.machine, r.seconds, r.passed(), r.checks.len(), if r.failed() == 0 { String::new() } else { format!(", {} FAIL", r.failed()) }));
    o.push_str("Every release of Datum Flow solves the problems below, whose answers are known, and compares. Any FAIL blocks the release. The same record is in the app under Help › Verification, and the cases are the test suite of `crates/flow-verify` (`cargo test -p flow-verify --release -- --include-ignored`).\n\n");
    o.push_str("## How to read it\n\n");
    o.push_str("- **Exact solutions** must be reproduced to round-off (1e-8): they lie in the space of the elements.\n");
    o.push_str("- **Order of accuracy:** the error must fall at the method's formal rate as the mesh is halved (3 for velocity and temperature, 2 for pressure, within ±0.1), by the method of manufactured solutions (Salari & Knupp 2000) and on Kovasznay's exact solution (1948).\n");
    o.push_str("- **Benchmarks** are published high-accuracy results. Each quantity is computed on three meshes (refinement ratio 1.5); the error of the finest must be within twice its Grid Convergence Index (Celik et al. 2008: safety factor 1.25 on the observed order), or within the reference's own stated accuracy where that is wider. Where the three values do not converge steadily the order cannot be established, and the band is three times the largest change between meshes instead.\n");
    o.push_str("- **Error** is relative to the reference, except where marked *abs* (orders of accuracy and values whose reference is zero).\n");
    o.push_str("- **Reference** says how each published number was confirmed. \"Secondary sources\" means it was not read from the original paper.\n\n");
    for (i, name) in TIERS.iter().enumerate() {
        let rows: Vec<&Check> = r.checks.iter().filter(|c| c.tier as usize == i + 1).collect();
        if rows.is_empty() {
            continue;
        }
        o.push_str(&format!("## {}. {}\n\n| Case | Quantity | Reference | Datum Flow | Error | Tolerance | | Source |\n| --- | --- | ---: | ---: | ---: | ---: | --- | --- |\n", i + 1, name));
        for c in &rows {
            let abs = if c.absolute { " abs" } else { "" };
            let conf = if c.confirmed == "exact" || c.confirmed.is_empty() { String::new() } else { format!(" *({})*", c.confirmed) };
            o.push_str(&format!("| {} | {} | {} | {} | {}{abs} | {}{abs} | **{}** | {}{conf} |\n", c.case, c.quantity, flow_core::units::num(c.reference, 12), n(c.result), e(c.error), e(c.tolerance), if c.pass { "PASS" } else { "FAIL" }, c.source));
        }
        o.push('\n');
        let notes: Vec<&&Check> = rows.iter().filter(|c| !c.note.is_empty() || !c.steps.is_empty()).collect();
        if !notes.is_empty() {
            o.push_str("Notes:\n\n");
            for c in notes {
                let steps = if c.steps.is_empty() { String::new() } else { format!(" Meshes, coarse to fine (element size → value): {}.", c.steps.iter().map(|s| format!("{} → {}", flow_core::units::num(s[0], 3), n(s[1]))).collect::<Vec<_>>().join("; ")) };
                o.push_str(&format!("- **{}: {}.** {}{}\n", c.case, c.quantity, c.note, steps));
            }
            o.push('\n');
        }
    }
    o.push_str("## Not verified\n\n");
    for line in &r.not_verified {
        o.push_str(&format!("- {line}\n"));
    }
    o.push_str("\n## Sources\n\n");
    o.push_str("- M. Schäfer, S. Turek, F. Durst, E. Krause, R. Rannacher (1996), \"Benchmark computations of laminar flow around a cylinder\", in *Flow Simulation with High-Performance Computers II*, Notes on Numerical Fluid Mechanics, Vieweg, 547–566. Reference values for case 2D-1 from the benchmark's results pages (TU Dortmund).\n");
    o.push_str("- O. Botella, R. Peyret (1998), \"Benchmark spectral results on the lid-driven cavity flow\", *Computers & Fluids* 27(4), 421–433.\n");
    o.push_str("- G. de Vahl Davis (1983), \"Natural convection of air in a square cavity: a bench mark numerical solution\", *International Journal for Numerical Methods in Fluids* 3, 249–264.\n");
    o.push_str("- B. F. Armaly, F. Durst, J. C. F. Pereira, B. Schönung (1983), \"Experimental and theoretical investigation of backward-facing step flow\", *Journal of Fluid Mechanics* 127, 473–496.\n");
    o.push_str("- L. I. G. Kovasznay (1948), \"Laminar flow behind a two-dimensional grid\", *Mathematical Proceedings of the Cambridge Philosophical Society* 44(1), 58–62.\n");
    o.push_str("- K. Salari, P. Knupp (2000), *Code Verification by the Method of Manufactured Solutions*, Sandia National Laboratories report SAND2000-1444.\n");
    o.push_str("- I. B. Celik, U. Ghia, P. J. Roache, C. J. Freitas, H. Coleman, P. E. Raad (2008), \"Procedure for estimation and reporting of uncertainty due to discretization in CFD applications\", *Journal of Fluids Engineering* 130(7), 078001.\n");
    o
}
