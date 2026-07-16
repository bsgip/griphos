use clap::Parser;
use log::{info};

use griphos::*;

fn run_network(mut netw: Network, solver_name: &str) {
    let master = NetworkModel::new_master(&netw).unwrap();
    let islands = master.islands().unwrap();

    info!("There are {} islands", islands.len());
    for nmod in islands.into_iter() {
        info!("Island:");
        let mut solver: Box<dyn NetworkSolver> = make_solver(solver_name, nmod);
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
    netw_name: String,
    outfile: String,
}

fn main() {
    let args = Args::parse();
    configure_logging();
    let netw = if args.netw_name == "netw_y" {
        netw_y(c!(0.23, 0.45))
    } else if args.netw_name == "netw_y_r" {
        netw_y(c!(0.23, 0.0))
    } else if args.netw_name == "netw_yy" {
        netw_yy(c!(0.23, 0.45))
    } else if args.netw_name == "netw_yy_r" {
        netw_yy(c!(0.23, 0.0))
    } else if args.netw_name == "netw_ys" {
        netw_ys(c!(0.23, 0.45), c!(0.05, 0.04))
    } else if args.netw_name == "netw_ys_r" {
        netw_ys(c!(0.23, 0.0), c!(0.05, 0.0))
    } else if args.netw_name == "netw_s" {
        netw_s(c!(0.23, 0.45))
    } else if args.netw_name == "netw_s_r" {
        netw_s(c!(0.23, 0.0))
    } else if args.netw_name == "cycles" {
        netw_cycles()
    } else {
        panic!("Bad network name");
    };
    run_network(netw, &args.solver)
}
