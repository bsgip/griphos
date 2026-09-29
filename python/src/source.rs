use pyo3::prelude::*;

use ::griphos as gr;
use gr::{c64, Shared, IntoShared};
use crate::plumbing::{element_subclass};
use crate::network::{BusPhaseId, bus_phase_ids_to_gr};

#[pyclass(eq, from_py_object)]
#[derive(Clone, PartialEq)]
pub enum SourceControl {
    SL {vmag: Vec<f64>, vang: Vec<f64>},
    PV {p: Vec<f64>, vmag: Vec<f64>},
    PQ {s: Vec<c64>},
}

impl From<&gr::SourceControl> for SourceControl {
    fn from(v: &gr::SourceControl) -> Self {
        match v {
            gr::SourceControl::SL {vmag, vang} => SourceControl::SL {vmag: vmag.clone(), vang: vang.clone()},
            gr::SourceControl::PV {p, vmag} => SourceControl::PV {p: p.clone(), vmag: vmag.clone()},
            gr::SourceControl::PQ {s} => SourceControl::PQ {s: s.clone()},
        }
    }
}

impl From<gr::SourceControl> for SourceControl {
    fn from(v: gr::SourceControl) -> Self {
        match v {
            gr::SourceControl::SL {vmag, vang} => SourceControl::SL {vmag, vang},
            gr::SourceControl::PV {p, vmag} => SourceControl::PV {p, vmag},
            gr::SourceControl::PQ {s} => SourceControl::PQ {s},
        }
    }
}

impl From<&SourceControl> for gr::SourceControl {
    fn from(v: &SourceControl) -> Self {
        match v {
            SourceControl::SL {vmag, vang} => gr::SourceControl::SL {vmag: vmag.clone(), vang: vang.clone()},
            SourceControl::PV {p, vmag} => gr::SourceControl::PV {p: p.clone(), vmag: vmag.clone()},
            SourceControl::PQ {s} => gr::SourceControl::PQ {s: s.clone()},
        }
    }
}

impl From<SourceControl> for gr::SourceControl {
    fn from(v: SourceControl) -> Self {
        match v {
            SourceControl::SL {vmag, vang} => gr::SourceControl::SL {vmag, vang},
            SourceControl::PV {p, vmag} => gr::SourceControl::PV {p, vmag},
            SourceControl::PQ {s} => gr::SourceControl::PQ {s},
        }
    }
}

element_subclass!(Source, gr::Source);

impl Source {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
        branches: Vec<(usize, usize)>,
        control: SourceControl
    ) -> Self {
        let gr_source = gr::Source::new(bus_phase_ids.len(), branches, control.into());
        let bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_source.element(uid, bps, false).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_wye_slack_raw(uid: &str, bus_phase_ids: Vec<BusPhaseId>, vmag: f64) -> Self {
        let n_phases = bus_phase_ids.len() - 1;
        let gr_source = gr::Source::new_wye_slack(vmag, n_phases);
        let bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_source.element(uid, bps, true).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_delta_slack_raw(uid: &str, bus_phase_ids: Vec<BusPhaseId>, vmag: f64) -> Self {
        let n_phases = bus_phase_ids.len();
        let gr_source = gr::Source::new_delta_slack(vmag, n_phases);
        let bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_source.element(uid, bps, true).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_wye_pv_raw(uid: &str, bus_phase_ids: Vec<BusPhaseId>, p_tot: f64, vmag: f64) -> Self {
        let n_phases = bus_phase_ids.len() - 1;
        let gr_source = gr::Source::new_wye_pv(p_tot, vmag, n_phases);
        let bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_source.element(uid, bps, true).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_delta_pv_raw(uid: &str, bus_phase_ids: Vec<BusPhaseId>, p_tot: f64, vmag: f64) -> Self {
        let n_phases = bus_phase_ids.len();
        let gr_source = gr::Source::new_delta_pv(p_tot, vmag, n_phases);
        let bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_source.element(uid, bps, true).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_wye_pq_raw(uid: &str, bus_phase_ids: Vec<BusPhaseId>, s_tot: c64) -> Self {
        let n_phases = bus_phase_ids.len() - 1;
        let gr_source = gr::Source::new_wye_pq(s_tot, n_phases);
        let bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_source.element(uid, bps, true).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_delta_pq_raw(uid: &str, bus_phase_ids: Vec<BusPhaseId>, s_tot: c64) -> Self {
        let n_phases = bus_phase_ids.len();
        let gr_source = gr::Source::new_delta_pq(s_tot, n_phases);
        let bps = bus_phase_ids_to_gr(bus_phase_ids);
        let elem = gr_source.element(uid, bps, true).into_shared();
        Source::from_elem(elem)
    }
}

#[pymethods]
impl Source {

    #[new]
    pub fn new(
        uid: &str,
        bus_phase_ids: Vec<BusPhaseId>,
        branches: Vec<(usize, usize)>,
        control: SourceControl,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_phase_ids, branches, control).into_initializer()
    }

    #[staticmethod]
    pub fn new_wye_slack(py: Python<'_>, uid: &str, bus_phase_ids: Vec<BusPhaseId>, vmag: f64) -> PyResult<Py<Self>> {
        Py::new(py, Self::new_wye_slack_raw(uid, bus_phase_ids, vmag).into_initializer())
    }

    #[staticmethod]
    pub fn new_delta_slack(py: Python<'_>, uid: &str, bus_phase_ids: Vec<BusPhaseId>, vmag: f64) -> PyResult<Py<Self>> {
        Py::new(py, Self::new_delta_slack_raw(uid, bus_phase_ids, vmag).into_initializer())
    }

    #[staticmethod]
    pub fn new_wye_pv(py: Python<'_>, uid: &str, bus_phase_ids: Vec<BusPhaseId>, p_tot: f64, vmag: f64) -> PyResult<Py<Self>> {
        Py::new(py, Self::new_wye_pv_raw(uid, bus_phase_ids, p_tot, vmag).into_initializer())
    }

    #[staticmethod]
    pub fn new_delta_pv(py: Python<'_>, uid: &str, bus_phase_ids: Vec<BusPhaseId>, p_tot: f64, vmag: f64) -> PyResult<Py<Self>> {
        Py::new(py, Self::new_delta_pv_raw(uid, bus_phase_ids, p_tot, vmag).into_initializer())
    }

    #[staticmethod]
    pub fn new_wye_pq(py: Python<'_>, uid: &str, bus_phase_ids: Vec<BusPhaseId>, s_tot: c64) -> PyResult<Py<Self>> {
        Py::new(py, Self::new_wye_pq_raw(uid, bus_phase_ids, s_tot).into_initializer())
    }

    #[staticmethod]
    pub fn new_delta_pq(py: Python<'_>, uid: &str, bus_phase_ids: Vec<BusPhaseId>, s_tot: c64) -> PyResult<Py<Self>> {
        Py::new(py, Self::new_delta_pq_raw(uid, bus_phase_ids, s_tot).into_initializer())
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use super::*;

    #[test]
    fn test_new_source() {
        let bps = vec![
            BusPhaseId {b: "bus_1".to_string(), p: Phase::A},
            BusPhaseId {b: "bus_1".to_string(), p: Phase::B},
            BusPhaseId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let branches = vec![(0, 1), (1, 2), (2, 0)];
        let source = Source::new_raw(
            "new_source", bps, branches, SourceControl::SL {vmag: vec![1.0], vang: vec![0.0]}
        );
        assert!(source.uid() == "new_source");
        assert!(source.elem.borrow().copper_plate() == vec![vec![0, 1, 2]]);
    }
}
