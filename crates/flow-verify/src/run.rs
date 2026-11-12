//! Solving a model from start to finish, for the cases and the command line.

use flow_core::model::Model;
use flow_mesh::Mesh;
use flow_post::{Locator, Values};
use flow_solve::{solve, Setup, Solved, SparseLu};

/// A model, its mesh and its solution.
pub struct Run {
    pub m: Model,
    pub mesh: Mesh,
    pub solved: Solved,
}

impl Run {
    pub fn values(&self) -> Values<'_> {
        Values::new(&self.m, &self.mesh, &self.solved.state, &self.solved.residual, self.solved.dofs)
    }
    pub fn locator(&self) -> Locator {
        Locator::new(&self.mesh)
    }
    /// A representative element size: √(area / elements).
    pub fn h(&self) -> f64 {
        (self.mesh.area() / self.mesh.tris.len().max(1) as f64).sqrt()
    }
}

/// Mesh and solve a model; an error if it cannot be meshed or does not converge.
pub fn solve_model(m: &Model) -> Result<Run, String> {
    let mesh = flow_mesh::mesh(m)?;
    let setup = Setup::from_model(m, &mesh)?;
    let solved = solve(&mesh, &setup, &m.solver, None, &mut SparseLu::default(), &mut |_| true)?;
    if !solved.converged {
        return Err(solved.message);
    }
    Ok(Run { m: m.clone(), mesh, solved })
}

/// The same model with every mesh size divided by `factor`.
pub fn refined(m: &Model, factor: f64) -> Model {
    let mut m = m.clone();
    m.mesh.size = flow_mesh::mesh::global_size(&m) / factor;
    for r in &mut m.mesh.refine {
        r.size /= factor;
    }
    m
}
