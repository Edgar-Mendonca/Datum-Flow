//! What the window is sent after every action: the model and everything worked out from it that
//! the window shows but must not calculate itself.

use crate::doc::Doc;
use crate::model::*;
use crate::numbers::{numbers, Numbers};
use crate::units::{table, Unit};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct EdgeInfo {
    pub shape: u32,
    pub edge: u32,
    pub name: String,
    /// The boundary it belongs to (0 = none yet).
    pub boundary: u32,
    pub length: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Out {
    pub m: Model,
    pub can_undo: bool,
    pub can_redo: bool,
    pub revision: u64,
    /// Display units of the model's unit system, by kind of quantity.
    pub units: BTreeMap<&'static str, Unit>,
    pub numbers: Numbers,
    /// Why the flow is probably turbulent, if it is.
    pub turbulent: Option<String>,
    pub edges: Vec<EdgeInfo>,
    /// The drawing's problem, if it cannot be meshed as it is.
    pub geometry_problem: Option<String>,
    /// [x0, y0, x1, y1] of the flow region.
    pub bounds: Option<[f64; 4]>,
}

pub fn out(d: &Doc) -> Out {
    let m = &d.m;
    let n = numbers(m);
    Out {
        can_undo: d.can_undo(),
        can_redo: d.can_redo(),
        revision: d.revision,
        units: table(m.units),
        turbulent: n.turbulent(),
        numbers: n,
        edges: m.all_edges().into_iter().map(|e| EdgeInfo {
            shape: e.shape, edge: e.edge, name: m.edge_name(e), boundary: m.boundary_of(e).map(|b| b.id).unwrap_or(0),
            length: m.shape(e.shape).map(|s| crate::numbers::edge_length(&s.geom, e.edge as usize)).unwrap_or(0.0),
        }).collect(),
        geometry_problem: m.check_geometry().err(),
        bounds: m.bounds(),
        m: m.clone(),
    }
}
