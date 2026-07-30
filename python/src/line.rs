use pyo3::prelude::*;
use ::griphos as gr;
use gr::{c64, Element, Shared, SharedExt};
use crate::network::BusTermId;

#[pyclass(module = "griphos", unsendable)]
pub struct Line {
    pub(crate) elem: Shared<Element>,
}

impl Line {
    fn with_spec<R>(&self, f: impl FnOnce(&gr::Line) -> R) -> R {
        let elem = self.elem.borrow();
        let spec = elem.as_type::<gr::Line>().expect("type mismatch");
        f(spec)
    }

    fn with_spec_mut<R>(&self, f: impl FnOnce(&mut gr::Line) -> R) -> R {
        let mut elem = self.elem.borrow_mut();
        let spec = elem.as_type_mut::<gr::Line>().expect("type mismatch");
        f(spec)
    }
}

#[pymethods]
impl Line {
    #[new]
    pub fn new(
        uid: &str, bus_phases_0: Vec<BusTermId>, bus_phases_1: Vec<BusTermId>, y_const: Vec<((usize, usize), c64)>
    ) -> Line {
        let n_phase = bus_phases_0.len() + bus_phases_1.len();
        let gr_line = gr::Line::new(n_phase, y_const);
        let gr_bps_0 = bus_phases_0.into_iter().map(|x| x.into_inner()).collect();
        let gr_bps_1 = bus_phases_1.into_iter().map(|x| x.into_inner()).collect();
        let elem = gr_line.element(uid, gr_bps_0,  gr_bps_1).into_shared();
        Line { elem }
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use gr::c;
    use super::*;

    #[test]
    fn test_new_line() {
        let bts0 = vec![BusTermId {b: "bus_1".to_string(), p: Phase::A}];
        let bts1 = vec![BusTermId {b: "bus_2".to_string(), p: Phase::A}];
        let y_const = vec![((0, 0), c!(1.0, 0.0))];
        let line = Line::new("new_line", bts0, bts1, y_const.clone());
        assert!(line.elem.borrow().uid == "new_line");
        line.with_spec(|x| assert!(x.y_const == line.elem.borrow().y_const()));
        assert!(line.elem.borrow().i_const() == vec![]);
        assert!(line.elem.borrow().s_const() == vec![]);
        assert!(line.elem.borrow().vmag_const() == vec![]);
        assert!(line.elem.borrow().vang_const() == vec![]);
    }
}
