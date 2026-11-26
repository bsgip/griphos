use clap::Parser;

use griphos::{ejson_phase_map_abcng, NetworkSolvers, parse_ejson_file, solve_network};

#[derive(Parser)]
struct Args {
    infile: String,
    solver: String,

    #[arg(long, default_value_t = 50)]
    iters_outer: u32,

    #[arg(long, default_value_t = 50)]
    iters_inner: u32,
}

fn main() {
    let args = Args::parse();

    // Parse once — profiling the parser too would blur the solver-specific hotspots.
    let mut netw = parse_ejson_file(&args.infile, &ejson_phase_map_abcng())
        .unwrap_or_else(|e| panic!("Error parsing ejson file {}: {}", args.infile, e));

    for _ in 0..args.iters_outer {
        let mut solvers: Option<NetworkSolvers> = None;
        for _ in 0..args.iters_inner {
            let result = std::hint::black_box(solve_network(&mut netw, &args.solver, solvers).unwrap());
            solvers = Some(result.solvers);
        }
    }
}
