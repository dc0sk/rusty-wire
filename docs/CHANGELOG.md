# Changelog

All notable changes to Rusty Wire are documented here.

## [2.6.0] — 2026-04-25

### Test Coverage
- **PR #49**: Antenna-model metadata coverage (5 lib tests)
  - Transformer ratio explanations: dipole, inverted-V, trap dipole, full-wave loop, OCFD
  - Impedance expectations for each model
- **PR #48**: TUI global key-event guards (2 lib tests)
  - Non-Press event rejection (Release events ignored)
  - Esc-without-popup quit behavior

## [Unreleased]
- **NEC calibration hardening**: `scripts/calibrate-conductor-model.sh` now tolerates blank lines and `#` comments in CSV inputs while preserving strict malformed-row validation.
- **NEC calibration regression script**: added `scripts/test-nec-calibration.sh` to lock template fit constants (`k = 0.011542`, `RMSE = 0.000000`) and validate parser behavior.
- **Docs/version sync**: updated README and CLI guide version labels to 2.6.0.
- **TUI warning cleanup**: removed non-test unused import warning by scoping `KeyEventState` usage to tests.
- **TUI export**: press `e` (CSV), `E` (JSON), `m` (Markdown), or `t` (plain text) to export results directly from the TUI; a status message is shown in the hints bar after each export attempt.
- **TUI step-size config field**: the `Step` field in the TUI config panel lets you cycle through search-step presets (0.01, 0.02, 0.05, 0.10, 0.25, 0.50, 1.00 m) with ←/→, matching the CLI `--step` flag.
- **TUI explicit frequency selection**: the `Frequencies` field in the TUI config panel lets you cycle through frequency presets (single frequencies or multi-frequency sets like "3.5, 7.0, 14.0 MHz") with ←/→, matching the CLI `--freq` and `--freq-list` flags.
- **TUI named band presets**: when `bands.toml` is present in the current working directory, the `Bands` field loads its named presets alongside the built-in band sets and the `Custom…` checklist option; `rusty-wire-tui --bands-config <path>` can override the preset file at startup.
- **`--freq-list <f1,f2,...>` flag**: compute wire lengths for multiple explicit frequencies in a single invocation, bypassing band selection entirely. Each frequency produces its own labelled result row.
- **Custom band presets via TOML**: added `--bands-preset <name>` to resolve named band sets from a config file with optional `--bands-config <path>` override (default: `bands.toml`). Presets reuse the existing band-token parser, so entries can include both names and ranges.
- **Conductor calibration regression coverage**: added tests and documentation that lock the current template NEC fit (`k = 0.011542`) to the placeholder reference CSV while explicitly noting that runtime clamps remain broader than the observed template span until real NEC sweep data is committed.
- **Balun optimizer app-layer foundation**: added `optimize_transformer_candidates(&AppConfig)` with ranked transformer candidates and per-candidate metadata (target impedance, mismatch factor, estimated efficiency, mismatch loss, and correction-shift penalty score). This is the prerequisite API for upcoming `advise` mode.
- **`--advise` CLI mode**: added ranked advise output that combines recommended wire length with balun/unun ratio candidates for the selected bands, including efficiency estimate, mismatch loss, clearance, and overall score.
- **Advise exports**: `--advise` now supports `--export csv,json,markdown,txt` with dedicated advise report outputs; Markdown export is available via `--export markdown`.
- **Math reference documentation**: added `docs/math.md` with KaTeX equations for core wire-length formulas, mismatch model, and optimizer objective functions (non-resonant, resonant compromise, OCFD split, and advise ranking).
- **Standard antenna height model**: added `--height 7|10|12` to CLI and interactive flows, with app-layer validation and propagation into calculations.
- **Ground-class model**: added `--ground poor|average|good` to CLI and interactive flows, propagating into skip-distance modeling.
- **Conductor-diameter model**: added `--conductor-mm <value>` (range `1.0..=4.0`, metric-only) to CLI and interactive flows, propagating into first-order impedance/length correction.
- **NEC calibration workflow scaffolding**: added `scripts/calibrate-conductor-model.sh`, `docs/data/nec_conductor_reference.csv`, and `docs/nec-calibration.md` to standardize fitting conductor-diameter correction constants from reference sweeps.
- **Optional advise validation flag**: added `--validate-with-fnec` for `--advise` runs to attempt per-candidate cross-tool checks via `fnec-rust` when `fnec` is available in `PATH`.
- **Configurable advise validation thresholds**: added `--fnec-pass-max-mismatch` and `--fnec-reject-min-mismatch` to classify fnec validation outcomes as passed, warning, or rejected.
- **Advise export validation metadata**: advise exports now include `validated`, `validation_status`, and `validation_note` fields in CSV/JSON/Markdown/TXT outputs for each candidate.
- **Trap dipole resonant guidance notes**: resonant compromise output now includes trap build notes covering total-vs-element interpretation, trap frequency/component tuning targets, physical trap placement, and common 40m/20m and 80m/40m pairings.
- **Interactive prompt coverage expansion**: added direct unit tests for remaining prompt helpers, including invalid wire-window fallback handling, display-unit defaults and fallback behavior, and ITU region selection validation.
- **TUI regression coverage**: added tests for documented hints-bar keybinding text, About popup metadata content, info-popup toggle/close behavior, and preservation of the trailing `Custom…` band-preset entry.
- **TUI checklist/export-status coverage**: added tests for custom-band checklist seeding, confirm/cancel behavior, no-results export warnings, and clearing the status banner on the next keypress.
- **TUI preset-transition coverage**: added tests for named-band preset cycling, custom-band fallback reuse, frequency preset forward/backward wrap behavior, and the `Use bands` reset path.
- **TUI focus/scroll coverage**: added tests for `Tab` focus switching, config-field navigation wrap behavior, and results-panel line/page scrolling with saturating bounds.
- **TUI run/enter behavior coverage**: added tests for Enter opening the custom-band checklist only in the config/bands/custom path, Enter-triggered recalculation elsewhere, and `r` recalculation resetting results scroll.
- **TUI checklist key-alias coverage**: added tests for checklist `j/k` cursor movement bounds, space-bar toggle behavior, `q` cancel handling, and overlay `Ctrl-C` quit behavior.

### Changed
- **Roadmap sequencing**: captured `advise` feature direction (candidate ranking for wire length + balun/unun choice with efficiency-style metadata) and marked balun optimizer groundwork as the prerequisite milestone.
- **Model realism adjustments**: inverted-V geometry now uses empirical apex-angle shortening factors (90°: 0.97, 120°: 0.985), and full-wave loop circumference now uses `1005/f` guidance.
- **Practical-limits first mitigation**: skip-distance summaries now include first-order height-aware scaling for 7 m, 10 m, and 12 m antenna heights.
- **Practical-limits extension**: skip-distance summaries now also include first-order ground-class scaling (poor/average/good).
- **Practical-limits extension**: resonant-length estimates now include first-order conductor-diameter correction around a 2.0 mm baseline.
- **Conductor calibration refinement**: updated conductor-diameter logarithmic coefficient to `0.011542` from the current reference sweep dataset.

## [2.5.2] - 2026-04-21

### Fixed
- **Trap dipole implementation restored**: `--antenna trap-dipole` (aliases `trap`, `trapdipole`) was documented in 2.5.1 but absent from the binary. Restored full implementation across calculations, app display views, CLI interactive selection, TUI model cycling, and all export formats (CSV/JSON/Markdown/TXT). Added 4 integration tests: `trap_dipole_antenna_mode_shows_trap_length`, `trap_dipole_aliases_accepted`, `trap_dipole_export_includes_trap_fields`, `trap_dipole_recommended_transformer_is_1_1`.

### Changed
- **Backlog**: added trap dipole guidance item (trap type selection, component specs, installation positions).

## [2.5.1] - 2026-04-21

### Added
- **Trap dipole antenna model**: added `--antenna trap-dipole` (aliases: `trap`, `trapdipole`) with dedicated output in calculations, app display views, CLI interactive selection, TUI model cycling, and all export formats (CSV/JSON/Markdown/TXT).
- **Project info surfaces**: restored TUI About popup metadata and added CLI/interactive parity output for version, author, GitHub URL, and license (`--info` plus interactive menu item).
- **TUI screenshot checklist + assets**: added canonical screenshot plan and image placeholders/paths for docs updates.

### Changed
- **Documentation refresh**: README, CLI guide, backlog, and screenshot documentation updated for trap dipole support, info surfaces, and screenshot placement guidance.
- **Version bump**: release version advanced from `2.5.0` to `2.5.1`.

## [2.5.0] - 2026-04-21

### Added
- **`--freq-list <f1,f2,...>` flag**: compute wire lengths for multiple explicit frequencies in a single invocation, bypassing band selection entirely. Each frequency produces its own labelled `WireCalculation` row (`X.XXX MHz`). Accepts any number of positive values up to 1000 MHz; combines freely with `--mode`, `--antenna`, `--quiet`, `--units`, and `--export`.
- **`AppConfig.freq_list_mhz: Vec<f64>`**: new app-layer field (default empty); `run_calculation` processes `freq_list_mhz` first, before `custom_freq_mhz` and band selection.
- **`AppAction::SetFreqList(Vec<f64>)`**: state-machine action for TUI and future GUI.
- **Mutual exclusion guard**: `--freq` and `--freq-list` cannot be used together; a clear error is printed if both are provided.
- **6 new integration tests**: `freq_list_computes_multiple_frequencies`, `freq_list_single_entry_behaves_like_freq`, `freq_list_and_freq_are_mutually_exclusive`, `freq_list_rejects_zero_frequency`, `freq_list_rejects_over_limit_frequency`, `freq_list_quiet_non_resonant_prints_compact`.

## [2.4.0] - 2026-04-21

### Added
- **Keyboard-driven TUI (`rusty-wire-tui`)**: a full ratatui 0.29 + crossterm 0.28 terminal UI with a two-panel layout (configuration left, results right). All nine config fields are editable with ←/→ from curated preset tables; results display the full `ResultsDisplayDocument` output with colour-coded headings, band titles, and warnings. Scrollable with ↑↓/jk/PgUp/PgDn.
- **`AppState` / `AppAction` / `apply_action`**: a pure, I/O-free state machine that bridges TUI (and future GUI) inputs to app-layer calculations. `apply_action(AppState, AppAction) → AppState` is the single update function; no side effects.
- **`AppState` / `AppAction` state-machine tests**: 13 dedicated unit tests in `src/app.rs` covering every `AppAction` variant and round-trip state transitions.
- **TUI band presets**: five named presets (40m–10m, 80m–10m, 160m–10m, 20m–10m, Contest 80/40/20/15/10) usable across all three ITU regions.
- **TUI velocity-factor and transformer presets**: ten VF presets (0.50–1.00) and all ten supported transformer ratios, cycled with ←/→.
- **Panic-safe terminal restore**: a `panic` hook guarantees crossterm raw mode and alternate screen are always restored before printing a panic message.
- **`BandListingView`**, **`TransformerRatioExplanation`**, **`SkippedBandDetail`**: new app-layer view types that expose band listing, transformer recommendation reasoning, and per-band skip reasons to any front-end without I/O.
- **`velocity_sweep_view` / `velocity_sweep_display_lines`**: velocity-sweep rendering extracted to the app layer; `cli.rs` delegates to these instead of formatting inline.
- **`format_quiet_summary`**: quiet-mode one-liner formatting extracted to the app layer.
- **`AppError::InvalidVelocitySweep(f64)`**: typed error for out-of-range velocity-factor values inside a sweep.

### Changed
- `cli.rs`: `show_all_bands_for_region_to_writer`, `run_velocity_sweep`, and `print_quiet_summary` all delegate to their new app-layer counterparts instead of formatting inline.
- `ratatui = "0.29"` and `crossterm = "0.28"` added to `[dependencies]`.

## [2.3.0] - 2026-04-20

### Added
- **`--quiet` flag**: suppresses the full results table. In non-resonant mode prints only the recommended wire length on a single line (respects `--units`); in resonant mode exits silently with code 0. Useful for scripting and automation.
- **`--freq <MHz>` flag**: computes wire lengths for a single explicit frequency instead of scanning named bands. Accepts any positive value up to 1000 MHz; bypasses band selection entirely. Combines with `--mode`, `--antenna`, `--quiet`, and all other output flags. `AppConfig` gained a `custom_freq_mhz: Option<f64>` field; `AppError::InvalidFrequency` is returned for out-of-range values.
- **`--velocity-sweep <v1,v2,...>` flag**: runs the same configuration at multiple velocity factors and prints a compact comparison table. Non-resonant mode shows recommended length and resonance clearance per VF; resonant mode shows per-band half-wave lengths per VF. Validates all VF values before executing any runs.
- **8 new integration tests** covering all three flags: quiet resonant/non-resonant, `--freq` basic usage and error handling, `--freq` combined with `--quiet`, and velocity sweep resonant/non-resonant/error paths.
- **Library entry point** (`src/lib.rs`): app, bands, and calculations modules are now exposed as a proper library crate; a thin `run_cli(args)` function bridges the binary entry-point to the CLI module. External front-ends (e.g. a future GUI) can depend on `rusty_wire::app::*` without pulling in CLI logic.
- **Shadow CLI types complete**: added `CliAntennaModel` and applied `Copy` to all CLI shadow enums; all five domain-facing fields in the `Cli` struct (`region`, `mode`, `antenna`, `units`, `export`) now use dedicated `Cli*` shadow types instead of the domain types directly.
- **App-layer contract tests**: six tests guard the stable `AppRequest → AppResponse` API boundary and assert that `ResultsDisplayDocument` is fully populated for both resonant and non-resonant defaults, and that all antenna models and calc modes execute without error.
- **Structured error handling (Priority 1)**: extended `AppError` with `EmptyBandSelection` and `AllBandsSkipped` variants; added empty-band check to `validate_config` and post-calculation check to `run_calculation_checked`. Removed three duplicated `calculations.is_empty()` guards from `cli.rs`.
- **Proper CLI exit codes**: `run_from_args` now returns `bool`; `main.rs` propagates a non-zero exit code on any error.
- **Regression coverage**: new unit and integration tests for all new error paths including exit code assertions.
- **`--step` flag for configurable non-resonant search resolution**: `AppConfig` now carries `step_m: f64` (default 0.05 m); the `--step <METERS>` CLI flag overrides it. `AppError::InvalidSearchStep` is returned when the step is ≤ 0 or ≥ the wire length window.

### Changed
- **clap decoupled from domain types**: `CalcMode`, `ExportFormat`, `UnitSystem`, `AntennaModel`, and `ITURegion` no longer implement `clap::ValueEnum`; the CLI wiring lives entirely in `cli.rs`.
- **Documentation consolidation**: reduced redundancy across README, CLI guide, testing guide, and roadmap; tightened command references and moved deep details to their canonical docs. Consolidated `memories.md` and `copilot-memories.md` into a single up-to-date `copilot-memories.md`.

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
- **SBOM generation via Cargo**: added Cargo aliases for SPDX and CycloneDX generation using `cargo-sbom`, with `cargo sbom` defaulting to SPDX and `cargo sbom-cdx` for CycloneDX JSON. SBOM regeneration is run on version bumps, not on every push.

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
