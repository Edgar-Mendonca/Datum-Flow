//! Reattachment length of the backward-facing step against mesh size and inlet length (tuning).
use flow_core::examples;
use flow_core::model::*;
use flow_verify::run::*;
fn main() {
    for re in [100.0, 150.0, 300.0, 389.0] {
        for (f, up) in [(1.0, 4.0), (1.5, 4.0), (2.25, 4.0), (1.5, 10.0)] {
            let mut m = examples::step(re);
            if up != 4.0 {
                let s = 0.0049;
                if let Geom::Poly { pts, .. } = &mut m.shapes[0].geom { pts[0][0] = -up * s; pts[5][0] = -up * s; }
            }
            let m = refined(&m, f);
            match solve_model(&m) {
                Ok(r) => println!("Re {re:5.0}  mesh ×{f:<4}  inlet {up:>4}S  {} el  x1/S {:.4}", r.mesh.tris.len(), flow_verify::cases::reattachment(&r)),
                Err(e) => println!("Re {re} ×{f}: {e}"),
            }
        }
    }
}
