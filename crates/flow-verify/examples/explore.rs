//! Scratch runs while tuning the benchmark meshes: prints key values on a series of meshes.
use flow_core::examples;
use flow_core::model::*;
use flow_verify::run::*;

fn main() {
    let which = std::env::args().nth(1).unwrap_or_default();
    let factors: Vec<f64> = std::env::args().skip(2).filter_map(|a| a.parse().ok()).collect();
    let factors = if factors.is_empty() { vec![1.0, 1.5, 2.25] } else { factors };
    for f in factors {
        let t = std::time::Instant::now();
        let base = match which.as_str() {
            "cylinder" => examples::cylinder(),
            "cavity" => examples::cavity(1000.0),
            "cavity100" => examples::cavity(100.0),
            "step" => examples::step(100.0),
            w if w.starts_with("heated") => examples::heated_cavity(w[6..].parse().unwrap_or(1e4)),
            _ => examples::channel(),
        };
        let m = refined(&base, f);
        match solve_model(&m) {
            Err(e) => println!("×{f}: FAILED {e}"),
            Ok(r) => {
                let v = r.values();
                let loc = r.locator();
                let (net, inflow) = v.balance();
                print!("×{f}: {} el, {} unk, {} its, {:.1}s, h={:.4e}, mass {:.1e}", r.mesh.tris.len(), r.solved.stats.unknowns, r.solved.history.len(), t.elapsed().as_secs_f64(), r.h(), net / inflow.max(1e-300));
                match which.as_str() {
                    "cylinder" => {
                        let f = v.drag_lift(6);
                        let dp = flow_post::values::sample(&r.mesh, &r.solved.state, &loc, [0.15, 0.2]).pressure - flow_post::values::sample(&r.mesh, &r.solved.state, &loc, [0.25, 0.2]).pressure;
                        println!("  cD {:.8} cL {:.9} dp {:.8}", f[0] * 500.0, f[1] * 500.0, dp);
                    }
                    w if w.starts_with("cavity") => {
                        let psi = flow_post::stream_function(&r.mesh, &r.solved.state, &mut flow_solve::SparseLu::default()).unwrap();
                        let (i, min) = psi.iter().enumerate().fold((0, f64::INFINITY), |a, (i, p)| if *p < a.1 { (i, *p) } else { a });
                        println!("  psi_min {:.7} at {:?}", min, r.mesh.nodes[i]);
                    }
                    w if w.starts_with("heated") => {
                        println!("  Nu hot {:.5} cold {:.5}", v.nusselt(2), v.nusselt(3));
                    }
                    _ => println!(),
                }
            }
        }
    }
    let _ = Continuation::Auto;
}
