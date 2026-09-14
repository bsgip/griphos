use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

mod e_json;
mod line;
mod network;
mod network_solvers;
mod plumbing;
mod py_file_reader;
mod transformer;

define_stub_info_gatherer!(stub_info);

#[pymodule]
mod griphos {
    use ::griphos as gr;
    use bitflags::Flags;
    use pyo3::prelude::*;

    #[pymodule_export] use super::network::{
        SwitchState, Phase, Bus, Element, Network
    };
    #[pymodule_export] use super::e_json::{
        parse_ejson, ejson_phase_map_abcng, ejson_phase_map_rwbng
    };
    #[pymodule_export] use super::network_solvers::{
        SolveNetworkResult, NetworkSolver, solve_network,
    };
    #[pymodule_export] use super::line::{
        Line,
    };

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        for flag in gr::SolverStatusFlags::FLAGS {
            m.add(flag.name(), flag.value().bits())?;
        }
        Ok(())
    }
}
