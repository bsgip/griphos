//! Transformer elements.

use std::collections::HashSet;

use serde::Serialize;

use crate::foundation::*;
use crate::network::*;
use crate::TypeStrOf;

/// Specifies the side of the transformer, P = primary, S = secondary
// TODO: Three-winding transformers. We will probably have a general framework, where TransformerSide becomes
// a usize, ratio becomes a vector etc. Will probably be fairly easy to generalize.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum TransformerSide {
    P,
    S,
    // S1,
}

/// Winding type in a vector group, D, Y or Z.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum WindingType {
    D,
    Y,
    Z,
    // V, // Non-standard, open-delta
}

impl WindingType {
    /// Converts the winding type into a character.
    pub fn as_char(self) -> char {
        match self {
            WindingType::D => 'd',
            WindingType::Y => 'y',
            WindingType::Z => 'z',
        }
    }

    /// Creates the winding type from a character.
    pub fn from_char(c: char) -> Option<WindingType> {
        match c.to_ascii_lowercase() {
            'd' => Some(WindingType::D),
            'y' => Some(WindingType::Y),
            'z' => Some(WindingType::Z),
            _ => None,
        }
    }
}

/// Struct to hold a transformer's vector group information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorGroup {
    pub winding_types: Vec<WindingType>,
    pub secondary_clocks: Vec<u32>,
    pub neutral_brought_out: Vec<bool>,
}

impl VectorGroup {
    /// Create a new VectorGroup, with the given winding types, clock shifts, and neutral brought out information.
    pub fn new(
        winding_types: Vec<WindingType>,
        secondary_clocks: Vec<u32>,
        neutral_brought_out: Vec<bool>,
    ) -> Result<Self, String> {
        if winding_types.len() < 2 {
            return Err(String::from("too few windings in vector group string"));
        }
        if secondary_clocks.len() != winding_types.len() - 1 || neutral_brought_out.len() != winding_types.len()
        {
            return Err(String::from("length of secondary clocks should b n_windings - 1"));
        }
        if let Some(&bad_clock) = secondary_clocks.iter().find(|&&c| c > 11) {
            return Err(format!("invalid clock number {bad_clock}"));
        }
        Ok(Self { winding_types, secondary_clocks, neutral_brought_out })
    }
}

impl std::fmt::Display for VectorGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, winding_type) in self.winding_types.iter().enumerate() {
            write!(f, "{}", winding_type.as_char())?;
            if self.neutral_brought_out[i] {
                write!(f, "n")?;
            }
            if i > 0 {
                write!(f, "{}", self.secondary_clocks[i - 1])?;
            }
        }
        Ok(())
    }
}

impl std::str::FromStr for VectorGroup {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Err(String::from("empty vector group string"));
        }

        let chars: Vec<char> = s.to_lowercase().chars().collect();
        let mut winding_types = Vec::new();
        let mut secondary_clocks = Vec::new();
        let mut neutral_brought_out = Vec::new();

        let mut i = 0;
        while i < chars.len() {
            let winding_type = WindingType::from_char(chars[i]).ok_or(
                format!("unknown winding char {}", chars[i]))?;
            i += 1;

            let neutral = chars.get(i) == Some(&'n');
            if neutral {
                i += 1;
            }

            if !winding_types.is_empty() {
                // not the primary: a clock number is required
                let start = i;
                while chars.get(i).is_some_and(|c| c.is_ascii_digit()) {
                    i += 1;
                }
                if start == i {
                    return Err(String::from("missing clock number"));
                }
                let digits: String = chars[start..i].iter().collect();
                let clock: u32 = digits
                    .parse()
                    .map_err(|_| format!("invalid clock number {}", digits.clone()))?;
                if clock > 11 {
                    return Err(format!("invalid_clock_number {digits}"));
                }
                secondary_clocks.push(clock);
            }

            winding_types.push(winding_type);
            neutral_brought_out.push(neutral);
        }

        if winding_types.len() < 2 {
            return Err(String::from("too few windings"));
        }

        Ok(VectorGroup { winding_types, secondary_clocks, neutral_brought_out })
    }
}

/// A transformer winding pair, i.e. a specification of a magnetically coupled pair of windings, one primary and one
/// secondary. Each winding has a pair of terminal indexes, specifying the terminals between which the winding is
/// connected. Note that the indexing starts at zero for both primary and secondary, so for a three phase DD
/// transformer, p = (0, 1) and s = (2, 0) means the winding runs between the first and second terminals on the
/// primary side and the third and first terminals on the secondary side.
#[derive(Clone, Copy, Debug)]
pub struct WindingPair {
    /// Index of the primary side terminals
    pub p: (usize, usize),

    /// Index of the secondary side terminals
    pub s: (usize, usize),
}

/// Given a vector group and a phase count, recreate the [`WindingPair`]s.
pub fn winding_pairs_from_vector_group(vg: &VectorGroup, n_phase: usize) -> Result<Vec<WindingPair>, String> {
    if vg.winding_types.len() != 2 {
        return Err(String::from(
            "only two-winding vector groups are supported for now",
        ));
    }

    if vg.winding_types.contains(&WindingType::Z) {
        return Err(String::from(
            "zigzag isn't supported yet: each zigzag phase spans two legs, which doesn't fit one WindingPair",
        ));
    }

    let primary_type = vg.winding_types[0];
    let secondary_type = vg.winding_types[1];
    let target_clock = vg.secondary_clocks[0] as i32;

    // Inherent phase shift (in clock hours) of a winding's own (first) terminal relative to its leg's reference angle.
    fn type_shift(t: WindingType) -> i32 {
        match t {
            WindingType::Y => 0,
            WindingType::D => -1,
            WindingType::Z => unreachable!("zigzag is rejected above"),
        }
    }

    let base_clock = (type_shift(primary_type) - type_shift(secondary_type)).rem_euclid(12);

    if n_phase == 1 {
        if target_clock != base_clock {
            return Err(format!(
                "a single-phase {}-{} transformer's clock is fixed at {} by its connection type (got {target_clock})",
                primary_type.as_char(),
                secondary_type.as_char(),
                base_clock
            ));
        }

        fn single_phase_winding(t: WindingType) -> (usize, usize) {
            match t {
                WindingType::Y => (0, 1), // line to neutral
                WindingType::D => (0, 1), // line to line (phase-to-phase)
                WindingType::Z => unreachable!("zigzag is rejected above"),
            }
        }

        Ok(vec![WindingPair {
            p: single_phase_winding(primary_type),
            s: single_phase_winding(secondary_type),
        }])
    } else if n_phase == 3 {
        fn natural_winding(t: WindingType, i: usize, n: usize) -> (usize, usize) {
            match t {
                WindingType::Y => (i, n),
                WindingType::D => (i, (i + 1) % n),
                WindingType::Z => unreachable!("zigzag is rejected above"),
            }
        }

        for flip in [false, true] {
            for r in 0..n_phase {
                let rotation_shift = 4 * ((n_phase - r) % n_phase) as i32;
                let flip_shift = if flip { 6 } else { 0 };
                let clock = (base_clock + rotation_shift + flip_shift).rem_euclid(12);

                if clock == target_clock {
                    let pairs = (0..n_phase)
                        .map(|leg| {
                            let secondary_index = (leg + r) % n_phase;
                            let mut s = natural_winding(secondary_type, secondary_index, n_phase);
                            if flip {
                                s = (s.1, s.0);
                            }
                            WindingPair {
                                p: natural_winding(primary_type, leg, n_phase),
                                s,
                            }
                        })
                        .collect();
                    return Ok(pairs);
                }
            }
        }

        Err(format!(
            "clock number {target_clock} isn't reachable for a {}-{} pair (only {} clock numbers are, since base alignment gives {})",
            primary_type.as_char(),
            secondary_type.as_char(),
            if base_clock.rem_euclid(2) == 0 { "even" } else { "odd" },
            base_clock
        ))
    } else {
        Err(String::from(
            "only single-phase and three-phase transformers are supported for now",
        ))
    }
}

/// Tap setup for a transformer with taps.
#[derive(Debug, Serialize)]
pub struct TransformerTaps {
    /// Is the transformer tapped on the primary or secondary side?
    pub side: TransformerSide,
    /// Tap factor.
    ///
    /// For a transformer primary winding at tap i, the tapped ratio is
    /// nom_turns_ratio * (1 + i *  factor). For a transformer secondary
    /// winding at tap i, the tapped ratio is
    /// nom_turns_ratio / (1 + i *  factor)
    pub factor: f64,
    /// Minimum allowable tap.
    pub min_tap: i32,
    /// Maximum allowable tap.
    pub max_tap: i32,
    /// Tap settings for each winding pair.
    settings: Vec<i32>,
}

impl TransformerTaps {
    /// Creates a new TransformerTaps.
    pub fn new(side: TransformerSide, factor: f64, min_tap: i32, max_tap: i32, n_phase: usize) -> Self {
        Self {side, factor, min_tap, max_tap, settings: vec![0; n_phase]}
    }

    /// Get the tap setting for tap `i`.
    pub fn get(&self, i: usize) -> i32 {
        self.settings[i]
    }

    /// Set tap `i`.
    pub fn set(&mut self, i: usize, setting: i32) {
        assert!(i < self.settings.len() && setting >= self.min_tap && setting <= self.max_tap);
        self.settings[i] = setting;
    }

    /// Get all tap settings.
    pub fn get_all(&self) -> &Vec<i32> {
        &self.settings
    }

    /// Set all tap settings.
    pub fn set_all(&mut self, settings: impl Iterator<Item = i32>) {
        for (targ, val) in self.settings.iter_mut().zip(settings) {
            assert!(val >= self.min_tap && val <= self.max_tap);
            *targ = val;
        }
    }
}

/// A power transformer.
///
/// Currently, we allow only two-windings transformers, with arbitrary numbers of phases.
#[derive(Debug, Serialize, TypeStrOf)]
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
    /// Creates a new [`Transformer`].
    pub fn new(
        winding_pairs: &[WindingPair],
        nom_turns_ratio: Vec<c64>,
        z_series: c64,
        y_shunt: c64,
        taps: Option<TransformerTaps>,
    ) -> Self {
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
            n_term, n_phase, branches, nom_turns_ratio, y_series: 1.0 / z_series, y_shunt, taps
        }
    }

    /// Creates a new delta-wye connected [`Transformer`].
    pub fn new_dy(
        n_phase: usize,
        nom_turns_ratio: c64,
        z_series: c64,
        y_shunt: c64,
        taps: Option<TransformerTaps>,
    ) -> Result<Self, String> {
        let windings_p = delta_connection(n_phase)?;
        let windings_s = wye_connection(n_phase)?;
        let winding_pairs: Vec<WindingPair> = windings_p.into_iter().zip(windings_s)
            .map(|(p, s)| WindingPair { p, s }).collect();
        Ok(Self::new(&winding_pairs, vec![nom_turns_ratio; winding_pairs.len()], z_series, y_shunt, taps))
    }

    /// Creates a new delta-delta connected [`Transformer`].
    pub fn new_dd(
        n_phase: usize,
        nom_turns_ratio: c64,
        z_series: c64,
        y_shunt: c64,
        taps: Option<TransformerTaps>,
    ) -> Result<Self, String> {
        let windings_p = delta_connection(n_phase)?;
        let windings_s = delta_connection(n_phase)?;
        let winding_pairs: Vec<WindingPair> = windings_p.into_iter().zip(windings_s)
            .map(|(p, s)| WindingPair { p, s }).collect();
        Ok(Self::new(&winding_pairs, vec![nom_turns_ratio; winding_pairs.len()], z_series, y_shunt, taps))
    }

    /// Creates a new wye-wye connected [`Transformer`].
    pub fn new_yy(
        n_phase: usize,
        nom_turns_ratio: c64,
        z_series: c64,
        y_shunt: c64,
        taps: Option<TransformerTaps>,
    ) -> Result<Self, String> {
        let windings_p = wye_connection(n_phase)?;
        let windings_s = wye_connection(n_phase)?;
        let winding_pairs: Vec<WindingPair> = windings_p.into_iter().zip(windings_s)
            .map(|(p, s)| WindingPair { p, s }).collect();
        Ok(Self::new(&winding_pairs, vec![nom_turns_ratio; winding_pairs.len()], z_series, y_shunt, taps))
    }

    /// Wraps a transformer in an [`Element`], with the given bus terminal connections.
    pub fn element(
        self, id: &str,
        bus_phase_ids_p: &[BusPhaseId],
        bus_phase_ids_s: &[BusPhaseId],
    ) -> Element {
        let bus_phase_ids: Vec<BusPhaseId> = bus_phase_ids_p.iter().chain(bus_phase_ids_s.iter()).cloned().collect();
        self.element_generic(id, bus_phase_ids)
    }

    /// Returns the turns ratio for the `i`th winding.
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
    fn n_terms(&self) -> usize {
        self.n_term
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
            y.push(((2*i,   2*i  ), (self.y_series + 0.5 * self.y_shunt) / self.turns_ratio(i).norm_sqr()));
            y.push(((2*i,   2*i+1), -self.y_series / self.turns_ratio(i).conj()));
            y.push(((2*i+1, 2*i  ), -self.y_series / self.turns_ratio(i)));
            y.push(((2*i+1, 2*i+1), self.y_series + 0.5 * self.y_shunt));
        }
        y
    }
}

impl_g_serialize!(Transformer);

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
                &[bp!("bus_1", Phase::A), bp!("bus_g", Phase::N)],
                &[bp!("bus_2", Phase::A), bp!("bus_g", Phase::N)],
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
                &[bp!("bus_1", Phase::A), bp!("bus_1", Phase::B), bp!("bus_1", Phase::C)],
                &[bp!("bus_2", Phase::A), bp!("bus_2", Phase::B), bp!("bus_2", Phase::C)],
        );

        let n = e.n_terms();
        let mut dense = DMatrix::<c64>::zeros(n, n);
        for ((r, c), x) in e.y_const() {
            dense[(r, c)] += x;  // += handles duplicate indices naturally
        }

        println!("y_const:\n{}", dense);
    }

    #[test]
    fn test_transformer_3p_dyn11() {
    }
}
