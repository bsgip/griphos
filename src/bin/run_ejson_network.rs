use clap::Parser;
use log::{debug, info};

use crate::configure_logging;
use griphos::{network_solver_nodal::NodalRectSolver, *};

fn run_network(mut netw: Network) {
    let master = NetworkModel::new_master(&netw).unwrap();

    debug!("Nodes:");
    for (i, n) in master.nodes.iter().enumerate() {
        let nm = n.name();
        debug!("{i:10}: {nm}");
    }

    let islands = master.islands().unwrap();

    info!("There are {} islands", islands.len());
    for nmod in islands.into_iter() {
        info!("Island:");
        let mut solver = NodalRectSolver::new(nmod);
        solver.solve();
        solver.propagate_to_network(&mut netw);
    }

    for b in netw.buses() {
        println!("v {} = {:?}", b.uid, b.vs().map(|x| x.norm()).collect::<Vec<f64>>());
    }
}

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    solver: String,
    infile: String,
    outfile: String,
}

fn main() {
    configure_logging();
    let args = Args::parse();
    let netw = parse_ejson_file(
        args.infile.as_str(),
        &ejson_phase_map_abcng()).expect("Error parsing ejson file {netw_fname}");
    run_network(netw)
}
