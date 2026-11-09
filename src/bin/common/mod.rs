use crate::{c64, Network, solve_network};

fn fmt_polar(x: c64) -> String {
    format!("{:>}", format!("{:.4}\u{2220}{:.4}", x.norm(), x.arg()))
}

pub fn run_network(mut netw: Network, solver_name: &str) {
    solve_network(&mut netw, solver_name, None).unwrap();

    println!("Bus Voltages:");
    for b in netw.buses() {
        let b = b.borrow();
        let vs = b.vs().map(|x| format!("{:16}", fmt_polar(x))).collect::<Vec<_>>().join(", ");
        println!("{:<24}: [ {} ]", b.uid, vs);
    }

    println!("\nBranch Voltages:");
    for e in netw.elements() {
        let e = e.borrow();
        let br_data = e.branches().collect::<Vec<_>>();
        let br_strs = br_data
            .iter()
            .map(
                |bdi| format!(
                    "{:40}", format!("{}->{}:{}", bdi.node_ids().0, bdi.node_ids().1, fmt_polar(bdi.v()))
                )
            )
            .collect::<Vec<_>>().join(", ");
        println!("{:<24}: [ {:>} ]", e.uid(), br_strs);
    }

    println!("\nCurrents:");
    for e in netw.elements() {
        let e = e.borrow();
        let br_data = e.branches().collect::<Vec<_>>();
        let br_strs = br_data
            .iter()
            .map(
                |bdi| format!(
                    "{:40}", format!("{}->{}:{:.4}", bdi.node_ids().0, bdi.node_ids().1, bdi.i())
                )
            )
            .collect::<Vec<_>>().join(", ");
        println!("{:<24}: [ {:<} ]", e.uid(), br_strs);
    }

    println!("\nPowers:");
    for e in netw.elements() {
        let e = e.borrow();
        let br_data = e.branches().collect::<Vec<_>>();
        let br_strs = br_data
            .iter()
            .map(
                |bdi| format!(
                    "{:40}", format!(
                        "{}->{}:{:.4}",
                        bdi.node_ids().0, bdi.node_ids().1, bdi.i().conj() * bdi.v()
                    )
                )
            )
            .collect::<Vec<_>>().join(", ");
        println!("{:<24}: [ {:<} ]", e.uid(), br_strs);
    }
}
