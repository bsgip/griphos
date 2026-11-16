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
    Bus, NodeId, C_ZERO, Element, Line, Load, Network, Phase, SQRT_3, Source,
    Transformer, nd_id, c, c64, typical_tx_z_series
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
    ).expect("adding src_bus failed"); // Unwrap or panic with error message.

    // We grabbed a handle to the new bus, and we do various things with it.
    // We need to borrow() it to access its contents, and we can also
    // borrow_mut() to mutate it,
    println!("Souce bus:\n{}", src_bus.borrow());
    println!("Souce bus base voltage:\n{}", src_bus.borrow().v_base);

    // Add a three phase bus with no neutral.
    netw.add_bus(Bus::new("tx_mv_bus", ABC.to_vec(), V_MV)).unwrap();

    // Add another bus, this time three-phase plus neutral and with a base
    // voltage of 230 V (line to ground).
    netw.add_bus(Bus::new("tx_lv_bus", ABCN.to_vec(), V_LV)).unwrap();

    // Add a wye-connected source on src_bus.
    //
    // Note that each different element type (i.e. the implementors of
    // ElementSpec, like Line, Transformer, Load, etc.), normally have its own
    // into_element(...) member function to conveniently connect it to the network.
    // The signatures of these element functions will typically vary depending
    // on the details of the element type.
    //
    // Add an element to the network...
    let source: Element<Source> = netw.add_element(
        // A source element, with 3 live phases (plus grounded neutral since
        // it's a wye connection - the `true` grounds the neutral).
        Source::new_wye_slack(V_MV, 3, true)
            // Wrap it in it's own element...
            .into_element(
                // with UID "source"...
                "source",
                // and connect to phases ABCN of src_bus, N being neutral.
                ABCN.iter().map(|ph| nd_id!("src_bus", *ph)).collect(),
                // Ground the neutral bus.
                true
            )
    ).expect("problem adding source");

    // Similar to buses, we can access the element using borrow() and
    // borrow_mut(). uid() is common to all elements.
    println!("Source element:\n{}", source.borrow());
    println!("source uid is {}", source.borrow().uid());

    // Because this is a typed element handle, Element<Source>, we can also
    // access the Source API.
    println!("source control is {:?}", source.borrow().control);

    // However, if we access the element via the network, we obtain a non-typed
    // element handle, Element instead of Element<Source>.
    let src_generic: Element = netw.element("source")
        .expect("problem getting source");
    // We can still access the common Element API:
    println!("source (generic) uid is {}", src_generic.borrow().uid());
    // But src_generic doesn't know that it's a Source Element.
    // Thus, the following would fail:
    // println!("source control: {:?}", src_generic.borrow().control());

    // Add a three-wire line between the src_bus and tx_mv_bus. The element(...)
    // member for Lines takes a UID and two lists of NodeIds: one for the
    // start of the line, and the other for the end of the line. Note how the
    // signature of element() is different than that for Sources, see above.
    netw.add_element(
        // z = 0.4+0.4j, z0 = 0.9+1.2j, n_phase = 3
        Line::new_from_sym_seq(c!(0.4, 0.4), c!(0.9, 1.2), 3)
            .expect("problem creating line")
            .into_element(
                "line",
                ABC.iter().map(|ph| nd_id!("src_bus", *ph)).collect(),
                ABC.iter().map(|ph| nd_id!("tx_mv_bus", *ph)).collect(),
            )
    ).expect("problem adding line");

    // Add a DYN11 transformer between tx_mv_bus and tx_lv_bus.
    let zs = typical_tx_z_series(500.0e3, 230.0); // Utility function for typical transformer series impedance.
    netw.add_element(
        Transformer::new_with_vector_group(
            3,                                      // 3 phases.
            vec![c!(11000.0, 0.0), c!(230.0, 0.0)], // scaled turns = [11000, 230].
            None,                                   // No taps.
            vec![C_ZERO, zs],                       // Series impedance [z_p, z_s].
            "dyn11"                                 // Vector group string, lowercase.
        ).expect("problem creating transformer")
            .into_element(
                "transformer",
                ABC.iter().map(|ph| nd_id!("tx_mv_bus", *ph)).chain(
                    ABCN.iter().map(|ph| nd_id!("tx_lv_bus", *ph))
                ).collect(),  // Secondary phases.
            )
    ).expect("problem adding transformer");

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
        ).into_element(
                "load",
                vec![
                    nd_id!("tx_lv_bus", Phase::A),
                    nd_id!("tx_lv_bus", Phase::B),
                    nd_id!("tx_lv_bus", Phase::C),
                    nd_id!("tx_lv_bus", Phase::N),
                ]
            )
    ).expect("problem adding load");

    // Set the load's constant power component.
    if let Some(sc) = load.borrow_mut().ld_s_const_mut() {
        sc.copy_from_slice(&[c!(1e3, 0.5e3), c!(2e3, 0.4e3), c!(1.5e3, 0.1e3)]);
    }

    // Take the load out of service.
    load.borrow_mut().set_is_in_service(false);
    assert!(!load.borrow().is_in_service());

    // Let's drill into the network nodes: individual phases of the buses.
    for bus in netw.buses() {
        println!("{}", bus.borrow().uid());
        for nd in bus.borrow().nodes() {
            println!("    {}, {}", nd.phase(), nd.v());
        }
    }

    // Now let's drill into the network branches: individual conducting paths of
    // elements. Note that there will be no branches for the load since we put
    // it out of service.
    for elem in netw.elements() {
        println!("{}", elem.borrow().uid());
        for br in elem.borrow().branches() {
            println!("    {:?}, {}, {}", br.term_idxs(), br.v(), br.i());
        }
    }

    // We can find a particular bus...
    let src_bus_found = netw.bus("src_bus").unwrap();
    println!("{}", src_bus_found.borrow().uid());

    // ...or a particular element:
    let ld_elem = netw.element("load").unwrap();
    println!("{}", ld_elem.borrow().is_in_service());

    // We can also downcast elements.
    let tx_elem: Element<Transformer> = netw.element("transformer").unwrap().downcast::<Transformer>().unwrap();
    // This lets us access members that are particular to that type of element.
    println!("{:?}", tx_elem.borrow().z_series);

    // We can iterate by element type (giving the downcast versions):
    for elem in netw.elements_of_type::<Transformer>() {
        println!("{}", elem.borrow().uid());
    }
}
