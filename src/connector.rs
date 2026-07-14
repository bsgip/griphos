use serde::Serialize;

use crate::foundation::*;
use crate::network::*;

#[derive(Serialize)]
pub struct Connector {
    pub n_node: usize,
    pub switch_state: SwitchState,
}

impl Connector {
    pub fn new(n_node: usize, switch_state: SwitchState) -> Connector {
        Connector {n_node, switch_state}
    }

    pub fn element(self, id: &str, bus_phases: Vec<BusTermId>) -> Element {
        self.element_generic(id, bus_phases)
    }
}

impl ElementSpec for Connector {
    fn element_type(&self) -> &str {"Connector"}
    fn is_live(&self) -> bool {self.switch_state != SwitchState::Open}
    fn copper_plate(&self) -> Vec<Vec<usize>> {vec![{(0..self.n_node).collect()}]}
}

impl_g_ser_disp!(Connector);

