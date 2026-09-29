// This example shows the detailed construction of a new network, and the
// creation and assignment to the network of several buses and elements. It will
// show details of how various elements are constructed and joined to the
// network. Normally, this kind of low-level network construction won't be done
// in-code like this, but instead the network will be read in from e-JSON or
// other parsing code. However, here the aim is to illustrate some details about
// how network modelling is done.
//
// We will construct the following network: source - line - transformer - load
// Where the source and load are wye-connected and grounded, the first line has
// three live wires, the transformer is dyn11 connected, and the load is wye
// connected and grounded.

use griphos::{
    bp, Bus, BusPhaseId, c, c64, Grounding, Line, Load, Network, Phase, Source,
    SQRT_3, Transformer
};

fn main() {
    // Some convenient quantities:
    const ABC: [Phase; 3] = [Phase::A, Phase::B, Phase::C];
    const ABCN: [Phase; 4] = [Phase::A, Phase::B, Phase::C, Phase::N];

    // Bus voltages are always given as line-to-ground equivalent.
    // Medium voltage is 11 kV line to line, or (11/sqrt(3)) kV line to ground.
    const V_MV: f64 = 11.0e3 / SQRT_3;
    // Low voltage is 230 V line to ground.
    const V_LV: f64 = 230.0;

    // Create a new, empty network.
    let mut netw = Network::new();

    // Add a three phase bus, with additional neutral, that will be grounded. We
    // divide the base voltage of 11 kV line to line by sqrt(3) to convert it to
    // line to ground, because base voltages of buses are always specified as
    // line to ground.
    let src_bus = netw.add_bus(
        Bus::new(
            "src_bus",      // Unique ID.
            ABC.to_vec(),   // Three phases ABC.
            V_MV            // Base voltage line to ground = 11 / sqrt(3) kV.
        )
    ).unwrap();

    // Add a three phase bus with no neutral.
    let tx_mv_bus = netw.add_bus(Bus::new("tx_mv_bus", ABC.to_vec(), V_MV))
        .unwrap();

    // Add another bus, this time three-phase plus neutral and with a base
    // voltage of 230 V (line to ground).
    let tx_lv_bus = netw.add_bus(Bus::new("tx_lv_bus", ABCN.to_vec(), V_LV))
        .unwrap();

    // Ground the neutral phases, using a Grounding elements.
    //
    // Note that each different element type (i.e. the implementors of
    // ElementSpec, like Line, Transformer, Load, etc.), normally have its own
    // element(...) member function to conveniently connect it to the network.
    // The signatures of these element functions will typically vary depending
    // on the details of the element type.
    //
    // Add an element to the network...
    let grounding_src = netw.add_element(
        // ... Grounding with 1 terminal
        Grounding::new(1)
            // Wrap it in it's own element...
            .element(
                // with UID "grounding_src_bus"...
                "grounding_src_bus",
                // and connect the element to phase N of src_bus.
                vec![bp!("src_bus", Phase::N)]
            )).unwrap();

    let grounding_lv = netw.add_element(Grounding::new(1).element(
        "grounding_tx_lv_bus", vec![bp!("tx_lv_bus", Phase::N)]
    )).unwrap();

    // Add a wye-connected source on src_bus.
    let source = netw.add_element(
        Source::new_wye_slack(V_MV, 3)
            .element(
                "source",
                ABCN.iter().map(|ph| bp!("src_bus", *ph)).collect(),
                true
            )
    ).unwrap();

    // Add a three-wire line between the src_bus and tx_mv_bus. The element(...)
    // member for Lines takes a UID and two lists of BusPhaseIds: one for the
    // start of the line, and the other for the end of the line. Note how the
    // signature of element() is different than that for Sources, see above.
    let line = netw.add_element(
        // z = 0.4+0.4j, z0 = 0.9+1.2j, n_phase = 3
        Line::new_from_sym_seq(c!(0.4, 0.4), c!(0.9, 1.2), 3)
            .unwrap()
            .element(
                "line",
                ABC.iter().map(|ph| bp!("src_bus", *ph)).collect(),
                ABC.iter().map(|ph| bp!("tx_mv_bus", *ph)).collect(),
            )
    ).unwrap();

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
                ABC.iter().map(|ph| bp!("tx_mv_bus", *ph)).collect(),
                ABCN.iter().map(|ph| bp!("tx_lv_bus", *ph)).collect(),
            )
    ).unwrap();

    // Add a wye-connected load at tx_bus_lv.
    let load = netw.add_element(
        Load::new_wye(
            // 3 phases.
            3,
            // No constant impedance component.
            None,
            // No constant current component.
            None,
            // Unbalanced 3-phase constant power load.
            Some(vec![c!(1e3, 0.5e3), c!(2e3, 0.4e3), c!(1.5e3, 0.1e3)]),
            // Star point is grounded.
            true
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
