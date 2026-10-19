use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use ::griphos as gr;
use gr::{c64, Shared, IntoShared};

use crate::plumbing::element_subclass;
use crate::network::{BusTermId, bus_term_ids_to_gr};

pub struct TransformerError(gr::TransformerError);

impl From<gr::TransformerError> for TransformerError {
    fn from(e: gr::TransformerError) -> Self { Self(e) }
}

impl From<TransformerError> for PyErr {
    fn from(e: TransformerError) -> PyErr {
        PyValueError::new_err(e.0.to_string())
    }
}

#[gen_stub_pyclass]
#[pyclass(module = "griphos", unsendable, from_py_object)]
#[derive(Clone)]
pub struct TransformerTapsData {
    pub side: usize,
    pub factor: f64,
    pub min_tap: i32,
    pub max_tap: i32,
    pub n_phases: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl TransformerTapsData {
    #[new]
    pub fn new(side: usize, factor: f64, min_tap: i32, max_tap: i32, n_phases: usize) -> Self {
        Self { side, factor, min_tap, max_tap, n_phases }
    }
}

#[gen_stub_pyclass]
#[pyclass(module = "griphos", unsendable)]
pub struct TransformerTaps {
    pub(crate) parent: Shared<gr::Transformer>,
}

impl TransformerTaps {
    fn with_taps<R>(&self, f: impl FnOnce(&gr::TransformerTaps) -> R) -> R {
        f(self.parent.borrow().taps.as_ref().unwrap())
    }

    fn with_taps_mut<R>(&mut self, f: impl FnOnce(&mut gr::TransformerTaps) -> R) -> R {
        f(self.parent.borrow_mut().taps.as_mut().unwrap())
    }
}

#[gen_stub_pymethods]
#[pymethods]
impl TransformerTaps {
    #[getter]
    fn side(&self) -> usize {
        self.with_taps(|taps| taps.side)
    }

    #[getter]
    fn factor(&self) -> f64 {
        self.with_taps(|taps| taps.factor)
    }

    #[getter]
    fn min_tap(&self) -> i32 {
        self.with_taps(|taps| taps.min_tap)
    }

    #[getter]
    fn max_tap(&self) -> i32 {
        self.with_taps(|taps| taps.max_tap)
    }

    pub fn get(&self, i: usize) -> i32 {
        self.with_taps(|taps| taps.get(i))
    }

    pub fn set(&mut self, i: usize, setting: i32) {
        self.with_taps_mut(|taps| taps.set(i, setting))
    }

    pub fn get_all(&self) -> Vec<i32> {
        self.with_taps(|taps| taps.get_all().clone())
    }

    pub fn set_all(&mut self, settings: Vec<i32>) {
        self.with_taps_mut(|taps| taps.set_all(settings.iter().cloned()))
    }

    fn __str__(&self) -> String {
        self.with_taps(|taps| format!("{:?}", taps))
    }
}

element_subclass!(Transformer, gr::Transformer);

impl Transformer {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    #[allow(clippy::too_many_arguments)]
    pub fn new_raw(
        uid: &str,
        bus_term_ids: Vec<BusTermId>,
        n_sides: usize,
        n_phases: usize,
        nom_turns: Vec<c64>,
        taps: Option<TransformerTapsData>,
        z_series: Vec<c64>,
        terminal_mapping: Vec<Vec<(usize, usize)>>,
    ) -> Result<Self, TransformerError> {
        let gr_taps = taps.map(
            |taps| gr::TransformerTaps::new(
                taps.side.into(),
                taps.factor,
                taps.min_tap,
                taps.max_tap,
                taps.n_phases,
            )
        );
        let gr_bts = bus_term_ids_to_gr(bus_term_ids);
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
        bus_term_ids: Vec<BusTermId>,
        n_phases: usize,
        nom_turns_ratio: c64,
        taps: Option<TransformerTapsData>,
        z_series: (c64, c64),
        vector_group_str: &str
    ) -> Result<Transformer, TransformerError> {
        let gr_taps = taps.map(
            |taps| gr::TransformerTaps::new(
                taps.side.into(),
                taps.factor,
                taps.min_tap,
                taps.max_tap,
                taps.n_phases,
            )
        );
        let gr_bts = bus_term_ids_to_gr(bus_term_ids);
        let gr_tx = gr::Transformer::new_with_vector_group(
            n_phases,
            nom_turns_ratio,
            gr_taps,
            z_series,
            vector_group_str,
        )?;
        let elem = gr_tx.element(uid, gr_bts).into_shared();
        Ok(Transformer::from_elem(elem))
    }
}

#[pymethods]
impl Transformer {
    #[allow(clippy::too_many_arguments)]
    #[new]
    pub fn new(
        uid: &str,
        bus_term_ids: Vec<BusTermId>,
        n_sides: usize,
        n_phases: usize,
        nom_turns: Vec<c64>,
        taps: Option<TransformerTapsData>,
        z_series: Vec<c64>,
        terminal_mapping: Vec<Vec<(usize, usize)>>,
    ) -> Result<PyClassInitializer<Self>, TransformerError> {
        Ok(Self::new_raw(uid, bus_term_ids, n_sides, n_phases, nom_turns, taps, z_series, terminal_mapping)?.into_initializer())
    }

    #[getter]
    pub fn n_windings(self_: PyRef<'_, Self>) -> usize {
        Self::with_spec(&self_, |s| s.n_windings())
    }

    pub fn turns(&self, i_side: usize, i_phase: usize) -> c64 {
        Self::with_spec(&self, |s| s.turns(i_side, i_phase))
    }

    pub fn turns_ratio(&self, i_side_0: usize, i_side_1: usize, i_phase: usize) -> c64 {
        Self::with_spec(&self, |s| s.turns_ratio(i_side_0, i_side_1, i_phase))
    }

    pub fn y_primitive(&self, i_side_0: usize, i_side_1: usize, i_phase: usize) -> Vec<Vec<c64>> {
        let m = Self::with_spec(&self, |s| s.y_primitive(i_side_0, i_side_1, i_phase));
        m.row_iter().map(|r| r.iter().copied().collect()).collect()
    }
}

mod tests {
    #[test]
    fn test() {
    }
}
