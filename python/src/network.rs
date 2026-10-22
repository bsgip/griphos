use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pymethods};

use ::griphos as gr;
use gr::{CooMat, c64, HasUid, Shared, IntoShared};

use crate::plumbing::{py_enum, py_value, py_wrapper};

py_enum!(
    /// Encodes the state of a switch if present, or the lack of any switch.
    SwitchState,
    gr::SwitchState { NoSwitch, Closed, Open }
);

py_enum!(
    /// Line or bus phase, A, B, C or N.
    ///
    /// A, B, C are assumed to form a positive sequence, and N is assumed to be a (possibly grounded) neutral.
    Phase,
    gr::Phase { A, B, C, N }
);

py_value!(
    /// A bus UID / phase pair, that together serve to identify a single bus node.
    BusNodeId,
    gr::BusNodeId { pub b: String, pub p: Phase }
);

/// Create a bus node id from a bus uid and a phase.
#[pyfunction]
pub fn bn(b: String, p: Phase) -> BusNodeId {
    BusNodeId { b, p }
}

/// Utility function to map a Vec of BusNodeIds to griphos equivalent.
pub(crate) fn bus_node_ids_to_gr(bus_node_ids: Vec<BusNodeId>) -> Vec<gr::BusNodeId> {
    bus_node_ids.into_iter().map(|x| x.into()).collect()
}

py_value!(
    /// An element UID / branch index pair, that together serve to identify a particular branch of a particular element.
    ElemBranchId,
    gr::ElemBranchId { pub e: String, pub b: usize }
);

py_value!(
    /// Per-branch data of an element
    ElemBranch,
    gr::ElemBranch {
        /// The voltage of the branch, v_term_0 - v_term_1.
        pub v: c64,
        /// The current of the branch, oriented from terminal 0 to terminal 1.
        pub i: c64
    }
);

py_wrapper!(
    /// A bus is a collection of one or more nodes, each distinguished by a different phase.
    Bus,
    gr::Bus
);

#[gen_stub_pymethods]
#[pymethods]
impl Bus {
    #[new]
    fn new(uid: &str, phases: Vec<Phase>, v_base: f64) -> Bus {
        Bus {
            inner: gr::Bus::new(uid, phases.into_iter().map(|x| x.into()).collect(), v_base).into_shared()
        }
    }

    /// Unique ID
    #[getter]
    fn uid(&self) -> String {
        self.inner.borrow().uid()
    }

    /// Base voltage in chosen units.
    #[getter]
    fn v_base(&self) -> f64 {
        self.inner.borrow().v_base
    }

    /// The phases of each bus node.
    #[getter]
    fn phases(&self) -> Vec<Phase> {
        self.inner.borrow().terms.iter().map(|t| Phase::from(t.phase)).collect()
    }

    /// The complex voltages of each bus node.
    #[getter]
    fn vs(&self) -> Vec<c64> {
        self.inner.borrow().terms.iter().map(|t| t.v).collect()
    }

    /// Set the complex voltages of each bus node.
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

py_wrapper!(
    /// An element is a set of current carrying connections netween terminals. It can include several different types of
    /// connection, such as constant y or constant s.
    Element,
    gr::Element, subclass
);

#[gen_stub_pymethods]
#[pymethods]
impl Element {
    /// The element's ID.
    #[getter]
    fn uid(&self) -> String {
        self.inner.borrow().uid()
    }

    /// The element's type string.
    #[getter]
    fn element_type(&self) -> String {
        self.inner.borrow().element_type().to_string()
    }

    /// The number of terminals on the element.
    ///
    /// Each terminal connects to a bus node i.e. a single phase of a bus.
    #[getter]
    pub fn n_terms(&self) -> usize {
        self.inner.borrow().n_terms()
    }

    /// The number of branches in the element.
    ///
    /// Each branch is a current path between two of the element's terminals.
    #[getter]
    pub fn n_branches(&self) -> usize {
        self.inner.borrow().n_branches()
    }

    /// The start and end terminals of each branch.
    ///
    /// Each branch is a current path between two of the element's terminals.
    #[getter]
    pub fn branch_wiring(&self) -> Vec<(usize, usize)> {
        self.inner.borrow().branch_wiring()
    }

    /// Is the element live (functioning).
    ///
    /// Reasons for not being live could be that it is not in service, or that
    /// its switch state is not closed.
    #[getter]
    pub fn is_live(&self) -> bool {
        self.inner.borrow().is_live()
    }

    /// The constant branch impedance matrix of the element.
    ///
    /// Specified as a list of nonzero elements, ((branch_idx_i, branch_idx_j), value)
    #[getter]
    pub fn y_const(&self) -> CooMat<c64> {
        self.inner.borrow().y_const()
    }

    /// The constant branch current vector of the element.
    ///
    /// Specified as a list of nonzero elements, (branch_idx, value)
    #[getter]
    pub fn i_const(&self) -> Vec<(usize, c64)> {
        self.inner.borrow().i_const()
    }

    /// The constant branch complex power vector of the element.
    ///
    /// Specified as a list of nonzero elements, (branch_idx, value)
    #[getter]
    pub fn s_const(&self) -> Vec<(usize, c64)> {
        self.inner.borrow().s_const()
    }

    /// The controlled constant voltage magnitude vector of the element.
    ///
    /// Specified as a list of nonzero elements, (branch_idx, value)
    #[getter]
    pub fn vmag_const(&self) -> Vec<(usize, f64)> {
        self.inner.borrow().vmag_const()
    }

    /// The controlled constant voltage angle vector of the element.
    ///
    /// Specified as a list of nonzero elements, (branch_idx, value)
    #[getter]
    pub fn vang_const(&self) -> Vec<(usize, f64)> {
        self.inner.borrow().vang_const()
    }

    /// The copperplated terminals of the element.
    ///
    /// Specified as a list of copperplated terminal groups, each group being a list of terminal indices.
    #[getter]
    pub fn copper_plate(&self) -> Vec<Vec<usize>> {
        self.inner.borrow().copper_plate()
    }

    /// The grounded terminals of the element.
    #[getter]
    pub fn grounding(&self) -> Vec<usize> {
        self.inner.borrow().grounding()
    }
}

py_wrapper!(
    /// A Network represents an entire electricity network.
    ///
    /// Returns:
    ///     An empty Network object.
    Network,
    gr::Network
);

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

    /// The number of `Bus`es in the network.
    #[getter]
    fn n_buses(&self) -> usize {
        self.inner.borrow().n_buses()
    }

    /// The `Bus`es in the network.
    #[getter]
    fn buses(&self) -> Vec<Bus> {
        self.inner.borrow().buses().map(|b| Bus {inner: b.clone()}).collect()
    }

    /// The `Bus` with given uid in the network.
    ///
    /// Args:
    ///     uid: The ID of the returned bus.
    /// Returns:
    ///     The `Bus` object for the bus with the given ID.
    fn bus(&self, uid: &str) -> Option<Bus> {
        self.inner.borrow().bus(uid)
            .map(|inner| Bus { inner: inner.clone() })
    }

    /// The `Bus` with given index in the network.
    ///
    /// Args:
    ///     i: The index of the returned bus.
    /// Returns:
    ///     The `Bus` object for the bus with the given index.
    fn bus_i(&self, i: usize) -> Option<Bus> {
        self.inner.borrow().bus_i(i)
            .map(|inner| Bus { inner: inner.clone() })
    }

    /// The number of `Element`s in the network.
    #[getter]
    fn n_elements(&self) -> usize {
        self.inner.borrow().n_elements()
    }

    /// The `Element`s in the network.
    #[getter]
    fn elements(&self) -> Vec<Element> {
        self.inner.borrow().elements().map(|e| Element {inner: e.clone()}).collect()
    }

    /// The `Element` with given uid in the network.
    ///
    /// Args:
    ///     uid: The ID of the returned element.
    /// Returns:
    ///     The `Element` object for the element with the given ID.
    fn element(&self, uid: &str) -> Option<Element> {
        self.inner.borrow().element(uid)
            .map(|inner| Element { inner: inner.clone() })
    }

    /// The `Element` with given index in the network.
    ///
    /// Args:
    ///     i: The index of the returned element.
    /// Returns:
    ///     The `Element` object for the element with the given index.
    fn element_i(&self, i: usize) -> Option<Element> {
        self.inner.borrow().element_i(i)
            .map(|inner| Element { inner: inner.clone() })
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner.borrow())
    }
}

// KLUDGE-ish: generate stubs involging &mut Network.
impl pyo3_stub_gen::PyStubType for &mut Network {
    fn type_input() -> pyo3_stub_gen::TypeInfo {
        <Network as pyo3_stub_gen::PyStubType>::type_input()
    }
    fn type_output() -> pyo3_stub_gen::TypeInfo {
        <Network as pyo3_stub_gen::PyStubType>::type_output()
    }
}
