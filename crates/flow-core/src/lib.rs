//! Datum Flow's engine, part one: the model and its edits, the fluids, display units and the
//! `.dflow` file. Pure Rust, no I/O; the mesher, the solver and the results build on it.

pub mod doc;
pub mod dxf;
pub mod examples;
pub mod file;
pub mod fluid;
pub mod model;
pub mod numbers;
pub mod units;
pub mod wire;

#[cfg(test)]
mod hardening;

pub use doc::{Doc, Op};
pub use model::*;
