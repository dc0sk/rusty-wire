---
project: rusty-wire
doc: docs/backlog.md
status: living
last_updated: 2026-04-30
---

# Backlog

Ideas that have not yet been agreed on for the roadmap.
Move an item to `docs/roadmap.md` once it is confirmed.

---

## CLI / Analysis

- Persistent user preferences file (default units, region, mode)

## TUI (2.x, `ratatui`)

- **Band selection refinement**: expand region-specific preset coverage and add a "Custom" preset that opens a band-checklist panel where the user can tick individual bands — similar to a multi-select dialog — replacing the fixed preset table for advanced users.

## GUI (3.x, `iced`)

- Side-by-side comparison of multiple configurations (VF, transformer, antenna model)
- Graphical display of resonant points and non-resonant optima across the search window
- Guided setup wizards for common use cases (EFHW planning, random-wire exploration, OCFD review)
- Printable / shareable report preview
- Theme support and accessibility-focused layout
- Background task history for exports and heavier analysis runs
- Custom-band / preset editor once user-defined band support exists

## Antenna Models

- Hybrid / multi-section models
- Balun/unun optimizer engine: score candidate transformer ratios against selected bands and feed assumptions; this should be implemented before `advise` mode

## Practical Limits Follow-up

- Extend height realism beyond current 7/10/12 m presets:
  - Ground-class selector (poor/average/good) and optional soil conductivity/permittivity input
  - Conductor-diameter input for end-effect/feedpoint approximation refinement
  - Calibration matrix against NEC sweeps for representative bands and heights

## Infrastructure

- `bands.toml` / `bands.json` for user-defined band presets
- Multi-window or detachable analysis panes (GUI only)

## Maintenance / Project Review (post-v2.14.0)

Items identified during the v2.14.0 project review.

### Quick wins
- **A. Doc version sync**: bring `README.md` and `docs/cli-guide.md` up to v2.14.0; document `--export html`, `--validate-with-fnec`, `--fnec-gate`, and TUI saved sessions.
- **B. CONTRIBUTING.md**: codify the shadow-type pattern (CLI types vs domain types), the I/O-free app layer, test requirements, commit convention, and PR checklist.
- **C. Clippy cleanup**: ~14 warnings across the tree (collapsible `if`/`match`, `clamp`, `RangeInclusive::contains`, field-after-`Default`, unnecessary `to_vec`, `trim().split_whitespace()` redundancy). Tighten pre-push to `-- -D warnings`.
- **D. `cargo audit` in CI**: add a GitHub Actions job that runs `cargo audit` on every PR.

### Medium refactors
- **E. Split `src/app.rs`**: ~5,100 lines is a god module. Move into `src/app/{mod,state,views,display}.rs` (state machine, view types, formatting helpers).
- **F. Export formatter trait**: extract `trait ExportFormatter` from `src/export.rs` to remove ~500 LOC of duplication across the 12 `to_*` formatters.
- **G. Sessions / prefs persistence tests**: roundtrip tests for `SessionStore::save/list/delete` and `UserPrefs::save/load/apply_to_config` using a `tempfile` + `XDG_CONFIG_HOME` override.
- **H. TUI overlay tests**: state-driven integration tests for the export-preview, session-save, session-picker, and band-checklist overlays (no terminal needed; drive `handle_key` directly).

### Pre-3.x foundation
- **I. `RequestContext` on `AppRequest`**: optional `request_id` + `timestamp` to make `AppRequest`/`AppResponse` IPC- and async-friendly for the iced GUI.
- **J. Canonical TUI screenshots**: regenerate `docs/images/tui/*.png` via the existing `tui-doc-snapshots` binary so `docs/tui-screenshots.md` references real images.
