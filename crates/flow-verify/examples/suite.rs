//! Prints the suite's rows while the cases are being tuned: `cargo run --release -p flow-verify --example suite [full]`.
fn main() {
    let full = std::env::args().any(|a| a == "full");
    let t = std::time::Instant::now();
    let rows = if full { flow_verify::cases::full() } else { flow_verify::cases::quick() };
    for c in &rows {
        println!("{} T{} {} | {} | ref {:.9} got {:.9} err {:.2e} tol {:.2e} | {}", if c.pass { "PASS" } else { "FAIL" }, c.tier, c.case, c.quantity, c.reference, c.result, c.error, c.tolerance, c.steps.iter().map(|s| format!("{:.3e}", s[1])).collect::<Vec<_>>().join(" "));
    }
    println!("{:.1}s", t.elapsed().as_secs_f64());
}
