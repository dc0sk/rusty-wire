# Copilot Collaboration Playbook

Last updated: 2026-04-10

This document is a practical guide for working with GitHub Copilot effectively in this repository.
It is based on this session's outcomes and general best practices for precise, efficient coding collaboration.

## One-Page Quick Checklist

Use this when you want fast, high-quality execution.

Before prompting:

- Define the outcome in one sentence.
- Name exact files/modules in scope.
- State constraints (parity, no feature removal, minimal diffs).
- Define acceptance criteria (including edge cases).
- Specify validation commands.

Prompt shape:

- Objective: what should be true when done.
- Constraints: what must not change.
- Acceptance criteria: observable expected behavior.
- Validation: `cargo fmt`, `cargo check`, `cargo test`.
- Delivery: edit only, edit + commit, or edit + commit + push.

During execution:

- Prefer small vertical slices over large refactors.
- Add/adjust tests when behavior changes.
- Update docs when user-facing behavior changes.
- Ask before any intentional feature removal or UX break.

Before push:

1. `cargo fmt`
2. `cargo check`
3. `cargo test`
4. Push only if all pass

Release cadence:

- Stable milestone as minor bump (example: `1.5.0`).
- Incremental follow-ups as patch bumps (`1.5.1`, `1.5.2`, ...).

## 1. What To Decide Before You Ask

Before sending a request, decide these first:

- Goal: what should be true at the end?
- Scope: which files or modules are in and out?
- Priority: speed, safety, minimal diff, or architecture quality?
- Validation: which commands prove success?
- Delivery: implement now, draft only, or review only?

If these are clear up front, iteration time drops significantly.

## 2. Prompt Template That Works Well

Use this structure when you want strong execution quality:

1. Objective:
- One sentence describing the outcome.

2. Constraints:
- Must keep CLI and interactive behavior in sync.
- Must not remove existing functionality without asking.
- Keep changes minimal and focused.

3. Acceptance criteria:
- List exact expected behaviors and output text when relevant.
- Include edge cases.

4. Validation:
- Run `cargo fmt`, `cargo check`, `cargo test`.
- Report results and any residual risk.

5. Commit instructions (optional):
- Single commit vs logical split.
- Commit message style.
- Push or do not push.

## 3. Phrasing Patterns That Improve Precision

Prefer:

- "Implement X in files A/B, do not touch C."
- "Keep existing UX; add behavior without regressions."
- "If tradeoffs exist, choose minimal-risk behavior and explain why."
- "Add tests for these cases: ..."
- "Do pre-push sequence and stop if any step fails."

Avoid vague prompts like:

- "Improve this"
- "Refactor everything"
- "Make it better"

The more concrete your expected behavior is, the fewer correction loops you need.

## 4. What Helps Most In This Repo Specifically

From this session, these repo-specific constraints matter a lot:

- CLI and interactive mode parity is important and should be preserved.
- Feature removal requires explicit confirmation.
- Changes should include docs and tests where behavior changes.
- Roadmap planning should stay in `docs/roadmap.md`.
- Releases are done with strict validation before push.

## 5. High-Value Request Types

### A) Feature work
Good request example:

"Add antenna model X, keep CLI and interactive parity, update exports and docs, add integration tests for default + model-filtered output, then run fmt/check/test."

### B) Bug fix
Good request example:

"Fix regression in resonant summary output for `--antenna loop`; keep current public CLI flags unchanged; add a failing test first, then fix, then run fmt/check/test."

### C) Review request
Good request example:

"Do a code review of this branch focused on bugs/regressions and missing tests. Findings first with severity and file/line references."

## 6. Fast Iteration Workflow

When you want speed without losing safety:

1. Ask for an implementation in small vertical slices.
2. Require tests per slice.
3. Require command validation after each slice.
4. Ask for commit after each stable slice.
5. Push only when the branch is green.

This avoids giant diffs and makes rollback easier.

## 7. Pre-Push and Release Discipline

Recommended standard workflow:

1. `cargo fmt`
2. `cargo check`
3. `cargo test`
4. Commit only validated changes
5. Push only if all checks pass

Release cadence preference established in session:

- Milestone release as minor bump (for example `1.5.0`)
- Follow-up incremental antenna batches as patch bumps (`1.5.1`, `1.5.2`, ...)

## 8. What To Tell Copilot When You Need More Control

If you want tighter control, explicitly say:

- "Do not push automatically."
- "Do not commit yet; show me summary first."
- "Use logical commit split: fmt, feature, docs."
- "Stop and ask before any behavior change in interactive mode."
- "Prefer compatibility over elegance for this change."

## 9. Handy Copy/Paste Prompt Starters

### Starter: exact implementation

"Implement [feature] in [files/scope]. Keep CLI and interactive parity. Do not remove existing behavior without asking. Add tests for [cases]. Update docs [files]. Run `cargo fmt`, `cargo check`, `cargo test`. Commit with message: '[message]'."

### Starter: safe investigation

"Investigate [issue]. Do read-only analysis first. Provide root cause, affected files, and minimal fix options. Do not edit until I choose an option."

### Starter: release prep

"Prepare release [version]. Update version files and changelog, run `cargo fmt`, `cargo check`, `cargo test`, commit release bump, create tags (`vX.Y.Z` and `X.Y.Z`), then push."

## 10. Final Principle

Best results come from combining:

- clear intent,
- explicit constraints,
- concrete acceptance criteria,
- mandatory validation,
- and small reversible steps.

That combination makes collaboration faster, safer, and far more predictable.
