use std::fmt;
use std::ops::Deref;

use log::{debug, info};
use sprs::{CsMat, CsMatBase, CsVecBase};

use crate::foundation::PrettyPrint;
use crate::network::{Network, NetworkModel, NetworkModels};

pub enum SolveStatus {
    LinearSolverFailed,
    NotConverged {iteration_errs: Vec<f64>},
    Converged {iteration_errs: Vec<f64>},
}

#[allow(unused)]
pub(crate) fn log_csmat<T, I, IptrStorage, IndStorage, DataStorage>(
    mat: &CsMatBase<T, I, IptrStorage, IndStorage, DataStorage>,
    mat_name: Option<&str>,
    row_names: Option<&[String]>,
    col_names: Option<&[String]>,
    log: impl Fn(std::fmt::Arguments),
)
where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    I: sprs::SpIndex,
    IptrStorage: std::ops::Deref<Target = [I]>,
    IndStorage:  std::ops::Deref<Target = [I]>,
    DataStorage: std::ops::Deref<Target = [T]>,
{
    let mat_name = if let Some(mat_name) = mat_name {
        format!("{mat_name}:\n")
    } else {
        "\n".to_string()
    };
    log(format_args!("{mat_name}{}", PrettyPrint(mat, (row_names, col_names))));
}

#[allow(unused)]
pub(crate) fn debug_csmat<T, I, IptrStorage, IndStorage, DataStorage>(
    mat: &CsMatBase<T, I, IptrStorage, IndStorage, DataStorage>,
    mat_name: Option<&str>,
    row_names: Option<&[String]>,
    col_names: Option<&[String]>,
)
where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    I: sprs::SpIndex,
    IptrStorage: std::ops::Deref<Target = [I]>,
    IndStorage:  std::ops::Deref<Target = [I]>,
    DataStorage: std::ops::Deref<Target = [T]>,
{
        log_csmat(mat, mat_name, row_names, col_names, |args| debug!("{}", args));
}

#[allow(unused)]
pub(crate) fn info_csmat<T, I, IptrStorage, IndStorage, DataStorage>(
    mat: &CsMatBase<T, I, IptrStorage, IndStorage, DataStorage>,
    mat_name: Option<&str>,
    row_names: Option<&[String]>,
    col_names: Option<&[String]>,
)
where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    I: sprs::SpIndex,
    IptrStorage: std::ops::Deref<Target = [I]>,
    IndStorage:  std::ops::Deref<Target = [I]>,
    DataStorage: std::ops::Deref<Target = [T]>,
{
        log_csmat(mat, mat_name, row_names, col_names, |args| info!("{}", args));
}

#[allow(unused)]
pub(crate) fn log_csvec<T, IStorage, DStorage>(
    vec: &CsVecBase<IStorage, DStorage, T>,
    vec_name: Option<&str>,
    row_names: Option<&[String]>,
    log: impl Fn(std::fmt::Arguments),
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
    log_csmat(&mat, vec_name, row_names, None, log)
}

#[allow(unused)]
pub(crate) fn debug_csvec<T, IStorage, DStorage>(
    vec: &CsVecBase<IStorage, DStorage, T>,
    vec_name: Option<&str>,
    row_names: Option<&[String]>,
) where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    IStorage: Deref<Target = [usize]>,
    DStorage: Deref<Target = [T]>,
{
        log_csvec(vec, vec_name, row_names, |args| debug!("{}", args));
}

#[allow(unused)]
pub(crate) fn info_csvec<T, IStorage, DStorage>(
    vec: &CsVecBase<IStorage, DStorage, T>,
    vec_name: Option<&str>,
    row_names: Option<&[String]>,
) where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    IStorage: Deref<Target = [usize]>,
    DStorage: Deref<Target = [T]>,
{
        log_csvec(vec, vec_name, row_names, |args| info!("{}", args));
}

pub trait NetworkSolver {
    fn solve(&mut self) -> SolveStatus;
    fn propagate_to_network(&self, netw: &mut Network);
}

pub type SolverConstructor = fn(NetworkModel) -> Box<dyn NetworkSolver>;

pub struct SolverPlugin {
    pub name: &'static str,
    pub constructor: SolverConstructor,
}

inventory::collect!(SolverPlugin);

pub fn list_solvers() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = inventory::iter::<SolverPlugin>.into_iter().map(|e| e.name).collect();
    names.sort_unstable();
    names
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSolver(pub String);

impl fmt::Display for UnknownSolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown solver: {}", self.0)?;
        let available = list_solvers();
        if available.is_empty() {
            write!(f, " (no solvers registered)")
        } else {
            write!(f, " (available: {})", available.join(", "))
        }
    }
}

impl std::error::Error for UnknownSolver {}

pub fn solver_constructor(name: &str) -> Result<SolverConstructor, UnknownSolver> {
    inventory::iter::<SolverPlugin>.into_iter().find(|e| e.name == name).map(|e| e.constructor)
        .ok_or_else(|| UnknownSolver(name.to_owned()))
}

pub struct SolveNetworkResult {
    pub statuses: Vec<SolveStatus>,
    pub solvers: Vec<Box<dyn NetworkSolver>>,
}

pub fn solve_network(
    netw: &mut Network, solver_name: &str, solvers: Option<Vec<Box<dyn NetworkSolver>>>,
) -> Result<SolveNetworkResult, UnknownSolver> {
    let mut solvers = match solvers {
        None => {
            let ctor = solver_constructor(solver_name)?;
            NetworkModels::new(netw).islands.into_iter().map(ctor).collect()
        }
        Some(solvers) => solvers,
    };
    let mut statuses: Vec<SolveStatus> = vec![];
    for solver in solvers.iter_mut() {
        let status = solver.solve();
        if matches!(status, SolveStatus::Converged{..}) {
            solver.propagate_to_network(netw);
        }
        statuses.push(status);
    }
    Ok(SolveNetworkResult {statuses, solvers})
}
