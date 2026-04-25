# NEC Calibration Workflow

This page describes how to calibrate Rusty Wire practical-model constants against NEC/reference sweeps.

Current focus:
- conductor-diameter correction used by the resonant length model

## Input Data Format

Use CSV with this exact header:

```csv
diameter_mm,length_factor
```

Definitions:
- diameter_mm: physical conductor diameter in millimeters
- length_factor: resonant length multiplier relative to the 2.0 mm baseline at the same setup
- blank lines and lines starting with `#` are ignored by the calibration script

Example:
- if a 4.0 mm conductor resonates at 99.2% of the 2.0 mm length, use 0.992

Template file:
- docs/data/nec_conductor_reference.csv

## Run Calibration

```bash
./scripts/calibrate-conductor-model.sh docs/data/nec_conductor_reference.csv
```

The script prints:
- fitted slope constant k for
  - F_d(d) = 1 - k * ln(d / d0)
  - d0 fixed at 2.0 mm
- RMSE and max absolute error
- suggested clamp bounds over your observed diameter span

Regression guard:

```bash
./scripts/test-nec-calibration.sh
```

This verifies the template fit constants and parser behavior before changing model constants.

Current template output (`docs/data/nec_conductor_reference.csv`):
- 7 data points at 0.5 mm intervals from 1.0 mm to 4.0 mm
- `k = 0.011542`
- `RMSE ≈ 0.000000` (model and template are consistent by construction)
- observed-span clamp suggestion: `0.992000 .. 1.008000`

## Apply Results

Update conductor correction constants in src/calculations.rs after reviewing fit quality.

Current implementation location:
- function conductor_diameter_correction_factor

Recommended process:
1. replace template CSV rows with NEC-derived points
2. run calibration script
3. update constants in src/calculations.rs
4. run cargo fmt --all, cargo check, cargo test
5. document calibrated constants in docs/math.md and docs/CHANGELOG.md

## Notes

- Keep all internal calculations metric-first.
- Use imperial only for output rendering.
- The script is intentionally non-destructive and does not edit source files automatically.
- `src/calculations.rs` currently uses the fitted template slope constant directly, but keeps broader runtime clamps (`0.97 .. 1.03`) than the template's observed span so the heuristic remains tolerant until real NEC sweep data replaces the placeholder CSV.
