use serde::Serialize;

use crate::foundation::*;
use crate::network::*;

#[derive(Serialize)]
pub struct Grounding {
    pub n_node: usize,
}

impl Grounding {
    pub fn new(n_node: usize) -> Grounding {
        Grounding {n_node}
    }

    pub fn element(self, id: &str, bus_term_ids: Vec<BusTermId>) -> Element {
        self.element_generic(id, bus_term_ids)
    }
}

impl ElementSpec for Grounding {
    fn element_type(&self) -> &str {"Grounding"}
    fn grounding(&self) -> Vec<usize> {(0..self.n_node).collect()}
}

impl_g_ser_disp!(Grounding);

