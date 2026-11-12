//! The linear algebra the nonlinear solver leans on: a compressed sparse column matrix whose
//! pattern is built once from the mesh, and the `LinearSolver` interface with its first
//! implementation, a sparse direct LU from faer (MIT). Iterative solvers will plug in here.

use faer::linalg::solvers::Solve;
use faer::sparse::linalg::solvers::{Lu, SymbolicLu};
use faer::sparse::{SparseColMatRef, SymbolicSparseColMatRef};
use faer::Mat;

/// A square sparse matrix, compressed by columns, rows sorted within each column.
#[derive(Clone, Debug, Default)]
pub struct Csc {
    pub n: usize,
    pub col_ptr: Vec<usize>,
    pub row_idx: Vec<usize>,
    pub val: Vec<f64>,
}

impl Csc {
    /// A zero matrix with the given columns of sorted row indices.
    pub fn from_columns(cols: Vec<Vec<usize>>) -> Csc {
        let n = cols.len();
        let mut col_ptr = Vec::with_capacity(n + 1);
        let mut row_idx = Vec::with_capacity(cols.iter().map(|c| c.len()).sum());
        col_ptr.push(0);
        for c in cols {
            row_idx.extend(c);
            col_ptr.push(row_idx.len());
        }
        let nnz = row_idx.len();
        Csc { n, col_ptr, row_idx, val: vec![0.0; nnz] }
    }
    pub fn clear(&mut self) {
        self.val.iter_mut().for_each(|v| *v = 0.0);
    }
    /// Add to entry (row, col); the entry must be in the pattern.
    #[inline]
    pub fn add(&mut self, row: usize, col: usize, v: f64) {
        let (a, b) = (self.col_ptr[col], self.col_ptr[col + 1]);
        match self.row_idx[a..b].binary_search(&row) {
            Ok(k) => self.val[a + k] += v,
            Err(_) => debug_assert!(v == 0.0, "entry ({row}, {col}) is not in the pattern"),
        }
    }
    /// A x.
    pub fn mul(&self, x: &[f64]) -> Vec<f64> {
        let mut y = vec![0.0; self.n];
        for c in 0..self.n {
            let xc = x[c];
            if xc != 0.0 {
                for k in self.col_ptr[c]..self.col_ptr[c + 1] {
                    y[self.row_idx[k]] += self.val[k] * xc;
                }
            }
        }
        y
    }
    pub fn nnz(&self) -> usize {
        self.row_idx.len()
    }
}

/// Solves A x = b. The nonlinear solver knows nothing else about how.
pub trait LinearSolver: Send {
    fn solve(&mut self, a: &Csc, b: &[f64]) -> Result<Vec<f64>, String>;
    fn name(&self) -> &'static str;
}

/// Sparse direct LU with partial pivoting (faer). The symbolic analysis (ordering and fill) is
/// done once per pattern and reused for every later matrix.
#[derive(Default)]
pub struct SparseLu {
    symbolic: Option<(usize, usize, SymbolicLu<usize>)>,
}

impl LinearSolver for SparseLu {
    fn name(&self) -> &'static str {
        "sparse direct LU"
    }
    fn solve(&mut self, a: &Csc, b: &[f64]) -> Result<Vec<f64>, String> {
        if a.n == 0 {
            return Ok(vec![]);
        }
        if !a.val.iter().all(|v| v.is_finite()) || !b.iter().all(|v| v.is_finite()) {
            return Err("The equations contain values that are not numbers; the last step went too far.".into());
        }
        let sym = SymbolicSparseColMatRef::new_checked(a.n, a.n, &a.col_ptr, None, &a.row_idx);
        let mat = SparseColMatRef::new(sym, &a.val[..]);
        let fresh = !matches!(&self.symbolic, Some((n, nnz, _)) if *n == a.n && *nnz == a.nnz());
        if fresh {
            let s = SymbolicLu::try_new(sym).map_err(|_| "Not enough memory to solve a model this large. Use a coarser mesh.".to_string())?;
            self.symbolic = Some((a.n, a.nnz(), s));
        }
        let s = self.symbolic.as_ref().expect("set above").2.clone();
        let lu = Lu::try_new_with_symbolic(s, mat).map_err(|_| singular())?;
        let mut rhs = Mat::<f64>::zeros(a.n, 1);
        for (i, v) in b.iter().enumerate() {
            rhs[(i, 0)] = *v;
        }
        let x = lu.solve(&rhs);
        let mut out: Vec<f64> = (0..a.n).map(|i| x[(i, 0)]).collect();
        if !out.iter().all(|v| v.is_finite()) {
            return Err(singular());
        }
        // One step of refinement with the same factors wins back digits lost to pivoting.
        let ax = a.mul(&out);
        for i in 0..a.n {
            rhs[(i, 0)] = b[i] - ax[i];
        }
        let dx = lu.solve(&rhs);
        if (0..a.n).all(|i| dx[(i, 0)].is_finite()) {
            for (i, o) in out.iter_mut().enumerate() {
                *o += dx[(i, 0)];
            }
        }
        Ok(out)
    }
}

fn singular() -> String {
    "The equations have no single answer: check that every edge has a boundary and that the flow has somewhere to go.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lu_solves_a_saddle_point_system_and_reuses_its_analysis() {
        // [[2, 1, 1], [1, 3, −1], [1, −1, 0]]: a zero on the diagonal, as pressure rows have.
        let mut a = Csc::from_columns(vec![vec![0, 1, 2], vec![0, 1, 2], vec![0, 1]]);
        for (r, c, v) in [(0, 0, 2.0), (1, 0, 1.0), (2, 0, 1.0), (0, 1, 1.0), (1, 1, 3.0), (2, 1, -1.0), (0, 2, 1.0), (1, 2, -1.0)] {
            a.add(r, c, v);
        }
        let want = [1.0, -2.0, 0.5];
        let b = a.mul(&want);
        let mut lu = SparseLu::default();
        let x = lu.solve(&a, &b).unwrap();
        assert!(x.iter().zip(want).all(|(p, q)| (p - q).abs() < 1e-13), "{x:?}");
        // New values, same pattern.
        a.clear();
        for (r, c, v) in [(0, 0, 4.0), (1, 0, 1.0), (2, 0, 2.0), (0, 1, 1.0), (1, 1, 5.0), (2, 1, -1.0), (0, 2, 2.0), (1, 2, -1.0)] {
            a.add(r, c, v);
        }
        let b = a.mul(&want);
        let x = lu.solve(&a, &b).unwrap();
        assert!(x.iter().zip(want).all(|(p, q)| (p - q).abs() < 1e-13), "{x:?}");
        // Not-a-number input is refused in words.
        assert!(lu.solve(&a, &[f64::NAN, 0.0, 0.0]).is_err());
    }
}
