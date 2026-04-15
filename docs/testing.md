# Testing

This document describes the automated test coverage in Rusty Wire and how to run it.

## Overview

Rusty Wire currently uses three layers of testing:

- `cargo test` for unit tests and integration tests
- `scripts/test-multi-optima.sh` for empirical non-resonant multi-optima discovery
- `scripts/test-itu-region-bands.sh` for region-aware band listing regression checks

The codebase currently includes:

- Unit tests in `src/app.rs`, `src/bands.rs`, `src/calculations.rs`, `src/cli.rs`, and `src/export.rs`
- Integration tests in `tests/cli_integration.rs`

## Cargo Test

Run the full Rust test suite:

```bash
cargo test
```

This executes:

- Unit tests for application configuration and orchestration
- Unit tests for band and region models
- Unit tests for calculation algorithms and transformer ratios
- Unit tests for export path validation and formatting behavior
- Integration tests that invoke the compiled `rusty-wire` binary through `std::process::Command`

Useful variants:

```bash
cargo test -- --nocapture
cargo test cli_integration
cargo test run_calculation
```

Use `--nocapture` when you want to see test output while debugging.

## Integration Tests

The integration suite in `tests/cli_integration.rs` exercises the real binary rather than internal helper functions.

Current CLI integration coverage includes:

- no-argument invocation prints help
- mixed meter/feet constraints return a validation error
- invalid velocity values return a validation error
- `--list-bands --region 2` shows region-specific output
- recommended transformer defaults resolve correctly for non-resonant runs and EFHW mode
- multiple export formats ignore a custom `--output` path and use default names
- single-format export respects the requested `--output` path

These tests are intentionally high-level so that clap parsing and the real CLI flow are both covered.

## Script: Multi-Optima Sweep

Run:

```bash
./scripts/test-multi-optima.sh
```

Purpose:

- builds the project
- sweeps band selections, velocity factors, and wire-length windows
- stops at the first case where multiple non-resonant optima are found

This is not a unit test. It is an empirical regression script used to confirm that the optimization logic still produces multiple-optima cases under realistic parameter sweeps.

Environment variables:

- `BIN`: path to the binary to execute, default `target/debug/rusty-wire`
- `SWEEP_OUT`: path to the temporary sweep output file, default `/tmp/rw_sweep_result.txt`

The sweep uses CLI band names/ranges (for example `40m`, `20m,17m`, `40m-10m`) to match the current `--bands` parser behavior.

## Script: ITU Region Band Regression

Run:

```bash
./scripts/test-itu-region-bands.sh
```

Purpose:

- builds the project if needed
- checks listed bands for Regions 1, 2, and 3
- verifies region-specific amateur band ranges remain correct

This script is primarily a regression check for the region-aware band database and CLI band listing behavior.

## Recommended Workflow

For normal development:

```bash
cargo test
```

For changes affecting CLI behavior, regions, or exports:

```bash
cargo test
./scripts/test-itu-region-bands.sh
```

For changes affecting non-resonant optimization behavior:

```bash
cargo test
./scripts/test-multi-optima.sh
```

For a broader confidence check after larger refactors:

```bash
cargo test
./scripts/test-itu-region-bands.sh
./scripts/test-multi-optima.sh
```

## Notes

- `cargo test` is the authoritative fast feedback loop and should be run for all code changes.
- The shell scripts are slower and more scenario-oriented; they complement the Rust tests rather than replace them.
- Interactive mode now has stdin/stdout-driven unit coverage for menu navigation, band entry validation, region switching, transformer prompt handling, export-format rejection, and writer-based rendering of results and equivalent CLI suggestions.