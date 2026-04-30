#!/usr/bin/env bash
set -euo pipefail

# Run the full Rusty Wire regression suite in one command.
#
# Designed for CI and pre-push local use.
#
# Runs in order:
#   1. cargo fmt --check  (format gate)
#   2. cargo check        (compile gate)
#   3. cargo clippy       (lint gate, -D warnings)
#   4. cargo test         (unit + integration)
#   5. scripts/test-itu-region-bands.sh
#   6. scripts/test-multi-optima.sh
#   7. scripts/test-nec-calibration.sh
#
# Exit codes:
#   0 -> all checks passed
#   1 -> at least one check failed
#
# Usage:
#   ./scripts/test-all.sh
#
# Optional env overrides (forwarded to sub-scripts):
#   BIN      — path to rusty-wire binary (default: target/debug/rusty-wire)
#   SWEEP_OUT — path for multi-optima sweep output (default: /tmp/rw_sweep_result.txt)

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PASS=0
FAIL=1
overall=$PASS

run_step() {
  local label="$1"
  shift
  echo "==> $label"
  if "$@"; then
    echo "    OK"
  else
    echo "    FAIL: $label" >&2
    overall=$FAIL
  fi
  echo
}

run_step "cargo fmt --check"       cargo fmt --all -- --check
run_step "cargo check"             cargo check
run_step "cargo clippy"            cargo clippy --all-targets -- -D warnings
run_step "cargo test"              cargo test
run_step "ITU region band checks"  "$ROOT_DIR/scripts/test-itu-region-bands.sh"
run_step "Non-resonant multi-optima" "$ROOT_DIR/scripts/test-multi-optima.sh"
run_step "NEC calibration checks"  "$ROOT_DIR/scripts/test-nec-calibration.sh"

if [[ $overall -eq $PASS ]]; then
  echo "All checks passed."
else
  echo "One or more checks failed." >&2
  exit 1
fi
