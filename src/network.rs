//! Network modelling.
//!
//! [`Network`]s comprise a list of [`Bus`]es and a list of [`Element`]s.
//!
//! Buses are nodes of the network, and consist of one or more blobs or bars of conductor with negligible resistance,
//! the bus nodes, or [`BusNode`]s. [`BusNode`]s contain a [`Phase`] label and a voltage. Each [`BusNode`] is
//! uniquely labelled by a [`BusNodeId`], recording the bus's UID and the phase.
//!
//! Buses are connected together by elements, which provide conductive pathways (termed *branches* in griphos
//! terminology) between pairs of bus nodes. Each [`Element`] has a given number of element *terminals* (connection
//! points), and is connected to the [`Network`] by assigning to each terminal a particular [`BusNodeId`]. Each of the
//! element's branches then provides a conductive pathway between a start terminal and an end terminal.
//!
//! For example, a three phase line with no neutral wire will have six terminals: three at the start of the line, and
//! three at the end. It will have three branches, each corresponding to a wire of the line. For the griphos terminal
//! ordering found in [`Line`](crate::Line) elements, the first branch will connect terminal 0 to terminal 3, the second
//! will connect terminal 1 to terminal 4, and the third will connect terminal 2 to terminal 5. Thus the [`Element`]'s
//! branches can be specified as `[(0, 3), (1, 4), (2, 5)]`
//!
//! The impedance of an [`Element`]'s branches can depend in various ways on the voltages at the terminals. For example,
//! branches can have a constant admittance (*y_const*) component, or a constant complex power (*s_const*) component. In
//! addition, they can apply voltage control across some or all of their branches, or ground some or all of their
//! terminals. The configuration and behaviour of the element with respect to its terminals and branches depends on the
//! *element type*. This is technically handled by [`Element`] having a private dynamic `spec` member, implementing a
//! special [`ElementSpec`] trait. There are several structs, such as [`Line`](crate::Line) and
//! [`Transformer`](crate::Transformer) that implement [`ElementSpec`], and each [`Element`] derives its polymorphic
//! behaviour from its own particular [`ElementSpec`], be it a line, transformer, or whatever. Thus, we can talk about
//! e.g. transformer elements, or line elements, meaning that the `spec` is either a [`Transformer`](crate::Transformer)
//! or [`Line`](crate::Line) respectively.
//!
//! # Examples
//!
//! <details>
//! <summary>Network tutorial example</summary>
//!
//! ```
#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../examples/network_tutorial.rs")]
//! ```
//! </details>

use std::any::Any;
use std::collections::HashSet;
use std::fmt;
use std::fmt::Display;
use std::hash::{Hash};
use std::ops::{Deref, DerefMut};
use std::marker::PhantomData;
use std::vec::Vec;

use indexmap::map::Entry;
use indexmap::{IndexMap};

use serde::ser::SerializeStruct;
use serde::Serialize;

use crate::foundation::*;

/// Encodes the state of a switch if present, or the lack of any switch.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum SwitchState {
    /// There is no switch.
    NoSwitch,
    /// There is a switch and it is closed (on).
    Closed,
    /// There is a switch and it is open (off).
    Open,
}

impl_g_serialize!(SwitchState);

impl fmt::Display for SwitchState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Remove quotation marks
        let s = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", &s[1..s.len()-1])
    }
}

/// Line or bus phase, A, B, C or N.
///
/// A, B, C are assumed to form a positive sequence, and N is assumed to be a (possibly grounded) neutral.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Phase {
    A,
    B,
    C,
    N,
}

impl Phase {
    /// Returns the standard phase angle of a phase, assuming ABC form a positive sequence set and A is the reference.
    ///
    /// N is arbitrarily assumed to have a standard phase angle of zero.
    pub fn va_nom(&self) -> f64 {
        match self {
            Phase::A => 0.0,
            Phase::B => 4.0 * std::f64::consts::PI / 3.0,
            Phase::C => 2.0 * std::f64::consts::PI / 3.0,
            Phase::N => 0.0
        }
    }

    /// Returns the standard pu voltage of a phase, assuming ABC form a positive sequence set and A is the reference.
    pub fn v_nom_pu(&self) -> c64 {
        match self {
            Phase::A => {p!(1.0, 0.0)},
            Phase::B => {p!(1.0, 4.0 * std::f64::consts::PI / 3.0)},
            Phase::C => {p!(1.0, 2.0 * std::f64::consts::PI / 3.0)},
            Phase::N => {c!(0.0, 0.0)},
        }
    }
}

impl_g_serialize!(Phase);

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Remove quotation marks
        let s = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", &s[1..s.len()-1])
    }
}

/// Returns terminal connections for a standard wye connection. Neutral is always considered to come last by convention.
pub fn wye_connection(n_phases: usize) -> Result<Vec<(usize, usize)>, String> {
    match n_phases {
        1 => Ok(vec![(0, 1)]),
        2 => Ok(vec![(0, 2), (1, 2)]),
        3 => Ok(vec![(0, 3), (1, 3), (2, 3)]),
        _ => Err(String::from("a wye connection must have 1, 2 or 3 phases"))
    }
}

/// Returns terminal connections for a standard wye connection. Neutral is always considered to come last by convention.
pub fn delta_connection(n_phases: usize) -> Result<Vec<(usize, usize)>, String> {
    match n_phases {
        1 => Ok(vec![(0, 1)]),
        3 => Ok(vec![(0, 1), (1, 2), (2, 0)]),
        _ => Err(String::from("a delta connection must have 1 or 3 phases"))
    }
}

/// A bus UID / phase pair, that together serve to identify a single bus node.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BusNodeId {
    /// Bus uid.
    pub b: String,
    /// Phase.
    pub p: Phase
}

impl_g_serialize!(BusNodeId);
impl fmt::Display for BusNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Remove quotation marks
        write!(f, "{}:{}", self.b, self.p)
    }
}

/// Macro to create a [`BusNodeId`]
///
/// # Examples
///
/// ```
/// # use griphos::{bn, BusNodeId, Phase};
/// let bus_node = bn!("bus_1", Phase::A);
/// ```
#[macro_export]
macro_rules! bn {
    ($b:expr, $p:expr) => {
        BusNodeId {b: $b.to_string(), p: $p}
    }
}

/// An element UID / branch index pair, that together serve to identify a particular branch of a particular element.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ElemBranchId {
    /// Element uid.
    pub e: String,
    /// Branch index.
    pub b: usize
}

impl_g_serialize!(ElemBranchId);

impl fmt::Display for ElemBranchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Remove quotation marks
        write!(f, "{}:{}", self.e, self.b)
    }
}

/// Macro to create an [`ElemBranchId`]
///
/// # Examples
///
/// ```
/// # use griphos::{eb, ElemBranchId};
/// let elem_branch = eb!("element_1", 3);
/// ```
#[macro_export]
macro_rules! eb {
    ($e:expr, $b:expr) => {
        ElemBranchId {e: $e.to_string(), b: $b}
    }
}

/// Identifies a delta or wye connection.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Wiring {
    DELTA,
    WYE,
}

impl_g_serialize!(Wiring);

impl fmt::Display for Wiring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Remove quotation marks
        let s = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{}", &s[1..s.len()-1])
    }
}

/// Per-phase data of a bus.
#[derive(Clone, Debug, Serialize)]
pub struct BusNode {
    /// The phase
    pub phase: Phase,
    /// Voltage
    pub v: c64,
}

impl_g_serialize!(BusNode);
impl_disp_from_fmt!(BusNode);

/// A bus is a collection of one or more nodes, each distinguished by a different phase.
#[derive(Clone, Debug, Serialize)]
pub struct Bus {
    /// Unique ID
    pub uid: String,

    /// Base voltage in chosen units.
    pub v_base: f64,

    /// Terminals
    pub terms: Vec<BusNode>,
}

impl Bus {
    /// Creates a new [`Bus`].
    pub fn new(uid: &str, phases: Vec<Phase>, v_base: f64) -> Bus {
        let terms: Vec<BusNode> = phases.iter()
            .map(|phase| BusNode {phase: *phase, v: v_base * phase.v_nom_pu()}).collect();
        Bus {uid: uid.to_string(), v_base, terms}
    }

    /// Reterns the unique ID of the Bus.
    pub fn uid(&self) -> String {
        self.uid.clone()
    }

    /// Searches for the terminal of a bus that has phase `p`.
    pub fn bus_node(&self, p: Phase) -> Option<&BusNode> {
        self.terms.iter().find(|x| x.phase == p)
    }

    /// Searches for the terminal of a bus that has phase `p`, mutable version.
    pub fn bus_node_mut(&mut self, p: Phase) -> Option<&mut BusNode> {
        self.terms.iter_mut().find(|x| x.phase == p)
    }

    /// Searches for the terminal index of a bus that has phase `p`.
    pub fn i_bus_node(&self, p: Phase) -> Option<usize> {
        self.terms.iter().position(|x| x.phase == p)
    }

    /// Returns the terminal voltages as an [`Iterator`].
    pub fn vs(&self) -> impl Iterator<Item = c64> {
        self.terms.iter().map(|t| t.v)
    }

    /// Sets the terminal voltages using an [`Iterator`].
    pub fn set_vs(&mut self, vs: impl Iterator<Item=c64>) {
        self.terms.iter_mut().zip(vs).for_each(|(t, v)| t.v = v);
    }
}

impl_g_serialize!(Bus);
impl_disp_from_fmt!(Bus);

/// Per-branch data of an element
#[derive(Clone, Debug, Default, Serialize)]
pub struct ElemBranch {
    /// Branch voltage. For branch (i, j), v is defined v_i - v_j.
    pub v: c64,
    /// Branch current. For branch (i, j), i has directionality i -> j.
    pub i: c64,
}

impl_g_serialize!(ElemBranch);
impl_disp_from_fmt!(ElemBranch);

#[derive(Debug)]
pub struct ElementCommon {
    pub uid: String,                    // Unique ID
    pub is_in_service: bool,            // Is the element in service / operational.
    pub bus_node_ids: Vec<BusNodeId>,   // Key of which bus/phases the terminals are connected to.
    pub branches: Vec<ElemBranch>,      // The branch info, a vector of [`ElemBranch`] of the element.
}

/// Trait for types that have an associated type_str string. Each ElementSpec must implement this by hand.
pub trait TypeStrOf {
    fn type_str_of() -> String where Self: Sized;
}

/// Bridge to allow calling on an instance; don't re-implement, machinery to allow dynamic type strings.
pub trait TypeStr {
    fn type_str(&self) -> String;
}

/// Blanket implementation of TypeStr for all types, machinery to allow dynamic type strings.
impl<T: TypeStrOf> TypeStr for T {
    fn type_str(&self) -> String {
        T::type_str_of()
    }
}

/// Parameter bundle that is convenient for the constraints of generic [`ElementSpec`]s.
///
/// The machinery here simply means any generic `ElementSpec`, say `Foo<T: ElementSpecParam>`, will work, so long as T
/// obeys the constraints.
pub trait ElementSpecParam: std::fmt::Debug + Send + Sync + 'static {}
impl<T: std::fmt::Debug + Send + Sync + 'static> ElementSpecParam for T {}

/// An element_spec collects together electrical connections of various types such as constant y or constant s.
pub trait ElementSpec: Any + std::fmt::Debug + Send + Sync + TypeStrOf + TypeStr {
    /// Returns the number of terminals (connection points).
    fn n_terms(&self) -> usize;

    /// Returns the number of branches (node-node connections)
    fn n_branches(&self) -> usize;

    /// Branch wiring, term_idx_i -> term_idx_j.
    fn branch_wiring(&self) -> Vec<(usize, usize)>;

    /// Is the element live (e.g. switched on)?
    fn is_live(&self) -> bool {true}

    /// Branch impedance matrix; I_i = y_ij V_j where i and j index branches.
    fn y_const(&self) -> CooMat<c64> {vec![]}

    /// Branch constant currents, where the index is a branch index.
    fn i_const(&self) -> Vec<(usize, c64)> {vec![]}

    /// Branch constant power, where the index is a branch index.
    fn s_const(&self) -> Vec<(usize, c64)> {vec![]}

    /// Branch voltage magnitude setpoint, where the index is a branch index.
    fn vmag_const(&self) -> Vec<(usize, f64)> {vec![]}

    /// Branch voltage angle setpoint, where the index is a branch index.
    fn vang_const(&self) -> Vec<(usize, f64)> {vec![]}

    /// List of (list of terminals that are copper plated together).
    fn copper_plate(&self) -> Vec<Vec<usize>> {vec![]}

    /// Terminal grounding as a list of indices of grounded terminals.
    fn grounding(&self) -> Vec<usize> {vec![]}

    /// Generic element validation code.
    // TODO: actually use this!
    fn validate(&self) -> Result<(), String> {
        let n_terms = self.n_terms();
        let branch_wiring = self.branch_wiring();
        let n_branches = branch_wiring.len();
        for (i, j) in &branch_wiring {
            if i == j {
                return Err("A branch (i, j) must not have i == j".to_string());
            }
            if *i >= n_terms || *j >= n_terms {
                return Err("A branch (i, j) must i < n_terms and j < n_terms".to_string());
            }
        }
        let s: HashSet<(usize, usize)> = branch_wiring.iter().cloned().collect();
        if s.len() != n_branches {
            return Err("Duplicate branches are not allowed".to_string());
        }

        for ((i, j), _) in self.y_const() {
            if i >= n_branches || j >= n_branches {
                return Err("y_const entries must contain valid branch indices".to_string());
            }
        }

        for (i, _) in self.i_const() {
            if i >= n_branches {
                return Err("i_const entries must contain valid branch indices".to_string());
            }
        }

        for (i, _) in self.s_const() {
            if i >= n_branches {
                return Err("s_const entries must contain valid branch indices".to_string());
            }
        }

        for (i, _) in self.vmag_const() {
            if i >= n_branches {
                return Err("vmag_const entries must contain valid branch indices".to_string());
            }
        }

        for (i, _) in self.vang_const() {
            if i >= n_branches {
                return Err("vang_const entries must contain valid branch indices".to_string());
            }
        }

        Ok(())
    }
}

impl dyn ElementSpec {
    pub fn is<S: ElementSpec>(&self) -> bool {
        let any: &dyn Any = self;
        any.is::<S>()
    }

    pub fn downcast_ref<S: ElementSpec>(&self) -> Option<&S> {
        let any: &dyn Any = self;
        any.downcast_ref()
    }

    pub fn downcast_mut<S: ElementSpec>(&mut self) -> Option<&mut S> {
        let any: &mut dyn Any = self;
        any.downcast_mut()
    }
}

#[derive(Debug)]
pub struct Foo<T> {
    pub t: T,
}

#[derive(Debug)]
struct ElementInner {
    // The common data.
    common: ElementCommon,

    // The specialised (dynamic) behaviour of the element.
    spec: Box<dyn ElementSpec>,
}

/// An element is a set of current carrying connections netween terminals. It can include several different types of
/// connection, such as constant y or constant s.
#[derive(Debug)]
pub struct Element<S: ElementSpec + ?Sized = dyn ElementSpec> {
    inner: Shared<ElementInner>,
    _m: PhantomData<fn(&S)>,
}

impl Element {
    pub fn is<S: ElementSpec>(&self) -> bool {
        self.borrow().spec.is::<S>()
    }

    pub fn downcast<S: ElementSpec>(&self) -> Option<Element<S>> {
        self.is::<S>().then(|| Element::from_inner(self.inner.clone()))
    }

    pub fn borrow(&self) -> ElementBorrowed<'_> {
        let (common, spec) = Borrowed::map_split(self.inner.borrow(), |i| {(&i.common, i.spec.as_ref())});
        ElementBorrowed { common, spec }
    }

    pub fn borrow_mut(&self) -> ElementBorrowedMut<'_> {
        let (common, spec) = BorrowedMut::map_split(self.inner.borrow_mut(), |i| {(&mut i.common, i.spec.as_mut())});
        ElementBorrowedMut { common, spec }
    }
}

impl<S: ElementSpec + ?Sized> Element<S> {
    fn from_inner(inner: Shared<ElementInner>) -> Element<S> {
        Element { inner, _m: PhantomData }
    }

    /// Upcasts to a generic Element.
    pub fn erase(&self) -> Element {
        Element::from_inner(self.inner.clone())
    }
}

impl<S: ElementSpec> Element<S> {
    pub fn new(
        uid: String, bus_node_ids: Vec<BusNodeId>, spec: S,
    ) -> Self {
        let n_terms = spec.n_terms();
        assert!(bus_node_ids.len() == n_terms, "Element expects {} bus_node_ids", n_terms);
        let n_branches = spec.n_branches();
        let branches = vec![ElemBranch::default(); n_branches];
        let common = ElementCommon { uid, is_in_service: true, bus_node_ids, branches };
        Self::from_inner(ElementInner { common, spec: Box::new(spec) }.into_shared())
    }

    pub fn borrow(&self) -> ElementBorrowed<'_, S> {
        let (common, spec) = Borrowed::map_split(self.inner.borrow(), |i| {
            (&i.common, i.spec.as_ref().downcast_ref::<S>().expect("spec type invariant"))
        });
        ElementBorrowed { common, spec }
    }

    pub fn borrow_mut(&self) -> ElementBorrowedMut<'_, S> {
        let (common, spec) = BorrowedMut::map_split(self.inner.borrow_mut(), |i| {
            (&mut i.common, i.spec.as_mut().downcast_mut::<S>().expect("spec type invariant"))
        });
        ElementBorrowedMut { common, spec }
    }
}

impl<S: ElementSpec + ?Sized> Clone for Element<S> {
    fn clone(&self) -> Self {
        Element { inner: self.inner.clone(), _m: PhantomData }
    }
}

pub struct ElementGuard<C, P> {
    common: C, // Borrowed<ElementCommon> or BorrowedMut<ElementCommon>
    spec: P,   // Borrowed<S> or BorrowedMut<S>
}

pub type ElementBorrowed<'a, S = dyn ElementSpec> = ElementGuard<Borrowed<'a, ElementCommon>, Borrowed<'a, S>>;
pub type ElementBorrowedMut<'a, S = dyn ElementSpec> = ElementGuard<BorrowedMut<'a, ElementCommon>, BorrowedMut<'a, S>>;

impl<C, P> ElementGuard<C, P>
where
    C: Deref<Target = ElementCommon>,
    P: Deref,
    P::Target: ElementSpec,
{
    /// Returns the unique ID of the element.
    pub fn uid(&self) -> String {
        self.common.uid.clone()
    }

    /// Returns the in_service status of the element.
    pub fn is_in_service(&self) -> bool {
        self.common.is_in_service
    }

    /// Returns a reference to the [`BusNodId`]s of the element.
    pub fn bus_node_ids(&self) -> &[BusNodeId] {
        &self.common.bus_node_ids
    }

    /// Returns a reference to the [`ElemBranch`]es of the element.
    pub fn branches(&self) -> &[ElemBranch] {
        &self.common.branches
    }

    /// Type string of an element, corresponding to the dynamic type of its `spec`.
    pub fn element_type(&self) -> String {
        self.spec.type_str()
    }

    /// Returns the number of terminals of the element.
    pub fn n_terms(&self) -> usize {
        self.spec.n_terms()
    }

    /// Returns the number of branches of the element.
    pub fn n_branches(&self) -> usize {
        if self.is_live() {self.spec.n_branches()} else {0}
    }

    /// Returns the branch wiring as a vector of (i, j) terminal connections.
    pub fn branch_wiring(&self) -> Vec<(usize, usize)> {
        if self.is_live() {self.spec.branch_wiring()} else {vec![]}
    }

    /// Is the element live (in service and switched on if there is a swtich).
    pub fn is_live(&self) -> bool {
        self.is_in_service() && self.spec.is_live()
    }

    /// Returns the constant y branch impedance matrix.
    ///
    /// The row and column indices correspond to branch indices. Off diagonal elements can occur e.g. due to magnetic
    /// coupling in lines for example. In a line, y_const is the phase impedance matrix, as distinct from the nodal
    /// impedance matrix. For a three-phase line, the former is a 3x3 matrix, and the latter is a 6x6 matrix.
    pub fn y_const(&self) -> CooMat<c64> {
        if self.is_live() {self.spec.y_const()} else {vec![]}
    }

    /// Returns the constant I component, where the indices are branch indices and the value is the constant current
    /// component.
    pub fn i_const(&self) -> Vec<(usize, c64)> {
        if self.is_live() {self.spec.i_const()} else {vec![]}
    }

    /// Returns the constant S component, where the indices are branch indices and the value is the constant S
    /// component.
    pub fn s_const(&self) -> Vec<(usize, c64)> {
        if self.is_live() {self.spec.s_const()} else {vec![]}
    }

    /// Returns the constant voltage magnitude, where the indices are branch indices and the values are the voltage
    /// magnitudes.
    pub fn vmag_const(&self) -> Vec<(usize, f64)> {
        if self.is_live() {self.spec.vmag_const()} else {vec![]}
    }

    /// Returns the constant voltage angle, where the indices are branch indices and the values are the voltage angles.
    pub fn vang_const(&self) -> Vec<(usize, f64)> {
        if self.is_live() {self.spec.vang_const()} else {vec![]}
    }

    /// Returns indices of copper-plated branches, where the terminals of each such branch are shorted together.
    pub fn copper_plate(&self) -> Vec<Vec<usize>> {
        if self.is_live() {self.spec.copper_plate()} else {vec![]}
    }

    /// Returns the indices of grounded terminals.
    pub fn grounding(&self) -> Vec<usize> {
        if self.is_live() {self.spec.grounding()} else {vec![]}
    }

    /// Writes the fields common to every element into an open struct serializer.
    fn serialize_common_fields<T: SerializeStruct>(&self, state: &mut T) -> Result<(), T::Error> {
        state.serialize_field("uid", &self.common.uid)?;
        state.serialize_field("element_type", &self.element_type())?;
        state.serialize_field("is_in_service", &self.is_in_service())?;
        state.serialize_field("bus_node_ids", &self.bus_node_ids())?;
        state.serialize_field("n_terms", &self.n_terms())?;
        state.serialize_field("branches", &self.branches())?;
        state.serialize_field("branch_wiring", &self.branch_wiring())?;
        state.serialize_field("y_const", &self.y_const())?;
        state.serialize_field("i_const", &self.i_const())?;
        state.serialize_field("s_const", &self.s_const())?;
        state.serialize_field("vmag_const", &self.vmag_const())?;
        state.serialize_field("vang_const", &self.vang_const())?;
        state.serialize_field("copper_plate", &self.copper_plate())?;
        state.serialize_field("grounding", &self.grounding())?;
        Ok(())
    }
}

impl<'a, S: ElementSpec + ?Sized> ElementBorrowedMut<'a, S> {
    /// Returns the in_service status of the element.
    pub fn set_is_in_service(&mut self, is_in_service: bool) {
        self.common.is_in_service = is_in_service
    }

    /// Returns a reference to the [`ElemBranch`]es of the element.
    pub fn branches_mut(&mut self) -> &mut [ElemBranch] {
        &mut self.common.branches
    }
}

impl<C, P> Deref for ElementGuard<C, P>
where
    P: Deref,
{
    type Target = P::Target;
    fn deref(&self) -> &P::Target { &self.spec }
}

impl<C, P> DerefMut for ElementGuard<C, P>
where
    P: DerefMut,
{
    fn deref_mut(&mut self) -> &mut P::Target { &mut self.spec }
}

impl<'a> GSerialize for ElementBorrowed<'a> {
    fn g_serialize<T>(&self, serializer: T) -> Result<T::Ok, T::Error> where T: serde::Serializer {
        let mut state = serializer.serialize_struct("Element", 15)?;
        self.serialize_common_fields(&mut state)?;
        state.end()
    }
}

impl<'a> Display for ElementBorrowed<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.g_format(f)
    }
}

impl<'a, S: ElementSpec + GSerialize> GSerialize for ElementBorrowed<'a, S> {
    fn g_serialize<T>(&self, serializer: T) -> Result<T::Ok, T::Error> where T: serde::Serializer {
        let mut state = serializer.serialize_struct("Element", 15)?;
        self.serialize_common_fields(&mut state)?;
        state.serialize_field("spec", &Wrap(&*self.spec))?;
        state.end()
    }
}

impl<'a, S: ElementSpec + GSerialize> Display for ElementBorrowed<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.g_format(f)
    }
}

impl GSerialize for Element {
    fn g_serialize<T: serde::Serializer>(&self, serializer: T) -> Result<T::Ok, T::Error> {
        self.borrow().g_serialize(serializer)
    }
}

impl<S: ElementSpec + GSerialize> GSerialize for Element<S> {
    fn g_serialize<T: serde::Serializer>(&self, serializer: T) -> Result<T::Ok, T::Error> {
        self.borrow().g_serialize(serializer)
    }
}

/// An electricity network.
///
/// Modelled as a list of [`Bus`]es and a list of [`Element`]s of differing types. The elements provide various types
/// of connections (e.g. constant admittance, constant voltage magnitude) between the phases of the buses.
#[derive(Debug, Serialize)]
pub struct Network {
    /// Master list of buses.
    #[serde(serialize_with="serialize_with_wrap")]
    buses: IndexMap<String, Shared<Bus>>,

    /// Master list of elements.
    #[serde(serialize_with="serialize_with_wrap")]
    elements: IndexMap<String, Element>,
}

impl Network {
    /// Creates an new empty [`Network`].
    pub fn new() -> Network {
        Network {
            buses: IndexMap::new(),
            elements: IndexMap::new(),
        }
    }

    /// Returns the number of [`Bus`]es in the network.
    pub fn n_buses(&self) -> usize {
        self.buses.len()
    }

    /// Returns the [`Bus`]es as an iterator.
    pub fn buses(&self) -> impl Iterator<Item = &Shared<Bus>> {
        self.buses.values()
    }

    /// Executes `f` for each bus in the network.
    pub fn foreach_bus<F>(&self, mut f: F) where F: FnMut(&Bus) {
        for b in self.buses.values() {f(&b.borrow());}
    }

    /// Executes `f` for each bus in the network, where `f` may mutate the buses.
    pub fn foreach_bus_mut<F>(&self, mut f: F) where F: FnMut(&mut Bus) {
        for b in self.buses.values() {f(&mut b.borrow_mut());}
    }

    /// Returns the bus with the given uid.
    pub fn bus(&self, bus_uid: &str) -> Option<&Shared<Bus>> {
        self.buses.get(&bus_uid.to_string())
    }

    /// Returns the bus at the given index.
    pub fn bus_i(&self, bus_idx: usize) -> Option<&Shared<Bus>> {
        self.buses.get_index(bus_idx).map(|x| x.1)
    }

    /// Returns the number of elements in the network.
    pub fn n_elements(&self) -> usize {
        self.elements.len()
    }

    /// Returns the elements in the network as an iterator.
    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.elements.values()
    }

    /// Returns the elements whose spec is of type `S`, as typed handles.
    pub fn elements_of_type<S: ElementSpec>(&self) -> impl Iterator<Item = Element<S>> + '_ {
        self.elements().filter_map(|e| e.downcast::<S>())
    }

    /// Retrns the element with the given uid.
    pub fn element(&self, elem_uid: &str) -> Option<&Element> {
        self.elements.get(&elem_uid.to_string())
    }

    /// Returns the element with the given index.
    pub fn element_i(&self, elem_idx: usize) -> Option<&Element> {
        self.elements.get_index(elem_idx).map(|x| x.1)
    }

    /// Adds a bus to the network.
    pub fn add_bus(&mut self, bus: Bus) -> Result<Shared<Bus>, String> {
        let existing = self.buses.entry(bus.uid.to_string());
        match existing {
            Entry::Occupied(_entry) => {
                Err(format!("Bus {} already exists", bus.uid))
            },
            Entry::Vacant(entry) => {
                let bus = entry.insert(bus.into_shared());
                Ok(bus.clone())
            }
        }
    }

    /// Adds an element to the network.
    pub fn add_element<S: ElementSpec>(&mut self, element: Element<S>) -> Result<Element<S>, String> {
        let existing = self.elements.entry(element.borrow().uid().to_string());
        match existing {
            Entry::Occupied(_entry) => {
                Err(format!("Element {} already exists", element.borrow().uid()))
            },
            Entry::Vacant(entry) => {
                entry.insert(element.erase());
                Ok(element.clone())
            }
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

impl_g_serialize!(Network);
impl_disp_from_fmt!(Network);

#[cfg(test)]
mod tests {
    use super::*;

    impl<T: ElementSpecParam> TypeStrOf for Foo<T> {
        fn type_str_of() -> String where Self: Sized {
            std::any::type_name::<T>().to_string()
        }
    }

    impl<T: ElementSpecParam> ElementSpec for Foo<T> {
        fn n_terms(&self) -> usize {
            2
        }

        fn n_branches(&self) -> usize {
            1
        }

        fn branch_wiring(&self) -> Vec<(usize, usize)> {
            vec![(0, 1)]
        }
    }

    impl<T: ElementSpecParam> Foo<T> {
        pub fn into_element(self, uid: &str, bus_node_ids: Vec<BusNodeId>) -> Element<Self> {
            Element::<Self>::new(uid.to_string(), bus_node_ids, self)
        }
    }

    #[test]
    fn downcast_roundtrip() {
        let e = Foo::<i32> { t: 7 }.into_element(
            "foo",
            vec![bn!("bus_1", Phase::A), bn!("bus_1", Phase::B)],
        );
        assert!(e.borrow().spec.t == 7);
        let e: Element = e.erase();
        assert!(e.is::<Foo<i32>>());
        assert!(e.downcast::<Foo<i32>>().is_some());
        assert!(e.downcast::<Foo<f64>>().is_none());
    }
}
