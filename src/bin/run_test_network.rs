use clap::Parser;
use griphos::*;

mod common;

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
    let netw = if args.netw_name == "test_netw_y" {
        test_netw_y(c!(0.23, 0.45))
    } else if args.netw_name == "test_netw_y_r" {
        test_netw_y(c!(0.23, 0.0))
    } else if args.netw_name == "test_netw_yy" {
        test_netw_yy(c!(0.23, 0.45))
    } else if args.netw_name == "test_netw_yy_r" {
        test_netw_yy(c!(0.23, 0.0))
    } else if args.netw_name == "test_netw_ys" {
        test_netw_ys(c!(0.23, 0.45), c!(0.05, 0.04))
    } else if args.netw_name == "test_netw_ys_r" {
        test_netw_ys(c!(0.23, 0.0), c!(0.05, 0.0))
    } else if args.netw_name == "test_netw_s" {
        test_netw_s(c!(0.23, 0.45))
    } else if args.netw_name == "test_netw_s_r" {
        test_netw_s(c!(0.23, 0.0))
    } else if args.netw_name == "cycles" {
        test_netw_cycles()
    } else if args.netw_name == "test_netw_large_a" {
        test_netw_large_a()
    } else if args.netw_name == "test_netw_large_b" {
        test_netw_large_b()
    } else {
        panic!("Bad network name");
    };
    common::run_network(netw, &args.solver)
}
