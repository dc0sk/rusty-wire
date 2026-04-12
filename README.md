# Rusty Wire

**Version 1.5.2**

A Rust-based utility for wire-antenna planning across ham-radio and shortwave bands.

See [docs/CHANGELOG.md](docs/CHANGELOG.md) for the full release history.

## Quick Start

> [!TIP]
> New to Rusty Wire? Start with interactive mode first:
>
> ```bash
> ./target/release/rusty-wire --interactive
> ```
>
> It walks you through region, bands, mode, antenna model, units, and export options step by step.

### Build from source

```bash
cargo build --release
```

### Show CLI help

```bash
./target/release/rusty-wire
```

### Run interactive mode

```bash
./target/release/rusty-wire --interactive
```

### Run from Cargo during development

```bash
cargo run -- [OPTIONS]
```

## Features

- **Resonant calculations**: Half-wave, full-wave, and quarter-wave dipole lengths
- **Derived antenna variants**: Also shows end-fed half-wave, full-wave loop, inverted-V dipole geometry, and OCFD segment dimensions
- **Antenna model filter**: Optional `--antenna dipole|inverted-v|efhw|loop|ocfd` to show one model at a time
- **Resonant point analysis**: Shows resonant harmonics within the active search window
- **Resonant shared compromises**: Shows closest combined compromise lengths to in-window resonant points
- **Non-resonant optimization**: Find the best single wire length for multiple bands
- **Non-resonant window optima**: Displays multiple local optimum candidates within the active search window
- **Equal-tie optima support**: Displays all equally-optimal wire lengths when ties occur
- **Velocity factor control**: Adjust for different wire types and insulation
- **Multiple export formats**: CSV, JSON, Markdown, plain text
- **Unit system flexibility**: Metric-only, imperial-only, or both
- **ITU region support**: Region-aware amateur band ranges (default: Region 1)
- **Band database**: Pre-configured ham and shortwave bands

## Documentation

For comprehensive CLI documentation and examples, see [docs/cli-guide.md](docs/cli-guide.md).
For test execution details, see [docs/testing.md](docs/testing.md).
For module layout and system design, see [docs/architecture.md](docs/architecture.md).
For planned future enhancements, see [docs/roadmap.md](docs/roadmap.md).

Key topics:
- CLI usage and all options
- Interactive mode via `--interactive`
- ITU region selection (`--region 1|2|3`)
- Resonant vs. non-resonant mode differences
- Wire search window (non-resonant only)
- Export format selection
- Unit system input/output control
- Multi-optima feature
- Testing with `cargo test` and the included shell scripts
- Project architecture and module boundaries
- Planned future enhancements and feature roadmap

## Testing

For the full testing guide, including `cargo test` and all bundled regression scripts, see [docs/testing.md](docs/testing.md).

Verify the multi-optima feature:

```bash
./scripts/test-multi-optima.sh
```

This script performs an exhaustive parameter sweep and exits on the first case where multiple optima are found.

Verify ITU region band ranges:

```bash
./scripts/test-itu-region-bands.sh
```

This script checks all listed band ranges for Regions 1, 2, and 3.

Interactive mode is available explicitly via:

```bash
cargo run -- --interactive
```

## SBOM

Rusty Wire supports Software Bill of Materials generation through Cargo.

Install the CycloneDX cargo subcommand once:

```bash
cargo install cargo-cyclonedx
```

Generate a JSON SBOM via Cargo alias:

```bash
cargo sbom
```

Or run the helper script:

```bash
./scripts/generate-sbom.sh
```

By default, output files are written under `target/cyclonedx/`.

## Architecture

For a full architectural overview, see [docs/architecture.md](docs/architecture.md).

## Examples

Resonant mode (default):
```bash
rusty-wire --bands 6,10,40 --velocity 0.95
```

Non-resonant optimization:
```bash
rusty-wire --mode non-resonant --bands 6,10,40 --wire-min 10 --wire-max 35
```

Export to multiple formats:
```bash
rusty-wire --mode non-resonant --bands 6,10 --export csv,json,markdown --output results
```

Metric-only output:
```bash
rusty-wire --mode non-resonant --bands 2 --units m --wire-min 6 --wire-max 30
```

Filter to EFHW output only:
```bash
rusty-wire --mode resonant --bands 4,6 --antenna efhw
```

Filter to inverted-V output only:
```bash
rusty-wire --mode resonant --bands 4,6 --antenna inverted-v
```

Filter to OCFD output only:
```bash
rusty-wire --mode resonant --bands 4,6 --antenna ocfd
```

For more examples, see [docs/cli-guide.md](docs/cli-guide.md).

## License

This project is licensed under the GNU General Public License, version 2 or later
(GPL-2.0-or-later).

See [LICENSE](LICENSE) for details.
