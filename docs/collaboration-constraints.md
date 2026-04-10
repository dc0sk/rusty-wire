# Collaboration Constraints Summary

Last updated: 2026-04-10

This file captures the working constraints and preferences established during recent Rusty Wire work.

## Product and UX Constraints

- Keep CLI mode and interactive mode aligned for user-facing features and outputs.
- If full CLI/interactive parity is not practical for a change, call it out explicitly before or during implementation.
- Do not remove existing features without explicit user confirmation.
- Antenna model behavior:
  - If no antenna model is selected, show all supported model outputs per band.
  - If a specific model is selected, show only that model's per-band output.
- Resonant summaries:
  - Dipole resonant-point summary remains dipole-oriented.
  - Compromise lengths may still be shown for EFHW/loop as tuner-assisted guidance.
  - For EFHW/loop compromise output, clearly label it as dipole-derived guidance and a starting point.

## Quality and Validation Constraints

- Use this default pre-push sequence:
  1. cargo fmt
  2. cargo check
  3. cargo test
  4. push only if all pass
- Keep integration tests in place for CLI behavior changes, especially when output semantics are updated.
- Maintain docs/tests alongside behavior changes so release state remains coherent.

## Release and Versioning Constraints

- Prefer releasing a stable milestone before stacking more model-expansion work.
- Release cadence preference:
  - Use a minor version for a milestone release (example: 1.5.0).
  - Use patch bumps for subsequent incremental model batches (1.5.1, 1.5.2, ...).
- Dual tagging convention is acceptable and currently used:
  - v-prefixed tag (example: v1.5.0)
  - non-prefixed tag (example: 1.5.0)

## Planning and Documentation Constraints

- Canonical future-work source is docs/roadmap.md.
- Prefer updating docs/roadmap.md over maintaining session-only planning notes.
- Keep changelog entries aligned with shipped behavior and test coverage.

## Persistent Memory Snapshot

These points mirror current persistent memory entries to keep this file as the single canonical reference.

### Workflow Memory

- Keep CLI mode and interactive mode aligned for user-facing features and outputs.
- If parity is not practical for a change, call it out explicitly before or during implementation.
- Default pre-push sequence:
  1. cargo fmt
  2. cargo check
  3. cargo test
  4. push only if all pass

### Release Memory

- Prefer shipping a stable milestone before stacking further antenna-model expansion work.
- Versioning cadence preference:
  - minor bump for a milestone (example: 1.5.0)
  - patch bumps for incremental follow-up batches (1.5.1, 1.5.2, ...)

### Planning Memory

- Canonical future-work source is docs/roadmap.md.
- Prefer updating docs/roadmap.md over session-only planning notes.
- Current high-priority roadmap themes:
  - interactive-mode testability
  - error-handling cleanup
  - non-resonant search precision control
  - custom frequency input
  - advanced antenna models
