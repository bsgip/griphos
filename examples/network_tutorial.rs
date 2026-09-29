// This example shows the detailed construction of a new network, and the creation and assignment to the network of
// several buses and elements. It will show details of how various elements are constructed and joined to the network.
// Normally, this kind of low-level network construction won't be done in-code like this, but instead the network will
// be read in from e-JSON or other parsing code. However, here the aim is to illustrate some details about how network
// modelling is done.
//
// We will construct the following network:
// source - line - transformer - load
// Where the source and load are wye-connected and grounded, the first line has three live wires, the transformer is
// dyn11 connected, and the load is wye connected and grounded.

use griphos::{bp, Bus, BusPhaseId, c, c64, Grounding, Line, Load, Network, Phase, Source, SQRT_3, Transformer };

fn main() {
    // Bus voltages are always given as line-to-ground equivalent.
    // Medium voltage is 11 kV line to line, or (11/sqrt(3)) kV line to ground.
    const V_MV: f64 = 11.0e3 / SQRT_3;
    // Low voltage is 230 V line to ground.
    const V_LV: f64 = 230.0;

    // Create a new, empty network.
    let mut netw = Network::new();

    // Add a three phase bus, with additional neutral, that will be grounded. We divide the base voltage of 11 kV line
    // to line by sqrt(3) to convert it to line to ground, because base voltages of buses are always specified as line
    // to ground.
    let src_bus = netw.add_bus(
        Bus::new(
            "src_bus",                                      // Unique ID.
            vec![Phase::A, Phase::B, Phase::C, Phase::N],   // Vector of phases neutral last by convention.
            V_MV                                            // Base voltage relative to ground.
        )
    ).unwrap();

    // Add a three phase bus with no neutral.
    let tx_mv_bus = netw.add_bus(Bus::new("tx_mv_bus", vec![Phase::A, Phase::B, Phase::C], V_MV)).unwrap();

    // Add another bus, this time three-phase plus neutral and with a base voltage of 230 V (line to ground).
    let tx_lv_bus = netw.add_bus(Bus::new("tx_lv_bus", vec![Phase::A, Phase::B, Phase::C, Phase::N], V_LV)).unwrap();

    // Ground the neutral phases, using a Grounding elements.
    //
    // Note that each different element type (i.e. the implementors of ElementSpec, like Line, Transformer, Load, etc.),
    // normally have its own element(...) member function to conveniently connect it to the network. The signatures of
    // these element functions will typically vary depending on the details of the element type.

    let grounding_src = netw.add_element(                           // A an element to the network...
        Grounding::new(1)                       // ... a Grounding object with 1 terminal
            .element(                           // Wrap it in it's own element...
                "grounding_src_bus",                    // with UID "grounding"...
                vec![bp!("src_bus", Phase::N)]  // and connect the element it to phase N of src_bus.
            )).unwrap();

    let grounding_lv = netw.add_element(                           // A an element to the network...
        Grounding::new(1)                       // ... a Grounding object with 1 terminal
            .element(                           // Wrap it in it's own element...
                "grounding_tx_lv_bus",                    // with UID "grounding"...
                vec![bp!("tx_lv_bus", Phase::N)]  // and connect the element it to phase N of src_bus.
            )).unwrap();

    // Add a wye-connected source on src_bus.
    let source = netw.add_element(
        Source::new_wye_slack(V_MV, 3)
            .element(
                "source",
                vec![
                    bp!("src_bus", Phase::A), // Terminal 0 connected to phase A of src_bus
                    bp!("src_bus", Phase::B), // Terminal 1 connected to phase B of src_bus
                    bp!("src_bus", Phase::C), // Terminal 2 connected to phase C of src_bus
                    bp!("src_bus", Phase::N), // Terminal 3 connected to phase N of src_bus
                ],
                true
            )
    ).unwrap();

    // Add a three-wire line between the src_bus and tx_mv_bus
    let line = netw.add_element(
        Line::new_from_sym_seq(c!(0.4, 0.4), c!(0.9, 1.2), 3)   // z = 0.4+0.4j, z0 = 0.9+1.2j, n_phase = 3
            .unwrap()
            .element(
                "line",
                // The element(...) member for Lines takes a UID and two lists of BusPhaseIds: one for the start of the
                // line, and the other for the end of the line. Note how the signature of element() is different than
                // that for Sources, see above.
                vec![bp!("src_bus", Phase::A), bp!("src_bus", Phase::B), bp!("src_bus", Phase::C)],
                vec![bp!("tx_mv_bus", Phase::A), bp!("tx_mv_bus", Phase::B), bp!("tx_mv_bus", Phase::C)],
            )).unwrap();

    // Add a DYN11 transformer between tx_mv_bus and tx_lv_bus.
    let tx = netw.add_element(
        Transformer::new_dy(
            3,                  // 3 phases, i.e. 3 pairs of windings
            c!(47.826, 0.),     // ratio = 11 kV / 230 V
            c!(0.06, 0.1),      // Series impedance = 0.06 + 0.1j.
            c!(0.0, 0.0),       // y_shunt is small and can be ignored.
            None,               // No taps for this transformer.
        ).unwrap().element(
                "transformer",
                &[bp!("tx_mv_bus", Phase::A), bp!("tx_mv_bus", Phase::B), bp!("tx_mv_bus", Phase::C)],
                &[bp!("tx_mv_bus", Phase::A), bp!("tx_mv_bus", Phase::B), bp!("tx_mv_bus", Phase::C)],
        )
    ).unwrap();

    // Add a wye-connected load at tx_bus_lv.
    let load = netw.add_element(
        Load::new_wye(
            3,
            None,                                                           // No constant impedance component.
            None,                                                           // No constant current component.
            Some(vec![c!(1e3, 0.5e3), c!(2e3, 0.4e3), c!(1.5e3, 0.1e3)]),   // Unbalanced 3-phase constant power load.
            true                                                            // Star point is grounded.
        ).element(
            "load",
            vec![
                bp!("tx_lv_bus", Phase::A),
                bp!("tx_lv_bus", Phase::B),
                bp!("tx_lv_bus", Phase::C),
                bp!("tx_lv_bus", Phase::N),
            ]
        )
    );
}
