#!/usr/bin/env python3

"""Creates synthetic time series loading data for an e-JSON network, for use with griphos_solve_ts.

Each load's e-JSON power is taken as its mean consumption. Consumption follows a residential profile (overnight
trough, morning and evening peaks) with per-load scaling, time shifts and AR(1) noise. Some loads also have rooftop PV
(unity power factor), which nets off consumption in the middle of the day to give a duck-shaped net load.
"""

import argparse
import json
import math
import random
import sys
from datetime import datetime, timedelta


def bump(t, centre, width):
    d = (t - centre + 12) % 24 - 12  # Wrap around midnight.
    return math.exp(-0.5 * (d / width) ** 2)


def consumption_shape(t, shift):
    t = t - shift
    return 0.45 + 0.45 * bump(t, 7.5, 1.2) + 0.25 * bump(t, 13.0, 3.0) + 1.0 * bump(t, 19.0, 1.8)


def pv_shape(t):
    # Clear-sky bell between ~06:30 and ~19:30.
    t = t % 24
    return max(0.0, math.sin(math.pi * (t - 6.5) / 13.0)) ** 1.5 if 6.5 < t < 19.5 else 0.0


def cloud(t):
    return 1.0 - 0.25 * bump(t, 14.5, 0.7)  # A passing cloud band each day, common to all PV.


def ar1(n, sigma, rho):
    x, out = 0.0, []
    for _ in range(n):
        x = rho * x + random.gauss(0.0, sigma)
        out.append(x)
    return out


def load_series(s_base, times, mean_shape, rho, sigma):
    """Returns a list over time of per-phase (p, q) in W, VAr."""
    n_ph = len(s_base)
    scale = random.lognormvariate(0.0, 0.25)
    shift = random.gauss(0.0, 0.5)
    has_pv = random.random() < 0.4
    pv_peak = random.uniform(0.8, 2.0) * sum(p for p, _ in s_base) / n_ph if has_pv else 0.0
    imbalance = [random.uniform(0.85, 1.15) for _ in range(n_ph)]
    noise = [ar1(len(times), sigma, rho) for _ in range(n_ph)]
    series = []
    for i, t in enumerate(times):
        cons = scale * consumption_shape(t, shift) / mean_shape
        pv = pv_peak * pv_shape(t) * cloud(t)
        row = []
        for ph, (p0, q0) in enumerate(s_base):
            f = max(0.05, cons * imbalance[ph] * (1.0 + noise[ph][i]))
            row.append((p0 * f - pv, q0 * f))
        series.append(row)
    return series


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("netw_file", help="e-JSON network file")
    parser.add_argument("timestep_min", type=float, help="timestep in minutes")
    parser.add_argument("total_hours", type=float, help="total time in hours")
    parser.add_argument("-o", "--outfile", help="output CSV file (default: stdout)")
    parser.add_argument("--start", default="2026-10-15T00:00:00+11:00", help="ISO8601 start time (local time of day)")
    parser.add_argument("--seed", type=int, default=1139, help="random seed")
    args = parser.parse_args()

    random.seed(args.seed)
    start = datetime.fromisoformat(args.start)
    dt_h = args.timestep_min / 60.0
    n_steps = round(args.total_hours / dt_h)
    t0 = start.hour + start.minute / 60.0 + start.second / 3600.0
    times = [t0 + i * dt_h for i in range(n_steps)]

    # Normalise consumption to a daily mean of 1.
    day_times = [i * dt_h for i in range(max(1, round(24.0 / dt_h)))]
    mean_shape = sum(consumption_shape(t, 0.0) for t in day_times) / len(day_times)

    # AR(1) noise, with correlation time and stationary std independent of the timestep (rho = 0.7 at 30 minutes).
    rho = 0.7 ** (dt_h / 0.5)
    sigma = 0.08 * math.sqrt((1.0 - rho**2) / (1.0 - 0.7**2))

    with open(args.netw_file) as f:
        netw = json.load(f)
    loads = []
    for c in netw["components"]:
        if c["type"] != "Load":
            continue
        if not c.get("s"):
            print(f"Skipping load {c['id']} with no constant power.", file=sys.stderr)
            continue
        loads.append(c)
    max_phases = max((len(ld["s"]) for ld in loads), default=1)
    series = {ld["id"]: load_series(ld["s"], times, mean_shape, rho, sigma) for ld in loads}

    out = open(args.outfile, "w") if args.outfile else sys.stdout
    with out:
        for i in range(n_steps):
            ts = (start + timedelta(hours=i * dt_h)).isoformat()
            for ld in loads:
                vals = [f"{x:.4f}" for p, q in series[ld["id"]][i] for x in (p, q)]
                vals += [""] * (2 * max_phases - len(vals))
                out.write(",".join([ts, ld["id"]] + vals) + "\n")


if __name__ == "__main__":
    main()
