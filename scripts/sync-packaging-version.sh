#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 <version> [debian_revision]" >&2
  exit 1
fi

version="$1"
debian_revision="${2:-1}"

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pkgbuild="${repo_root}/packaging/arch/rusty-wire/PKGBUILD"
debian_changelog="${repo_root}/packaging/debian/changelog"

if [[ ! -f "${pkgbuild}" ]]; then
  echo "Missing file: ${pkgbuild}" >&2
  exit 1
fi

if [[ ! -f "${debian_changelog}" ]]; then
  echo "Missing file: ${debian_changelog}" >&2
  exit 1
fi

sed -i "s/^pkgver=.*/pkgver=${version}/" "${pkgbuild}"
sed -i "s/^pkgrel=.*/pkgrel=1/" "${pkgbuild}"

# Keep existing distribution/urgency suffix and only replace package version+revision.
sed -Ei "1 s/^rusty-wire \([^)]+\)/rusty-wire (${version}-${debian_revision})/" "${debian_changelog}"

echo "Synchronized packaging versions to ${version}-${debian_revision}."
