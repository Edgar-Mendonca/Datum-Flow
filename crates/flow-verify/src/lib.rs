//! Datum Flow's engine, part five: verification. Every case solves a problem whose answer is
//! known and compares; each comparison is one row of the report shown in Help › Verification and
//! one assertion of the test suite. A failing row blocks a release.

#![allow(clippy::needless_range_loop)]

pub mod cases;
pub mod gci;
pub mod report;
pub mod run;

#[cfg(test)]
mod tests;

pub use run::{solve_model, Run};
