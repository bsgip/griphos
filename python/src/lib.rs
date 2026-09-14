use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

mod connector;
mod e_json;
mod generator;
mod generic_element;
mod grounding;
mod line;
mod load;
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

    #[pymodule_export] use super::connector::{
        Connector
    };
    #[pymodule_export] use super::e_json::{
        parse_ejson, ejson_phase_map_abcng, ejson_phase_map_rwbng
    };
    #[pymodule_export] use super::generator::{
        Generator, GeneratorControl
    };
    #[pymodule_export] use super::generic_element::{
        GenericElement
    };
    #[pymodule_export] use super::grounding::{
        Grounding
    };
    #[pymodule_export] use super::line::{
        Line,
    };
    #[pymodule_export] use super::load::{
        Load,
    };
    #[pymodule_export] use super::network::{
        SwitchState, Phase, Bus, Element, Network
    };
    #[pymodule_export] use super::network_solvers::{
        SolveNetworkResult, NetworkSolver, solve_network,
    };
    #[pymodule_export] use super::transformer::{
        Transformer, TransformerSide, TransformerTaps, TransformerTapsData
    };

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        for flag in gr::SolverStatusFlags::FLAGS {
            m.add(flag.name(), flag.value().bits())?;
        }
        Ok(())
    }
}
