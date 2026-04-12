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
    jsonq=""
    if command -v jq >/dev/null 2>&1; then
      jsonq="jq"
    elif command -v jaq >/dev/null 2>&1; then
      jsonq="jaq"
    fi

    if [[ -z "$jsonq" ]]; then
      echo "error: jq-compatible processor is required for deterministic SPDX normalization." >&2
      echo "install one of: jq (system package) or jaq (cargo install jaq)." >&2
      exit 1
    fi

    mkdir -p sbom
    out_file="${out_file:-sbom/rusty-wire.spdx.json}"
    tmp_raw="$(mktemp)"
    trap 'rm -f "$tmp_raw"' EXIT
    cargo sbom --output-format spdx_json_2_3 >"$tmp_raw"

    # Normalize volatile fields and ordering to keep the tracked SPDX stable.
    "$jsonq" '
      .creationInfo.created = "1970-01-01T00:00:00Z"
      | .documentNamespace = "https://spdx.org/spdxdocs/rusty-wire"
      | (.packages // []) |= sort_by(.name // "", .versionInfo // "", .SPDXID // "")
      | (.files // []) |= sort_by(.fileName // "", .SPDXID // "")
      | (.relationships // []) |= sort_by(.spdxElementId // "", .relationshipType // "", .relatedSpdxElement // "")
    ' "$tmp_raw" >"$out_file"

    rm -f "$tmp_raw"
    trap - EXIT
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

