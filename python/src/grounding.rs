use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pymethods;

use ::griphos as gr;
use crate::plumbing::{element_subclass};
use crate::network::{NodeId, node_ids_to_gr};

element_subclass!(
    /// An element that grounds its terminals.
    ///
    /// Args:
    ///     node_ids: list of NodeId containing (bus_id, phase) for terminals.
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
        node_ids: Vec<NodeId>,
    ) -> Self {
        let gr_grounding = gr::Grounding::new(node_ids.len());
        let bts = node_ids_to_gr(node_ids);
        let elem = gr_grounding.into_element(uid, bts);
        Grounding::from_typed(elem)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Grounding {
    #[new]
    pub fn new(
        uid: &str,
        node_ids: Vec<NodeId>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, node_ids).into_initializer()
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use super::*;

    #[test]
    fn test_new_grounding() {
        let bts = vec![
            NodeId {b: "bus_1".to_string(), p: Phase::A},
            NodeId {b: "bus_1".to_string(), p: Phase::B},
            NodeId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let grounding = Grounding::new_raw("new_grounding", bts);
        let grounding_ref = grounding.elem.borrow();
        assert!(grounding_ref.uid() == "new_grounding");
        assert!(grounding_ref.grounding() == vec![0, 1, 2]);
    }
}
