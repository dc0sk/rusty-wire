#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo is required" >&2
  exit 1
fi

if ! cargo --list | grep -q '^    cyclonedx'; then
  echo "error: cargo-cyclonedx is not installed." >&2
  echo "install it with: cargo install cargo-cyclonedx" >&2
  exit 1
fi

cargo sbom

echo "SBOM generated via cargo-cyclonedx (JSON)."
echo "Default output location: target/cyclonedx/"
