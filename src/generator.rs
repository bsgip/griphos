use serde::Serialize;

use crate::foundation::*;
use crate::network::*;

#[derive(serde::Serialize)]
pub enum GeneratorControl {
    SL {vmag: f64, vang: f64},
    PV {p: f64, vmag: f64},
    PQ {s: c64},
}

#[derive(serde::Serialize)]
pub struct Generator {
    pub n_terms: usize,
    pub branches: Vec<(usize, usize)>,
    pub control: GeneratorControl
}

impl Generator {
    pub fn new(
        n_terms: usize, branches: Vec<(usize, usize)>, control: GeneratorControl
    ) -> Generator {
        Generator {n_terms, branches, control}
    }

    pub fn element(self, id: &str, bus_phases: Vec<BusTermId>) -> Element {
        self.element_generic(id, bus_phases)
    }
}

impl ElementSpec for Generator {
    fn element_type(&self) -> &str {
        "Generator"
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

    fn s_const(&self) -> Vec<(usize, c64)> {
        match self.control {
            GeneratorControl::PV {p, vmag: _} => (0..self.n_branches()).map(|i| (i, c!(p, 0.0))).collect(),
            GeneratorControl::PQ {s} => (0..self.n_branches()).map(|i| (i, s)).collect(),
            _ => vec![]
        }
    }

    fn vmag_const(&self) -> Vec<(usize, f64)> {
        match self.control {
            GeneratorControl::SL {vmag, vang: _} => (0..self.n_branches()).map(|i| (i, vmag)).collect(),
            GeneratorControl::PV {p: _, vmag} => (0..self.n_branches()).map(|i| (i, vmag)).collect(),
            _ => vec![]
        }
    }

    fn vang_const(&self) -> Vec<(usize, f64)> {
        match self.control {
            GeneratorControl::SL {vmag: _, vang} => (0..self.n_branches()).map(|i| (i, vang)).collect(),
            _ => vec![]
        }
    }
}

impl_g_ser_disp!(Generator);
impl_dbg_from_wrap!(Generator);
