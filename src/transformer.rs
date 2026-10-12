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
    /// For a transformer primary winding at tap i, the tapped number of turns is nom_turns * (1 + i *  factor).
    /// For a transformer secondary winding at tap i, the tapped ratio is nom_turns / (1 + i *  factor)
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
    #[error("bad winding groups {0:?}.")] BadWindingGroups(Vec<Vec<(usize, usize)>>),
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
/// Currently, we allow only two-windings transformers, with arbitrary numbers of phases.
///
/// A transformer model is based on a primitive model, where we only care about the windings. The primitive model
/// has `n_sides` sides (which is, e.g., 2 for a 2-winding transformer and 3 for a 3-winding transformer), and
/// `n_phases` phases, which is the number of phases.
///
/// There are `n_sides x n_phases` windings, ordered as, e.g. `[PA, SA, PB, SB, PC, SC]` for a 2-winding 3-phase
/// transformer, where `P` is primary, `S` is secondary, and `A, B, C` are the phases. This means, for example, that the
/// primitive branch impedance matrix is block diagonal matrix organised as three 2x2 blocks. It depends only on the
/// following: `n_sides`, `n_phases`, the leakage impedance of each winding, and the relative turns ratios
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
    // In other words, the primitive model is a list of winding groups (one group per phase), plus the associated
    // parameters.
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

    /// In transformers, the branches are [(start_terminal_idx, end_terminal_idx)] indexed by winding.
    pub branches: Vec<(usize, usize)>,
}

impl Transformer {
    /// Creates a new [`Transformer`].
    pub fn new(
        n_sides: usize,
        n_phases: usize,
        nom_turns: Vec<c64>,
        taps: Option<TransformerTaps>,
        z_series: Vec<c64>,
        winding_groups: Vec<Vec<(usize, usize)>>,
    ) -> Result<Self, TransformerError> {

        let mut terms: Vec<usize> = winding_groups.iter().flatten().flat_map(|w| [w.0, w.1]).collect();
        terms.sort();
        terms.dedup();
        if terms != (0..terms.len()).collect::<Vec<usize>>() {
            return Err(TransformerError::BadWindingGroups(winding_groups))
        }

        let branches: Vec<(usize, usize)> = winding_groups.into_iter().flatten().collect();

        Ok(Transformer {n_sides, n_phases, nom_turns, taps, z_series, n_terms: terms.len(), branches})
    }

    /// Creates a new two-winding [`Transformer`] with the given vector group, number of phases and other data.
    ///
    /// turns_ratio is np/ns. vector_group_str is all lowercase, with the primary always assumed to come first.
    pub fn new_with_vector_group(
        n_phases: usize,
        nom_turns_ratio: c64,
        taps: Option<TransformerTaps>,
        z_series: (c64, c64),
        vector_group_str: &str
    ) -> Result<Transformer, TransformerError> {
        let vg: VectorGroup = VectorGroup::from_str(vector_group_str)?;
        let winding_groups = winding_groups_from_vector_group(&vg, n_phases)?;
        let nom_turns = vec![nom_turns_ratio, C_ONE];
        let zs: Vec<c64> = vec![z_series.0, z_series.1];
        Transformer::new(2, n_phases, nom_turns, taps, zs, winding_groups)
    }

    /// Wraps a transformer in an [`Element`], with the given bus/phase connections.
    ///
    /// the [`BusTermId`]s are grouped into winding groups, e.g. [[PA, SA], [PB, SB], [PC, SC]].
    pub fn element(
        self, id: &str,
        bus_term_id_groups: Vec<Vec<BusTermId>>,
    ) -> Element {
        self.element_generic(id, bus_term_id_groups.into_iter().flatten().collect())
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
        self.branches.clone()
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

/// Given a vector group and a phase count, partition the windings into coupled groups (pairs in this case).
fn winding_groups_from_vector_group(vg: &VectorGroup, n_phases: usize) -> Result<Vec<Vec<(usize, usize)>>, String> {
    if vg.winding_types.len() != 2 {
        return Err(String::from(
            "only two-winding vector groups are supported for now.",
        ));
    }

    if vg.winding_types.contains(&WindingType::Z) {
        return Err(String::from("zigzag isn't supported yet."));
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

    if n_phases == 1 {
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

        Ok(vec![vec![single_phase_winding(primary_type), single_phase_winding(secondary_type)]])
    } else if n_phases == 3 {
        fn natural_winding(t: WindingType, i: usize, n: usize) -> (usize, usize) {
            match t {
                WindingType::Y => (i, n),
                WindingType::D => (i, (i + 1) % n),
                WindingType::Z => unreachable!("zigzag is rejected above"),
            }
        }

        for flip in [false, true] {
            for r in 0..n_phases {
                let rotation_shift = 4 * ((n_phases - r) % n_phases) as i32;
                let flip_shift = if flip { 6 } else { 0 };
                let clock = (base_clock + rotation_shift + flip_shift).rem_euclid(12);

                if clock == target_clock {
                    let pairs = (0..n_phases)
                        .map(|leg| {
                            let secondary_index = (leg + r) % n_phases;
                            let mut s = natural_winding(secondary_type, secondary_index, n_phases);
                            if flip {
                                s = (s.1, s.0);
                            }
                            vec![natural_winding(primary_type, leg, n_phases), s]
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
    use crate::load::*;
    use crate::source::*;
    use crate::test_utils::*;
    use super::*;

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

        let bus0 = add_bus(&mut netw, PHASES_ABCN.to_vec());
        let bus0_uid = bus0.borrow().uid();
        let bts_abcn_0: Vec<BusTermId> = PHASES_ABCN.iter().map(|p| bt!(bus0_uid, *p)).collect();

        let bus1 = add_bus(&mut netw, PHASES_ABCN.to_vec());
        let bus1_uid = bus1.borrow().uid();
        let bts_abcn_1: Vec<BusTermId> = PHASES_ABCN.iter().map(|p| bt!(bus1_uid, *p)).collect();

        let source_uid = elem_uid::<Source>(&netw);
        netw.add_element(Source::new_wye_slack(bus0.borrow().v_base, 3)
            .element(&source_uid, bts_abcn_0, true)).unwrap();

        let load_uid = elem_uid::<Source>(&netw);
        let s = vec![c!(1e-3, 0.0); 3];
        netw.add_element(Load::new_wye(3, None, None, Some(s), true)
            .element(&load_uid, bts_abcn_1)).unwrap();

        netw
    }

    #[test]
    fn test_three_phase_dyn11_tx() {
        let turns = c!(5.0, 0.0);
        let zs = c!(0.0, 0.01);
        let tx = Transformer::new_with_vector_group(
            3, turns, None, (C_ZERO, zs), "dyn11"
        ).unwrap();
        let yc = coo_mat_to_dense(&tx.y_const(), 6, 6);
        println!("{}", yc);
        expect_three_phase_yc(&yc);
        let mut netw = base_netw();
        let bus0 = netw.bus_i(0).unwrap();
        let bus0_uid = bus0.borrow().uid();
        let bus1 = netw.bus_i(1).unwrap();
        let bus1_uid = bus1.borrow().uid();
        let bts_abc_0 = PHASES_ABC.iter().map(|p| bt!(bus0_uid, *p)).collect();
        let bts_abcn_1 = PHASES_ABCN.iter().map(|p| bt!(bus1_uid, *p)).collect();
        let e = netw.add_element(tx.element("transformer", vec![bts_abc_0, bts_abcn_1])).unwrap();
        dbg!(&e.borrow().branches);
    }

    #[test]
    fn typical_z_series_one_mva_11kv() {
        // 5 % of 121 ohm = 6.05 ohm
        let z = typical_tx_z_series(1000.0e3, 11.0e3);
        assert!((z.re.hypot(z.im) - 6.05).abs() < 1e-9);
        assert!(z.im > z.re);
    }
}
