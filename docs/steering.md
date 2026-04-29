# Project Steering

This document captures the design goals, constraints, and trade-offs that should guide work on Rusty Wire.

It is intentionally project-facing. Tooling- or editor-specific agent instructions belong in workspace configuration, not in repository documentation.

## Product Direction

Rusty Wire is a planning tool for amateur-radio wire antennas.

Its priorities are:

- fast, scriptable CLI execution
- consistent interactive and TUI workflows
- transparent calculations with documented assumptions
- practical outputs for antenna building and comparison
- a shared application layer that can support future front-ends

The project should remain useful both to operators who want one quick answer and to users who want to inspect assumptions, exports, and trade-offs in detail.

## Non-Goals

Rusty Wire does not attempt to be a full electromagnetic simulator.

Unless a feature explicitly says otherwise, the project should not claim to model:

- feedline loss or feedline transformation
- choke or common-mode behavior
- terrain, clutter, or full propagation prediction
- multi-element arrays or beam design
- exact NEC-grade current distribution or pattern analysis

Heuristic estimates are acceptable when they are clearly labeled as heuristics and their limits are documented.

## Modeling Posture

Rusty Wire should prefer transparent, cited formulas over opaque tuning.

When a result is based on reference material, the source should be documented in user-facing docs. When a result is based on a heuristic, fitted constant, or rule of thumb, that status should be explicit.

In practice this means:

- reference formulas should be traceable in documentation
- empirical constants should be easy to find and explain
- order-of-magnitude estimates should not be presented as high-precision predictions
- output wording should match model confidence

If a feature risks implying more certainty than the underlying model supports, the output or documentation should be corrected.

## Architecture Rules

Rusty Wire's core architectural rule is that the application layer remains I/O-free.

The intended layering is:

- `bands.rs` and `calculations.rs` contain reusable domain logic
- `app.rs` orchestrates validation, request handling, and shared result/view data
- `cli.rs` owns clap parsing, interactive prompts, and terminal printing
- `tui/` owns ratatui state, rendering, and event handling
- `export.rs` owns file export formatting and path validation

Additional rules:

- front-ends should share the same app-layer behavior instead of reimplementing business logic
- CLI-only shadow types should stay separate from domain types
- new features should be added to the app layer first when they affect more than one front-end
- result formatting should move toward reusable structured view data rather than ad hoc printing paths

If a change blurs these boundaries, it should be treated as design debt and justified explicitly.

## UX Principles

User-facing behavior should stay aligned across normal CLI mode, interactive mode, and the TUI where practical.

That includes:

- consistent naming for antenna models, regions, units, and exports
- consistent defaults and fallback behavior
- equivalent output semantics even when presentation differs by front-end
- clear validation errors rather than silent correction, except where a documented default is intentionally reused

When parity is not practical, the difference should be deliberate and documented.

## Documentation and Release Discipline

Documentation is part of the product, not an afterthought.

Changes that alter behavior, options, or release expectations should keep these documents current:

- `README.md` for the public feature surface and examples
- `docs/cli-guide.md` for option and workflow details
- `docs/CHANGELOG.md` for notable user-visible changes
- `docs/math.md` or related technical docs when formulas or heuristics change

Packaging metadata should stay in sync with the crate version, and release automation should keep enforcing that contract.

## Testing Expectations

The default quality bar is:

- `cargo fmt`
- `cargo check`
- `cargo test`

For changes that touch heuristics, optimizer behavior, band tables, exports, or packaging logic, the corresponding higher-level regression checks should also be updated or rerun.

Tests should favor meaningful behavioral guarantees over test-count growth for its own sake.

## Near-Term Steering

The next stage of the project is to strengthen the shared app layer so the current TUI and a future GUI can reuse the same validated behavior.

That implies:

- keeping app-layer contracts stable and explicit
- continuing to improve interactive-mode testability
- separating view-friendly result data from terminal-specific rendering
- resisting front-end shortcuts that duplicate core logic

## Decision Rule

When two implementations are both workable, prefer the one that:

1. keeps the app layer clean and reusable
2. makes assumptions easier to explain to users
3. preserves CLI, interactive, and TUI parity
4. reduces maintenance cost for future front-ends