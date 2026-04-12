#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required" >&2
  exit 1
fi

format="${1:-spdx}"
out_file="${2:-}"

if ! cargo --list | grep -q '^    sbom'; then
  echo "error: cargo-sbom is not installed." >&2
  echo "install it with: cargo install cargo-sbom" >&2
  exit 1
fi

case "$format" in
  spdx)
    mkdir -p sbom
    out_file="${out_file:-sbom/rusty-wire.spdx.json}"
    cargo sbom --output-format spdx_json_2_3 >"$out_file"
    echo "SBOM generated via cargo-sbom (SPDX JSON 2.3): $out_file"
    ;;
  cyclonedx|cdx)
    mkdir -p sbom
    out_file="${out_file:-sbom/rusty-wire.cdx.json}"
    cargo sbom --output-format cyclone_dx_json_1_6 >"$out_file"
    echo "SBOM generated via cargo-sbom (CycloneDX JSON 1.6): $out_file"
    ;;
  *)
    echo "usage: $0 [spdx|cyclonedx]" >&2
    exit 1
    ;;
esac

