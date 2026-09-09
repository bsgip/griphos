use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use ::griphos as gr;
use gr::{c64, Shared, SharedExt};

use crate::plumbing::{element_subclass, py_enum, py_value};
use crate::network::{BusTermId};

py_enum!(TransformerSide, gr::TransformerSide {P, S});

py_value!(WindingPair, gr::WindingPair { pub p: (usize, usize), pub s: (usize, usize) });

#[gen_stub_pyclass]
#[pyclass(module = "griphos", unsendable)]
pub struct TransformerTapsData {
    pub side: TransformerSide,
    pub factor: f64,
    pub min_tap: i32,
    pub max_tap: i32,
    pub n_phases: usize,
}

#[gen_stub_pymethods]
#[pymethods]
impl TransformerTapsData {
    #[new]
    pub fn new(side: TransformerSide, factor: f64, min_tap: i32, max_tap: i32, n_phases: usize) -> Self {
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
    fn side(&self) -> TransformerSide {
        self.with_taps(|taps| taps.side.into())
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
    pub fn new_raw(
        uid: &str,
        bus_phases_0: Vec<BusTermId>,
        bus_phases_1: Vec<BusTermId>,
        winding_pairs: &[WindingPair],
        nom_turns_ratio: Vec<c64>,
        y_series: c64,
        y_shunt: c64,
        taps: Option<TransformerTapsData>,
    ) -> Self {
        let gr_taps = taps.map(
            |taps| gr::TransformerTaps::new(
                taps.side.to_rust(),
                taps.factor,
                taps.min_tap,
                taps.max_tap,
                taps.n_phases,
            )
        );
        let gr_bps_0: Vec<gr::BusTermId> = bus_phases_0.into_iter().map(|x| x.into_rust()).collect();
        let gr_bps_1: Vec<gr::BusTermId> = bus_phases_1.into_iter().map(|x| x.into_rust()).collect();
        let gr_tx = gr::Transformer::new(
            &winding_pairs.iter().map(|wp| gr::WindingPair::from(wp)).collect::<Vec<gr::WindingPair>>(),
            nom_turns_ratio,
            y_series,
            y_shunt,
            gr_taps
        );
        let elem = gr_tx.element(uid, &gr_bps_0, &gr_bps_1).into_shared();
        Transformer::from_elem(elem)
    }
}

mod tests {
    use super::*;

    #[test]
    fn test() {
    }
}
