---
project: rusty-wire
doc: docs/testing.md
status: living
last_updated: 2026-04-30
---

# Testing

Rusty Wire uses four test layers:

- Rust unit tests via `cargo test --lib` (calculations, CLI, TUI, app layer)
- Rust integration tests via `cargo test --test '*'` (binary behavior, export format contracts, corpus validation)
- `scripts/test-itu-region-bands.sh` for region-band regression checks
- `scripts/test-multi-optima.sh` for non-resonant multi-optima regression sweeps
- `scripts/test-nec-calibration.sh` for conductor-calibration regression checks

All of the above can be run together:

```bash
./scripts/test-all.sh
```

## Test Files

| File | Layer | What it covers |
|:-----|:------|:--------------|
| `src/calculations.rs` (mod tests) | unit | All `TransformerRatio`/`GroundClass` variants, skip-factor branches, antenna formulas, optimizer edge cases — 44 tests |
| `src/cli.rs` (mod tests) | unit | Interactive prompt I/O, export format selection, mode dispatch |
| `src/app.rs` (mod tests) | unit | App-layer calculation dispatch, velocity factor range |
| `src/tui/mod.rs` (mod tests) | unit | TUI key handling, band preset cycling, checklist overlay, scroll |
| `tests/cli_integration.rs` | integration | Real binary invocation: flags, exit codes, output content |
| `tests/export_format_contract.rs` | contract | PAR-001 v1 (CSV) and PAR-002 v1 (JSON) field order, precision, schema |
| `tests/corpus_validation.rs` | corpus | Golden reference cases against published standards (ITU-R P.368) |
| `tests/tolerance_helpers.rs` | helpers | Reusable tolerance-check utilities for corpus tests |

## Primary Command

```bash
cargo test
```

Useful variants:

```bash
cargo test -- --nocapture
cargo test cli_integration
cargo test --test export_format_contract
cargo test --test corpus_validation
```

## Integration Coverage

Integration tests live in `tests/cli_integration.rs` and validate real binary behavior, including:

- no-argument help output
- invalid input validation (for example velocity and mixed-unit window flags)
- region-aware band listing
- transformer recommendation resolution
- export path behavior for single and multi-format runs
- height and ground-class skip-distance scaling (monotonicity checks)

## Output Format Contract Tests

`tests/export_format_contract.rs` locks the PAR-001 v1 (CSV) and PAR-002 v1 (JSON) output contracts:
- Header field order and unit-specific columns
- Numeric precision (frequency: 3 decimal places, lengths: 2 decimal places)
- JSON array structure and field naming
- Backward compatibility: same inputs produce same results in both formats

These are CI-gated. A contract failure means a breaking format change has occurred without a version bump.

## Corpus Validation Tests

`tests/corpus_validation.rs` validates calculations against published reference standards:
- **Active**: `corpus_skip_distance_40m_itut_p368` — 40m skip distance vs ITU-R P.368-10 (2019)
- **Deferred (GAP-011)**: NEC-dependent cases (dipole, inverted-V, OCFD) pending NEC reference sweeps

See `docs/corpus-guide.md` and `corpus/` for adding new cases.

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

Full suite (format + compile + unit + all regression scripts):

```bash
./scripts/test-all.sh
```

Default pre-push verification (Rust only):

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

For the full release flow (including manual real-terminal screenshot refresh),
see `docs/release-checklist.md`.

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

When changing conductor-diameter correction constants:

```bash
cargo test
./scripts/test-nec-calibration.sh
```
