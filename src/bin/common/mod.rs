use crate::{Network, solve_network};

pub fn run_network(mut netw: Network, solver_name: &str) {
    solve_network(&mut netw, solver_name, None).unwrap();

    println!("Voltages:");
    for b in netw.buses() {
        let b = b.borrow();
        let vmags = b.vs().map(|x| x.norm().to_string()).collect::<Vec<_>>().join(", ");
        println!("v {:<12} = [{:>10.4}]", b.uid, vmags);
    }

    println!("\nCurrents:");
    for e in netw.elements() {
        let e = e.borrow();
        let br_data = e.branches_connected().collect::<Vec<_>>();
        let br_strs = br_data
            .iter()
            .map(|bdi| format!("{}->{}:{:.4}", bdi.bus_node_ids.0, bdi.bus_node_ids.1, bdi.state.i))
            .collect::<Vec<_>>().join(", ");
        println!("i {:<12}: [{:>30}]", e.uid(), br_strs);
    }

    println!("\nPowers:");
    for e in netw.elements() {
        let e = e.borrow();
        let br_data = e.branches_connected().collect::<Vec<_>>();
        let br_strs = br_data
            .iter()
            .map(
                |bdi| format!(
                    "{}->{}:{:.4}",
                    bdi.bus_node_ids.0, bdi.bus_node_ids.1, bdi.state.i.conj() * bdi.state.v
                )
            )
            .collect::<Vec<_>>().join(", ");
        println!("i {:<12}: [{:>30}]", e.uid(), br_strs);
    }
}
