use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3_stub_gen::derive::gen_stub_pyfunction;

use crate::network::{Network, Phase};
use crate::py_file_reader::PyFileReader;

/// Parse an e-JSON file like object into a Network.
#[pyfunction]
#[gen_stub_pyfunction]
pub fn parse_ejson(
    file: &Bound<'_, PyAny>,
    phase_map: HashMap<String, Phase>,
) -> PyResult<Network> {
    let core_phase_map: HashMap<String, ::griphos::Phase> = phase_map
        .into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect();

    let reader = PyFileReader {file: file.clone()};
    ::griphos::parse_ejson(reader, &core_phase_map)
        .map(|netw| {Network::new_with_inner(netw)})
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
}

#[pyfunction]
#[gen_stub_pyfunction]
pub fn ejson_phase_map_abcng() -> HashMap<String, Phase> {
    ::griphos::e_json::ejson_phase_map_abcng().into_iter().map(|(k, v)| (k, Phase::from(v)))
        .collect()
}

#[pyfunction]
#[gen_stub_pyfunction]
pub fn ejson_phase_map_rwbng() -> HashMap<String, Phase> {
    ::griphos::e_json::ejson_phase_map_rwbng().into_iter().map(|(k, v)| (k, Phase::from(v)))
        .collect()
}
