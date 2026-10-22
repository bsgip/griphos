use pyo3::prelude::*;
use pyo3_stub_gen::derive::gen_stub_pymethods;

use ::griphos as gr;
use gr::{Shared, IntoShared};
use crate::plumbing::{element_subclass, py_enum};
use crate::network::{BusTermId, bus_term_ids_to_gr};

py_enum!(
    SwitchState,
    gr::SwitchState {NoSwitch, Closed, Open}
);

element_subclass!(
    /// A simple connector element joining multiple bus terminals.
    ///
    /// Acts as a single node (copper plate) across all of its terminals when
    /// its switch is closed, and splits them apart when open.
    ///
    /// Args:
    ///     uid: the element ID
    ///     bus_term_ids: list of BusTermId containing (bus_id, phase) for terminals
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
        bus_term_ids: Vec<BusTermId>,
        switch_state: SwitchState,
    ) -> Self {
        let gr_connector = gr::Connector::new(bus_term_ids.len(), switch_state.into());
        let bts = bus_term_ids_to_gr(bus_term_ids);
        let elem = gr_connector.element(uid, bts).into_shared();
        Connector::from_elem(elem)
    }
}

#[pymethods]
#[gen_stub_pymethods]
impl Connector {
    #[new]
    pub fn new(
        uid: &str,
        bus_term_ids: Vec<BusTermId>,
        switch_state: SwitchState,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_term_ids, switch_state).into_initializer()
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use super::*;

    #[test]
    fn test_new_connector() {
        let bts = vec![
            BusTermId {b: "bus_1".to_string(), p: Phase::A},
            BusTermId {b: "bus_1".to_string(), p: Phase::B},
            BusTermId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let connector = Connector::new_raw("new_connector", bts, SwitchState::Closed);
        assert!(connector.uid() == "new_connector");
        assert!(connector.elem.borrow().copper_plate() == vec![vec![0, 1, 2]]);
    }
}
