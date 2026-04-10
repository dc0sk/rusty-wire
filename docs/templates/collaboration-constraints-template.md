# Collaboration Constraints Template

Last updated: YYYY-MM-DD

Use this file as a project-specific contract between you and your coding assistant.
Replace placeholders and delete sections you do not need.

## Purpose

Define non-negotiable constraints for implementation, validation, release hygiene, and communication.

## Product and UX Constraints

- Preserve backward compatibility for existing public behavior unless explicitly approved.
- Call out user-facing behavior changes before implementation.
- Keep interfaces aligned across entry points (for example: CLI, UI, API, automation).
- Prefer additive changes over destructive changes.
- If removal is required, get explicit confirmation first.

Project-specific UX rules:

- [Add project-specific rule]
- [Add project-specific rule]

## Code and Architecture Constraints

- Keep diffs focused and minimal; avoid unrelated refactors.
- Preserve existing public APIs unless migration is part of the task.
- Follow existing module boundaries and naming conventions.
- Add concise comments only where logic is non-obvious.
- Prefer deterministic behavior over implicit side effects.

## Testing and Validation Constraints

Default validation sequence:

1. [formatter command]
2. [static analysis/lint command]
3. [build/check command]
4. [test command]
5. Push only if all pass

Coverage expectations:

- Add/adjust tests for behavior changes.
- Include regression tests for bug fixes.
- Include at least one integration-level validation for user-facing flows.

## Documentation Constraints

- Update user-facing docs whenever behavior changes.
- Keep changelog aligned with shipped changes.
- Maintain one canonical planning document for future work.
- If docs are intentionally deferred, note that explicitly in PR/commit summary.

Canonical docs paths:

- Roadmap: [path]
- Changelog: [path]
- Testing guide: [path]

## Security and Privacy Constraints

- Do not expose credentials, tokens, or sensitive data in code, logs, or examples.
- Avoid insecure defaults in sample configs.
- Validate and sanitize untrusted input.
- Prefer explicit error messages that do not leak sensitive internals.

## Dependency and Tooling Constraints

- Add new dependencies only when justified.
- Prefer well-maintained, widely used libraries.
- Document dependency rationale for non-trivial additions.
- Pin or constrain versions according to team policy.

## Git Workflow Constraints

- Use logical commit boundaries (for example: fmt, feature, docs).
- Do not rewrite history on shared branches unless explicitly requested.
- Never run destructive git commands without explicit approval.
- Push only after successful local validation.

Branch and PR conventions:

- Branch naming: [convention]
- Commit style: [convention]
- PR checklist: [link or bullets]

## Release Constraints

- Define release cadence and versioning policy in advance.
- Require release notes/changelog updates before tagging.
- Use consistent tagging format (for example: `vX.Y.Z` and/or `X.Y.Z`).
- Verify release artifacts with the same validation sequence used pre-push.

Versioning policy:

- Major: [definition]
- Minor: [definition]
- Patch: [definition]

## Communication Constraints

- Ask clarifying questions only when blockers exist.
- Report progress in short checkpoints during long tasks.
- Surface risks, assumptions, and tradeoffs early.
- Provide concrete next steps after implementation.

## Decision Log (Optional)

Record stable decisions here so future work remains consistent.

- YYYY-MM-DD: [decision]
- YYYY-MM-DD: [decision]
