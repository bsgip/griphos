use ::griphos as gr;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pyclass_init::PyClassInitializer;
use pyo3::Borrowed;

use crate::network::Network;

#[pyclass(subclass)]
pub struct SolveStatus;

#[pyclass(extends=SolveStatus)]
pub struct LinearSolverFailed;

#[pymethods]
impl LinearSolverFailed {
    #[new]
    fn new() -> PyClassInitializer<Self> {
        PyClassInitializer::from(SolveStatus).add_subclass(LinearSolverFailed)
    }
}

#[pyclass(extends=SolveStatus)]
pub struct NotConverged {
    #[pyo3(get)]
    pub iteration_errs: Vec<f64>,
}

#[pymethods]
impl NotConverged {
    #[new]
    fn new(iteration_errs: Vec<f64>) -> PyClassInitializer<Self> {
        PyClassInitializer::from(SolveStatus).add_subclass(NotConverged { iteration_errs })
    }
}

#[pyclass(extends=SolveStatus)]
pub struct Converged {
    #[pyo3(get)]
    pub iteration_errs: Vec<f64>,
}

#[pymethods]
impl Converged {
    #[new]
    fn new(iteration_errs: Vec<f64>) -> PyClassInitializer<Self> {
        PyClassInitializer::from(SolveStatus).add_subclass(Converged { iteration_errs })
    }
}

fn solve_status_to_py(py: Python, status: gr::SolveStatus) -> PyResult<Py<PyAny>> {
    match status {
        gr::SolveStatus::LinearSolverFailed => {
            Ok(Py::new(py, PyClassInitializer::from(SolveStatus).add_subclass(LinearSolverFailed))?.into())
        }
        gr::SolveStatus::NotConverged { iteration_errs } => {
            Ok(Py::new(py, PyClassInitializer::from(SolveStatus)
                .add_subclass(NotConverged { iteration_errs }))?.into())
        }
        gr::SolveStatus::Converged { iteration_errs } => {
            Ok(Py::new(py, PyClassInitializer::from(SolveStatus)
                .add_subclass(Converged { iteration_errs }))?.into())
        }
    }
}

#[pyclass(unsendable)]
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

#[pyclass]
pub struct SolveNetworkResult {
    #[pyo3(get)]
    pub statuses: Vec<Py<PyAny>>,
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
        .map(|s| solve_status_to_py(py, s))
        .collect::<PyResult<Vec<_>>>()?;

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

#[pyfunction]
#[pyo3(signature = (netw, solver_name, solvers=None))]
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
