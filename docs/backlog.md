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

- ~~**Band selection refinement**~~: ✅ done — "Custom…" preset opens a band-checklist overlay where the user can tick individual bands; Enter confirms.

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
- ~~**A. Doc version sync**~~ ✅ done
- ~~**B. CONTRIBUTING.md**~~ ✅ done
- ~~**C. Clippy cleanup**~~ ✅ done
- ~~**D. `cargo audit` in CI**~~ ✅ done

### Medium refactors
- ~~**E. Split `src/app.rs`**~~ ✅ done — `src/app/{mod,state,advise}.rs`
- ~~**F. Export formatter trait**~~ ✅ done — `trait ExportFormatter` in `src/export.rs`
- ~~**G. Sessions / prefs persistence tests**~~ ✅ done — roundtrip tests in `src/sessions.rs` + `src/prefs.rs`
- ~~**H. TUI overlay tests**~~ ✅ done — session-save, session-picker, export-preview integration tests

### Pre-3.x foundation
- ~~**I. `RequestContext` on `AppRequest`**~~ ✅ done — `AppRequest`/`AppResponse` carry optional `RequestContext { request_id, timestamp_secs }`
- ~~**J. Canonical TUI screenshots**~~ ✅ done — `scripts/render-tui-snapshots.py` renders `docs/images/tui/*.png` from the `tui-doc-snapshots` HTML output
