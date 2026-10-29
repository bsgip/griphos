use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3_stub_gen::derive::gen_stub_pymethods;

use ::griphos as gr;
use gr::c64;
use crate::plumbing::element_subclass;
use crate::network::{BusNodeId, bus_node_ids_to_gr};

element_subclass!(
    /// A load element, using a ZIP load model.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_node_ids: list of BusNodeId containing (bus_id, phase) for terminals.
    ///     branches: Start and end terminals of each branch (current path) in the element.
    ///     ld_y_const: The optional constant Y branch impedance matrix, as `((i, j), value)` tuples.
    ///     ld_i_const: The optional constant I branch impedance vector, as `(i, value)` tuples.
    ///     ld_s_const: The optional constant S branch impedance vector, as `(i, value)` tuples.
    ///     ld_grounding: The optional grounded terminal indices.
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
        ld_y_const: Option<Vec<c64>>,
        ld_i_const: Option<Vec<c64>>,
        ld_s_const: Option<Vec<c64>>,
        ld_grounding: Option<Vec<usize>>,
    ) -> Self {
        let n_terms = bus_node_ids.len();
        let gr_load = gr::Load::new(n_terms, branches, ld_y_const, ld_i_const, ld_s_const, ld_grounding);
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
        ld_y_const: Option<Vec<c64>>,
        ld_i_const: Option<Vec<c64>>,
        ld_s_const: Option<Vec<c64>>,
        ld_grounding: Option<Vec<usize>>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_node_ids, branches, ld_y_const, ld_i_const, ld_s_const, ld_grounding).into_initializer()
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

    #[getter]
    fn ld_y_const(&self) -> Option<Vec<c64>> {
        self.elem.borrow().ld_y_const().map(<[c64]>::to_vec)
    }

    #[setter]
    fn set_ld_y_const(&self, values: Vec<c64>) -> PyResult<()> {
        let mut b = self.elem.borrow_mut();
        let dst = b.ld_y_const_mut().ok_or_else(|| PyValueError::new_err("load has no y_const"))?;
        if dst.len() != values.len() {
            return Err(PyValueError::new_err(format!("expected {} values, got {}", dst.len(), values.len())));
        }
        dst.copy_from_slice(&values);
        Ok(())
    }

    #[getter]
    fn ld_i_const(&self) -> Option<Vec<c64>> {
        self.elem.borrow().ld_i_const().map(<[c64]>::to_vec)
    }

    #[setter]
    fn set_ld_i_const(&self, values: Vec<c64>) -> PyResult<()> {
        let mut b = self.elem.borrow_mut();
        let dst = b.ld_i_const_mut().ok_or_else(|| PyValueError::new_err("load has no i_const"))?;
        if dst.len() != values.len() {
            return Err(PyValueError::new_err(format!("expected {} values, got {}", dst.len(), values.len())));
        }
        dst.copy_from_slice(&values);
        Ok(())
    }

    #[getter]
    fn ld_s_const(&self) -> Option<Vec<c64>> {
        self.elem.borrow().ld_s_const().map(<[c64]>::to_vec)
    }

    #[setter]
    fn set_ld_s_const(&self, values: Vec<c64>) -> PyResult<()> {
        let mut b = self.elem.borrow_mut();
        let dst = b.ld_s_const_mut().ok_or_else(|| PyValueError::new_err("load has no s_const"))?;
        if dst.len() != values.len() {
            return Err(PyValueError::new_err(format!("expected {} values, got {}", dst.len(), values.len())));
        }
        dst.copy_from_slice(&values);
        Ok(())
    }

    #[getter]
    fn ld_grounding(&self) -> Vec<usize> {
        self.elem.borrow().ld_grounding().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use gr::{c, CooMat};
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
        assert_eq!(load_ref.ld_y_const(), Some(y_const.as_slice()));
        assert!(load_ref.ld_i_const().is_none());
        assert!(load_ref.ld_s_const().is_none());
    }
}
