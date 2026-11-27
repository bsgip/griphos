#!/usr/bin/env python3

"""Plots the distribution of node voltage magnitudes over time from griphos_solve_ts output.

Shows the median as a black line, with 40-60, 5-95 and 0-100 percentile bands in decreasingly intense blues.
Neutral and ground nodes are excluded.
"""

import argparse
import csv
from collections import defaultdict
from datetime import datetime

import matplotlib.dates as mdates
import matplotlib.pyplot as plt
import numpy as np

INK = "#191919"
INK_2 = "#666666"
GRID = "#e7e7e7"
AXIS = "#c2c2c2"
BANDS = [  # (lo, hi, colour), outermost first.
    (0, 100, "#cce3fd"),
    (5, 95, "#84b9f3"),
    (40, 60, "#338ced"),
]


def read_vmags(path):
    vmags = defaultdict(list)
    with open(path) as f:
        for row in csv.reader(f):
            t, node, vmag = row[0].strip(), row[1].strip(), float(row[2])
            if node.rsplit(":", 1)[-1] in ("N", "G"):
                continue
            vmags[datetime.fromisoformat(t)].append(vmag)
    times = sorted(vmags)
    return times, [np.array(vmags[t]) for t in times]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("infile", help="griphos_solve_ts output CSV")
    parser.add_argument("-o", "--outfile", help="save to this file instead of showing")
    args = parser.parse_args()

    times, vmags = read_vmags(args.infile)
    pct = lambda q: [np.percentile(v, q) for v in vmags]

    fig, ax = plt.subplots(figsize=(10, 5))
    for lo, hi, colour in BANDS:
        ax.fill_between(times, pct(lo), pct(hi), color=colour, linewidth=0, label=f"{lo}–{hi}th percentile")
    ax.plot(times, pct(50), color=INK, linewidth=2, label="Median")

    tz = times[0].tzinfo
    ax.xaxis.set_major_locator(mdates.HourLocator(interval=3, tz=tz))
    ax.xaxis.set_major_formatter(mdates.DateFormatter("%H:%M", tz=tz))
    ax.set_xlim(times[0], times[-1])
    ax.set_xlabel(f"Time ({times[0]:%Y-%m-%d %Z})", color=INK_2)
    ax.set_ylabel("Voltage magnitude (pu)", color=INK_2)
    ax.set_title(f"Node voltage magnitudes ({len(vmags[0])} nodes)", color=INK, loc="left")
    ax.grid(True, color=GRID, linewidth=0.8)
    ax.set_axisbelow(True)
    for side in ("top", "right"):
        ax.spines[side].set_visible(False)
    for side in ("left", "bottom"):
        ax.spines[side].set_color(AXIS)
    ax.tick_params(colors=INK_2)

    handles, labels = ax.get_legend_handles_labels()
    order = [3, 2, 1, 0]  # Median first, then bands inner to outer.
    ax.legend([handles[i] for i in order], [labels[i] for i in order], frameon=False, labelcolor=INK_2)
    fig.tight_layout()

    if args.outfile:
        fig.savefig(args.outfile, dpi=150)
    else:
        plt.show()


if __name__ == "__main__":
    main()
