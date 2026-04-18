# Testing

Rusty Wire uses three test layers:

- Rust tests via `cargo test` (unit + integration)
- `scripts/test-itu-region-bands.sh` for region-band regression checks
- `scripts/test-multi-optima.sh` for non-resonant multi-optima regression sweeps

## Primary Command

```bash
cargo test
```

Useful variants:

```bash
cargo test -- --nocapture
cargo test cli_integration
```

## Integration Coverage

Integration tests live in `tests/cli_integration.rs` and validate real binary behavior, including:

- no-argument help output
- invalid input validation (for example velocity and mixed-unit window flags)
- region-aware band listing
- transformer recommendation resolution
- export path behavior for single and multi-format runs

## Regression Scripts

### ITU region bands

```bash
./scripts/test-itu-region-bands.sh
```

Checks listed ranges for Regions 1, 2, and 3.

### Multi-optima sweep

```bash
./scripts/test-multi-optima.sh
```

Builds and sweeps parameter combinations, then exits at the first confirmed multi-optima case.

Environment variables for the sweep script:
- `BIN` default: `target/debug/rusty-wire`
- `SWEEP_OUT` default: `/tmp/sweep_out.txt`

## Recommended Workflow

Default pre-push verification:

```bash
cargo fmt
cargo check
cargo test
```

When changing region or band behavior:

```bash
cargo test
./scripts/test-itu-region-bands.sh
```

When changing non-resonant optimization behavior:

```bash
cargo test
./scripts/test-multi-optima.sh
```
