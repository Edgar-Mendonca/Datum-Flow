//! The verification cases as tests: any FAIL fails the build. The slow benchmarks are ignored by
//! default and run nightly and before every release (`--include-ignored`, in release mode).

use crate::cases;
use crate::report::*;

fn assert_pass(rows: &[Check]) {
    let bad: Vec<String> = rows.iter().filter(|c| !c.pass).map(|c| format!("{} | {} | reference {} result {} error {:.3e} tolerance {:.3e} | {}", c.case, c.quantity, c.reference, c.result, c.error, c.tolerance, c.note)).collect();
    assert!(bad.is_empty(), "verification FAILED:\n{}", bad.join("\n"));
    assert!(!rows.is_empty());
}

#[test]
fn exact_solutions_are_reproduced_to_round_off() {
    let mut rows = cases::couette();
    rows.extend(cases::poiseuille());
    rows.extend(cases::conduction());
    assert_pass(&rows);
    assert!(rows.iter().all(|c| c.tier == 1 && c.tolerance <= 1e-8));
}

#[test]
fn errors_fall_at_the_formal_order() {
    // One mesh level fewer than the report, to keep unoptimised test runs short; the report's
    // levels run in `the_full_suite_passes`.
    let rows = cases::manufactured(&[8, 16, 32]);
    assert_pass(&rows);
    assert!(rows.len() == 3 && rows.iter().all(|c| c.tolerance == 0.1 && c.steps.len() == 3));
}

#[test]
fn the_cylinder_benchmark_passes() {
    let rows = cases::cylinder();
    assert_pass(&rows);
    // Beyond its band, the drag is right to four figures even on these coarse meshes.
    assert!(rows[0].error < 1e-4 && rows[0].steps.len() == 3, "{:?}", rows[0]);
}

#[test]
#[ignore = "minutes: run in release mode, nightly and before a release"]
fn the_full_suite_passes() {
    let rows = cases::full();
    assert_pass(&rows);
    // Every row of the record shipped with this build is still produced, and still passes.
    let shipped = shipped();
    for s in shipped.checks.iter().filter(|c| c.tier < 5) {
        let now = rows.iter().find(|c| c.tier == s.tier && c.case == s.case && c.quantity == s.quantity).unwrap_or_else(|| panic!("the shipped record has a row the suite no longer makes: {} | {}", s.case, s.quantity));
        assert!(now.pass);
    }
    assert_eq!(rows.len(), shipped.checks.iter().filter(|c| c.tier < 5).count(), "the suite has rows the shipped record lacks: run the report again");
}

#[test]
fn the_shipped_record_is_current_and_has_no_failures() {
    let r = shipped();
    assert_eq!(r.version, env!("CARGO_PKG_VERSION"), "run `cargo run --release -p flow-verify --bin report` for this version");
    assert!(r.failed() == 0 && r.checks.len() >= 45, "{} checks, {} failed", r.checks.len(), r.failed());
    // The never-cut list: the cylinder and cavity benchmarks, mass conservation, every tier 1 to 3.
    for need in ["Cylinder in a channel, Re 20", "Lid-driven cavity, Re 1000", "Heated cavity, Ra 1e6", "Plane Poiseuille flow", "Manufactured solution: flow, heat and buoyancy", "Backward-facing step, Re 389"] {
        assert!(r.checks.iter().any(|c| c.case == need), "{need} is missing from the record");
    }
    assert!(r.checks.iter().any(|c| c.quantity.starts_with("Net flow through all boundaries")));
    assert!(!r.not_verified.is_empty() && markdown(&r).contains("checks pass"));
}

#[test]
fn a_changed_value_is_flagged_against_the_last_release() {
    let old = Report { version: "0.0.9".into(), checks: vec![
        Check { tier: 3, case: "A".into(), quantity: "x".into(), reference: 1.0, result: 1.001, tolerance: 0.01, pass: true, ..Default::default() },
        Check { tier: 3, case: "A".into(), quantity: "y".into(), reference: 2.0, result: 2.0, tolerance: 0.01, pass: true, ..Default::default() },
    ], ..Default::default() };
    let mut now = old.checks.clone();
    now[0].result = 1.004;
    now.push(Check { tier: 1, case: "New".into(), quantity: "z".into(), pass: true, ..Default::default() });
    let rows = regress(&old, &now);
    assert!(rows.len() == 2 && rows[0].reference == 2.0 && rows[0].result == 1.0 && rows[0].pass);
    assert!(rows[1].case == "Changed since 0.0.9" && rows[1].quantity == "A: x" && rows[1].reference == 1.001 && rows[1].result == 1.004 && rows[1].pass);
    // A value that moved out of its tolerance fails the regression row too.
    now[0].pass = false;
    assert!(!regress(&old, &now)[1].pass);
}
