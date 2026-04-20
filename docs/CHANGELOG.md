# Changelog

All notable changes to Rusty Wire are documented here.

## [Unreleased]

### Added
- **Structured error handling (Priority 1)**: extended `AppError` with `EmptyBandSelection` and `AllBandsSkipped` variants; added empty-band check to `validate_config` and post-calculation check to `run_calculation_checked`. Removed three duplicated `calculations.is_empty()` guards from `cli.rs` — all error paths now flow through the app layer.
- **Proper CLI exit codes**: `run_from_args` now returns `bool`; `main.rs` propagates a non-zero exit code on any error, fixing the bug where invalid inputs silently exited 0.
- **Regression coverage**: added unit tests for `EmptyBandSelection` and `AllBandsSkipped`; added integration tests for invalid wire window and out-of-range velocity including exit code assertions.

### Changed
- **Documentation consolidation**: reduced redundancy across README, CLI guide, testing guide, and roadmap; tightened command references and moved deep details to their canonical docs.

### Added
- **Session exports documentation**: added `docs/steering.md` and `docs/memories.md` exports for session steering and memory state snapshots.

## [2.2.0] - 2026-04-14

### Added
- **Interactive session defaults**: interactive mode now remembers user choices during a session and reuses them as prompt defaults.

### Changed
- **UI-integration preparation**: refactored application layering by extracting display views into the app layer and centralizing shared band/transformer helpers.
- **Validation and CLI cleanup**: refactored app-side validation and streamlined CLI housekeeping to support cleaner front-end boundaries.
- **Regression updates**: refreshed supporting regression coverage/scripts to align with helper and app-layer refactors.
- **Documentation refresh**: updated roadmap and user/developer docs to reflect interactive session defaults and UI-prep architecture direction.

### Fixed
- **Output labeling**: fixed the recommended marker in both-units optima output.
- **Antenna naming**: corrected OCFD/Windom mislabeling in display output.

## [2.1.0] - 2026-04-12

### Added
- **Recommended transformer selection**: `--transformer recommended` is now supported and is the default CLI behavior.
- **Transformer recommendation coverage**: added unit and integration tests for recommended-ratio resolution and EFHW/non-resonant defaults.

### Changed
- **Mode/model-aware transformer defaults**: generic resonant runs now default to `1:1`, generic non-resonant runs to `1:9`, EFHW to `1:56`, and OCFD to `1:4` unless explicitly overridden.
- **Documentation refresh**: README, CLI guide, testing guide, architecture notes, and roadmap now reflect the 2.1.0 transformer recommendation behavior and current project state.

## [2.0.0] - 2026-04-12

### Added
- **SBOM generation via Cargo**: added Cargo aliases for SPDX and CycloneDX generation using `cargo-sbom`, with `cargo sbom` defaulting to SPDX and `cargo sbom-cdx` for CycloneDX JSON.
- **Pre-push SPDX SBOM step**: added a repository pre-push hook that regenerates `sbom/rusty-wire.spdx.json` and blocks push when the tracked SBOM is outdated.

### Changed
- **Band selection syntax refactor (breaking)**: `--bands` now accepts real band names and name ranges such as `10m,40m,10m-15m,60m-80m` in both CLI and interactive mode.
- **Equivalent command output updated**: interactive equivalent CLI suggestions now print named bands instead of numeric indices.

## [1.5.2] - 2026-04-11

### Added
- **Inverted-V antenna model mode**: added `--antenna inverted-v` with inverted-V total length, per-leg length, and estimated 90°/120° apex span output.
- **Inverted-V integration coverage**: added CLI integration tests validating inverted-V filtered output and export-field presence.

### Changed
- **Antenna model output expansion**: default all-model output now includes inverted-V geometry in addition to dipole, EFHW, loop, and OCFD fields.
- **Export payload expansion**: CSV/JSON/Markdown/TXT exports now include inverted-V fields in metric, imperial, and both-unit modes.
- **Inverted-V compromise detail**: resonant compromise guidance in inverted-V mode now prints each-leg and 90°/120° apex span estimates for each candidate total length.

## [1.5.1] - 2026-04-11

### Added
- **OCFD antenna model mode**: added `--antenna ocfd` with off-center-fed dipole leg split output (33/67 and 20/80 variants).
- **OCFD integration coverage**: added CLI integration tests validating OCFD-filtered output and tuner-assisted compromise guidance labeling.
- **OCFD split-ratio optimization**: compromise output now includes an optimized feedpoint split ratio recommendation per candidate length, with worst-leg resonance-clearance percentage.

### Changed
- **Antenna model output expansion**: default all-model output now includes OCFD leg split guidance in addition to dipole, EFHW, and loop fields.
- **Export payload expansion**: CSV/JSON/Markdown exports now include OCFD segment fields in metric, imperial, and both-unit modes.
- **OCFD compromise clarity**: OCFD tuner-assisted compromise lines now explicitly print 33/67 and 20/80 leg lengths under each candidate total length.

## [1.5.0] - 2026-04-10

### Added
- **Antenna model selection mode**: new `--antenna dipole|efhw|loop` option for filtering per-band output to a selected model.
- **Interactive antenna model parity**: interactive flow now prompts for antenna model selection, aligned with CLI behavior.
- **Derived antenna model dimensions**: per-band outputs and exports now include end-fed half-wave and full-wave loop dimensions.
- **Loop-mode integration coverage**: added CLI integration coverage validating loop-model output filtering and tuner-assisted compromise labeling.

### Changed
- **Resonant output behavior by model**: dipole resonant-point summary is shown for dipole/all, while compromise guidance is shown for all models.
- **Tuner-assisted guidance labeling**: in EFHW and loop modes, resonant compromise suggestions are explicitly labeled as dipole-derived tuner-assisted starting points.
- **Export payload expansion**: CSV/JSON/Markdown/TXT exports now carry first-batch derived antenna fields.

## [1.4.0] - 2026-04-10

### Changed
- **CLI rewritten with clap**: replaced manual argument parsing with a clap-based parser. All flags and validation behaviour are unchanged; the new parser provides built-in `--help` output and type-safe argument handling.
- **No-argument behavior changed**: running the binary without arguments now shows clap help instead of immediately starting an interactive or default calculation flow.
- **Interactive mode retained behind a flag**: the stdin-driven workflow is still available, but now requires `--interactive`.

### Removed
- **30 CLI-parser unit tests**: tests for the old hand-written parse functions (`parse_band_list`, `parse_itu_region`, `parse_export_format_list`, etc.) were removed alongside those functions. Equivalent input validation is now enforced by clap's type system, with current behavior covered by CLI integration tests.

### Added
- **CLI integration tests**: added binary-level tests for no-argument help output, CLI validation branches, region-aware band listing, and export-path selection behavior.

## [1.3.0] - 2026-04-09

### Added
- **Non-resonant search-window local optima**: non-resonant mode now lists multiple local optimum wire lengths (clearance maxima) within the active search window.
- **Comprehensive unit test suite**: added 45 tests covering calculations, band database, and app logic for improved code quality and regression prevention.

### Changed
- **Non-resonant output detail**: output now distinguishes between local window optima and equal global optima, marking the selected recommendation in the local-optima list.
- **Export security hardening**: export output paths are now validated to reject absolute paths and parent-directory traversal (`..`) before writing files.
- **Documentation updates**: README and CLI guide now describe the new non-resonant search-window local-optima output.

### Tests Added
- **Transformer ratio**: parsing, labels, impedance calculations (5 tests)
- **Band calculations**: velocity factor effects, unit conversions, distance averaging (8 tests)
- **Optimization algorithms**: non-resonant optima, resonant compromises, window-local candidates (7 tests)
- **Band database**: region-specific frequencies, ITU adjustments (13 tests)
- **Application logic**: configuration, multi-region support, calculation modes (8 tests)
- **Export**: path validation, format rejection (4 tests)

## [1.2.0] - 2026-04-09

### Added
- **Resonant points in output and exports**: resonant mode now prints all resonant points (quarter-wave harmonics) within the active search window, sorted by wire length.
- **Resonant shared compromise candidates**: resonant mode now shows closest combined compromise lengths to in-window resonant points across selected bands.

### Changed
- **Compromise candidate selection**: resonant compromise output now includes multiple near-best local candidates instead of only a single global winner.
- **Documentation updates**: CLI guide and README were updated to describe resonant-point and resonant-compromise behavior.

### Fixed
- **Resonant-mode ambiguity**: removed misleading non-resonant compromise block from resonant-mode terminal output.
- **Export mode gating**: non-resonant recommendation payloads are omitted in resonant-mode exports while remaining available for non-resonant-mode exports.
- **Markdown export column alignment**: fixed Ratio/Frequency column order mismatch in markdown table rows.

## [1.1.0] - 2026-04-08

### Added
- **ITU region selection** in both CLI and interactive mode.
	- New CLI flag: `--region 1|2|3` (default: Region 1).
	- Interactive mode now prompts for region and allows changing region from the menu.
- **Regional band listing**: `--list-bands` now respects the selected ITU region.
- **Regression test script**: `scripts/test-itu-region-bands.sh` validates all listed bands and ranges for Regions 1, 2, and 3.

### Changed
- **Band model is now region-aware** for amateur allocations that differ by region:
	- 80m: Region 1 `3.5-3.8`, Region 2 `3.5-4.0`, Region 3 `3.5-3.9`.
	- 40m: Region 1 `7.0-7.2`, Region 2 `7.0-7.3`, Region 3 `7.0-7.2`.
	- 60m uses a harmonized WRC-15 segment: `5.3515-5.3665`.
- Calculation frequencies are now derived from region-adjusted ranges so resonant results match the selected ITU region.

## [1.0.0] — 2026-04-03

### Added
- **Multi-format export**: `--export` now accepts a comma-separated list of formats: `csv`, `json`, `markdown`, `txt`. All selected formats are written in one run.
- **Unit system control**: new `--units m|ft|both` flag. Input in meters vs. feet is auto-detected from the flags used; `--units` overrides the display/export units independently.
- **Multiple optima**: when several wire lengths are equally optimal in non-resonant mode, all are listed in ascending order under "Additional equal optima in range".
- **Resonant mode compromise length**: resonant mode now shows the optimum common wire length (with search window) alongside the per-band resonant lengths.
- **Architecture refactor**: codebase split into `app.rs` (pure computation API), `export.rs` (formatting), `cli.rs` (CLI/interactive front-end), and `main.rs` (entry point) to enable a future iced-based GUI without touching core logic.
- **Bash validation script**: `scripts/test-multi-optima.sh` performs an exhaustive parameter sweep to verify multi-optima behaviour is reachable.
- **Version number**: binary now reports its version in the interactive banner and in `--help` output.

### Changed
- **Strict mode isolation**: resonant and non-resonant modes no longer share output blocks. Resonant mode shows only resonant wire lengths plus the compact compromise summary. Non-resonant mode shows the full optimisation block with search window and tied optima.
- **Wire constraint flags renamed**: `--non-res-min`/`--non-res-max` replaced by clearer `--wire-min`/`--wire-max` (and `--wire-min-ft`/`--wire-max-ft`).
- **Interactive resonant flow**: the wire length window prompt is skipped entirely when resonant mode is selected interactively.
- **Equivalent CLI reconstruction**: the printed command now always includes `--units`; `--wire-min`/`--wire-max` are omitted for resonant mode.

### Fixed
- Non-resonant recommendation block was incorrectly shown in resonant mode.
- Equivalent CLI call in resonant mode included non-resonant-only flags.

## [0.1.0] — initial development

- Core resonant dipole calculations (half-wave, full-wave, quarter-wave).
- Non-resonant random-wire optimisation.
- Skip-distance summary.
- Interactive menu and basic CLI argument parsing.
- CSV and JSON export.
- Band database: 10 ham HF bands + 11 shortwave broadcast bands.
