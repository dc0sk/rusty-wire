# Memory Export

Export date: 2026-04-18

## User Memory (/memories)

### /memories/cli-parity.md

- Keep CLI mode and interactive mode aligned for user-facing features and outputs when making changes to Rusty Wire.
- If parity is not practical for a change, explicitly tell the user before or when implementing it.
- Default pre-push sequence for Rusty Wire: `cargo fmt`, `cargo check`, `cargo test`, then push only if all pass.
- Release cadence preference: ship current stable milestone as a minor bump (e.g., 1.5.0), then increment patch versions (1.5.1, 1.5.2, ...) for subsequent antenna-model expansion batches.
- Decision preference: when offered numbered execution options, default to option 1 (logical split commits and push) unless told otherwise.

## Session Memory (/memories/session)

- No memories currently stored.

## Repository Memory (/memories/repo)

### /memories/repo/roadmap.md

- Canonical future-work document: /home/dc0sk/git/rusty-wire/docs/roadmap.md
- Use docs/roadmap.md instead of session-only plan notes for remaining improvements.
- High-priority remaining items: interactive-mode testability, error-handling cleanup, search precision control, custom frequency input, advanced antenna models.
- Antenna-model additions need parity updates in src/app.rs, src/calculations.rs, src/cli.rs, src/export.rs, tests/cli_integration.rs, README.md, docs/cli-guide.md, and docs/CHANGELOG.md; note that TXT export has its own formatter and is easy to miss.