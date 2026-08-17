#!/bin/bash

FEEDER=$1
SOLVER=$2
N=$3

samply record --no-open ../target/release/examples/profile $FEEDER $SOLVER $N 2>&1 \
  | grep --line-buffered '^https://' \
  | xargs -n1 open -a "Google Chrome"
