//! Connector elements.

use indexmap::IndexSet;
use serde::Serialize;

use crate::network::*;
use crate::TypeStrOf;

/// A simple connector element joining multiple bus nodes.
///
/// Acts as a single node (copper plate) across all of its terminals when
/// its switch is closed, and splits them apart when open.
#[derive(Debug, Serialize, TypeStrOf)]
pub struct Connector {
    pub n_terms: usize,
    pub term_groups: Vec<Vec<usize>>,
    pub switch_state: SwitchState,
}

impl Connector {
    /// Creates a `Connector` with the given terminal count and switch state.
    pub fn new(n_terms: usize, switch_state: SwitchState) -> Connector {
        Connector {n_terms, term_groups: vec![], switch_state}
    }

    /// Wraps this connector as an [`Element`], bound to the given bus node IDs.
    ///
    /// `node_ids` must have length [`Connector::n_terms`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use griphos::{nd_id, NodeId, Connector, Phase, SwitchState};
    /// let connector = Connector::new(2, SwitchState::Closed);
    /// let element = connector.into_element("c1", vec![nd_id!("bus_1", Phase::A), nd_id!("bus_2", Phase::A)]);
    /// ```
    pub fn into_element(mut self, uid: &str, node_ids: Vec<NodeId>) -> Element<Self> {
        assert!(
            node_ids.len() == self.n_terms,
            "connector {} expects {} bus nodes, input is {:?}", uid, self.n_terms, node_ids
        );
        let phs = node_ids.iter().map(|nd_id| nd_id.p).collect::<IndexSet<_>>();
        self.term_groups = phs
            .into_iter()
            .map(
                |p|
                node_ids.iter().enumerate().filter(move |(_, nd_id)| nd_id.p == p).map(|(i, _)| i).collect::<Vec<_>>()
            )
            .collect::<Vec<_>>();
        Element::<Self>::new(uid.to_string(), node_ids, self)
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
        self.term_groups.clone()
    }
}

impl_g_serialize!(Connector);
