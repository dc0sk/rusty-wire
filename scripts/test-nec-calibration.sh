#!/usr/bin/env bash
set -euo pipefail

# Regression checks for the conductor calibration workflow.
#
# Verifies that the template NEC/reference dataset still fits to the documented
# constants and that parser hardening (comments/blank lines) behaves correctly.
#
# Usage:
#   ./scripts/test-nec-calibration.sh

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CAL_SCRIPT="$ROOT_DIR/scripts/calibrate-conductor-model.sh"
TEMPLATE_CSV="$ROOT_DIR/docs/data/nec_conductor_reference.csv"

if [[ ! -x "$CAL_SCRIPT" ]]; then
  echo "Error: calibration script is not executable: $CAL_SCRIPT" >&2
  exit 1
fi

if [[ ! -f "$TEMPLATE_CSV" ]]; then
  echo "Error: template CSV not found: $TEMPLATE_CSV" >&2
  exit 1
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

echo "[1/3] Validating template fit constants"
out_template="$tmpdir/template.out"
"$CAL_SCRIPT" "$TEMPLATE_CSV" > "$out_template"

grep -F "k = 0.011542" "$out_template" >/dev/null
grep -F "RMSE = 0.000000" "$out_template" >/dev/null
grep -F "min_factor = 0.992000" "$out_template" >/dev/null
grep -F "max_factor = 1.008000" "$out_template" >/dev/null

echo "[2/3] Validating parser tolerance for comments/blank lines"
commented_csv="$tmpdir/commented.csv"
cat > "$commented_csv" <<'CSV'
diameter_mm,length_factor
# Baseline
2.0,1.000

# Thinner wire
1.0,1.008

# Thicker wire
4.0,0.992
CSV

out_commented="$tmpdir/commented.out"
"$CAL_SCRIPT" "$commented_csv" > "$out_commented"
grep -F "Rows used: 3" "$out_commented" >/dev/null
grep -F "k = 0.011542" "$out_commented" >/dev/null

echo "[3/3] Validating malformed row rejection"
malformed_csv="$tmpdir/malformed.csv"
cat > "$malformed_csv" <<'CSV'
diameter_mm,length_factor
2.0,1.000,extra
CSV

if "$CAL_SCRIPT" "$malformed_csv" >/dev/null 2>&1; then
  echo "FAIL: malformed CSV row was accepted unexpectedly" >&2
  exit 1
fi

echo "PASS: NEC calibration regression checks passed."
