// One-shot diagnostic pass over every (network, solver) combination:
// prints total time, per-phase breakdown (from griphos::timing), and
// success/failure. Separate from benches/solver_bench.rs on purpose -
// this runs each combination exactly once, whereas Criterion runs each
// one hundreds of times for statistical timing.
//
// --dir selects which directory to look in (non-recursive), --networks
// filters filenames within it by regex, --solvers filters solver names
// by regex.
//
// Examples:
//   cargo run --example compare_solvers --release
//   cargo run --example compare_solvers --release -- --dir test_data --networks 'netw_a.json'
//
// Run with --release: debug-mode phase timings aren't representative.

use std::path::{Path, PathBuf};

use clap::Parser;
use griphos::{configure_logging, SolverPlugin, SolverStatusFlags, ejson_phase_map_abcng, parse_ejson_file, solve_network};
use regex::Regex;

#[derive(Parser)]
#[command(
    about = "Solver diagnostics run over all (network, solver) combinations."
)]
struct Args {
    /// Directory to look in for network files (not recursive). Defaults to CWD.
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,

    /// Regex to select filenames within --dir.
    #[arg(short, long, default_value = r".*\.json$")]
    networks: String,

    /// Regex to select solver names.
    #[arg(short, long, default_value = ".*")]
    solvers: String,
}

fn discover_networks(dir: &Path, name_regex: &Regex) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read directory '{}': {e}", dir.display()))
        .filter_map(|entry| match entry {
            Ok(e) => Some(e.path()),
            Err(e) => {
                eprintln!("warning: skipping unreadable entry: {e}");
                None
            }
        })
        .filter(|path| path.is_file())
        .filter(|path| {
            let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name_regex.is_match(filename)
        })
        .collect()
}

fn all_solver_names() -> Vec<&'static str> {
    inventory::iter::<SolverPlugin>().map(|p| p.name).collect()
}

fn main() {
    configure_logging();

    let args = Args::parse();

    let network_regex = Regex::new(&args.networks)
        .unwrap_or_else(|e| panic!("invalid --networks regex '{}': {e}", args.networks));
    let solver_regex = Regex::new(&args.solvers)
        .unwrap_or_else(|e| panic!("invalid --solvers regex '{}': {e}", args.solvers));

    let networks = discover_networks(&args.dir, &network_regex);

    let mut solvers = all_solver_names();
    solvers.retain(|s| solver_regex.is_match(s));

    assert!(
        !networks.is_empty(),
        "no files in '{}' matched --networks '{}'",
        args.dir.display(),
        args.networks
    );
    assert!(!solvers.is_empty(), "no solvers matched --solvers '{}'", args.solvers);

    println!("{:<20} {:<20} {:<6} {:<6} {:>10}", "network", "solver", "ok", "iters", "walltime_ms");
    println!("{}", "-".repeat(60));

    for network_path in &networks {
        let network_name = network_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let network_filename = network_path.to_str().expect("non-UTF8 path");

        for &solver_name in &solvers {
            let mut netw = parse_ejson_file(network_filename, &ejson_phase_map_abcng())
                .expect("could not parse network {network_filename");
            let solve_result = solve_network(&mut netw, solver_name, None).unwrap();
            let all_ok = solve_result.statuses.iter().all(|x| x.status.flags.contains(SolverStatusFlags::SUCCESS));
            let n_iter: usize = solve_result.statuses.iter().map(|status| status.iteration_statuses.len()).sum();
            let tot_time_ms: f64 = 1e3 * solve_result.statuses.iter().map(|x| x.status.total_time_s).sum::<f64>();
            println!("{network_name:<20} {solver_name:<20} {all_ok:<6} {n_iter:<6} {tot_time_ms:>10.3?}");
        }
    }
}
