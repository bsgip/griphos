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
    pub n_term: usize,
    pub switch_state: SwitchState,
}

impl Connector {
    /// Creates a `Connector` with the given terminal count and switch state.
    pub fn new(n_term: usize, switch_state: SwitchState) -> Connector {
        Connector {n_term, switch_state}
    }

    /// Wraps this connector as an [`Element`], bound to the given bus terminal IDs.
    ///
    /// `bus_phase_ids` must have length [`Connector::n_term`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use griphos::{bp, BusPhaseId, Connector, Phase, SwitchState};
    /// let connector = Connector::new(2, SwitchState::Closed);
    /// let element = connector.element("c1", vec![bp!("bus_1", Phase::A), bp!("bus_2", Phase::A)]);
    /// ```
    pub fn element(self, uid: &str, bus_phase_ids: Vec<BusPhaseId>) -> Element {
        assert!(bus_phase_ids.len() == self.n_term, "connector {} expects {} bus terminals", uid, self.n_term);
        self.element_generic(uid, bus_phase_ids)
    }
}

impl ElementSpec for Connector {
    /// Returns `true` unless the switch is open.
    fn is_live(&self) -> bool {
        self.switch_state != SwitchState::Open
    }

    /// All terminals are shorted together when the switch is closed.
    fn copper_plate(&self) -> Vec<Vec<usize>> {
        vec![{(0..self.n_term).collect()}]
    }
}

impl_g_serialize!(Connector);
