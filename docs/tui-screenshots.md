# TUI Screenshot Plan

Use this checklist when preparing or refreshing TUI screenshots for docs and releases.

Keep only the canonical PNG set in this directory. Delete scratch captures or superseded alternates before committing.

## Canonical Image Paths

Store screenshots in:

- `docs/images/tui/01-default-layout.png`
- `docs/images/tui/02-trap-dipole-results.png`
- `docs/images/tui/03-non-resonant-window.png`
- `docs/images/tui/04-about-popup.png`
- `docs/images/tui/05-results-scroll.png`

## What To Capture

1. `01-default-layout.png`
- App opened in TUI default state
- Config panel visible on left, results panel visible on right
- One line of keyboard hints visible at bottom

2. `02-trap-dipole-results.png`
- Antenna set to trap dipole
- Results showing trap dipole total and per-element lines

3. `03-non-resonant-window.png`
- Mode set to non-resonant
- Wire min/max fields visible in config panel
- Results include best non-resonant recommendation

4. `04-about-popup.png`
- Info popup opened via `i` or `?`
- Popup must show: version, author, GitHub URL, license, platform

5. `05-results-scroll.png`
- Results panel focused
- Scrolled content visible (not only top of document)

## Placement Map

- `README.md`
  - TUI section: include `01-default-layout.png` and `04-about-popup.png`
- `docs/cli-guide.md`
  - Interactive/TUI section: include `02-trap-dipole-results.png`, `03-non-resonant-window.png`, `05-results-scroll.png`

## Capture Notes

- Prefer 120x30 or larger terminal size for consistent framing.
- Redact local file paths or shell history if visible.
- Re-capture after any TUI layout, keybinding, built-in preset label, or info-popup content change.
- Regenerate the HTML gallery with `cargo run --bin tui-doc-snapshots`, then capture each section by id (`01-default-layout` through `05-results-scroll`) into the canonical PNG paths.
- The HTML gallery already provides the framed presentation layer for capture; do not keep separate ad-hoc screenshot variants once the canonical five PNGs have been refreshed.