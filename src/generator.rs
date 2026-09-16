use serde::Serialize;

use crate::foundation::*;
use crate::network::*;

#[derive(serde::Serialize)]
pub enum GeneratorControl {
    SL {vmags: Vec<f64>, vangs: Vec<f64>},
    PV {p: Vec<f64>, vmags: Vec<f64>},
    PQ {s: Vec<c64>},
}

impl GeneratorControl {
    pub fn new_wye_slack_with_std_phase_angles(vmag: f64, live_term_phases: &[Phase]) -> Self {
        let n_branch = live_term_phases.len();
        Self::SL {vmags: vec![vmag; n_branch], vangs: live_term_phases.iter().map(|p| p.va_nom()).collect()}
    }

    pub fn new_delta_slack_with_std_phase_angles(vmag: f64, term_phases: &[Phase]) -> Self {
        let n_branch = term_phases.len();
        let vangs: Vec<f64> = (0..n_branch).map(
            |i| (term_phases[i].v_nom_pu() - term_phases[(i+1)%n_branch].v_nom_pu()).arg()
        ).collect();
        Self::SL {vmags: vec![vmag; n_branch], vangs}
    }
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

    pub fn element(self, id: &str, bus_term_ids: Vec<BusTermId>) -> Element {
        self.element_generic(id, bus_term_ids)
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
        match &self.control {
            GeneratorControl::PV {p, vmags: _} => {
                (0..self.n_branches()).zip(p).map(|(i, pi)| (i, c!(*pi, 0.0))).collect()
            },
            GeneratorControl::PQ {s} => {
                (0..self.n_branches()).zip(s).map(|(i, si)| (i, *si)).collect()
            }
            _ => vec![]
        }
    }

    fn vmag_const(&self) -> Vec<(usize, f64)> {
        match &self.control {
            GeneratorControl::SL {vmags, vangs: _} => {
                (0..self.n_branches()).zip(vmags).map(|(i, vmagi)| (i, *vmagi)).collect()
            }
            GeneratorControl::PV {p: _, vmags} => {
                (0..self.n_branches()).zip(vmags).map(|(i, vmagi)| (i, *vmagi)).collect()
            },
            _ => vec![]
        }
    }

    fn vang_const(&self) -> Vec<(usize, f64)> {
        match &self.control {
            GeneratorControl::SL {vmags: _, vangs} => {
                (0..self.n_branches()).zip(vangs).map(|(i, vangi)| (i, *vangi)).collect()
            },
            _ => vec![]
        }
    }
}

impl_g_ser_disp!(Generator);
impl_dbg_from_wrap!(Generator);
