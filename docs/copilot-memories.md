---
project: rusty-wire
doc: docs/copilot-memories.md
status: living
last_updated: 2026-04-30
---

# Rusty Wire Copilot Memory Export

This file is the canonical snapshot of the current Copilot memory state for this repository.
It consolidates user memory, session memory, and repository memory in one place.
Keep it synchronized with the contents of `/memories` and `/memories/repo` before pushing changes.

Export date: 2026-04-20

## User memory

### /memories/cli-parity.md

- Keep CLI mode and interactive mode aligned for user-facing features and outputs when making changes to Rusty Wire.
- If parity is not practical for a change, explicitly tell the user before or when implementing it.
- Default pre-push sequence for Rusty Wire: `cargo fmt`, `cargo check`, `cargo test`, then push only if all pass.
- Release cadence preference: ship current stable milestone as a minor bump (e.g., 2.3.0), then increment patch versions (2.3.1, 2.3.2, ...) for subsequent smaller batches.
- Decision preference: when offered numbered execution options, default to option 1 (logical split commits and push) unless told otherwise.

## Session memory

No session memory entries are currently stored.

## Repository memory

### /memories/repo/roadmap.md

- Canonical future-work document: /home/dc0sk/git/rusty-wire/docs/roadmap.md
- Use docs/roadmap.md instead of session-only plan notes for remaining improvements.
- High-priority remaining items: app-layer API refinements for future GUI work, advanced custom-band inputs, additional antenna models, interactive-mode testability.
- Antenna-model additions need parity updates in src/app.rs, src/calculations.rs, src/cli.rs, src/export.rs, tests/cli_integration.rs, README.md, docs/cli-guide.md, and docs/CHANGELOG.md; note that TXT export has its own formatter and is easy to miss.

### /memories/repo/workflow-sync.md

- Before pushing, ensure documentation is up to date, docs/copilot-memories.md is synchronized with /memories and /memories/repo, cargo fmt and cargo test have run, the additional regression scripts have run, and all necessary files are staged.
- On version bumps: regenerate SBOM (`cargo sbom` for SPDX, `cargo sbom-cdx` for CycloneDX), commit the updated sbom/ files, then tag the release commit.

### /memories/repo/itu-regions-implementation.md

Key implementation notes:
- `bands.rs`: `ITURegion` enum (Region1/2/3), `regions` field on `Band`, `get_bands_for_region()`, `get_band_by_index_for_region()`
- `app.rs`: `DEFAULT_ITU_REGION = Region1`, `itu_region` field in `AppConfig`, region-aware `build_calculations()`
- `cli.rs`: `--region 1|2|3` CLI flag (default 1), `prompt_itu_region()` for interactive menu