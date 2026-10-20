use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;

mod connector;
mod e_json;
mod generic_element;
mod grounding;
mod line;
mod load;
mod network;
mod network_solvers;
mod plumbing;
mod py_file_reader;
mod source;
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
        bt, Bus, Element, Network, Phase, SwitchState,
    };
    #[pymodule_export] use super::network_solvers::{
        SolveNetworkResult, NetworkSolver, solve_network,
    };
    #[pymodule_export] use super::source::{
        Source, SourceControl
    };
    #[pymodule_export] use super::transformer::{
        Transformer, TransformerTaps, TransformerTapsData
    };
    #[pymodule_export] const C_ZERO: gr::c64 = gr::C_ZERO;
    #[pymodule_export] const C_ONE: gr::c64 = gr::C_ONE;
    #[pymodule_export] const C_I: gr::c64 = gr::C_I;
    #[pymodule_export] const SQRT_2: f64 = gr::SQRT_2;
    #[pymodule_export] const SQRT_2_INV: f64 = gr::SQRT_2_INV;
    #[pymodule_export] const SQRT_3: f64 = gr::SQRT_3;
    #[pymodule_export] const SQRT_3_INV: f64 = gr::SQRT_3_INV;
    #[pymodule_export] const PI: f64 = gr::PI;
    #[pymodule_export] const TAU: f64 = gr::TAU;

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        for flag in gr::SolverStatusFlags::FLAGS {
            m.add(flag.name(), flag.value().bits())?;
        }
        Ok(())
    }
}
