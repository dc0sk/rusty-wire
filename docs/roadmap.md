---
project: rusty-wire
doc: docs/roadmap.md
status: living
last_updated: 2026-05-05
---

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

## Recently Completed

### v2.12.0
- ✅ **EFHW transformer comparison**: ranked 1:49/1:56/1:64 table in results header and TUI panel, showing SWR/efficiency/loss per ratio.
- ✅ **`--transformer-sweep`**: sweep over comma-separated transformer ratios; shows SWR, efficiency, and per-band lengths per ratio.
- ✅ **Sustainability gating (`--fnec-gate`)**: removes `Rejected` candidates from `--advise` output; status badges in CLI and colour-coded TUI panel.
- ✅ **Trap dipole guidance**: structured section with trap frequency, inner/outer leg lengths, full span, and example L/C component pairs per band pair.

### v2.3.0 and earlier

- clap-based CLI with `--interactive`, named band/range selection, ITU region support
- antenna model expansion: dipole, inverted-v, EFHW, loop, OCFD; recommended transformer defaults
- export path hardening; SBOM generation and pre-push enforcement
- structured `AppError` across all validation paths; proper CLI exit codes
- shadow CLI types decoupled from domain types; library crate (`src/lib.rs`)
- app-layer contract tests for `AppRequest → AppResponse` boundary
- shared app-layer wire-window normalization and band-selection parsing
- `--step`, `--quiet`, `--freq`, `--velocity-sweep` flags
- standard antenna-height presets (`--height 7|10|12`) with first-order height-aware skip-distance scaling
- ground-class presets (`--ground poor|average|good`) with first-order skip-distance scaling

---

## 2.x Priorities — TUI Readiness

Work needed before or alongside the TUI. Items are roughly in dependency order.

### 1) App-layer API hardening
- ✅ Stabilise `AppConfig` / `AppResults` / `AppRequest` / `AppResponse` as the single shared interface used by CLI, TUI, and future GUI
- ✅ Separate pure result formatting from terminal printing so TUI can reuse summaries, labels, warnings, and recommendation text — `results_display_document()` used by both CLI and TUI
- ✅ Introduce view-friendly metadata: `ResultsDisplayDocument` carries `transformer_explanation: TransformerRatioExplanation` and `skipped_band_details: Vec<SkippedBandDetail>` for structured access by TUI/GUI without extra calls

### 2) Custom-band and frequency input
- ✅ Support user-defined band presets via a config file (`bands.toml` or similar) — v2.16.0
- ✅ `--freq-list <f1,f2,...>` for multiple explicit frequencies in one run — implemented, integration-tested

### 3) Additional antenna models
- ✅ Trap dipole multi-section model with structured guidance (trap freq, leg lengths, component examples) — v2.12.0
- ✅ Antenna-specific feed recommendations at the app layer — v2.19.0: `TransformerMismatchWarning` in `ResultsDisplayDocument`; TUI shows `⚠` when configured ratio differs from model recommendation
- ✅ Hybrid multi-section dipole baseline model — implemented: `--antenna hybrid-multi` (CLI + TUI), per-band 40/35/25 section split guidance, app-layer narrative support

### 3a) Balun optimizer foundation (prerequisite for advise mode)
- ✅ App-layer optimizer ranks balun/unun ratios for the selected band set and antenna assumptions
- ✅ Optimizer output surfaced as structured `TransformerOptimizerView` / `AdviseView` usable by CLI/TUI/GUI

### 3b) `advise` candidate ranking mode
- ✅ User-facing `--advise` flag produces ranked wire-length + balun/unun candidates
- ✅ Compact scoring metadata per candidate: efficiency %, mismatch loss dB, resonance clearance %, score
- ✅ Tradeoff notes (v2.11.0): one-sentence human-readable summary per candidate — best match, SWR into target impedance, ATU advice. Available in CLI output and all export formats (CSV, JSON, Markdown, TXT). **GAP items 3a/3b closed.**

### 3c) Practical-limits mitigation (height/ground/conductor realism)
- Implemented first pass: standardized antenna-height presets (7 m, 10 m, 12 m) with height-aware skip-distance scaling.
- Implemented second pass: ground-class presets (poor/average/good) with additional skip-distance scaling.
- Implemented third pass: optional conductor-diameter input (`--conductor-mm 1.0..4.0`) with first-order impedance/length correction.
- ✅ Implemented fourth pass: NEC-calibrated feedpoint resistance and mismatch/SWR estimates (v2.9.0). `nec_calibrated_dipole_r()` interpolates height/ground anchor points from fnec-rust corpus data (7/10/12 m AGL × poor/average/good). Band display now shows `Est. feedpoint R: XX.X Ω (NEC-calibrated, SWR ≈ N.N:1 into ZZ Ω)`. Transformer optimizer uses calibrated R for all dipole-family gamma and mismatch-loss calculations. **GAP-011 item 3c closed.**

### 6) NEC2 card deck export
- ✅ `nec` export format (`--export nec` / TUI `N` key) — implemented in v2.18.0
- Emits CM/GW/GE/GN/FR/EX/RP/EN cards; all six antenna models; Sommerfeld-Norton ground; no NEC2 runtime dependency

### 4) Interactive-mode testability
- ✅ Refactor interactive prompts to accept injected I/O — done
- ✅ Automated test coverage for all interactive prompt paths — done in v2.18.0 (4 new tests; `nec` format bug also fixed)

### 5) TUI (`ratatui`)
- ✅ Add `src/bin/tui.rs` (or `--tui` flag on the main binary)
- ✅ Event loop: render `AppConfig` state → dispatch input actions → recalculate → re-render
- ✅ Feature parity with current CLI/interactive: band selection, antenna model, calc mode, wire window, transformer, export
- ✅ Shared `AppState` / `AppAction` types so GUI reuse is straightforward
- **Status**: Completed in v2.8.0+. TUI accessible via `--tui` / `-t` flag with 35 comprehensive unit tests.

---

## 3.x Priorities — GUI Readiness

Depends on TUI being stable and `AppState`/`AppAction` being settled.

- GUI front-end using `iced`, built on the same app-layer API as the TUI
- Packaging decision: single binary with multiple entry paths, or separate crates
- See `docs/backlog.md` for GUI-specific enhancement ideas

This remains one of the most substantial feature areas and likely requires changes in both `src/calculations.rs` and the user-facing configuration model.

These would make the CLI easier to integrate into larger workflows.

## Suggested Priority Order

If work continues incrementally, a good order is:

1. error-handling cleanup
2. UI-integration prerequisite refactors
3. TUI integration with `ratatui` (see plan above)
4. first-pass `iced` UI with feature parity
5. configurable non-resonant search resolution
6. direct/custom frequency input
7. NEC2 card deck export (`--export nec`)
8. transformer recommendation optimization
9. logging and automation modes
10. next-generation antenna models

## Affected Areas

- `src/cli.rs`: options, validation, messaging
- `src/app.rs`: orchestration and error propagation
- `src/calculations.rs`: optimization and model logic
- `src/bands.rs`: custom-band support
- `src/export.rs`: format/schema evolution
- `src/nec_export.rs`: NEC2 card deck generation
- `tests/` and `scripts/`: regression coverage
