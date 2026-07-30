use clap::Parser;
use griphos::*;

fn run_network(mut netw: Network, solver_name: &str) {
    solve_network(&mut netw, solver_name, None).unwrap();
    netw.foreach_bus(|b| {println!("v {} = {:?}", b.uid, b.vs().map(|x| x.norm()).collect::<Vec<f64>>());});
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
    run_network(netw, &args.solver)
}
