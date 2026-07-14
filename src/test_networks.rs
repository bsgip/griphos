use crate::foundation::*;
use crate::network::*;
use crate::test_utils::*;

pub fn netw_y(y: c64) -> Network {
    let mut netw = Network::new();

    add_bus(&mut netw, 0, Phase::N);
    add_bus(&mut netw, 1, Phase::A);

    add_ground(&mut netw, 0);

    add_v_const(&mut netw, 1, 0, 1.0);
    add_y_const(&mut netw, 0, 1, y);

    netw
}

pub fn netw_s(s: c64) -> Network {
    let mut netw = Network::new();

    add_bus(&mut netw, 0, Phase::N);
    add_bus(&mut netw, 1, Phase::A);

    add_ground(&mut netw, 0);

    add_v_const(&mut netw, 1, 0, 1.0);

    add_s_const(&mut netw, 1, 0, s);

    netw
}

pub fn netw_yy(y: c64) -> Network {
    let mut netw = Network::new();

    add_bus(&mut netw, 0, Phase::N);
    add_bus(&mut netw, 1, Phase::A);
    add_bus(&mut netw, 2, Phase::A);

    add_ground(&mut netw, 0);

    add_v_const(&mut netw, 1, 0, 1.0);

    add_y_const(&mut netw, 1, 2, y);
    add_y_const(&mut netw, 2, 0, y);

    netw
}

pub fn netw_ys(y: c64, s: c64) -> Network {
    let mut netw = Network::new();

    add_bus(&mut netw, 0, Phase::N);
    add_bus(&mut netw, 1, Phase::A);
    add_bus(&mut netw, 2, Phase::A);

    add_ground(&mut netw, 0);

    add_v_const(&mut netw, 1, 0, 1.0);

    add_y_const(&mut netw, 1, 2, y);

    add_s_const(&mut netw, 2, 0, s);

    netw
}

pub fn netw_tx() -> Network {
    let mut netw = Network::new();

    add_bus(&mut netw, 0, Phase::N);
    add_bus(&mut netw, 1, Phase::A);
    add_bus(&mut netw, 2, Phase::A);
    add_bus(&mut netw, 3, Phase::A);

    add_ground(&mut netw, 0);

    add_v_const(&mut netw, 1, 0, 1.0);

    add_tx(&mut netw, (1, 0), (2, 0), c!(2.0, 0.0));

    add_y_const(&mut netw, 2, 3, c!(1.0, 0.0));
    add_y_const(&mut netw, 3, 0, c!(1.0, 0.0));

    netw
}

pub fn netw_cycles() -> Network {
    let mut netw = Network::new();

    add_bus(&mut netw, 0, Phase::N);
    add_bus(&mut netw, 1, Phase::A);
    add_bus(&mut netw, 2, Phase::A);
    add_bus(&mut netw, 3, Phase::A);
    add_bus(&mut netw, 4, Phase::A);

    add_ground(&mut netw, 0);

    add_v_const(&mut netw, 1, 0, 1.0);

    add_y_const(&mut netw, 1, 2, c!(1.0, 0.0));
    add_y_const(&mut netw, 1, 3, c!(1.0, 0.0));
    add_y_const(&mut netw, 1, 4, c!(1.0, 0.0));
    add_y_const(&mut netw, 2, 0, c!(1.0, 0.0));
    add_y_const(&mut netw, 3, 0, c!(1.0, 0.0));
    add_y_const(&mut netw, 4, 0, c!(1.0, 0.0));

    netw
}

pub fn netw_large_a() -> Network {
    let mut netw = Network::new();

    add_bus(&mut netw, 0, Phase::N);
    add_bus(&mut netw, 1, Phase::A);
    add_bus(&mut netw, 2, Phase::A);
    add_bus(&mut netw, 3, Phase::A);
    add_bus(&mut netw, 4, Phase::A);
    add_bus(&mut netw, 5, Phase::A);

    add_ground(&mut netw, 0);

    add_v_const(&mut netw, 1, 0, 1.0);

    add_y_const(&mut netw, 1, 2, c!(10.0, 0.0));
    add_y_const(&mut netw, 1, 3, c!(10.0, 0.0));
    add_y_const(&mut netw, 3, 0, c!(10.0, 0.0));
    add_y_const(&mut netw, 2, 4, c!(10.0, 0.0));
    add_y_const(&mut netw, 2, 5, c!(10.0, 0.0));

    add_s_const(&mut netw, 4, 0, c!(0.2, 0.0));
    add_s_const(&mut netw, 5, 0, c!(0.2, 0.0));

    netw
}

pub fn netw_large_b() -> Network {
    let mut netw = Network::new();

    add_bus(&mut netw, 0, Phase::N);
    add_bus(&mut netw, 1, Phase::A);
    add_bus(&mut netw, 2, Phase::A);
    add_bus(&mut netw, 3, Phase::A);
    add_bus(&mut netw, 4, Phase::A);
    add_bus(&mut netw, 5, Phase::A);
    add_bus(&mut netw, 6, Phase::A);
    add_bus(&mut netw, 7, Phase::A);
    add_bus(&mut netw, 8, Phase::A);
    add_bus(&mut netw, 9, Phase::A);
    add_bus(&mut netw, 10, Phase::A);
    add_bus(&mut netw, 11, Phase::A);

    add_ground(&mut netw, 0);

    add_v_const(&mut netw, 1, 0, 1.0);

    add_y_const(&mut netw, 1, 2, c!(10.0, 1.0));
    add_y_const(&mut netw, 1, 3, c!(5.0, 0.3));
    add_y_const(&mut netw, 2, 4, c!(13.0, 4.7));
    add_y_const(&mut netw, 2, 5, c!(11.0, 10.0));
    add_y_const(&mut netw, 3, 5, c!(16.0, 0.0));
    add_y_const(&mut netw, 3, 6, c!(30.0, 0.0));
    add_y_const(&mut netw, 4, 7, c!(10.0, 0.07));
    add_y_const(&mut netw, 5, 8, c!(27.0, 0.0));
    add_y_const(&mut netw, 6, 9, c!(17.0, 0.0));
    add_y_const(&mut netw, 8, 9, c!(10.3, 4.2));
    add_y_const(&mut netw, 9, 10, c!(10.0, 0.0));
    add_y_const(&mut netw, 9, 11, c!(10.0, 0.0));

    add_s_const(&mut netw, 3, 0, c!(0.13, 0.0));
    add_s_const(&mut netw, 7, 0, c!(0.2, 0.0));
    add_s_const(&mut netw, 8, 0, c!(0.13, 0.0));
    add_s_const(&mut netw, 10, 0, c!(0.13, 0.0));
    add_s_const(&mut netw, 11, 0, c!(0.17, 0.0));

    netw
}
