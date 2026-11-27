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

        let bpg = nd_id!("bus_g", Phase::N);
        let bp1a = nd_id!("bus_1", Phase::A);
        let bp1b = nd_id!("bus_1", Phase::B);
        let bp1c = nd_id!("bus_1", Phase::C);
        let bp2a = nd_id!("bus_2", Phase::A);
        let bp2b = nd_id!("bus_2", Phase::B);
        let bp2c = nd_id!("bus_2", Phase::C);
        let bp3a = nd_id!("bus_3", Phase::A);

        netw.add_element(Grounding::new(1).into_element("gr", vec![bpg.clone()])).unwrap();

        netw.add_element(
            Line::new_from_sym_seq(c!(1.2, 1.3), c!(1.21, 1.32), 3).unwrap()
                .into_element(
                    "ln_1abc_2abc",
                    vec![bp1a.clone(), bp1b.clone(), bp1c.clone()],
                    vec![bp2a.clone(), bp2b.clone(), bp2c.clone()]
                )).unwrap();

        netw.add_element(
            Line::new_from_sym_seq(c!(1.2, 1.3), c!(1.21, 1.32), 1).unwrap()
                .into_element(
                    "ln_2b_3a",
                    vec![bp2b.clone()],
                    vec![bp3a.clone()]
                )).unwrap();

        netw.add_element(
            Source::new_wye_slack(1.0, 3, false)
                .into_element("src_1abc", vec![bpg.clone(), bp1a.clone(), bp1b.clone(), bp1c.clone()], true)
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

    #[test]
    fn test_structure_hash() {
        let netw = test_netw_ejson("netw-i3l3s3.json", None);
        let hash = netw.structure_hash();
        assert_eq!(hash, test_netw_ejson("netw-i3l3s3.json", None).structure_hash());

        // Changing values doesn't change the structure.
        let ld = netw.element("ld2").unwrap().downcast::<Load>().unwrap();
        for s in ld.borrow_mut().ld_s_const_mut().unwrap() {
            *s *= 3.0;
        }
        assert_eq!(netw.structure_hash(), hash);

        // Taking an element out of service does.
        ld.borrow_mut().set_is_in_service(false);
        assert_ne!(netw.structure_hash(), hash);
    }

    fn bus_vs(netw: &Network) -> Vec<c64> {
        netw.buses().flat_map(|b| b.borrow().vs().collect::<Vec<_>>()).collect()
    }

    fn scale_load(netw: &Network, uid: &str, factor: f64) {
        let ld = netw.element(uid).unwrap().downcast::<Load>().unwrap();
        for s in ld.borrow_mut().ld_s_const_mut().unwrap() {
            *s *= factor;
        }
    }

    #[test]
    fn test_reused_solvers_pick_up_new_values() {
        let mut netw = test_netw_ejson("netw-i3l3s3.json", None);
        let SolveNetworkResult {solvers, ..} = solve_network(&mut netw, "nodal_rect", None).unwrap();
        let vs_orig = bus_vs(&netw);

        // Twice, to check that values are picked up on every reuse, not just the first.
        let mut solvers = Some(solvers);
        for factor in [3.0, 0.5] {
            scale_load(&netw, "ld2", factor);
            let SolveNetworkResult {statuses, solvers: used} =
                solve_network(&mut netw, "nodal_rect", solvers.take()).unwrap();
            assert!(statuses.iter().all(|s| s.status.flags.contains(SolverStatusFlags::SUCCESS)));
            solvers = Some(used);
            let vs_reused = bus_vs(&netw);
            assert!(vs_reused.iter().zip(&vs_orig).any(|(a, b)| (a - b).norm() > 1e-3));

            // Should match fresh solvers on an identically modified network.
            let mut netw_fresh = test_netw_ejson("netw-i3l3s3.json", None);
            for ld in netw_fresh.elements_of_type::<Load>() {
                let src = netw.element(ld.borrow().uid()).unwrap().downcast::<Load>().unwrap();
                ld.borrow_mut().ld_s_const_mut().unwrap().copy_from_slice(src.borrow().ld_s_const().unwrap());
            }
            solve_network(&mut netw_fresh, "nodal_rect", None).unwrap();
            for (a, b) in vs_reused.iter().zip(&bus_vs(&netw_fresh)) {
                assert!((a - b).norm() < 1e-6, "reused {a} != fresh {b}");
            }
        }
    }

    #[test]
    fn test_solvers_reused_or_rebuilt() {
        fn ptrs(solvers: &NetworkSolvers) -> Vec<*const u8> {
            solvers.solvers().iter().map(|s| &**s as *const dyn NetworkSolver as *const u8).collect()
        }

        let mut netw = test_netw_ejson("netw-i3l3s3.json", None);
        let SolveNetworkResult {solvers, ..} = solve_network(&mut netw, "nodal_rect", None).unwrap();
        let hash = netw.structure_hash();
        assert_eq!(solvers.structure_hash(), hash);

        // Unchanged structure: the same solvers are reused.
        let ptrs_orig = ptrs(&solvers);
        let SolveNetworkResult {solvers, ..} = solve_network(&mut netw, "nodal_rect", Some(solvers)).unwrap();
        assert_eq!(ptrs(&solvers), ptrs_orig);

        // Changed structure: new solvers are created and the solve still succeeds.
        netw.element("ld2").unwrap().borrow_mut().set_is_in_service(false);
        let SolveNetworkResult {statuses, solvers} = solve_network(&mut netw, "nodal_rect", Some(solvers)).unwrap();
        assert!(statuses.iter().all(|s| s.status.flags.contains(SolverStatusFlags::SUCCESS)));
        let new_hash = netw.structure_hash();
        assert_ne!(new_hash, hash);
        assert_eq!(solvers.structure_hash(), new_hash);
    }
}
