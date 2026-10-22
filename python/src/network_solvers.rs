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

/// A [`NetworkSolver`] solver solves for the currents and voltages in the network, incluing solving and modifying the
/// network based on the solution.
#[pyclass(unsendable)]
#[gen_stub_pyclass]
pub struct NetworkSolver(pub Option<Box<dyn gr::NetworkSolver>>);

impl<'a, 'py> FromPyObject<'a, 'py> for NetworkSolver {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let cell: Borrowed<'a, 'py, NetworkSolver> = obj.cast()?;
        let mut guard = cell.borrow_mut();
        let inner = guard
            .0
            .take()
            .ok_or_else(|| PyValueError::new_err("this solver has already been consumed"))?;
        Ok(NetworkSolver(Some(inner)))
    }
}

/// Return value for solve_network.
///
/// [`solve_network`] splits the problem into network into islands, and records results for each island. It also returns
/// the solvers that were created by the [`solve_network`] call, for possible later reuse.
#[pyclass]
#[gen_stub_pyclass]
pub struct SolveNetworkResult {
    #[pyo3(get)]
    pub statuses: Vec<SolverStatus>,
    #[pyo3(get)]
    pub solvers: Vec<Py<NetworkSolver>>,
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

    let solvers = result
        .solvers
        .into_iter()
        .map(|s| Py::new(py, NetworkSolver(Some(s))))
        .collect::<PyResult<Vec<_>>>()?;

    Ok(SolveNetworkResult { statuses, solvers })
}

fn unknown_solver_to_pyerr(err: gr::UnknownSolver) -> PyErr {
    PyValueError::new_err(err.to_string()) // assumes UnknownSolver: Display
}

/// Solves the network using the given solver (specified as a key).
///
/// Accepts as the `solvers` argument an optional list of [`NetworkSolver`]s, which, if supplied, will be used for the
/// new solve call, meaning that we don't need to do the work of creating solvers from scratch. This makes it much more
/// efficient to perform multiple solve calls, e.g. solve the network every five minutes during a day long horizon.
///
/// Args:
///     netw: The network to solve for.
///     solver_name: The solver selection key, such as "nodal_rect".
///     solvers: Optional list of `NetworkSolver`s; if supplied will use these instead of creating new ones.
#[pyfunction]
#[pyo3(signature = (netw, solver_name, solvers=None))]
#[gen_stub_pyfunction]
pub fn solve_network(
    py: Python,
    netw: &mut Network,
    solver_name: &str,
    solvers: Option<Vec<NetworkSolver>>,
) -> PyResult<SolveNetworkResult> {
    let solvers_rust: Option<Vec<Box<dyn gr::NetworkSolver>>> = solvers.map(|v| {
        v.into_iter()
            .map(|s| s.0.expect("FromPyObject guarantees Some here"))
            .collect()
    });

    let result = gr::solve_network(&mut netw.inner.borrow_mut(), solver_name, solvers_rust)
        .map_err(unknown_solver_to_pyerr)?;
    solve_network_result_to_py(py, result)
}
