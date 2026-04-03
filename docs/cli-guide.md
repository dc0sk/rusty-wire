# Rusty Wire

**Version 1.0.0**

Rusty Wire is a Rust-based utility for wire-antenna planning across ham-radio and shortwave bands.

See [CHANGELOG.md](CHANGELOG.md) for the full release history.

It supports:
- Resonant wire length calculations (half-wave, full-wave, quarter-wave)
- Non-resonant common wire optimization across selected bands with multi-optima support
- Skip-distance summaries for selected bands
- Interactive and non-interactive (CLI) workflows
- Multiple export formats: CSV, JSON, Markdown, and plain text
- Unit system filtering: metric-only, imperial-only, or both
- Empirical test validation

## Features

- Band database with ham + shortwave bands
- Default band selection for quick use: 40m to 10m (band numbers: 4,5,6,7,8,9,10)
- Calculation mode selection:
  - Resonant (default)
  - Non-resonant
- Velocity factor input (default: 0.95)
- Non-resonant search constraints in either meters (default) or feet
- Multiple equally-optimal wire lengths displayed in ascending order
- Unit system awareness:
  - `--units m`: metric output only
  - `--units ft`: imperial output only
  - `--units both`: both systems (default when mixing unit inputs)
- Multiple export formats: CSV, JSON, Markdown, plain text
- Comma-separated export format selection: `--export csv,json,markdown,txt`
- Equivalent CLI command printed from interactive runs

## Interactive Mode

Run the binary without CLI flags:

```bash
rusty-wire
```

Interactive mode lets you:
- List all available bands
- Select one or multiple bands
- Choose calculation mode (default: resonant)
- Set velocity factor
- **Resonant mode**: display units prompt only (wire window uses defaults)
- **Non-resonant mode**: wire length window with units (m or ft), then display units
- Optionally export results to one or more formats (CSV, JSON, Markdown, TXT)

At the end of a run, Rusty Wire prints the equivalent CLI command for reproducibility.

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
- `--list-bands` — List all available bands
- `--bands <csv>` — Comma-separated band numbers (e.g., `6,10,40`)
- `--mode <resonant|non-resonant>` — Calculation mode (default: resonant)
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
- If no `--bands` are provided, Rusty Wire defaults to 40m-10m (`4,5,6,7,8,9,10`).

### Unit system and display options

- `--units <m|ft|both>` — Display units for output (auto-detected from input, default: both when mixing inputs)
  - `m` — Metric only (meters)
  - `ft` — Imperial only (feet)
  - `both` — Both metrics and imperial

### Export options

- `--export <format-list>` — Comma-separated export formats (e.g., `csv,json,markdown,txt`)
  - `csv` — Comma-separated values
  - `json` — JSON format
  - `markdown` — Markdown table format
  - `txt` — Plain text table format
- `--output <file>` — Output file path for exports (default: generates filename per format)

## Examples

### 1) Default run with defaults (resonant + default bands)

```bash
rusty-wire --velocity 0.95
```

### 2) Resonant calculation for selected bands

```bash
rusty-wire --mode resonant --bands 6,10 --velocity 0.90
```

### 3) Non-resonant optimization with metric constraints

```bash
rusty-wire --mode non-resonant --bands 6,10 --velocity 0.90 --wire-min 10 --wire-max 20
```

### 4) Non-resonant optimization with feet constraints

```bash
rusty-wire --mode non-resonant --bands 6,10 --velocity 0.90 --wire-min-ft 30 --wire-max-ft 90
```

### 5) Export to single format (CSV)

```bash
rusty-wire --mode resonant --bands 4,5,6,7,8,9,10 --export csv --output results.csv
```

### 6) Export to multiple formats simultaneously

```bash
rusty-wire --mode non-resonant --bands 6,10 --export csv,json,markdown --output results
```

This generates: `results.csv`, `results.json`, `results.md`

### 7) Metric-only output and export

```bash
rusty-wire --mode non-resonant --bands 6,10 --wire-min 10 --wire-max 20 --units m --export csv,json
```

### 8) Imperial-only output and export

```bash
rusty-wire --mode non-resonant --bands 6,10 --wire-min-ft 30 --wire-max-ft 60 --units ft --export markdown,txt
```

### 9) View multiple optima in non-resonant mode

```bash
rusty-wire --mode non-resonant --bands 2 --velocity 0.50 --wire-min 6 --wire-max 30
```

If multiple wire lengths are equally optimal, all will be displayed in ascending order:
```
Best non-resonant wire length for selected bands:
  15.00 m (49.21 ft), resonance clearance: 33.33%
  Additional equal optima in range (ascending):
     1. 15.00 m (49.21 ft, clearance: 33.33%)
     2. 25.00 m (82.02 ft, clearance: 20.00%)
```

## Output Summary

### Resonant mode includes:
- Per-band resonant lengths (with optional unit system filtering)
- Skip-distance summary
- **Optimum common wire length** (search window + clearance %)
- Multiple export format support (CSV, JSON, Markdown, plain text)

### Non-resonant mode includes:
- Band context overview
- Skip-distance summary
- **Best non-resonant wire length** with search window and resonance clearance
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
