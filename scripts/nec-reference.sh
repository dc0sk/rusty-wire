#!/usr/bin/env bash
# Regenerate NEC-2 reference values for the rusty-wire validation corpus using
# `nec2c` (the double-precision C translation of NEC-2). Unlike the fnec-rust
# Hallén solver, nec2c supports finite ground and multi-wire (non-collinear)
# geometry, so it can validate the inverted-V and ground cases too.
#
# The committed reference values live in corpus/nec2c-reference.json; this script
# reproduces them. CI does NOT run nec2c — it tests rusty-wire against the
# committed JSON, exactly like the fnec reference workflow.
#
# Usage:
#   scripts/nec-reference.sh solve <deck.nec>        # print feedpoint "R X" (ohms)
#   scripts/nec-reference.sh dipole-resonance <f_MHz> <radius_m> [segs]
#                                                    # bisect the resonant length
set -euo pipefail

if ! command -v nec2c >/dev/null 2>&1; then
  echo "error: nec2c not found in PATH (install 'necpp'/'nec2c')." >&2
  exit 1
fi

# nec2c has a short output-path buffer, so work in a short temp dir.
work="$(mktemp -d /tmp/necref.XXXX)"
trap 'rm -rf "$work"' EXIT

# Solve a deck and echo "R X" (real and imaginary feedpoint impedance, ohms).
solve() {
  local deck="$1"
  cp "$deck" "$work/in.nec"
  nec2c -i "$work/in.nec" -o "$work/out.txt" >/dev/null 2>&1 || true
  grep -A3 'ANTENNA INPUT PARAMETERS' "$work/out.txt" | tail -1 \
    | awk '{printf "%.3f %.3f\n", $7, $8}'
}

# Bisection on total length to find X ≈ 0 (resonance) for a free-space dipole.
dipole_resonance() {
  local f="$1" radius="$2" segs="${3:-51}"
  local lo hi
  # Start bracket around the free-space half wavelength.
  lo="$(awk -v f="$f" 'BEGIN{printf "%.4f", 0.90*149.896229/f}')"
  hi="$(awk -v f="$f" 'BEGIN{printf "%.4f", 1.02*149.896229/f}')"
  local half feed
  feed="$(awk -v s="$segs" 'BEGIN{printf "%d", int(s/2)+1}')"
  for _ in $(seq 1 30); do
    local mid; mid="$(awk -v a="$lo" -v b="$hi" 'BEGIN{printf "%.5f", (a+b)/2}')"
    half="$(awk -v t="$mid" 'BEGIN{printf "%.5f", t/2}')"
    cat > "$work/in.nec" <<EOF
CM resonance probe
CE
GW 1 $segs 0 0 -$half 0 0 $half $radius
GE 0
FR 0 1 0 0 $f 0
EX 0 1 $feed 0 1.0 0.0
RP 0 1 1 1000 0 0 0 0
EN
EOF
    nec2c -i "$work/in.nec" -o "$work/out.txt" >/dev/null 2>&1 || true
    local x; x="$(grep -A3 'ANTENNA INPUT PARAMETERS' "$work/out.txt" | tail -1 | awk '{printf "%.4f", $8}')"
    # X<0 => too short (capacitive) => raise low bound; X>0 => shorten.
    if awk -v x="$x" 'BEGIN{exit !(x<0)}'; then lo="$mid"; else hi="$mid"; fi
  done
  awk -v a="$lo" -v b="$hi" 'BEGIN{printf "%.3f\n", (a+b)/2}'
}

cmd="${1:-}"
case "$cmd" in
  solve)            solve "$2" ;;
  dipole-resonance) dipole_resonance "$2" "$3" "${4:-51}" ;;
  *) echo "usage: $0 {solve <deck>|dipole-resonance <f_MHz> <radius_m> [segs]}" >&2; exit 2 ;;
esac
