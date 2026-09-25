//! Network modelling.
//!
//! [`Network`]s comprise a list of [`Bus`]es and a list of [`Element`]s.
//!
//! Buses are nodes of the network, and consist of one or more blobs or bars of conductor with negligible resistance,
//! the bus phases. Each [`Bus`] object has one or more [`BusPhase`]s. [`BusPhase`]s contain a [`Phase`] label and a
//! voltage. Each [`BusPhase`] is uniquely labelled by a [`BusPhaseId`], recording the bus's UID and the phase.
//!
//! Buses are connected together by elements, which provide conductive pathways (termed *branches* in griphos
//! terminology) between bus phases. Each [`Element`] has a given number of *terminals* (connection points), and is
//! connected to the [`Network`] by assigning to each terminal a particular [`BusPhaseId`]. Each of the element's
//! branches then provides a conductive pathway between a start terminal and an end terminal.
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
//! The example below shows construction of a new network, and the creation and assignment to the network of several
//! buses and elements. It will show details of how various elements are constructed and joined to the network.
//! ```
//! # use griphos::{bp, Bus, BusPhaseId, c, c64, Generator, GeneratorControl, Grounding, Line, Phase, Network};
//! # fn main() {
//! // Create a new, empty network.
//! let mut netw = Network::new();
//!
//! // Add a neutral bus, with phase N and base voltage 1.
//! let bus_n = netw.add_bus(Bus::new("bus_ground", vec![Phase::N], 1.0)).unwrap();
//!
//! // Add two three-phase buses, with phases ABC and base voltage 1.
//! let bus_abc_1 = netw.add_bus(Bus::new("bus_abc_1", vec![Phase::A, Phase::B, Phase::C], 1.0)).unwrap();
//! let bus_abc_2 = netw.add_bus(Bus::new("bus_abc_2", vec![Phase::A, Phase::B, Phase::C], 1.0)).unwrap();
//!
//! // Ground the neutral bus, using a `Grounding` element. Note that each element type will normally have its own
//! // `element(...)` member function to conveniently connect it to the network. The signatures of these `element`
//! // functions will typically vary depending on the details of the element type.
//! netw.add_element(
//! Grounding::new(1)                           // Create a `Grounding` object with 1 terminal
//!     .element(                               // Wrap it in it's own element...
//!         "grounding",                        // with UID "grounding"...
//!         vec![bp!("bus_ground", Phase::N)]   // and connect to phase N of bus "bus_ground".
//!     )).unwrap();
//!
//! // Add a wye-connected generator on bus_abc_1
//! netw.add_element(
//!     Generator::new(
//!         4, // 4 terminals: 3 phases + neutral.
//!         vec![(0, 3), (1, 3), (2, 3)], // Wye-connected branches, with neutral last.
//!         GeneratorControl::new_wye_slack_with_std_phase_angles(
//!             1.0, // Setpoint voltage = 1.0.
//!             &[Phase::A, Phase::B, Phase::C]), // Phase angles calculated based on these live phases.
//!     ).element(
//!         "generator", // The UID of the element.
//!         vec![
//!             bp!("bus_abc_1", Phase::A),
//!             bp!("bus_abc_1", Phase::B),
//!             bp!("bus_abc_1", Phase::C),
//!             bp!("bus_abc_1", Phase::N), // Neutral comes last
//!         ]
//!     )).unwrap();
//!
//! // Add a line between the two live buses.
//! netw.add_element(
//!     Line::new_from_sym_seq(c!(1.0, 0.0), c!(1.0, 0.0), 3) // z = 1.0+0j, z0 = 1.0+0j, n_phase = 3
//!         .unwrap()
//!         .element(
//!             "line",
//!             vec![bp!("bus_abc_1", Phase::A), bp!("bus_abc_1", Phase::B), bp!("bus_abc_1", Phase::C)],
//!             vec![bp!("bus_abc_2", Phase::A), bp!("bus_abc_2", Phase::B), bp!("bus_abc_2", Phase::C)],
//!         )).unwrap();
//! # }
//! ```

use std::any::Any;
use std::collections::{BTreeSet, HashSet, HashMap};
use std::fmt;
use std::hash::{Hash};
use std::ops::{Add, MulAssign};
use std::vec::Vec;

use indexmap::map::Entry;
use indexmap::{IndexMap};

use petgraph::graph::UnGraph;
use petgraph::prelude::{NodeIndex};
use petgraph::algo::kosaraju_scc;
use petgraph::visit::EdgeRef;

use serde::ser::SerializeStruct;
use serde::Serialize;

use crate::foundation::*;
use crate::graph::*;

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

/// Line or bus terminal phase, A, B, C or N.
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
pub fn wye_connection(n_phase: usize) -> Result<Vec<(usize, usize)>, String> {
    match n_phase {
        1 => Ok(vec![(0, 1)]),
        2 => Ok(vec![(0, 2), (1, 2)]),
        3 => Ok(vec![(0, 3), (1, 3), (2, 3)]),
        _ => Err(String::from("a wye connection must have 1, 2 or 3 phases"))
    }
}

/// Returns terminal connections for a standard wye connection. Neutral is always considered to come last by convention.
pub fn delta_connection(n_phase: usize) -> Result<Vec<(usize, usize)>, String> {
    match n_phase {
        1 => Ok(vec![(0, 1)]),
        3 => Ok(vec![(0, 1), (1, 2), (2, 0)]),
        _ => Err(String::from("a delta connection must have 1 or 3 phases"))
    }
}

/// A bus UID / phase pair, that together serve to identify which phase of which bus a terminal is connected to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BusPhaseId {
    /// Bus uid.
    pub b: String,
    /// Bus phase.
    pub p: Phase
}

impl_g_serialize!(BusPhaseId);
impl fmt::Display for BusPhaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Remove quotation marks
        write!(f, "{}:{}", self.b, self.p)
    }
}

/// Macro to create a [`BusPhaseId`]
///
/// # Examples
///
/// ```
/// # use griphos::{bp, BusPhaseId, Phase};
/// let bus_phase = bp!("bus_1", Phase::A);
/// ```
#[macro_export]
macro_rules! bp {
    ($b:expr, $p:expr) => {
        BusPhaseId {b: $b.to_string(), p: $p}
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

/// Trait for objects that have a uid (unique ID)
pub trait HasUid {
    /// UID of an object.
    fn uid(&self) -> String;
}

/// Per-phase data of a bus.
#[derive(Clone, Debug, Serialize)]
pub struct BusPhase {
    /// The phase
    pub phase: Phase,
    /// Voltage
    pub v: c64,
}

impl_g_serialize!(BusPhase);
impl_disp_from_fmt!(BusPhase);

/// A bus is a collection of one or more nodes, each distinguished by a different phase.
#[derive(Clone, Debug, Serialize)]
pub struct Bus {
    /// Unique ID
    pub uid: String,

    /// Base voltage in chosen units.
    pub v_base: f64,

    /// Terminals
    pub terms: Vec<BusPhase>,
}

impl Bus {
    /// Creates a new [`Bus`].
    pub fn new(uid: &str, phases: Vec<Phase>, v_base: f64) -> Bus {
        let terms: Vec<BusPhase> = phases.iter()
            .map(|phase| BusPhase {phase: *phase, v: v_base * phase.v_nom_pu()}).collect();
        Bus {uid: uid.to_string(), v_base, terms}
    }

    /// Searches for the terminal of a bus that has phase `p`.
    pub fn bus_phase(&self, p: Phase) -> Option<&BusPhase> {
        self.terms.iter().find(|x| x.phase == p)
    }

    /// Searches for the terminal of a bus that has phase `p`, mutable version.
    pub fn bus_phase_mut(&mut self, p: Phase) -> Option<&mut BusPhase> {
        self.terms.iter_mut().find(|x| x.phase == p)
    }

    /// Searches for the terminal index of a bus that has phase `p`.
    pub fn i_bus_phase(&self, p: Phase) -> Option<usize> {
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

impl HasUid for Bus {fn uid(&self) -> String {self.uid.clone()}}

impl_g_serialize!(Bus);
impl_disp_from_fmt!(Bus);

/// Per-branch data of an element
#[derive(Clone, Debug, Serialize)]
pub struct ElemBranch {
    /// Branch voltage. For branch (i, j), v is defined v_i - v_j.
    pub v: c64,
    /// Branch current. For branch (i, j), i has directionality i -> j.
    pub i: c64,
}

impl_g_serialize!(ElemBranch);
impl_disp_from_fmt!(ElemBranch);

/// An element is a set of current carrying connections netween terminals. It can include several different types of
/// connection, such as constant y or constant s.
#[derive(std::fmt::Debug)]
pub struct Element {
    /// Unique ID
    pub uid: String,
    /// Is the element in service / operational.
    pub is_in_service: bool,
    /// Key of which bus/phases the terminals are connected to.
    pub bus_phase_ids: Vec<BusPhaseId>,
    /// The branch info, a vector of [`ElemBranch`] of the element.
    pub branches: Vec<ElemBranch>,

    // The specialized behaviour of the element, an object that implements ElementSpec.
    spec: Box<dyn ElementSpec>,
}

impl Element {
    /// Creates a new element.
    pub fn new<T: ElementSpec + 'static>(uid: &str, bus_phase_ids: Vec<BusPhaseId>, spec: T) -> Element {
        let n_branches = spec.n_branches();
        Element {
            uid: uid.to_string(),
            is_in_service: true,
            bus_phase_ids,
            spec: Box::new(spec),
            branches: vec![ElemBranch {v: c!(0.0, 0.0), i: c!(0.0, 0.0)}; n_branches],
        }
    }

    /// Type string of an element, corresponding to the dynamic type of its `spec`.
    pub fn element_type(&self) -> &str {
        self.spec.type_str()
    }

    /// Downcasts an element's `spec` to the given type, which must implement [`ElementSpec`].
    pub fn as_type<T: 'static>(&self) -> Option<&T> {
        self.spec.downcast_ref::<T>()
    }

    /// Downcasts an element's `spec` to the given type, which must implement [`ElementSpec`], mutable version.
    pub fn as_type_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.spec.downcast_mut::<T>()
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
        self.is_in_service && self.spec.is_live()
    }

    /// Returns the constant y branch impedance matrix.
    ///
    /// The row and column indices correspond to branch indices. Off diagonal elements can occur e.g. due to magnetic
    /// coupling in lines for example. In a line, y_const is the phase impedance matrix, as distinct from the nodal
    /// impedance matrix. For a three-phase line, the former is a 3x3 matrix, and the latter is a 6x6 matrix.
    pub fn y_const(&self) -> Vec<((usize, usize), c64)> {
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
}

impl HasUid for Element {fn uid(&self) -> String {self.uid.clone()}}

impl GSerialize for Element {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        let mut state = serializer.serialize_struct("Element", 4)?;
        state.serialize_field("uid", &self.uid)?;
        state.serialize_field("element_type", &self.element_type())?;
        state.serialize_field("is_in_service", &self.is_in_service)?;
        state.serialize_field("bus_phase_ids", &self.bus_phase_ids)?;
        state.serialize_field("n_terms", &self.spec.n_terms())?;
        state.serialize_field("branch_wiring", &self.branch_wiring())?;
        state.serialize_field("y_const", &self.spec.y_const())?;
        state.serialize_field("i_const", &self.spec.i_const())?;
        state.serialize_field("s_const", &self.spec.s_const())?;
        state.serialize_field("vmag_const", &self.spec.vmag_const())?;
        state.serialize_field("vang_const", &self.spec.vang_const())?;
        state.serialize_field("copper_plate", &self.spec.copper_plate())?;
        state.serialize_field("grounding", &self.spec.grounding())?;
        state.end()
    }
}

impl_disp_from_fmt!(Element);

/// Trait for types that have an associated type_str string. Each ElementSpec must implement this by hand.
pub trait TypeStrOf {
    fn type_str_of() -> &'static str where Self: Sized;
}

/// Bridge to allow calling on an instance; don't re-implement, machinery to allow dynamic type strings.
pub trait TypeStr {
    fn type_str(&self) -> &'static str;
}

/// Blanket implementation of TypeStr for all types, machinery to allow dynamic type strings.
impl<T: TypeStrOf> TypeStr for T {
    fn type_str(&self) -> &'static str {
        T::type_str_of()
    }
}

/// An element_spec collects together electrical connections of various types such as constant y or constant s.
pub trait ElementSpec: Any + std::fmt::Debug + Send + Sync + TypeStrOf + TypeStr {
    fn element_generic(self, uid: &str, bus_phase_ids: Vec<BusPhaseId>) -> Element where Self: Sized + 'static {
        Element::new(uid, bus_phase_ids, self)
    }

    /// Returns the number of terminals (connection points).
    fn n_terms(&self) -> usize {0}

    /// Returns the number of branches (node-node connections)
    fn n_branches(&self) -> usize {0}

    /// Branch wiring, term_idx_i -> term_idx_j.
    fn branch_wiring(&self) -> Vec<(usize, usize)> {vec![]}

    /// Is the element live (e.g. switched on)?
    fn is_live(&self) -> bool {true}

    /// Branch impedance matrix; I_i = y_ij V_j where i and j index branches.
    fn y_const(&self) -> Vec<((usize, usize), c64)> {vec![]}

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
    /// Downcasts an [`ElementSpec`] to type `T`.
    fn downcast_ref<T: Any>(&self) -> Option<&T> {
        let any: &dyn Any = self;
        any.downcast_ref::<T>()
    }

    /// Mutably downcasts an [`ElementSpec`] to type `T`.
    fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        let any: &mut dyn Any = self;
        any.downcast_mut::<T>()
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
    elements: IndexMap<String, Shared<Element>>,
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
    pub fn elements(&self) -> impl Iterator<Item = &Shared<Element>> {
        self.elements.values()
    }

    /// Returns the elements of type `T` in the network as an iterator.
    pub fn elements_of_type<T: ElementSpec + 'static>(&self) -> impl Iterator<Item = &Shared<Element>> + '_ {
        self.elements().filter(|elem| elem.borrow().as_type::<T>().is_some())
    }

    /// Executes `f` for each element in the network.
    pub fn foreach_element<F>(&self, mut f: F) where F: FnMut(&Element) {
        for e in self.elements.values() {f(&e.borrow());}
    }

    /// Executes `f` for each element in the network, where `f` may mutate the elements.
    pub fn foreach_element_mut<F>(&self, mut f: F) where F: FnMut(&mut Element) {
        for e in self.elements.values() {f(&mut e.borrow_mut());}
    }

    /// Executes `f` for each element of type `T` in the network.
    pub fn foreach_element_of_type<F, T>(&self, mut f: F) where F: FnMut(&Element), T: ElementSpec {
        for e in self.elements.values().filter(|e| e.borrow().as_type::<T>().is_some()) {
            f(&e.borrow());
        }
    }

    /// Executes `f` for each element of type `T` in the network, where `f` may mutate the elements.
    pub fn foreach_element_of_type_mut<F, T>(&self, mut f: F) where F: FnMut(&mut Element), T: ElementSpec {
        for e in self.elements.values().filter(|e| e.borrow().as_type::<T>().is_some()) {
            f(&mut e.borrow_mut());
        }
    }

    /// Retrns the element with the given uid.
    pub fn element(&self, elem_uid: &str) -> Option<&Shared<Element>> {
        self.elements.get(&elem_uid.to_string())
    }

    /// Returns the element with the given index.
    pub fn element_i(&self, elem_idx: usize) -> Option<&Shared<Element>> {
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
    pub fn add_element(&mut self, element: Element) -> Result<Shared<Element>, String> {
        let existing = self.elements.entry(element.uid.to_string());
        match existing {
            Entry::Occupied(_entry) => {
                Err(format!("Element {} already exists", element.uid))
            },
            Entry::Vacant(entry) => {
                let element = entry.insert(element.into_shared());
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

/// A node is a single phase of a bus, to be used in a NetworkModel. It has a complex voltage.
#[derive(Clone, Debug, Serialize)]
pub struct Node {
    /// Nominal voltage
    pub v_nom: c64,

    /// Is the node grounded?
    pub is_grounded: bool,

    /// There can be more than one BusPhaseId due to the possibility of copperplating.
    pub bus_phase_ids: Vec<BusPhaseId>,

    /// Voltage, in chosen units, if known.
    pub v: Option<c64>,
}

impl Node {
    /// Creates a new [`Node`].
    fn new(v_nom: c64, is_grounded: bool) -> Node {
        Node {v_nom, is_grounded, bus_phase_ids: vec![], v: None}
    }

    /// Display name for the node.
    pub fn name(&self) -> String {
        self.bus_phase_ids.iter().map(|bp| bp.to_string()).collect::<Vec<_>>().join("|")
    }
}

impl_g_serialize!(Node);
impl_disp_from_fmt!(Node);

/// A branch is an ordered connection between two nodes. It has a current and a voltage.
#[derive(Clone, Debug, Serialize)]
pub struct Branch {
    /// Node 0 index
    pub nd_0_idx: usize,

    /// Node 1 index
    pub nd_1_idx: usize,

    /// Nominal voltage, based on the nodes.
    pub v_nom: c64,

    /// Voltage v_01 = v0 - v1
    pub v_01: Option<c64>,

    /// Current i->j
    pub i_01: Option<c64>,
}

impl Branch {
    /// Creates a new branch.
    fn new(nd_0_idx: usize, nd_1_idx: usize, v_nom: c64) -> Branch {
        Branch {nd_0_idx, nd_1_idx, v_nom, v_01: None, i_01: None}
    }
}

impl_g_serialize!(Branch);
impl_disp_from_fmt!(Branch);

/// # [`NetworkModel`]s
///
/// A [`NetworkModel`] is a lower level representation of the network, which breaks apart the grouping into [`Bus`]es
/// with phases and [`Element`]s with branches, and instead works with [`Node`]s (each corresponding to a single phase
/// of a bus) and [`Branch`]es (each corresponding to a single connection between two nodes).
#[derive(Debug)]
pub struct NetworkModel {
    // Distinct nodes (wrt copperplating), keys are uid
    pub nodes: Vec<Node>,

    // Branches, keys are uid
    pub branches: Vec<Branch>,

    // BusPhaseId to Node map
    pub bp_to_nd_idx: HashMap<BusPhaseId, usize>,

    // ElemTerm to Branch map
    pub eb_to_br_idx: HashMap<ElemBranchId, usize>,

    // Structure graph: graph nodes are network nodes, graph edges are branches.
    // Labels are master node index, master branch index.
    pub structure_graph: UnGraph<(), ()>,

    pub y_const: CsMat<c64>, // Indices are branch indices (not UIDs) in this NetworkModel's branches
    pub i_const: CsVec<c64>, // Indices are branch indices (not UIDs) in this NetworkModel's branches
    pub s_const: CsVec<c64>, // Indices are branch indices (not UIDs) in this NetworkModel's branches
    pub vmag_const: CsVec<f64>, // Indices are branch indices (not UIDs) in this NetworkModel's branches
    pub vang_const: CsVec<f64>, // Indices are branch indices (not UIDs) in this NetworkModel's branches
}

impl Default for NetworkModel {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkModel {
    /// Creates an netw empty NetworkModel.
    pub fn new() -> NetworkModel {
        NetworkModel {
            nodes: vec![],
            branches: vec![],
            bp_to_nd_idx: HashMap::new(),
            eb_to_br_idx: HashMap::new(),
            structure_graph: UnGraph::new_undirected(),
            y_const: CsMat::empty(sprs::CompressedStorage::CSC, 0),
            i_const: CsVec::empty(0),
            s_const: CsVec::empty(0),
            vmag_const: CsVec::empty(0),
            vang_const: CsVec::empty(0),
        }
    }

    /// Creates an netw master NetworkModel from a [`Network`].
    ///
    /// The master model contains all nodes and branches in the network, as distinct from island models, which split the
    /// network into electrically isolated islands.
    pub fn new_master(netw: &Network) -> Result<NetworkModel, String> {
        let mut nmod = NetworkModel::new();
        nmod.make_nodes(netw)?;
        nmod.make_branches(netw)?;
        nmod.make_structure_graph()?;

        nmod.y_const = nmod.elem_sparse_mat(netw, |x| x.y_const())?;
        nmod.i_const = nmod.elem_sparse_vec(netw, |x| x.i_const())?;
        nmod.s_const = nmod.elem_sparse_vec(netw, |x| x.s_const())?;
        nmod.vmag_const = nmod.elem_sparse_vec(netw, |x| x.vmag_const())?;
        nmod.vang_const = nmod.elem_sparse_vec(netw, |x| x.vang_const())?;

        Ok(nmod)
    }

    /// Splits a master NetworkModel into islands.
    pub fn islands(&self) -> Result<Vec<NetworkModel>, String> {
        // Ensure that transformers don't separate out two components that should be together.
        // We do this by adding extra edges to the structure.

        let mut sg_plus = self.structure_graph.clone();

        for (_, (ibr, jbr)) in self.y_const.iter() {
            let n_bri_0 = self.branches[ibr].nd_0_idx;
            let n_bri_1 = self.branches[ibr].nd_1_idx;
            let n_brj_0 = self.branches[jbr].nd_0_idx;
            let n_brj_1 = self.branches[jbr].nd_1_idx;
            if [n_bri_0, n_bri_1, n_brj_0, n_brj_1].into_iter().collect::<HashSet<usize>>().len() == 4 {
                sg_plus.add_edge(NodeIndex::new(n_bri_0), NodeIndex::new(n_brj_1), ());
            }
        }

        let ccs = kosaraju_scc(&sg_plus);

        let mut nd_idx_to_island: HashMap<usize, usize> = HashMap::new();
        let mut nd_idxs_of_island: Vec<BTreeSet<usize>> = (0..ccs.len()).map(|_| BTreeSet::new()).collect();
        for (island_idx, cci) in ccs.iter().enumerate() {
            for nd_idx in cci {
                nd_idx_to_island.insert(nd_idx.index(), island_idx);
                nd_idxs_of_island[island_idx].insert(nd_idx.index());
            }
        }

        let mut br_idxs_of_island: Vec<BTreeSet<usize>> = (0..ccs.len()).map(|_| BTreeSet::new()).collect();
        for (br_idx, br) in self.branches.iter().enumerate() {
            let island_idx = nd_idx_to_island[&br.nd_0_idx];
            br_idxs_of_island[island_idx].insert(br_idx);
        }

        let nd_idxs_of_island: Vec<Vec<usize>> = nd_idxs_of_island.into_iter()
            .map(|btreeset| btreeset.into_iter().collect()).collect();

        let br_idxs_of_island: Vec<Vec<usize>> = br_idxs_of_island.into_iter()
            .map(|btreeset| btreeset.into_iter().collect()).collect();

        let mut island_models: Vec<NetworkModel> = Vec::with_capacity(nd_idxs_of_island.len());

        for (island_nd_master_idxs, island_br_master_idxs) in nd_idxs_of_island.iter().zip(br_idxs_of_island) {
            let n_branches = island_br_master_idxs.len();

            // Map master -> island indices for nodes and branches.
            let nd_map: HashMap<usize, usize> = island_nd_master_idxs.iter().enumerate()
                .map(|(iisland, &imaster)| (imaster, iisland)).collect();
            let br_map: HashMap<usize, usize> = island_br_master_idxs.iter().enumerate()
                .map(|(iisland, &imaster)| (imaster, iisland)).collect();

            let nd_idx_set: HashSet<usize> = island_nd_master_idxs.iter().copied().collect();
            let br_idx_set: HashSet<usize> = island_br_master_idxs.iter().copied().collect();

            let bp_to_nd_idx = self.bp_to_nd_idx.iter()
                .filter(|(_, nd_idx)| nd_idx_set.contains(*nd_idx))
                .map(|(bp, nd_idx)| (bp.clone(), *nd_idx)).collect();

            let eb_to_br_idx = self.eb_to_br_idx.iter()
                .filter(|(_, br_idx)| br_idx_set.contains(*br_idx))
                .map(|(eb, br_idx)| (eb.clone(), *br_idx)).collect();

            let island_nodes = island_nd_master_idxs.iter()
                .map(|&nd_idx_master| self.nodes[nd_idx_master].clone())
                .collect::<Vec<Node>>();

            let island_branches = island_br_master_idxs.iter()
                .map(|&br_idx_master| self.branches[br_idx_master].clone())
                .collect::<Vec<Branch>>();

            let mut island_structure_graph: UnGraph<(), ()> = UnGraph::new_undirected();

            island_nd_master_idxs.iter().for_each(|_| {island_structure_graph.add_node(());});

            for e in self.structure_graph.edge_references() {
                let nd_idx_master_src = e.source().index();
                let nd_idx_master_trg = e.target().index();
                if let Some(&nd_idx_island_src) = nd_map.get(&nd_idx_master_src)
                && let Some(&nd_idx_island_trg) = nd_map.get(&nd_idx_master_trg) {
                    island_structure_graph.add_edge(
                        NodeIndex::new(nd_idx_island_src), NodeIndex::new(nd_idx_island_trg), ()
                    );
                }
            }

            let y_const = map_spmat(&self.y_const, n_branches, &br_map);
            let i_const = map_spvec(&self.i_const, n_branches, &br_map);
            let s_const = map_spvec(&self.s_const, n_branches, &br_map);
            let vmag_const = map_spvec(&self.vmag_const, n_branches, &br_map);
            let vang_const = map_spvec(&self.vang_const, n_branches, &br_map);

            island_models.push(NetworkModel {
                nodes: island_nodes,
                branches: island_branches,
                bp_to_nd_idx,
                eb_to_br_idx,
                y_const,
                i_const,
                s_const,
                vmag_const,
                vang_const,
                structure_graph: island_structure_graph,
            });
        }
        Ok(island_models)
    }

    #[allow(clippy::type_complexity)]
    fn make_nodes(&mut self, netw: &Network) -> Result<(), String> {
        // We want to coalesce any bus/phase pairs that are connected by copperplate connections
        // into a single node.

        // Create a graph of copper plate connected bus/phase pairs
        let mut copper = UnGraph::<BusPhaseId, ()>::new_undirected();

        // Insert all bus/phase pairs as graph nodes while noting the mapping between bus/phase and
        // node index.
        let mut bp_to_gnode = HashMap::<BusPhaseId, NodeIndex>::new();
        for (_, bus) in netw.buses.iter() {
            let bus = bus.borrow();
            for t in bus.terms.iter() {
                let bp = bp!(&bus.uid(), t.phase);
                if !bp_to_gnode.contains_key(&bp) {
                    let idx = copper.add_node(bp.clone());
                    bp_to_gnode.insert(bp.clone(), idx);
                }
            }
        }

        // Insert copper plate connections as edges.
        for (_k, e) in netw.elements.iter() {
            let e = e.borrow();
            let cps = e.copper_plate();
            for cp in cps {
                let gnodei = bp_to_gnode[&e.bus_phase_ids[cp[0]]];
                for j in &cp[1..] {
                    let gnodej = bp_to_gnode[&e.bus_phase_ids[cp[*j]]];
                    copper.update_edge(gnodei, gnodej, ());
                }
            }
        }

        // Copper plate connect all ground nodes to the first ground node. This will have the
        // effect that there is a single ground node.
        let mut first_ground: Option<NodeIndex> = None;
        for (_k, e) in netw.elements.iter() {
            let e = e.borrow();
            let gnd = e.grounding();
            for i in gnd {
                let gnodei = bp_to_gnode[&e.bus_phase_ids[i]];
                if let Some(g0) = first_ground {
                    copper.update_edge(gnodei, g0, ());
                } else {
                    first_ground = Some(gnodei);
                }
            }
        }

        // Find connected components.
        let ccs = kosaraju_scc(&copper);

        // For each connected component, make a single node and use it for all bus/phase pairs in
        // the component.
        let mut nodes: Vec<(usize, Node)> = Vec::with_capacity(ccs.len());

        {
            let mut bp_to_nd_idx: HashMap<BusPhaseId, usize> = HashMap::new();
            for cc in ccs.iter() {
                // Get v_base by looking at the first bus that has this node.
                let BusPhaseId {b: b0, p: p0} = &copper.node_weight(cc[0]).unwrap();
                let v_base = netw.buses[b0].borrow().v_base;
                let v_nom = v_base * p0.v_nom_pu();

                // Create a shared node for this set of copperplated nodes.
                nodes.push((nodes.len(), Node::new(v_nom, false)));
                let (nd_idx, node) = &mut nodes.last_mut().unwrap();

                for &cc_i in cc.iter() {
                    // TODO: Think about what happens if two nodes with different v_base are copperplated.
                    let bp = copper.node_weight(cc_i).unwrap();
                    assert!(bp.p == *p0, "Can't copperplate two bus/phase pairs with different phases");
                    node.bus_phase_ids.push(bp.clone());
                    bp_to_nd_idx.insert(bp.clone(), *nd_idx);
                }
                node.bus_phase_ids.sort();
            }

            // Set grounding flag
            for (_eid, e) in netw.elements.iter() {
                for i in e.borrow().grounding() {
                    let nd = &mut nodes[bp_to_nd_idx[&e.borrow().bus_phase_ids[i]]];
                    nd.1.is_grounded = true;
                    nd.1.v = Some(c!(0.0, 0.0));
                }
            }

            // Run through bus/phases in order and set node indices.
            let mut seen = HashSet::<usize>::new();
            let mut i = 0usize;
            for b in netw.buses.values() {
                for t in b.borrow().terms.iter() {
                    let bp = bp!(b.borrow().uid, t.phase);
                    let (nd_idx, _) = &mut nodes[bp_to_nd_idx[&bp]];
                    if seen.insert(*nd_idx) {
                        *nd_idx = i;
                        i += 1;
                    }
                }
            }

            nodes.sort_by_key(|(i, _)| *i);
        }

        self.nodes = nodes.into_iter().map(|(_, nd)| nd).collect();

        for (nd_idx, nd) in self.nodes.iter().enumerate() {
            for bp in nd.bus_phase_ids.iter() {
                self.bp_to_nd_idx.insert(bp.clone(), nd_idx);
            }
        }

        Ok(())
    }

    fn make_branches(
        &mut self,
        netw: &Network,
    ) -> Result<(), String> {
        for (_, elem) in netw.elements.iter() {
            let elem = elem.borrow();
            for (ibr, (i, j)) in elem.branch_wiring().iter().enumerate() {
                let indi = self.bp_to_nd_idx[&elem.bus_phase_ids[*i]];
                let indj = self.bp_to_nd_idx[&elem.bus_phase_ids[*j]];
                let v_nom = self.nodes[indi].v_nom - self.nodes[indj].v_nom;
                self.eb_to_br_idx.insert(eb!(elem.uid, ibr), self.branches.len());
                self.branches.push(Branch::new(indi, indj, v_nom));
            }
        }
        Ok(())
    }

    fn make_structure_graph(&mut self) -> Result<(), String> {
        for _ in self.nodes.iter() {
            self.structure_graph.add_node(());
        }

        for b in self.branches.iter() {
            let idx_0 = NodeIndex::new(b.nd_0_idx);
            let idx_1 = NodeIndex::new(b.nd_1_idx);
            self.structure_graph.add_edge(idx_0, idx_1, ());
        }

        Ok(())
    }

    fn elem_sparse_vec<F, T>(&self, netw: &Network, f: F) -> Result<CsVec<T>, String>
    where
        F: Fn(&Element) -> Vec<(usize, T)>,
        T: Clone + Add<Output = T> + MulAssign<f64>
    {
        let n_branches = self.branches.len();
        let mut entries: Vec<(usize, T)> = Vec::with_capacity(n_branches);
        netw.foreach_element(|elem| {
            for (i, x) in f(elem) {
                let eb = eb!(&elem.uid, i);
                let idx_br = self.eb_to_br_idx[&eb];
                entries.push((idx_br, x));
            }
        });
        entries.sort_by_key(|(idx, _)| *idx);
        let (indices, data): (Vec<_>, Vec<_>) = entries.into_iter().unzip();
        let vec: CsVec<T> = CsVec::new(n_branches, indices, data);
        Ok(vec)
    }

    fn elem_sparse_mat<F, T>(&self, netw: &Network, f: F) -> Result<CsMat<T>, String>
    where
        F: Fn(&Element) -> Vec<((usize, usize), T)>,
        T: Clone + Add<Output = T>
    {
        let n_branches = self.branches.len();
        let mut tri = TriMat::<T>::new((n_branches, n_branches));
        netw.foreach_element(|elem| {
            for ((i, j), x) in f(elem) {
                let eb_i = eb!(&elem.uid, i);
                let eb_j = eb!(&elem.uid, j);
                let idx_br_i = self.eb_to_br_idx[&eb_i];
                let idx_br_j = self.eb_to_br_idx[&eb_j];
                tri.add_triplet(idx_br_i, idx_br_j, x);
            }
        });
        Ok(tri.to_csc())
    }

    /// Sets the node voltages based on branch voltages.
    pub fn propagate_branch_voltages_to_nodes(&mut self) {
        // TODO: for now we're only using ground to propagate the voltages.
        let mut somes: Vec<usize> = self.nodes.iter().enumerate()
            .filter(|(_, nd)| nd.v.is_some()).map(|(i, _)| i).collect();

        if somes.is_empty() {
            return;
        }

        somes.sort_by(
            |i, j| self.nodes[*i].v.unwrap().norm_sqr().partial_cmp(&self.nodes[*j].v.unwrap().norm_sqr())
                .unwrap_or(std::cmp::Ordering::Equal)
        );

        let mut n_none = self.nodes.len() - somes.len();

        while n_none > 0 {
            let idx_start = &somes[0];
            let idx_start = NodeIndex::new(*idx_start);

            dfs(&self.structure_graph, idx_start, |cur_node_idx, maybe_incoming| -> bool {
                if let Some((prev_node_idx, edge_idx)) = maybe_incoming {
                    // Only update if we're coming from a previous node.
                    let cur_node_v = &self.nodes[cur_node_idx.index()].v;
                    if cur_node_v.is_none() {
                        let prev_node = &self.nodes[prev_node_idx.index()];
                        let branch = &self.branches[edge_idx.index()];
                        let prev_node_v = prev_node.v.unwrap(); // Will never be None
                        if let Some(v_01) = branch.v_01 {
                            let v_cur_prev = {
                                if cur_node_idx.index() == branch.nd_1_idx {-v_01}
                                else {v_01}
                            };
                            self.nodes[cur_node_idx.index()].v = Some(prev_node_v + v_cur_prev);
                            n_none -= 1;
                            true
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                } else {
                    true
                }
            });
        }
    }

    /// Sets the bus voltages based on nodes, and the element voltages and currents based on branches.
    pub fn propagate_to_network(&self, netw: &mut Network) {
        for (bp, nd_idx) in self.bp_to_nd_idx.iter() {
            if let Some(nd_v) = self.nodes[*nd_idx].v {
                let bus = netw.buses.get_mut(&bp.b).unwrap();
                bus.borrow_mut().bus_phase_mut(bp.p).unwrap().v = nd_v;
            }
        }

        for (eb, br_idx) in self.eb_to_br_idx.iter() {
            let br = &self.branches[*br_idx];
            let elem_shared = netw.element(&eb.e).unwrap();
            let mut elem = elem_shared.borrow_mut();
            let elem_br = &mut elem.branches[eb.b];
            if let Some(v) = br.v_01 {
                elem_br.v = v;
            }
            if let Some(i) = br.i_01 {
                elem_br.i = i;
            }
        }
    }
}

/// Convenience struct to store master and island [`NetworkModel`]s.
#[derive(Debug)]
pub struct NetworkModels {
    pub master: NetworkModel,
    pub islands: Vec<NetworkModel>,
}

impl NetworkModels {
    /// Creates a new [`NetworkModels`].
    pub fn new(netw: &Network) -> NetworkModels {
        let master = NetworkModel::new_master(netw).unwrap();
        let islands = master.islands().unwrap();
        NetworkModels {master, islands}
    }
}

#[cfg(test)]
mod tests {
    use crate::e_json::*;
    use crate::generic_element::GenericElement;
    use super::*;

    /// Make a network with optional ground for the first bus and specified phases for the rest.
    fn make_network(add_ground: bool, bus_n_phases: &[usize]) -> Network {
        let mut netw = Network::new();
        if add_ground {
            netw.add_bus(Bus::new("bus_g", vec![Phase::A], 1.0)).unwrap();
        }
        for (i, n_phs) in bus_n_phases.iter().enumerate() {
            let uid = format!("bus_{i}");
            let phs = match n_phs {
                0 => todo!(),
                1 => vec![Phase::A],
                2 => vec![Phase::A, Phase::B],
                3 => vec![Phase::A, Phase::B, Phase::C],
                4.. => todo!(),
            };
            netw.add_bus(Bus::new(&uid, phs, 1.0)).unwrap();
        }

        netw
    }

    #[test]
    fn test_coalesce_nodes() {
        let mut netw = make_network(false, &[1, 1, 1]); // 3 single phase buses.

        let y_const = vec![((0, 0), c!(1.0, 0.0))]; // A single branch constant Y
        let copper_plate = vec![vec![0, 1]]; // Copper plate two nodes together

        netw.add_element(
            (GenericElement {n_terms: 2, branches: vec![(0, 1)], y_const, ..Default::default()})
                .element("y01", vec![bp!("bus_0", Phase::A), bp!("bus_1", Phase::A)])).unwrap();

        netw.add_element(
            (GenericElement {n_terms: 2, copper_plate, ..Default::default()})
                .element("cp12", vec![bp!("bus_1", Phase::A), bp!("bus_2", Phase::A)])).unwrap();

        let mods = NetworkModels::new(&netw);
        assert!(mods.master.nodes.len() == 2);
        assert!(netw.buses["bus_0"].borrow().terms.len() == 1);
        assert!(netw.buses["bus_1"].borrow().terms.len() == 1);
        assert!(netw.buses["bus_2"].borrow().terms.len() == 1);
    }

    // TODO: May have taken the teeth out of some of these tests, review.
    #[test]
    fn test_coalesce_ground() {
        // Four single phase buses, no ground.
        let mut netw = make_network(false, &[1, 1, 1, 1]);

        // Function to connect two buses with a constant y connection
        let y_const = vec![((0, 0), c!(1.0, 0.0))];

        fn bps(i: usize, j: usize) -> Vec<BusPhaseId> {
            vec![
                bp!(&format!("bus_{i}"), Phase::A),
                bp!(&format!("bus_{j}"), Phase::A),
            ]
        }

        // b0-b1g-b2g-b3
        // We expect buses 2 and 3 to get aggregated into a single ground node, thus the indexing
        // should be 0, 1, 1, 2
        netw.add_element(
            (GenericElement {n_terms: 2, branches: vec![(0, 1)], y_const: y_const.clone(), ..Default::default()})
                .element("y01", bps(0, 1))).unwrap();
        netw.add_element(
            (GenericElement {n_terms: 2, branches: vec![(0, 1)], y_const: y_const.clone(), ..Default::default()})
                .element("y12", bps(1, 2))).unwrap();
        netw.add_element(
            (GenericElement {n_terms: 2, grounding: vec![0, 1], ..Default::default()})
            .element("g12", bps(1, 2))).unwrap();
        netw.add_element(
            (GenericElement {n_terms: 2, branches: vec![(0, 1)], y_const, ..Default::default()})
            .element("y23", bps(2, 3))).unwrap();
        let mods = NetworkModels::new(&netw);
        assert!(mods.master.nodes.len() == 3);
    }

    #[test]
    fn test_cycles_and_islands_a() {
        // Setup: create a phase map (adjust based on your actual Phase type)
        let ejson_phase_map = ejson_phase_map_abcng();

        let netw = parse_ejson_file("test_data/netw-i1l1s1.json", &ejson_phase_map)
            .expect("Failed to parse e-JSON");
        let mods = NetworkModels::new(&netw);

        println!("Network:\n{}", &netw);

        println!("{:?}", mods.master.y_const.to_dense());
        println!("{}", graph_dot(&mods.master.structure_graph));
        assert!(mods.islands.len() == 1);
    }

    #[test]
    fn test_cycles_and_islands_b() {
        // Setup: create a phase map (adjust based on your actual Phase type)
        let ejson_phase_map = ejson_phase_map_abcng();

        let netw = parse_ejson_file("test_data/netw-i2l2s2.json", &ejson_phase_map)
            .expect("Failed to parse e-JSON");

        println!("Network:\n{}", Wrap(&netw));

        let mods = NetworkModels::new(&netw);
        println!("{:?}", mods.master.y_const.to_dense());
        println!("{}", graph_dot(&mods.master.structure_graph));
        assert!(mods.islands.len() == 1);
    }

    #[test]
    fn test_cycles_and_islands_c() {
        // Setup: create a phase map (adjust based on your actual Phase type)
        let ejson_phase_map = ejson_phase_map_abcng();

        let netw = parse_ejson_file("test_data/netw-dup-i1l1s1.json", &ejson_phase_map).expect("Failed to parse e-JSON");
        println!("Network:\n{}", Wrap(&netw));

        let mods = NetworkModels::new(&netw);
        println!("{:?}", mods.master.y_const.to_dense());
        println!("{}", graph_dot(&mods.master.structure_graph));
        assert!(mods.islands.len() == 1)
    }
}
