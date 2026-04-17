# Backlog

This document tracks near-term follow-up work that is agreed but not yet implemented.

## TUI Follow-ups

### 1. Editable export output-path input in Export panel

- Priority: High
- Status: Planned
- Motivation: The current export panel can cycle formats and write output, but output path changes still require leaving the TUI workflow.
- Scope:
  - Add an editable field for export output path in Export focus.
  - Reuse the existing input-edit interaction style (start edit, type, apply/cancel).
  - Validate path using the shared export-path validation flow before write.
- Acceptance criteria:
  - User can edit the output path entirely from TUI.
  - Invalid paths show a clear status message and do not write files.
  - Valid paths are persisted in TUI state and used by export action.

### 2. Per-field validation hints in panel lines

- Priority: Medium
- Status: Planned
- Motivation: Validation failures currently surface mainly through status messages; field-local hints improve discoverability and reduce error correction time.
- Scope:
  - Add inline validation indicators for editable input fields (bands, velocity factor, wire min, wire max).
  - Add export-panel inline hinting for invalid output-path states.
  - Keep app-layer validation as source of truth; UI hints should mirror app constraints.
- Acceptance criteria:
  - Input panel displays field-specific validation hints after invalid edits.
  - Export panel displays field-specific validation hints for path issues.
  - Existing status message channel remains available for operation-level feedback.

## Notes

- Backlog items here should be moved to roadmap only when promoted to active implementation work.
- Keep this file focused on actionable, implementation-ready follow-ups.
