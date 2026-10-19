use pyo3::prelude::*;

use ::griphos as gr;
use gr::{CooMat, c64, Shared, IntoShared};
use crate::plumbing::element_subclass;
use crate::network::{BusTermId, bus_term_ids_to_gr};

element_subclass!(Line, gr::Line);

impl Line {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    pub fn new_raw(
        uid: &str,
        bus_term_ids_0: Vec<BusTermId>,
        bus_term_ids_1: Vec<BusTermId>,
        y_const: CooMat<c64>,
    ) -> Self {
        let n_phase = bus_term_ids_0.len();
        let gr_line = gr::Line::new(n_phase, y_const);
        let gr_bts_0 = bus_term_ids_to_gr(bus_term_ids_0);
        let gr_bts_1 = bus_term_ids_to_gr(bus_term_ids_1);
        let elem = gr_line.element(uid, gr_bts_0, gr_bts_1).into_shared();
        Line::from_elem(elem)
    }
}

#[pymethods]
impl Line {

    #[new]
    pub fn new(
        uid: &str,
        bus_term_ids_0: Vec<BusTermId>,
        bus_term_ids_1: Vec<BusTermId>,
        y_const: CooMat<c64>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(uid, bus_term_ids_0, bus_term_ids_1, y_const).into_initializer()
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
        let line = Line::new_raw("new_line", bts0, bts1, y_const.clone());
        assert!(line.uid() == "new_line");
        line.with_spec(|x| assert!(x.y_const == line.elem.borrow().y_const()));
        assert!(line.elem.borrow().i_const() == vec![]);
        assert!(line.elem.borrow().s_const() == vec![]);
        assert!(line.elem.borrow().vmag_const() == vec![]);
        assert!(line.elem.borrow().vang_const() == vec![]);
    }
}
