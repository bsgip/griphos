use std::fmt;
use std::ops::Deref;

use bitflags::bitflags;
use log::{debug, info};
use sprs::{CsMat, CsMatBase, CsVecBase};

use crate::foundation::PrettyPrint;
use crate::network::{Network, NetworkModel, NetworkModels};

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq)]
    pub struct SolverStatusFlags : u8 {
        const SUCCESS = 0b0000_0001;
        const FAILURE = 0b0000_0010;
        const LINEAR_SOLVER_FAILURE = 0b0000_0100;
        const TIMEOUT = 0b0000_1000;
        const MAX_ITER_EXCEEDED = 0b0001_0000;
    }
}

const SOLVER_STATUS_ORS: SolverStatusFlags = SolverStatusFlags::from_bits(0b0001_1110).unwrap();
const SOLVER_STATUS_ANDS: SolverStatusFlags = SolverStatusFlags::from_bits(0b0000_0001).unwrap();

impl SolverStatusFlags {
    pub fn combine(self, other: Self) -> Self {
        let self_ands = self.intersection(SOLVER_STATUS_ANDS);
        let other_ands = other.intersection(SOLVER_STATUS_ANDS);
        let self_ors = self.intersection(SOLVER_STATUS_ORS);
        let other_ors = other.intersection(SOLVER_STATUS_ORS);
        self_ands.intersection(other_ands).union(self_ors.union(other_ors))
    }
}

#[derive(Default, Debug, Clone)]
pub struct SolverStatusItem {
    pub flags: SolverStatusFlags,
    pub initial_residual: f64,
    pub final_residual: f64,
    pub total_time_s: f64,
    pub linear_solver_time: f64,
}

#[derive(Default, Debug, Clone)]
pub struct SolverStatus {
    pub status: SolverStatusItem,
    pub iteration_statuses: Vec<SolverStatusItem>,
}

impl SolverStatus {
    pub fn finalize(mut self, maybe_combine_final_flags: Option<SolverStatusFlags>) -> Self {
        self.status.flags = self.iteration_statuses
            .iter()
            .map(|x| x.flags)
            .fold(SolverStatusFlags::SUCCESS, SolverStatusFlags::combine);
        if let Some(combine_final_flags) = maybe_combine_final_flags {
            self.status.flags = self.status.flags.combine(combine_final_flags);
        }
        self
    }
}

#[allow(unused)]
pub(crate) fn log_csmat<T, I, IptrStorage, IndStorage, DataStorage>(
    mat: &CsMatBase<T, I, IptrStorage, IndStorage, DataStorage>,
    mat_name: Option<&str>,
    row_names: Option<&[String]>,
    col_names: Option<&[String]>,
    log: impl Fn(std::fmt::Arguments),
) where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    I: sprs::SpIndex,
    IptrStorage: std::ops::Deref<Target = [I]>,
    IndStorage: std::ops::Deref<Target = [I]>,
    DataStorage: std::ops::Deref<Target = [T]>,
{
    let mat_name = if let Some(mat_name) = mat_name { format!("{mat_name}:\n") } else { "\n".to_string() };
    log(format_args!("{mat_name}{}", PrettyPrint(mat, (row_names, col_names))));
}

#[allow(unused)]
pub(crate) fn debug_csmat<T, I, IptrStorage, IndStorage, DataStorage>(
    mat: &CsMatBase<T, I, IptrStorage, IndStorage, DataStorage>,
    mat_name: Option<&str>,
    row_names: Option<&[String]>,
    col_names: Option<&[String]>,
) where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    I: sprs::SpIndex,
    IptrStorage: std::ops::Deref<Target = [I]>,
    IndStorage: std::ops::Deref<Target = [I]>,
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
) where
    T: std::fmt::Display + std::fmt::Debug + Copy,
    I: sprs::SpIndex,
    IptrStorage: std::ops::Deref<Target = [I]>,
    IndStorage: std::ops::Deref<Target = [I]>,
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
    let mat = CsMat::new((1, vec.dim()), vec![0, vec.nnz()], vec.indices().to_vec(), vec.data().to_vec());
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
    fn solve(&mut self) -> SolverStatus;
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
    inventory::iter::<SolverPlugin>
        .into_iter()
        .find(|e| e.name == name)
        .map(|e| e.constructor)
        .ok_or_else(|| UnknownSolver(name.to_owned()))
}

pub struct SolveNetworkResult {
    pub statuses: Vec<SolverStatus>,
    pub solvers: Vec<Box<dyn NetworkSolver>>,
}

pub fn solve_network(
    netw: &mut Network,
    solver_name: &str,
    solvers: Option<Vec<Box<dyn NetworkSolver>>>,
) -> Result<SolveNetworkResult, UnknownSolver> {
    let mut solvers = match solvers {
        None => {
            let ctor = solver_constructor(solver_name)?;
            NetworkModels::new(netw).islands.into_iter().map(ctor).collect()
        }
        Some(solvers) => solvers,
    };
    let mut statuses: Vec<SolverStatus> = vec![];
    for solver in solvers.iter_mut() {
        let results = solver.solve();
        if results.status.flags.contains(SolverStatusFlags::SUCCESS) {
            solver.propagate_to_network(netw);
        }
        statuses.push(results);
    }
    Ok(SolveNetworkResult { statuses, solvers })
}
