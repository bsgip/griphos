use std::fs::File;
use std::io::{Write};

use clap::Parser;
use crate::configure_logging;

use griphos::*;

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

    let solver_name = &args.solver;

    let mut netw = parse_ejson_file(&args.infile, &ejson_phase_map_abcng())
        .expect("Error parsing ejson file {netw_fname}");
    let result = solve_network(&mut netw, solver_name, None).unwrap();

    let mut outfile = File::create(&args.outfile).unwrap();
    let all_ok = result.statuses.iter().all(|x| x.status.flags.contains(SolverStatusFlags::SUCCESS));
    for (status_i, status) in result.statuses.iter().enumerate() {
        println!("Island {}:\n{:?}", status_i, status);
    }
    if all_ok {
        for b in netw.buses() {
            let b = b.borrow();
            for t in b.terms.iter() {
                writeln!(outfile, "{}:{}, {}", b.uid, t.phase, t.v.norm()/b.v_base).unwrap();
            }
        }
    }
}
