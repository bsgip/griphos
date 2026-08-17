use griphos::{ejson_phase_map_abcng, parse_ejson_file, solve_network};
// use griphos::configure_logging;

fn main() {
    // configure_logging(); // For debugging

    let args: Vec<String> = std::env::args().collect();
    let [network_path, solver_name] = match args.get(1..3) {
        Some([a, b]) => [a.clone(), b.clone()],
        _ => {
            eprintln!("usage: {} <network_path> <solver_name> [iterations]", args[0]);
            std::process::exit(1);
        }
    };
    let iterations: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(500);

    // Parse once — profiling the parser too would blur the solver-specific hotspots.
    let mut netw = parse_ejson_file(&network_path, &ejson_phase_map_abcng())
        .unwrap_or_else(|e| panic!("Error parsing ejson file {network_path}: {e}"));

    for _ in 0..iterations {
        std::hint::black_box(solve_network(&mut netw, &solver_name, None).unwrap());
    }
}
