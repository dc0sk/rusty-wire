# Architecture

This document describes the current architecture of Rusty Wire, including module responsibilities and the main execution flow.

## High-Level Structure

Rusty Wire is a single-binary Rust application with a small, explicit module split:

- `src/main.rs`: entry point
- `src/cli.rs`: command-line parsing, interactive mode, terminal output, export dispatch
- `src/app.rs`: shared application types and orchestration of calculations
- `src/bands.rs`: region-aware band database and band lookup helpers
- `src/calculations.rs`: physics calculations, optimization algorithms, and result models
- `src/export.rs`: output-path validation and formatting/file-writing for export formats

The codebase is organized so that core calculation logic is isolated from user interaction and file-system concerns.

## Execution Flow

The runtime flow is:

1. `src/main.rs` collects process arguments.
2. `src/cli.rs` decides whether to show clap help, launch interactive mode, list bands, or run a normal CLI calculation.
3. `src/cli.rs` converts validated user input into `AppConfig`.
4. `src/app.rs` runs the calculation pipeline.
5. `src/app.rs` queries `src/bands.rs` for region-specific band definitions.
6. `src/app.rs` delegates computational work to `src/calculations.rs`.
7. `src/cli.rs` renders terminal output and optionally calls `src/export.rs` to write files.

## Module Responsibilities

### `src/main.rs`

This file is intentionally thin.

- collects `env::args()`
- delegates all behavior to `cli::run_from_args`

Keeping `main.rs` minimal makes CLI behavior testable and keeps process-level setup in one place.

### `src/cli.rs`

This is the outer application layer.

Responsibilities:

- clap-based parsing of non-interactive CLI arguments
- explicit `--interactive` mode dispatch
- validation of CLI-specific constraints such as mixed meter/feet inputs
- interactive prompts and menu flow
- listing region-aware bands
- formatting terminal output for resonant and non-resonant runs
- coordinating exports and export warnings

This module owns I/O:

- stdin/stdout interaction for interactive mode
- stderr validation messages
- user-facing console formatting

It does not implement the actual RF/wire math itself.

### `src/app.rs`

This module is the application orchestration layer.

Responsibilities:

- defines shared enums such as `CalcMode`, `ExportFormat`, and `UnitSystem`
- defines `AppConfig` and `AppResults`
- normalizes reusable wire-window input through shared app-layer helpers used by CLI and future UI code
- parses reusable band-selection input and band-label resolution through shared app-layer helpers used by CLI and future UI code
- formats reusable transformer-recommendation fallback messaging for CLI and future UI code
- maps selected band indices to region-specific band definitions
- resolves mode- and antenna-aware default transformer recommendations
- runs one calculation pass for the full request
- assembles optimization and recommendation outputs

This module sits between the UI layer (`cli.rs`) and the math/data layers (`bands.rs`, `calculations.rs`).

It is intentionally mostly free of direct I/O, which makes it easier to unit-test and reuse.

### `src/bands.rs`

This module is the static domain-data layer.

Responsibilities:

- defines `Band`, `BandType`, and `ITURegion`
- stores amateur and shortwave band definitions
- applies region-specific variations for affected amateur bands
- provides region-aware band lookup functions
- provides human-readable display formatting for bands and regions

This is the source of truth for which frequencies the rest of the application uses.

### `src/calculations.rs`

This module contains the computational core.

Responsibilities:

- transformer ratio modeling and parsing
- resonant length calculations
- corrected-length calculations based on transformer impedance ratios
- skip-distance helpers and summary calculations
- non-resonant optimum search across a wire-length window
- resonant compromise generation across multiple selected bands
- result data structures for calculations and recommendations

This is the highest-value logic to protect with unit tests because mistakes here directly affect output correctness.

### `src/export.rs`

This module is the export boundary.

Responsibilities:

- validating output paths
- preventing unsafe export destinations such as absolute paths and `..`
- generating default output filenames per format
- converting calculation results into CSV, JSON, Markdown, or plain-text output
- writing formatted output to disk

The formatting functions are pure string builders; file writes are concentrated here so filesystem behavior stays localized.

## Separation of Concerns

The project follows a simple layered approach:

- CLI and interactive UX in `cli.rs`
- request orchestration in `app.rs`
- domain data in `bands.rs`
- algorithms in `calculations.rs`
- file export in `export.rs`

This split keeps most changes localized:

- adding a new flag usually touches `cli.rs`
- changing business rules often touches `app.rs`
- updating regional band allocations touches `bands.rs`
- changing wire math or optimization touches `calculations.rs`
- changing output formats or path safety touches `export.rs`

## Testing Architecture

Testing is split by layer:

- unit tests in core modules verify algorithms, enums, and validation logic
- integration tests under `tests/` verify real CLI behavior through the compiled binary
- shell scripts under `scripts/` provide broader scenario and regression checks

For operational details, see [testing.md](testing.md).

## Design Tradeoffs

The current design deliberately favors:

- straightforward module boundaries over heavy abstraction
- direct data flow over framework-style indirection
- unit-testable core logic with a thin CLI shell
- a small number of explicit export formats and calculation modes

The main remaining structural opportunity is cleaner error propagation from `app.rs` into `cli.rs`. Interactive mode is now exercised with stdin/stdout-driven tests, so that area is in better shape than earlier releases.