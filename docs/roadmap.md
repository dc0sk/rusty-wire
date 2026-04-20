# Roadmap

This document captures the milestone plan and near-term priorities after the 2.3.0 release.

New ideas that are not yet agreed on go to `docs/backlog.md` first.

---

## Version Milestones

| Series | Goal |
|--------|------|
| **2.x** | TUI front-end using `ratatui` — keyboard-driven, full feature parity with current CLI/interactive mode |
| **3.x** | GUI front-end using `iced` — desktop app, built on the same app-layer API as the TUI |

---

## Recently Completed (2.3.0 and earlier)

- clap-based CLI with `--interactive`, named band/range selection, ITU region support
- antenna model expansion: dipole, inverted-v, EFHW, loop, OCFD; recommended transformer defaults
- export path hardening; SBOM generation and pre-push enforcement
- structured `AppError` across all validation paths; proper CLI exit codes
- shadow CLI types decoupled from domain types; library crate (`src/lib.rs`)
- app-layer contract tests for `AppRequest → AppResponse` boundary
- shared app-layer wire-window normalization and band-selection parsing
- `--step`, `--quiet`, `--freq`, `--velocity-sweep` flags

---

## 2.x Priorities — TUI Readiness

Work needed before or alongside the TUI. Items are roughly in dependency order.

### 1) App-layer API hardening
- Stabilise `AppConfig` / `AppResults` / `AppRequest` / `AppResponse` as the single shared interface used by CLI, TUI, and future GUI
- Separate pure result formatting from terminal printing so TUI can reuse summaries, labels, warnings, and recommendation text
- Introduce view-friendly metadata where useful: recommended-transformer explanations, skipped-band reasons, per-band annotations

### 2) Custom-band and frequency input
- Support user-defined band presets via a config file (`bands.toml` or similar)
- `--freq-list <f1,f2,...>` for multiple explicit frequencies in one run

### 3) Additional antenna models
- Trap dipole and other multi-section models
- Evaluate antenna-specific feed recommendations at the app layer

### 4) Interactive-mode testability
- Refactor interactive prompts to accept injected I/O (already partially done)
- Add automated test coverage for all interactive prompt paths

### 5) TUI (`ratatui`)
- Add `src/bin/tui.rs` (or `--tui` flag on the main binary)
- Event loop: render `AppConfig` state → dispatch input actions → recalculate → re-render
- Feature parity with current CLI/interactive: band selection, antenna model, calc mode, wire window, transformer, export
- Shared `AppState` / `AppAction` types so GUI reuse is straightforward

---

## 3.x Priorities — GUI Readiness

Depends on TUI being stable and `AppState`/`AppAction` being settled.

- GUI front-end using `iced`, built on the same app-layer API as the TUI
- Packaging decision: single binary with multiple entry paths, or separate crates
- See `docs/backlog.md` for GUI-specific enhancement ideas

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
