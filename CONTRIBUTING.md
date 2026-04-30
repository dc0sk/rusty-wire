# Contributing to Rusty Wire

Thanks for your interest in improving Rusty Wire. This guide covers the
conventions and contracts that the project relies on so that new code
fits cleanly with the rest of the codebase.

If anything here is unclear, please open an issue rather than guess.

---

## 1. Architectural contracts

Rusty Wire has a small number of architectural rules that are not
enforced by the compiler. Please read these before adding new code.

### 1.1 The app layer is I/O-free

`src/app.rs`, `src/calculations.rs`, `src/bands.rs`, and the rest of the
domain layer **must not** perform any I/O directly. That means no
`println!`, no `eprintln!`, no `std::fs::*`, no `std::env::*` in those
modules. The only exception is `src/fnec_validation.rs`, which is
explicitly an external-tool integration.

The contract is:

```
AppRequest  ──>  execute_request_checked()  ──>  AppResponse
                       (pure function)
```

Anything user-facing — terminal output, file writes, exit codes,
prompts, file paths — lives in `src/cli.rs`, `src/main.rs`,
`src/tui/`, `src/export.rs`, `src/prefs.rs`, or `src/sessions.rs`.

If you find yourself wanting to print something from the app layer,
return a structured field on `AppResponse`, `AppResults`, or one of the
view types instead and let the front-end format it.

### 1.2 Shadow types: keep clap out of the domain

`clap` derives live on **CLI shadow types only** (e.g.
`CliAntennaModel`, `CliCalcMode`, `CliExportFormat`). The domain types
in `src/app.rs` (e.g. `AntennaModel`, `CalcMode`, `ExportFormat`) must
never carry `#[derive(clap::ValueEnum, ...)]` or any other CLI-framework
attribute.

Conversions between the two live in `src/cli.rs` as `From<...>` impls.

This rule exists so the `iced` GUI (3.x) and any future front-end can
depend on the domain types without pulling in clap.

### 1.3 Three configuration containers, three roles

| Type | File | Role |
|------|------|------|
| `AppConfig` | `src/app.rs` | Live runtime configuration; mutable |
| `UserPrefs` | `src/prefs.rs` | Sparse defaults persisted to `~/.config/rusty-wire/config.toml` |
| `SessionConfig` | `src/sessions.rs` | Full named-session snapshot persisted to `~/.config/rusty-wire/sessions.toml` |

Add a field to **all three** when introducing a new persisted
configuration option. Add a roundtrip test in `prefs.rs` and
`sessions.rs` covering the new field.

---

## 2. Test requirements

Rusty Wire ships with ~400 tests across lib, CLI, corpus, and contract
suites. New features are expected to come with tests at the appropriate
level:

| Change | Required tests |
|--------|---------------|
| New domain calculation | Unit tests in the same module + at least one CLI integration test |
| New `AppAction` variant | Unit tests for `apply_action()` covering both happy path and rejection |
| New export format | A `to_<format>()` unit test, an entry in `tests/export_format_contract.rs`, and a CLI integration test |
| New CLI flag | Unit test for clap parsing + integration test in `tests/cli_integration.rs` |
| New TUI overlay | State-driven test that drives `handle_key()` directly (no terminal needed) |
| New persisted config field | Roundtrip test in both `prefs.rs` and `sessions.rs` |

Run the full suite locally before opening a PR:

```bash
bash scripts/test-all.sh
```

This runs `cargo fmt --check`, `cargo clippy`, `cargo test`, and the
NEC calibration regression checks.

---

## 3. Commit and PR conventions

### 3.1 Commit message format

Use conventional-commit prefixes:

- `feat:` — new user-facing feature
- `fix:` — bug fix
- `refactor:` — internal change with no behavioural difference
- `chore:` — version bumps, packaging, CI, dependencies
- `docs:` — documentation only
- `test:` — tests only

Optionally scope: `feat(tui):`, `fix(export):`, `refactor(app):`.

The first line should be ≤ 72 characters. Use the body for context,
references to issues, and any breaking-change notes.

### 3.2 PR checklist

Before requesting review, please confirm:

- [ ] `bash scripts/test-all.sh` passes locally
- [ ] `cargo clippy --all-targets` produces no new warnings
- [ ] `docs/CHANGELOG.md` has an entry under `[Unreleased]` (for
      user-facing changes)
- [ ] `docs/backlog.md` and `docs/roadmap.md` are updated if the change
      closes a backlog item
- [ ] Any new public API on the lib crate has a doc comment

### 3.3 Releases

Releases are cut from `main`. The release procedure is documented in
`docs/release-checklist.md` and includes:

1. Bump `Cargo.toml` and run `scripts/sync-packaging-version.sh <ver>`
2. Move CHANGELOG entries from `[Unreleased]` to a new versioned section
3. Regenerate SBOM artifacts: `bash scripts/generate-sbom.sh spdx` and
   `bash scripts/generate-sbom.sh cyclonedx`
4. `bash scripts/test-all.sh` and `scripts/check-packaging-version-sync.sh`
5. Commit, then `git tag -a vX.Y.Z`

---

## 4. Where to put new code

| Concern | Location |
|---------|----------|
| Pure antenna math | `src/calculations.rs` |
| Band tables, ITU regions | `src/bands.rs` |
| Custom band-preset loading | `src/band_presets.rs` |
| State machine, view types, orchestration | `src/app.rs` |
| Export formatters | `src/export.rs` |
| CLI parsing and shadow types | `src/cli.rs` |
| TUI rendering and key handling | `src/tui/mod.rs` |
| Persistent preferences | `src/prefs.rs` |
| Named sessions | `src/sessions.rs` |
| Optional fnec-rust integration | `src/fnec_validation.rs` |

If you are unsure where a change belongs, prefer the most
domain-specific location (calculations > app > front-end).

---

## 5. Reporting issues

Useful issue reports include:

- Rusty Wire version (`rusty-wire --info`)
- The exact command line or the TUI key sequence
- Expected vs actual output
- OS and terminal (for TUI issues)

For calculation discrepancies, please include the band, antenna model,
height, ground class, and conductor diameter so we can reproduce.
