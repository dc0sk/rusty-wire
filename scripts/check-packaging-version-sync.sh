#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo_toml="${repo_root}/Cargo.toml"
pkgbuild="${repo_root}/packaging/arch/rusty-wire/PKGBUILD"
debian_changelog="${repo_root}/packaging/debian/changelog"

cargo_version="$(grep -E '^version\s*=\s*"' "${cargo_toml}" | head -n1 | sed -E 's/^version\s*=\s*"([^"]+)"/\1/')"
pkgbuild_version="$(grep -E '^pkgver=' "${pkgbuild}" | head -n1 | cut -d= -f2)"
debian_version="$(sed -nE '1 s/^rusty-wire \(([0-9]+\.[0-9]+\.[0-9]+)-[0-9]+\).*/\1/p' "${debian_changelog}")"

if [[ -z "${cargo_version}" || -z "${pkgbuild_version}" || -z "${debian_version}" ]]; then
  echo "Failed to parse one or more version fields." >&2
  echo "cargo_version='${cargo_version}' pkgbuild_version='${pkgbuild_version}' debian_version='${debian_version}'" >&2
  exit 1
fi

if [[ "${cargo_version}" != "${pkgbuild_version}" || "${cargo_version}" != "${debian_version}" ]]; then
  echo "Version mismatch detected:" >&2
  echo "  Cargo.toml:                 ${cargo_version}" >&2
  echo "  packaging/arch/PKGBUILD:    ${pkgbuild_version}" >&2
  echo "  packaging/debian/changelog: ${debian_version}" >&2
  echo "Run: scripts/sync-packaging-version.sh ${cargo_version}" >&2
  exit 1
fi

echo "Packaging versions are synchronized at ${cargo_version}."
