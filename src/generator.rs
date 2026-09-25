//! Generator elements.

use std::fmt::Debug;

use serde::Serialize;

use crate::foundation::*;
use crate::network::*;
use crate::TypeStrOf;

/// Tagged enum for power/voltage control in generators.
#[derive(Debug, serde::Serialize)]
pub enum GeneratorControl {
    SL {vmag: Vec<f64>, vang: Vec<f64>},
    PV {p: Vec<f64>, vmag: Vec<f64>},
    PQ {s: Vec<c64>},
}

impl GeneratorControl {
    /// Creates a new GeneratorControl for a wye-connected slack bus.
    pub fn new_wye_slack_with_std_phase_angles(vmag: f64, live_term_phases: &[Phase]) -> Self {
        let n_branch = live_term_phases.len();
        Self::SL {vmag: vec![vmag; n_branch], vang: live_term_phases.iter().map(|p| p.va_nom()).collect()}
    }

    /// Creates a new GeneratorControl for a delta-connected slack bus, using the normal positive sequence phase angles.
    pub fn new_delta_slack_with_std_phase_angles(vmag: f64, term_phases: &[Phase]) -> Self {
        let n_branch = term_phases.len();
        let vang: Vec<f64> = (0..n_branch).map(
            |i| (term_phases[i].v_nom_pu() - term_phases[(i+1)%n_branch].v_nom_pu()).arg()
        ).collect();
        Self::SL {vmag: vec![vmag; n_branch], vang}
    }

    /// Creates a new GeneratorControl for a PV bus.
    pub fn new_pv(p_tot: f64, vmag: f64, n_phase: usize) -> Self {
        Self::PV { p: vec![p_tot / n_phase as f64; n_phase], vmag: vec![vmag; n_phase] }
    }

    /// Creates a new GeneratorControl for a PQ bus.
    pub fn new_pq(s_tot: c64, n_phase: usize) -> Self {
        Self::PQ { s: vec![s_tot / n_phase as f64; n_phase] }
    }
}

/// A generator or infeeder that may be voltage controlled.
#[derive(Debug, serde::Serialize, TypeStrOf)]
pub struct Generator {
    /// Number of terminals on the generator.
    pub n_terms: usize,
    /// Branches specified as (term_0, term_1) tuples.
    pub branches: Vec<(usize, usize)>,
    /// Generator control, slack, PV or PQ.
    pub control: GeneratorControl
}

impl Generator {
    /// Creates a new [`Generator`].
    pub fn new(
        n_terms: usize, branches: Vec<(usize, usize)>, control: GeneratorControl
    ) -> Generator {
        Generator {n_terms, branches, control}
    }

    /// Wraps a [`Generator`] in a new [`Element`], with the given [`BusTermId`]s.
    pub fn element(self, id: &str, bus_term_ids: Vec<BusTermId>) -> Element {
        self.element_generic(id, bus_term_ids)
    }
}

impl ElementSpec for Generator {
    fn n_terms(&self) -> usize {
        self.n_terms
    }

    fn n_branches(&self) -> usize {
        self.branches.len()
    }

    fn branch_wiring(&self) -> Vec<(usize, usize)> {
        self.branches.clone()
    }

    /// The constant power component of a [`Generator`] depends on its [`GeneratorControl`] type.
    fn s_const(&self) -> Vec<(usize, c64)> {
        match &self.control {
            GeneratorControl::PV {p, vmag: _} => {
                (0..self.n_branches()).zip(p).map(|(i, pi)| (i, c!(*pi, 0.0))).collect()
            },
            GeneratorControl::PQ {s} => {
                (0..self.n_branches()).zip(s).map(|(i, si)| (i, *si)).collect()
            }
            _ => vec![]
        }
    }

    /// The constant voltage magnitude component of a [`Generator`] is present for slack or PV.
    fn vmag_const(&self) -> Vec<(usize, f64)> {
        match &self.control {
            GeneratorControl::SL {vmag, vang: _} => {
                (0..self.n_branches()).zip(vmag).map(|(i, vmagi)| (i, *vmagi)).collect()
            }
            GeneratorControl::PV {p: _, vmag} => {
                (0..self.n_branches()).zip(vmag).map(|(i, vmagi)| (i, *vmagi)).collect()
            },
            _ => vec![]
        }
    }

    /// The constant voltage angle component of a [`Generator`] is present for slack.
    fn vang_const(&self) -> Vec<(usize, f64)> {
        match &self.control {
            GeneratorControl::SL {vmag: _, vang} => {
                (0..self.n_branches()).zip(vang).map(|(i, vangi)| (i, *vangi)).collect()
            },
            _ => vec![]
        }
    }
}

impl_g_serialize!(Generator);
