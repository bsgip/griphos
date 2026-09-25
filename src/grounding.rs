//! Grounding elements that ground bus terminals.

use serde::Serialize;

use crate::network::*;
use crate::TypeStrOf;

/// An element that grounds its bus terminals.
#[derive(Debug, Serialize, TypeStrOf)]
pub struct Grounding {
    /// The number of terminals to ground.
    pub n_term: usize,
}

impl Grounding {
    /// Creates a new [`Grounding`].
    pub fn new(n_term: usize) -> Grounding {
        Grounding {n_term}
    }

    /// Wraps grounding in an [`Element`] with the given terminal mapping.
    pub fn element(self, id: &str, bus_phase_ids: Vec<BusPhaseId>) -> Element {
        self.element_generic(id, bus_phase_ids)
    }
}

impl ElementSpec for Grounding {
    fn grounding(&self) -> Vec<usize> {(0..self.n_term).collect()}
}

impl_g_serialize!(Grounding);

