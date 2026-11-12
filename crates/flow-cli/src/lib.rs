//! Datum Flow's engine, as one session: the mesh and the results of the open model, and
//! everything the window or the command line can ask of them. The desktop shell, the command-line
//! tool and the window's tests all go through `Engine`, so they cannot disagree.

pub mod engine;
pub use engine::{Engine, Job, Progress};
