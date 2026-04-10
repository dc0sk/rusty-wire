# Copilot Collaboration Playbook Template

Last updated: YYYY-MM-DD

This is a reusable playbook for working with Copilot (or similar coding assistants) across projects.
Customize examples and commands for your stack.

## One-Page Quick Checklist

Before prompting:

- State the outcome in one sentence.
- Define scope boundaries (in/out of scope files/modules).
- List constraints (compatibility, UX, performance, security).
- Define acceptance criteria and edge cases.
- Define validation commands.

Execution mode:

- Decide one: analysis-only, implement-only, implement+commit, implement+commit+push.
- State whether rebases/history rewrites are allowed.

Before push:

1. [format command]
2. [lint command]
3. [build/check command]
4. [test command]
5. Push only if all pass

## 1. How To Ask for High-Quality Results

### Prompt structure

1. Objective
- What should be true when done.

2. Constraints
- What must not change.

3. Acceptance criteria
- Observable outputs/behaviors.

4. Validation
- Commands and expected pass condition.

5. Delivery
- File edits only, commit policy, push policy.

### Precision boosters

Use phrases like:

- "Implement X in [files], do not touch [files]."
- "Keep public behavior unchanged except [explicit delta]."
- "If blocked, choose minimal-risk fallback and explain it."
- "Add regression tests for [exact scenario]."

Avoid:

- "Refactor everything"
- "Improve this" without criteria
- "Make it better" without measurable goals

## 2. Work Modes You Can Request

### Analysis mode

Use when risk is high or requirements are fuzzy.

Prompt starter:

"Do read-only analysis. Provide root cause, impact, and 2-3 fix options with tradeoffs. Do not edit yet."

### Execution mode

Use when requirements are clear.

Prompt starter:

"Implement [change] with [constraints], then run [validation], and summarize results."

### Review mode

Use before merge.

Prompt starter:

"Review this branch for bugs, regressions, and missing tests. Findings first, ordered by severity."

## 3. Planning and Scoping Patterns

- Prefer thin vertical slices over broad refactors.
- Land one behavior change at a time.
- Keep acceptance criteria testable.
- Explicitly mark non-goals.
- Timebox exploratory work and report what remains uncertain.

## 4. Testing Strategy Guidance

When asking for tests, specify layers:

- Unit tests: core logic and edge cases.
- Integration tests: user-visible flows across module boundaries.
- Smoke checks: startup/build/basic commands.
- Regression tests: exact previously broken behavior.

Useful request:

"Add one failing test that reproduces the bug, implement fix, then prove it passes."

## 5. Documentation and Change Hygiene

- Require docs updates for user-visible changes.
- Require changelog entries for release-impacting behavior.
- Keep one canonical roadmap/planning file.
- Ask for concise migration notes when APIs/flags change.

## 6. Security and Compliance Topics To Include

If relevant, ask explicitly for:

- Input validation and sanitization checks
- Secret handling and redaction in logs
- Dependency risk review for new packages
- License compatibility checks
- Safe defaults for config and network behavior

## 7. Performance and Reliability Topics To Include

When needed, ask for:

- Baseline vs after-change metrics
- Complexity impact and hot-path review
- Memory/CPU/network implications
- Timeout/retry/backoff behavior for external calls
- Failure-mode and graceful-degradation behavior

## 8. Git and Release Workflow Suggestions

- Ask for logical commit splits (for example: feature, tests, docs).
- Ask for pre-push validation summary in every implementation request.
- Define versioning policy up front (major/minor/patch meaning).
- Define tag format convention (`vX.Y.Z` and/or `X.Y.Z`).
- Ask for release checklist execution before tagging.

## 9. Collaboration Anti-Patterns

Watch for and avoid:

- Ambiguous scope with no acceptance criteria
- Large mixed commits (feature + refactor + docs + unrelated cleanup)
- Missing regression tests for bug fixes
- Skipping validation because changes "look small"
- Silent behavior changes not reflected in docs

## 10. Reusable Prompt Starters

### Feature implementation

"Implement [feature] in [scope]. Constraints: [list]. Acceptance criteria: [list]. Add tests for [cases]. Update docs [files]. Run [validation commands]."

### Risk-limited change

"Implement minimal-diff fix for [issue]. Preserve APIs/UX. Do not refactor unrelated code. Add one regression test."

### Release prep

"Prepare release [version]: update version files + changelog, run [validation], create tags per convention, and provide release summary."

### Architecture discussion

"Propose 2-3 designs for [problem] with tradeoffs across complexity, performance, and migration risk. No code edits yet."

## 11. Terminology Glossary Template

Define project terms once and reuse them consistently.

- Acceptance criteria: [project definition]
- Breaking change: [project definition]
- Regression: [project definition]
- Compatibility: [project definition]
- Validation: [project definition]
- Scope: [project definition]
- Constraints: [project definition]

Project-domain terms:

- [term 1]: [plain-language definition]
- [term 2]: [plain-language definition]
- [term 3]: [plain-language definition]

## 12. Language Guide for Non-Native Speakers (Template)

Recommended style:

- Keep sentences short.
- Use one idea per sentence.
- Prefer direct words over idioms.
- Use consistent vocabulary for recurring concepts.
- Give examples for non-obvious options.

Wording template:

- Instead of: "[vague phrase]"
- Use: "[specific observable behavior]"

Example transformations:

- Instead of: "Improve this"
- Use: "Change X so output contains A and does not contain B"
- Instead of: "Handle edge cases"
- Use: "Handle these edge cases: [list]"

## 13. Meaning Check Questions (Template)

Ask these before implementation and before merge:

- Do we mean same behavior, same output text, or same implementation?
- Is this additive, replacement, or removal?
- Who is impacted by this change?
- What commands prove success?
- What is explicitly out of scope?

## 14. Example Phrase Bank (Template)

Scope and boundaries:

- "Implement [feature] in [files/modules] only."
- "Do not edit [files/modules]."
- "Out of scope: [list]."

Behavior and compatibility:

- "Keep public behavior unchanged except [explicit delta]."
- "Do not remove existing functionality without confirmation."
- "Prefer minimal-diff change unless I request refactoring."

Testing and validation:

- "Add regression tests for [specific failure mode]."
- "Run [format], [lint], [build], [tests]; stop on failure."
- "Report pass/fail and any residual risk."

Documentation and release:

- "Update [docs/changelog] to match user-visible behavior changes."
- "Prepare release [version] using our validation and tagging conventions."

Commit and push control:

- "Use logical commit split: [split]."
- "Do not push until I approve."

## 15. Team Adoption Tips

- Keep this template in-repo and review quarterly.
- Add project-specific command blocks and examples.
- Keep a short "decision log" section for stable conventions.
- Pair this playbook with a constraints file for best consistency.
