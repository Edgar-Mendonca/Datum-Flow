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
