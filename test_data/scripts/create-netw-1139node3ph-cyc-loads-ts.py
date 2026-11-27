#!/usr/bin/env python3

"""Creates one day of 30 minute loading data for netw-1139node3ph-cyc.json, for use with griphos_solve_ts.

Each load's e-JSON power is taken as its mean consumption. Consumption follows a residential profile (overnight
trough, morning and evening peaks) with per-load scaling, time shifts and AR(1) noise. Some loads also have rooftop PV
(unity power factor), which nets off consumption in the middle of the day to give a duck-shaped net load.
"""

import json
import math
import random
from pathlib import Path

test_data = Path(__file__).resolve().parent.parent
netw_file = test_data / "netw-1139node3ph-cyc.json"
out_file = test_data / "netw-1139node3ph-cyc-loads-ts.csv"

date = "2026-10-15"
tz = "+11:00"
n_steps = 48
dt_h = 0.5
max_phases = 3

random.seed(1139)


def bump(t, centre, width):
    d = (t - centre + 12) % 24 - 12  # Wrap around midnight.
    return math.exp(-0.5 * (d / width) ** 2)


def consumption_shape(t, shift):
    t = t - shift
    return 0.45 + 0.45 * bump(t, 7.5, 1.2) + 0.25 * bump(t, 13.0, 3.0) + 1.0 * bump(t, 19.0, 1.8)


def pv_shape(t):
    # Clear-sky bell between ~06:30 and ~19:30.
    return max(0.0, math.sin(math.pi * (t - 6.5) / 13.0)) ** 1.5 if 6.5 < t < 19.5 else 0.0


times = [i * dt_h for i in range(n_steps)]
mean_shape = sum(consumption_shape(t, 0.0) for t in times) / n_steps
cloud = [1.0 - 0.25 * bump(t, 14.5, 0.7) for t in times]  # A passing cloud band, common to all PV.


def ar1(n, sigma, rho=0.7):
    x, out = 0.0, []
    for _ in range(n):
        x = rho * x + random.gauss(0.0, sigma)
        out.append(x)
    return out


def load_series(s_base):
    """Returns a list over time of per-phase (p, q) in W, VAr."""
    n_ph = len(s_base)
    scale = random.lognormvariate(0.0, 0.25)
    shift = random.gauss(0.0, 0.5)
    has_pv = random.random() < 0.4
    pv_peak = random.uniform(0.8, 2.0) * sum(p for p, _ in s_base) / n_ph if has_pv else 0.0
    imbalance = [random.uniform(0.85, 1.15) for _ in range(n_ph)]
    noise = [ar1(n_steps, 0.08) for _ in range(n_ph)]
    series = []
    for i, t in enumerate(times):
        cons = scale * consumption_shape(t, shift) / mean_shape
        pv = pv_peak * pv_shape(t) * cloud[i]
        row = []
        for ph, (p0, q0) in enumerate(s_base):
            f = max(0.05, cons * imbalance[ph] * (1.0 + noise[ph][i]))
            row.append((p0 * f - pv, q0 * f))
        series.append(row)
    return series


def main():
    netw = json.loads(netw_file.read_text())
    loads = [c for c in netw["components"] if c["type"] == "Load"]
    series = {ld["id"]: load_series(ld["s"]) for ld in loads}
    with out_file.open("w") as f:
        for i, t in enumerate(times):
            ts = f"{date}T{int(t):02d}:{int(round(60 * (t % 1))):02d}:00{tz}"
            for ld in loads:
                vals = [f"{x:.4f}" for p, q in series[ld["id"]][i] for x in (p, q)]
                vals += [""] * (2 * max_phases - len(vals))
                f.write(",".join([ts, ld["id"]] + vals) + "\n")


if __name__ == "__main__":
    main()
