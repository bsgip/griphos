//! Functions to generate simple networks for testing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::foundation::*;
use crate::network::*;
use crate::e_json::*;
use crate::test_utils::*;

/// Returns the path to the test_data directory.
pub fn test_data_path(filename: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data").join(filename)
}

/// Returns a network by parsing the e-JSON file with the given filename.
///
/// The file is assumed to live in the test_data directory.
pub fn test_netw_ejson(filename: &str, ejson_phase_map: Option<HashMap<String, Phase>>) -> Network {
    let ejson_phase_map = ejson_phase_map.unwrap_or_else(ejson_phase_map_abcng);
    parse_ejson_file(test_data_path(filename).to_str().unwrap(), &ejson_phase_map).unwrap()
}

/// Returns a simple test network with a single constant Y connection.
pub fn test_netw_y(y: c64) -> Network {
    let mut netw = Network::new();

    let bus_0 = add_bus(&mut netw, vec![Phase::N]);
    let node_0 = nd_id!(&bus_0.borrow().uid(), Phase::N);

    let bus_1 = add_bus(&mut netw, vec![Phase::A]);
    let node_1 = nd_id!(&bus_1.borrow().uid(), Phase::A);

    add_ground(&mut netw, node_0.clone());
    add_v_const(&mut netw, node_1.clone(), node_0.clone(), 1.0);
    add_y_const(&mut netw, node_0.clone(), node_1.clone(), y);

    netw
}

/// Returns a simple test network with a single constant S connection.
pub fn test_netw_s(s: c64) -> Network {
    let mut netw = Network::new();

    let bus_0 = add_bus(&mut netw, vec![Phase::N]);
    let node_0 = nd_id!(&bus_0.borrow().uid(), Phase::N);

    let bus_1 = add_bus(&mut netw, vec![Phase::A]);
    let node_1 = nd_id!(&bus_1.borrow().uid(), Phase::A);

    add_ground(&mut netw, node_0.clone());

    add_v_const(&mut netw, node_1.clone(), node_0.clone(), 1.0);
    add_s_const(&mut netw, node_0.clone(), node_1.clone(), s);

    netw
}

/// Returns a simple test network with two constant Y connections.
pub fn test_netw_yy(y: c64) -> Network {
    let mut netw = Network::new();

    let bus_0 = add_bus(&mut netw, vec![Phase::N]);
    let node_0 = nd_id!(&bus_0.borrow().uid(), Phase::N);

    let bus_1 = add_bus(&mut netw, vec![Phase::A]);
    let node_1 = nd_id!(&bus_1.borrow().uid(), Phase::A);

    let bus_2 = add_bus(&mut netw, vec![Phase::A]);
    let node_2 = nd_id!(&bus_2.borrow().uid(), Phase::A);

    add_ground(&mut netw, node_0.clone());

    add_v_const(&mut netw, node_1.clone(), node_0.clone(), 1.0);
    add_y_const(&mut netw, node_1.clone(), node_2.clone(), y);
    add_y_const(&mut netw, node_2.clone(), node_0.clone(), y);

    netw
}

/// Returns a simple test network with a constant Y connection and a constant S connection.
pub fn test_netw_ys(y: c64, s: c64) -> Network {
    let mut netw = Network::new();

    let bus_0 = add_bus(&mut netw, vec![Phase::N]);
    let node_0 = nd_id!(&bus_0.borrow().uid(), Phase::N);

    let bus_1 = add_bus(&mut netw, vec![Phase::A]);
    let node_1 = nd_id!(&bus_1.borrow().uid(), Phase::A);

    let bus_2 = add_bus(&mut netw, vec![Phase::A]);
    let node_2 = nd_id!(&bus_2.borrow().uid(), Phase::A);

    add_ground(&mut netw, node_0.clone());

    add_v_const(&mut netw, node_1.clone(), node_0.clone(), 1.0);
    add_y_const(&mut netw, node_1.clone(), node_2.clone(), y);
    add_s_const(&mut netw, node_2.clone(), node_0.clone(), s);

    netw
}

/// Returns a simple test network with a one-phase transformer.
pub fn test_netw_tx() -> Network {
    let mut netw = Network::new();

    let bus_0 = add_bus(&mut netw, vec![Phase::N]);
    let node_0 = nd_id!(&bus_0.borrow().uid(), Phase::N);

    let bus_1 = add_bus(&mut netw, vec![Phase::A]);
    let node_1 = nd_id!(&bus_1.borrow().uid(), Phase::A);

    let bus_2 = add_bus(&mut netw, vec![Phase::A]);
    let node_2 = nd_id!(&bus_2.borrow().uid(), Phase::A);

    let bus_3 = add_bus(&mut netw, vec![Phase::A]);
    let node_3 = nd_id!(&bus_3.borrow().uid(), Phase::A);

    add_ground(&mut netw, node_0.clone());

    add_v_const(&mut netw, node_1.clone(), node_0.clone(), 1.0);

    add_tx(&mut netw, &node_0.b, &node_1.b, &node_2.b);

    add_y_const(&mut netw, node_2.clone(), node_3.clone(), c!(1.0, 0.0));
    add_y_const(&mut netw, node_3.clone(), node_0.clone(), c!(1.0, 0.0));

    netw
}

/// Returns a simple test network that includes cycles.
pub fn test_netw_cycles() -> Network {
    let mut netw = Network::new();

    let bus_0 = add_bus(&mut netw, vec![Phase::N]);
    let node_0 = nd_id!(&bus_0.borrow().uid(), Phase::N);

    let bus_1 = add_bus(&mut netw, vec![Phase::A]);
    let node_1 = nd_id!(&bus_1.borrow().uid(), Phase::A);

    let bus_2 = add_bus(&mut netw, vec![Phase::A]);
    let node_2 = nd_id!(&bus_2.borrow().uid(), Phase::A);

    let bus_3 = add_bus(&mut netw, vec![Phase::A]);
    let node_3 = nd_id!(&bus_3.borrow().uid(), Phase::A);

    let bus_4 = add_bus(&mut netw, vec![Phase::A]);
    let node_4 = nd_id!(&bus_4.borrow().uid(), Phase::A);

    add_ground(&mut netw, node_0.clone());

    add_v_const(&mut netw, node_1.clone(), node_0.clone(), 1.0);

    add_y_const(&mut netw, node_1.clone(), node_2.clone(), c!(1.0, 0.0));
    add_y_const(&mut netw, node_1.clone(), node_3.clone(), c!(1.0, 0.0));
    add_y_const(&mut netw, node_1.clone(), node_4.clone(), c!(1.0, 0.0));
    add_y_const(&mut netw, node_2.clone(), node_0.clone(), c!(1.0, 0.0));
    add_y_const(&mut netw, node_3.clone(), node_0.clone(), c!(1.0, 0.0));
    add_y_const(&mut netw, node_4.clone(), node_0.clone(), c!(1.0, 0.0));

    netw
}

/// Returns a medium sized test network.
pub fn test_netw_large_a() -> Network {
    let mut netw = Network::new();

    let bus_0 = add_bus(&mut netw, vec![Phase::N]);
    let node_0 = nd_id!(&bus_0.borrow().uid(), Phase::N);

    let bus_1 = add_bus(&mut netw, vec![Phase::A]);
    let node_1 = nd_id!(&bus_1.borrow().uid(), Phase::A);

    let bus_2 = add_bus(&mut netw, vec![Phase::A]);
    let node_2 = nd_id!(&bus_2.borrow().uid(), Phase::A);

    let bus_3 = add_bus(&mut netw, vec![Phase::A]);
    let node_3 = nd_id!(&bus_3.borrow().uid(), Phase::A);

    let bus_4 = add_bus(&mut netw, vec![Phase::A]);
    let node_4 = nd_id!(&bus_4.borrow().uid(), Phase::A);

    let bus_5 = add_bus(&mut netw, vec![Phase::A]);
    let node_5 = nd_id!(&bus_5.borrow().uid(), Phase::A);

    add_ground(&mut netw, node_0.clone());

    add_v_const(&mut netw, node_1.clone(), node_0.clone(), 1.0);

    add_y_const(&mut netw, node_1.clone(), node_2.clone(), c!(10.0, 0.0));
    add_y_const(&mut netw, node_1.clone(), node_3.clone(), c!(10.0, 0.0));
    add_y_const(&mut netw, node_3.clone(), node_0.clone(), c!(10.0, 0.0));
    add_y_const(&mut netw, node_2.clone(), node_4.clone(), c!(10.0, 0.0));
    add_y_const(&mut netw, node_2.clone(), node_5.clone(), c!(10.0, 0.0));

    add_s_const(&mut netw, node_4.clone(), node_0.clone(), c!(0.2, 0.0));
    add_s_const(&mut netw, node_5.clone(), node_0.clone(), c!(0.2, 0.0));

    netw
}

/// Returns a larger test network.
pub fn test_netw_large_b() -> Network {
    let mut netw = Network::new();

    let bus_0 = add_bus(&mut netw, vec![Phase::N]);
    let node_0 = nd_id!(&bus_0.borrow().uid(), Phase::N);

    let bus_1 = add_bus(&mut netw, vec![Phase::A]);
    let node_1 = nd_id!(&bus_1.borrow().uid(), Phase::A);

    let bus_2 = add_bus(&mut netw, vec![Phase::A]);
    let node_2 = nd_id!(&bus_2.borrow().uid(), Phase::A);

    let bus_3 = add_bus(&mut netw, vec![Phase::A]);
    let node_3 = nd_id!(&bus_3.borrow().uid(), Phase::A);

    let bus_4 = add_bus(&mut netw, vec![Phase::A]);
    let node_4 = nd_id!(&bus_4.borrow().uid(), Phase::A);

    let bus_5 = add_bus(&mut netw, vec![Phase::A]);
    let node_5 = nd_id!(&bus_5.borrow().uid(), Phase::A);

    let bus_6 = add_bus(&mut netw, vec![Phase::A]);
    let node_6 = nd_id!(&bus_6.borrow().uid(), Phase::A);

    let bus_7 = add_bus(&mut netw, vec![Phase::A]);
    let node_7 = nd_id!(&bus_7.borrow().uid(), Phase::A);

    let bus_8 = add_bus(&mut netw, vec![Phase::A]);
    let node_8 = nd_id!(&bus_8.borrow().uid(), Phase::A);

    let bus_9 = add_bus(&mut netw, vec![Phase::A]);
    let node_9 = nd_id!(&bus_9.borrow().uid(), Phase::A);

    let bus_10 = add_bus(&mut netw, vec![Phase::A]);
    let node_10 = nd_id!(&bus_10.borrow().uid(), Phase::A);

    let bus_11 = add_bus(&mut netw, vec![Phase::A]);
    let node_11 = nd_id!(&bus_11.borrow().uid(), Phase::A);

    add_ground(&mut netw, node_0.clone());

    add_v_const(&mut netw, node_1.clone(), node_0.clone(), 1.0);

    add_y_const(&mut netw, node_1.clone(), node_2.clone(), c!(10.0, 1.0));
    add_y_const(&mut netw, node_1.clone(), node_3.clone(), c!(5.0, 0.3));
    add_y_const(&mut netw, node_2.clone(), node_4.clone(), c!(13.0, 4.7));
    add_y_const(&mut netw, node_2.clone(), node_5.clone(), c!(11.0, 10.0));
    add_y_const(&mut netw, node_3.clone(), node_5.clone(), c!(16.0, 0.0));
    add_y_const(&mut netw, node_3.clone(), node_6.clone(), c!(30.0, 0.0));
    add_y_const(&mut netw, node_4.clone(), node_7.clone(), c!(10.0, 0.07));
    add_y_const(&mut netw, node_5.clone(), node_8.clone(), c!(27.0, 0.0));
    add_y_const(&mut netw, node_6.clone(), node_9.clone(), c!(17.0, 0.0));
    add_y_const(&mut netw, node_8.clone(), node_9.clone(), c!(10.3, 4.2));
    add_y_const(&mut netw, node_9.clone(), node_10.clone(), c!(10.0, 0.0));
    add_y_const(&mut netw, node_9.clone(), node_11.clone(), c!(10.0, 0.0));

    add_s_const(&mut netw, node_3.clone(), node_0.clone(), c!(0.13, 0.0));
    add_s_const(&mut netw, node_7.clone(), node_0.clone(), c!(0.2, 0.0));
    add_s_const(&mut netw, node_8.clone(), node_0.clone(), c!(0.13, 0.0));
    add_s_const(&mut netw, node_10.clone(), node_0.clone(), c!(0.13, 0.0));
    add_s_const(&mut netw, node_11.clone(), node_0.clone(), c!(0.17, 0.0));

    netw
}
