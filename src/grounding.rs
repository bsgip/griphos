use serde::Serialize;

use crate::network::*;
use crate::TypeStrOf;

#[derive(Debug, Serialize, TypeStrOf)]
pub struct Grounding {
    pub n_term: usize,
}

impl Grounding {
    pub fn new(n_term: usize) -> Grounding {
        Grounding {n_term}
    }

    pub fn element(self, id: &str, bus_term_ids: Vec<BusTermId>) -> Element {
        self.element_generic(id, bus_term_ids)
    }
}

impl ElementSpec for Grounding {
    fn grounding(&self) -> Vec<usize> {(0..self.n_term).collect()}
}

impl_g_serialize!(Grounding);

