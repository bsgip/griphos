use crate::foundation::c64;
use crate::network::{BusTermId, Element, ElementSpec};

#[derive(Default)]
pub struct GenericElement {
    pub n_terms: usize,
    pub branches: Vec<(usize, usize)>,
    pub y_const: Vec<((usize, usize), c64)>,
    pub i_const: Vec<(usize, c64)>,
    pub s_const: Vec<(usize, c64)>,
    pub vmag_const: Vec<(usize, f64)>,
    pub vang_const: Vec<(usize, f64)>,
    pub copper_plate: Vec<Vec<usize>>,
    pub grounding: Vec<usize>,
}

impl GenericElement {
    pub fn new(
        n_terms: usize, branches: Vec<(usize, usize)>, y_const: Vec<((usize, usize), c64)>, i_const: Vec<(usize, c64)>,
        s_const: Vec<(usize, c64)>, vmag_const: Vec<(usize, f64)>, vang_const: Vec<(usize, f64)>,
        copper_plate: Vec<Vec<usize>>, grounding: Vec<usize>
    ) -> GenericElement {
        GenericElement {n_terms, branches, y_const, i_const, s_const, vmag_const, vang_const, copper_plate, grounding}
    }

    pub fn element(self, id: &str, bus_phases: Vec<BusTermId>) -> Element {
        self.element_generic(id, bus_phases)
    }
}

impl ElementSpec for GenericElement {
    fn element_type(&self) -> &str {
        "GenericElement"
    }
    fn n_terms(&self) -> usize {
        self.n_terms
    }
    fn n_branches(&self) -> usize {
        self.branches.len()
    }
    fn branch_wiring(&self) -> Vec<(usize, usize)> {
        self.branches.clone()
    }
    fn y_const(&self) -> Vec<((usize, usize), c64)> {
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
