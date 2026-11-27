use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};

use clap::Parser;

use griphos::*;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    solver: String,
    /// Reuse the solvers from the previous timestep.
    #[arg(short, long)]
    reuse_solvers: bool,
    infile: String,
    loads_file: String,
    outfile: String,
}

const S_FACTOR: f64 = 1e-6; // Convert CSV W (as in e-JSON) to internal MW

/// Sets a load's constant power from the value columns of a loading CSV row.
fn set_load(netw: &Network, uid: &str, vals: &[String]) {
    let n_vals = vals.iter().rposition(|v| !v.is_empty()).map_or(0, |i| i + 1);
    let vals: Vec<f64> =
        vals[..n_vals].iter().map(|v| v.parse().unwrap_or_else(|_| panic!("Load {uid}: bad value '{v}'"))).collect();
    assert!(vals.len() >= 2 && vals.len() % 2 == 0, "Load {uid}: need an even number (>= 2) of values");
    let s: Vec<c64> = vals.chunks(2).map(|c| c64::new(c[0], c[1]) * S_FACTOR).collect();

    let ld = netw.element(uid).and_then(|e| e.downcast::<Load>()).unwrap_or_else(|| panic!("No load with UID {uid}"));
    let mut ld = ld.borrow_mut();
    let s_const = ld.ld_s_const_mut().unwrap_or_else(|| panic!("Load {uid} has no constant power component"));
    let n_ph = s_const.len();
    if s.len() == 1 {
        s_const.fill(s[0] / n_ph as f64);
    } else {
        assert_eq!(s.len(), n_ph, "Load {uid}: expected 1 or {n_ph} complex values");
        s_const.copy_from_slice(&s);
    }
}

fn main() {
    configure_logging();
    let args = Args::parse();

    let t_start = Instant::now();
    let mut netw =
        parse_ejson_file(&args.infile, &ejson_phase_map_abcng()).expect("Error parsing ejson file {netw_fname}");
    let rows = read_csv(&args.loads_file).expect("Error reading loads file");

    // Group consecutive rows by timestep.
    let mut timesteps: Vec<(&str, Vec<&Vec<String>>)> = Vec::new();
    for row in rows.iter() {
        assert!(row.len() >= 4, "Loads file row too short: {row:?}");
        match timesteps.last_mut() {
            Some((t, t_rows)) if *t == row[0] => t_rows.push(row),
            _ => timesteps.push((&row[0], vec![row])),
        }
    }

    let parse_time = t_start.elapsed();

    let mut solve_time = Duration::ZERO;
    let mut write_time = Duration::ZERO;
    let mut outfile = File::create(&args.outfile).unwrap();
    let mut solvers = None;
    let n_timesteps = timesteps.len();
    for (t, t_rows) in timesteps {
        let t_solve = Instant::now();
        for row in t_rows {
            set_load(&netw, &row[1], &row[2..]);
        }
        let result = solve_network(&mut netw, &args.solver, solvers.take()).unwrap();
        if args.reuse_solvers {
            solvers = Some(result.solvers);
        }
        solve_time += t_solve.elapsed();

        let t_write = Instant::now();
        let all_ok = result.statuses.iter().all(|x| x.status.flags.contains(SolverStatusFlags::SUCCESS));
        for (status_i, status) in result.statuses.iter().enumerate() {
            println!("{t}: island {}:\n{:?}", status_i, status.status);
        }
        if all_ok {
            for b in netw.buses() {
                let b = b.borrow();
                for nd in b.nodes.iter() {
                    let v = nd.v();
                    writeln!(
                        outfile,
                        "{t},{:20}{:9.5},{:9.5}",
                        format!("{}:{},", b.uid, nd.phase()),
                        v.norm() / b.v_base,
                        v.arg()
                    )
                    .unwrap();
                }
            }
        }
        write_time += t_write.elapsed();
    }

    println!("Parse time: {:.3} s", parse_time.as_secs_f64());
    println!("Solve time: {:.3} s", solve_time.as_secs_f64());
    println!("Solve time per timestep: {:.3} ms", 1e3 * solve_time.as_secs_f64() / n_timesteps as f64);
    println!("Write time: {:.3} s", write_time.as_secs_f64());
}
