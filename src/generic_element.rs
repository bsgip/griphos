//! A generic element that can have various functions.

use crate::foundation::{CooMat, c64};
use crate::network::{BusNodeId, Element, ElementSpec};
use crate::TypeStrOf;

/// A generic, non-specialized element.
///
/// Can be used to directly specify any combination of `y_const`, `i_const`, `s_const`, ...
#[derive(Debug, Default, TypeStrOf)]
pub struct GenericElement {
    /// Number of terminals.
    pub n_terms: usize,
    /// Branch wiring, as a vector of `(term_0, term_1)` tuples.
    pub branch_wiring: Vec<(usize, usize)>,
    /// Constant admittance component, as branch admittance matrix elements.
    ///
    /// Mostly, the admittance matrix will be diagonal, with indices indicating which branch has the given admittance.
    /// However, we can also have off-diagonal terms `(i, j)` (e.g. for transformers), where the current in branch `i`
    /// depends on the voltage across branch `j`.
    pub y_const: CooMat<c64>,
    /// Constant current component per branch.
    pub i_const: Vec<(usize, c64)>,
    /// Constant complex power component per branch.
    pub s_const: Vec<(usize, c64)>,
    /// Constant voltage magnitude per branch.
    pub vmag_const: Vec<(usize, f64)>,
    /// Constant voltage angle per branch.
    pub vang_const: Vec<(usize, f64)>,
    /// Copper-plated (shorted) branches
    pub copper_plate: Vec<Vec<usize>>,
    /// Grounded bus nodes.
    pub grounding: Vec<usize>,
}

impl GenericElement {
    /// Creates a new GenericElement.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        n_terms: usize, branch_wiring: Vec<(usize, usize)>, y_const: CooMat<c64>, i_const: Vec<(usize, c64)>,
        s_const: Vec<(usize, c64)>, vmag_const: Vec<(usize, f64)>, vang_const: Vec<(usize, f64)>,
        copper_plate: Vec<Vec<usize>>, grounding: Vec<usize>
    ) -> GenericElement {
        GenericElement {
            n_terms, branch_wiring, y_const, i_const, s_const, vmag_const, vang_const, copper_plate, grounding
        }
    }

    /// Wraps a GenericElement in an Element, with given bus nodes connections.
    pub fn into_element(self, uid: &str, bus_node_ids: Vec<BusNodeId>) -> Element<Self> {
        Element::<GenericElement>::new(uid.to_string(), bus_node_ids, self)
    }
}

impl ElementSpec for GenericElement {
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
        self.y_const.clone()
    }

    fn i_const(&self) -> Vec<(usize, c64)> {
        self.i_const.clone()
    }

    fn s_const(&self) -> Vec<(usize, c64)> {
        self.s_const.clone()
    }

    fn vmag_const(&self) -> Vec<(usize, f64)> {
        self.vmag_const.clone()
    }

    fn vang_const(&self) -> Vec<(usize, f64)> {
        self.vang_const.clone()
    }

    fn copper_plate(&self) -> Vec<Vec<usize>> {
        self.copper_plate.clone()
    }

    fn grounding(&self) -> Vec<usize> {
        self.grounding.clone()
    }
}
