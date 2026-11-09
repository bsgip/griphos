//! Grounding elements that ground their terminals.

use serde::Serialize;

use crate::network::*;
use crate::TypeStrOf;

/// An element that grounds its terminals.
#[derive(Debug, Serialize, TypeStrOf)]
pub struct Grounding {
    /// The number of terminals to ground.
    pub n_terms: usize,
}

impl Grounding {
    /// Creates a new [`Grounding`].
    pub fn new(n_terms: usize) -> Grounding {
        Grounding {n_terms}
    }

    /// Wraps grounding in an [`Element`] with the given terminal mapping.
    pub fn into_element(self, uid: &str, node_ids: Vec<NodeId>) -> Element<Self> {
        Element::<Self>::new(uid.to_string(), node_ids, self)
    }
}

impl ElementSpec for Grounding {
    fn n_terms(&self) -> usize {
        self.n_terms
    }

    fn n_branches(&self) -> usize {
        0
    }

    fn branch_wiring(&self) -> Vec<(usize, usize)> {
        vec![]
    }

    fn grounding(&self) -> Vec<usize> {(0..self.n_terms).collect()}
}

impl_g_serialize!(Grounding);

