#!/usr/bin/env bash
# Runs the throughput matrix and appends one JSON line per run to OUT.
# Repetitions are the outer loop, so load changes from other processes spread across
# every mode instead of biasing one.
#
# Usage: bench.sh MANIFEST CAPTURE_DIR OUT.jsonl
# Environment: REPS (3), SLICES ("window all"), THREADS ("1 10"),
#   MODES ("read value typed prefilter cache cache-verify"), BIN.
set -euo pipefail

manifest=$1
capture=$2
out=$3
bin=${BIN:-"$(dirname "$0")/target/release/log-throughput-spike"}
reps=${REPS:-3}
slices=${SLICES:-"window all"}
threads_list=${THREADS:-"1 10"}
modes=${MODES:-"read value typed prefilter cache cache-verify"}

for rep in $(seq 0 $((reps - 1))); do
  for slice in $slices; do
    for threads in $threads_list; do
      for mode in $modes; do
        "$bin" run --manifest "$manifest" --capture "$capture" --mode "$mode" \
          --threads "$threads" --slice "$slice" --rep "$rep" >>"$out"
      done
    done
  done
done
