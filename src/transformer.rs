use std::collections::HashSet;

use serde::Serialize;

use crate::foundation::*;
use crate::network::*;

/// Specifies the side of the transformer, P = primary, S = secondary
#[derive(Debug, Serialize)]
pub enum TransformerSide {
    P,
    S,
}

/// A transformer winding pair, i.e. a specification of a magnetically coupled
/// pair of windings, one primary and one secondary. Each winding has a pair
/// of terminal indexes, specifying the terminals between which the winding is
/// connected. Note that the indexing starts at zero for both primary and
/// secondary, so for a three phase DD transformer, p = (0, 1) and s = (2, 0)
/// means the winding runs between the first and second terminals on the
/// primary side and the third and first terminals on the secondary side.
#[derive(Clone, Copy, Debug)]
pub struct WindingPair {
    /// Index of the primary side terminals
    pub p: (usize, usize),

    /// Index of the secondary side terminals
    pub s: (usize, usize),
}

/// Tap setup for a transformer with taps.
#[derive(Debug, Serialize)]
pub struct TransformerTaps {
    /// Is the transformer tapped on the primary or secondary side?
    pub side: TransformerSide,
    /// Tap factor. For a transformer at tap i, the actual ratio is (1 + i) *
    pub factor: f64,
    pub min: i32,
    pub max: i32,
    settings: Vec<i32>,
}

impl TransformerTaps {
    pub fn new(side: TransformerSide, factor: f64, min: i32, max: i32, n_phases: usize) -> Self {
        Self {side, factor, min, max, settings: vec![0; n_phases]}
    }

    pub fn get(&self, i: usize) -> i32 {
        self.settings[i]
    }

    pub fn set(&mut self, i: usize, setting: i32) {
        assert!(i < self.settings.len() && setting >= self.min && setting <= self.max);
        self.settings[i] = setting;
    }

    pub fn get_all(&self) -> &Vec<i32> {
        &self.settings
    }

    pub fn set_all(&mut self, settings: impl Iterator<Item = i32>) {
        for (targ, val) in self.settings.iter_mut().zip(settings) {
            assert!(val >= self.min && val <= self.max);
            *targ = val;
        }
    }
}

#[derive(Serialize)]
pub struct Transformer {
    /// Number of terminals.
    ///
    /// A three phase DD will have 6 terminals while a three phase DYN will have 7.
    pub n_term: usize,

    /// Number of phases, where each phase correponds to a single pair of windings.
    pub n_phase: usize,

    /// Branches
    pub branches: Vec<(usize, usize)>,

    /// Transformer turns ratio n_p / n_s
    pub nom_turns_ratio: Vec<c64>,

    /// Series impedance, comprising the leakage impedance plus winding resistance
    pub y_series: c64,

    /// Shunt admittance, comprising magnetising susceptance plus core losses
    pub y_shunt: c64,

    /// Specification for optional taps
    pub taps: Option<TransformerTaps>,

}

impl Transformer {
    pub fn new(
        winding_pairs: &[WindingPair],
        nom_turns_ratio: Vec<c64>,
        y_series: c64,
        y_shunt: c64,
        taps: Option<TransformerTaps>,
    ) -> Transformer {
        let n_phase = winding_pairs.len();
        #[allow(clippy::type_complexity)]
        let (windings_p, windings_s): (Vec<(usize, usize)>, Vec<(usize, usize)>) = winding_pairs.iter()
            .map(|WindingPair {p: wpp, s: wps}| (wpp, wps)).unzip();

        fn check_terms(windings: &[(usize, usize)]) -> Result<usize, ()> {
            let terms: HashSet<usize> = windings.iter().flat_map(|&(a, b)| [a, b]).collect();
            let nterm = terms.len();
            if terms == (0..nterm).collect::<HashSet<usize>>() {Ok(nterm)} else {Err(())}
        }

        let nterm_p = check_terms(&windings_p).expect("windings_p must be specified as a vector using a permutation of 0..nterm_p");
        let nterm_s = check_terms(&windings_s).expect("windings_s must be specified as a vector using a permutation of 0..nterm_s");
        let n_term = nterm_p + nterm_s;

        // Branches are ordered as [winding_0_p, winding_0_s, winding_1_p, winding_1_s, ...]
        // Terminals are ordered as [term_0_p, term_1_p ... term_0_s, term_1_s, ...]
        let mut branches: Vec<(usize, usize)> = Vec::with_capacity(2 * n_phase);
        for wp in winding_pairs {
            let ws: (usize, usize) = (wp.s.0 + nterm_p, wp.s.1 + nterm_p);
            branches.push(wp.p);
            branches.push(ws);
        }
        assert!(nom_turns_ratio.len() == n_phase);
        Transformer {
            n_term, n_phase, branches, nom_turns_ratio, y_series, y_shunt, taps
        }
    }

    pub fn element(
        self, id: &str,
        bus_phases_p: &[BusTermId],
        bus_phases_s: &[BusTermId],
    ) -> Element {
        let bus_phases: Vec<BusTermId> = bus_phases_p.iter().chain(bus_phases_s.iter()).cloned().collect();
        self.element_generic(id, bus_phases)
    }

    pub fn turns_ratio(&self, i: usize) -> c64 {
        if let Some(taps) = &self.taps {
            let x = 1.0 + taps.factor * (taps.settings[i] as f64);
            match taps.side {
                TransformerSide::P => self.nom_turns_ratio[i] * x,
                TransformerSide::S => self.nom_turns_ratio[i] / x,
            }
        } else {
            self.nom_turns_ratio[i]
        }
    }
}

impl ElementSpec for Transformer {
    fn element_type(&self) -> &str {
        "Transformer"
    }

    fn n_terms(&self) -> usize {
        2 * self.n_phase
    }

    fn n_branches(&self) -> usize {
        2 * self.n_phase
    }

    /// Branches are ordered as [p0, s0, p1, s1, ...]
    fn branch_wiring(&self) -> Vec<(usize, usize)> {
        self.branches.clone()
    }

    fn y_const(&self) -> Vec<((usize, usize), c64)> {
        let mut y: Vec<((usize, usize), c64)> = Vec::with_capacity((2 * self.n_phase).pow(2));
        for i in 0..self.n_phase {
            y.push(((i, i), (self.y_series + 0.5 * self.y_shunt) / self.turns_ratio(i).norm_sqr()));
            y.push(((i, i + self.n_phase), -self.y_series / self.turns_ratio(i).conj()));
            y.push(((i + self.n_phase, i), -self.y_series / self.turns_ratio(i)));
            y.push(((i + self.n_phase, i + self.n_phase), self.y_series + 0.5 * self.y_shunt));
        }
        y
    }
}

impl_g_ser_disp!(Transformer);

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::DMatrix;

    #[test]
    fn test_transformer_topology_1() {
        let e: Element = Transformer::new(
            &[WindingPair {p: (1, 0), s: (1, 0)}],
            vec![c!(2.0, 0.0)],
            c!(1.0, 0.0),
            c!(0.0, 0.0),
            Some(TransformerTaps::new(TransformerSide::P, 0.02, 1, -5, 5))
        ).element(
                "transformer",
                &[bt!("bus_1", Phase::A), bt!("bus_g", Phase::N)],
                &[bt!("bus_2", Phase::A), bt!("bus_g", Phase::N)],
        );

        let n = e.n_terms();
        let mut dense = DMatrix::<c64>::zeros(n, n);
        for ((r, c), x) in e.y_const() {
            dense[(r, c)] += x;  // += handles duplicate indices naturally
        }

        println!("y_const:\n{}", dense);
    }

    #[test]
    fn test_transformer_topology_3() {
        let e: Element = Transformer::new(
            &[WindingPair {p: (0, 1), s: (0, 1)}, WindingPair {p: (1, 2), s: (0, 2)}, WindingPair {p:(2, 0), s: (0, 3)}],
            vec![c!(1.0, 0.0), c!(1.0, 0.0), c!(1.0, 0.0)],
            c!(1.0, 0.0),
            c!(1.0, 0.0),
            Some(TransformerTaps::new(TransformerSide::P, 0.02, 3, -5, 5)),
        ).element(
                "transformer",
                &[bt!("bus_1", Phase::A), bt!("bus_1", Phase::B), bt!("bus_1", Phase::C)],
                &[bt!("bus_2", Phase::A), bt!("bus_2", Phase::B), bt!("bus_2", Phase::C)],
        );

        let n = e.n_terms();
        let mut dense = DMatrix::<c64>::zeros(n, n);
        for ((r, c), x) in e.y_const() {
            dense[(r, c)] += x;  // += handles duplicate indices naturally
        }

        println!("y_const:\n{}", dense);
    }
}
