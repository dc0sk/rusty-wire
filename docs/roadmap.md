# Roadmap

This document captures the most relevant future improvements that remain after the 1.4.0 CLI refactor, testing expansion, and documentation work.

It is intentionally scoped to what is still useful, not a dump of already completed work.

## Already Landed

These items were part of earlier planning and are now complete:

- clap-based CLI parsing
- restored interactive mode behind `--interactive`
- no-argument help behavior
- export path validation hardening
- unit and integration test coverage for current CLI behavior
- testing and architecture documentation

## Remaining High-Value Improvements

## Error Handling Cleanup

- return structured errors from `app::run_calculation` and related helpers instead of relying on `eprintln!` for invalid-band reporting
- centralize CLI/display error formatting in `src/cli.rs`
- reduce duplicated validation and conversion logic between interactive prompts and non-interactive CLI handling

This would make the application easier to test, easier to reuse from future front ends, and less coupled to terminal output.

## Interactive-Mode Testability

- extract interactive stdin/stdout handling behind a small I/O interface
- add automated tests for menu navigation, prompt validation, and export selection

The current interactive flow is functional, but it still depends on direct terminal I/O and is therefore mostly manually tested.

## Advanced Input Support

- support direct frequency input such as `--freq 7.1`
- support multiple explicit frequencies such as `--freq-list 7.0,10.1`
- support user-defined band presets through a config file such as `bands.toml` or `bands.json`

These would make the tool more useful outside fixed ham-band workflows.

## Antenna Model Expansion

- add additional antenna models beyond the current dipole-centered calculations
- evaluate full-wave, loop, center-fed, off-center-fed, and end-fed modelling options
- explore trap and hybrid antenna support

This is the most substantial feature area and likely requires changes in both `src/calculations.rs` and the user-facing configuration model.

## Search and Analysis Controls

- add a configurable `--precision` or `--step` option for non-resonant search resolution
- add batch output for multiple velocity factors or multiple transformer ratios in one run
- add a compact `--report` or `--summary` mode for automation-friendly output

These changes would improve power-user workflows without requiring a large architectural shift.

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

1. interactive-mode testability
2. error-handling cleanup
3. configurable non-resonant search resolution
4. custom frequency and user-defined band input
5. advanced antenna models such as end-fed and trap support

## Affected Areas

- `src/cli.rs`: interactive I/O, CLI options, validation, automation modes
- `src/app.rs`: request orchestration and error propagation
- `src/calculations.rs`: new antenna models, search controls, output batching
- `src/bands.rs`: custom/user-defined bands
- `src/export.rs`: richer export formats and schemas
- `tests/` and `scripts/`: expanded regression coverage as features grow