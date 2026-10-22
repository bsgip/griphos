use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pymethods;

use ::griphos as gr;
use gr::{Shared, IntoShared};
use crate::plumbing::{element_subclass};
use crate::network::{BusNodeId, bus_node_ids_to_gr};

element_subclass!(
    /// An element that grounds its terminals.
    ///
    /// Args:
    ///     bus_node_ids: list of BusNodeId containing (bus_id, phase) for terminals.
    ///
    /// Returns:
    ///     A new `Grounding` object.
    Grounding,
    gr::Grounding
);

impl Grounding {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
    ) -> Self {
        let gr_grounding = gr::Grounding::new(bus_node_ids.len());
        let bts = bus_node_ids_to_gr(bus_node_ids);
        let elem = gr_grounding.element(uid, bts).into_shared();
        Grounding::from_elem(elem)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Grounding {
    #[new]
    pub fn new(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_node_ids).into_initializer()
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use super::*;

    #[test]
    fn test_new_grounding() {
        let bts = vec![
            BusNodeId {b: "bus_1".to_string(), p: Phase::A},
            BusNodeId {b: "bus_1".to_string(), p: Phase::B},
            BusNodeId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let grounding = Grounding::new_raw("new_grounding", bts);
        assert!(grounding.uid() == "new_grounding");
        assert!(grounding.elem.borrow().grounding() == vec![0, 1, 2]);
    }
}
