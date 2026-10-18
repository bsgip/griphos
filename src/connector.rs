//! Connector elements.

use serde::Serialize;

use crate::network::*;
use crate::TypeStrOf;

/// A simple connector element joining multiple bus terminals.
///
/// Acts as a single node (copper plate) across all of its terminals when
/// its switch is closed, and splits them apart when open.
#[derive(Debug, Serialize, TypeStrOf)]
pub struct Connector {
    pub n_terms: usize,
    pub switch_state: SwitchState,
}

impl Connector {
    /// Creates a `Connector` with the given terminal count and switch state.
    pub fn new(n_terms: usize, switch_state: SwitchState) -> Connector {
        Connector {n_terms, switch_state}
    }

    /// Wraps this connector as an [`Element`], bound to the given bus terminal IDs.
    ///
    /// `bus_term_ids` must have length [`Connector::n_terms`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use griphos::{bt, BusTermId, Connector, Phase, SwitchState};
    /// let connector = Connector::new(2, SwitchState::Closed);
    /// let element = connector.element("c1", vec![bt!("bus_1", Phase::A), bt!("bus_2", Phase::A)]);
    /// ```
    pub fn element(self, uid: &str, bus_term_ids: Vec<BusTermId>) -> Element {
        assert!(bus_term_ids.len() == self.n_terms, "connector {} expects {} bus terminals", uid, self.n_terms);
        self.element_generic(uid, bus_term_ids)
    }
}

impl ElementSpec for Connector {
    fn n_terms(&self) -> usize {
        self.n_terms
    }

    fn n_branches(&self) -> usize {
        0
    }

    fn branch_wiring(&self) -> Vec<(usize, usize)> {
        vec![]
    }

    /// Returns `true` unless the switch is open.
    fn is_live(&self) -> bool {
        self.switch_state != SwitchState::Open
    }

    /// All terminals are shorted together when the switch is closed.
    fn copper_plate(&self) -> Vec<Vec<usize>> {
        vec![{(0..self.n_terms).collect()}]
    }
}

impl_g_serialize!(Connector);
