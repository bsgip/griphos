use log::warn;
use sprs::vec;
use std::fs::File;
use std::io::{BufReader, Read};
use serde::Deserialize;
use serde::de::{DeserializeSeed, Deserializer};
use std::collections::HashMap;

use crate::connector::*;
use crate::e_json_generated_types::{self as ejt, TwoNumberArray, TwoPositiveNumberArray, TapSide};
use crate::foundation::*;
use crate::generator::*;
use crate::grounding::*;
use crate::line::*;
use crate::load::*;
use crate::network::*;
use crate::transformer::*;

// Stores state for deserialization.
struct EJsonParser<'a> {
    ejson_phase_map: &'a HashMap<String, Phase>,
}

// TODO: much cleaning up is needed!

const V_FACTOR: f64 = 1e-3; // Convert e-JSON V to internal kV
const I_FACTOR: f64 = 1e-3; // Convert e-JSON A to internal kA
const S_FACTOR: f64 = 1e-6; // Convert e-JSON W to internal MW
const Z_FACTOR: f64 = 1.0; // Convert e-JSON Ohm to internal Ohm
const Y_FACTOR: f64 = 1.0; // Convert e-JSON S to internal S

#[derive(Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
enum Component {
    Connector(ejt::Connector),
    Gen(ejt::Gen),
    Infeeder(ejt::Infeeder),
    Line(ejt::Line),
    Load(ejt::Load),
    Node(ejt::Node),
    Transformer(ejt::Transformer),
}

#[derive(Deserialize)]
struct Root {
    ejson_version: Option<String>,
    id: Option<String>,
    voltage_type: String,
    user_data: Option<Box<serde_json::value::RawValue>>,
    components: Vec<Component>,
}

/// Parse the e-JSON file at path, using the specified e-JSON -> griphos phase map.
///
/// # Examples
///
/// ```
/// # use griphos::{parse_ejson_file, ejson_phase_map_abcng};
/// let netw = parse_ejson_file("test_data/netw-i1l1s1.json", &ejson_phase_map_abcng());
/// ```
pub fn parse_ejson_file(path: &str, ejson_phase_map: &HashMap<String, Phase>) -> serde_json::Result<Network> {
    let file = File::open(path).expect("Failed to open test file");
    let reader = BufReader::new(file);
    parse_ejson(reader, ejson_phase_map)
}

/// Parse the e-JSON file like object, using the specified e-JSON -> griphos phase map.
///
/// # Examples
///
/// ```
/// # use griphos::{parse_ejson, ejson_phase_map_abcng};
/// # use std::io::BufReader;
/// # use std::fs::File;
/// let file = File::open("test_data/netw-i1l1s1.json").expect("Failed to open test file");
/// let reader = BufReader::new(file);
/// let netw = parse_ejson(reader, &ejson_phase_map_abcng());
/// ```
pub fn parse_ejson<R: Read>(reader: R, ejson_phase_map: &HashMap<String, Phase>) -> serde_json::Result<Network> {
    let mut json_value: serde_json::Value = serde_json::from_reader(reader)?;
    fix_transformers_kludge(&mut json_value);

    let parser = EJsonParser { ejson_phase_map };
    parser.deserialize(json_value)
}

fn fix_transformers_kludge(json_value: &mut serde_json::Value) {
    if let Some(obj) = json_value.as_object_mut()
        && let Some(serde_json::Value::Array(components)) = obj.get_mut("components")
    {
        for component in components {
            if let Some(comp_obj) = component.as_object_mut() &&
                matches!(comp_obj.get("type"), Some(serde_json::Value::String(s)) if s == "Transformer")
            {
                let z_p = comp_obj.get("z_p").cloned();
                let z_s = comp_obj.get("z_s").cloned();
                let user_data = comp_obj.entry("user_data")
                    .or_insert_with(|| serde_json::Value::Object(Default::default()));
                if let Some(user_data_obj) = user_data.as_object_mut() {
                    user_data_obj.entry("z_p".to_string()).or_insert(z_p.unwrap_or(serde_json::Value::Null));
                    user_data_obj.entry("z_s".to_string()).or_insert(z_s.unwrap_or(serde_json::Value::Null));
                }
            }
        }
    }
}

/// e-JSON to griphos phase mapping, A->A, B->B, C->C, N->N, G->N and lowercase variants.
pub fn ejson_phase_map_abcng() -> HashMap<String, Phase> {
    HashMap::from_iter(vec![
        ("A".to_string(), Phase::A),
        ("B".to_string(), Phase::B),
        ("C".to_string(), Phase::C),
        ("N".to_string(), Phase::N),
        ("G".to_string(), Phase::N),
        ("a".to_string(), Phase::A),
        ("b".to_string(), Phase::B),
        ("c".to_string(), Phase::C),
        ("n".to_string(), Phase::N),
        ("g".to_string(), Phase::N),
    ])
}

/// e-JSON to griphos phase mapping, R->A, W->B, B->C, N->N, G->N and lowercase variants.
pub fn ejson_phase_map_rwbng() -> HashMap<String, Phase> {
    HashMap::from_iter(vec![
        ("R".to_string(), Phase::A),
        ("W".to_string(), Phase::B),
        ("B".to_string(), Phase::C),
        ("N".to_string(), Phase::N),
        ("G".to_string(), Phase::N),
        ("r".to_string(), Phase::A),
        ("w".to_string(), Phase::B),
        ("b".to_string(), Phase::C),
        ("n".to_string(), Phase::N),
        ("g".to_string(), Phase::N),
    ])
}

impl<'de> DeserializeSeed<'de> for EJsonParser<'de> {
    type Value = Network;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
        let root = Root::deserialize(deserializer)?;

        let network = convert_to_network(root, self.ejson_phase_map)
            .map_err(serde::de::Error::custom)?;

        Ok(network)
    }
}

fn convert_to_network(schema: Root, ejson_phase_map: &HashMap<String, Phase>) -> Result<Network, String> {
    let mut netw = Network::new();

    // Add an explicit ground node.
    let _ = netw.add_bus(Bus::new("G", vec![Phase::N], 1.0));

    // Make sure it's explicitly grounded.
    let _ = netw.add_element(Grounding::new(1).element("G", vec![bt!("G", Phase::N)])).unwrap();

    let vt = schema.voltage_type; // TODO
    let comps = &schema.components;
    for comp in comps {
        match comp {
            Component::Node(ejt::Node{
                id, lat_long: _, phs, type_: _, user_data: _, v, v_base, xy: _
            }) => {
                handle_node(&mut netw, ejson_phase_map, id, phs, v_base.0, v)?
            },
            Component::Infeeder(ejt::Infeeder{
                cons, i_max, id, in_service, type_, user_data, v_setpoint
            }) => {
                handle_infeeder(&mut netw, ejson_phase_map, id, cons, in_service, v_setpoint)?
            },
            Component::Gen(ejt::Gen{
                cons, cost, fixed_voltage, id, in_service, is_reference, p_max, p_min, q_max, q_min, type_, user_data,
                v_setpoint, wiring
            }) => {
                warn!("Unimplemented ejson {type_}. Ignoring.");
            },
            Component::Line(ejt::Line::Variant0{
                b_chg, cons, i_max, id, in_service, length, type_: _, user_data: _, z, z0
            }) => {
                handle_line_variant_0(&mut netw, ejson_phase_map, id, cons, in_service, i_max, z, z0, b_chg, *length);
            },
            Component::Line(ejt::Line::Variant1{
                cons, i_max, id, in_service, length, type_, user_data, y_bus
            }) => {
                warn!("Unimplemented ejson {type_}. Ignoring.");
            },
            Component::Transformer(transformer @ ejt::Transformer::Variant0 {..}) => {
                handle_transformer(&mut netw, ejson_phase_map, transformer).unwrap();
            },
            Component::Transformer(transformer @ ejt::Transformer::Variant1 {..}) => {
                handle_transformer(&mut netw, ejson_phase_map, transformer).unwrap();
            },
            Component::Connector(ejt::Connector{
                cons, i_max, id, in_service, switch_state,
                type_,
                user_data,
            }) => {
                handle_connector(&mut netw, ejson_phase_map, id, cons, in_service, i_max, switch_state).unwrap();
            },
            Component::Load(ejt::Load{
                cons, id, s, type_: _, user_data: _, wiring, in_service,
            }) => {
                // KLUDGE: typify is messing up due to the fact that s is optional but has min_items 1.
                let s: Option<Vec<ejt::ComplexNumber>> = if s.is_empty() {None} else {Some(s.clone())};
                handle_load(&mut netw, ejson_phase_map, id, cons, wiring, in_service, &s).unwrap();
            },
        };
    }
    Ok(netw)
}

fn convert_complex(c: &ejt::ComplexNumber, factor: f64) -> c64 {
    c64 {re: c[0] * factor, im: c[1] * factor}
}

fn convert_complexes<'a>(cs: impl Iterator<Item = &'a ejt::ComplexNumber>, factor: f64) -> Vec<c64> {
    cs.map(|c| convert_complex(c, factor)).collect()
}

fn convert_phases(phs: &ejt::Phs, ejson_phase_map: &HashMap<String, Phase>) -> Vec<Phase> {
    phs.iter().map(|ph| ejson_phase_map[ph]).collect()
}

fn convert_in_service(in_service: &Option<ejt::InService>) -> bool {
    match in_service {
        Some(x) => x.0,
        None => true,
    }
}

fn get_bts(
    cons: &[ejt::Connection],
    ejson_phase_map: &HashMap<String, Phase>
) -> Vec<Vec<BusTermId>> {
    let bus_ids: Vec<&String> = cons.iter().map(|x| {&x.node}).collect();
    let phs: Vec<Vec<Phase>> = cons.iter().map(|x| {convert_phases(&x.phs, ejson_phase_map)}).collect();
    bus_ids.iter().cloned().zip(phs.iter())
        .map(|(b, ps)| ps.iter().cloned().map(|p| bt!(b.clone(), p)).collect()).collect()
}

fn flatten_bts(bts: &[Vec<BusTermId>]) -> Vec<BusTermId> {
    bts.iter().flatten().cloned().collect()
}

fn handle_node(
    netw: &mut Network, ejson_phase_map: &HashMap<String, Phase>,
    id: &str, phs: &ejt::Phs, v_base: f64, maybe_v: &Option<ejt::ComplexArray>,
) -> Result<(), String> {
    let phs = convert_phases(phs, ejson_phase_map);
    let bus = netw.add_bus(Bus::new(id, phs, v_base * V_FACTOR))?;
    if let Some(v) = maybe_v {
        bus.borrow_mut().set_vs(convert_complexes(v.iter(), V_FACTOR).into_iter());
    }
    Ok(())
}

fn handle_infeeder(
    netw: &mut Network, ejson_phase_map: &HashMap<String, Phase>,
    id: &str, cons: &ejt::OneConnection, in_service: &Option<ejt::InService>,
    v_setpoint: &ejt::PositiveNumber,
) -> Result<(), String> {
    let con = &cons[0];
    let bus_id: &str = &con.node;
    let phs = convert_phases(&con.phs, ejson_phase_map);
    let mut bts: Vec<BusTermId> = Vec::with_capacity(phs.len() + 1);
    phs.iter().for_each(|p| bts.push(bt!(bus_id, *p)));
    bts.push(bt!("G".to_string(), Phase::N));
    let in_service = convert_in_service(in_service);
    let vmag = v_setpoint.0 * V_FACTOR;
    let elem = netw.add_element(
        Generator::new(
            bts.len(), (0..bts.len()-1).map(|i| (i, bts.len()-1)).collect(),
            GeneratorControl::new_wye_slack_with_std_phase_angles(vmag, &phs)
        ).element(id, bts)).unwrap();
    elem.borrow_mut().is_in_service = in_service;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_line_variant_0(
    netw: &mut Network, ejson_phase_map: &HashMap<String, Phase>,
    id: &str, cons: &ejt::TwoConnections, in_service: &Option<ejt::InService>, i_max: &Option<ejt::IMax>,
    z: &ejt::ComplexNumber, z0: &ejt::ComplexNumber, b_chg: &Option<f64>, length: f64
) -> Result<(), String> {
    let bts = get_bts(&cons.0, ejson_phase_map);
    let [bts0, bts1] = bts.as_slice() else {panic!("Expected 2 elements")};
    let n_phase = bts0.len();
    let in_service = convert_in_service(in_service);
    let z = length * convert_complex(z, Z_FACTOR);
    let z0= length * convert_complex(z0, Z_FACTOR);
    if !b_chg.is_none() {
        warn!("Unimplemented b_chg for e-JSON line. Ignoring.");
    }
    let elem = netw.add_element(Line::new_from_sym_seq(z, z0, n_phase).unwrap().element(id, bts0.clone(), bts1.clone())).unwrap();
    elem.borrow_mut().is_in_service = in_service;
    Ok(())
}

fn ejson_complex(value: &serde_json::Value) -> Option<c64> {
    if value.is_null() {
        None
    } else {
        value.as_array().map(|arr| c!(arr[0].as_f64().unwrap(), arr[1].as_f64().unwrap()))
    }
}

fn handle_transformer(
    netw: &mut Network, ejson_phase_map: &HashMap<String, Phase>,
    transformer: &ejt::Transformer
) -> Result<(), String> {
    let (
        id, mut bts, vector_group, n_winding_pairs, nom_turns_ratio, z0_p, z0_s, tap_side, tap_factor, tap_range, taps,
        user_data
    ) = match transformer {
        ejt::Transformer::Variant0 {
            id, cons, vector_group, n_winding_pairs, nom_turns_ratio, z0_p, z0_s, user_data,
            tap_side, tap_factor, tap_range, taps, ..
        } => {
            (
                id, get_bts(&cons.0, ejson_phase_map), vector_group, n_winding_pairs, nom_turns_ratio, z0_p, z0_s,
                tap_side, tap_factor, tap_range, taps, user_data.as_ref().unwrap(),
            )
        },
        ejt::Transformer::Variant1 {
            id, cons, vector_group, n_winding_pairs, nom_turns_ratio, z0_p, z0_s, user_data,
            tap_side, tap_factor, tap_range, taps, ..
        } => {
            (
                id, get_bts(&cons.0, ejson_phase_map), vector_group, n_winding_pairs, nom_turns_ratio, z0_p, z0_s,
                tap_side, tap_factor, tap_range, taps, user_data.as_ref().unwrap()
            )
        },
    };

    let n_winding_pairs = n_winding_pairs.as_ref().expect("Currenly n_winding_pairs is required").0 as usize;

    // typify messes up z_p and z_s, so we're using a KLUDGE involving user_data.
    let z_p: Option<c64> = ejson_complex(user_data.get("z_p").unwrap());
    let z_s: Option<c64> = ejson_complex(user_data.get("z_s").unwrap());
    assert!(
        (z_p.is_none() || z_p.iter().all(|x| *x == c!(0.0, 0.0)))
        && z_s.is_some(), "Currently require z_s and not z_p in Transformer components");
    let z_s = z_s.unwrap();

    assert!(z0_p.is_none(), "Currently can't handle z0_p in Transformer components");
    let y_shunt = match z0_s {Some(c) => 1.0 / convert_complex(c, Y_FACTOR), None => c!(0.0, 0.0)};

    let vector_group: VectorGroup = vector_group.parse()?;

    let winding_pairs = winding_pairs_from_vector_group(&vector_group, n_winding_pairs)?;

    for (wt, btsi) in vector_group.winding_types.iter().zip(bts.iter_mut()) {
        // For every Y winding, insert an extra grounded neutral.
        if matches!(wt, WindingType::Y) {
            btsi.push(bt!("G", Phase::N));
        }
    }

    let turns_ratio: Vec<c64> = vec![convert_complex(nom_turns_ratio, 1.0); n_winding_pairs];

    let y_series = 1.0 / z_s;

    let tx_taps = if let Some(tap_side) = tap_side && let Some(factor) = tap_factor && let Some(tap_range) = tap_range {
        let side = match tap_side {
            TapSide::Primary => TransformerSide::P,
            TapSide::Secondary => TransformerSide::S,
        };
        let min_tap = tap_range[0];
        let max_tap = tap_range[1];
        Some(TransformerTaps::new(side, *factor, min_tap as i32, max_tap as i32, taps.len()))
    } else {
        None
    };

    netw.add_element(Transformer::new(&winding_pairs, turns_ratio, y_series, y_shunt, tx_taps)
        .element(id, bts[0].as_ref(), bts[1].as_ref())).unwrap();

    Ok(())
}

fn handle_connector(
    netw: &mut Network, ejson_phase_map: &HashMap<String, Phase>,
    id: &str, cons: &ejt::TwoOrMoreConnections, in_service: &Option<ejt::InService>, i_max: &Option<ejt::IMax>,
    switch_state: &Option<ejt::ConnectorSwitchState>
) -> Result<(), String> {
    let bts = get_bts(&cons.0, ejson_phase_map);
    let bts_flat = flatten_bts(&bts);
    let switch_state = if let Some(x) = switch_state {
        match x {
            ejt::ConnectorSwitchState::NoSwitch => SwitchState::NoSwitch,
            ejt::ConnectorSwitchState::Closed => SwitchState::Closed,
            ejt::ConnectorSwitchState::Open => SwitchState::Open,
        }
    } else {SwitchState::NoSwitch};
    let elem = netw.add_element(Connector::new(bts.len(), switch_state).element(id, bts_flat)).unwrap();
    elem.borrow_mut().is_in_service = convert_in_service(in_service);
    Ok(())
}

fn handle_load(
    netw: &mut Network, ejson_phase_map: &HashMap<String, Phase>,
    id: &str, cons: &ejt::OneConnection,
    wiring: &ejt::Wiring, in_service: &Option<ejt::InService>, s: &Option<Vec<ejt::ComplexNumber>>
) -> Result<(), String> {
    let mut bts = get_bts(&cons.0, ejson_phase_map)[0].clone();
    let s: Option<Vec<c64>> = s.clone().map(|v| v.iter().map(|c| convert_complex(c, S_FACTOR)).collect());
    let spec = match *wiring {
        ejt::Wiring::Delta => {Load::new_delta(bts.len(), None, None, s)},
        ejt::Wiring::Wye => {
            let n_term = bts.len();
            bts.insert(0, bt!("G", Phase::N));
            Load::new_wye(n_term, None, None, s, true)
        },
    };
    let elem = netw.add_element(spec.element(id, bts)).unwrap();
    elem.borrow_mut().is_in_service = convert_in_service(in_service);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ejson_file() {
        // Setup: create a phase map (adjust based on your actual Phase type)
        let ejson_phase_map = ejson_phase_map_abcng();

        let mut network = parse_ejson_file("test_data/netw-i1l1s1.json", &ejson_phase_map)
            .expect("Failed to parse e-JSON");
        println!("{}", Wrap(&network));

        // Add assertions based on what you expect in the network
        // For example:
        // assert_eq!(network.nodes.len(), 5);
        // assert_eq!(network.name, "TestNetwork");
    }
}
