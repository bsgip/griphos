use clap::Parser;
use griphos::*;

mod common;

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
    common::run_network(netw, &args.solver)
}
