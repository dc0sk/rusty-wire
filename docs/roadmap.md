# Roadmap

This document captures the most relevant work that remains after the 2.2.0 release.

## Recently Completed

- clap-based CLI parsing with explicit `--interactive`
- region-aware band selection and named band/range inputs
- antenna model expansion: dipole, inverted-v, EFHW, loop, OCFD
- recommended transformer defaults by mode/model
- export path hardening
- SBOM generation and pre-push enforcement
- broad unit/integration/script coverage

## Current Priorities

1. Error handling cleanup
2. App-layer API refinements for future GUI work
3. Search/analysis controls for power users
4. Advanced frequency and custom-band inputs
5. Additional antenna models and recommendation logic
- clap-based CLI parsing and no-argument help behavior
- interactive mode restoration behind `--interactive`
- interactive-mode I/O refactor and automated prompt/menu coverage
- region-aware band selection and named band/range input
- antenna model expansion through EFHW, loop, inverted-V, and OCFD
- recommended transformer selection with mode/model-aware defaults
- export path validation hardening
- SBOM generation and pre-push SBOM enforcement
- unit, integration, and regression-script coverage for current CLI behavior
- testing, architecture, and CLI documentation refresh
- shared app-layer wire-window normalization for CLI and future UI input paths
- shared app-layer band-selection parsing and label resolution for CLI and future UI input paths
- shared app-layer transformer recommendation fallback messaging for CLI and future UI input paths

## 1) Error Handling Cleanup

- Return structured errors from app-layer calculation paths.
- Centralize final user-facing formatting in `src/cli.rs`.
- Reduce duplicated validation between CLI and interactive mode.

## 2) GUI Readiness (`iced` Prerequisites)

- Stabilize an app-layer request/response boundary around `AppConfig` and `AppResults`.
- Keep shared summaries and warnings reusable outside terminal output.
- Add app-layer contract tests to protect future UI work.
- Decide packaging direction: single binary with multiple entry paths, or library + separate binaries.
### Implementation Plan

1. **Inventory All Error Handling**
	- Review all uses of `Result`, `Err`, `unwrap`, `expect`, and `panic!` in core logic, CLI, and calculations modules.

2. **Define Structured Error Types**
	- Create or extend an `AppError` enum in `src/app.rs` for all error cases (validation, calculation, export, etc).
	- Ensure all app-layer functions return `Result<T, AppError>`.

3. **Refactor Core Logic**
	- Update `app.rs` and `calculations.rs` to return structured errors instead of strings or panics.
	- Remove direct user messaging from these layers—errors should be data, not formatted text.

4. **Centralize Error Formatting**
	- In `src/cli.rs`, add a function to convert `AppError` to user-facing messages.
	- All CLI and interactive error reporting should use this function.

5. **Remove Duplicated Validation**
	- Move all validation logic (input checks, config validation, etc.) to reusable helpers in the app layer.
	- Ensure both CLI and future UIs call these helpers.

6. **Test and Document**
	- Add or expand tests for error propagation and formatting.
	- Document the error-handling flow for future UI integration.

This will make the code easier to reuse from future front ends and easier to test at the app layer.

## 3) Search and Analysis Controls

- Add configurable non-resonant resolution (`--precision` or `--step`).
- Add optional batch runs over multiple velocity factors or transformer ratios.
- Add a compact automation-oriented summary/report mode.

## 4) Advanced Input Support

- move remaining user-facing error decisions fully out of terminal-oriented code paths and expose structured app-layer errors that a GUI can render cleanly
- define a stable request/response boundary around `AppConfig` and `AppResults` so both CLI and UI use the same application service interface
- separate pure result formatting from terminal printing so a GUI can reuse summaries, labels, warnings, and recommendation text without scraping CLI output
- extract reusable validation helpers for wire-window constraints, band selection, transformer recommendation messaging, and export configuration
- continue extracting reusable validation helpers for export configuration
- introduce explicit view-friendly metadata where useful, such as recommended transformer explanations, skipped-band reasons, and per-band annotations
- review long-running or repeated operations such as export generation and future sweeps/batch runs so they can be surfaced as asynchronous UI tasks instead of blocking the event loop
- decide whether the project should remain a single binary with multiple entry paths or be split into a reusable library crate plus separate CLI/UI binaries
- add a small set of integration-style tests around the app-layer request/response contract so future UI work is protected from CLI refactors

## 5) Antenna and Recommendation Expansion

- Add more antenna models beyond current set.
- Improve recommendation transparency in help/output.
- Evaluate optional transformer ranking/optimization passes while keeping explicit overrides.

## Secondary Backlog

- New export targets (for example YAML/HTML)
- Logging and automation flags (`--quiet`, `--verbose`, `--dry-run`)
- UI-first enhancements after baseline parity (comparison views, saved sessions, richer visualizations)
The first UI should cover all major CLI/interactive capabilities, but it can also go beyond them in ways that are awkward in a terminal.

#### Baseline Feature Parity with CLI/Interactive

- region selection with immediate refresh of available bands
- named band and range selection with a list-based multi-select alternative
- calculation mode switching between resonant and non-resonant
- antenna model selection including all current models
- transformer selection with visible `recommended` rationale
- wire-window controls with unit-aware entry and inline validation
- unit-system output controls
- full results display for per-band lengths, skip distances, resonant points, non-resonant recommendations, and resonant compromises
- export actions for all supported output formats

#### UI-First Enhancements Beyond the CLI

- side-by-side comparison of multiple configurations, such as different velocity factors, transformer ratios, or antenna models
- live recalculation as inputs change, optionally with debounce for heavier operations
- richer band pickers with grouped amateur/shortwave sections, search, and preset chips
- collapsible result panels for dipole, EFHW, loop, inverted-V, and OCFD views
- visual highlighting of recommended transformer ratio, skipped bands, and resonance-clearance warnings
- result bookmarking or saved sessions for returning to previous scenarios
- graphical presentation of resonant points and non-resonant optima across the search window
- export preview before writing files
- guided workflows or setup wizards for common use cases such as EFHW planning, random-wire exploration, or OCFD split review
- a future custom-band/preset editor once user-defined band support exists

#### Nice-to-Have UI Features for Later

- multi-window or detachable analysis panes
- printable/shareable report preview
- theme support and accessibility-focused layout options
- persistent user preferences for units, region, default mode, and export behavior
- background task history for exports and heavier analysis runs


## TUI Integration with `ratatui` (and GUI Coexistence)

To provide a modern terminal user interface (TUI) using [`ratatui`](https://github.com/ratatui-org/ratatui) while preserving the ability to add an `iced` GUI later, the following plan will be followed:

### Goals

- Add a TUI frontend using `ratatui` for interactive, keyboard-driven operation
- Ensure all core logic, state, and validation remain UI-agnostic and reusable
- Architect the TUI and GUI to coexist, sharing application state and actions

### Plan

1. **Define AppState and AppAction**
	- Create a central `AppState` struct representing all user-editable fields, results, and UI status.
	- Define an `AppAction` enum for all user-driven events (input changes, menu selections, calculation triggers, etc).
	- Move all calculation, validation, and recommendation logic to pure functions operating on `AppState` and `AppAction`.

2. **Refactor CLI/Interactive Mode**
	- Refactor CLI and interactive mode to use `AppState` and `AppAction` for all state transitions and calculations.
	- Ensure prompt helpers and session memory use the same state/actions as the future TUI/GUI.

3. **Scaffold TUI with `ratatui`**
	- Add a new binary (e.g., `src/bin/tui.rs`) or feature flag for TUI mode.
	- Implement a basic event loop: render `AppState` to the terminal, dispatch `AppAction` on user input, update state, and re-render.
	- Start with core flows: band selection, antenna model, calculation mode, and results display.

4. **Iterate on TUI Features**
	- Add menus, popups, and navigation for all major CLI features.
	- Integrate export actions, error display, and session memory.
	- Add visual polish: layout, color, and accessibility improvements.

5. **Prepare for GUI Coexistence**
	- Ensure all TUI logic is isolated from core state/actions (no direct calculation or validation in TUI code).
	- Document and test the `AppState`/`AppAction` contract for future GUI use.
	- Plan for a future `iced` binary or feature flag using the same state/actions.

6. **Documentation and Testing**
	- Document the TUI architecture and how it shares logic with CLI and GUI.
	- Add regression tests for state transitions and calculation flows.

### Benefits

- Enables rapid TUI development without blocking GUI work
- Ensures all business logic is UI-agnostic and testable
- Allows users to choose terminal or desktop UI as preferred

### Affected Areas

- `src/app.rs`: AppState, AppAction, core logic
- `src/cli.rs`: refactor to use shared state/actions
- `src/bin/tui.rs` or `src/ui/tui.rs`: new TUI frontend
- `src/ui/` or future `src/bin/gui.rs`: future GUI frontend
- `tests/`: state/action regression coverage

## Advanced Input Support

- support direct frequency input such as `--freq 7.1`
- support multiple explicit frequencies such as `--freq-list 7.0,10.1`
- support user-defined band presets through a config file such as `bands.toml` or `bands.json`

These would make the tool more useful outside fixed amateur-band workflows.

## Transformer Recommendation and Selection

- keep `--transformer recommended` as the default entry point, but make the recommendation model more transparent in CLI help and output
- evaluate whether EFHW should remain fixed at `1:56` or be promoted to a ranked recommendation across `1:49`, `1:56`, and `1:64`
- consider an optional recommendation/optimization pass that compares plausible transformer ratios for the selected mode, antenna model, and band set
- present recommendations as guidance while still allowing explicit user override

The current implementation uses fixed recommended defaults by mode and antenna model. Future work here is about ranking or optimizing those choices rather than hard-coding more one-off rules.

## Search and Analysis Controls

- add a configurable `--precision` or `--step` option for non-resonant search resolution
- add batch output for multiple velocity factors or multiple transformer ratios in one run
- add a compact `--report` or `--summary` mode for automation-friendly output

These changes would improve power-user workflows without requiring a large architectural shift.

## Antenna Model Expansion

- add additional models beyond the current dipole, inverted-V, EFHW, loop, and OCFD support
- explore trap, hybrid, and other multi-section antenna models
- evaluate whether more antenna-specific feed recommendations should be modeled in the application layer

This remains one of the most substantial feature areas and likely requires changes in both `src/calculations.rs` and the user-facing configuration model.

## Export Improvements

- add richer machine-readable export formats such as YAML
- consider HTML export for printable/shareable reports
- improve the JSON schema for programmatic consumers if external integration becomes important

## Logging and Automation Modes

- add `--quiet` and/or `--verbose` flags
- add a `--dry-run` mode for automation and script validation

These would make the CLI easier to integrate into larger workflows.

## Suggested Priority Order

If work continues incrementally, a good order is:

1. error-handling cleanup
2. UI-integration prerequisite refactors
3. TUI integration with `ratatui` (see plan above)
4. first-pass `iced` UI with feature parity
5. configurable non-resonant search resolution
6. direct/custom frequency input
7. transformer recommendation optimization
8. logging and automation modes
9. next-generation antenna models

## Affected Areas

- `src/cli.rs`: options, validation, messaging
- `src/app.rs`: orchestration and error propagation
- `src/calculations.rs`: optimization and model logic
- `src/bands.rs`: custom-band support
- `src/export.rs`: format/schema evolution
- `tests/` and `scripts/`: regression coverage
