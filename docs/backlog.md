# Backlog

Ideas that have not yet been agreed on for the roadmap.
Move an item to `docs/roadmap.md` once it is confirmed.

---

## CLI / Analysis

- `--transformer-sweep <r1,r2,...>` — like `--velocity-sweep` but over transformer ratios
- `--freq-list <f1,f2,...>` — multiple explicit frequencies in one run
- `--verbose` / `--dry-run` flags
- Expand `--info` metadata output with optional runtime/build details
- YAML and HTML export targets
- Persistent user preferences file (default units, region, mode)

## TUI (2.x, `ratatui`)

- **Band selection refinement**: the current preset list omits 60m and other region-specific bands. Two improvements to consider together:
  - Add 60m to relevant presets (or keep presets lean and note the gap)
  - "Custom" preset that opens a band-checklist panel where the user can tick individual bands — similar to a multi-select dialog. This would replace the fixed preset table for advanced users.
- Refresh and publish TUI screenshots in docs (defaults, model-specific result view, export flow)
- Keep info popup parity-complete: author, version, GitHub URL, and license
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

- **Trap dipole guidance**: For trap dipole mode, provide detailed information on:
  - Which trap frequencies / components to use (Q-factor, impedance guidance)
  - Physical installation positions along the wire
  - Leg length calculations for optimal multi-band resonance
  - Common trap configurations (40m/20m, 80m/40m, etc.)
- Hybrid / multi-section models
- Ranked transformer recommendation for EFHW (compare 1:49, 1:56, 1:64)

## Infrastructure

- `bands.toml` / `bands.json` for user-defined band presets
- Multi-window or detachable analysis panes (GUI only)
