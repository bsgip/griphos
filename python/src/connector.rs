use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pymethods;

use ::griphos as gr;
use crate::plumbing::{element_subclass};
use crate::network::{NodeId, node_ids_to_gr, SwitchState};

element_subclass!(
    /// A simple connector element for joining (copperplating) multiple bus nodes.
    ///
    /// Acts as a single node (copper plate) across all of its terminals when
    /// its switch is closed, and splits them apart when open.
    ///
    /// Args:
    ///     uid: the element ID
    ///     node_ids: list of NodeId containing (bus_id, phase) for terminals
    ///     switch_state: SwitchState that can be NoSwitch, Open or Closed
    ///
    /// Returns:
    ///     New Connector object
    Connector,
    gr::Connector
);

impl Connector {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        node_ids: Vec<NodeId>,
        switch_state: SwitchState,
    ) -> Self {
        let gr_connector = gr::Connector::new(node_ids.len(), switch_state.into());
        let bts = node_ids_to_gr(node_ids);
        let elem = gr_connector.into_element(uid, bts);
        Connector::from_typed(elem)
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Connector {
    #[new]
    pub fn new(
        uid: &str,
        node_ids: Vec<NodeId>,
        switch_state: SwitchState,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, node_ids, switch_state).into_initializer()
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use super::*;

    #[test]
    fn test_new_connector() {
        let bts = vec![
            NodeId {b: "bus_1".to_string(), p: Phase::A},
            NodeId {b: "bus_1".to_string(), p: Phase::B},
            NodeId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let connector = Connector::new_raw("new_connector", bts, SwitchState::Closed);
        assert!(connector.elem.borrow().uid() == "new_connector");
        assert!(connector.elem.borrow().copper_plate() == vec![vec![0, 1, 2]]);
    }
}
