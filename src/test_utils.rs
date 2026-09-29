//! Functions to help with testing network generation and testing.

use crate::foundation::*;
use crate::generic_element::*;
use crate::grounding::*;
use crate::line::*;
use crate::load::*;
use crate::source::*;
use crate::transformer::*;
use crate::network::*;

/// Phases \[A\].
pub const PHASES_A: &[Phase; 1] = &[Phase::A];
/// Phases \[B\].
pub const PHASES_B: &[Phase; 1] = &[Phase::B];
/// Phases \[C\].
pub const PHASES_C: &[Phase; 1] = &[Phase::C];
/// Phases \[A, N\]
pub const PHASES_AN: &[Phase; 2] = &[Phase::A, Phase::N];
/// Phases \[B, N\]
pub const PHASES_BN: &[Phase; 2] = &[Phase::B, Phase::N];
/// Phases \[C, N\]
pub const PHASES_CN: &[Phase; 2] = &[Phase::C, Phase::N];
/// Phases \[A, B\]
pub const PHASES_AB: &[Phase; 2] = &[Phase::A, Phase::B];
/// Phases \[B, C\]
pub const PHASES_BC: &[Phase; 2] = &[Phase::B, Phase::C];
/// Phases \[C, A\]
pub const PHASES_CA: &[Phase; 2] = &[Phase::C, Phase::A];
/// Phases \[A, B, C\]
pub const PHASES_ABC: &[Phase; 3] = &[Phase::A, Phase::B, Phase::C];
/// Phases \[A, B, C, N\]
pub const PHASES_ABCN: &[Phase; 4] = &[Phase::A, Phase::B, Phase::C, Phase::N];

/// Returns the bus reference and all phases of the given bus_uid.
fn bus_and_phases(netw: &Network, bus_uid: &str) -> (Shared<Bus>, Vec<Phase>) {
    let bus = netw.bus(bus_uid).unwrap().clone();
    let phs = bus.borrow().terms.iter().map(|t| t.phase).collect();
    (bus, phs)
}

// Creates the UID of the next bus to be added.
fn bus_uid(netw: &Network) -> String {
    format!("bus_{}", netw.buses().count())
}

// Creates the UID of the next element to be added.
fn elem_uid<T: ElementSpec>(netw: &Network) -> String {
    format!("{}_{}", T::type_str_of(), netw.elements_of_type::<T>().count())
}

// Adds a bus with the given phases to the network.
pub fn add_bus(netw: &mut Network, phases: Vec<Phase>) -> Vec<BusPhaseId> {
    let uid = &bus_uid(netw);
    let bus = netw.add_bus(Bus::new(uid, phases, 1.0)).unwrap();
    bus.borrow().terms.iter().map(|t| bp!(bus.borrow().uid(), t.phase)).collect()
}

// Adds a constant Y generic element.
pub fn add_y_const(
    netw: &mut Network, bus_phase_0: BusPhaseId, bus_phase_1: BusPhaseId, y_const: c64
) -> Shared<Element> {
    let uid = elem_uid::<GenericElement>(netw);
    let bps = vec![bus_phase_0, bus_phase_1];
    netw.add_element(
        GenericElement {
            n_terms: 2,
            branches: vec![(0usize, 1usize)],
            y_const: vec![((0, 0), y_const)],
            ..Default::default()
        }.element(&uid, bps)
    ).unwrap()
}

// Adds a constant I generic element.
pub fn add_i_const(
    netw: &mut Network, bus_phase_0: BusPhaseId, bus_phase_1: BusPhaseId, i_const: c64
) -> Shared<Element> {
    let uid = elem_uid::<GenericElement>(netw);
    let bps = vec![bus_phase_0, bus_phase_1];
    netw.add_element(
        GenericElement {
            n_terms: 2,
            branches: vec![(0usize, 1usize)],
            i_const: vec![(0, i_const)],
            ..Default::default()
        }.element(&uid, bps)
    ).unwrap()
}

// Adds a constant S generic element.
pub fn add_s_const(
    netw: &mut Network, bus_phase_0: BusPhaseId, bus_phase_1: BusPhaseId, s_const: c64
) -> Shared<Element> {
    let uid = elem_uid::<GenericElement>(netw);
    let bps = vec![bus_phase_0, bus_phase_1];
    netw.add_element(
        GenericElement {
            n_terms: 2,
            branches: vec![(0usize, 1usize)],
            s_const: vec![(0, s_const)],
            ..Default::default()
        }.element(&uid, bps)
    ).unwrap()
}

// Adds a constant V (slack) generic element.
pub fn add_v_const(
    netw: &mut Network, bus_phase_0: BusPhaseId, bus_phase_1: BusPhaseId, v_const: f64
) -> Shared<Element> {
    let uid = elem_uid::<GenericElement>(netw);
    let bps = vec![bus_phase_0, bus_phase_1];
    netw.add_element(
        GenericElement {
            n_terms: 2,
            branches: vec![(0usize, 1usize)],
            vmag_const: vec![(0, v_const)],
            vang_const: vec![(0, 0.0)],
            ..Default::default()
        }.element(&uid, bps)
    ).unwrap()
}

// Adds a grounding.
pub fn add_ground(
    netw: &mut Network, bus_phase: BusPhaseId
) -> Shared<Element> {
    let uid = &elem_uid::<Grounding>(netw);
    netw.add_element(
        Grounding::new(1).element(uid, vec![bus_phase])
    ).unwrap()
}

// Adds a wye-connected source involving all phases of a given bus.
pub fn add_source(netw: &mut Network, bus_uid: &str) -> Shared<Element> {
    let uid = elem_uid::<Source>(netw);
    let (bus, phs) = bus_and_phases(netw, bus_uid);
    assert!(matches!(phs.last(), Some(Phase::N)));
    let bps: Vec<BusPhaseId> = phs.iter().map(|p| bp!(bus_uid, *p)).collect();
    let n_phase = phs.len() - 1;
    netw.add_element(Source::new_wye_slack(bus.borrow().v_base, n_phase).element(&uid, bps, true)).unwrap()
}

// Adds a line between two buses involving all phases and with the given z+, z0.
pub fn add_line(netw: &mut Network, bus_uid_0: &str, bus_uid_1: &str, z: c64) -> Shared<Element> {
    let uid = elem_uid::<Line>(netw);
    let (_, phs0) = bus_and_phases(netw, bus_uid_0);
    let (_, phs1) = bus_and_phases(netw, bus_uid_1);
    let n_phase = phs0.len();
    netw.add_element(
        Line::new_from_sym_seq(z, z, n_phase)
            .unwrap()
            .element(
                &uid,
                phs0.iter().map(|p| bp!(bus_uid_0, *p)).collect(),
                phs1.iter().map(|p| bp!(bus_uid_1, *p)).collect(),
            )).unwrap()
}

// Adds a transformer between two buses and grounded to an additional ground bus.
pub fn add_tx(netw: &mut Network, bus_uid_g: &str, bus_uid_p: &str, bus_uid_s: &str) -> Shared<Element> {
    let uid = elem_uid::<Transformer>(netw);

    let (_, phs0) = bus_and_phases(netw, bus_uid_g);
    assert!(phs0 == [Phase::N]);

    let (busp, phsp) = bus_and_phases(netw, bus_uid_p);
    assert!(phsp == [Phase::A]);

    let (buss, phss) = bus_and_phases(netw, bus_uid_s);
    assert!(phss == [Phase::A]);

    let bpsp: Vec<BusPhaseId> = vec![bp!(bus_uid_p, Phase::A), bp!(bus_uid_g, Phase::N)];
    let bpss: Vec<BusPhaseId> = vec![bp!(bus_uid_s, Phase::A), bp!(bus_uid_g, Phase::N)];
    let ratio = c!(busp.borrow().v_base / buss.borrow().v_base, 0.0);
    let wps = winding_pairs_from_vector_group(&"yy0".parse().unwrap(), 1).unwrap();

    netw.add_element(
        Transformer::new(
            &wps,
            vec![ratio],
            c!(1000.0, 0.0),
            c!(0.0, 0.0),
            None
        ).element(&uid, bpsp, bpss)).unwrap()
}

// Adds a wye-connected load to all phases of the given bus with constant S.
pub fn add_load(netw: &mut Network, bus_uid: &str, s_tot: c64) -> Shared<Element> {
    let uid = elem_uid::<Load>(netw);
    let (_, phs) = bus_and_phases(netw, bus_uid);
    let bps: Vec<BusPhaseId> = phs.iter().map(|p| bp!(bus_uid, *p)).collect();
    let n_phase = phs.len();
    let s = vec![s_tot / n_phase as f64; n_phase];
    netw.add_element(
        Load::new_wye(
            phs.len(),
            None,
            None,
            Some(s),
            true
        ).element(&uid, bps)
    ).unwrap()
}
