use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pymethods;

use ::griphos as gr;
use gr::{CooMat, c64};
use crate::plumbing::element_subclass;
use crate::network::{BusNodeId, bus_node_ids_to_gr};

element_subclass!(
    /// A load element, using a ZIP load model.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_node_ids: list of BusNodeId containing (bus_id, phase) for terminals.
    ///     branches: Start and end terminals of each branch (current path) in the element.
    ///     y_const: The optional constant Y branch impedance matrix, as `((i, j), value)` tuples.
    ///     i_const: The optional constant I branch impedance vector, as `(i, value)` tuples.
    ///     s_const: The optional constant S branch impedance vector, as `(i, value)` tuples.
    ///     grounding: The optional grounded terminal indices.
    /// Returns:
    ///     A new `Load` object.
    Load,
    gr::Load
);

impl Load {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        branches: Vec<(usize, usize)>,
        y_const: Option<CooMat<c64>>,
        i_const: Option<Vec<(usize, c64)>>,
        s_const: Option<Vec<(usize, c64)>>,
        grounding: Option<Vec<usize>>,
    ) -> Self {
        let n_terms = bus_node_ids.len();
        let gr_load = gr::Load::new(n_terms, branches, y_const, i_const, s_const, grounding);
        let gr_bts = bus_node_ids_to_gr(bus_node_ids);
        let elem = gr_load.into_element(uid, gr_bts);
        Load::from_typed(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw_wye(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        y_const: Option<Vec<c64>>,
        i_const: Option<Vec<c64>>,
        s_const: Option<Vec<c64>>,
        is_grounded: bool,
    ) -> Self {
        let n_phases = bus_node_ids.len() - 1;
        let gr_load = gr::Load::new_wye(n_phases, y_const, i_const, s_const, is_grounded);
        let gr_bts = bus_node_ids_to_gr(bus_node_ids);
        let elem = gr_load.into_element(uid, gr_bts);
        Load::from_typed(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw_delta(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        y_const: Option<Vec<c64>>,
        i_const: Option<Vec<c64>>,
        s_const: Option<Vec<c64>>,
    ) -> Self {
        let n_phases = bus_node_ids.len();
        let gr_load = gr::Load::new_delta(n_phases, y_const, i_const, s_const);
        let gr_bts = bus_node_ids_to_gr(bus_node_ids);
        let elem = gr_load.into_element(uid, gr_bts);
        Load::from_typed(elem)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Load {
    #[new]
    pub fn new(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        branches: Vec<(usize, usize)>,
        y_const: Option<CooMat<c64>>,
        i_const: Option<Vec<(usize, c64)>>,
        s_const: Option<Vec<(usize, c64)>>,
        grounding: Option<Vec<usize>>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_node_ids, branches, y_const, i_const, s_const, grounding).into_initializer()
    }

    /// Creates a new wye connected load.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_node_ids: list of BusNodeId containing (bus_id, phase) for terminals, with the final one being neutral.
    ///     y_const: Constant impedance component for each phase (branch) of the load.
    ///     i_const: Constant current component for each phase (branch) of the load.
    ///     s_const: Constant complex power component for each phase (branch) of the load.
    ///     is_grounded: Is the neutral (final) terminal grounded
    #[staticmethod]
    pub fn new_wye<'py>(
        py: Python<'py>,
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        y_const: Option<Vec<c64>>,
        i_const: Option<Vec<c64>>,
        s_const: Option<Vec<c64>>,
        is_grounded: bool,
    ) -> PyResult<Bound<'py, Self>> {
        Bound::new(py, Self::new_raw_wye(uid, bus_node_ids, y_const, i_const, s_const, is_grounded).into_initializer())
    }

    /// Creates a new delta connected load.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_node_ids: list of BusNodeId containing (bus_id, phase) for terminals.
    ///     y_const: Constant impedance component for each phase (branch) of the load.
    ///     i_const: Constant current component for each phase (branch) of the load.
    ///     s_const: Constant complex power component for each phase (branch) of the load.
    #[staticmethod]
    pub fn new_delta<'py>(
        py: Python<'py>,
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        y_const: Option<Vec<c64>>,
        i_const: Option<Vec<c64>>,
        s_const: Option<Vec<c64>>,
    ) -> PyResult<Bound<'py, Self>> {
        Bound::new(py, Self::new_raw_delta(uid, bus_node_ids, y_const, i_const, s_const).into_initializer())
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use gr::c;
    use super::*;

    #[test]
    fn test_new_load() {
        let bts = vec![
            BusNodeId {b: "bus_1".to_string(), p: Phase::A},
            BusNodeId {b: "bus_1".to_string(), p: Phase::B},
            BusNodeId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let y_const = vec![
            c!(1.0, 0.0),
            c!(1.0, 0.0),
            c!(1.0, 0.0),
        ];
        let load = Load::new_raw_wye("new_load", bts, Some(y_const.clone()), None, None, true);
        let load_ref = load.elem.borrow();
        assert!(load_ref.uid() == "new_load");
        assert!(load_ref.y_const() == y_const.iter().enumerate().map(|(i, x)| ((i, i), *x)).collect::<CooMat<c64>>());
        assert!(load_ref.i_const().is_empty());
        assert!(load_ref.s_const().is_empty());
    }
}
