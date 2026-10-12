//! Source elements.

use std::fmt::Debug;

use serde::Serialize;

use crate::foundation::*;
use crate::network::*;
use crate::TypeStrOf;

/// Tagged enum for power/voltage control in sources.
#[derive(Debug, serde::Serialize)]
pub enum SourceControl {
    SL {vmag: Vec<f64>, vang: Vec<f64>},
    PV {p: Vec<f64>, vmag: Vec<f64>},
    PQ {s: Vec<c64>},
}

/// A generation source or infeeder that may be voltage controlled.
#[derive(Debug, serde::Serialize, TypeStrOf)]
pub struct Source {
    /// Number of terminals on the source.
    pub n_terms: usize,
    /// Branches specified as (term_0, term_1) tuples.
    pub branches: Vec<(usize, usize)>,
    /// Source control, slack, PV or PQ.
    pub control: SourceControl
}

impl Source {
    /// Creates a new [`Source`].
    pub fn new(
        n_terms: usize, branches: Vec<(usize, usize)>, control: SourceControl
    ) -> Source {
        Source {n_terms, branches, control}
    }

    /// Creates a new wye-connected slack source with default positive sequence reference angles.
    pub fn new_wye_slack(vmag: f64, n_phases: usize) -> Self {
        let vang: Vec<f64> = (0..n_phases).map(|i| (-(i as f64) * TAU / 3.0).rem_euclid(TAU)).collect();
        Self {
            n_terms: n_phases + 1, // Add 1 for neutral.
            branches: wye_connection(n_phases).unwrap(),
            control: SourceControl::SL {vmag: vec![vmag; n_phases], vang },
        }
    }

    /// Creates a new delta-connected slack source with default positive sequence reference angles.
    pub fn new_delta_slack(vmag: f64, n_phases: usize) -> Self {
        let vang: Vec<f64> = (0..n_phases).map(|i| (((1 - i) as f64) * TAU / 3.0).rem_euclid(TAU)).collect();
        Self {
            n_terms: n_phases,
            branches: delta_connection(n_phases).unwrap(),
            control: SourceControl::SL {vmag: vec![vmag; n_phases], vang },
        }
    }

    /// Creates a new wye-connected PV source. n_phases is the number of live terminals.
    pub fn new_wye_pv(p_tot: f64, vmag: f64, n_phases: usize) -> Self {
        Self {
            n_terms: n_phases + 1,
            branches: wye_connection(n_phases).unwrap(),
            control: SourceControl::PV { p: vec![p_tot / n_phases as f64; n_phases], vmag: vec![vmag; n_phases] }
        }
    }

    /// Creates a new delta-connected PV source.
    pub fn new_delta_pv(p_tot: f64, vmag: f64, n_phases: usize) -> Self {
        Self {
            n_terms: n_phases,
            branches: delta_connection(n_phases).unwrap(),
            control: SourceControl::PV { p: vec![p_tot / n_phases as f64; n_phases], vmag: vec![vmag; n_phases] }
        }
    }

    /// Creates a new wye-connected PQ (uncontrolled) source. n_phases is the number of live terminals.
    pub fn new_wye_pq(s_tot: c64, n_phases: usize) -> Self {
        Self {
            n_terms: n_phases + 1,
            branches: wye_connection(n_phases).unwrap(),
            control: SourceControl::PQ { s: vec![s_tot / n_phases as f64; n_phases] }
        }
    }

    /// Creates a new delta-connected PQ (uncontrolled) source.
    pub fn new_delta_pq(s_tot: c64, n_phases: usize) -> Self {
        Self {
            n_terms: n_phases + 1,
            branches: delta_connection(n_phases).unwrap(),
            control: SourceControl::PQ { s: vec![s_tot / n_phases as f64; n_phases] }
        }
    }

    /// Wraps a [`Source`] in a new [`Element`], with the given [`BusTermId`]s.
    ///
    /// If set_angle_refs_from_phases is true and control is slack, set the phase reference angles using the standard
    /// positive sequence angles for phases A, B, C.
    pub fn element(mut self, id: &str, bus_term_ids: Vec<BusTermId>, set_angle_refs_from_phases: bool) -> Element {
        if set_angle_refs_from_phases && let SourceControl::SL {vmag: _, vang } = &mut self.control {
            *vang = self.branches.iter().map(
                |(i0, i1)| (bus_term_ids[*i0].p.v_nom_pu() - bus_term_ids[*i1].p.v_nom_pu()).arg()
            ).collect();
        }
        self.element_generic(id, bus_term_ids)
    }
}

impl ElementSpec for Source {
    fn n_terms(&self) -> usize {
        self.n_terms
    }

    fn n_branches(&self) -> usize {
        self.branches.len()
    }

    fn branch_wiring(&self) -> Vec<(usize, usize)> {
        self.branches.clone()
    }

    /// The constant power component of a [`Source`] depends on its [`SourceControl`] type.
    fn s_const(&self) -> Vec<(usize, c64)> {
        match &self.control {
            SourceControl::PV {p, vmag: _} => {
                (0..self.n_branches()).zip(p).map(|(i, pi)| (i, c!(*pi, 0.0))).collect()
            },
            SourceControl::PQ {s} => {
                (0..self.n_branches()).zip(s).map(|(i, si)| (i, *si)).collect()
            }
            _ => vec![]
        }
    }

    /// The constant voltage magnitude component of a [`Source`] is present for slack or PV.
    fn vmag_const(&self) -> Vec<(usize, f64)> {
        match &self.control {
            SourceControl::SL {vmag, vang: _} => {
                (0..self.n_branches()).zip(vmag).map(|(i, vmagi)| (i, *vmagi)).collect()
            }
            SourceControl::PV {p: _, vmag} => {
                (0..self.n_branches()).zip(vmag).map(|(i, vmagi)| (i, *vmagi)).collect()
            },
            _ => vec![]
        }
    }

    /// The constant voltage angle component of a [`Source`] is present for slack.
    fn vang_const(&self) -> Vec<(usize, f64)> {
        match &self.control {
            SourceControl::SL {vmag: _, vang} => {
                (0..self.n_branches()).zip(vang).map(|(i, vangi)| (i, *vangi)).collect()
            },
            _ => vec![]
        }
    }
}

impl_g_serialize!(Source);
