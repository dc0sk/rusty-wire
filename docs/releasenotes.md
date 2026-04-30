---
project: rusty-wire
doc: docs/releasenotes.md
status: living
last_updated: 2026-04-30
---

# Release Notes

User-facing summaries of Rusty Wire releases. For detailed technical changes, see [CHANGELOG.md](CHANGELOG.md).

---

## Unreleased (Development)

### What's Coming

The next release focuses on **output format stability** and **testing infrastructure**:
- Locked output contracts for CSV and JSON exports (breaking format changes now require explicit versioning)
- CI gates for documentation, code quality, and test coverage (90% threshold)
- Export format contract tests (PAR-001 v1 CSV, PAR-002 v1 JSON) implemented
- Corpus validation infrastructure in place; ITU-R P.368 skip-distance seed case active
- NEC reference sweeps deferred (GAP-011)

**Estimated Q3 2026.**

---

## [2.7.0] — April 29, 2026

**The Release Pipeline & TUI Advise Update**

### Highlights

**Packaged for Linux distributions.** Rusty Wire is now available as an Arch Linux package and Debian `.deb` for `x86_64` and ARM64 (e.g., Raspberry Pi). Install via package manager or build from tarball. SBOM (software bill of materials) is included for supply-chain visibility.

**TUI export and advise panel.** Export results directly from the TUI (press `e`, `E`, `m`, or `t` for CSV, JSON, Markdown, or text). New **advise mode** ranks wire-length + balun/unun candidates by efficiency score; toggle with `a` in results view.

**User preferences persist.** Set your default region, antenna model, velocity factor, height, ground class, and output units once in `~/.config/rusty-wire/config.toml`; they're remembered across sessions in CLI and TUI.

**Custom band presets via TOML.** Define named band sets in `bands.toml` and switch between them in the TUI with ←/→. Example preset file included in packaging.

### New Features

- **Advise mode** (`--advise`): ranked wire + balun/unun candidates with efficiency estimates and tradeoff notes
- **TUI exports**: press `e`/`E`/`m`/`t` to save results in CSV/JSON/Markdown/TXT format directly from the TUI
- **Platform info**: `--info` and TUI About popup now show OS and CPU architecture
- **Persistent config**: `~/.config/rusty-wire/config.toml` saves and restores session preferences
- **Custom band presets**: load named band sets from TOML; use in CLI (`--bands-preset <name>`) and TUI (cycle with ←/→)
- **Conductor-diameter model**: `--conductor-mm 1.0..4.0` corrects wire lengths for different wire gauges
- **Advise validation** (optional): `--validate-with-fnec` cross-checks candidates with fnec-rust if available
- **NEC calibration workflow**: new scripts and reference data to fit conductor-correction constants

### Breaking Changes

- None. All new features are additive.

### Improvements

- TUI now remembers your last region and band set between runs
- Skip-distance estimates now account for antenna height and ground class
- 60m band added to TUI "160m–10m + 60m (10 bands)" preset
- Release binaries and CI now multi-target x86_64 and ARM64 architectures

### Downloads

- **Binary releases**: Available on GitHub Releases for x86_64 and ARM64 Linux
- **Arch Linux**: `pacman -S rusty-wire` (or `rusty-wire-git` for latest main)
- **Debian/Ubuntu**: Download `.deb` from GitHub Releases or build with `cargo deb`
- **Other platforms**: Build from source with `cargo build --release`

---

## [2.6.0] — April 25, 2026

**Antenna Model Metadata & TUI Hardening**

### Highlights

- Antenna model descriptions now include transformer-ratio explanations and impedance expectations
- TUI global key-event handling improved (non-Press events now rejected consistently)

### Breaking Changes

- None.

---

## [2.5.2] — April 21, 2026

**Trap Dipole Restoration**

### Highlights

- Trap-dipole antenna model (listed in 2.5.1 docs but missing from 2.5.1 binary) is now fully restored
- All trap-dipole features working: CLI, TUI, export formats, transformer recommendations

### Breaking Changes

- None.

---

## [2.5.1] — April 21, 2026

**Trap Dipole & Info Surfaces**

### Highlights

- New antenna model: **trap dipole** (`--antenna trap-dipole`, aliases: `trap`, `trapdipole`)
- Project info surfaces: `--info` flag and TUI About popup now show version, author, GitHub URL, and license
- TUI screenshot assets added to documentation

### Breaking Changes

- None.

---

## [2.5.0] — April 21, 2026

**Multi-Frequency Input & TUI Foundation**

### Highlights

- **`--freq-list <f1,f2,...>`**: calculate wire lengths for multiple explicit frequencies in one run (e.g., `--freq-list 3.5,7.0,14.0`)
- **Keyboard-driven TUI** (`rusty-wire-tui` binary): full terminal UI with configuration panel and scrollable results, powered by `ratatui`
- **Pure state machine**: `AppState` / `AppAction` API isolates all business logic from I/O, enabling TUI and future GUI development
- **Five named band presets**: "40m–10m", "80m–10m", "160m–10m", "20m–10m", "Contest 80/40/20/15/10"
- **Preset cycling**: velocity factor (0.50–1.00) and transformer ratio presets cycled with ←/→ in TUI

### Breaking Changes

- None. All features are additive.

### What This Means for Users

- **TUI is keyboard-only**: no mouse required. Tab to navigate config fields, ↓↑ or j/k to scroll results
- **CLI remains the reference**: TUI feature parity is a development constraint
- **State is preserved**: TUI remembers your last configuration until you close the program

---

## [2.4.0] — April 21, 2026

**Full TUI with ratatui & State Machine**

### Highlights

- Complete terminal UI (`rusty-wire-tui`): two-panel layout with editable config (left) and scrollable results (right)
- State machine (`AppState` / `AppAction` / `apply_action`): pure, testable orchestration of all frontend inputs
- Panic-safe terminal restore: crossterm mode and alternate screen always restored on crash

### Breaking Changes

- None.

---

## [2.3.0] — April 20, 2026

**Scripting Support & Library Entry Point**

### Highlights

- **`--quiet` flag**: suppress results table for scripting (non-resonant: single line; resonant: silent on success)
- **`--freq <MHz>` flag**: calculate for a single explicit frequency instead of scanning bands
- **`--velocity-sweep <v1,v2,...>` flag**: run the same config at multiple velocity factors and compare
- **Library crate** (`src/lib.rs`): external frontends can now depend on `rusty_wire::app::*` for their own UI
- **CLI exit codes**: proper non-zero codes on error, zero on success
- **Structured error handling**: all errors are typed (`AppError` enum); validation happens early
- **`--step` flag**: configurable non-resonant search resolution (default 0.05 m)

### Breaking Changes

- `CalcMode`, `ExportFormat`, `UnitSystem`, and `AntennaModel` are no longer `clap::ValueEnum` directly. The CLI is now decoupled from domain types via shadow types (`CliAntennaModel`, etc.). This is an internal change; the CLI interface is unchanged.

### What This Means for Users

- Scripts can now use `--quiet --freq 7.0` to get a single wire length with exit code 0/1
- You can compare how velocity factor affects resonance in one command: `--velocity-sweep 0.90,0.95,0.98`
- We're building a library so that other tools can use Rusty Wire calculations without shelling out

---

## [2.2.0] — April 14, 2026

**Early Releases (v1.x–v2.1.x)**

Earlier releases focused on core CLI features (band selection, antenna models, export formats, ITU regions) and the foundation for multi-frontend architecture. See [CHANGELOG.md](CHANGELOG.md) for full history.

---

## Installation & Quick Start

**Install from package manager (Linux):**

```bash
# Arch Linux
pacman -S rusty-wire

# Debian/Ubuntu (from GitHub Releases)
sudo dpkg -i rusty-wire_2.7.0_amd64.deb
```

**Or build from source:**

```bash
cargo install --path .
```

**Try it:**

```bash
# CLI
rusty-wire --bands 40m,20m --antenna dipole

# Interactive mode
rusty-wire --interactive

# TUI
rusty-wire-tui
```

---

## Support & Feedback

- **GitHub Issues**: Report bugs and request features at https://github.com/dc0sk/rusty-wire/issues
- **Documentation**: See `docs/` directory for CLI guide, math reference, testing guide, and architecture notes

---
