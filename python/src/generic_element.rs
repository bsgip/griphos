use pyo3::prelude::*;

use ::griphos as gr;
use gr::{CooMat, c64, Shared, IntoShared};
use crate::plumbing::element_subclass;
use crate::network::{BusTermId, bus_term_ids_to_gr};

element_subclass!(GenericElement, gr::GenericElement);

impl GenericElement {
    // Plain-Rust constructor: no PyO3 involved, enables testing within rust.
    #[allow(clippy::too_many_arguments)]
    pub fn new_raw(
        uid: &str,
        bus_term_ids: Vec<BusTermId>,
        branches: Vec<(usize, usize)>,
        y_const: Option<CooMat<c64>>,
        i_const: Option<Vec<(usize, c64)>>,
        s_const: Option<Vec<(usize, c64)>>,
        vmag_const: Option<Vec<(usize, f64)>>,
        vang_const: Option<Vec<(usize, f64)>>,
        copper_plate: Option<Vec<Vec<usize>>>,
        grounding: Option<Vec<usize>>,
    ) -> Self {
        let gr_generic_element = gr::GenericElement::new(
            bus_term_ids.len(),
            branches,
            y_const.unwrap_or_default(),
            i_const.unwrap_or_default(),
            s_const.unwrap_or_default(),
            vmag_const.unwrap_or_default(),
            vang_const.unwrap_or_default(),
            copper_plate.unwrap_or_default(),
            grounding.unwrap_or_default(),
        );
        let gr_bts = bus_term_ids_to_gr(bus_term_ids);
        let elem = gr_generic_element.element(uid, gr_bts).into_shared();
        GenericElement::from_elem(elem)
    }
}

#[pymethods]
impl GenericElement {

    #[new]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        uid: &str,
        bus_term_ids: Vec<BusTermId>,
        branches: Vec<(usize, usize)>,
        y_const: Option<CooMat<c64>>,
        i_const: Option<Vec<(usize, c64)>>,
        s_const: Option<Vec<(usize, c64)>>,
        vmag_const: Option<Vec<(usize, f64)>>,
        vang_const: Option<Vec<(usize, f64)>>,
        copper_plate: Option<Vec<Vec<usize>>>,
        grounding: Option<Vec<usize>>,
    ) -> PyClassInitializer<Self> {
        Self::new_raw(
            uid, bus_term_ids, branches, y_const, i_const, s_const,
            vmag_const, vang_const, copper_plate, grounding
        ).into_initializer()
    }
}

#[cfg(test)]
mod tests {
    use crate::network::Phase;
    use gr::c;
    use super::*;

    #[test]
    fn test_new_generic_element() {
        let bts = vec![
            BusTermId {b: "bus_1".to_string(), p: Phase::N},
            BusTermId {b: "bus_1".to_string(), p: Phase::A},
            BusTermId {b: "bus_1".to_string(), p: Phase::B},
            BusTermId {b: "bus_1".to_string(), p: Phase::C},
        ];
        let branches = vec![(0, 1), (0, 2), (0, 3)];
        let y_const = vec![
            ((0, 0), c!(1.0, 0.0)),
            ((1, 1), c!(1.0, 0.0)),
            ((2, 2), c!(1.0, 0.0)),
        ];
        let i_const = vec![
            (0, c!(1.0, 0.0)),
            (1, c!(1.0, 0.0)),
            (2, c!(1.0, 0.0)),
        ];
        let s_const = vec![
            (0, c!(1.0, 0.0)),
            (1, c!(1.0, 0.0)),
            (2, c!(1.0, 0.0)),
        ];
        let vmag_const = vec![
            (0, 1.0),
            (1, 1.0),
            (2, 1.0),
        ];

        let generic_element = GenericElement::new_raw(
            "new_generic_element", bts, branches, Some(y_const), Some(i_const), Some(s_const),
            Some(vmag_const), None, None, None
        );
        assert!(generic_element.uid() == "new_generic_element");
        generic_element.with_spec(|x| assert!(x.y_const == generic_element.elem.borrow().y_const()));
        generic_element.with_spec(|x| assert!(x.i_const == generic_element.elem.borrow().i_const()));
        generic_element.with_spec(|x| assert!(x.s_const == generic_element.elem.borrow().s_const()));
        generic_element.with_spec(|x| assert!(x.vmag_const == generic_element.elem.borrow().vmag_const()));
        generic_element.with_spec(|x| assert!(x.vang_const == generic_element.elem.borrow().vang_const()));
    }
}
