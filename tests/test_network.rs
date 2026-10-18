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
            Source::new_wye_slack(1.0, 3, false)
                .element("src_1abc", vec![bpg.clone(), bp1a.clone(), bp1b.clone(), bp1c.clone()], true)
        ).unwrap();

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

    fn test_solve_netw(filename: &str, solver_key: &str) {
        let mut netw = test_netw_ejson(filename, None);
        let solve_result = solve_network(&mut netw, solver_key, None);
        assert!(solve_result.is_ok());
        if let Ok(SolveNetworkResult {statuses, solvers: _}) = solve_result {
            println!("----------------");
            println!("Testing: {filename}");
            for status in statuses.iter() {
                println!("\n    Overall:");
                println!("        {:?}", status.status);
                for iter_status in status.iteration_statuses.iter() {
                    println!("\n    Iter:");
                    println!("        {:?}", iter_status);
                }
            }
        }
    }

    #[test]
    fn test_solve_networks() {
        test_solve_netw("netw-i1l1s1.json", "nodal_rect");
        test_solve_netw("netw-i2l2s2.json", "nodal_rect");
        test_solve_netw("netw-i3l3s3.json", "nodal_rect");
    }
}
