use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pymethods;

use ::griphos as gr;
use gr::{CooMat, c64};
use crate::plumbing::element_subclass;
use crate::network::{BusNodeId, bus_node_ids_to_gr};

element_subclass!(
    /// A generic, non-specialized element.
    ///
    /// Can be used to directly specify any combination of `y_const`, `i_const`, `s_const`, ...
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_node_ids: list of BusNodeId containing (bus_id, phase) for terminals.
    ///     branches: Start and end terminals of each branch (current path) in the element.
    ///     y_const: The optional constant Y branch impedance matrix, as `((i, j), value)` tuples.
    ///     i_const: The optional constant I branch impedance vector, as `(i, value)` tuples.
    ///     s_const: The optional constant S branch impedance vector, as `(i, value)` tuples.
    ///     vmag_const: The optional controlled constant voltage magnitude vector, as `(i, value)` tuples.
    ///     vang_const: The optional controlled constant voltage angle vector, as `(i, value)` tuples.
    ///     copper_plate: The groups of copper-plated (shorted) terminal indices; each group of terminals is shorted.
    ///     grounding: The grounded terminal indices.
    ///
    /// Returns:
    ///     A new `GenericElement` object.
    GenericElement,
    gr::GenericElement
);

impl GenericElement {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    #[allow(clippy::too_many_arguments)]
    pub fn new_raw(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        branches: Vec<(usize, usize)>,
        y_const: Option<CooMat<c64>>,
        i_const: Option<Vec<(usize, c64)>>,
        s_const: Option<Vec<(usize, c64)>>,
        vmag_const: Option<Vec<(usize, f64)>>,
        vang_const: Option<Vec<(usize, f64)>>,
        copper_plate: Option<Vec<Vec<usize>>>,
        grounding: Option<Vec<usize>>,
    ) -> Self {
        let gr_generic_element = gr::GenericElement::new(
            bus_node_ids.len(),
            branches,
            y_const.unwrap_or_default(),
            i_const.unwrap_or_default(),
            s_const.unwrap_or_default(),
            vmag_const.unwrap_or_default(),
            vang_const.unwrap_or_default(),
            copper_plate.unwrap_or_default(),
            grounding.unwrap_or_default(),
        );
        let gr_bts = bus_node_ids_to_gr(bus_node_ids);
        let elem = gr_generic_element.into_element(uid, gr_bts);
        GenericElement::from_typed(elem)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl GenericElement {
    #[new]
    #[pyo3(signature = (
        uid, bus_node_ids, branches, y_const = None, i_const = None, s_const = None, vmag_const = None,
        vang_const = None, copper_plate = None, grounding = None
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        branches: Vec<(usize, usize)>,
        y_const: Option<CooMat<c64>>,
        i_const: Option<Vec<(usize, c64)>>,
        s_const: Option<Vec<(usize, c64)>>,
        vmag_const: Option<Vec<(usize, f64)>>,
        vang_const: Option<Vec<(usize, f64)>>,
        copper_plate: Option<Vec<Vec<usize>>>,
        grounding: Option<Vec<usize>>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(
            uid, bus_node_ids, branches, y_const, i_const, s_const,
            vmag_const, vang_const, copper_plate, grounding
        ).into_initializer()
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use gr::c;
    use super::*;

    #[test]
    fn test_new_generic_element() {
        let bts = vec![
            BusNodeId {b: "bus_1".to_string(), p: Phase::N},
            BusNodeId {b: "bus_1".to_string(), p: Phase::A},
            BusNodeId {b: "bus_1".to_string(), p: Phase::B},
            BusNodeId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let branches = vec![(0, 1), (0, 2), (0, 3)];
        let y_const = vec![
            ((0, 0), c!(1.0, 0.0)),
            ((1, 1), c!(1.0, 0.0)),
            ((2, 2), c!(1.0, 0.0)),
        ];
        let i_const = vec![
            (0, c!(1.0, 0.0)),
            (1, c!(1.0, 0.0)),
            (2, c!(1.0, 0.0)),
        ];
        let s_const = vec![
            (0, c!(1.0, 0.0)),
            (1, c!(1.0, 0.0)),
            (2, c!(1.0, 0.0)),
        ];
        let vmag_const = vec![
            (0, 1.0),
            (1, 1.0),
            (2, 1.0),
        ];

        let generic_element = GenericElement::new_raw(
            "new_generic_element", bts, branches, Some(y_const.clone()), Some(i_const.clone()), Some(s_const.clone()),
            Some(vmag_const.clone()), None, None, None
        );
        let ge_ref = generic_element.elem.borrow();
        assert!(ge_ref.uid() == "new_generic_element");
        assert!(ge_ref.y_const() == y_const);
        assert!(ge_ref.i_const() == i_const);
        assert!(ge_ref.s_const() == s_const);
        assert!(ge_ref.vmag_const() == vmag_const);
        assert!(ge_ref.grounding().is_empty());
        assert!(ge_ref.copper_plate().is_empty());
    }
}
