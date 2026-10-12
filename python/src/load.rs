use pyo3::prelude::*;

use ::griphos as gr;
use gr::{c64, Shared, IntoShared};
use crate::plumbing::element_subclass;
use crate::network::{BusPhaseId, bus_phase_ids_to_gr};

element_subclass!(Load, gr::Load);

impl Load {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
        branches: Vec<(usize, usize)>,
        y_const: Option<CooMat<c64>>,
        i_const: Option<Vec<(usize, c64)>>,
        s_const: Option<Vec<(usize, c64)>>,
        grounding: Option<Vec<usize>>,
    ) -> Self {
        let n_terms = bus_phase_ids.len();
        let gr_load = gr::Load::new(n_terms, branches, y_const, i_const, s_const, grounding);
        let gr_bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_load.element(uid, gr_bps).into_shared();
        Load::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw_wye(
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
        y_const: Option<Vec<c64>>,
        i_const: Option<Vec<c64>>,
        s_const: Option<Vec<c64>>,
        is_grounded: bool,
    ) -> Self {
        let n_phases = bus_phase_ids.len() - 1;
        let gr_load = gr::Load::new_wye(n_phases, y_const, i_const, s_const, is_grounded);
        let gr_bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_load.element(uid, gr_bps).into_shared();
        Load::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw_delta(
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
        y_const: Option<Vec<c64>>,
        i_const: Option<Vec<c64>>,
        s_const: Option<Vec<c64>>,
    ) -> Self {
        let n_phases = bus_phase_ids.len();
        let gr_load = gr::Load::new_delta(n_phases, y_const, i_const, s_const);
        let gr_bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_load.element(uid, gr_bps).into_shared();
        Load::from_elem(elem)
    }
}

#[pymethods]
impl Load {

    #[new]
    pub fn new(
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
        branches: Vec<(usize, usize)>,
        y_const: Option<CooMat<c64>>,
        i_const: Option<Vec<(usize, c64)>>,
        s_const: Option<Vec<(usize, c64)>>,
        grounding: Option<Vec<usize>>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_phase_ids, branches, y_const, i_const, s_const, grounding).into_initializer()
    }

    #[staticmethod]
    pub fn new_wye(
        py: Python<'_>,
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
        y_const: Option<Vec<c64>>,
        i_const: Option<Vec<c64>>,
        s_const: Option<Vec<c64>>,
        is_grounded: bool,
    ) -> PyResult<Py<Self>> {
        Py::new(py, Self::new_raw_wye(uid, bus_phase_ids, y_const, i_const, s_const, is_grounded).into_initializer())
    }

    #[staticmethod]
    pub fn new_delta(
        py: Python<'_>,
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
        y_const: Option<Vec<c64>>,
        i_const: Option<Vec<c64>>,
        s_const: Option<Vec<c64>>,
    ) -> PyResult<Py<Self>> {
        Py::new(py, Self::new_raw_delta(uid, bus_phase_ids, y_const, i_const, s_const).into_initializer())
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use gr::c;
    use super::*;

    #[test]
    fn test_new_load() {
        let bps = vec![
            BusPhaseId {b: "bus_1".to_string(), p: Phase::A},
            BusPhaseId {b: "bus_1".to_string(), p: Phase::B},
            BusPhaseId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let y_const = vec![
            c!(1.0, 0.0),
            c!(1.0, 0.0),
            c!(1.0, 0.0),
        ];
        let load = Load::new_raw_wye("new_load", bps, Some(y_const.clone()), None, None, true);
        assert!(load.uid() == "new_load");
        load.with_spec(|x| assert!(x.y_const == load.elem.borrow().y_const()));
        assert!(load.elem.borrow().i_const() == vec![]);
        assert!(load.elem.borrow().s_const() == vec![]);
        assert!(load.elem.borrow().vmag_const() == vec![]);
        assert!(load.elem.borrow().vang_const() == vec![]);
    }
}
