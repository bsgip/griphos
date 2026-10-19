use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pymethods};

use ::griphos as gr;
use gr::{CooMat, c64, HasUid, Shared, IntoShared};

use crate::plumbing::{py_enum, py_value, py_wrapper};

py_enum!(SwitchState, gr::SwitchState { NoSwitch, Closed, Open });

py_enum!(Phase, gr::Phase { A, B, C, N });

py_value!(BusTermId, gr::BusTermId { pub b: String, pub p: Phase });

/// Utility function to map a Vec of BusTermIds to griphos equivalent.
pub(crate) fn bus_term_ids_to_gr(bus_term_ids: Vec<BusTermId>) -> Vec<gr::BusTermId> {
    bus_term_ids.into_iter().map(|x| x.into()).collect()
}

py_value!(ElemBranchId, gr::ElemBranchId { pub e: String, pub b: usize });

py_wrapper!(Bus, gr::Bus);

#[gen_stub_pymethods]
#[pymethods]
impl Bus {
    #[new]
    fn new(uid: &str, phases: Vec<Phase>, v_base: f64) -> Bus {
        Bus {
            inner: gr::Bus::new(uid, phases.into_iter().map(|x| x.into()).collect(), v_base).into_shared()
        }
    }

    #[getter]
    fn uid(&self) -> String {
        self.inner.borrow().uid()
    }

    #[getter]
    fn v_base(&self) -> f64 {
        self.inner.borrow().v_base
    }

    #[getter]
    fn phases(&self) -> Vec<Phase> {
        self.inner.borrow().terms.iter().map(|t| Phase::from(t.phase)).collect()
    }

    #[getter]
    fn vs(&self) -> Vec<c64> {
        self.inner.borrow().terms.iter().map(|t| t.v).collect()
    }

    #[setter]
    fn set_vs(&self, vs: Vec<c64>) {
        for (term, v) in self.inner.borrow_mut().terms.iter_mut().zip(vs) {
            term.v = v;
        }
    }

    fn __str__(&self) -> String {
        format!("{:?}", &self.inner.borrow())
    }
}

py_wrapper!(Element, gr::Element, subclass);

#[gen_stub_pymethods]
#[pymethods]
impl Element {
    #[getter]
    fn uid(&self) -> String {
        self.inner.borrow().uid()
    }

    #[getter]
    fn element_type(&self) -> String {
        self.inner.borrow().element_type().to_string()
    }

    #[getter]
    pub fn n_terms(&self) -> usize {
        self.inner.borrow().n_terms()
    }

    #[getter]
    pub fn n_branches(&self) -> usize {
        self.inner.borrow().n_branches()
    }

    #[getter]
    pub fn branch_wiring(&self) -> Vec<(usize, usize)> {
        self.inner.borrow().branch_wiring()
    }

    #[getter]
    pub fn is_live(&self) -> bool {
        self.inner.borrow().is_live()
    }

    #[getter]
    pub fn y_const(&self) -> CooMat<c64> {
        self.inner.borrow().y_const()
    }

    #[getter]
    pub fn i_const(&self) -> Vec<(usize, c64)> {
        self.inner.borrow().i_const()
    }

    #[getter]
    pub fn s_const(&self) -> Vec<(usize, c64)> {
        self.inner.borrow().s_const()
    }

    #[getter]
    pub fn vmag_const(&self) -> Vec<(usize, f64)> {
        self.inner.borrow().vmag_const()
    }

    #[getter]
    pub fn vang_const(&self) -> Vec<(usize, f64)> {
        self.inner.borrow().vang_const()
    }

    #[getter]
    pub fn copper_plate(&self) -> Vec<Vec<usize>> {
        self.inner.borrow().copper_plate()
    }

    #[getter]
    pub fn grounding(&self) -> Vec<usize> {
        self.inner.borrow().grounding()
    }
}

py_wrapper!(Network, gr::Network);

impl Network {
    pub(crate) fn new_with_inner(inner: gr::Network) -> Network {
        Network { inner: inner.into_shared() }
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Network {
    #[new]
    pub fn new() -> Network {
        Network::new_with_inner(gr::Network::new())
    }

    #[getter]
    fn n_buses(&self) -> usize {
        self.inner.borrow().n_buses()
    }

    #[getter]
    fn buses(&self) -> Vec<Bus> {
        self.inner.borrow().buses().map(|b| Bus {inner: b.clone()}).collect()
    }

    fn bus(&self, uid: &str) -> Option<Bus> {
        self.inner.borrow().bus(uid)
            .map(|inner| Bus { inner: inner.clone() })
    }

    fn bus_i(&self, i: usize) -> Option<Bus> {
        self.inner.borrow().bus_i(i)
            .map(|inner| Bus { inner: inner.clone() })
    }

    #[getter]
    fn n_elements(&self) -> usize {
        self.inner.borrow().n_elements()
    }

    #[getter]
    fn elements(&self) -> Vec<Element> {
        self.inner.borrow().elements().map(|e| Element {inner: e.clone()}).collect()
    }

    fn element(&self, uid: &str) -> Option<Element> {
        self.inner.borrow().element(uid)
            .map(|inner| Element { inner: inner.clone() })
    }

    fn element_i(&self, i: usize) -> Option<Element> {
        self.inner.borrow().element_i(i)
            .map(|inner| Element { inner: inner.clone() })
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner.borrow())
    }
}
