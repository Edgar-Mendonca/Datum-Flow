//! Datum Flow's engine, part three: the flow solver. Steady, laminar, incompressible flow with
//! heat and buoyancy on Taylor–Hood elements (quadratic velocity and temperature, linear
//! pressure), one coupled system solved by Picard then Newton with a sparse direct LU.

pub mod advice;
pub mod assemble;
pub mod fe;
pub mod setup;
pub mod solve;
pub mod sparse;

pub use assemble::Dofs;
pub use setup::Setup;
pub use solve::{solve, Row, Solved, State, Stats, Step};
pub use sparse::{Csc, LinearSolver, SparseLu};

#[cfg(test)]
mod tests;
