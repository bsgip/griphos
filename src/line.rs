//! Line elements.

use serde::Serialize;

use crate::foundation::*;
use crate::network::*;
use crate::TypeStrOf;

/// A line element, with given phase admittance matrix.
#[derive(Debug, Serialize, TypeStrOf)]
pub struct Line {
    /// The number of phases (wires).
    pub n_phase: usize,
    /// The phase admittance matrix, specified as ((term_0, term_1), y) triples.
    pub y_const: CooMat<c64>,
}

impl Line {
    /// Creates a new line, with the given number of phases and phase admittance matrix.
    ///
    /// # Examples
    ///
    /// ```
    /// # use griphos::{c, c64, Line, y_branch_from_sym_seq};
    /// let line = Line::new(3, y_branch_from_sym_seq(c!(1.0, 1.0), c!(1.0, 1.0), 3).unwrap());
    /// ```
    pub fn new(n_phase: usize, y_const: CooMat<c64>) -> Line {
        Line {n_phase, y_const}
    }

    /// Creates a new line, with the given z and z0.
    ///
    /// # Examples
    ///
    /// ```
    /// # use griphos::{c, c64, Line};
    /// let line = Line::new_from_sym_seq(c!(1.0, 1.0), c!(1.0, 1.0), 3);
    /// ```
    pub fn new_from_sym_seq(z_plus: c64, z_0: c64, n_phase: usize) -> Result<Line, String> {
        match y_branch_from_sym_seq(z_plus, z_0, n_phase) {
            Ok(y_const) => {
                Ok(Line{n_phase , y_const})
            },
            Err(s) => Err(s)
        }
    }

    /// Wraps an existing [`Line`] in an [`Element`].
    pub fn element(self, uid: &str, bus_term_ids_0: Vec<BusTermId>, bus_term_ids_1: Vec<BusTermId>) -> Element {
        assert!(bus_term_ids_0.len() == bus_term_ids_1.len(), "Lines must have the same number of phases at each end");
        self.element_generic(uid, bus_term_ids_0.into_iter().chain(bus_term_ids_1).collect())
    }
}

impl ElementSpec for Line {
    /// n_phase terminals at the start and n_phase at the end.
    fn n_terms(&self) -> usize {
        2 * self.n_phase
    }

    /// Each wire is a branch.
    fn n_branches(&self) -> usize {
        self.n_phase
    }

    /// Each branch runs from the terminal at one end to the corresponding terminal at the other end.
    ///
    /// Terminals are ordered as, e.g., (a0, b0, c0, a1, b1, c1) where the ends are 0, 1 and phases a, b, c.
    fn branch_wiring(&self) -> Vec<(usize, usize)> {
        (0..self.n_phase).map(|i| (i, i + self.n_phase)).collect()
    }

    fn y_const(&self) -> CooMat<c64> {
        self.y_const.clone()
    }
}

impl_g_serialize!(Line);

/// Calculates the branch admittance matrix from symmetric sequence components.
///
/// An implicit assumption is that z_plus == z_minus, which assumes certain
/// symmnetry in the line geometry. This is why "sym" is used in the name. It
/// is common to make this assumption as an approximation to the real world
/// case.
pub fn y_branch_from_sym_seq(z_plus: c64, mut z_0: c64, n_phase: usize) -> Result<CooMat<c64>, String> {
    if n_phase == 1 {
        // NOTE: How do we treat sequence components with single (and for that matter, two) wire lines?
        // We could effectively use the same matrix elements no matter the number of phases, i.e. picking submatrices
        // of the 3x3 3-wire phase impedance matrix. Comparing to opendss, we can confirm that this all matches for 3
        // or 2 phases, but not for single phase. In OpenDSS, single phase lines are treated as a special case by
        // setting z_0 => Z+ and then doing the regular calculation. This has the effect of giving a 1x1 impedance
        // matrix whose single element is Z+.
        //
        // All of this is open to interpretation (what does it mean to have sequence components for a single wire).
        // Below, we change the code to match OpenDSS's way of doing things, but is this really right?
        //
        // TODO: It would be nice to see how e.g. SINCAL does things too. It's not certain this is the best way.

        z_0 = z_plus;
    }

    if n_phase == 3 {
        let a: c64 = (2.0 * z_0 + z_plus) / (3.0 * z_0 * z_plus);
        let b: c64 = (-z_0 + z_plus) / (3.0 * z_0 * z_plus);
        Ok(vec![
            ((0, 0), a), ((0, 1), b), ((0, 2), b),
            ((1, 0), b), ((1, 1), a), ((1, 2), b),
            ((2, 0), b), ((2, 1), b), ((2, 2), a),
        ])
    } else if n_phase == 2 {
        let a = (z_0 + 2.0 * z_plus) / (2.0 * z_0 * z_plus + z_plus * z_plus);
        let b = (-z_0 + z_plus) / (2.0 * z_0 * z_plus + z_plus * z_plus);
        Ok(vec![
            ((0, 0), a), ((0, 1), b),
            ((1, 0), b), ((1, 1), a),
        ])
    } else if n_phase == 1 {
        let a = 3.0 / (z_0 + 2.0 * z_plus);
        Ok(vec![
            ((0, 0), a)
        ])
    }
    else {Err("n_phase must be 1, 2 or 3".to_string())}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line() {
        let ge = Line::new_from_sym_seq(c!(1.3, 0.1), c!(0.7,0.4), 3);
        println!("{}", &Wrap(ge.as_ref().unwrap()));
    }
}
