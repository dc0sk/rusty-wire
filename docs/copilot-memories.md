# Rusty Wire Copilot Memory Export

This file is a documentation snapshot of the current Copilot memory state for this repository.
Keep it synchronized with the contents of `/memories` and `/memories/repo` before pushing changes.

Export date: 2026-04-15

## User memory

No user memory entries are currently stored.

## Session memory

No session memory entries are currently stored.

## Repository memory

### Source: `/memories/repo/rusty-wire-notes.md`

# Rusty Wire Notes

- Error handling and validation are now unified in the app layer with a comprehensive AppError enum. All user input and config validation is tested.
- Tests cover all AppError variants and validation helpers (see app_error_tests in src/app.rs).
- CLI and future UIs should use app-layer helpers for all validation and error reporting.
- CLI interactive prompt helpers were extracted to module-level functions in src/cli.rs so runtime paths and tests share the same behavior.
- Interactive CLI result rendering and equivalent-command suggestions now stay on the provided writer abstraction, with CLI tests covering writer-based output instead of relying on stdout leaks.
- Frontend planning now treats AppConfig/AppRequest/AppResults/AppResponse as the shared app contract; TUI and GUI state should remain frontend-specific rather than becoming a universal AppState/AppAction core.
- Non-resonant recommendation rendering in UnitSystem::Both retains the ", recommended" marker for local optima; regression coverage was added in src/app.rs.
- Version 2.2.0 was released on 2026-04-14; tags 2.2.0 and v2.2.0 both point to the corrected release commit on main.
- Post-release cleanup is complete: merged branch feature/ui-integration-initial-prep was deleted, and new work should continue from feature/ui-state-foundation branched from updated main.