use std::hint::black_box;
use std::path::PathBuf;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use glob::glob;
use griphos::{ejson_phase_map_abcng, parse_ejson_file, solve_network, SolveNetworkResult, SolverPlugin};


fn all_test_networks() -> Vec<PathBuf> {
    glob("test_data/*.json")
        .expect("invalid glob pattern")
        .filter_map(|entry| match entry {
            Ok(path) => Some(path),
            Err(e) => {
                eprintln!("warning: skipping unreadable path: {e}");
                None
            }
        })
        .collect()
}

fn all_solver_names() -> Vec<&'static str> {
    inventory::iter::<SolverPlugin>().map(|p| p.name).collect()
}

fn solve(network_path: &str, solver_name: &str) -> SolveNetworkResult {
    let mut netw = parse_ejson_file(network_path, &ejson_phase_map_abcng())
        .unwrap_or_else(|e| panic!("Error parsing ejson file {network_path}: {e}"));
    solve_network(&mut netw, solver_name, None).unwrap()
}

fn bench_all_combinations(c: &mut Criterion) {
    let networks = all_test_networks();
    let solvers = all_solver_names();

    assert!(!networks.is_empty(), "no *.json files found under test_data/");
    assert!(!solvers.is_empty(), "no SolverPlugin entries registered in inventory");

    // One group per network => solvers are compared against each other within
    // that network's report; Criterion never compares across networks, since
    // each network gets its own independent group.
    for network_path in &networks {
        let network_name = network_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
        let network_filename = network_path.to_str().expect("non-UTF8 path").to_string();

        let mut group = c.benchmark_group(&network_name);

        for &solver_name in &solvers {
            group.bench_with_input(
                BenchmarkId::from_parameter(solver_name),
                &network_filename,
                |b, filename| {
                    // solve(network_path, solver_name) - note the order matches `solve`'s signature.
                    b.iter(|| black_box(solve(black_box(filename), black_box(solver_name))));
                },
            );
        }

        group.finish();
    }
}

criterion_group!(benches, bench_all_combinations);
criterion_main!(benches);
