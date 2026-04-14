# Rusty Wire

**Version 2.1.0**

Rusty Wire is a Rust-based utility for wire-antenna planning across ham-radio and shortwave bands.

See [CHANGELOG.md](CHANGELOG.md) for the full release history.

It supports:
- Resonant wire length calculations (half-wave, full-wave, quarter-wave)
- Derived antenna outputs for end-fed half-wave, full-wave loop, inverted-V dipole geometry, and off-center-fed dipole layouts
- Non-resonant common wire optimization across selected bands with multi-optima support
- Skip-distance summaries for selected bands
- Interactive and non-interactive (CLI) workflows
- ITU region-aware amateur band handling (Region 1/2/3)
- Multiple export formats: CSV, JSON, Markdown, and plain text
- Unit system filtering: metric-only, imperial-only, or both

## Features

- Band database with ham + shortwave bands
- Default band selection for quick use: a built-in multi-band preset (shown in `--help` and used when `--bands` is omitted)
- Calculation mode selection:
  - Resonant (default)
  - Non-resonant
- Velocity factor input (default: 0.95)
- Additional resonant-model guidance:
  - End-fed half-wave total wire length
  - Full-wave loop circumference
  - Full-wave loop square-side estimate
  - OCFD leg split estimates (33/67 and 20/80)
- Non-resonant search constraints in either meters (default) or feet
- Multiple local optima displayed for the active non-resonant search window
- Multiple equally-optimal wire lengths displayed in ascending order when ties occur
- Unit system awareness:
  - `--units m`: metric output only
  - `--units ft`: imperial output only
  - `--units both`: both systems (default when mixing unit inputs)
- Multiple export formats: CSV, JSON, Markdown, plain text
- Comma-separated export format selection: `--export csv,json,markdown,txt`

## Interactive Mode

Interactive mode is available explicitly:

```bash
rusty-wire --interactive
```

It lets you:
- List all available bands
- Select one or multiple bands
- Choose calculation mode
- Set velocity factor
- Choose transformer ratio
- Configure non-resonant wire windows interactively
- Export results and print an equivalent CLI command

### Interactive Mode: Per-Session Defaults

Starting with the next release, interactive mode remembers your last-used values for each prompt (bands, calculation mode, antenna model, velocity factor, transformer ratio, wire window, and units) during your session. When you repeat a calculation, prompts will pre-fill with your previous choices, making iterative planning much faster.

- To accept the previous value, just press Enter at the prompt.
- To change a value, type a new one as usual.
- Defaults reset when you exit and restart the program.

This feature applies to both multi-band and quick single-band calculations in interactive mode.

## CLI Usage

```bash
rusty-wire [OPTIONS]
```

If running from source during development, use:

```bash
cargo run -- [OPTIONS]
```

### Core options

- `--help` — Display help message
- `--interactive` — Launch interactive menu mode
- `--list-bands` — List all available bands
- `--region <1|2|3>` — ITU region selection (default: `1`)
- `--bands <csv>` — Comma-separated band names and optional ranges (e.g., `40m,20m,10m-15m,60m-80m`)
- `--mode <resonant|non-resonant>` — Calculation mode (default: resonant)
- `--antenna <dipole|inverted-v|efhw|loop|ocfd>` — Filter output to a single antenna model (default: all models)
- `--transformer <recommended|1:1|1:2|1:4|1:5|1:6|1:9|1:16|1:49|1:56|1:64>` — Feed transformer selection (default: `recommended`)
- `--velocity <value>` — Velocity factor (0.0–1.0, default: 0.95)

### Non-resonant search window constraints

> Only relevant for `--mode non-resonant`.

Metric (default):
- `--wire-min <meters>` — Minimum wire length in meters
- `--wire-max <meters>` — Maximum wire length in meters

Feet (optional alternative):
- `--wire-min-ft <feet>` — Minimum wire length in feet
- `--wire-max-ft <feet>` — Maximum wire length in feet

Notes:
- Do not mix meter and feet constraints in the same command.
- If no `--bands` are provided, Rusty Wire uses the built-in default band set.
- Region-specific amateur band ranges are applied before calculation.

### ITU region behavior

- Region `1`: Europe, Africa, Middle East
- Region `2`: Americas
- Region `3`: Asia-Pacific

Example regional differences currently modeled:
- 80m: R1 `3.5-3.8`, R2 `3.5-4.0`, R3 `3.5-3.9`
- 40m: R1 `7.0-7.2`, R2 `7.0-7.3`, R3 `7.0-7.2`
- 60m: harmonized segment `5.3515-5.3665`

### Unit system and display options

- `--units <m|ft|both>` — Display units for output (auto-detected from input, default: both when mixing inputs)
  - `m` — Metric only (meters)
  - `ft` — Imperial only (feet)
  - `both` — Both metrics and imperial

### Antenna model selection

- If `--antenna` is omitted, per-band output includes dipole, end-fed half-wave, full-wave loop, inverted-V, and OCFD dimensions.
- If `--antenna dipole` is selected, output is filtered to dipole lengths.
- If `--antenna inverted-v` is selected, output is filtered to inverted-V total length, per-leg length, and estimated span at common apex angles.
- If `--antenna efhw` is selected, output is filtered to end-fed half-wave lengths.
- If `--antenna loop` is selected, output is filtered to full-wave loop dimensions.
- If `--antenna ocfd` is selected, output is filtered to off-center-fed dipole leg splits.
- Resonant point summary remains dipole-oriented and is shown for `dipole`, `inverted-v`, or when all models are shown.
- Compromise lengths are shown for all antenna selections; in `efhw`, `loop`, and `ocfd` modes they are labeled as tuner-assisted, dipole-derived guidance.
- In `ocfd` mode, each compromise line is a total wire length and now includes:
  - explicit 33/67 leg lengths
  - explicit 20/80 leg lengths
  - an optimized split ratio recommendation with worst-leg resonance-clearance percentage

### Transformer recommendations

- `--transformer recommended` is the default and resolves to a concrete ratio from the selected mode and antenna model.
- Current built-in recommendations are:
  - generic resonant mode: `1:1`
  - generic non-resonant mode: `1:9`
  - dipole, inverted-V, and loop: `1:1`
  - EFHW: `1:56`
  - OCFD: `1:4`
- You can always override the recommendation with an explicit ratio.
- The current recommendation logic is a fixed policy, not a transformer optimization pass.

### Export options

- `--export <format-list>` — Comma-separated export formats (e.g., `csv,json,markdown,txt`)
  - `csv` — Comma-separated values
  - `json` — JSON format
  - `markdown` — Markdown table format
  - `txt` — Plain text table format
- `--output <file>` — Output file path for exports (default: generates filename per format). Only relative file paths are accepted; absolute paths and parent-directory references (`..`) are rejected for safety.

Exports also include resonant points within the active search window for each selected band.
In resonant mode, non-resonant recommendation payloads are omitted from exports.

### SBOM generation

Rusty Wire supports SBOM generation using Cargo with both SPDX and CycloneDX output.

SPDX is the default/recommended format.

Install SBOM support:

```bash
cargo install cargo-sbom
```

Generate SPDX via the repository alias:

```bash
cargo sbom
```

Generate CycloneDX JSON:

```bash
cargo sbom-cdx
```

Or use the helper script:

```bash
./scripts/generate-sbom.sh
```

Use CycloneDX from the helper script:

```bash
./scripts/generate-sbom.sh cyclonedx
```

Default tracked outputs are `sbom/rusty-wire.spdx.json` and `sbom/rusty-wire.cdx.json`.

Pre-push workflow in this repository regenerates `sbom/rusty-wire.spdx.json` with deterministic normalization (requires `jq` or `jaq`) and blocks push until SBOM updates are committed.

## Examples

### 1) Default run with defaults (resonant + default bands)

```bash
rusty-wire --velocity 0.95
```

### 2) Resonant calculation for selected bands

```bash
rusty-wire --mode resonant --bands 20m,10m --velocity 0.90
```

### 2a) Region-specific listing and calculation

```bash
rusty-wire --list-bands --region 1
rusty-wire --region 2 --mode resonant --bands 40m --velocity 0.95
```

### 3) Non-resonant optimization with metric constraints

```bash
rusty-wire --mode non-resonant --bands 20m,10m --velocity 0.90 --wire-min 10 --wire-max 20
```

### 4) Non-resonant optimization with feet constraints

```bash
rusty-wire --mode non-resonant --bands 20m,10m --velocity 0.90 --wire-min-ft 30 --wire-max-ft 90
```

### 5) Export to single format (CSV)

```bash
rusty-wire --mode resonant --bands 40m-10m --export csv --output results.csv
```

### 6) Export to multiple formats simultaneously

```bash
rusty-wire --mode non-resonant --bands 20m,10m-15m --export csv,json,markdown --output results
```

This generates: `results.csv`, `results.json`, `results.md`

### 7) Metric-only output and export

```bash
rusty-wire --mode non-resonant --bands 20m,10m --wire-min 10 --wire-max 20 --units m --export csv,json
```

### 8) Imperial-only output and export

```bash
rusty-wire --mode non-resonant --bands 20m,10m --wire-min-ft 30 --wire-max-ft 60 --units ft --export markdown,txt
```

### 9) View multiple optima in non-resonant mode

```bash
rusty-wire --mode non-resonant --bands 80m --velocity 0.50 --wire-min 6 --wire-max 30
```

Non-resonant mode displays local optima for the active search window and, when present, equal-tie optima:
```
Best non-resonant wire length for selected bands:
  15.00 m (49.21 ft), resonance clearance: 33.33%
  Local optima in search window (ascending):
     1. 10.35 m (33.96 ft, clearance: 3.95%)
     2. 15.00 m (49.21 ft, clearance: 33.33%, recommended)
     3. 19.65 m (64.47 ft, clearance: 18.32%)
  Additional equal optima in range (ascending):
     1. 15.00 m (49.21 ft, clearance: 33.33%)
     2. 25.00 m (82.02 ft, clearance: 20.00%)
```

## Output Summary

### Resonant mode includes:
- Per-band resonant lengths (with optional unit system filtering)
- Skip-distance summary
- **Resonant points within the active search window** (quarter-wave harmonics for selected bands)
- **Closest combined compromises to resonant points** (multiple near-best shared lengths across selected bands)
- Multiple export format support (CSV, JSON, Markdown, plain text)

### Non-resonant mode includes:
- Band context overview
- Skip-distance summary
- **Best non-resonant wire length** with search window and resonance clearance
- **Local optima in the active search window** in ascending order
- **Multiple equally-optimal wire lengths** in ascending order (if ties exist)
- Multiple export format support (CSV, JSON, Markdown, plain text)

## Testing

### Running the multi-optima test script

Rusty Wire includes a comprehensive test script to verify that the multi-optima feature works correctly:

```bash
./scripts/test-multi-optima.sh
```

This script:
- Builds the project
- Performs an exhaustive parameter sweep across:
  - Band combinations (1–10 + multi-band selections)
  - Velocity factors (0.50–1.00 in 0.05 steps)
  - Wire length windows (various metric ranges)
- Exits on first non-resonant calculation that produces multiple optima
- Prints the discovered case and example output
- Returns exit code 0 on success, 1 if no multi-optima found

**Environment variables:**
- `BIN` — Path to the compiled binary (default: `target/debug/rusty-wire`)
- `SWEEP_OUT` — Path for sweep results file (default: `/tmp/sweep_out.txt`)

**Example output:**
```
FOUND_MULTIPLE
bands=2 vf=0.50 min=6 max=30
Best non-resonant wire length for selected bands:
  15.00 m (49.21 ft), resonance clearance: 33.33%
  Additional equal optima in range (ascending):
     1. 15.00 m (49.21 ft, clearance: 33.33%)
     2. 25.00 m (82.02 ft, clearance: 20.00%)

PASS: multi-optima behavior is reachable.
```

### Running the ITU region regression script

```bash
./scripts/test-itu-region-bands.sh
```

This script:
- Builds the project
- Runs `--list-bands` for Regions 1, 2, and 3
- Verifies all listed bands and ranges against expected values
- Returns exit code 0 on success, non-zero on mismatch
