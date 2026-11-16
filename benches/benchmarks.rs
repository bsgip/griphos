use std::hint::black_box;
use std::path::PathBuf;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use glob::glob;
use griphos::{ejson_phase_map_abcng, parse_ejson_file, solve_network, SolveNetworkResult, SolverStatusFlags, SolverPlugin};


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
    inventory::iter::<SolverPlugin>().map(|p| p.key).collect()
}

fn bench_combinations(c: &mut Criterion, reuse: bool) {
    let networks = all_test_networks();
    let solvers = all_solver_names();

    assert!(!networks.is_empty(), "no *.json files found under test_data/");
    assert!(!solvers.is_empty(), "no SolverPlugin entries registered in inventory");

    // One group per network (and mode), so solvers are compared against each
    // other within that network's report.
    for network_path in &networks {
        let network_name = network_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
        let filename = network_path.to_str().expect("non-UTF8 path");

        let group_name = if reuse { format!("{network_name}-reuse") } else { format!("{network_name}-noreuse") };
        let mut group = c.benchmark_group(group_name);

        // Replace with whatever defines "success" for your solver, e.g. a converged flag.
        fn is_success(res: &SolveNetworkResult) -> bool {
            res.statuses.iter().all(|s| s.status.flags.contains(SolverStatusFlags::SUCCESS))
        }

        for &solver_name in &solvers {
            // Untimed, once: parse + first solve.
            let mut netw = parse_ejson_file(filename, &ejson_phase_map_abcng()).unwrap();
            let mut result = Some(solve_network(&mut netw, solver_name, None).unwrap());

            let mut attempts: u64 = 0;
            let mut successes: u64 = 0;

            group.bench_function(BenchmarkId::from_parameter(solver_name), |b| {
                b.iter(|| {
                    let prev = result.take().unwrap();
                    // Reuse the previous solver state, or discard it and rebuild.
                    let prev_solvers = if reuse { Some(prev.solvers) } else { None };
                    let res = solve_network(&mut netw, solver_name, prev_solvers).unwrap();
                    attempts += 1;
                    if is_success(&res) {
                        successes += 1;
                    }
                    result = Some(res);
                    black_box(&result);
                });
            });
            if attempts > 0 {
                println!("Successes = {} out of {}", successes, attempts);
            }
        }

        group.finish();
    }
}

fn bench_noreuse(c: &mut Criterion) {
    bench_combinations(c, false);
}

fn bench_reuse(c: &mut Criterion) {
    bench_combinations(c, true);
}

criterion_group!(benches, bench_noreuse, bench_reuse);
criterion_main!(benches);
