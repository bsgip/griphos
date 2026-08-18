#!/bin/bash

FEEDER=$1
SOLVER=$2
N_OUTER=$3
N_INNER=$4

samply record --no-open ../target/release/examples/profile --iters-outer $N_OUTER --iters-inner $N_INNER $FEEDER $SOLVER 2>&1 \
  | grep --line-buffered '^https://' \
  | xargs -n1 open -a "Google Chrome"
