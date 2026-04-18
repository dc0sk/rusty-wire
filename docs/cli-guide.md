# CLI Guide

**Version 2.1.0**

Use this page as the command reference for Rusty Wire.

For test procedures, see [testing.md](testing.md).
For architecture details, see [architecture.md](architecture.md).
For release history, see [CHANGELOG.md](CHANGELOG.md).

## Basic Usage

```bash
rusty-wire [OPTIONS]
```

From source:

```bash
cargo run -- [OPTIONS]
```

Interactive mode:

```bash
rusty-wire --interactive
```

## Core Options

- `--help` Show help
- `--interactive` Start interactive mode
- `--list-bands` List bands for selected region
- `--region <1|2|3>` ITU region (default: `1`)
- `--bands <csv>` Band names/ranges, for example `40m,20m,10m-15m`
- `--mode <resonant|non-resonant>` Calculation mode (default: `resonant`)
- `--velocity <value>` Velocity factor, valid range `0.50..=1.00` (default: `0.95`)
- `--antenna <dipole|inverted-v|efhw|loop|ocfd>` Filter output to one model (omit to show all)
- `--transformer <recommended|1:1|1:2|1:4|1:5|1:6|1:9|1:16|1:49|1:56|1:64>`
- `--units <m|ft|both>` Output unit filter

## Non-Resonant Window Options

Only used with `--mode non-resonant`.

Metric:
- `--wire-min <meters>`
- `--wire-max <meters>`

Imperial:
- `--wire-min-ft <feet>`
- `--wire-max-ft <feet>`

Rules:
- Do not mix metric and imperial window flags in the same command.
- If `--bands` is omitted, Rusty Wire uses the built-in default band set.

## Transformer Recommendation Defaults

`--transformer recommended` is the default. Current policy:

- Resonant + no specific model: `1:1`
- Non-resonant + no specific model: `1:9`
- Dipole / inverted-v / loop: `1:1`
- EFHW: `1:56`
- OCFD: `1:4`

You can always override with an explicit ratio.

## Export Options

- `--export <csv,json,markdown,txt>` One or more formats
- `--output <file>` Output path (single format uses this name; multiple formats use per-format filenames)

Path safety:
- Absolute paths are rejected.
- Parent traversal with `..` is rejected.

## Region Notes

Modeled differences include:
- 80m: R1 `3.5-3.8`, R2 `3.5-4.0`, R3 `3.5-3.9`
- 40m: R1 `7.0-7.2`, R2 `7.0-7.3`, R3 `7.0-7.2`
- 60m: harmonized `5.3515-5.3665`

## Examples

Resonant run:

```bash
rusty-wire --mode resonant --bands 40m,20m --velocity 0.95
```

Non-resonant run with metric window:

```bash
rusty-wire --mode non-resonant --bands 40m,20m,10m --wire-min 10 --wire-max 35
```

Non-resonant run with feet window:

```bash
rusty-wire --mode non-resonant --bands 20m,15m --wire-min-ft 30 --wire-max-ft 90 --units ft
```

Antenna-specific resonant output:

```bash
rusty-wire --mode resonant --bands 40m,20m --antenna ocfd --transformer recommended
```

List regional bands:

```bash
rusty-wire --list-bands --region 2
```

Export multiple formats:

```bash
rusty-wire --mode non-resonant --bands 20m,10m-15m --export csv,json,markdown --output results
```

SBOM commands:

```bash
cargo sbom
cargo sbom-cdx
```
