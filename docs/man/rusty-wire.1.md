---
title: RUSTY-WIRE
section: 1
header: Rusty Wire Manual
footer: rusty-wire 2.16.0
date: 2026-04-30
---

# NAME

rusty-wire — wire antenna length calculator for amateur radio and shortwave

# SYNOPSIS

**rusty-wire** [*OPTIONS*]

# DESCRIPTION

**rusty-wire** computes resonant and non-resonant wire lengths for common HF
antenna types across amateur radio and shortwave bands. It accounts for
velocity factor, antenna height, ground class, conductor diameter, and ITU
region, and supports ranked candidate output (**\-\-advise**) with optional
NEC model cross-validation.

Both a non-interactive CLI mode and a full-screen TUI (**\-\-interactive**) are
available. Results can be exported as CSV, JSON, Markdown, plain text, YAML,
or HTML.

# OPTIONS

## General

**-h**, **\-\-help**
:   Print a short usage summary.

**-V**, **\-\-version**
:   Print the version string and exit.

**\-\-info**
:   Print project metadata (author, version, GitHub URL, license, platform).

**-i**, **\-\-interactive**
:   Launch the full-screen TUI. All options that apply in CLI mode can also
    be set interactively.

## Band and Region Selection

**-r**, **\-\-region** *1|2|3*
:   ITU region (default: **1**).
    1 = Europe / Africa / Middle East,
    2 = Americas,
    3 = Asia-Pacific.

**-b**, **\-\-bands** *LIST*
:   Comma-separated band names or ranges, e.g. **40m,20m,10m-15m**.
    Mutually exclusive with **\-\-bands-preset**.

**\-\-bands-preset** *NAME*
:   Named preset loaded from the TOML config file, e.g. **portable-dx**.
    Mutually exclusive with **\-\-bands**.

**\-\-bands-config** *PATH*
:   Override the preset config file path.  
    Auto-discovery order: **~/.config/rusty-wire/bands.toml**, then
    **./bands.toml**.

**\-\-list-bands**
:   List available bands for the selected region and exit.

## Frequency Input

**\-\-freq** *MHz*
:   Compute wire lengths for a single explicit frequency, bypassing band
    selection entirely. Range: 0 < f ≤ 1000.

**\-\-freq-list** *f1,f2,...*
:   Compute wire lengths for multiple explicit frequencies in a single run.
    Mutually exclusive with **\-\-freq**.

## Calculation Mode

**-m**, **\-\-mode** *resonant|non-resonant*
:   Calculation mode (default: **resonant**).

**\-\-wire-min** *METERS*
:   Non-resonant search window lower bound in metres.

**\-\-wire-max** *METERS*
:   Non-resonant search window upper bound in metres.

**\-\-wire-min-ft** *FEET*
:   Non-resonant search window lower bound in feet.

**\-\-wire-max-ft** *FEET*
:   Non-resonant search window upper bound in feet.

**\-\-step** *METERS*
:   Non-resonant search resolution (default: **0.05**).

## Antenna and RF Parameters

**\-\-antenna** *MODEL*
:   Restrict output to one antenna model.  
    Values: **dipole**, **inverted-v**, **efhw**, **loop**, **ocfd**,
    **trap-dipole**.

**-v**, **\-\-velocity** *VALUE*
:   Velocity factor, range 0.50–1.00 (default: **0.95**).

**\-\-velocity-sweep** *v1,v2,...*
:   Run at multiple velocity factors and print a side-by-side comparison.

**-t**, **\-\-transformer** *RATIO*
:   Transformer ratio (default: **recommended**).  
    Values: **recommended**, **1:1**, **1:2**, **1:4**, **1:5**, **1:6**,
    **1:9**, **1:16**, **1:49**, **1:56**, **1:64**.

**\-\-transformer-sweep** *r1,r2,...*
:   Run at multiple transformer ratios and print a side-by-side comparison.

**\-\-height** *7|10|12*
:   Antenna height in metres, used for height-aware skip estimates
    (default: **10**).

**\-\-ground** *poor|average|good*
:   Ground class for skip-distance scaling (default: **average**).

**\-\-conductor-mm** *VALUE*
:   Conductor diameter in millimetres, range 1.0–4.0 (default: **2.0**).
    Applies a first-order length correction.

## Advise Mode

**\-\-advise**
:   Print ranked wire length and balun/unun candidates with efficiency,
    mismatch loss, resonance clearance, and per-candidate tradeoff notes.

**\-\-validate-with-fnec**
:   Cross-validate each candidate with **fnec-rust** (if available in PATH).
    Prints pass/fail status alongside each candidate.

**\-\-fnec-pass-max-mismatch** *VALUE*
:   Mismatch factor at or below which a candidate is marked **passed**
    (range 0.0–1.0, default: **0.25**).

**\-\-fnec-reject-min-mismatch** *VALUE*
:   Mismatch factor at or above which a candidate is marked **rejected**
    (range 0.0–1.0, default: **0.60**).

**\-\-fnec-gate**
:   Remove **Rejected** candidates from advise output entirely. Requires
    **\-\-validate-with-fnec**.

## Output

**-u**, **\-\-units** *m|ft|both*
:   Display unit filter (default: both when mixing inputs).

**-e**, **\-\-export** *FORMATS*
:   Comma-separated export formats: **csv**, **json**, **markdown**, **txt**,
    **yaml**, **html**.

**-o**, **\-\-output** *PATH*
:   Base output file path for exports.

**\-\-quiet**
:   Suppress the full results table; print only the key recommendation.

**\-\-verbose**
:   Print the resolved run configuration before executing.

**\-\-dry-run**
:   Validate inputs and print the resolved configuration without calculating
    or exporting.

**\-\-save-prefs**
:   Save the current resolved settings as persistent user defaults to
    **~/.config/rusty-wire/config.toml**. The calculation still runs after
    saving.

# CONFIGURATION

## User Preferences

Persistent defaults are stored in **~/.config/rusty-wire/config.toml**. Use
**\-\-save-prefs** to write the current resolved settings.

## Custom Band Presets

Create **~/.config/rusty-wire/bands.toml** (or **./bands.toml**) to define
named presets and custom band entries:

```toml
[presets]
portable = ["40m", "20m", "15m", "10m"]
fieldday = ["80m", "40m", "20m", "15m", "10m"]

[[band]]
name = "60m-channel-1"
freq_low_mhz  = 5.3515
freq_high_mhz = 5.3665

[[band]]
name = "FT8-40m"
freq_low_mhz  = 7.074
freq_high_mhz = 7.076
freq_center_mhz = 7.074
```

Named presets are selectable with **\-\-bands-preset** or via the TUI.  
Custom **[[band]]** entries appear in the TUI band checklist and are included
when referenced by a preset.

# EXAMPLES

Resonant lengths for the default band set:

    rusty-wire

40 m and 20 m only, EFHW model, metric output:

    rusty-wire --bands 40m,20m --antenna efhw --units m

Non-resonant 10–30 m window, all bands, ranked advise candidates:

    rusty-wire --mode non-resonant --wire-min 10 --wire-max 30 --advise

Explicit FT8 frequencies, CSV export:

    rusty-wire --freq-list 7.074,14.074,21.074 --export csv

Velocity sweep for a 40 m dipole at three wire types:

    rusty-wire --bands 40m --antenna dipole --velocity-sweep 0.66,0.85,0.95

Interactive TUI with a custom band config:

    rusty-wire --interactive --bands-config ~/my-bands.toml

# TUI KEYBINDINGS

| Key           | Action                                      |
|---------------|---------------------------------------------|
| ↑ / k         | Select previous field                       |
| ↓ / j         | Select next field                           |
| ← / h         | Decrease value                              |
| → / l         | Increase value                              |
| r / Enter     | Run calculation                             |
| a             | Toggle advise panel                         |
| e             | Export CSV                                  |
| E             | Export JSON                                 |
| m             | Export Markdown                             |
| t             | Export plain text                           |
| i / ?         | Toggle info popup                           |
| Tab           | Toggle focus: config ↔ results              |
| PgUp / PgDn   | Scroll results panel                        |
| q / Esc       | Quit                                        |

# FILES

**~/.config/rusty-wire/config.toml**
:   Persistent user preferences (written by **\-\-save-prefs**).

**~/.config/rusty-wire/bands.toml**
:   User-defined band presets and custom band entries.

**./bands.toml**
:   Project-local band presets (fallback when the user config is absent).

# SEE ALSO

The full documentation is available at:  
<https://github.com/dc0sk/rusty-wire>

# AUTHOR

Simon Keimer (DC0SK)

# LICENSE

GPL-2.0-or-later
