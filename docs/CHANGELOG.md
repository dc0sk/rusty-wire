# Changelog

All notable changes to Rusty Wire are documented here.

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
