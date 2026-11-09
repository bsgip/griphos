//! Load elements.

use crate::foundation::{CooMat, CooVec, dense_to_coo_vec, dense_diag_to_coo_mat, c64};
use crate::network::{NodeId, Element, ElementSpec};
use crate::TypeStrOf;

/// A load element, using a ZIP load model.
#[derive(Debug, TypeStrOf)]
pub struct Load {
    /// The number of terminals.
    n_terms: usize,
    /// The branch wiring entries, each item in the form (term_0_idx, term_1_idx)
    branch_wiring: Vec<(usize, usize)>,
    /// constant Y component of ZIP model, equivalent to Z in ZIP.
    ld_y_const: Option<Vec<c64>>,
    /// constant I component of ZIP model, equivalent to I in ZIP.
    ld_i_const: Option<Vec<c64>>,
    /// constant S component of ZIP model, equivalent to P in ZIP.
    ld_s_const: Option<Vec<c64>>,
    /// Index of any grounded terminal(s), normally the neutral terminal of the load if there is one.
    ld_grounding: Vec<usize>,
}

impl Load {
    /// Creates a new load.
    pub fn new(
        n_terms: usize,
        branch_wiring: Vec<(usize, usize)>,
        ld_y_const: Option<Vec<c64>>,
        ld_i_const: Option<Vec<c64>>,
        ld_s_const: Option<Vec<c64>>,
        ld_grounding: Option<Vec<usize>>,
    ) -> Load {
        let ld_grounding = ld_grounding.unwrap_or_default();
        Load {n_terms, branch_wiring, ld_y_const, ld_i_const, ld_s_const, ld_grounding }
    }

    /// Creates a new wye connected load.
    pub fn new_wye(
        n_phases: usize, ld_y_const: Option<Vec<c64>>, ld_i_const: Option<Vec<c64>>, ld_s_const: Option<Vec<c64>>,
        is_grounded: bool
    ) -> Load {
        let n_terms = n_phases + 1;
        let branch_wiring = (0..n_phases).map(|i| (i, n_phases)).collect();
        let ld_grounding = if is_grounded {vec![n_phases]} else {vec![]};
        Load { n_terms, branch_wiring, ld_y_const, ld_i_const, ld_s_const, ld_grounding }
    }

    /// Creates a new delta connected load.
    pub fn new_delta(
        n_phases: usize, ld_y_const: Option<Vec<c64>>, ld_i_const: Option<Vec<c64>>, ld_s_const: Option<Vec<c64>>
    ) -> Load {
        let branch_wiring: Vec<(usize, usize)> = (0..n_phases).map(|i| (i, (i+1) % n_phases)).collect();
        Load { n_terms: n_phases, branch_wiring, ld_y_const, ld_i_const, ld_s_const, ld_grounding: vec![] }
    }

    /// Wraps a [`Load`] in a new [`Element`], with the given terminal mapping.
    pub fn into_element(self, uid: &str, node_ids: Vec<NodeId>) -> Element<Self> {
        Element::<Self>::new(uid.to_string(), node_ids, self)
    }
}

impl Load {
    pub fn ld_y_const(&self) -> Option<&[c64]> {
        self.ld_y_const.as_deref()
    }

    pub fn ld_y_const_mut(&mut self) -> Option<&mut [c64]> {
        self.ld_y_const.as_deref_mut()
    }

    pub fn ld_i_const(&self) -> Option<&[c64]> {
        self.ld_i_const.as_deref()
    }

    pub fn ld_i_const_mut(&mut self) -> Option<&mut [c64]> {
        self.ld_i_const.as_deref_mut()
    }

    pub fn ld_s_const(&self) -> Option<&[c64]> {
        self.ld_s_const.as_deref()
    }

    pub fn ld_s_const_mut(&mut self) -> Option<&mut [c64]> {
        self.ld_s_const.as_deref_mut()
    }

    pub fn ld_grounding(&self) -> &[usize] {
        &self.ld_grounding
    }

    pub fn ld_grounding_mut(&mut self) -> &mut [usize] {
        &mut self.ld_grounding
    }
}

impl ElementSpec for Load {
    fn n_terms(&self) -> usize {
        self.n_terms
    }

    fn n_branches(&self) -> usize {
        self.branch_wiring.len()
    }

    fn branch_wiring(&self) -> Vec<(usize, usize)> {
        self.branch_wiring.clone()
    }

    fn y_const(&self) -> CooMat<c64> {
        self.ld_y_const.as_deref().map_or_else(CooMat::default, dense_diag_to_coo_mat)
    }

    fn i_const(&self) -> CooVec<c64> {
        self.ld_i_const.as_deref().map_or_else(CooVec::default, dense_to_coo_vec)
    }

    fn s_const(&self) -> CooVec<c64> {
        self.ld_s_const.as_deref().map_or_else(CooVec::default, dense_to_coo_vec)
    }

    fn grounding(&self) -> Vec<usize> {
        self.ld_grounding.clone()
    }
}
