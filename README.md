# Rusty Wire

**Version 2.2.0**

A Rust-based utility for wire-antenna planning across ham-radio and shortwave bands.

See [docs/CHANGELOG.md](docs/CHANGELOG.md) for the full release history.

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
- **Transformer recommendation mode**: `--transformer recommended` is the default and resolves from calculation mode plus antenna model
- **Multiple export formats**: CSV, JSON, Markdown, plain text
- **Unit system flexibility**: Metric-only, imperial-only, or both
- **ITU region support**: Region-aware amateur band ranges (default: Region 1)
- **Band database**: Pre-configured ham and shortwave bands

## Quick Start

Build:

```bash
cargo build --release
```

Show help:

```bash
./target/release/rusty-wire
```

Run interactive mode:

```bash
./target/release/rusty-wire --interactive
```

Run from Cargo during development:

```bash
cargo run -- [OPTIONS]
```

## Documentation

- CLI usage and examples: [docs/cli-guide.md](docs/cli-guide.md)
- Testing workflow: [docs/testing.md](docs/testing.md)
- Module design and execution flow: [docs/architecture.md](docs/architecture.md)
- Planned work: [docs/roadmap.md](docs/roadmap.md)
- Release history: [docs/CHANGELOG.md](docs/CHANGELOG.md)

## Testing

Primary check:

```bash
cargo test
```

Additional regression scripts:

```bash
./scripts/test-multi-optima.sh
./scripts/test-itu-region-bands.sh
```

For full details, see [docs/testing.md](docs/testing.md).

## SBOM

Rusty Wire supports Software Bill of Materials generation through Cargo.

Install the SBOM cargo subcommand (recommended/default):

```bash
cargo install cargo-sbom
```

Generate an SPDX SBOM (JSON 2.3) via Cargo:

```bash
cargo sbom
```

Generate CycloneDX JSON via Cargo alias:

```bash
cargo sbom-cdx
```

Or run the helper script:

```bash
./scripts/generate-sbom.sh
```

The helper script defaults to SPDX and also supports CycloneDX:

```bash
./scripts/generate-sbom.sh cyclonedx
```

Default tracked outputs are:

- `sbom/rusty-wire.spdx.json` (SPDX)
- `sbom/rusty-wire.cdx.json` (CycloneDX, when generated)

### Pre-push enforcement

This repository includes a pre-push hook at `.githooks/pre-push`.
Enable repository hooks with:

```bash
git config core.hooksPath .githooks
```

It runs:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- SPDX SBOM regeneration via `./scripts/generate-sbom.sh spdx`

SPDX generation is normalized for deterministic output (requires `jq` or `jaq`).
If `sbom/rusty-wire.spdx.json` changes during pre-push, the hook blocks push until the updated SBOM is committed.

## Examples

Resonant mode (default):
```bash
rusty-wire --bands 20m,10m,40m --velocity 0.95
```

Non-resonant optimization:
```bash
rusty-wire --mode non-resonant --bands 40m,20m,10m-15m --wire-min 10 --wire-max 35
```

Export to multiple formats:
```bash
rusty-wire --mode non-resonant --bands 20m,10m-15m --export csv,json,markdown --output results
```

Metric-only output:
```bash
rusty-wire --mode non-resonant --bands 80m --units m --wire-min 6 --wire-max 30
```

Filter to EFHW output only:
```bash
rusty-wire --mode resonant --bands 40m,20m --antenna efhw
```

Filter to inverted-V output only:
```bash
rusty-wire --mode resonant --bands 40m,20m --antenna inverted-v
```

Filter to OCFD output only:
```bash
rusty-wire --mode resonant --bands 40m,20m --antenna ocfd
```

For more examples, see [docs/cli-guide.md](docs/cli-guide.md).

## License

This project is licensed under the GNU General Public License, version 2 or later
(GPL-2.0-or-later).

See [LICENSE](LICENSE) for details.
