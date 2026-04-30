---
project: rusty-wire
doc: docs/requirements.md
status: living
last_updated: 2026-04-30
---

# Requirements Engineering

This document defines the scope decisions, functional requirements, non-functional requirements, compatibility contracts, and open gaps for the Rusty Wire project.

---

## Scope Decisions (DEC-NNN)

**DEC-001**: Rusty Wire is a RF (radio frequency) wire antenna calculator, not a general-purpose antenna simulator.
Rationale: The project is focused on practical dipole-like antenna design for ham radio. While the codebase can be extended to other antenna types, full electromagnetic simulation (NEC, MMANA, CST) is out of scope.

**DEC-002**: The project shall support multiple frontends (CLI, TUI, GUI) sharing a single I/O-free application layer (`src/app.rs`).
Rationale: Code reuse across frontends reduces maintenance burden and ensures consistent behavior across all user interfaces. The app layer is responsible for business logic; frontends are wiring only.

**DEC-003**: CLI is the reference implementation. TUI and GUI must demonstrate feature parity with the CLI, not extend beyond it.
Rationale: CLI is available on all platforms and is the most portable interface. Feature completeness is verified against CLI behavior.

**DEC-004**: All numeric outputs are subject to documented tolerance bounds (relative and absolute). Tolerance changes are breaking changes.
Rationale: Users rely on deterministic results within acceptable precision. Silent precision drift is a serious bug.

**DEC-005**: The project targets both traditional (meters) and Imperial (feet) unit systems with explicit, user-selectable output.
Rationale: Ham radio communities use both systems depending on region. Ambiguous default units cause real-world errors.

**DEC-006**: External baselines (NEC-2/4, published propagation tables, ITU-R standards) are the source of truth for antenna behavior. Divergence from baselines is tracked as a parity gap, not as a bug in the baseline.
Rationale: The project is a practical tool that conforms to existing standards, not a research platform that re-implements them.

---

## Functional Requirements (FR-NNN)

### Core Calculation and Selection

**FR-001**: The system shall calculate resonant and non-resonant dipole wire lengths for a given frequency.
Acceptance: Given a frequency and antenna model, the system returns a wire length (m) or length range (m), with optional transformer recommendations.
Phase: 1 (completed in v1.0)

**FR-002**: The system shall support band selection by ITU region and band name (e.g., "40m", "20m-15m" range).
Acceptance: User provides region (1, 2, or 3) and band names (comma-separated, supporting ranges). System returns all frequencies in the selected bands.
Phase: 1 (completed in v2.3)

**FR-003**: The system shall support six antenna models: dipole, inverted-V, EFHW, loop, OCFD, trap-dipole.
Acceptance: User specifies `--antenna <model>`. For each model, the system applies model-specific adjustments (e.g., OCFD splits into multiple wire sections).
Phase: 1–2 (dipole–OCFD completed v2.0+; trap-dipole v2.5+)

**FR-004**: The system shall support explicit frequency input (`--freq <MHz>` or `--freq-list <f1,f2,...>`).
Acceptance: User provides one or more frequencies explicitly. Band selection is bypassed. System calculates for all provided frequencies.
Phase: 2 (completed; GAP-002 resolved 2026-04-30)

**FR-005**: The system shall support user-defined band presets via a config file (`bands.toml`).
Acceptance: User provides `--config bands.toml` or file is auto-loaded from standard path. System loads custom bands and merges with ITU presets.
Phase: 2 (completed; GAP-001 resolved 2026-04-30)

**FR-006**: The system shall provide balun/unun transformer recommendations based on antenna model and frequency band.
Acceptance: Recommended transformers are flagged and explained (e.g., "1:9 unun recommended for EFHW"). Alternatives are shown if requested.
Phase: 1–2 (basic recommendations v2.0+; optimizer-driven ranking v2.6+)

**FR-007**: The system shall calculate antenna skip distance (ground-wave range) with adjustments for antenna height and ground class.
Acceptance: User specifies height and ground class. System scales skip distance estimates accordingly. Must align with ITU-R P.368 propagation model within tolerance (see §Tolerance Matrix).
Phase: 2 (completed v2.5 with height presets and ground-class scaling)

### Output and Export

**FR-008**: The system shall output results in multiple formats: human-readable terminal, CSV, JSON, Markdown, plain text.
Acceptance: User provides `--export <format>`. Output is written to stdout or specified file. Format is valid and parseable.
Phase: 1–2 (completed v2.3+)

**FR-009**: The system shall provide an interactive mode where the user selects parameters via prompts and can refine results iteratively.
Acceptance: User runs `--interactive` or `-i`. System presents prompts for band, antenna, mode, units, and other options. Results are shown after each parameter change.
Phase: 1–2 (completed v2.3)

**FR-010**: The system shall provide a TUI (Text User Interface) using `ratatui` with feature parity to CLI/interactive mode.
Acceptance: User runs `tui` binary or `rusty-wire --tui`. TUI provides keyboard-driven band selection, antenna choice, mode selection, and export. All CLI features are accessible.
Phase: 2 (in progress)

**FR-011**: The system shall support scriptability: stable stdout/stderr separation, machine-readable output, mode identification in headers.
Acceptance: Stdout contains results (or empty on quiet mode). Stderr contains diagnostics and warnings. CSV/JSON have stable field order and names across versions. Exit code indicates success/failure.
Phase: 1–2 (completed for CSV/JSON v2.3+; tested in cli_integration.rs)

### Validation and Error Handling

**FR-012**: The system shall validate all user input (frequency range, velocity factor, antenna type, units, etc.) and provide actionable error messages.
Acceptance: Invalid input results in an error message that names the invalid parameter, the provided value, and acceptable alternatives. Exit code is 1.
Phase: 1–2 (completed v2.3+; contract tested in cli_integration.rs)

**FR-013**: The system shall provide informational output (`--info`) listing supported antenna models, bands, transformer types, and project metadata.
Acceptance: `--info` outputs metadata (version, license, supported models, region summary). Exit code is 0.
Phase: 1–2 (completed v2.0+)

---

## Non-Functional Requirements (NFR-NNN)

**NFR-001**: Calculation performance.
Metric: Time to compute wire length for a single band and antenna model.
Threshold: < 10 ms on a modern CPU (2020+). This is verified in ci/performance.yml (advisory).
Phase: 1 (baseline met in v1.x)

**NFR-001a**: Pre-commit hook latency.
Metric: Time to run pre-commit checks (format + lint + unit tests) before allowing a commit.
Threshold: < 30 seconds on a modern CPU. Goal is to provide fast feedback without blocking developer workflow.
Phase: 2 (implemented v2.7+)

**NFR-002**: CLI startup time.
Metric: Time from invocation to first output (or prompt).
Threshold: < 100 ms on a modern CPU. Verified locally with `time rusty-wire --info`.
Phase: 1–2 (baseline met)

**NFR-003**: Code maintainability and test coverage.
Metric: Cyclomatic complexity per function; lines of untested code.
Threshold: No function > 15 cyclomatic complexity. Core calculation functions (calculations.rs) ≥ 90% line coverage.
Phase: 1–2 (calculations.rs: 44 unit tests, up from 23; GAP-004 resolved 2026-04-30)

**NFR-004**: Dependency footprint.
Metric: Number of direct dependencies; size of compiled binary.
Threshold: Binary size < 10 MB (stripped, release build). Direct deps: < 20 for core library.
Phase: 1–2 (v2.5.0: ~18 direct deps; ~6 MB binary)

**NFR-005**: Documentation completeness.
Metric: Coverage of user-facing features in docs/ and README.md.
Threshold: Every CLI flag and interactive option has a documented purpose and example. Architecture diagram is current within each phase.
Phase: 1–2 (living document)

**NFR-006**: Accessibility (TUI).
Metric: Keyboard navigation completeness; screen-reader compatibility expectations.
Threshold: All TUI functions navigable with keyboard alone. Tab, Enter, arrow keys cover all features. No mouse required for core workflows.
Phase: 2 (implemented; GAP-005 resolved 2026-04-30)

---

## Compatibility Requirements (COMP-NNN)

### Numeric Tolerance Matrix

Wire-length calculations and propagation estimates are subject to documented tolerance bounds. These bounds are enforced by golden corpus tests (ci/corpus-validation.yml) and are the source of truth for acceptable numeric accuracy.

| Metric | Model | Relative Tolerance | Absolute Tolerance | Reference | Phase |
|:-------|:------|:------------------|:------------------|:----------|:------|
| **Resonant length** | Dipole | ≤ 1.0% | ≤ 0.10 m | EZNEC NEC-2 | deferred (GAP-011) |
| **Resonant length** | Inverted-V | ≤ 1.5% | ≤ 0.15 m | EZNEC NEC-2 | deferred (GAP-011) |
| **Resonant length** | EFHW | ≤ 2.0% | ≤ 0.20 m | EZNEC NEC-2 | deferred (GAP-011) |
| **Resonant length** | Loop | ≤ 2.0% | ≤ 0.20 m | EZNEC NEC-2 | deferred (GAP-006, GAP-011) |
| **Resonant length** | OCFD | ≤ 1.5% | ≤ 0.15 m | EZNEC NEC-2 | deferred (GAP-011) |
| **Resonant length** | Trap dipole | ≤ 2.0% | ≤ 0.20 m | EZNEC NEC-2 | deferred (GAP-006, GAP-011) |
| **Non-resonant recommend** | All | ≤ 2.0% | ≤ 0.20 m | Historical data | 1 |
| **Skip distance (ground wave)** | All | ≤ 5.0% | ≤ 5 km | ITU-R P.368 | 2 |
| **Height-aware skip scaling** | All | ≤ 10% | ≤ 2 km | Empirical first-order | 2 |
| **Ground-class skip scaling** | All | ≤ 10% | ≤ 2 km | Empirical first-order | 2 |
| **Conductor correction** | All | ≤ 2.0% | ≤ 0.15 m | NEC template fit | deferred (GAP-011) |

**Rules:**
1. Every tolerance uses the **wider** bound (relative OR absolute)
2. Deferred tolerances are not CI-gated; reference sweeps must be obtained first
3. Tolerance changes are **breaking changes** (require version bump from x.y.z → (x+1).0.0)
4. Each tolerance row has a dedicated corpus test (e.g., corpus/dipole_40m_nec_reference.*)
5. If implementation produces a result outside tolerance bounds, CI fails

**Tolerance Justification:**
- **≤1% (tight)**: Resonant dipole is the reference model; high precision expected
- **≤2%**: Standard wire models; reasonable engineering tolerance
- **≤5–10%**: Propagation and first-order correction factors; empirical models have wider bounds

---

## Compatibility Requirements (COMP-NNN)

**COMP-001**: Wire-length calculations shall match NEC-based reference results (EZNEC) within tolerance.
Tolerance: ≤ 1% relative or ≤ 0.1 m absolute (whichever is wider) for resonant length at center band frequency. Non-resonant results: ≤ 2% relative or ≤ 0.2 m absolute.
Reference: EZNEC v7.0 (NEC-2 engine) / fnec-rust (Hallén solver).
Status: **Partial** — Baseline dipole (free space) NEC reference and CI-gated test now active (as of 2026-04-30). Ground variants, height-aware, inverted-V/EFHW/conductor-correction remain Phase 3. See [docs/nec-requirements.md](nec-requirements.md) for full scope and Phase 3 plan.
Phase: 2–3 (baseline active; comprehensive deferred)

**COMP-002**: Skip distance estimates shall match ITU-R P.368 propagation model within tolerance.
Tolerance: ≤ 5% relative or ≤ 5 km absolute (whichever is wider).
Reference: ITU-R P.368-10 (2019); Section 3 (50% field strength propagation).
Status: Active for height 5–30 m, ground class poor/average/good, frequencies 2–30 MHz.
Phase: 2 (implemented v2.5; partial tolerance verification GAP-007)

**COMP-003**: Output format (CSV, JSON) shall remain backward-compatible across minor versions.
Tolerance: New fields may be added (additive); existing field names, order, and semantics must not change. Breaking format changes require a new versioned contract (PAR-NNN v2).
Reference: PAR-001 v1 (locked in v2.0); current schema in docs/output-formats.md.
Status: Active; enforced by ci/report-format.yml.
Phase: 1–2 (completed)

---

## Versioned Output Contracts (PAR-NNN vN)

**PAR-001 v1**: CSV export format (locked in v2.0).
Version: 1
Status: locked
Fields:
```
frequency_mhz,band,antenna_model,resonant_length_m,non_resonant_min_m,non_resonant_max_m,
recommended_transformer,skip_distance_km,velocity_factor,height_m,ground_class,export_timestamp
```
Breaking change policy: Renaming any field or changing the order is a breaking change (requires PAR-001 v2). Adding new columns at the end is additive; no version bump.
Deprecated fields: None yet (v1 is current).
Verified by: tests/export_format_contract.rs (PAR-001 v1 CSV contract tests, implemented 2026-04-30).

**PAR-002 v1**: JSON export format (locked in v2.3).
Version: 1
Status: locked
Schema: array of objects per calculation with fields matching CSV (snake_case field names).
Breaking change policy: Same as PAR-001.
Verified by: tests/export_format_contract.rs (PAR-002 v1 JSON contract tests, implemented 2026-04-30).

---

## Gap List (GAP-NNN)

**GAP-001**: User-defined band presets (`bands.toml`) not yet implemented.
Status: **resolved** (2026-04-30)
Target phase: 2
Owner: unassigned
Resolution: Implemented in v2.7.0. `src/band_presets.rs` loads presets from `~/.config/rusty-wire/bands.toml` (XDG) or `./bands.toml` (cwd). `--bands-preset <name>` and `--bands-config <path>` flags wired through CLI. TUI discovers the same file at startup.
Notes: Affects FR-005. Resolved.

**GAP-002**: Frequency-list input (`--freq-list f1,f2,...`) partially implemented; needs full integration.
Status: **resolved** (2026-04-30)
Target phase: 2
Owner: unassigned
Resolution: `--freq-list` flag fully implemented and integration-tested in v2.7.0. Mutual exclusion with `--freq` enforced with clear error. Six integration tests in tests/cli_integration.rs (multi-frequency, single-entry, mutual exclusion, zero-value, over-limit, quiet non-resonant compact output).
Notes: Affects FR-004. Resolved.

**GAP-003**: Balun optimizer (candidate ranking mode) not yet implemented.
Status: **resolved** (2026-04-30)
Target phase: 2
Owner: unassigned
Resolution: `--advise` flag implemented in CLI (`src/cli.rs::print_advise_candidates`). App-layer `build_advise_candidates` and `optimize_transformer_candidates` in `src/app.rs`. TUI `a` key toggles advise panel (`toggle_advise_panel`). 7+ integration tests in `tests/cli_integration.rs` for EFHW/preset/markdown/JSON/fnec-validation/threshold-error paths. Unit test `build_advise_candidates_returns_ranked_wire_and_ratio_matches` in `src/app.rs`.
Notes: Affects FR-006. Resolved.

**GAP-004**: Test coverage for calculations.rs below target (currently ~85%, target 90%).
Status: **resolved** (2026-04-30)
Target phase: 2
Owner: unassigned
Resolution: Added 21 new unit tests covering GroundClass::as_label, all TransformerRatio variants, TransformerRatio::from_str error path, height_skip_factor fallback/clamp branches, ground_skip_factor Poor/Good paths, WireCalculation Display, optimize_ocfd_split_for_length edge cases, zero-step invalid configs, and nearest_resonance_clearance_pct edge cases. Calculations module now has 44 unit tests (up from 23).
Notes: Affects NFR-003. Coverage tooling (cargo-tarpaulin) failed to compile on this machine; estimate improved from ~85% toward ≥90%.

**GAP-005**: TUI accessibility (keyboard navigation) not yet defined.
Status: **resolved** (2026-04-30)
Target phase: 2 or later
Owner: unassigned
Resolution: Full keyboard navigation implemented in `src/tui/mod.rs::handle_key`. Tab toggles focus between config and results. ↑↓/jk navigate fields; ←→/hl change values; Enter runs calculation; Space toggles band checklist; PgUp/PgDn scroll results; r/a/e/E/m/t shortcuts for run/advise/export. All features reachable without mouse. 30+ TUI unit tests in `src/tui/mod.rs` covering key dispatch, focus toggle, scroll, checklist, advise toggle, and export.
Notes: Affects NFR-006. Resolved.

**GAP-006**: NEC-based validation corpus for loop and trap-dipole models.
Status: deferred
Target phase: 3
Owner: unassigned
Resolution: —
Notes: Affects COMP-001. Requires reference sweeps from EZNEC/NEC-4. Deferred pending adoption of these models in production use.

**GAP-007**: ITU-R P.368 tolerance verification incomplete.
Status: **resolved** (2026-04-30)
Target phase: 2
Owner: unassigned
Resolution: Five corpus tests active in `tests/corpus_validation.rs`: baseline (h=10m, average), height-scaled (7m → factor 0.78, 12m → factor 1.12), and ground-class-scaled (poor → factor 0.88, good → factor 1.10). Each test validates factor application within ±10% / ±2 km. All monotonicity constraints verified (poor < average < good, 7m < 10m < 12m).
Notes: Affects COMP-002. Resolved. NEC-dependent COMP-001 tolerances remain deferred (GAP-011).

**GAP-008**: Output format contract tests (PAR-001 v1, PAR-002 v1) not yet implemented.
Status: **resolved** (2026-04-30)
Target phase: 2
Owner: unassigned
Resolution: tests/export_format_contract.rs created with 16+ tests validating field order, precision, and schema stability for both CSV and JSON formats.
Notes: Affects COMP-003. Resolved.

**GAP-009**: Interactive-mode prompt testability incomplete.
Status: **resolved** (2026-04-30)
Target phase: 2
Owner: unassigned
Resolution: All interactive prompts use injected I/O (`&mut dyn Read` / `&mut dyn Write`) and are unit-tested. 50+ tests in src/cli.rs mod tests cover run_interactive_with_io, interactive_export_prompt, prompt_calc_mode, prompt_velocity_factor, prompt_antenna_height, prompt_ground_class, prompt_conductor_diameter, prompt_display_units, prompt_itu_region, prompt_antenna_model, and prompt_transformer_ratio paths including edge cases and error paths.
Notes: Affects FR-009. Resolved.

**GAP-010**: Golden corpus (testdata/) not yet established.
Status: **resolved** (2026-04-30)
Target phase: 2
Owner: unassigned
Resolution: Corpus directory, `reference-results.json`, `corpus-guide.md`, and CI gate in place. Nine corpus tests active: 5 ITU-R skip-distance cases (baseline + 4 height/ground-scaled), 1 non-resonant multi-band case, 1 NEC resonant baseline (dipole free space), 1 NEC case deferred (inverted-V). Non-NEC corpus complete; minimal NEC baseline active (GAP-011 partial).
Notes: NEC seed cases (`resonant_dipole_40m_nec`, `inverted_v_40m_nec`) remain ignored pending GAP-011. All non-NEC cases are CI-gated.

**GAP-011**: NEC reference sweeps postponed — COMP-001 tolerance matrix cannot be CI-gated.
Status: **partial** (2026-04-30)
Target phase: 2 (minimal), 3 (complete)
Owner: unassigned
Resolution: Minimal baseline established. 40m free-space resonant dipole (7.1 MHz) NEC deck created (`corpus/dipole-40m-freesp.nec`), fnec reference obtained (Z = 62.94 - j69.28 Ω), and corpus test `corpus_resonant_dipole_40m_nec` enabled (active, CI-gated baseline validation). Remaining work: 14+ NEC decks for ground variants, height-aware cases, inverted-V, EFHW, and conductor correction. See [docs/nec-requirements.md](nec-requirements.md) for full Phase 2/3 plan. Estimated completion: ~6.5 hours of NEC deck generation and testing.
Notes: COMP-001 resonant tolerance matrix rows remain partially deferred (dipole free-space now CI-gated; ground/height/other-antennas remain Phase 3). Affects full closure of COMP-001. Decision recorded: 2026-04-30.

---

## Traceability Matrix

| Requirement | Test File(s) | Status | Coverage Notes |
|:-----------|:------------|:-------|:------------------|
| FR-001 | tests/cli_integration.rs | covered | Basic resonant dipole, exit code 0 on success |
| FR-002 | tests/cli_integration.rs | covered | Band name parsing, ITU region selection |
| FR-003 | src/app.rs unit tests | covered | Antenna model dispatch and basic validation |
| FR-004 | tests/cli_integration.rs | covered | `--freq` and `--freq-list` both implemented and tested; 6 integration tests for freq-list paths (GAP-002 resolved) |
| FR-005 | tests/cli_integration.rs | covered | Custom bands via `bands.toml` (XDG + cwd), `--bands-preset`, `--bands-config`; TUI auto-discovers same file (GAP-001 resolved) |
| FR-006 | tests/cli_integration.rs, src/app.rs unit tests | covered | `--advise` mode, `build_advise_candidates`, TUI `a` key — 7+ integration tests; optimizer ranked candidates (GAP-003 resolved) |
| FR-007 | src/calculations.rs unit tests | covered | Skip distance calculation with height/ground scaling |
| FR-008 | tests/cli_integration.rs, src/export.rs unit tests | covered | CSV, JSON, Markdown, plain text all exported |
| FR-009 | src/cli.rs unit tests | covered | 50+ unit tests via injected I/O for all interactive prompt paths (GAP-009 resolved) |
| FR-010 | — | gap | TUI feature parity not yet complete; in-progress |
| FR-011 | tests/cli_integration.rs | covered | Scriptability contract tests for exit codes, field stability |
| FR-012 | tests/cli_integration.rs | covered | Invalid flags, bad values, all error paths tested |
| FR-013 | tests/cli_integration.rs | covered | `--info` output verified |
| NFR-001 | ci/performance.yml | advisory | Advisory threshold; not a hard build gate |
| NFR-002 | — | not-tested | Startup time not currently gated |
| NFR-003 | ci/coverage.yml | improved | calculations.rs now 44 unit tests (up from 23); tarpaulin unavailable on dev machine to measure exact % (GAP-004 resolved) |
| NFR-004 | — | observed | ~6 MB binary observed in v2.5.0 release |
| NFR-005 | — | living | Docs kept current via PR reviews |
| NFR-006 | src/tui/mod.rs unit tests | covered | Full keyboard navigation implemented; 30+ TUI key-dispatch tests (GAP-005 resolved) |
| COMP-001 | tests/corpus_validation.rs | partial | Baseline dipole (free space) NEC test enabled (CI-gated). Ground variants, height-aware, inverted-V/EFHW, conductor correction remain Phase 3 (GAP-011 partial resolution 2026-04-30) |
| COMP-002 | tests/corpus_validation.rs | covered | 5 skip-distance corpus cases (ITU-R baseline + 4 height/ground-scaled) + 1 non-resonant multi-band case (GAP-007 and non-NEC GAP-010 resolved) |
| COMP-003 | tests/export_format_contract.rs | covered | PAR-001 v1 (CSV) and PAR-002 v1 (JSON) contract tests in place (GAP-008 resolved) |

---

## Notes

- **Phase 1**: Resonant calculations, band selection, basic CLI, export (completed v1.0–v2.3).
- **Phase 2**: Non-resonant calculations, TUI, custom bands, balun optimizer, compliance testing, tolerance matrices (in progress, target v2.6–v3.0).
- **Phase 3**: GUI (iced), advanced antenna models, multi-modal optimization (planned).

---
