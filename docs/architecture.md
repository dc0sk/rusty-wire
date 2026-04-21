# Architecture

This document describes the current architecture of Rusty Wire, including module responsibilities, key types, and the main execution flows.

> Last updated: 2026-04-21 (v2.5.2)

---

## Binaries and Crate Structure

The crate produces two binaries from a shared library:

| Binary | Entry point | Front-end |
|--------|-------------|-----------|
| `rusty-wire` | `src/main.rs` → `src/bin/` implicit | CLI / interactive |
| `tui` | `src/bin/tui.rs` | ratatui TUI |

Both binaries depend on the library crate defined in `src/lib.rs`.

`src/lib.rs` exports:
- `pub mod app` — shared application layer (I/O-free)
- `pub mod bands` — band database (I/O-free)
- `pub mod calculations` — RF math (I/O-free)
- `pub mod tui` — TUI front-end
- `pub(crate) mod cli` — CLI front-end (not public API)
- `pub(crate) mod export` — file-export (not public API)
- `pub fn run_cli(args: &[String]) -> bool` — main entry point for the CLI binary

---

## Module Responsibilities

### `src/main.rs`

Minimal. Calls `rusty_wire::run_cli(&args)` and exits with code 1 on failure.

### `src/bin/tui.rs`

Minimal. Calls `rusty_wire::tui::run()` and exits with code 1 on error.

### `src/cli.rs`

The CLI and interactive-mode front-end. Internal to the crate (`pub(crate)`).

**Argument parsing (clap derive):**

```
struct Cli {
    region       -- CliITURegion (default: 1)
    mode         -- CliCalcMode (resonant | non-resonant)
    bands        -- Option<String> (e.g. "40m,20m,10m-15m")
    velocity     -- f64 (default 0.95)
    transformer  -- CliTransformerSelection (recommended | 1:1 | 1:9 | ...)
    antenna      -- Option<CliAntennaModel>
    wire_min/max -- Option<f64> meters or feet
    step         -- Option<f64>
    units        -- Option<CliUnitSystem> (m | ft | both)
    export       -- Option<Vec<CliExportFormat>> (csv, json, markdown, txt)
    output       -- Option<String>
    list_bands   -- bool
    interactive  -- bool (-i)
    info         -- bool
    freq         -- Option<f64> MHz (bypasses band selection)
    freq_list    -- Option<Vec<f64>> MHz (bypasses band selection)
    quiet        -- bool
    velocity_sweep -- Option<Vec<f64>>
}
```

Shadow CLI enums (`CliAntennaModel`, `CliCalcMode`, etc.) are distinct from domain enums.  Each has a `From<Cli*> for Domain*` impl.

`AntennaModel` values accepted by `--antenna`:  
`dipole` | `inverted-v` | `efhw` | `loop` | `ocfd` | `trap-dipole`

Interactive aliases for antenna (prompt only, not clap):  
`d` / `e` / `l` / `v` / `o` / `t` / `trap` / `trap-dipole` / `trapdipole`

**Entry point:** `pub fn run_from_args(args: &[String]) -> bool`

Dispatch order:
1. `--info` → print metadata, return
2. `--interactive` → `run_interactive()`, return
3. `--list-bands` → `show_all_bands_for_region()`, return
4. Normal CLI run → validate, build `AppConfig`, call `execute_request_checked`, render output

**I/O ownership:** all stdin/stdout/stderr; the calculation layers have no I/O.

### `src/app.rs`

The I/O-free application orchestration layer. Shared between CLI and TUI.

**Key types:**

```rust
struct AppConfig {
    band_indices: Vec<usize>,   // 0-based indices into BANDS
    velocity_factor: f64,
    mode: CalcMode,             // Resonant | NonResonant
    wire_min_m, wire_max_m, step_m: f64,
    units: UnitSystem,          // Metric | Imperial | Both
    itu_region: ITURegion,
    transformer_ratio: TransformerRatio,
    antenna_model: Option<AntennaModel>,
    custom_freq_mhz: Option<f64>,     // bypasses band selection
    freq_list_mhz: Vec<f64>,          // bypasses band selection, overrides custom_freq_mhz
}

struct AppResults {
    config: AppConfig,
    calculations: Vec<WireCalculation>,
    skipped_band_indices: Vec<usize>,
    recommendation: Option<NonResonantRecommendation>,
    // ... resonant compromise, OCFD optimum, velocity sweep, etc.
}

// State-machine types (used by TUI/GUI):
struct AppState {
    config: AppConfig,
    results: Option<AppResults>,
    error: Option<AppError>,
}

enum AppAction {
    SetBandIndices(Vec<usize>),
    SetMode(CalcMode),
    SetAntennaModel(Option<AntennaModel>),
    SetVelocityFactor(f64),
    SetTransformerRatio(TransformerRatio),
    SetWireMin(f64), SetWireMax(f64), SetStep(f64),
    SetUnits(UnitSystem),
    SetItuRegion(ITURegion),
    SetCustomFreq(Option<f64>),
    SetFreqList(Vec<f64>),
    RunCalculation,
    ClearResults,
    ClearError,
}

fn apply_action(state: AppState, action: AppAction) -> AppState  // pure, no I/O
```

**Antenna models:**

```rust
enum AntennaModel {
    Dipole, InvertedVDipole, EndFedHalfWave,
    FullWaveLoop, OffCenterFedDipole, TrapDipole,
}
```

`FromStr` aliases: `trap-dipole` | `trap` | `trapdipole` etc.

**Key pure functions:**

| Function | Purpose |
|----------|---------|
| `run_calculation(config)` | run one full pass; returns `AppResults` |
| `execute_request_checked(req)` | validated entry used by CLI |
| `apply_action(state, action)` | state-machine update for TUI/GUI |
| `results_display_document(results)` | returns `ResultsDisplayDocument` — all render-ready text |
| `band_display_view(row, units, model)` | per-band display lines |
| `band_listing_view(region)` | list of all bands for a region (used by TUI checklist) |
| `parse_band_selection(str, region)` | parse "40m,20m,10m-15m" → `Vec<usize>` |
| `recommended_transformer_ratio(mode, model)` | default ratio for a mode+model combo |
| `validate_velocity_sweep(velocities)` | validate a VF sweep list |

**Display document model (`ResultsDisplayDocument`):**

```rust
struct ResultsDisplayDocument {
    overview_heading: &'static str,
    overview_header_lines: Vec<String>,
    band_views: Vec<BandDisplayView>,
    summary_lines: Vec<String>,
    sections: Vec<ResultsTextSectionView>,
    warning_lines: Vec<String>,
}
```

Both the TUI and CLI consume `results_display_document()` for consistent rendering.

**Constants:**

```rust
DEFAULT_BAND_SELECTION: [usize; 7] = [4, 5, 6, 7, 8, 9, 10]  // 40m–10m
DEFAULT_ITU_REGION: ITURegion::Region1
DEFAULT_TRANSFORMER_RATIO: TransformerRatio::R1To1
```

### `src/bands.rs`

Static domain-data layer. I/O-free.

**Key types:** `Band`, `BandType` (`HF` | `MF`), `ITURegion` (`Region1` | `Region2` | `Region3`)

**`BANDS` array:** 21 entries (0-based). Indices used throughout:
- 0 = 160m, 1 = 80m, 2 = 60m, 3 = 40m, 4 = 30m, 5 = 20m, 6 = 17m, 7 = 15m, 8 = 12m, 9 = 10m, then shortwave (indices 10–20)

Note: `AppConfig.band_indices` stores 0-based indices; `DEFAULT_BAND_SELECTION = [4,5,6,7,8,9,10]` is 30m–10m.  
Note: `ordered_band_indices_for_region` returns 1-based for display; band selection parsing is also 1-based.

Region-specific variations applied:
- 80m: R1 3.5–3.8, R2 3.5–4.0, R3 3.5–3.9
- 60m: WRC-15 shared 5.3515–5.3665
- 40m: R1/R3 7.0–7.2, R2 7.0–7.3

**Key functions:** `get_bands_for_region(region)`, `get_band_by_index_for_region(idx, region)`

### `src/calculations.rs`

RF physics and optimization. I/O-free.

**Key types:** `WireCalculation`, `TransformerRatio`, `NonResonantSearchConfig`, `ResonantCompromise`

`WireCalculation` fields include per-model lengths:
- half-wave dipole (m/ft)
- inverted-V legs and span
- EFHW
- full-wave loop circumference
- OCFD 33/67 split
- trap dipole total and per-leg (`trap_dipole_total_m`, `trap_dipole_leg_m`, `trap_dipole_total_ft`, `trap_dipole_leg_ft`)

Trap dipole formula: `total_ft = (450.0 / freq_mhz) * velocity_factor`, then converted to metric; `leg = total / 2`.

**Transformer ratios available:** 1:1 | 1:2 | 1:4 | 1:5 | 1:6 | 1:9 | 1:16 | 1:49 | 1:56 | 1:64

### `src/export.rs`

File-export boundary. `pub(crate)`.

Formats: CSV, JSON, Markdown, TXT — all with Metric/Imperial/Both unit-system variants.

All four formats include trap-dipole fields (`trap_dipole_total_m`, `trap_dipole_leg_m`, `trap_dipole_total_ft`, `trap_dipole_leg_ft`).

Path validation: rejects absolute paths, `..` components, and unsafe destinations.

---

## TUI (`src/tui/mod.rs`)

**Dependencies:** `ratatui 0.29` + `crossterm 0.28`

**Entry point:** `pub fn run() -> Result<(), Box<dyn Error>>` (called by `src/bin/tui.rs`)

### Event loop

```
TuiState::new()
    └── initial run_calculation()
loop:
    terminal.draw(render)
    event::poll(200ms)
        Key → handle_key(key)
            → compute_action / dispatch(AppAction)
            → apply_action(AppState, action) → new AppState
    if quit: break
restore_terminal()
```

### Layout

```
┌─ title bar (1 line) ──────────────────────────────────────────┐
│ Configuration (38%)           │ Results (62%)                  │
│  ← → to change, ↑↓ to select │  scrollable, ↑↓/PgUp/PgDn     │
├─ hints bar (1 line) ──────────────────────────────────────────┤
```

Overlays (rendered on top via `Clear`):
- **Info popup** (`i` / `?`): version, author, GitHub URL, license
- **Band-checklist overlay** (Enter on Custom… band preset): per-band toggle checkboxes

### `TuiState` fields

```rust
struct TuiState {
    app: AppState,               // full app-layer state
    focus: Focus,                // Config | Results
    field_idx: usize,            // selected ConfigField
    band_preset_idx: usize,      // index into BAND_PRESETS
    vf_idx: usize,               // index into VF_PRESETS
    ratio_idx: usize,            // index into TRANSFORMER_RATIOS
    wire_min_idx: usize,         // index into WIRE_MIN_PRESETS
    wire_max_idx: usize,         // index into WIRE_MAX_PRESETS
    results_scroll: u16,
    show_info_popup: bool,
    show_band_checklist: bool,
    band_checklist_items: Vec<(usize, String, bool)>,  // (1-based idx, label, checked)
    band_checklist_cursor: usize,
    custom_band_indices: Vec<usize>,  // last confirmed custom selection
    quit: bool,
}
```

### Preset tables (cycling with ←/→)

| Field | Preset table |
|-------|-------------|
| Bands | `BAND_PRESETS`: 40m–10m, 80m–10m, 160m–10m, 20m–10m, Contest 80/40/20/15/10, **Custom…** (sentinel) |
| Velocity factor | `VF_PRESETS`: 0.50 → 1.00 (10 values) |
| Transformer | `TRANSFORMER_RATIOS`: 1:1 → 1:64 (10 values) |
| Wire min | `WIRE_MIN_PRESETS`: 5–20 m |
| Wire max | `WIRE_MAX_PRESETS`: 20–100 m |
| Mode | toggle Resonant ↔ Non-resonant |
| Antenna | cycle None → Dipole → Inverted-V → EFHW → Loop → OCFD → Trap Dipole |
| ITU Region | cycle Region1 → Region2 → Region3 |
| Units | cycle Both → Metric → Imperial |

### Keybindings

| Key | Action |
|-----|--------|
| `↑` / `k` | Select previous config field |
| `↓` / `j` | Select next config field |
| `←` / `h` | Decrease/previous value for selected field |
| `→` / `l` | Increase/next value for selected field |
| `r` | Run calculation |
| `Enter` | Run calculation (or open band checklist when on Custom… band preset) |
| `i` / `?` | Toggle info popup |
| `Tab` | Toggle focus Config ↔ Results |
| `q` / `Esc` | Quit (or close overlay if one is open) |
| `Ctrl-C` | Quit |
| `PgUp` / `PgDn` | Scroll results (Results panel focused) |

**Band-checklist overlay keys:**

| Key | Action |
|-----|--------|
| `↑`/`↓` / `j`/`k` | Move cursor |
| `Space` | Toggle band on/off |
| `Enter` | Confirm selection and close |
| `Esc` / `q` | Cancel and close |

### Custom band selection flow

1. Cycle Bands to "Custom…" (last `BAND_PRESETS` entry, empty slice sentinel).
2. Press `Enter` → `open_band_checklist()` builds items via `band_listing_view(region)`.
3. User toggles bands with `Space`, confirms with `Enter`.
4. Confirmed indices stored in `custom_band_indices`; `AppAction::SetBandIndices` dispatched.
5. Bands field shows "Custom (N bands)". Custom selection is remembered if user cycles away and back.

---

## Separation of Concerns

| Layer | Module | I/O |
|-------|--------|-----|
| Binary entry points | `main.rs`, `bin/tui.rs` | none |
| CLI/interactive UX | `cli.rs` | stdin/stdout/stderr |
| TUI UX | `tui/mod.rs` | terminal raw mode |
| Shared orchestration | `app.rs` | none |
| Domain data | `bands.rs` | none |
| RF algorithms | `calculations.rs` | none |
| File export | `export.rs` | filesystem only |

### Where changes land

| Change type | Touches |
|-------------|---------|
| New CLI flag | `cli.rs` |
| New TUI field | `tui/mod.rs` (preset table + `TuiState` + `all_field_values` + `compute_action`) |
| New antenna model | `app.rs` (enum, FromStr, transformer default, display views) + `calculations.rs` (formula + `WireCalculation` fields) + `cli.rs` (shadow enum + prompt) + `tui/mod.rs` (`MODELS` array + `all_field_values`) + `export.rs` (all format × unit-system sections) + `tests/cli_integration.rs` + docs |
| Band allocation change | `bands.rs` |
| Wire math / optimization | `calculations.rs` |
| Export format | `export.rs` |
| Display text for results | `app.rs` (`results_display_document`, `band_display_view`) |

---

## Testing Architecture

| Layer | Where |
|-------|-------|
| Unit tests | Inline `#[cfg(test)]` in each module |
| Integration tests | `tests/cli_integration.rs` — run binary via `Command` |
| Shell scenarios | `scripts/` — broader regression checks |

For operational details, see [testing.md](testing.md).