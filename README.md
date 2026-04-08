# Rusty Wire

**Version 1.1.0**

A Rust-based utility for wire-antenna planning across ham-radio and shortwave bands.

See [docs/CHANGELOG.md](docs/CHANGELOG.md) for the full release history.

## Quick Start

### Build from source

```bash
cargo build --release
```

### Run with defaults (interactive mode)

```bash
./target/release/rusty-wire
```

### Run from Cargo during development

```bash
cargo run -- [OPTIONS]
```

## Features

- **Resonant calculations**: Half-wave, full-wave, and quarter-wave dipole lengths
- **Non-resonant optimization**: Find the best single wire length for multiple bands
- **Multi-optima support**: Displays all equally-optimal wire lengths
- **Velocity factor control**: Adjust for different wire types and insulation
- **Multiple export formats**: CSV, JSON, Markdown, plain text
- **Unit system flexibility**: Metric-only, imperial-only, or both
- **Interactive and CLI modes**: Choose your workflow
- **ITU region support**: Region-aware amateur band ranges (default: Region 1)
- **Band database**: Pre-configured ham and shortwave bands

## Documentation

For comprehensive CLI documentation and examples, see [docs/cli-guide.md](docs/cli-guide.md).

Key topics:
- CLI usage and all options
- ITU region selection (`--region 1|2|3`)
- Resonant vs. non-resonant mode differences
- Wire search window (non-resonant only)
- Export format selection
- Unit system input/output control
- Multi-optima feature
- Testing with the included test script

## Testing

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

## Architecture

- **src/main.rs**: Program entry point and mode dispatch
- **src/cli.rs**: CLI interface, interactive prompts, display and export logic
- **src/calculations.rs**: Core physics calculations and optimization algorithms
- **src/bands.rs**: Region-aware band database and frequency ranges
- **scripts/test-multi-optima.sh**: Empirical validation test for multi-optima feature
- **scripts/test-itu-region-bands.sh**: Regression test for ITU region band ranges

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

For more examples, see [docs/cli-guide.md](docs/cli-guide.md).

## License

This project is licensed under the GNU General Public License, version 2 or later
(GPL-2.0-or-later).

See [LICENSE](LICENSE) for details.
