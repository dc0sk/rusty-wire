# Roadmap

This page tracks high-value work after 2.1.0.

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

## 1) Error Handling Cleanup

- Return structured errors from app-layer calculation paths.
- Centralize final user-facing formatting in `src/cli.rs`.
- Reduce duplicated validation between CLI and interactive mode.

## 2) GUI Readiness (`iced` Prerequisites)

- Stabilize an app-layer request/response boundary around `AppConfig` and `AppResults`.
- Keep shared summaries and warnings reusable outside terminal output.
- Add app-layer contract tests to protect future UI work.
- Decide packaging direction: single binary with multiple entry paths, or library + separate binaries.

## 3) Search and Analysis Controls

- Add configurable non-resonant resolution (`--precision` or `--step`).
- Add optional batch runs over multiple velocity factors or transformer ratios.
- Add a compact automation-oriented summary/report mode.

## 4) Advanced Input Support

- Support direct frequency input (`--freq 7.1`).
- Support explicit frequency lists (`--freq-list ...`).
- Support user-defined band presets via config file.

## 5) Antenna and Recommendation Expansion

- Add more antenna models beyond current set.
- Improve recommendation transparency in help/output.
- Evaluate optional transformer ranking/optimization passes while keeping explicit overrides.

## Secondary Backlog

- New export targets (for example YAML/HTML)
- Logging and automation flags (`--quiet`, `--verbose`, `--dry-run`)
- UI-first enhancements after baseline parity (comparison views, saved sessions, richer visualizations)

## Affected Areas

- `src/cli.rs`: options, validation, messaging
- `src/app.rs`: orchestration and error propagation
- `src/calculations.rs`: optimization and model logic
- `src/bands.rs`: custom-band support
- `src/export.rs`: format/schema evolution
- `tests/` and `scripts/`: regression coverage
