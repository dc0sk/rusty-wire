#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required" >&2
  exit 1
fi

format="${1:-spdx}"
out_file="${2:-}"

# Pin the cargo-sbom version so the committed SBOM is reproducible. Different
# cargo-sbom releases enumerate different per-target dependency sets, which would
# otherwise make the SBOM churn between contributors' machines.
PINNED_SBOM_VERSION="0.10.0"

if ! cargo --list | grep -q '^    sbom'; then
  echo "error: cargo-sbom is not installed." >&2
  echo "install the pinned version with: cargo install cargo-sbom --version $PINNED_SBOM_VERSION --locked" >&2
  exit 1
fi

installed_sbom_version="$(cargo sbom --version 2>/dev/null | awk 'NR==1{print $NF}')"
if [[ -n "$installed_sbom_version" && "$installed_sbom_version" != "$PINNED_SBOM_VERSION" ]]; then
  echo "warning: cargo-sbom $installed_sbom_version differs from the pinned $PINNED_SBOM_VERSION;" >&2
  echo "         the generated SBOM may differ from the committed one. To match, run:" >&2
  echo "         cargo install cargo-sbom --version $PINNED_SBOM_VERSION --locked" >&2
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

