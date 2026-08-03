use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

mod e_json;
mod line;
mod network;
mod network_solvers;
mod plumbing;

define_stub_info_gatherer!(stub_info);

#[pymodule]
mod griphos {
    #[pymodule_export] use super::network::{
        SwitchState, Phase, Bus, Element, Network
    };
    #[pymodule_export] use super::e_json::{
        parse_ejson, ejson_phase_map_abcng, ejson_phase_map_rwbng
    };
    #[pymodule_export] use super::network_solvers::{
        SolveStatus, SolveNetworkResult, Converged, NotConverged, LinearSolverFailed, NetworkSolver, solve_network,
    };
    #[pymodule_export] use super::line::{
        Line,
    };
}
