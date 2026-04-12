# Roadmap

This document captures the most relevant work that remains after the 2.1.0 release.

It is intentionally trimmed to the items that are still useful. Completed milestones are kept short so the roadmap stays actionable.

## Recently Landed

These areas are no longer roadmap items:

- clap-based CLI parsing and no-argument help behavior
- interactive mode restoration behind `--interactive`
- interactive-mode I/O refactor and automated prompt/menu coverage
- region-aware band selection and named band/range input
- antenna model expansion through EFHW, loop, inverted-V, and OCFD
- recommended transformer selection with mode/model-aware defaults
- export path validation hardening
- SBOM generation and pre-push SBOM enforcement
- unit, integration, and regression-script coverage for current CLI behavior
- testing, architecture, and CLI documentation refresh

## Remaining High-Value Improvements

## Error Handling Cleanup

- return structured errors from `app::run_calculation` and related helpers instead of relying on terminal-oriented reporting
- centralize end-user error formatting in `src/cli.rs`
- reduce duplicated validation and conversion logic shared between interactive prompts and non-interactive CLI execution

This would make the code easier to reuse from future front ends and easier to test at the app layer.

## Advanced Input Support

- support direct frequency input such as `--freq 7.1`
- support multiple explicit frequencies such as `--freq-list 7.0,10.1`
- support user-defined band presets through a config file such as `bands.toml` or `bands.json`

These would make the tool more useful outside fixed amateur-band workflows.

## Transformer Recommendation and Selection

- keep `--transformer recommended` as the default entry point, but make the recommendation model more transparent in CLI help and output
- evaluate whether EFHW should remain fixed at `1:56` or be promoted to a ranked recommendation across `1:49`, `1:56`, and `1:64`
- consider an optional recommendation/optimization pass that compares plausible transformer ratios for the selected mode, antenna model, and band set
- present recommendations as guidance while still allowing explicit user override

The current implementation uses fixed recommended defaults by mode and antenna model. Future work here is about ranking or optimizing those choices rather than hard-coding more one-off rules.

## Search and Analysis Controls

- add a configurable `--precision` or `--step` option for non-resonant search resolution
- add batch output for multiple velocity factors or multiple transformer ratios in one run
- add a compact `--report` or `--summary` mode for automation-friendly output

These changes would improve power-user workflows without requiring a large architectural shift.

## Antenna Model Expansion

- add additional models beyond the current dipole, inverted-V, EFHW, loop, and OCFD support
- explore trap, hybrid, and other multi-section antenna models
- evaluate whether more antenna-specific feed recommendations should be modeled in the application layer

This remains one of the most substantial feature areas and likely requires changes in both `src/calculations.rs` and the user-facing configuration model.

## Export Improvements

- add richer machine-readable export formats such as YAML
- consider HTML export for printable/shareable reports
- improve the JSON schema for programmatic consumers if external integration becomes important

## Logging and Automation Modes

- add `--quiet` and/or `--verbose` flags
- add a `--dry-run` mode for automation and script validation

These would make the CLI easier to integrate into larger workflows.

## Suggested Priority Order

If work continues incrementally, a good order is:

1. error-handling cleanup
2. configurable non-resonant search resolution
3. direct/custom frequency input
4. transformer recommendation optimization
5. logging and automation modes
6. next-generation antenna models

## Affected Areas

- `src/cli.rs`: CLI options, interactive prompts, validation, automation modes, recommendation messaging
- `src/app.rs`: request orchestration, recommendation policy, and error propagation
- `src/calculations.rs`: new antenna models, search controls, and transformer-comparison logic
- `src/bands.rs`: custom/user-defined bands and frequency presets
- `src/export.rs`: richer export formats and schemas
- `tests/` and `scripts/`: expanded regression coverage as features grow