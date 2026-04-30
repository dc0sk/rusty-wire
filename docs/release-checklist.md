---
project: rusty-wire
doc: docs/release-checklist.md
status: living
last_updated: 2026-04-30
---

# Release Checklist

Use this checklist for each version bump and release tag.

## 1. Prepare Content

- Review pending entries in `docs/CHANGELOG.md` under `[Unreleased]`.
- Update `README.md` user-facing sections for any new features/flags.
- Update `docs/cli-guide.md` option references and examples if CLI behavior changed.

## 2. Refresh TUI Screenshots (Manual)

- Capture fresh **real terminal** screenshots (not generated/fake snapshots).
- Replace only the canonical files in `docs/images/tui/`:
  - `01-default-layout.png`
  - `02-trap-dipole-results.png`
  - `03-non-resonant-window.png`
  - `04-about-popup.png`
  - `05-results-scroll.png`
- Follow framing/content expectations in `docs/tui-screenshots.md`.

## 3. Update Version and SBOM

- Bump version in `Cargo.toml`.
- Sync packaging versions using `scripts/sync-packaging-version.sh <version>`.
- Confirm both files were updated:
  - `packaging/arch/rusty-wire/PKGBUILD`: `pkgver=` set to the new version and `pkgrel=1`
  - `packaging/debian/changelog`: first entry version set to `<version>-1`
- After downloading the source tarball, run `makepkg -g` to regenerate Arch `sha256sums`.
- Regenerate SBOM artifacts:

```bash
cargo sbom
cargo sbom-cdx
```

- Stage updated `sbom/` files with version/doc changes.

## 4. Validate Before Tag

Run the default verification sequence:

```bash
scripts/check-packaging-version-sync.sh
cargo fmt
cargo check
cargo test
```

## 5. Tag and Release

- Merge release PR to `main`.
- Create/push annotated tag (for example `v2.6.1`) from the release commit.
- Publish GitHub release notes from `docs/CHANGELOG.md` highlights.
