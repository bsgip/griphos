use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use ::griphos as gr;
use gr::{c64, Shared, IntoShared};

use crate::plumbing::{element_subclass, py_wrapper};
use crate::network::{BusNodeId, bus_node_ids_to_gr};

/// Describes various types transformer errors.
#[derive(Debug)]
pub struct TransformerError(gr::TransformerError);

impl From<gr::TransformerError> for TransformerError {
    fn from(e: gr::TransformerError) -> Self { Self(e) }
}

impl From<TransformerError> for PyErr {
    fn from(e: TransformerError) -> PyErr {
        PyValueError::new_err(e.0.to_string())
    }
}

/// Tap data for tapped transformers.
#[pyclass(module = "griphos", unsendable, from_py_object)]
#[gen_stub_pyclass]
#[derive(Clone)]
pub struct TransformerTapsData {
    /// Which side (0 = primary, 1 = secondary, 2 = tertiary (for two-winding transformers) etc.
    pub side: usize,
    /// The factor by which each tap increase increases the winding count of the given winding.
    pub factor: f64,
    /// The minimum possible tap setting.
    pub min_tap: i32,
    /// The maximum possible tap setting.
    pub max_tap: i32,
}

#[gen_stub_pymethods]
#[pymethods]
impl TransformerTapsData {
    #[new]
    pub fn new(side: usize, factor: f64, min_tap: i32, max_tap: i32) -> Self {
        Self { side, factor, min_tap, max_tap}
    }
}

py_wrapper!(
    /// A tapped transformer's tap settins.
    #[derive(Debug)]
    TransformerTaps,
    gr::Element
);

impl TransformerTaps {
    fn with_taps<R>(&self, f: impl FnOnce(&gr::TransformerTaps) -> R) -> R {
        f(self.inner.borrow().as_type::<gr::Transformer>().unwrap().taps.as_ref().unwrap())
    }

    fn with_taps_mut<R>(&self, f: impl FnOnce(&mut gr::TransformerTaps) -> R) -> R {
        f(self.inner.borrow_mut().as_type_mut::<gr::Transformer>().unwrap().taps.as_mut().unwrap())
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl TransformerTaps {
    /// The side of the transformer that is tapped: 0 = primary, 1 = secondary, 3 = tertiary etc.
    #[getter]
    fn side(&self) -> usize {
        self.with_taps(|taps| taps.side)
    }

    /// The factor by which each tap increase increases the winding count of the given winding.
    #[getter]
    fn factor(&self) -> f64 {
        self.with_taps(|taps| taps.factor)
    }

    /// The minimum possible tap setting.
    #[getter]
    fn min_tap(&self) -> i32 {
        self.with_taps(|taps| taps.min_tap)
    }

    /// The maximum possible tap setting.
    #[getter]
    fn max_tap(&self) -> i32 {
        self.with_taps(|taps| taps.max_tap)
    }

    /// Get the tap setting for the tap on phase index i.
    ///
    /// Args:
    ///     i: The phase index of which to return the tap setting.
    /// Returns:
    ///     The tap setting of phase i.
    pub fn get(&self, i: usize) -> i32 {
        self.with_taps(|taps| taps.get(i))
    }

    /// Set the tap setting for the tap on phase index i.
    ///
    /// Args:
    ///     i: The phase index of which to return the tap setting.
    ///     setting: The desired tap setting.
    pub fn set(&self, i: usize, setting: i32) {
        self.with_taps_mut(|taps| taps.set(i, setting))
    }

    /// Get all tap settings.
    ///
    /// Returns:
    ///     List of tap settings.
    pub fn get_all(&self) -> Vec<i32> {
        self.with_taps(|taps| taps.get_all().clone())
    }

    /// Set all tap settings.
    ///
    /// Args:
    ///     settings: List of all tap settings.
    pub fn set_all(&self, settings: Vec<i32>) {
        self.with_taps_mut(|taps| taps.set_all(settings.iter().cloned()))
    }

    fn __str__(&self) -> String {
        self.with_taps(|taps| format!("{:?}", taps))
    }
}

element_subclass!(
    /// A power transformer.
    ///
    /// A transformer model is based on a primitive model, where we only care about the windings. The primitive model
    /// has `n_sides` sides (which is, e.g., 2 for a 2-winding transformer and 3 for a 3-winding transformer), and
    /// `n_phases` phases, which is the number of phases.
    ///
    /// There are `n_sides x n_phases` windings, ordered as, e.g. `[PA, SA, PB, SB, PC, SC]` for a 2-winding 3-phase
    /// transformer, where `P` is primary, `S` is secondary, and `A, B, C` are the phases. This means, for example, that the
    /// primitive branch impedance matrix is block diagonal matrix organised as three 2x2 blocks. It depends only on the
    /// following: `n_sides`, `n_phases`, the leakage impedance of each winding, and the relative turns ratios
    /// between the windings.
    ///
    /// After calculating the primitive branch impedance matrix, we can map terminals to branches in various permutations.
    /// This allows us to distinguish, for example, between a DYN11 and a DYN1 transformer.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_node_ids: list of BusNodeId containing (bus_id, phase) for terminals
    ///     n_sides: The number of "sides" (primary, secondary, tertiary...); 2 for a normal 2-winding transformer.
    ///     n_phases: The number of "phases" (windings on each side); 3 for a 3 phase transformer, etc.
    ///     nom_turns: The nominal number of turns on each side, normally normalised to be relative to the primary which
    ///         is therefore 1.
    ///     taps: Optional `TransformerTapsData` specifying the transformer's taps.
    ///     z_series: Series impedance, one complex number per side, which is the impedance of each winding on that side.
    ///     terminal_mapping: One entry per side, each such entry containing one `(i, j)` tuple per phase, where `i` is
    ///         the index of the start terminal of the winding, and `j` is the index of the end terminal.
    ///
    /// Returns:
    ///     A new transformer object.
    #[derive(Debug)]
    Transformer,
    gr::Transformer
);

impl Transformer {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    #[allow(clippy::too_many_arguments)]
    pub fn new_raw(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        n_sides: usize,
        n_phases: usize,
        nom_turns: Vec<c64>,
        taps: Option<TransformerTapsData>,
        z_series: Vec<c64>,
        terminal_mapping: Vec<Vec<(usize, usize)>>,
    ) -> Result<Self, TransformerError> {
        let gr_taps = taps.map(
            |taps| gr::TransformerTaps::new(
                taps.side,
                taps.factor,
                taps.min_tap,
                taps.max_tap,
                n_phases,
            )
        );
        let gr_bts = bus_node_ids_to_gr(bus_node_ids);
        let gr_tx = gr::Transformer::new(
            n_sides,
            n_phases,
            nom_turns,
            gr_taps,
            z_series,
            terminal_mapping,
        )?;
        let elem = gr_tx.element(uid, gr_bts).into_shared();
        Ok(Transformer::from_elem(elem))
    }

    /// Creates a new two-winding [`Transformer`] with the given vector group, number of phases and other data.
    ///
    /// turns_ratio is np/ns. vector_group_str is all lowercase, with the primary always assumed to come first.
    pub fn new_raw_with_vector_group(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        n_phases: usize,
        nom_turns: Vec<c64>,
        taps: Option<TransformerTapsData>,
        z_series: (c64, c64),
        vector_group_str: &str
    ) -> Result<Transformer, TransformerError> {
        let gr_taps = taps.map(
            |taps| gr::TransformerTaps::new(
                taps.side,
                taps.factor,
                taps.min_tap,
                taps.max_tap,
                n_phases,
            )
        );
        let gr_bts = bus_node_ids_to_gr(bus_node_ids);
        let gr_tx = gr::Transformer::new_with_vector_group(
            n_phases,
            nom_turns,
            gr_taps,
            z_series,
            vector_group_str,
        )?;
        let elem = gr_tx.element(uid, gr_bts).into_shared();
        Ok(Transformer::from_elem(elem))
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl Transformer {
    #[allow(clippy::too_many_arguments)]
    #[new]
    pub fn new(
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        n_sides: usize,
        n_phases: usize,
        nom_turns: Vec<c64>,
        taps: Option<TransformerTapsData>,
        z_series: Vec<c64>,
        terminal_mapping: Vec<Vec<(usize, usize)>>,
    ) -> Result<PyClassInitializer<Self>, TransformerError> {
        Ok(Self::new_raw(uid, bus_node_ids, n_sides, n_phases, nom_turns, taps, z_series, terminal_mapping)?.into_initializer())
    }

    /// Create a new two-winding transformer with the given vector group.
    ///
    /// Args:
    ///     uid: The ID of the element.
    ///     bus_node_ids: list of BusNodeId containing (bus_id, phase) for terminals, with the final one being neutral.
    ///     n_phases:
    ///     nom_turns_ratio: The turns ratio, np/ns.
    ///     taps: Optional `TransformerTapsData` specifying the transformer's taps.
    ///     z_series: Series impedance, one complex number per side, which is the impedance of each winding on that side.
    ///     vector_group_str: The vector group, all lowercase.
    #[staticmethod]
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_vector_group<'py>(
        py: Python<'py>,
        uid: &str,
        bus_node_ids: Vec<BusNodeId>,
        n_phases: usize,
        nom_turns: Vec<c64>,
        taps: Option<TransformerTapsData>,
        z_series: (c64, c64),
        vector_group_str: &str
    ) -> PyResult<Bound<'py, Self>> {
        Bound::new(py, Self::new_raw_with_vector_group(
            uid, bus_node_ids, n_phases, nom_turns, taps, z_series, vector_group_str
        )?.into_initializer())
    }

    /// The number of sides (primary/secondary/tertiary...).
    ///
    /// Normal two-winding transformers have two sides, primary and secondary, while three-winding transformers
    /// have an additional tertiary winding. This is distinct from the number of phases of the transformer, e.g. we can
    /// have a three-phase, two-winding transformer. We reserve the term "winding" to mean a *single* winding of wire,
    /// i.e. a particular phase of a side.
    #[getter]
    pub fn n_sides(self_: PyRef<'_, Self>) -> usize {
        Self::with_spec(&self_, |s| s.n_sides)
    }

    /// Number of phases (single phase, three phase, etc.).
    #[getter]
    pub fn n_phases(self_: PyRef<'_, Self>) -> usize {
        Self::with_spec(&self_, |s| s.n_phases)
    }

    /// The number of windings, just `n_sides` x `n_phases`.
    #[getter]
    pub fn n_windings(self_: PyRef<'_, Self>) -> usize {
        Self::with_spec(&self_, |s| s.n_windings())
    }

    /// The nominal number of turns per side, scaled by an arbitrary factor.
    ///
    /// Only the ratios between the sides matter. So for a two winding transformer, we can use [np/ns, 1.0] or
    /// equivalently [1.0, ns/np]. For a three winding transformer, it is easiest to use [1.0, ns/np, nt/np].
    #[getter]
    pub fn nom_turns(self_: PyRef<'_, Self>) -> Vec<c64> {
        Self::with_spec(&self_, |s| s.nom_turns.clone())
    }

    /// Optional TransformerTaps (if the transformer is tapped).
    #[getter]
    pub fn taps(&self) -> Option<TransformerTaps> {
        self.elem.borrow().as_type::<gr::Transformer>().unwrap().taps.is_some().then(|| self.elem.clone().into())
    }

    /// The scaled number of turns for side `i_side` and phase index `i_phase`.
    pub fn turns(&self, i_side: usize, i_phase: usize) -> c64 {
        Self::with_spec(self, |s| s.turns(i_side, i_phase))
    }

    /// The turns ratio, `turns(i_side_0, i_phase) / turns(i_side_1, i_phase)`
    ///
    /// Args:
    ///     i_side_0: The first side index.
    ///     i_side_1: The second side index.
    ///     i_phase: The phase index for both sides.
    /// Returns:
    ///     The turns ratio.
    pub fn turns_ratio(&self, i_side_0: usize, i_side_1: usize, i_phase: usize) -> c64 {
        Self::with_spec(self, |s| s.turns_ratio(i_side_0, i_side_1, i_phase))
    }

    /// The primative branch impedance matrix for a single pair of windings with given phase and sides.
    ///
    /// Args:
    ///     i_side_0: The first side index.
    ///     i_side_1: The second side index.
    ///     i_phase: The phase index for both sides.
    /// Returns:
    ///     The primative branch impedance matrix.
    pub fn y_primitive(&self, i_side_0: usize, i_side_1: usize, i_phase: usize) -> Vec<Vec<c64>> {
        let m = Self::with_spec(self, |s| s.y_primitive(i_side_0, i_side_1, i_phase));
        m.row_iter().map(|r| r.iter().copied().collect()).collect()
    }
}
#[cfg(test)]
mod tests {
    use ::griphos as gr;
    use crate::network::{bn, Phase};
    use crate::transformer::{Transformer, TransformerTapsData};
    use gr::{c, c64, C_ZERO};

    #[test]
    fn test() {
        let taps_data = TransformerTapsData::new(0, 0.05, -3, 3);
        let bus_nodes = vec![
            bn("bus_0".to_string(), Phase::A),
            bn("bus_0".to_string(), Phase::B),
            bn("bus_0".to_string(), Phase::C),
            bn("bus_1".to_string(), Phase::A),
            bn("bus_1".to_string(), Phase::B),
            bn("bus_1".to_string(), Phase::C),
            bn("bus_1".to_string(), Phase::N),
        ];
        let tx = Transformer::new_raw_with_vector_group(
            "tx",
            bus_nodes,
            3,
            vec![c!(11000.0, 0.0), c!(230.0, 0.0)],
            Some(taps_data),
            (C_ZERO, c!(0.0, 1e-3)),
            "dyn11"
        ).unwrap();
        let taps = tx.taps().unwrap();
        assert!(tx.taps().unwrap().get_all() == vec![0, 0, 0]);
        dbg!(&taps.side());
        taps.set(1, -2);
        assert!(tx.taps().unwrap().get_all() == vec![0, -2, 0]);
    }
}
