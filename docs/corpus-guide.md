---
project: rusty-wire
doc: docs/corpus-guide.md
status: living
last_updated: 2026-04-30
---

# Corpus Guide

This guide explains how to create, validate, and maintain the golden corpus of antenna calculation reference cases in `corpus/`.

---

## Overview

The golden corpus is a version-controlled collection of input fixtures paired with reference outputs. It validates that Rusty Wire produces results within tolerance of established external references (primarily NEC-2/4 calculations and published ITU-R standards).

**Directory structure:**

```
corpus/
├── reference-results.json          # Machine-readable reference metadata and tolerances
├── <case-name>.input               # Input configuration for case
├── <case-name>.expected            # Expected output from reference source
├── <case-name>.notes               # Human notes on source and background
├── ...more cases...
└── README.md                       # This guide
```

---

## Anatomy of a Corpus Case

### `<case-name>.input`

A JSON or TOML configuration file that specifies the calculation parameters. Example:

```json
{
  "bands": ["40m"],
  "antenna": "dipole",
  "velocity_factor": 0.95,
  "mode": "resonant",
  "height_m": 10.0,
  "ground_class": "average",
  "conductor_mm": 2.0
}
```

**Naming convention:** Use descriptive names (e.g., `dipole_40m_resonant.input`, `ocfd_40m_20m_nec_sweep.input`). No spaces; use underscores or hyphens.

### `<case-name>.expected`

The reference output for this case. Format depends on the case:
- **Numeric results** (wire lengths, skip distance): plain text with one value per line or CSV format
- **Complex results** (multi-band, multi-antenna): CSV with headers

Example for resonant dipole:

```
frequency_mhz,band,resonant_length_m
7.0,40m,20.35
```

### `<case-name>.notes`

Human-readable documentation. Include:
- Reference source (tool name, version, commit)
- Date generated
- Any special configuration or assumptions
- Known limitations or deviations
- Background on why this case matters

Example:

```
Case: Resonant dipole 40m band
Source: EZNEC v7.0 (commit abc123), NEC-2 engine
Reference: NEC model: dipole at 10m height, free-space
Generated: 2026-04-30
Frequency: 7.0 MHz (40m band center)

Expected result: 20.35 m (resonant half-wave length)
Tolerance: ±1% relative (±0.2 m absolute)

Notes:
- Reference calculated with 0.95 velocity factor
- Free-space NEC (no ground effect modeled in reference sweep)
- First-order height correction will be applied in tolerance check
```

### `reference-results.json`

Machine-readable metadata for all corpus cases. Specifies the expected values and acceptance tolerances. Format:

```json
{
  "case_1_dipole_40m_resonant": {
    "source": "EZNEC v7.0 (NEC-2)",
    "source_url": "https://github.com/dc0sk/rusty-wire-reference/tree/nec-2023-04",
    "generated_date": "2026-04-30",
    "metrics": {
      "resonant_length_m": {
        "expected": 20.35,
        "rel_tol": 0.01,
        "abs_tol": 0.2
      }
    },
    "status": "active"
  },
  "case_2_ocfd_40m_20m_nec": {
    "source": "EZNEC v7.0 (NEC-2)",
    "source_url": "...",
    "generated_date": "2026-04-30",
    "metrics": {
      "resonant_length_m": {
        "expected": 40.12,
        "rel_tol": 0.02,
        "abs_tol": 0.5
      }
    },
    "status": "active"
  }
}
```

**Fields:**

| Field | Type | Required | Meaning |
|:------|:-----|:---------|:--------|
| `source` | string | yes | Tool name and version that generated the reference (e.g., "EZNEC v7.0 (NEC-2)") |
| `source_url` | string | no | URL to reference tool or dataset repository |
| `generated_date` | string | yes | ISO 8601 date (YYYY-MM-DD) when reference was generated |
| `metrics` | object | yes | Key-value pairs of metric name → {expected, rel_tol, abs_tol} |
| `status` | enum | yes | `active` \| `deferred` \| `experimental` |

**Metric fields:**

| Field | Type | Meaning |
|:------|:-----|:--------|
| `expected` | number | The reference value |
| `rel_tol` | number | Relative tolerance as a decimal (e.g., 0.01 for ±1%) |
| `abs_tol` | number | Absolute tolerance in metric units |

**Status values:**

- `active` — CI gate runs this case; failure blocks release
- `deferred` — Case is in corpus but excluded from CI; used for historical reference
- `experimental` — CI gate runs but failures are warnings, not blockers

---

## Adding a New Corpus Case

### 1. Generate or obtain reference output

Use an external tool (EZNEC, NEC-4, published tables) to compute the reference output:

```bash
# Example: EZNEC calculation
# Load model, set frequency, run, export results
```

### 2. Create case files

```bash
cd corpus/

# 1. Create input file
cat > dipole_40m_nec_reference.input << 'EOF'
{
  "bands": ["40m"],
  "antenna": "dipole",
  "velocity_factor": 0.95,
  "mode": "resonant"
}
EOF

# 2. Create expected output file
cat > dipole_40m_nec_reference.expected << 'EOF'
frequency_mhz,band,resonant_length_m
7.0,40m,20.35
EOF

# 3. Create notes file
cat > dipole_40m_nec_reference.notes << 'EOF'
Case: Resonant dipole 40m band
Source: EZNEC v7.0 (commit abc123)
Generated: 2026-04-30
Expected: 20.35 m
Tolerance: ±1% relative or ±0.2 m absolute
EOF
```

### 3. Add metadata to `reference-results.json`

```bash
# Edit reference-results.json and add:
{
  "dipole_40m_nec_reference": {
    "source": "EZNEC v7.0 (NEC-2)",
    "source_url": "https://github.com/dc0sk/rusty-wire-reference/tree/nec-2023-04",
    "generated_date": "2026-04-30",
    "metrics": {
      "resonant_length_m": {
        "expected": 20.35,
        "rel_tol": 0.01,
        "abs_tol": 0.2
      }
    },
    "status": "active"
  }
}
```

### 4. Create or update CI test

In `tests/corpus_validation.rs`, add a test that:
1. Loads the case from `reference-results.json`
2. Runs Rusty Wire with the input
3. Compares output against expected within tolerance
4. Reports pass/fail

Example (pseudocode):

```rust
#[test]
fn corpus_dipole_40m_nec_reference() {
    let case = CORPUS_METADATA["dipole_40m_nec_reference"].clone();
    
    // Run Rusty Wire
    let result = run_calculation(&case.input);
    let resonant_length = result.metrics["resonant_length_m"];
    
    // Check tolerance
    let expected = case.metrics["resonant_length_m"].expected;
    let rel_tol = case.metrics["resonant_length_m"].rel_tol;
    let abs_tol = case.metrics["resonant_length_m"].abs_tol;
    
    let diff = (resonant_length - expected).abs();
    let rel_diff = diff / expected.abs();
    
    assert!(
        rel_diff <= rel_tol || diff <= abs_tol,
        "Tolerance breach: {} (expected {}, tol: ±{}% or ±{})",
        resonant_length, expected, rel_tol * 100.0, abs_tol
    );
}
```

### 5. Verify and commit

```bash
# Run the new test
cargo test corpus_dipole_40m_nec_reference

# Commit as part of a PR with reference information
git add corpus/dipole_40m_nec_reference.*
git add corpus/reference-results.json
git add tests/corpus_validation.rs
git commit -m "corpus: add dipole 40m NEC reference case

Adds EZNEC v7.0 reference sweep for resonant dipole at 40m.
Expected: 20.35 m resonant length.
Tolerance: ±1% or ±0.2 m.
Reference: EZNEC v7.0 (NEC-2), commit abc123.
"
```

---

## Corpus Case Lifecycle

### Creating a Case: Best Practices

1. **Use authoritative sources only.** Reference outputs must come from respected external tools (NEC-2/4, EZNEC, published standards).
2. **Document the source rigorously.** Include tool version, engine/method, any special settings, and date.
3. **Start conservative with tolerances.** If unsure, begin with wider tolerances (e.g., ±2%) and tighten later if real-world data justifies it.
4. **Mark new cases as `experimental`** until they pass several release cycles without alarm; then promote to `active`.

### Updating a Case

If a case needs to change (e.g., tolerance adjustment, reference update):

1. **Do NOT edit the case files.** Instead, create a new case with a versioned name (`dipole_40m_nec_reference_v2.input`).
2. **Document the reason for the new version** in the `.notes` file.
3. **Deprecate the old case** by changing its status to `deferred` in `reference-results.json`.
4. **Update tests** to reference the new case.
5. **Commit with a clear message** explaining the rationale.

### Deferred Cases

Cases marked `deferred` in `reference-results.json`:
- Are not run by the CI gate (corpus-validation.yml)
- Remain in the repository for historical reference
- Can be re-activated later (e.g., when a blocker is resolved)

Example: An OCFD loop case that is deferred pending NEC sweep data:

```json
"ocfd_loop_40m_nec": {
  "source": "EZNEC v7.0 (NEC-4)",
  "generated_date": "2026-04-30",
  "metrics": { ... },
  "status": "deferred"
}
```

---

## CI Integration

### Corpus Validation Gate (`corpus-validation.yml`)

The CI gate (`ci/corpus-validation.yml`) runs before every PR and push to main:

1. Loads `reference-results.json`
2. For each case with `status: active`:
   - Runs Rusty Wire with the case's `.input` file
   - Compares results against `.expected` within tolerance bounds
   - Fails the build if any result falls outside tolerance
3. For cases with `status: experimental`, reports warnings but does not block merge
4. For cases with `status: deferred`, skips entirely

**How to run locally:**

```bash
cargo test --test corpus_validation
```

### Monitoring Tolerance Breaches

If a corpus test fails:

```
FAILED: corpus_dipole_40m_nec_reference
  Expected: 20.35 m (±1%)
  Got: 20.85 m
  Tolerance breach: 0.50 m (rel: 2.45%)
```

**Steps to investigate:**

1. **Check if the change is intentional.** Did you modify the calculation engine? Review the change against the corpus case notes.
2. **Run the case locally** with `--verbose` to see intermediate values.
3. **Update the tolerance** if the change is justified and real-world validated.
4. **Mark the case as experimental** if you're unsure (change status to `experimental`); tolerances can tighten after several release cycles.
5. **Open an issue** if the breach is unexpected; this is a quality signal.

---

## Reference Data Repository

Over time, it is valuable to maintain a separate reference repository with NEC sweeps, ITU-R propagation tables, and baseline tool outputs:

```
https://github.com/dc0sk/rusty-wire-reference/
├── nec-2-sweeps/
│   ├── dipole_40m_10m_height.nec
│   ├── dipole_40m_10m_height.output
│   └── ...
├── itut-propagation/
│   ├── p368-50pct-fieldstrength.csv
│   └── ...
└── LICENSE
```

Reference to this repository in case `.notes` files and `reference-results.json`:

```json
"dipole_40m_nec_reference": {
  "source": "EZNEC v7.0 (NEC-2)",
  "source_url": "https://github.com/dc0sk/rusty-wire-reference/blob/nec-2023-04/nec-2-sweeps/dipole_40m_10m_height.output",
  ...
}
```

---

## Current Corpus Status

**As of 2026-04-30:** Corpus structure and 6 active seed cases are in place (GAP-010 resolved). Two NEC-dependent cases remain deferred (GAP-011). See [docs/requirements.md](requirements.md) for gap status.

Initial seed cases to add (priority order):

1. **Resonant dipole, 40m band** (NEC-2) — reference antenna model
2. **Resonant inverted-V, 40m band** (NEC-2) — common variant
3. **Resonant OCFD, 40m/20m split** (NEC-4) — multi-band model
4. **Skip distance, 40m at 10m height** (ITU-R P.368) — propagation model
5. **Non-resonant compromise, 40/20/15m** (historical data) — multi-band optimization

---

