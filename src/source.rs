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
    /// Branch wiring specified as (term_0, term_1) tuples.
    pub branch_wiring: Vec<(usize, usize)>,
    /// Source control, slack, PV or PQ.
    pub control: SourceControl,
    /// Index of any grounded terminal(s), normally the neutral terminal of the load if there is one.
    pub grounding: Vec<usize>,
}

impl Source {
    /// Creates a new [`Source`].
    pub fn new(
        n_terms: usize, branch_wiring: Vec<(usize, usize)>, control: SourceControl, grounding: Option<Vec<usize>>
    ) -> Source {
        Source { n_terms, branch_wiring, control, grounding: grounding.unwrap_or_default() }
    }

    /// Creates a new wye-connected slack source with default positive sequence reference angles.
    pub fn new_wye_slack(vmag: f64, n_phases: usize, is_grounded: bool) -> Self {
        let vang: Vec<f64> = (0..n_phases).map(|i| (-(i as f64) * TAU / 3.0).rem_euclid(TAU)).collect();
        let grounding = if is_grounded {vec![n_phases]} else {vec![]};
        Self {
            n_terms: n_phases + 1, // Add 1 for neutral.
            branch_wiring: wye_connection(n_phases).unwrap(),
            control: SourceControl::SL {vmag: vec![vmag; n_phases], vang },
            grounding,
        }
    }

    /// Creates a new delta-connected slack source with default positive sequence reference angles.
    pub fn new_delta_slack(vmag: f64, n_phases: usize) -> Self {
        let vang: Vec<f64> = (0..n_phases).map(|i| (((1 - i) as f64) * TAU / 3.0).rem_euclid(TAU)).collect();
        Self {
            n_terms: n_phases,
            branch_wiring: delta_connection(n_phases).unwrap(),
            control: SourceControl::SL {vmag: vec![vmag; n_phases], vang },
            grounding: vec![],
        }
    }

    /// Creates a new wye-connected PV source. n_phases is the number of live terminals.
    pub fn new_wye_pv(p_tot: f64, vmag: f64, n_phases: usize, is_grounded: bool) -> Self {
        let grounding = if is_grounded {vec![n_phases]} else {vec![]};
        Self {
            n_terms: n_phases + 1,
            branch_wiring: wye_connection(n_phases).unwrap(),
            control: SourceControl::PV { p: vec![p_tot / n_phases as f64; n_phases], vmag: vec![vmag; n_phases] },
            grounding,
        }
    }

    /// Creates a new delta-connected PV source.
    pub fn new_delta_pv(p_tot: f64, vmag: f64, n_phases: usize) -> Self {
        Self {
            n_terms: n_phases,
            branch_wiring: delta_connection(n_phases).unwrap(),
            control: SourceControl::PV { p: vec![p_tot / n_phases as f64; n_phases], vmag: vec![vmag; n_phases] },
            grounding: vec![],
        }
    }

    /// Creates a new wye-connected PQ (uncontrolled) source. n_phases is the number of live terminals.
    pub fn new_wye_pq(s_tot: c64, n_phases: usize, is_grounded: bool) -> Self {
        let grounding = if is_grounded {vec![n_phases]} else {vec![]};
        Self {
            n_terms: n_phases + 1,
            branch_wiring: wye_connection(n_phases).unwrap(),
            control: SourceControl::PQ { s: vec![s_tot / n_phases as f64; n_phases] },
            grounding,
        }
    }

    /// Creates a new delta-connected PQ (uncontrolled) source.
    pub fn new_delta_pq(s_tot: c64, n_phases: usize) -> Self {
        Self {
            n_terms: n_phases + 1,
            branch_wiring: delta_connection(n_phases).unwrap(),
            control: SourceControl::PQ { s: vec![s_tot / n_phases as f64; n_phases] },
            grounding: vec![],
        }
    }

    /// Wraps a [`Source`] in a new [`Element`], with the given [`BusNodeId`]s.
    ///
    /// If set_angle_refs_from_phases is true and control is slack, set the phase reference angles using the standard
    /// positive sequence angles for phases A, B, C.
    pub fn into_element(mut self, uid: &str, bus_node_ids: Vec<BusNodeId>, set_angle_refs_from_phases: bool)
    -> Element<Self> {
        if set_angle_refs_from_phases && let SourceControl::SL {vmag: _, vang } = &mut self.control {
            *vang = self.branch_wiring.iter().map(
                |(i0, i1)| (bus_node_ids[*i0].p.v_nom_pu() - bus_node_ids[*i1].p.v_nom_pu()).arg()
            ).collect();
        }
        Element::<Self>::new(uid.to_string(), bus_node_ids, self)
    }
}

impl ElementSpec for Source {
    fn n_terms(&self) -> usize {
        self.n_terms
    }

    fn n_branches(&self) -> usize {
        self.branch_wiring.len()
    }

    fn branch_wiring(&self) -> Vec<(usize, usize)> {
        self.branch_wiring.clone()
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

    fn grounding(&self) -> Vec<usize> {
        self.grounding.clone()
    }
}

impl_g_serialize!(Source);
