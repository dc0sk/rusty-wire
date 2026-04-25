# Testing

Rusty Wire uses three test layers:

- Rust tests via `cargo test` (unit + integration)
- `scripts/test-itu-region-bands.sh` for region-band regression checks
- `scripts/test-multi-optima.sh` for non-resonant multi-optima regression sweeps
- `scripts/test-nec-calibration.sh` for conductor-calibration regression checks

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

- no-argument invocation prints help
- mixed meter/feet constraints return a validation error
- invalid velocity values return a validation error
- `--list-bands --region 2` shows region-specific output
- recommended transformer defaults resolve correctly for non-resonant runs and EFHW mode
- multiple export formats ignore a custom `--output` path and use default names
- single-format export respects the requested `--output` path

These tests are intentionally high-level so that clap parsing and the real CLI flow are both covered.

## Future Validation Idea

- Add an advise-mode cross-tool validation script that calls `fnec-rust` for the top-ranked wire + transformer candidates.
- Record per-candidate sustainability status (for example: validated, warning, rejected) based on agreed efficiency/thermal thresholds.
- Use this as a regression guard when optimizer scoring weights or practical-limit coefficients are changed.

## Script: Multi-Optima Sweep

Run:

```bash
./scripts/test-multi-optima.sh
```

Purpose:

- builds the project
- sweeps band selections, velocity factors, and wire-length windows
- stops at the first case where multiple non-resonant optima are found

This is not a unit test. It is an empirical regression script used to confirm that the optimization logic still produces multiple-optima cases under realistic parameter sweeps.

Environment variables:

- `BIN`: path to the binary to execute, default `target/debug/rusty-wire`
- `SWEEP_OUT`: path to the temporary sweep output file, default `/tmp/rw_sweep_result.txt`

The sweep uses CLI band names/ranges (for example `40m`, `20m,17m`, `40m-10m`) to match the current `--bands` parser behavior.

## Script: ITU Region Band Regression

Run:

```bash
./scripts/test-itu-region-bands.sh
```

Checks listed ranges for Regions 1, 2, and 3.

## Script: NEC Calibration Regression

Run:

```bash
./scripts/test-nec-calibration.sh
```

Checks:

- template dataset fit stays at documented values (`k = 0.011542`, `RMSE = 0.000000`)
- parser accepts blank lines and `#` comment lines in calibration CSV input
- malformed rows are rejected with non-zero exit

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

On version bump (before tagging):

```bash
cargo sbom
cargo sbom-cdx
git add sbom/
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
