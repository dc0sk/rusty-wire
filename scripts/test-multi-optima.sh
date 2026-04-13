#!/usr/bin/env bash
set -euo pipefail

# Verifies that rusty-wire can surface multiple non-resonant optima
# (equal-tie optima and/or local search-window optima).
#
# Exit codes:
#   0 -> at least one multi-optima case found
#   1 -> no multi-optima case found in sweep
#
# Usage:
#   scripts/test-multi-optima.sh
#
# Optional env overrides:
#   BIN=./target/debug/rusty-wire
#   SWEEP_OUT=/tmp/rw_sweep_result.txt

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${BIN:-$ROOT_DIR/target/debug/rusty-wire}"
SWEEP_OUT="${SWEEP_OUT:-/tmp/rw_sweep_result.txt}"

cd "$ROOT_DIR"
cargo build --quiet

found=0
: > "$SWEEP_OUT"

for bands in \
  "160m" "80m" "60m" "40m" "30m" "20m" "17m" "15m" "12m" "10m" \
  "160m,80m" "80m,60m" "60m,40m" "40m,30m" "30m,20m" "20m,17m" "40m-10m"
do
  for vf in 0.50 0.55 0.60 0.65 0.70 0.75 0.80 0.85 0.90 0.95 1.00
  do
    for min in 6 7 8 9 10 11 12 13 14 15
    do
      for max in 16 18 20 22 24 26 28 30 32 35
      do
        [[ "$max" -gt "$min" ]] || continue

        out="$($BIN \
          --mode non-resonant \
          --bands "$bands" \
          --velocity "$vf" \
          --wire-min "$min" \
          --wire-max "$max" \
          --units both)"

        if printf "%s" "$out" | grep -q -E "Additional equal optima in range|Local optima in search window"; then
          {
            echo "FOUND_MULTIPLE"
            echo "bands=$bands vf=$vf min=$min max=$max"
            printf "%s\n" "$out" | grep -E "Best non-resonant|Additional equal optima|Local optima in search window|^[[:space:]]+[0-9]+\\."
          } > "$SWEEP_OUT"

          cat "$SWEEP_OUT"
          echo
          echo "PASS: multi-optima behavior is reachable."
          found=1
          break 4
        fi
      done
    done
  done
done

if [[ "$found" -eq 0 ]]; then
  {
    echo "NO_MULTIPLE_FOUND_IN_SWEEP"
    echo "Sweep completed but no multi-optima output was found."
  } > "$SWEEP_OUT"

  cat "$SWEEP_OUT"
  echo
  echo "FAIL: multi-optima behavior not observed in sampled parameter space."
  exit 1
fi
