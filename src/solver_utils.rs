use std::ops::Deref;

use log::{debug, info};
use sprs::{CsMat, CsMatBase, CsVecBase};

use crate::foundation::PrettyPrint;

pub enum SolveStatus {
    LinearSolverFailed,
    NotConverged {iteration_errs: Vec<f64>},
    Converged {iteration_errs: Vec<f64>},
}

#[allow(unused)]
pub(crate) fn debug_print_csmat<T, I, IptrStorage, IndStorage, DataStorage>(
    mat: &CsMatBase<T, I, IptrStorage, IndStorage, DataStorage>,
    row_names: Option<&[String]>,
    col_names: Option<&[String]>)
where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    I: sprs::SpIndex,
    IptrStorage: std::ops::Deref<Target = [I]>,
    IndStorage:  std::ops::Deref<Target = [I]>,
    DataStorage: std::ops::Deref<Target = [T]>,
{
    debug!("\n{}", PrettyPrint(mat, (row_names, col_names)));
}

#[allow(unused)]
pub(crate) fn info_print_csmat<T, IptrStorage, IndStorage, DataStorage>(
    mat: &CsMatBase<T, usize, IptrStorage, IndStorage, DataStorage>,
    row_names: Option<&[String]>, col_names: Option<&[String]>)
where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    IptrStorage: std::ops::Deref<Target = [usize]>,
    IndStorage:  std::ops::Deref<Target = [usize]>,
    DataStorage: std::ops::Deref<Target = [T]>,
{
    info!("\n{}", PrettyPrint(mat, (row_names, col_names)));
}

#[allow(unused)]
pub(crate) fn debug_print_csvec<T, IStorage, DStorage>(
    vec: &CsVecBase<IStorage, DStorage, T>,
    row_names: Option<&[String]>,
) where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    IStorage: Deref<Target = [usize]>,
    DStorage: Deref<Target = [T]>,
{
    let mat = CsMat::new(
        (1, vec.dim()),
        vec![0, vec.nnz()],
        vec.indices().to_vec(),
        vec.data().to_vec(),
    );
    debug_print_csmat(&mat, row_names, None)
}

#[allow(unused)]
pub(crate) fn info_print_csvec<T, IStorage, DStorage>(
    vec: &CsVecBase<IStorage, DStorage, T>,
    row_names: Option<&[String]>,
) where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    IStorage: Deref<Target = [usize]>,
    DStorage: Deref<Target = [T]>,
{
    let mat = CsMat::new(
        (1, vec.dim()),
        vec![0, vec.nnz()],
        vec.indices().to_vec(),
        vec.data().to_vec(),
    );
    info_print_csmat(&mat, row_names, None)
}
