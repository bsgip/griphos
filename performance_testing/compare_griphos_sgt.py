#!/usr/bin/env python3

import subprocess
import sys

import matplotlib.pyplot as plt
import pandas as pd

infile = sys.argv[1]
solver = sys.argv[2]

subprocess.run(f'./test_sgt -i {infile} -o sgt.csv'.split(), check=False)
subprocess.run(f'cargo run --example test_griphos -- --solver {solver} {infile} griphos.csv'.split(), check=False)

dfs = pd.read_csv("sgt.csv", index_col=0, header=None)
dfs.columns = ['s_vmag', 's_varg']
dfg = pd.read_csv("griphos.csv", index_col=0, header=None)
dfg.columns = ['g_vmag', 'g_varg']
df = dfg.join(dfs)
df.sort_values('s_vmag', ascending=False, inplace=True)
df['diff'] = abs(df['s_vmag'] - df['g_vmag'])

plt.plot(df['diff'])
plt.show()
