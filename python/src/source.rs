use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass_enum, gen_stub_pymethods};

use ::griphos as gr;
use gr::{c64, Shared, IntoShared};
use crate::plumbing::{element_subclass};
use crate::network::{BusTermId, bus_term_ids_to_gr};

/// Tagged enum for power/voltage control in sources.
#[pyclass(eq, from_py_object)]
#[gen_stub_pyclass_enum]
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

element_subclass!(
    /// A generation source or infeeder that may be voltage controlled.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_term_ids: list of BusTermId containing (bus_id, phase) for terminals.
    ///     branches: Start and end terminals of each branch (current path) in the element.
    ///     control: `SourceControl` that specifies the type and parameters of voltage control, if any.
    ///     grounding: Optional list of terminal indices that are grounded.
    Source,
    gr::Source
);

impl Source {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        bus_term_ids: Vec<BusTermId>,
        branches: Vec<(usize, usize)>,
        control: SourceControl,
        grounding: Option<Vec<usize>>,
    ) -> Self {
        let gr_source = gr::Source::new(bus_term_ids.len(), branches, control.into(), grounding);
        let bts = bus_term_ids_to_gr(bus_term_ids);
        let elem = gr_source.element(uid, bts, false).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_wye_slack_raw(uid: &str, bus_term_ids: Vec<BusTermId>, vmag: f64, is_grounded: bool) -> Self {
        let n_phases = bus_term_ids.len() - 1;
        let gr_source = gr::Source::new_wye_slack(vmag, n_phases, is_grounded);
        let bts = bus_term_ids_to_gr(bus_term_ids);
        let elem = gr_source.element(uid, bts, true).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_delta_slack_raw(uid: &str, bus_term_ids: Vec<BusTermId>, vmag: f64) -> Self {
        let n_phases = bus_term_ids.len();
        let gr_source = gr::Source::new_delta_slack(vmag, n_phases);
        let bts = bus_term_ids_to_gr(bus_term_ids);
        let elem = gr_source.element(uid, bts, true).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_wye_pv_raw(uid: &str, bus_term_ids: Vec<BusTermId>, p_tot: f64, vmag: f64, is_grounded: bool) -> Self {
        let n_phases = bus_term_ids.len() - 1;
        let gr_source = gr::Source::new_wye_pv(p_tot, vmag, n_phases, is_grounded);
        let bts = bus_term_ids_to_gr(bus_term_ids);
        let elem = gr_source.element(uid, bts, true).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_delta_pv_raw(uid: &str, bus_term_ids: Vec<BusTermId>, p_tot: f64, vmag: f64) -> Self {
        let n_phases = bus_term_ids.len();
        let gr_source = gr::Source::new_delta_pv(p_tot, vmag, n_phases);
        let bts = bus_term_ids_to_gr(bus_term_ids);
        let elem = gr_source.element(uid, bts, true).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_wye_pq_raw(uid: &str, bus_term_ids: Vec<BusTermId>, s_tot: c64, is_grounded: bool) -> Self {
        let n_phases = bus_term_ids.len() - 1;
        let gr_source = gr::Source::new_wye_pq(s_tot, n_phases, is_grounded);
        let bts = bus_term_ids_to_gr(bus_term_ids);
        let elem = gr_source.element(uid, bts, true).into_shared();
        Source::from_elem(elem)
    }

    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_delta_pq_raw(uid: &str, bus_term_ids: Vec<BusTermId>, s_tot: c64) -> Self {
        let n_phases = bus_term_ids.len();
        let gr_source = gr::Source::new_delta_pq(s_tot, n_phases);
        let bts = bus_term_ids_to_gr(bus_term_ids);
        let elem = gr_source.element(uid, bts, true).into_shared();
        Source::from_elem(elem)
    }
}

#[pymethods]
#[gen_stub_pymethods]
impl Source {

    #[new]
    pub fn new(
        uid: &str,
        bus_term_ids: Vec<BusTermId>,
        branches: Vec<(usize, usize)>,
        control: SourceControl,
        grounding: Option<Vec<usize>>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_term_ids, branches, control, grounding).into_initializer()
    }

    /// A new wye-connected slack source with default positive sequence reference angles.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_term_ids: list of BusTermId containing (bus_id, phase) for terminals.
    ///     vmag: the controlled voltage magnitude, single value for all branches.
    ///     is_grounded: Is the neutral terminal grounded?
    ///
    /// Returns:
    ///     A new `Source` object.
    #[staticmethod]
    pub fn new_wye_slack<'py>(py: Python<'py>, uid: &str, bus_term_ids: Vec<BusTermId>, vmag: f64, is_grounded: bool)
    -> PyResult<Bound<'py, Self>>
    {
        Bound::new(py, Self::new_wye_slack_raw(uid, bus_term_ids, vmag, is_grounded).into_initializer())
    }

    /// A new delta-connected slack source with default positive sequence reference angles.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_term_ids: list of BusTermId containing (bus_id, phase) for terminals.
    ///     vmag: the controlled voltage magnitude, single value for all branches.
    ///
    /// Returns:
    ///     A new `Source` object.
    #[staticmethod]
    pub fn new_delta_slack<'py>(py: Python<'py>, uid: &str, bus_term_ids: Vec<BusTermId>, vmag: f64)
    -> PyResult<Bound<'py, Self>>
    {
        Bound::new(py, Self::new_delta_slack_raw(uid, bus_term_ids, vmag).into_initializer())
    }

    /// A new wye-connected PV source with default positive sequence reference angles.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_term_ids: list of BusTermId containing (bus_id, phase) for terminals.
    ///     vmag: the controlled voltage magnitude, single value for all branches.
    ///     is_grounded: Is the neutral terminal grounded?
    ///
    /// Returns:
    ///     A new `Source` object.
    #[staticmethod]
    pub fn new_wye_pv<'py>(
        py: Python<'py>, uid: &str, bus_term_ids: Vec<BusTermId>, p_tot: f64, vmag: f64, is_grounded: bool
    ) -> PyResult<Bound<'py, Self>> {
        Bound::new(py, Self::new_wye_pv_raw(uid, bus_term_ids, p_tot, vmag, is_grounded).into_initializer())
    }

    /// A new delta-connected PV source with default positive sequence reference angles.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_term_ids: list of BusTermId containing (bus_id, phase) for terminals.
    ///     p_tot: the total real power, to be equally distributed among the branches.
    ///     vmag: the controlled voltage magnitude, single value for all branches.
    ///
    /// Returns:
    ///     A new `Source` object.
    #[staticmethod]
    pub fn new_delta_pv<'py>(
        py: Python<'py>, uid: &str, bus_term_ids: Vec<BusTermId>, p_tot: f64, vmag: f64
    ) -> PyResult<Bound<'py, Self>> {
        Bound::new(py, Self::new_delta_pv_raw(uid, bus_term_ids, p_tot, vmag).into_initializer())
    }

    /// A new wye-connected PQ (no voltage control) source with default positive sequence reference angles.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_term_ids: list of BusTermId containing (bus_id, phase) for terminals.
    ///     s_tot: the total complex power, to be equally distributed among the branches.
    ///     is_grounded: Is the neutral terminal grounded?
    ///
    /// Returns:
    ///     A new `Source` object.
    #[staticmethod]
    pub fn new_wye_pq<'py>(py: Python<'py>, uid: &str, bus_term_ids: Vec<BusTermId>, s_tot: c64, is_grounded: bool)
    -> PyResult<Bound<'py, Self>> {
        Bound::new(py, Self::new_wye_pq_raw(uid, bus_term_ids, s_tot, is_grounded).into_initializer())
    }

    /// A new delta-connected PQ (no voltage control) source with default positive sequence reference angles.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_term_ids: list of BusTermId containing (bus_id, phase) for terminals.
    ///     s_tot: the total complex power, to be equally distributed among the branches.
    ///
    /// Returns:
    ///     A new `Source` object.
    #[staticmethod]
    pub fn new_delta_pq<'py>(py: Python<'py>, uid: &str, bus_term_ids: Vec<BusTermId>, s_tot: c64)
    -> PyResult<Bound<'py, Self>> {
        Bound::new(py, Self::new_delta_pq_raw(uid, bus_term_ids, s_tot).into_initializer())
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use super::*;
    use gr::PI;

    #[test]
    fn test_new_source() {
        let bts = vec![
            BusTermId {b: "bus_1".to_string(), p: Phase::A},
            BusTermId {b: "bus_1".to_string(), p: Phase::B},
            BusTermId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let branches = vec![(0, 1), (1, 2), (2, 0)];
        let source = Source::new_raw(
            "new_source", bts, branches,
            SourceControl::SL {vmag: vec![1.0, 1.0, 1.0], vang: vec![0.0, -2.0*PI/3.0, 2.0*PI/3.0]}, None
        );
        assert!(source.uid() == "new_source");
        assert!(source.elem.borrow().vmag_const() == vec![(0, 1.0), (1, 1.0), (2, 1.0)]);
    }
}
