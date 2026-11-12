//! Runs every verification case and writes the record: `crates/flow-verify/report.json` (built
//! into the app), `docs/VERIFICATION.md`, and `docs/verification/<version>.{md,json}` (kept, so
//! the next release can compare with this one). Run from the repository root, in release mode:
//! `cargo run --release -p flow-verify --bin report`.

use flow_verify::report::{markdown, not_verified, regress, Report};
use std::path::Path;

/// Today as yyyy-mm-dd (UTC), from the system clock.
fn today() -> String {
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
    // Civil date from a day count (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let (d, m) = (doy - (153 * mp + 2) / 5 + 1, if mp < 10 { mp + 3 } else { mp - 9 });
    format!("{:04}-{:02}-{:02}", yoe + era * 400 + (m <= 2) as i64, m, d)
}

fn version_key(v: &str) -> Vec<u32> {
    v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

fn main() {
    let version = env!("CARGO_PKG_VERSION");
    let dir = Path::new("docs/verification");
    if !Path::new("crates/flow-verify").is_dir() {
        eprintln!("Run this from the repository root.");
        std::process::exit(2);
    }
    // `--rewrite`: write the documents again from the record already made, without running the cases.
    if std::env::args().any(|a| a == "--rewrite") {
        let r = flow_verify::report::shipped();
        let md = markdown(&r);
        std::fs::write("docs/VERIFICATION.md", &md).expect("docs/VERIFICATION.md");
        std::fs::write(dir.join(format!("{}.md", r.version)), &md).expect("docs/verification");
        return;
    }
    let t = std::time::Instant::now();
    let mut checks = flow_verify::cases::full();
    let step = flow_verify::cases::step_lengths();
    let mut notes = not_verified(&step);
    // The newest record of an earlier version.
    let prev = std::fs::read_dir(dir).ok().into_iter().flatten().flatten()
        .filter_map(|f| f.file_name().to_str().and_then(|n| n.strip_suffix(".json")).map(str::to_string))
        .filter(|v| version_key(v) < version_key(version)).max_by_key(|v| version_key(v))
        .and_then(|v| std::fs::read_to_string(dir.join(format!("{v}.json"))).ok()).and_then(|s| serde_json::from_str::<Report>(&s).ok()).filter(|r| !r.checks.is_empty());
    match &prev {
        Some(p) => checks.extend(regress(p, &checks)),
        None => notes.push("Regression: this is the first record, so there is no earlier release to compare with. Later releases compare every value with this one.".into()),
    }
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let r = Report { version: version.into(), date: today(), checks, not_verified: notes, seconds: t.elapsed().as_secs_f64().round(), machine: format!("{threads} threads, {} {}", std::env::consts::OS, std::env::consts::ARCH) };
    let json = serde_json::to_string_pretty(&r).expect("the record") + "\n";
    let md = markdown(&r);
    std::fs::create_dir_all(dir).expect("docs/verification");
    for (path, text) in [("crates/flow-verify/report.json".to_string(), &json), ("docs/VERIFICATION.md".to_string(), &md), (format!("docs/verification/{version}.json"), &json), (format!("docs/verification/{version}.md"), &md)] {
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("{path}: {e}"));
    }
    println!("{} of {} checks pass in {:.0} s", r.passed(), r.checks.len(), r.seconds);
    for c in r.checks.iter().filter(|c| !c.pass) {
        println!("FAIL {} | {} | reference {} result {} error {:.2e} tolerance {:.2e}", c.case, c.quantity, c.reference, c.result, c.error, c.tolerance);
    }
    if r.failed() > 0 {
        std::process::exit(1);
    }
}
