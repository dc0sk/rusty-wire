#!/usr/bin/env bash
set -euo pipefail

# Fit conductor-diameter correction constants from NEC/reference sweep data.
#
# Expected CSV columns (header required):
#   diameter_mm,length_factor
# where length_factor is the measured multiplier relative to a 2.0 mm baseline.
# Example: if 4.0 mm needs 0.992x of the 2.0 mm length, use 0.992.
#
# Model form (before clamp):
#   F_d(d) = 1 - k * ln(d / d0), with d0 = 2.0 mm
#
# This script solves least-squares k with intercept fixed to 1.0:
#   k = sum( x * (1 - y) ) / sum( x^2 )
#   x = ln(d / d0), y = length_factor
#
# Usage:
#   ./scripts/calibrate-conductor-model.sh [path/to/reference.csv]

DATA_FILE="${1:-docs/data/nec_conductor_reference.csv}"
D0_MM="2.0"

if [[ ! -f "$DATA_FILE" ]]; then
  echo "Error: data file not found: $DATA_FILE" >&2
  exit 1
fi

if ! head -n 1 "$DATA_FILE" | grep -Eq '^diameter_mm,length_factor$'; then
  echo "Error: CSV header must be exactly: diameter_mm,length_factor" >&2
  exit 1
fi

awk -F',' -v d0="$D0_MM" '
BEGIN {
  n = 0
  sum_x_num = 0.0
  sum_x2 = 0.0
  min_d = 1e9
  max_d = 0.0
}
NR == 1 { next }
{
  if (NF != 2) {
    printf("Error: malformed row %d\n", NR) > "/dev/stderr"
    exit 2
  }

  d = $1 + 0.0
  y = $2 + 0.0

  if (d <= 0.0) {
    printf("Error: diameter must be > 0 at row %d\n", NR) > "/dev/stderr"
    exit 2
  }
  if (y <= 0.0) {
    printf("Error: length_factor must be > 0 at row %d\n", NR) > "/dev/stderr"
    exit 2
  }

  x = log(d / d0)
  sum_x_num += x * (1.0 - y)
  sum_x2 += x * x

  rows_d[n] = d
  rows_y[n] = y
  rows_x[n] = x
  n++

  if (d < min_d) min_d = d
  if (d > max_d) max_d = d
}
END {
  if (n < 2) {
    print "Error: need at least 2 data rows" > "/dev/stderr"
    exit 2
  }
  if (sum_x2 <= 0.0) {
    print "Error: degenerate data (sum_x2 == 0)" > "/dev/stderr"
    exit 2
  }

  k = sum_x_num / sum_x2

  sse = 0.0
  max_abs_err = 0.0
  min_pred = 1e9
  max_pred = -1e9

  for (i = 0; i < n; i++) {
    pred = 1.0 - k * rows_x[i]
    err = pred - rows_y[i]
    ae = err < 0 ? -err : err

    sse += err * err
    if (ae > max_abs_err) max_abs_err = ae
    if (pred < min_pred) min_pred = pred
    if (pred > max_pred) max_pred = pred
  }

  rmse = sqrt(sse / n)

  printf("Data file: %s\n", ARGV[1])
  printf("Rows used: %d\n", n)
  printf("Diameter span: %.3f .. %.3f mm\n", min_d, max_d)
  printf("\n")
  printf("Suggested model constants:\n")
  printf("  d0_mm = %.3f\n", d0)
  printf("  k = %.6f\n", k)
  printf("\n")
  printf("Model quality (unclamped):\n")
  printf("  RMSE = %.6f\n", rmse)
  printf("  MaxAbsError = %.6f\n", max_abs_err)
  printf("\n")
  printf("Suggested clamp bounds for your observed span:\n")
  printf("  min_factor = %.6f\n", min_pred)
  printf("  max_factor = %.6f\n", max_pred)
  printf("\n")
  printf("To apply manually in src/calculations.rs:\n")
  printf("  (1.0 - %.6f * (d_mm / DEFAULT_CONDUCTOR_DIAMETER_MM).ln()).clamp(MIN_CLAMP, MAX_CLAMP)\n", k)
}
' "$DATA_FILE"
