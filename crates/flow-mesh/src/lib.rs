//! Datum Flow's engine, part two: the mesher (six-node triangles from a constrained Delaunay
//! triangulation with local refinement), its quality report, and the checks made before solving.

pub mod check;
pub mod mesh;
pub mod size;

pub use check::{check, Issue};
pub use mesh::{mesh, rebuild, BEdge, Mesh, Quality};
