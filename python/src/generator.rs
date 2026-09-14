use pyo3::prelude::*;

use ::griphos as gr;
use gr::{c64, Shared, SharedExt};
use crate::plumbing::{element_subclass};
use crate::network::{BusTermId, bus_term_ids_to_gr};

#[pyclass(eq, from_py_object)]
#[derive(Clone, PartialEq)]
pub enum GeneratorControl {
    SL {vmag: f64, vang: f64},
    PV {p: f64, vmag: f64},
    PQ {s: c64},
}

impl From<&gr::GeneratorControl> for GeneratorControl {
    fn from(v: &gr::GeneratorControl) -> Self {
        match v {
            gr::GeneratorControl::SL {vmag, vang} => GeneratorControl::SL {vmag: *vmag, vang: *vang},
            gr::GeneratorControl::PV {p, vmag} => GeneratorControl::PV {p: *p, vmag: *vmag},
            gr::GeneratorControl::PQ {s} => GeneratorControl::PQ {s: *s},
        }
    }
}

impl From<gr::GeneratorControl> for GeneratorControl {
    fn from(v: gr::GeneratorControl) -> Self {
        match v {
            gr::GeneratorControl::SL {vmag, vang} => GeneratorControl::SL {vmag, vang},
            gr::GeneratorControl::PV {p, vmag} => GeneratorControl::PV {p, vmag},
            gr::GeneratorControl::PQ {s} => GeneratorControl::PQ {s},
        }
    }
}

impl From<&GeneratorControl> for gr::GeneratorControl {
    fn from(v: &GeneratorControl) -> Self {
        match v {
            GeneratorControl::SL {vmag, vang} => gr::GeneratorControl::SL {vmag: *vmag, vang: *vang},
            GeneratorControl::PV {p, vmag} => gr::GeneratorControl::PV {p: *p, vmag: *vmag},
            GeneratorControl::PQ {s} => gr::GeneratorControl::PQ {s: *s},
        }
    }
}

impl From<GeneratorControl> for gr::GeneratorControl {
    fn from(v: GeneratorControl) -> Self {
        match v {
            GeneratorControl::SL {vmag, vang} => gr::GeneratorControl::SL {vmag, vang},
            GeneratorControl::PV {p, vmag} => gr::GeneratorControl::PV {p, vmag},
            GeneratorControl::PQ {s} => gr::GeneratorControl::PQ {s},
        }
    }
}

element_subclass!(Generator, gr::Generator);

impl Generator {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        bus_term_ids: Vec<BusTermId>,
        branches: Vec<(usize, usize)>,
        control: GeneratorControl
    ) -> Self {
        let gr_generator = gr::Generator::new(bus_term_ids.len(), branches, control.into());
        let bts = bus_term_ids_to_gr(bus_term_ids);
        let elem = gr_generator.element(uid, bts).into_shared();
        Generator::from_elem(elem)
    }
}

#[pymethods]
impl Generator {

    #[new]
    pub fn new(
        uid: &str,
        bus_term_ids: Vec<BusTermId>,
        branches: Vec<(usize, usize)>,
        control: GeneratorControl,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_term_ids, branches, control).into_initializer()
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use super::*;

    #[test]
    fn test_new_generator() {
        let bts = vec![
            BusTermId {b: "bus_1".to_string(), p: Phase::A},
            BusTermId {b: "bus_1".to_string(), p: Phase::B},
            BusTermId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let branches = vec![(0, 1), (1, 2), (2, 0)];
        let generator = Generator::new_raw("new_generator", bts, branches, GeneratorControl::SL {vmag: 1.0, vang: 0.0});
        assert!(generator.uid() == "new_generator");
        assert!(generator.elem.borrow().copper_plate() == vec![vec![0, 1, 2]]);
    }
}
