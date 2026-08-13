#!/usr/bin/env python3

import subprocess
import sys

import matplotlib.pyplot as plt
import pandas as pd

infile = sys.argv[1]
solver = sys.argv[2]

subprocess.run(f'./test_sgt -i {infile} -o sgt.csv'.split())
subprocess.run(f'cargo run --example test_griphos -- --solver {solver} {infile} griphos.csv'.split())

dfs = pd.read_csv("sgt.csv", index_col=0)
dfg = pd.read_csv("griphos.csv", index_col=0)
df = dfg.join(dfs)
df.columns = ['sgt', 'griphos']
df.sort_values('sgt', ascending=False, inplace=True)
df['diff'] = abs(df['sgt'] - df['griphos'])

plt.plot(df['diff'])
