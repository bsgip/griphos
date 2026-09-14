use crate::foundation::c64;
use crate::network::{BusTermId, Element, ElementSpec};

#[derive(Debug)]
pub struct Load {
    pub n_terms: usize,
    pub branches: Vec<(usize, usize)>,
    pub y_const: Vec<((usize, usize), c64)>,
    pub i_const: Vec<(usize, c64)>,
    pub s_const: Vec<(usize, c64)>,
    pub grounding: Vec<usize>,
}

impl Load {
    pub fn new_wye(
        n_phases: usize, y_const: Option<Vec<c64>>, i_const: Option<Vec<c64>>, s_const: Option<Vec<c64>>,
        is_grounded: bool
    ) -> Load {
        let branches = (1..=n_phases).map(|i| (i, 0)).collect();

        let y_const: Vec<((usize, usize), c64)> = if let Some(yca) = y_const {
            yca.iter().cloned().enumerate().map(|(i, ycai)| ((i, i), ycai)).collect()
        } else {
            vec![]
        };

        let i_const = if let Some(ica) = i_const {ica.iter().cloned().enumerate().collect()} else {vec![]};

        let s_const = if let Some(sca) = s_const {sca.iter().cloned().enumerate().collect()} else {vec![]};

        let grounding = if is_grounded {vec![0]} else {vec![]};

        Load {
            n_terms: n_phases + 1,
            branches,
            y_const,
            i_const,
            s_const,
            grounding,
        }
    }

    pub fn new_delta(
        n_phases: usize, y_const: Option<Vec<c64>>, i_const: Option<Vec<c64>>, s_const: Option<Vec<c64>>
    ) -> Load {
        let branches: Vec<(usize, usize)> = (0..=n_phases).map(|i| (i, (i+1) % n_phases)).collect();

        let y_const: Vec<((usize, usize), c64)> = if let Some(yca) = y_const {
            yca.iter().cloned().enumerate().map(|(i, ycai)| ((i, i), ycai)).collect()
        } else {
            vec![]
        };

        let i_const = if let Some(ica) = i_const {ica.iter().cloned().enumerate().collect()} else {vec![]};

        let s_const = if let Some(sca) = s_const {sca.iter().cloned().enumerate().collect()} else {vec![]};

        Load {
            n_terms: n_phases,
            branches,
            y_const,
            i_const,
            s_const,
            grounding: vec![],
        }
    }

    pub fn element(self, id: &str, bus_term_ids: Vec<BusTermId>) -> Element {
        self.element_generic(id, bus_term_ids)
    }
}

impl ElementSpec for Load {
    fn element_type(&self) -> &str {
        "Load"
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
    fn grounding(&self) -> Vec<usize> {
        self.grounding.clone()
    }
}
