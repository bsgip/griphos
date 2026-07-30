use pyo3::prelude::*;

mod adaptors;
mod e_json;
mod line;
mod network;
mod network_solvers;

#[pymodule]
mod griphos {
    #[pymodule_export] use super::network::Bus;
    #[pymodule_export] use super::network::Network;
    #[pymodule_export] use super::network::Phase;
    #[pymodule_export] use super::e_json::parse_ejson;
    #[pymodule_export] use super::e_json::ejson_phase_map_abcng;
    #[pymodule_export] use super::e_json::ejson_phase_map_rwbng;
    #[pymodule_export] use super::network_solvers::SolveStatus;
    #[pymodule_export] use super::network_solvers::SolveNetworkResult;
    #[pymodule_export] use super::network_solvers::Converged;
    #[pymodule_export] use super::network_solvers::NotConverged;
    #[pymodule_export] use super::network_solvers::LinearSolverFailed;
    #[pymodule_export] use super::network_solvers::NetworkSolver;
    #[pymodule_export] use super::network_solvers::solve_network;
}

use pyo3_stub_gen::define_stub_info_gatherer;
define_stub_info_gatherer!(stub_info);
