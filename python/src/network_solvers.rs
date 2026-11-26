use ::griphos as gr;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::Borrowed;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction};

use crate::network::Network;

/// The solver status for a single iteration or the result of all iterations.
#[pyclass(module = "griphos", get_all, eq, frozen, skip_from_py_object)]
#[gen_stub_pyclass]
#[derive(Clone, PartialEq)]
pub struct SolverStatusItem {
    pub flags: u8,
    pub initial_residual: f64,
    pub final_residual: f64,
    pub total_time_s: f64,
    pub linear_solver_time_s: f64,
}

impl From<&gr::SolverStatusItem> for SolverStatusItem {
    fn from(v: &gr::SolverStatusItem) -> Self {
        SolverStatusItem {
            flags: v.flags.bits(),
            initial_residual: v.initial_residual,
            final_residual: v.final_residual,
            total_time_s: v.total_time_s,
            linear_solver_time_s: v.linear_solver_time_s,
        }
    }
}

/// The iteration status and final status for a single solver run.
#[pyclass(module = "griphos", get_all, eq, frozen, skip_from_py_object)]
#[gen_stub_pyclass]
#[derive(Clone, PartialEq)]
pub struct SolverStatus {
    pub status: SolverStatusItem,
    pub iteration_statuses: Vec<SolverStatusItem>,
}

impl From<&gr::SolverStatus> for SolverStatus {
    fn from(v: &gr::SolverStatus) -> Self {
        SolverStatus {
            status: (&v.status).into(),
            iteration_statuses: v.iteration_statuses.iter().map(Into::into).collect(),
        }
    }
}

/// The solvers for a network, one per island, as returned by `solve_network` for reuse in later calls.
///
/// Opaque: pass it back into `solve_network` to reuse the solvers. Each `NetworkSolvers` can only be passed in once,
/// after which the solvers that were actually used are available from the new `SolveNetworkResult`.
#[pyclass(unsendable)]
#[gen_stub_pyclass]
pub struct NetworkSolvers(pub Option<gr::NetworkSolvers>);

impl<'a, 'py> FromPyObject<'a, 'py> for NetworkSolvers {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let cell: Borrowed<'a, 'py, NetworkSolvers> = obj.cast()?;
        let mut guard = cell.borrow_mut();
        let inner = guard
            .0
            .take()
            .ok_or_else(|| PyValueError::new_err("these solvers have already been consumed"))?;
        Ok(NetworkSolvers(Some(inner)))
    }
}

/// Return value for solve_network.
///
/// [`solve_network`] splits the problem into network into islands, and records results for each island. It also returns
/// the solvers that were used by the [`solve_network`] call, for possible later reuse.
#[pyclass]
#[gen_stub_pyclass]
pub struct SolveNetworkResult {
    #[pyo3(get)]
    pub statuses: Vec<SolverStatus>,
    #[pyo3(get)]
    pub solvers: Py<NetworkSolvers>,
}

fn solve_network_result_to_py(
    py: Python,
    result: gr::SolveNetworkResult,
) -> PyResult<SolveNetworkResult> {
    let statuses = result
        .statuses
        .into_iter()
        .map(|s| SolverStatus::from(&s))
        .collect::<Vec<SolverStatus>>();

    let solvers = Py::new(py, NetworkSolvers(Some(result.solvers)))?;

    Ok(SolveNetworkResult { statuses, solvers })
}

fn unknown_solver_to_pyerr(err: gr::UnknownSolver) -> PyErr {
    PyValueError::new_err(err.to_string()) // assumes UnknownSolver: Display
}

/// Solves the network using the given solver (specified as a key).
///
/// Accepts as the `solvers` argument optional `NetworkSolvers` from a previous call, which, if supplied, will be used
/// for the new solve call, meaning that we don't need to do the work of creating solvers from scratch. This makes it
/// much more efficient to perform multiple solve calls, e.g. solve the network every five minutes during a day long
/// horizon.
///
/// Supplied solvers are only reused if the network's structure is unchanged since they were created. Otherwise (e.g. an
/// element was added or taken out of service), new solvers are created instead.
///
/// Args:
///     netw: The network to solve for.
///     solver_name: The solver selection key, such as "nodal_rect".
///     solvers: Optional `NetworkSolvers` from a previous call; if supplied will reuse these if possible.
#[pyfunction]
#[pyo3(signature = (netw, solver_name, solvers=None))]
#[gen_stub_pyfunction]
pub fn solve_network(
    py: Python,
    netw: &mut Network,
    solver_name: &str,
    solvers: Option<NetworkSolvers>,
) -> PyResult<SolveNetworkResult> {
    let solvers_rust = solvers.map(|s| s.0.expect("FromPyObject guarantees Some here"));

    let result = gr::solve_network(&mut netw.inner.borrow_mut(), solver_name, solvers_rust)
        .map_err(unknown_solver_to_pyerr)?;
    solve_network_result_to_py(py, result)
}
