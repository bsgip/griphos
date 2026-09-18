use crate::foundation::*;
use crate::generic_element::*;
use crate::generator::*;
use crate::grounding::*;
use crate::line::*;
use crate::load::*;
use crate::transformer::*;
use crate::network::*;

pub const PHASES_A: &[Phase; 1] = &[Phase::A];
pub const PHASES_B: &[Phase; 1] = &[Phase::B];
pub const PHASES_C: &[Phase; 1] = &[Phase::C];
pub const PHASES_AN: &[Phase; 2] = &[Phase::A, Phase::N];
pub const PHASES_BN: &[Phase; 2] = &[Phase::B, Phase::N];
pub const PHASES_CN: &[Phase; 2] = &[Phase::C, Phase::N];
pub const PHASES_AB: &[Phase; 2] = &[Phase::A, Phase::B];
pub const PHASES_BC: &[Phase; 2] = &[Phase::B, Phase::C];
pub const PHASES_CA: &[Phase; 2] = &[Phase::C, Phase::A];
pub const PHASES_ABC: &[Phase; 3] = &[Phase::A, Phase::B, Phase::C];
pub const PHASES_ABCN: &[Phase; 4] = &[Phase::A, Phase::B, Phase::C, Phase::N];

fn bus_and_phases(netw: &Network, bus_id: &str) -> (Shared<Bus>, Vec<Phase>) {
    let bus = netw.bus(bus_id).unwrap().clone();
    let phs = bus.borrow().terms.iter().map(|t| t.phase).collect();
    (bus, phs)
}

fn bus_uid(netw: &Network) -> String {
    format!("bus_{}", netw.buses().count())
}

fn elem_uid<T: ElementSpec>(netw: &Network) -> String {
    format!("{}_{}", T::type_str_of(), netw.elements_filtered::<T>().count())
}

pub fn add_bus(netw: &mut Network, phases: Vec<Phase>) -> Vec<BusTermId> {
    let uid = &bus_uid(netw);
    let bus = netw.add_bus(Bus::new(uid, phases, 1.0)).unwrap();
    bus.borrow().terms.iter().map(|t| bt!(bus.borrow().uid(), t.phase)).collect()
}

pub fn add_y_const(
    netw: &mut Network, bus_term_0: BusTermId, bus_term_1: BusTermId, y_const: c64
) -> Shared<Element> {
    let uid = elem_uid::<GenericElement>(netw);
    let bts = vec![bus_term_0, bus_term_1];
    netw.add_element(
        GenericElement {
            n_terms: 2,
            branches: vec![(0usize, 1usize)],
            y_const: vec![((0, 0), y_const)],
            ..Default::default()
        }.element(&uid, bts)
    ).unwrap()
}

pub fn add_i_const(
    netw: &mut Network, bus_term_0: BusTermId, bus_term_1: BusTermId, i_const: c64
) -> Shared<Element> {
    let uid = elem_uid::<GenericElement>(netw);
    let bts = vec![bus_term_0, bus_term_1];
    netw.add_element(
        GenericElement {
            n_terms: 2,
            branches: vec![(0usize, 1usize)],
            i_const: vec![(0, i_const)],
            ..Default::default()
        }.element(&uid, bts)
    ).unwrap()
}

pub fn add_s_const(
    netw: &mut Network, bus_term_0: BusTermId, bus_term_1: BusTermId, s_const: c64
) -> Shared<Element> {
    let uid = elem_uid::<GenericElement>(netw);
    let bts = vec![bus_term_0, bus_term_1];
    netw.add_element(
        GenericElement {
            n_terms: 2,
            branches: vec![(0usize, 1usize)],
            s_const: vec![(0, s_const)],
            ..Default::default()
        }.element(&uid, bts)
    ).unwrap()
}

pub fn add_v_const(
    netw: &mut Network, bus_term_0: BusTermId, bus_term_1: BusTermId, v_const: f64
) -> Shared<Element> {
    let uid = elem_uid::<GenericElement>(netw);
    let bts = vec![bus_term_0, bus_term_1];
    netw.add_element(
        GenericElement {
            n_terms: 2,
            branches: vec![(0usize, 1usize)],
            vmag_const: vec![(0, v_const)],
            vang_const: vec![(0, 0.0)],
            ..Default::default()
        }.element(&uid, bts)
    ).unwrap()
}

pub fn add_ground(
    netw: &mut Network, bus_term: BusTermId
) -> Shared<Element> {
    let uid = &elem_uid::<Grounding>(netw);
    netw.add_element(
        Grounding::new(1).element(uid, vec![bus_term])
    ).unwrap()
}

pub fn add_generator(netw: &mut Network, bus_id: &str) -> Shared<Element> {
    let uid = elem_uid::<Generator>(netw);
    let (bus, phs) = bus_and_phases(netw, bus_id);
    let bts: Vec<BusTermId> = phs.iter().map(|p| bt!(bus_id, *p)).collect();
    let n_phase = phs.len();
    let branches = wye_connection(n_phase).unwrap();
    let live_phs: Vec<Phase> = phs.iter().filter(|p| !matches!(p, Phase::N)).cloned().collect();
    netw.add_element(
        Generator::new(
            phs.len(),
            branches,
            GeneratorControl::new_wye_slack_with_std_phase_angles(bus.borrow().v_base, &live_phs)
        ).element(&uid, bts)
    ).unwrap()
}

pub fn add_line(netw: &mut Network, bus_id_0: &str, bus_id_1: &str, z: c64) -> Shared<Element> {
    let uid = elem_uid::<Line>(netw);
    let (_, phs0) = bus_and_phases(netw, bus_id_0);
    let (_, phs1) = bus_and_phases(netw, bus_id_1);
    let n_phase = phs0.len();
    netw.add_element(
        Line::new_from_sym_seq(z, z, n_phase)
            .unwrap()
            .element(
                &uid,
                phs0.iter().map(|p| bt!(bus_id_0, *p)).collect(),
                phs1.iter().map(|p| bt!(bus_id_1, *p)).collect(),
            )).unwrap()
}

pub fn add_tx(netw: &mut Network, bus_id_g: &str, bus_id_p: &str, bus_id_s: &str) -> Shared<Element> {
    let uid = elem_uid::<Transformer>(netw);

    let (_, phs0) = bus_and_phases(netw, bus_id_g);
    assert!(phs0 == [Phase::N]);

    let (busp, phsp) = bus_and_phases(netw, bus_id_p);
    assert!(phsp == [Phase::A]);

    let (buss, phss) = bus_and_phases(netw, bus_id_s);
    assert!(phss == [Phase::A]);

    let btsp: Vec<BusTermId> = vec![bt!(bus_id_p, Phase::A), bt!(bus_id_g, Phase::N)];
    let btss: Vec<BusTermId> = vec![bt!(bus_id_s, Phase::A), bt!(bus_id_g, Phase::N)];
    let ratio = c!(busp.borrow().v_base / buss.borrow().v_base, 0.0);
    let wps = winding_pairs_from_vector_group(&"yy0".parse().unwrap(), 1).unwrap();

    netw.add_element(
        Transformer::new(
            &wps,
            vec![ratio],
            c!(1000.0, 0.0),
            c!(0.0, 0.0),
            None
        ).element(&uid, &btsp, &btss)).unwrap()
}

pub fn add_load(netw: &mut Network, bus_id: &str, s_tot: c64) -> Shared<Element> {
    let uid = elem_uid::<Load>(netw);
    let (_, phs) = bus_and_phases(netw, bus_id);
    let bts: Vec<BusTermId> = phs.iter().map(|p| bt!(bus_id, *p)).collect();
    let n_phase = phs.len();
    let s = vec![s_tot / n_phase as f64; n_phase];
    netw.add_element(
        Load::new_wye(
            phs.len(),
            None,
            None,
            Some(s),
            true
        ).element(&uid, bts)
    ).unwrap()
}
