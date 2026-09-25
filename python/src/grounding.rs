use pyo3::prelude::*;

use ::griphos as gr;
use gr::{Shared, IntoShared};
use crate::plumbing::{element_subclass};
use crate::network::{BusPhaseId, bus_phase_ids_to_gr};

element_subclass!(Grounding, gr::Grounding);

impl Grounding {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
    ) -> Self {
        let gr_grounding = gr::Grounding::new(bus_phase_ids.len());
        let bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_grounding.element(uid, bps).into_shared();
        Grounding::from_elem(elem)
    }
}

#[pymethods]
impl Grounding {

    #[new]
    pub fn new(
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_phase_ids).into_initializer()
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use super::*;

    #[test]
    fn test_new_grounding() {
        let bps = vec![
            BusPhaseId {b: "bus_1".to_string(), p: Phase::A},
            BusPhaseId {b: "bus_1".to_string(), p: Phase::B},
            BusPhaseId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let grounding = Grounding::new_raw("new_grounding", bps);
        assert!(grounding.uid() == "new_grounding");
        assert!(grounding.elem.borrow().grounding() == vec![0, 1, 2]);
    }
}
