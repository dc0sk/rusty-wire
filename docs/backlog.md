---
project: rusty-wire
doc: docs/backlog.md
status: living
last_updated: 2026-05-05
---

# Backlog

Ideas that have not yet been agreed on for the roadmap.
Move an item to `docs/roadmap.md` once it is confirmed.

---

## CLI / Analysis

- ~~**Persistent user preferences file**~~ ✅ done — `~/.config/rusty-wire/config.toml`; saved by `--save-prefs` (CLI) or `s` key (TUI); supports region, mode, velocity-factor, height, ground-class, conductor-diameter, units.

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

- ~~**Hybrid / multi-section models**~~ ✅ baseline done — `hybrid-multi` dipole mode added with per-side 40/35/25 section planning splits in CLI/TUI/app output.
- ~~**Balun/unun optimizer engine**~~ ✅ done — `--advise` ranks transformer ratios against selected bands and feed assumptions; `TransformerOptimizerView` / `AdviseView` used by CLI/TUI/GUI.

## Practical Limits Follow-up

- ~~**Extend height realism**~~ ✅ done — standard height presets (7/10/12 m) with height-aware skip-distance scaling.
- ~~**Ground-class selector**~~ ✅ done — `--ground poor|average|good` with first-order skip-distance scaling.
- ~~**Conductor-diameter input**~~ ✅ done — `--conductor-mm 1.0..4.0` with first-order impedance/length correction.
- ~~**NEC calibration matrix**~~ ✅ done (v2.9.0) — `nec_calibrated_dipole_r()` interpolates NEC corpus data (7/10/12 m AGL × poor/average/good).

## Infrastructure

- ~~**`bands.toml` / `bands.json`**~~ ✅ done (v2.16.0) — user-defined band presets loaded from `bands.toml`.
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
