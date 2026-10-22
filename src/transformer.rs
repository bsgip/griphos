//! Transformer elements.

use std::str::FromStr;

use serde::Serialize;
use thiserror::Error;

use crate::foundation::*;
use crate::network::*;
use crate::TypeStrOf;

/// Tap setup for a transformer with taps.
#[derive(Debug, Serialize)]
pub struct TransformerTaps {
    /// Is the transformer tapped on the primary, secondary (or tertiary if it exists) side?
    pub side: usize,

    /// Tap factor.
    ///
    /// The tapped number of turns is nom_turns * (1 + i *  factor).
    pub factor: f64,

    /// Minimum allowable tap.
    pub min_tap: i32,

    /// Maximum allowable tap.
    pub max_tap: i32,

    /// Tap settings for each phase.
    settings: Vec<i32>,
}

impl TransformerTaps {
    /// Creates a new TransformerTaps.
    pub fn new(side: usize, factor: f64, min_tap: i32, max_tap: i32, n_phases: usize) -> Self {
        Self {side, factor, min_tap, max_tap, settings: vec![0; n_phases]}
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

#[derive(Debug, Error)]
pub enum TransformerError {
    #[error("bad terminal mapping {0:?}.")] BadTerminalMapping(Vec<Vec<(usize, usize)>>),
    #[error("wrong number of phases {0}.")] WrongNumerOfPhases(usize),
    #[error("error {0:?}")] Error(String),
}

impl From<String> for TransformerError {
    fn from(s: String) -> Self {
        Self::Error(s)
    }
}

impl From<TransformerError> for String {
    fn from(e: TransformerError) -> Self {
        e.to_string()
    }
}

/// A power transformer.
///
/// A transformer model is based on a primitive model, where we only care about the windings. The primitive model
/// has `n_sides` sides (which is, e.g., 2 for a 2-winding transformer and 3 for a 3-winding transformer), and
/// `n_phases` phases, which is the number of phases.
///
/// There are `n_sides x n_phases` windings, ordered as, e.g. `[PA, SA, PB, SB, PC, SC]` for a 2-winding 3-phase
/// transformer, where `P` is primary, `S` is secondary, and `A, B, C` are the phases. This means, for example, that the
/// primitive branch impedance matrix is block diagonal matrix organised as three 2x2 blocks. It depends only on the
/// following: `n_sides`, `n_phases`, the leakage impedance of each winding, and the relative turns.
/// between the windings.
///
/// After calculating the primitive branch impedance matrix, we can map terminals to branches in various permutations.
/// This allows us to distinguish, for example, between a DYN11 and a DYN1 transformer.
#[derive(Debug, Serialize, TypeStrOf)]
pub struct Transformer {

    // -------------------------------------------------------------------------
    // Primative model
    //
    // The primitive model describes the transformer configuration excluding all mapping between windings and terminals.
    // -------------------------------------------------------------------------

    /// The number of sides (primary/secondary/tertiary...).
    ///
    /// Normal two-winding transformers have two sides, primary and secondary, while three-winding transformers
    /// have an additional tertiary winding. This is distinct from the number of phases of the transformer, e.g. we can
    /// have a three-phase, two-winding transformer. We reserve the term "winding" to mean a *single* winding of wire,
    /// i.e. a particular phase of a side.
    pub n_sides: usize,

    /// Number of phases (single phase, three phase, etc.).
    pub n_phases: usize,

    /// The nominal number of turns per side, scaled by an arbitrary factor.
    ///
    /// Only the ratios between the sides matter. So for a two winding transformer, we can use [np/ns, 1.0] or
    /// equivalently [1.0, ns/np]. For a three winding transformer, it is easiest to use [1.0, ns/np, nt/np].
    pub nom_turns: Vec<c64>,

    /// Specification for optional taps of each winding.
    ///
    /// The ordering is the same as the windings, e.g. [PA, SA, PB, SB, PC, SC].
    pub taps: Option<TransformerTaps>,

    /// Series impedance of each side, assumed to be duplicated across the windings.
    pub z_series: Vec<c64>,

    // -------------------------------------------------------------------------
    // Terminal mapping
    // -------------------------------------------------------------------------

    /// Number of terminals. These are terminals of the model, regardless of whether the terminal is "brought out", i.e.
    /// exposed to the outside world, in the actual equipment. So for example an internal star point counts as a
    /// terminal. A three phase DD transformer will have 6 terminals while a three phase DYN will have 7.
    pub n_terms: usize,

    /// In transformers, the branches are [(start_terminal_idx, end_terminal_idx)] indexed by winding. The
    /// start_terminal_idx can be equated with the "dotted" terminals for each winding.
    pub branch_wiring: Vec<(usize, usize)>,
}

impl Transformer {
    /// Creates a new [`Transformer`].
    ///
    /// n_sides:
    ///     The number of sides, i.e. 2 for a normal 2-winding transformer, 3 for a 3-winding transformer, etc.
    /// n_phases:
    ///     The number of phases, i.e. 1 for a single phase transformer, 3 for a 3-phase transformer.
    /// nom_turns:
    ///     The normalised number of turns on each side - only the ratios between the values matter.
    /// taps:
    ///     Optional taps as TransformerTaps.
    /// z_series:
    ///     The series impedance on each side.
    /// terminal_mapping:
    ///     For each side, and for each phase in that side, the start and end terminal indexes.
    ///     Example 1: a 2-winding 3-phase dyn11 transformer:
    ///         [[(0, 1), (1, 2), (2, 0)], [(3, 6), (4, 6), (5, 6)]].
    ///     Example 2: a 2-winding 3-phase ynyn0 transformer with the neutrals tied together:
    ///         [[(0, 6), (1, 6), (2, 6)], [(3, 6), (4, 6), (5, 6)]].
    ///     Note that by convention, the first index is assumed to be the "dotted" one, and the neutrals come last.
    pub fn new(
        n_sides: usize,
        n_phases: usize,
        nom_turns: Vec<c64>,
        taps: Option<TransformerTaps>,
        z_series: Vec<c64>,
        terminal_mapping: Vec<Vec<(usize, usize)>>,
    ) -> Result<Self, TransformerError> {
        fn flatten_transposed<T>(v: &[Vec<T>]) -> Vec<T> where T: Clone {
            let cols = v.first().map_or(0, Vec::len);
            (0..cols).flat_map(|i| v.iter().map(move |row| &row[i])).cloned().collect()
        }
        let branch_wiring: Vec<(usize, usize)> = flatten_transposed(&terminal_mapping);
        let mut terms: Vec<usize> = branch_wiring.iter().flat_map(|(i, j)| [*i, *j]).collect();
        terms.sort();
        terms.dedup();
        let n_terms = terms.len();
        if terms.is_empty() || *terms.first().unwrap() != 0 || *terms.last().unwrap() != n_terms - 1 {
            // Set of terminals must be a contiguous set {0...n_terms}
            return Err(TransformerError::BadTerminalMapping(terminal_mapping));
        }
        Ok(Transformer {n_sides, n_phases, nom_turns, taps, z_series, n_terms: terms.len(), branch_wiring})
    }

    /// Creates a new two-winding [`Transformer`] with the given vector group, number of phases and other data.
    ///
    /// nom_turns is the nominal turns count for each winding - only the ratios between the values matter.
    /// vector_group_str is all lowercase, with the primary always assumed to come first.
    pub fn new_with_vector_group(
        n_phases: usize,
        nom_turns: Vec<c64>,
        taps: Option<TransformerTaps>,
        z_series: (c64, c64),
        vector_group_str: &str
    ) -> Result<Transformer, TransformerError> {
        let vg: VectorGroup = VectorGroup::from_str(vector_group_str)?;
        let term_mapping = terminal_mapping_from_vector_group(&vg, n_phases)?;
        let zs: Vec<c64> = vec![z_series.0, z_series.1];
        Transformer::new(2, n_phases, nom_turns, taps, zs, term_mapping)
    }

    /// Wraps a transformer in an [`Element`], with the given bus/phase connections.
    ///
    /// The [`BusNodeId`]s are in the order of the terminal indexes used in the terminal_mapping. So for example,
    /// a dyn11 transformer has the terminal mapping [[(0,1), (1, 2), (2, 0)], [(3, 6), (4, 6), (5, 6)]], and the
    /// `BusNodeIds` are given as a seven element list whose indices correspond to the numbers in the terminal mapping.
    pub fn element(self, id: &str, bus_node_ids: Vec<BusNodeId>) -> Element {
        self.element_generic(id, bus_node_ids)
    }

    /// The number of windings, just `n_sides` x `n_phases`.
    pub fn n_windings(&self) -> usize {
        self.n_sides * self.n_phases
    }

    /// The winding index for side = i_side, phase = i_phase.
    fn i_winding(&self, i_side: usize, i_phase: usize) -> usize {
        i_phase * self.n_sides + i_side
    }

    /// Returns the turns count for the winding on side i_side and phase i_phase, relative to the winding on side 0
    /// and phase i_phase.
    pub fn turns(&self, i_side: usize, i_phase: usize) -> c64 {
        let mut f = 1.0;
        if let Some(taps) = &self.taps && taps.side == i_side {
            f += taps.factor * (taps.settings[i_phase] as f64);
        }
        self.nom_turns[i_side] * f
    }

    /// Returns the turns ratio, turns(i_side_0, i_phase) / turns(i_side_1, i_phase).
    pub fn turns_ratio(&self, i_side_0: usize, i_side_1: usize, i_phase: usize) -> c64 {
        self.turns(i_side_0, i_phase) / self.turns(i_side_1, i_phase)
    }

    /// The primitive branch admittance matrix for a single phase and two particular sides.
    pub fn y_primitive(&self, i_side_0: usize, i_side_1: usize, i_phase: usize) -> Mat<c64> {
        let n0 = self.turns(i_side_0, i_phase);
        let n02 = c!(n0.norm_sqr(), 0.0);
        let n1 = self.turns(i_side_1, i_phase);
        let n12 = c!(n1.norm_sqr(), 0.0);
        let z0 = self.z_series[i_side_0];
        let z1 = self.z_series[i_side_1];
        let f = C_ONE / (n02 * z1 + n12 * z0);
        Mat::from_row_slice(2, 2, &[
            f * n12,                -f * n0 * n1.conj(),
            -f * n0.conj() * n1,    f * n02
        ])
    }
}

impl ElementSpec for Transformer {
    fn n_terms(&self) -> usize {
        self.n_terms
    }

    fn n_branches(&self) -> usize {
        self.n_sides * self.n_phases
    }

    /// Branches are ordered as, e.g.,  [PA, SA, PB, SB, ...]
    fn branch_wiring(&self) -> Vec<(usize, usize)> {
        self.branch_wiring.clone()
    }

    fn y_const(&self) -> CooMat<c64> {
        let mut y: CooMat<c64> = Vec::with_capacity(self.n_branches().pow(2));
        for ip in 0..self.n_phases {
            for is0 in 0..self.n_sides {
                for is1 in (is0 + 1)..self.n_sides {
                    let yi = self.y_primitive(is0, is1, ip);
                    let i0 = self.i_winding(is0, ip);
                    let i1 = self.i_winding(is1, ip);
                    y.push(((i0, i0), yi[(0, 0)]));
                    y.push(((i0, i1), yi[(0, 1)]));
                    y.push(((i1, i0), yi[(1, 0)]));
                    y.push(((i1, i1), yi[(1, 1)]));
                }
            }
        }
        y
    }
}

impl_g_serialize!(Transformer);

// Vector Group Stuff

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

/// Terminal pairs for one side's windings, numbered from `first_terminal`.
/// Y: each winding runs (phase terminal, shared neutral), so it uses n + 1 terminals.
/// D: each winding runs (phase terminal, next phase terminal), so it uses n terminals.
fn natural_winding_terminals(t: WindingType, n_phases: usize, first_terminal: usize) -> Vec<(usize, usize)> {
    // A single-phase winding is just two terminals, regardless of type.
    if n_phases == 1 {
        vec![(first_terminal, first_terminal + 1)]
    } else {
        match t {
            WindingType::Y => {
                (0..n_phases).map(|i| (first_terminal + i, first_terminal + n_phases)).collect()
            },
            WindingType::D => {
                (0..n_phases).map(|i| (first_terminal + i, first_terminal + (i + 1) % n_phases)).collect()
            },
            WindingType::Z => unreachable!("zigzag is rejected by the caller"),
        }
    }
}

/// How many terminals one side occupies.
fn terminal_count(t: WindingType, n_phases: usize) -> usize {
    match (t, n_phases) {
        (_, 1) => 2,
        (WindingType::Y, n) => n + 1,
        (WindingType::D, n) => n,
        (WindingType::Z, _) => unreachable!("zigzag is rejected by the caller"),
    }
}

/// Inherent phase shift (in clock hours) of a winding's first terminal relative to its leg's reference angle.
fn type_shift(t: WindingType) -> i32 {
    match t {
        WindingType::Y => 0,
        WindingType::D => -1,
        WindingType::Z => unreachable!("zigzag is rejected by the caller"),
    }
}

/// Convert a vector group and phase count into a terminal mapping:
/// `[primary_windings, secondary_windings]`, each a list of (terminal, terminal) pairs.
/// e.g. dyn11, 3 phases -> [[(0,1),(1,2),(2,0)], [(3,6),(4,6),(5,6)]]
fn terminal_mapping_from_vector_group(vg: &VectorGroup, n_phases: usize) -> Result<Vec<Vec<(usize, usize)>>, String> {
    if vg.winding_types.len() != 2 {
        return Err("only two-winding vector groups are supported for now".into());
    }
    if vg.winding_types.contains(&WindingType::Z) {
        return Err("zigzag isn't supported yet".into());
    }
    if n_phases != 1 && n_phases != 3 {
        return Err("only single-phase and three-phase transformers are supported for now".into());
    }

    let primary_type = vg.winding_types[0];
    let secondary_type = vg.winding_types[1];
    let target_clock = vg.secondary_clocks[0] as i32;

    let primary_terminals = natural_winding_terminals(primary_type, n_phases, 0);
    let secondary_first = terminal_count(primary_type, n_phases);

    let base_clock = (type_shift(primary_type) - type_shift(secondary_type)).rem_euclid(12);

    if n_phases == 1 {
        // Single phase: a winding is just two terminals, so the only freedom is polarity.
        // Natural order gives clock 0; swapping the secondary terminals gives clock 6.
        let flip = match target_clock {
            0 => false,
            6 => true,
            _ => {
                return Err(format!(
                    "a single-phase transformer's clock must be 0 or 6 (got {target_clock})"
                ));
            }
        };

        let mut secondary = natural_winding_terminals(secondary_type, 1, secondary_first);
        if flip {
            secondary = secondary.into_iter().map(|(a, b)| (b, a)).collect();
        }
        return Ok(vec![primary_terminals, secondary]);
    }

    // Three phase: try each (flip, rotation) of the secondary relative to the primary.
    let secondary_natural = natural_winding_terminals(secondary_type, n_phases, secondary_first);

    for flip in [false, true] {
        for r in 0..n_phases {
            let rotation_shift = 4 * ((n_phases - r) % n_phases) as i32;
            let flip_shift = if flip { 6 } else { 0 };
            let clock = (base_clock + rotation_shift + flip_shift).rem_euclid(12);

            if clock == target_clock {
                // Primary leg `leg` is coupled to secondary winding (leg + r) % n,
                // optionally with reversed polarity. The secondary list is in leg order.
                let secondary = (0..n_phases)
                    .map(|leg| {
                        let (a, b) = secondary_natural[(leg + r) % n_phases];
                        if flip { (b, a) } else { (a, b) }
                    })
                    .collect();
                return Ok(vec![primary_terminals, secondary]);
            }
        }
    }

    Err(format!(
        "clock number {target_clock} isn't reachable for a {}-{} pair (only {} clock numbers are, since base alignment gives {base_clock})",
        primary_type.as_char(),
        secondary_type.as_char(),
        if base_clock % 2 == 0 { "even" } else { "odd" },
    ))
}

/// Typical series impedance (R, X) in ohms of a two-winding transformer.
///
/// Referred to the winding with line-to-line voltage `v` (V), for a rated power `s_va` (VA).
pub fn typical_tx_z_series(s_va: f64, v_v: f64) -> c64 {
    // Typical short-circuit impedance in percent (IEC 60076-5 style steps)
    let uk_pct = match s_va {
        s if s <= 630.0e3 => 4.0,
        s if s <= 1250.0e3 => 5.0,
        s if s <= 3150.0e3 => 6.25,
        s if s <= 6300.0e3 => 7.15,
        s if s <= 12500.0e3 => 8.35,
        s if s <= 25000.0e3 => 10.0,
        _ => 12.5,
    };

    // Rough empirical X/R ratio, growing with size
    let x_over_r = 6.0 * (s_va / 1e6).powf(0.4);

    let z_base = v_v.powi(2) / s_va; // ohm
    let z = uk_pct / 100.0 * z_base;
    let r = z / (1.0 + x_over_r * x_over_r).sqrt();
    c!(r, r * x_over_r)
}

#[cfg(test)]
mod tests {
    use crate::SolverStatusFlags;
    use crate::load::*;
    use crate::source::*;
    use crate::test_utils::*;
    use super::*;

    #[test]
    fn test_natural_winding_terminals() {
        assert_eq!(natural_winding_terminals(WindingType::Y, 1, 0), [(0, 1)]);
        assert_eq!(natural_winding_terminals(WindingType::Y, 2, 0), [(0, 2), (1, 2)]);
        assert_eq!(natural_winding_terminals(WindingType::Y, 3, 0), [(0, 3), (1, 3), (2, 3)]);

        assert_eq!(natural_winding_terminals(WindingType::D, 3, 0), [(0, 1), (1, 2), (2, 0)]);

        assert_eq!(natural_winding_terminals(WindingType::Y, 1, 2), [(2, 3)]);
        assert_eq!(natural_winding_terminals(WindingType::Y, 2, 3), [(3, 5), (4, 5)]);
        assert_eq!(natural_winding_terminals(WindingType::Y, 3, 4), [(4, 7), (5, 7), (6, 7)]);

        assert_eq!(natural_winding_terminals(WindingType::D, 3, 3), [(3, 4), (4, 5), (5, 3)]);
    }

    #[test]
    fn test_terminal_mapping_from_vector_group() {
        fn do_test(vg_str: &str, n_phases: usize, expected: &[&[(usize, usize)]]) {
            let vg = VectorGroup::from_str(vg_str).unwrap();
            let term_map = terminal_mapping_from_vector_group(&vg, n_phases).unwrap();
            assert_eq!(term_map, expected)
        }

        do_test("ynyn0", 1, &[&[(0, 1)], &[(2, 3)]]);
        do_test("ynyn6", 1, &[&[(0, 1)], &[(3, 2)]]);
        do_test("yy0", 1, &[&[(0, 1)], &[(2, 3)]]);

        do_test("ynyn0", 3, &[&[(0, 3), (1, 3), (2, 3)], &[(4, 7), (5, 7), (6, 7)]]);
        do_test("ynyn4", 3, &[&[(0, 3), (1, 3), (2, 3)], &[(6, 7), (4, 7), (5, 7)]]);
        do_test("ynyn8", 3, &[&[(0, 3), (1, 3), (2, 3)], &[(5, 7), (6, 7), (4, 7)]]);
        do_test("ynyn6", 3, &[&[(0, 3), (1, 3), (2, 3)], &[(7, 4), (7, 5), (7, 6)]]);
        do_test("ynyn10", 3, &[&[(0, 3), (1, 3), (2, 3)], &[(7, 6), (7, 4), (7, 5)]]);
        do_test("ynyn2", 3, &[&[(0, 3), (1, 3), (2, 3)], &[(7, 5), (7, 6), (7, 4)]]);

        do_test("dd0", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(3, 4), (4, 5), (5, 3)]]);
        do_test("dd4", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(5, 3), (3, 4), (4, 5)]]);
        do_test("dd8", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(4, 5), (5, 3), (3, 4)]]);
        do_test("dd6", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(4, 3), (5, 4), (3, 5)]]);
        do_test("dd10", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(3, 5), (4, 3), (5, 4)]]);
        do_test("dd2", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(5, 4), (3, 5), (4, 3)]]);

        do_test("dyn11", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(3, 6), (4, 6), (5, 6)]]);
        do_test("dyn3", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(5, 6), (3, 6), (4, 6)]]);
        do_test("dyn7", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(4, 6), (5, 6), (3, 6)]]);
        do_test("dyn5", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(6, 3), (6, 4), (6, 5)]]);
        do_test("dyn9", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(6, 5), (6, 3), (6, 4)]]);
        do_test("dyn1", 3, &[&[(0, 1), (1, 2), (2, 0)], &[(6, 4), (6, 5), (6, 3)]]);
    }

    #[test]
    fn test_single_phase_tx() {
        let turns = c!(5.0, 0.0);
        let zs = c!(0.0, 0.01);
        let tx = Transformer::new(
            2, 1, vec![turns, C_ONE], None, vec![C_ZERO, zs], vec![vec![(0, 1), (2, 0)]]
        ).unwrap();
        let yc = coo_mat_to_dense(&tx.y_const(), 2, 2);
        println!("{}", yc);
        assert!((yc[(0, 0)] - c!(0.0, -4.0)).abs() < 1e-6);
        assert!((yc[(0, 1)] - c!(0.0, 20.0)).abs() < 1e-6);
        assert!((yc[(1, 0)] - c!(0.0, 20.0)).abs() < 1e-6);
        assert!((yc[(1, 1)] - c!(0.0, -100.0)).abs() < 1e-6);
    }

    #[test]
    fn test_single_phase_tx_cplx_turns() {
        let turns = c!(0.0, 5.0);
        let zs = c!(0.0, 0.01);
        let tx = Transformer::new(
            2, 1, vec![turns, C_ONE], None, vec![C_ZERO, zs], vec![vec![(0, 1), (2, 0)]]
        ).unwrap();
        let yc = coo_mat_to_dense(&tx.y_const(), 2, 2);
        println!("{}", yc);
        assert!((yc[(0, 0)] - c!(0.0, -4.0)).abs() < 1e-6);
        assert!((yc[(0, 1)] - c!(-20.0, 0.0)).abs() < 1e-6);
        assert!((yc[(1, 0)] - c!(20.0, 0.0)).abs() < 1e-6);
        assert!((yc[(1, 1)] - c!(0.0, -100.0)).abs() < 1e-6);
    }

    #[test]
    fn test_single_phase_tx_z_prim() {
        let turns = c!(5.0, 0.0);
        let zs = c!(0.0, 0.01);
        let tx = Transformer::new(
            2, 1, vec![turns, C_ONE], None, vec![zs, C_ZERO], vec![vec![(0, 1), (2, 0)]]
        ).unwrap();
        let yc = coo_mat_to_dense(&tx.y_const(), 2, 2);
        println!("{}", yc);
        assert!((yc[(0, 0)] - c!(0.0, -100.0)).abs() < 1e-6);
        assert!((yc[(0, 1)] - c!(0.0, 500.0)).abs() < 1e-6);
        assert!((yc[(1, 0)] - c!(0.0, 500.0)).abs() < 1e-6);
        assert!((yc[(1, 1)] - c!(0.0, -2500.0)).abs() < 1e-6);
    }

    fn expect_three_phase_yc(yc: &Mat<c64>) {
        for ip in 0..3 {
            assert!((yc[(2*ip, 2*ip)] - c!(0.0, -4.0)).abs() < 1e-6);
            assert!((yc[(2*ip, 2*ip+1)] - c!(0.0, 20.0)).abs() < 1e-6);
            assert!((yc[(2*ip+1, 2*ip)] - c!(0.0, 20.0)).abs() < 1e-6);
            assert!((yc[(2*ip+1, 2*ip+1)] - c!(0.0, -100.0)).abs() < 1e-6);

            for j in 1..=2 {
                let jp = (ip + j) % 3;
                assert!(yc[(2*ip, 2*jp)] == C_ZERO);
                assert!(yc[(2*ip, 2*jp+1)] == C_ZERO);
                assert!(yc[(2*ip+1, 2*jp)] == C_ZERO);
                assert!(yc[(2*ip+1, 2*jp+1)] == C_ZERO);
            }
        }
    }

    #[test]
    fn test_three_phase_tx() {
        let turns = c!(5.0, 0.0);
        let zs = c!(0.0, 0.01);
        let tx = Transformer::new(
            2, 3, vec![turns, C_ONE], None, vec![C_ZERO, zs], vec![vec![(0, 1), (2, 0)]]
        ).unwrap();
        let yc = coo_mat_to_dense(&tx.y_const(), 6, 6);
        println!("{}", yc);
        expect_three_phase_yc(&yc);
    }

    /// Returns a simple test network minus the transformer
    fn base_netw() -> Network {
        let mut netw = Network::new();

        let bus_uid_0 = bus_uid(&netw);
        let bus_0 = netw.add_bus(Bus::new(&bus_uid_0, PHASES_ABCN.to_vec(), 11000.0 * SQRT_3_INV)).unwrap();
        let bts_abcn_0: Vec<BusNodeId> = PHASES_ABCN.iter().map(|p| bn!(bus_uid_0, *p)).collect();

        let bus_uid_1 = bus_uid(&netw);
        netw.add_bus(Bus::new(&bus_uid_1, PHASES_ABCN.to_vec(), 230.0)).unwrap();
        let bts_abcn_1: Vec<BusNodeId> = PHASES_ABCN.iter().map(|p| bn!(bus_uid_1, *p)).collect();

        let source_uid = elem_uid::<Source>(&netw);
        netw.add_element(Source::new_wye_slack(bus_0.borrow().v_base, 3, true)
            .element(&source_uid, bts_abcn_0, true)).unwrap();

        let load_uid = elem_uid::<Load>(&netw);
        let y = vec![c!(1e-6, 0.0); 3];
        netw.add_element(Load::new_wye(3, Some(y), None, None, true).element(&load_uid, bts_abcn_1)).unwrap();

        netw
    }

    use crate::network_solvers::solve_network;

    fn solve_with_vg(vg: &str) -> Vec<Vec<(f64, f64)>> {
        const ZS: c64 = c!(0.0, 0.01);
        let nom_turns = vec![c!(47.826, 0.0), C_ONE];
        let tx = Transformer::new_with_vector_group(3, nom_turns, None, (C_ZERO, ZS), vg).unwrap();

        let mut netw = base_netw();
        let bus0 = netw.bus_i(0).unwrap();
        let bus0_uid = bus0.borrow().uid();
        let bus1 = netw.bus_i(1).unwrap();
        let bus1_uid = bus1.borrow().uid();
        let bts = PHASES_ABCN.iter().map(|p| bn!(bus0_uid, *p))
            .chain(PHASES_ABCN.iter().map(|p| bn!(bus1_uid, *p))).collect();
        netw.add_element(tx.element("transformer", bts)).unwrap();

        let solve_result = solve_network(&mut netw, "nodal_rect", None);
        assert!(solve_result.is_ok());
        let solve_result = solve_result.unwrap();
        for status in solve_result.statuses.iter() {
            assert!(status.status.flags.contains(SolverStatusFlags::SUCCESS));
        }
        netw.buses()
            .map(
                |b| b.borrow().terms.iter().map(|t| t.v.to_polar()).collect::<Vec<(f64, f64)>>()
            ).collect::<Vec<Vec<(f64, f64)>>>()
    }

    #[test]
    fn test_solve() {
        configure_logging();
        let vs = solve_with_vg("yy0");
        for (i, vs_bus) in vs.iter().enumerate() {
            println!("bus {i}:");
            for v in vs_bus.iter() {
                println!("    {v:?}");
            }
        }
    }

    #[test]
    fn typical_z_series_one_mva_11kv() {
        // 5 % of 121 ohm = 6.05 ohm
        let z = typical_tx_z_series(1000.0e3, 11.0e3);
        assert!((z.re.hypot(z.im) - 6.05).abs() < 1e-9);
        assert!(z.im > z.re);
    }
}
