//! Datum Flow's engine, part four: what is read off a solution. Fields at the nodes (speed,
//! pressure, temperature, vorticity, stream function), values at points and along lines, values
//! integrated over boundaries (flow rate, pressure drop, forces, heat), and the VTU and CSV files.

#![allow(clippy::needless_range_loop)]

pub mod export;
pub mod fields;
pub mod locate;
pub mod values;

#[cfg(test)]
mod tests;

pub use fields::{nodal, stream_function, FIELDS};
pub use locate::Locator;
pub use values::{monitor, Sample, Values};
