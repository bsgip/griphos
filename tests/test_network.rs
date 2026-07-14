extern crate nalgebra as na;

#[cfg(test)]
mod tests {
    use griphos::*;

    #[test]
    fn test_network() {
        let mut netw = Network::new();
        assert!(netw.add_bus(Bus::new("bus_g", vec![Phase::N], 1.0)).is_ok());
        assert!(netw.add_bus(Bus::new("bus_1", vec![Phase::A, Phase::B, Phase::C], 1.0)).is_ok());
        assert!(netw.add_bus(Bus::new("bus_2", vec![Phase::A, Phase::B, Phase::C], 1.0)).is_ok());
        assert!(netw.add_bus(Bus::new("bus_3", vec![Phase::A], 1.0)).is_ok());

        let bpg = bt!("bus_g", Phase::N);
        let bp1a = bt!("bus_1", Phase::A);
        let bp1b = bt!("bus_1", Phase::B);
        let bp1c = bt!("bus_1", Phase::C);
        let bp2a = bt!("bus_2", Phase::A);
        let bp2b = bt!("bus_2", Phase::B);
        let bp2c = bt!("bus_2", Phase::C);
        let bp3a = bt!("bus_3", Phase::A);

        netw.add_element(Grounding::new(1).element("gr", vec![bpg.clone()])).unwrap();

        netw.add_element(
            Line::new_from_sym_seq(c!(1.2, 1.3), c!(1.21, 1.32), 3).unwrap()
                .element(
                    "ln_1abc_2abc",
                    vec![bp1a.clone(), bp1b.clone(), bp1c.clone()],
                    vec![bp2a.clone(), bp2b.clone(), bp2c.clone()]
                )).unwrap();

        netw.add_element(
            Line::new_from_sym_seq(c!(1.2, 1.3), c!(1.21, 1.32), 1).unwrap()
                .element(
                    "ln_2b_3a",
                    vec![bp2b.clone()],
                    vec![bp3a.clone()]
                )).unwrap();

        netw.add_element(
            Generator::new(4, vec![(0, 1), (0, 2), (0, 3)], GeneratorControl::SL {vmag: 1.0, vang: 0.0})
            .element("gen_1abc", vec![bpg.clone(), bp1a.clone(), bp1b.clone(), bp1c.clone()])).unwrap();

        println!("\nNetwork:\n{}", &Wrap(&netw));

        assert_eq!(netw.n_buses(), 4);
        assert_eq!(netw.n_elements(), 4);

        let nmod = NetworkModel::new_master(&netw).unwrap();

        println!("Network y_const:\n{}", Wrap(&nmod.y_const));
        println!("Network i_const:\n{}", Wrap(&nmod.i_const));
        println!("Network s_const:\n{}", Wrap(&nmod.s_const));
        println!("Network vmag_const:\n{}", Wrap(&nmod.vmag_const));
        println!("Network vang_const:\n{}", Wrap(&nmod.vang_const));
        println!("Network structure_graph:\n{:?}", graph_dot(&nmod.structure_graph));
    }
}
