use crate::foundation::*;
use crate::generic_element::*;
use crate::grounding::*;
use crate::transformer::*;
use crate::network::*;

fn bus_id(i: usize) -> String {format!("b_{i}")}

fn elem_id_1(i: usize, prefix: &str) -> String {format!("e_{prefix}_{i}")}

fn elem_id_2(i: usize, j: usize, prefix: &str) -> String {format!("e_{prefix}_{i}_{j}")}

fn elem_id_4(i: usize, j: usize, k: usize, l: usize, prefix: &str) -> String {format!("e_{prefix}_{i}_{j}_{k}_{l}")}

pub fn add_bus(netw: &mut Network, i: usize, phase: Phase) {
    netw.add_bus(Bus::new(&bus_id(i), vec![phase], 1.0)).unwrap();
}

pub fn add_ground(netw: &mut Network, i: usize) {
    netw.add_element(Grounding::new(1).element(&elem_id_1(i, "g"), vec![bt!(&bus_id(i), Phase::N)])).unwrap();
}

pub fn add_y_const(netw: &mut Network, i: usize, j: usize, y: c64) {
    let pi = netw.bus_i(i).unwrap().terms[0].phase;
    let pj = netw.bus_i(j).unwrap().terms[0].phase;
    netw.add_element(
        (GenericElement {
            n_terms: 2,
            branches: vec![(0, 1)],
            y_const: vec![((0, 0), y)],
            ..Default::default()})
            .element(&elem_id_2(i, j, "y"), vec![bt!(&bus_id(i), pi), bt!(bus_id(j), pj)])).unwrap();
}

pub fn add_s_const(netw: &mut Network, i: usize, j: usize, s: c64) {
    let pi = netw.bus_i(i).unwrap().terms[0].phase;
    let pj = netw.bus_i(j).unwrap().terms[0].phase;
    netw.add_element(
        (GenericElement {
            n_terms: 2,
            branches: vec![(0, 1)],
            s_const: vec![(0, s)],
            ..Default::default()})
            .element(&elem_id_2(i, j, "s"), vec![bt!(&bus_id(i), pi), bt!(bus_id(j), pj)])).unwrap();
}

pub fn add_v_const(netw: &mut Network, i: usize, j: usize, vmag: f64) {
    let pi = netw.bus_i(i).unwrap().terms[0].phase;
    let pj = netw.bus_i(j).unwrap().terms[0].phase;
    netw.add_element(
        (GenericElement {
            n_terms: 2,
            branches: vec![(0, 1)],
            vmag_const: vec![(0, vmag)],
            vang_const: vec![(0, 0.0)],
            ..Default::default()})
            .element(&elem_id_2(i, j, "v"), vec![bt!(&bus_id(i), pi), bt!(bus_id(j), pj)])).unwrap();
}

pub fn add_tx(netw: &mut Network, ij_p: (usize, usize), ij_s: (usize, usize), ratio: c64) {
    let pip = netw.bus_i(ij_p.0).unwrap().terms[0].phase;
    let pjp = netw.bus_i(ij_p.1).unwrap().terms[0].phase;
    let pis = netw.bus_i(ij_s.0).unwrap().terms[0].phase;
    let pjs = netw.bus_i(ij_s.1).unwrap().terms[0].phase;
    netw.add_element(
        Transformer::new(
            &[WindingPair {p: (1, 0), s: (1, 0)}],
            vec![ratio],
            c!(1000.0, 0.0),
            c!(0.0, 0.0),
            None
        ).element(
            &elem_id_4(ij_p.0, ij_p.1, ij_s.0, ij_s.1, "t"),
            &[bt!(&bus_id(ij_p.0), pip), bt!(bus_id(ij_p.1), pjp)],
            &[bt!(bus_id(ij_s.0), pis), bt!(bus_id(ij_s.1), pjs)]
        )
    ).unwrap();
}
