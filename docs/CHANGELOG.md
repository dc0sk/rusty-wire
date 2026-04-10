# Changelog

All notable changes to Rusty Wire are documented here.

## [1.3.0] - 2026-04-09

### Added
- **Non-resonant search-window local optima**: non-resonant mode now lists multiple local optimum wire lengths (clearance maxima) within the active search window.

### Changed
- **Non-resonant output detail**: output now distinguishes between local window optima and equal global optima, marking the selected recommendation in the local-optima list.
- **Export security hardening**: export output paths are now validated to reject absolute paths and parent-directory traversal (`..`) before writing files.
- **Documentation updates**: README and CLI guide now describe the new non-resonant search-window local-optima output.

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
