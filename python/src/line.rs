use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pymethods;

use ::griphos as gr;
use gr::{CooMat, c64};
use crate::plumbing::element_subclass;
use crate::network::{NodeId, node_ids_to_gr};

element_subclass!(
    /// A line element, with given phase admittance matrix.
    ///
    /// Args:
    ///     uid: the element ID
    ///     node_ids_0: list of NodeId containing (bus_id, phase) for terminals at the start of the line.
    ///     node_ids_1: list of NodeId containing (bus_id, phase) for terminals at the end of the line.
    ///     y_const: Branch admittance matrix, as `((i, j) value)` tuples, e.g. 3x3 matrix for 3 phase line.
    ///
    /// Returns:
    ///     A new `Line` object.
    Line,
    gr::Line
);

impl Line {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        node_ids_0: Vec<NodeId>,
        node_ids_1: Vec<NodeId>,
        y_const: CooMat<c64>,
    ) -> Self {
        let n_phases = node_ids_0.len();
        let gr_line = gr::Line::new(n_phases, y_const);
        let gr_bts_0 = node_ids_to_gr(node_ids_0);
        let gr_bts_1 = node_ids_to_gr(node_ids_1);
        let elem = gr_line.into_element(uid, gr_bts_0, gr_bts_1);
        Line::from_typed(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw_from_sym_seq(
        uid: &str,
        node_ids_0: Vec<NodeId>,
        node_ids_1: Vec<NodeId>,
        z_plus: c64,
        z_0: c64,
        n_phases: usize,
    ) -> Self {
        let gr_bts_0 = node_ids_to_gr(node_ids_0);
        let gr_bts_1 = node_ids_to_gr(node_ids_1);
        let elem = gr::Line::new_from_sym_seq(z_plus, z_0, n_phases)
            .expect("Problem constructing line")
            .into_element(uid, gr_bts_0, gr_bts_1);
        Line::from_typed(elem)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Line {
    #[new]
    pub fn new(
        uid: &str,
        node_ids_0: Vec<NodeId>,
        node_ids_1: Vec<NodeId>,
        y_const: CooMat<c64>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, node_ids_0, node_ids_1, y_const).into_initializer()
    }

    /// A line element, with given z0 and z+. z- is assumed to equal z+.
    ///
    /// Args:
    ///     uid: The element ID
    ///     node_ids_0: List of NodeId containing (bus_id, phase) for terminals at the start of the line.
    ///     node_ids_1: List of NodeId containing (bus_id, phase) for terminals at the end of the line.
    ///     z_plus: the positive (also negative) sequence impedance.
    ///     z_0: the zero sequence impedance.
    ///
    /// Returns:
    ///     A new `Line` object.
    #[staticmethod]
    pub fn new_from_sym_seq<'py>(
        py: Python<'py>,
        uid: &str,
        node_ids_0: Vec<NodeId>,
        node_ids_1: Vec<NodeId>,
        z_plus: c64,
        z_0: c64,
        n_phases: usize
    ) -> PyResult<Bound<'py, Self>> {
        Bound::new(
            py,
            Self::new_raw_from_sym_seq(uid, node_ids_0, node_ids_1, z_plus, z_0, n_phases).into_initializer()
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use gr::c;
    use super::*;

    #[test]
    fn test_new_line() {
        let bts0 = vec![NodeId {b: "bus_1".to_string(), p: Phase::A}];
        let bts1 = vec![NodeId {b: "bus_2".to_string(), p: Phase::A}];
        let y_const = vec![((0, 0), c!(1.0, 0.0))];
        let line = Line::new_raw("new_line", bts0, bts1, y_const.clone());
        let line_ref = line.elem.borrow();
        assert!(line_ref.uid() == "new_line");
        assert!(line_ref.y_const() == y_const);
        assert!(line_ref.i_const().is_empty());
        assert!(line_ref.s_const().is_empty());
    }
}
