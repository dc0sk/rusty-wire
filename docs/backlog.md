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

- YAML and HTML export targets
- Persistent user preferences file (default units, region, mode)

## TUI (2.x, `ratatui`)

- **Band selection refinement**: expand region-specific preset coverage and add a "Custom" preset that opens a band-checklist panel where the user can tick individual bands — similar to a multi-select dialog — replacing the fixed preset table for advanced users.
- Live recalculation as inputs change (with debounce for non-resonant search)
- Collapsible result panels per antenna model
- Visual highlighting of recommended transformer ratio and skipped bands
- Export preview before writing files
- Saved sessions / named configurations

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
