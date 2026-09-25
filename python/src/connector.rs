use pyo3::prelude::*;

use ::griphos as gr;
use gr::{Shared, IntoShared};
use crate::plumbing::{element_subclass, py_enum};
use crate::network::{BusPhaseId, bus_phase_ids_to_gr};

py_enum!(SwitchState, gr::SwitchState {NoSwitch, Closed, Open});

element_subclass!(Connector, gr::Connector);

impl Connector {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
        switch_state: SwitchState,
    ) -> Self {
        let gr_connector = gr::Connector::new(bus_phase_ids.len(), switch_state.into());
        let bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_connector.element(uid, bps).into_shared();
        Connector::from_elem(elem)
    }
}

#[pymethods]
impl Connector {

    #[new]
    pub fn new(
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
        switch_state: SwitchState,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_phase_ids, switch_state).into_initializer()
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use super::*;

    #[test]
    fn test_new_connector() {
        let bps = vec![
            BusPhaseId {b: "bus_1".to_string(), p: Phase::A},
            BusPhaseId {b: "bus_1".to_string(), p: Phase::B},
            BusPhaseId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let connector = Connector::new_raw("new_connector", bps, SwitchState::Closed);
        assert!(connector.uid() == "new_connector");
        assert!(connector.elem.borrow().copper_plate() == vec![vec![0, 1, 2]]);
    }
}
