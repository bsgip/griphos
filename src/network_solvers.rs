//! Generic network solver code.

use std::any::Any;
use std::fmt;
use std::ops::Deref;

use bitflags::bitflags;
use log::{debug, info};
use sprs::{CsMat, CsMatBase, CsVecBase};

use crate::foundation::PrettyPrint;
use crate::network::{Network};
use crate::network_model::{NetworkModel, NetworkModels};

bitflags! {
    /// Flags for generic network solver status.
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
    /// Combines two flag sets into a joint flags.
    pub fn combine(self, other: Self) -> Self {
        let self_ands = self.intersection(SOLVER_STATUS_ANDS);
        let other_ands = other.intersection(SOLVER_STATUS_ANDS);
        let self_ors = self.intersection(SOLVER_STATUS_ORS);
        let other_ors = other.intersection(SOLVER_STATUS_ORS);
        self_ands.intersection(other_ands).union(self_ors.union(other_ors))
    }
}

/// The solver status for a single iteration or the result of all iterations.
#[derive(Default, Debug, Clone)]
pub struct SolverStatusItem {
    pub flags: SolverStatusFlags,
    pub initial_residual: f64,
    pub final_residual: f64,
    pub total_time_s: f64,
    pub linear_solver_time_s: f64,
}

/// The solver status, including all iterations and the end result.
#[derive(Default, Debug, Clone)]
pub struct SolverStatus {
    pub status: SolverStatusItem,
    pub iteration_statuses: Vec<SolverStatusItem>,
}

impl SolverStatus {
    /// Creates the final overall solver status based on the combined iterations.
    pub fn finalize(mut self, maybe_combine_final_flags: Option<SolverStatusFlags>) -> Self {
        self.status.flags = self.iteration_statuses
            .iter()
            .map(|x| x.flags)
            .fold(SolverStatusFlags::SUCCESS, SolverStatusFlags::combine);

        if !self.iteration_statuses.is_empty() {
            self.status.linear_solver_time_s = self.iteration_statuses.iter().map(|x| x.linear_solver_time_s).sum();
            self.status.initial_residual = self.iteration_statuses.first().unwrap().initial_residual;
            self.status.final_residual = self.iteration_statuses.last().unwrap().final_residual;
        } else {
            self.status.linear_solver_time_s = 0.0;
            self.status.initial_residual = 0.0;
            self.status.final_residual = 0.0;
        }

        if let Some(combine_final_flags) = maybe_combine_final_flags {
            self.status.flags = self.status.flags.combine(combine_final_flags);
        }

        self
    }
}

/// Utility function that logs a CsMat sparse matrix, along with row and column names.
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

/// Utility function that logs a CsMat sparse matrix to debug, along with row and column names.
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

/// Utility function that logs a CsMat sparse matrix to info, along with row and column names.
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

/// Utility function that logs a CsVec sparse vector, along with index names.
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

/// Utility function that logs a CsVec sparse vector to debug, along with index names.
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

/// Utility function that logs a CsVec sparse vector to info, along with index names.
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

/// A [`NetworkSolver`] solver solves for the currents and voltages in the network, incluing solving and modifying the
/// network based on the solution.
pub trait NetworkSolver: Any {
    /// Solves power flow (or whatever), using the current values (powers, admittances, setpoints etc.) from `netw`.
    ///
    /// `netw` must have the same structure (see [`Network::structure_hash`]) as the network the solver was created
    /// from; [`solve_network`] ensures this.
    fn solve(&mut self, netw: &Network) -> SolverStatus;
    /// Propagates the solution to the network after solving.
    fn propagate_to_network(&self, netw: &mut Network);
}

impl dyn NetworkSolver {
    pub fn is<T: NetworkSolver>(&self) -> bool {
        (self as &dyn Any).is::<T>()
    }

    pub fn downcast_ref<T: NetworkSolver>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }

    pub fn downcast_mut<T: NetworkSolver>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut::<T>()
    }
}

/// A funtion used to autoregister solvers that will be chosen and constructed using a key.
pub type SolverConstructor = fn(NetworkModel) -> Box<dyn NetworkSolver>;

/// Struct that allows solvers to autoregister themselves.
pub struct SolverPlugin {
    pub key: &'static str,
    pub constructor: SolverConstructor,
}

// Creates the means to autoregister solvers.
inventory::collect!(SolverPlugin);

/// Lists all the autoregistered solvers.
pub fn list_solvers() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = inventory::iter::<SolverPlugin>.into_iter().map(|e| e.key).collect();
    keys.sort_unstable();
    keys
}

/// Error type for unknown solver error.
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

/// Constructs the correct solver that matches the supplied key.
pub fn solver_constructor(key: &str) -> Result<SolverConstructor, UnknownSolver> {
    inventory::iter::<SolverPlugin>
        .into_iter()
        .find(|e| e.key == key)
        .map(|e| e.constructor)
        .ok_or_else(|| UnknownSolver(key.to_owned()))
}

/// The solvers for a network, one per island, as returned by [`solve_network`] for reuse in later calls.
///
/// Records the [`Network::structure_hash`] of the network the solvers were created from, so that [`solve_network`] can
/// tell whether they can be reused.
pub struct NetworkSolvers {
    structure_hash: u64,
    solvers: Vec<Box<dyn NetworkSolver>>,
}

impl NetworkSolvers {
    /// Creates new solvers for `netw`, one per island, using the solver with the given key.
    pub fn new(netw: &Network, solver_name: &str) -> Result<NetworkSolvers, UnknownSolver> {
        let ctor = solver_constructor(solver_name)?;
        let structure_hash = netw.structure_hash();
        let solvers = NetworkModels::new(netw).islands.into_iter().map(ctor).collect();
        Ok(NetworkSolvers { structure_hash, solvers })
    }

    /// The [`Network::structure_hash`] of the network the solvers were created from.
    pub fn structure_hash(&self) -> u64 {
        self.structure_hash
    }

    /// The solvers, one per island.
    pub fn solvers(&self) -> &[Box<dyn NetworkSolver>] {
        &self.solvers
    }

    /// Mutable access to the solvers, one per island.
    pub fn solvers_mut(&mut self) -> &mut [Box<dyn NetworkSolver>] {
        &mut self.solvers
    }
}

/// Return value for solve_network.
///
/// [`solve_network`] splits the problem into network into islands, and records results for each island. It also returns
/// the solvers that were used by the [`solve_network`] call, for possible later reuse.
pub struct SolveNetworkResult {
    /// The solve statuses, per island.
    pub statuses: Vec<SolverStatus>,
    /// The solvers that were used by the call.
    ///
    /// These are either the solvers passed into the call, or new ones if none were passed in or the network's
    /// structure had changed.
    pub solvers: NetworkSolvers,
}

/// Solves the network using the given solver (specified as a key).
///
/// Accepts as the `solvers` argument optional [`NetworkSolvers`] from a previous call, which, if supplied, will be used
/// for the new solve call, meaning that we don't need to do the work of creating solvers from scratch. This makes it
/// much more efficient to perform multiple solve calls, e.g. solve the network every five minutes during a day long
/// horizon.
///
/// Between calls, values in the network such as load powers and source setpoints may change, and reused solvers will
/// pick up the changes. Supplied solvers are only reused if the network's [`Network::structure_hash`] matches the one
/// they were created from. Otherwise, the structure has changed (e.g. an element was added or taken out of service),
/// and new solvers are created instead.
///
/// # Examples
///
/// ```
/// # use griphos::{parse_ejson_file, ejson_phase_map_abcng, solve_network, SolveNetworkResult};
/// let mut netw = parse_ejson_file("test_data/netw-i1l1s1.json", &ejson_phase_map_abcng()).unwrap();
/// let SolveNetworkResult { statuses, solvers } = solve_network(&mut netw, "nodal_rect", None).unwrap();
/// // Can modify the network here...
/// let SolveNetworkResult { statuses, solvers } = solve_network(&mut netw, "nodal_rect", Some(solvers)).unwrap(); // Reuse solvers.
/// ```
pub fn solve_network(
    netw: &mut Network,
    solver_name: &str,
    solvers: Option<NetworkSolvers>,
) -> Result<SolveNetworkResult, UnknownSolver> {
    let mut solvers = match solvers {
        Some(solvers) if solvers.structure_hash == netw.structure_hash() => solvers,
        solvers => {
            if solvers.is_some() {
                info!("Network structure changed: creating new solvers.");
            }
            NetworkSolvers::new(netw, solver_name)?
        }
    };
    let mut statuses: Vec<SolverStatus> = vec![];
    for solver in solvers.solvers.iter_mut() {
        let results = solver.solve(netw);
        if results.status.flags.contains(SolverStatusFlags::SUCCESS) {
            solver.propagate_to_network(netw);
        }
        statuses.push(results);
    }
    Ok(SolveNetworkResult { statuses, solvers })
}
