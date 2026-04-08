#!/usr/bin/env bash
set -euo pipefail

# Regression test for ITU region-specific band ranges.
#
# Verifies all listed bands for Regions 1, 2, and 3.
#
# Usage:
#   scripts/test-itu-region-bands.sh
#
# Optional env overrides:
#   BIN=./target/debug/rusty-wire

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${BIN:-$ROOT_DIR/target/debug/rusty-wire}"

cd "$ROOT_DIR"
cargo build --quiet

expected_lines_for_region() {
  local region="$1"
  case "$region" in
    1)
      cat <<'EOF'
160m [HF] (1.8-2 MHz)
80m [HF] (3.5-3.8 MHz)
60m [HF] (5.3515-5.3665 MHz)
40m [HF] (7-7.2 MHz)
30m [HF] (10.1-10.15 MHz)
20m [HF] (14-14.35 MHz)
17m [HF] (18.068-18.168 MHz)
15m [HF] (21-21.45 MHz)
12m [HF] (24.89-24.99 MHz)
10m [HF] (28-29.7 MHz)
120m SW (2.3-2.495 MHz) [MF] (2.3-2.495 MHz)
90m SW (3.2-3.4 MHz) [MF] (3.2-3.4 MHz)
75m SW (3.9-4.0 MHz) [HF] (3.9-4 MHz)
49m SW (5.9-6.2 MHz) [HF] (5.9-6.2 MHz)
41m SW (7.2-7.45 MHz) [HF] (7.2-7.45 MHz)
31m SW (9.4-9.9 MHz) [HF] (9.4-9.9 MHz)
25m SW (11.6-12.1 MHz) [HF] (11.6-12.1 MHz)
22m SW (13.57-13.87 MHz) [HF] (13.57-13.87 MHz)
19m SW (15.1-15.8 MHz) [HF] (15.1-15.8 MHz)
16m SW (17.48-17.9 MHz) [HF] (17.48-17.9 MHz)
13m SW (21.45-21.85 MHz) [HF] (21.45-21.85 MHz)
EOF
      ;;
    2)
      cat <<'EOF'
160m [HF] (1.8-2 MHz)
80m [HF] (3.5-4 MHz)
60m [HF] (5.3515-5.3665 MHz)
40m [HF] (7-7.3 MHz)
30m [HF] (10.1-10.15 MHz)
20m [HF] (14-14.35 MHz)
17m [HF] (18.068-18.168 MHz)
15m [HF] (21-21.45 MHz)
12m [HF] (24.89-24.99 MHz)
10m [HF] (28-29.7 MHz)
120m SW (2.3-2.495 MHz) [MF] (2.3-2.495 MHz)
90m SW (3.2-3.4 MHz) [MF] (3.2-3.4 MHz)
75m SW (3.9-4.0 MHz) [HF] (3.9-4 MHz)
49m SW (5.9-6.2 MHz) [HF] (5.9-6.2 MHz)
41m SW (7.2-7.45 MHz) [HF] (7.2-7.45 MHz)
31m SW (9.4-9.9 MHz) [HF] (9.4-9.9 MHz)
25m SW (11.6-12.1 MHz) [HF] (11.6-12.1 MHz)
22m SW (13.57-13.87 MHz) [HF] (13.57-13.87 MHz)
19m SW (15.1-15.8 MHz) [HF] (15.1-15.8 MHz)
16m SW (17.48-17.9 MHz) [HF] (17.48-17.9 MHz)
13m SW (21.45-21.85 MHz) [HF] (21.45-21.85 MHz)
EOF
      ;;
    3)
      cat <<'EOF'
160m [HF] (1.8-2 MHz)
80m [HF] (3.5-3.9 MHz)
60m [HF] (5.3515-5.3665 MHz)
40m [HF] (7-7.2 MHz)
30m [HF] (10.1-10.15 MHz)
20m [HF] (14-14.35 MHz)
17m [HF] (18.068-18.168 MHz)
15m [HF] (21-21.45 MHz)
12m [HF] (24.89-24.99 MHz)
10m [HF] (28-29.7 MHz)
120m SW (2.3-2.495 MHz) [MF] (2.3-2.495 MHz)
90m SW (3.2-3.4 MHz) [MF] (3.2-3.4 MHz)
75m SW (3.9-4.0 MHz) [HF] (3.9-4 MHz)
49m SW (5.9-6.2 MHz) [HF] (5.9-6.2 MHz)
41m SW (7.2-7.45 MHz) [HF] (7.2-7.45 MHz)
31m SW (9.4-9.9 MHz) [HF] (9.4-9.9 MHz)
25m SW (11.6-12.1 MHz) [HF] (11.6-12.1 MHz)
22m SW (13.57-13.87 MHz) [HF] (13.57-13.87 MHz)
19m SW (15.1-15.8 MHz) [HF] (15.1-15.8 MHz)
16m SW (17.48-17.9 MHz) [HF] (17.48-17.9 MHz)
13m SW (21.45-21.85 MHz) [HF] (21.45-21.85 MHz)
EOF
      ;;
    *)
      echo "unknown region: $region" >&2
      return 1
      ;;
  esac
}

extract_list_lines() {
  local region="$1"
  "$BIN" --list-bands --region "$region" 2>/dev/null \
    | awk '/^[[:space:]]*[0-9]+\./ {sub(/^[[:space:]]*[0-9]+\. /, ""); print; if (++n == 21) exit}'
}

check_region() {
  local region="$1"
  mapfile -t expected < <(expected_lines_for_region "$region")
  mapfile -t actual < <(extract_list_lines "$region")

  if [[ "${#actual[@]}" -ne "${#expected[@]}" ]]; then
    echo "FAIL: Region $region returned ${#actual[@]} bands, expected ${#expected[@]}" >&2
    return 1
  fi

  local i
  for i in "${!expected[@]}"; do
    if [[ "${actual[$i]}" != "${expected[$i]}" ]]; then
      echo "FAIL: Region $region mismatch at entry $((i + 1))" >&2
      echo "  expected: ${expected[$i]}" >&2
      echo "  actual:   ${actual[$i]}" >&2
      return 1
    fi
  done

  echo "PASS: Region $region band list matches expected ranges (${#expected[@]} bands)."
}

check_region 1
check_region 2
check_region 3

echo "PASS: ITU region regression checks passed."
