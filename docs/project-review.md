# Project Review — Rusty Wire

**Date:** 2026-04-29  
**Scope:** Full codebase review at v2.7.0, covering project understanding, requirements, research, testing, and architecture.

---

## 1. Project Understanding

Rusty Wire is a command-line and terminal-UI application for amateur (ham) radio **wire-antenna length planning**. Its purpose is to compute resonant and non-resonant wire lengths across ITU-region-aware band tables, covering six antenna models, and to recommend balun/unun transformer ratios automatically.

### What it does

| Capability | Detail |
|---|---|
| **Antenna models** | Half-wave dipole, inverted-V, EFHW, full-wave loop, OCFD, trap dipole |
| **Calculation modes** | Resonant (per-band resonant lengths + harmonic analysis) and non-resonant (optimizer finds wire that avoids all resonances) |
| **ITU region support** | Region 1/2/3 with band variants for 80 m, 40 m, and 60 m |
| **Transformer advice** | Recommends and ranks balun/unun ratios; `--advise` mode ranks candidates by a scored model |
| **Export** | CSV, JSON, Markdown, TXT — in metric, imperial, or both |
| **Frontends** | CLI with `--interactive` prompt mode, and a full-screen TUI (`rusty-wire-tui`) built with ratatui |
| **Extensibility** | Named band presets from TOML, persistent user preferences, optional fnec-rust cross-validation |

### Engineering posture

The project is well beyond a prototype. It has:
- A layered, I/O-free application core (`app.rs`) shared by both the CLI and TUI
- Structured error types and proper CLI exit codes
- A pre-push hook enforcing `fmt` + `check` + `test`
- Committed SPDX and CycloneDX SBOMs
- Multi-architecture CI packaging (x86_64 + aarch64 Debian + Arch)
- Detailed documentation across architecture, math, testing, roadmap, and calibration

The project is clearly the work of a single, disciplined author with a coherent vision and above-average engineering hygiene for a personal tool.

---

## 2. Improvement Suggestions

### 2.1 Requirements

**Finding: requirements exist only implicitly.**

Functional requirements are scattered across `README.md`, `docs/roadmap.md`, `docs/backlog.md`, and code comments. There is no single requirements document that states what the tool must do, at what precision, and under what constraints. This is common for personal projects, but it creates two practical problems:

1. **No acceptance criteria.** There is no way to say a feature is "done" beyond it passing tests. The advise-mode scoring weight `0.35` in `docs/math.md` is a good example: where did that value come from, and what would falsify it?
2. **No user-level traceability.** Tests cannot be linked back to a stated requirement, so coverage claims are informal.

**Suggestions:**
- Add a lightweight `docs/requirements.md` that enumerates the top-level functional requirements (e.g., "the tool shall compute resonant half-wave length for any single band in ITU Region 1/2/3 with an error of less than ±0.5% relative to the ARRL handbook constant"). Even ten bullet points would sharpen what tests must cover.
- Document the *precision contracts* for each antenna model. For example: "EFHW lengths are computed to ±1 cm and reported to two decimal places." Right now this is implicit in the code.
- Add explicit scope boundaries: what the tool does *not* try to model (feedline losses, common-mode choke impedance, multi-element arrays). `docs/math.md` does this partially under "Practical Limits" — it should be elevated.
- The `steering.md` file appears to be a GitHub Copilot session configuration committed by mistake. It instructs an AI assistant to identify itself as "GPT-5.3-Codex". This has no place in the repository and should be removed or replaced with an actual project-steering document describing the guiding principles and design goals that inform decisions.

---

### 2.2 Research

**Finding: the empirical constants are documented but not validated against real data.**

The math in `docs/math.md` is admirably transparent. Formulas are cited to the ARRL Antenna Book, RSGB handbook, Kraus, and Pozar. However, several numerical constants in the code path are either unvalidated approximations or fitted to synthetic (template) data:

| Constant / model | Current status | Risk |
|---|---|---|
| **Conductor-diameter correction** (`k = 0.011542`) | Fitted to a synthetic template CSV; `docs/nec-calibration.md` admits this explicitly | The runtime clamp (`0.97..1.03`) is deliberately wider than the fit span because real NEC data has not been collected yet |
| **Inverted-V shortening factors** (0.97, 0.985) | Empirical rule-of-thumb from handbooks; no apex-angle sensitivity beyond two fixed values | May be significantly off for apex angles other than 90° and 120° |
| **Advise-mode scoring weight** (0.35) | Unexplained; no sensitivity analysis | A different weight would produce different rankings, which may matter for a user's actual antenna decision |
| **Skip-distance scaling** (height and ground-class multipliers) | Fixed discrete multipliers; no underlying propagation model cited | Gives false precision; error compounds when both height and ground are non-average |
| **Transformer length-correction heuristic** | Bounded logarithmic approximation; stated as "not a substitute for NEC modeling" | Users may not notice the disclaimer |

**Suggestions:**
- Collect at least one real NEC sweep for conductor diameter (1 mm, 2 mm, 4 mm, at 7 MHz and 14 MHz) and replace the template CSV. This would validate or correct `k` and allow the runtime clamp to tighten.
- Document the source and derivation of the advise-mode weight (0.35). If it was chosen by feel, say so and consider making it a configurable parameter so users can tune it to their own priorities.
- Explicitly state the expected precision of the skip-distance model (e.g., "order-of-magnitude only; do not use for propagation planning"). The current output format implies more precision than the model warrants.
- Consider separating *reference material* (ARRL/RSGB formulas) from *heuristics* (empirical shortening factors, scoring weights) in the math document so readers can judge how much to trust each value.

---

### 2.3 Testing

**Finding: good breadth, gaps in depth and feedback loop.**

The three-layer strategy (unit + integration + shell regression) is solid. 255 tests for a ~13,000 line codebase is respectable. The pre-push hook ensures tests are not skipped accidentally.

**Gaps identified:**

**Coverage is untracked.** There is no `cargo tarpaulin` or `cargo llvm-cov` configured. The 255-test count is known, but which code paths those tests exercise is not. `calculations.rs` and the non-resonant optimizer in `app.rs` are the most algorithmic parts of the codebase, and it is unclear how thoroughly their edge cases are covered.

**No property-based tests.** The optimizer in `calculations.rs` has mathematical invariants that are ideal candidates for property-based testing:
- For any wire length `L` returned by the non-resonant optimizer, `d(L)` should be ≥ `d(L ± step)`.
- Resonant lengths should always decrease monotonically as frequency increases.
- OCFD split ratios should always sum to 1.0 and stay within `[0.20, 0.80]`.

The `proptest` crate would let these be verified over thousands of random inputs without hand-crafting each case.

**Shell regression scripts are second-class.** The regression scripts (`test-itu-region-bands.sh`, `test-multi-optima.sh`, `test-nec-calibration.sh`) require the binary to be pre-built and do not run inside `cargo test`. This means they are easy to skip (the hook only runs `cargo test`, not `./scripts/test-all.sh`) and they do not appear in CI on pull requests.

**Interactive mode testing is incomplete.** The changelog notes slices 1–5 of interactive-mode coverage were added in 2.7.0 (up to 255 tests). The roadmap still lists "interactive-mode testability" as a priority. It is not clear which interactive paths remain uncovered.

**No mutation testing.** It is possible to write a test suite that passes even when the optimizer's comparison operator is flipped (e.g., `>` vs. `≥`). A single `cargo mutants` run on `calculations.rs` would reveal whether the existing tests would catch such regressions.

**No benchmarks.** The non-resonant sweep and multi-optima search are computationally heavier than the resonant path. Adding Criterion benchmarks for the optimizer would catch accidental performance regressions and provide a record of how performance scales with step size and band count.

**Suggestions:**
- Add `cargo llvm-cov --html` to the CI workflow on pull requests and commit the threshold (e.g., reject if line coverage drops below 70% in `calculations.rs`).
- Add property-based tests for the optimizer invariants using `proptest`.
- Move the regression scripts into `cargo test` integration tests (or call them from a `#[test]` that uses `std::process::Command`) so they appear in the standard test run and CI.
- Run `cargo mutants` on `calculations.rs` once and fix any surviving mutants. This is a one-time exercise that pays dividends.
- Document in `docs/testing.md` which interactive prompt paths are still uncovered and what the plan is to reach them.

---

### 2.4 Architecture and Design

**Finding: strong foundations, but scaling pressure is building.**

The separation of I/O-free `app.rs` from the CLI and TUI is the best design decision in the project. It made TUI development possible without rewriting the core, and it will do the same for the planned `iced` GUI. That investment is paying off.

**Observations:**

**`app.rs` is 4,280 lines.** It is the single largest file in the codebase by a wide margin. It contains orchestration, validation, display-document generation, transformer-recommendation logic, and result formatting. These are distinct responsibilities. As the TUI and future GUI demand richer structured output, the file will grow further. The risk is that `app.rs` becomes a second `cli.rs` — a place where unrelated logic accumulates because it doesn't clearly belong anywhere else.

**The crate is monolithic.** At 13,000+ lines across a single crate, the project is approaching the size where a Cargo workspace would help. The natural split is already visible in the architecture document:
- A `rusty-wire-core` library crate: `bands.rs`, `calculations.rs`, `app.rs`, `export.rs`
- A `rusty-wire-cli` binary crate: `cli.rs`, `main.rs`
- A `rusty-wire-tui` binary crate: `tui/mod.rs`, `bin/tui.rs`

This split would enforce the layer boundaries that are currently maintained only by convention, make `rusty-wire-core` independently versioned and publishable on crates.io, and reduce compile times for front-end-only changes.

**`lib.rs` is 22 lines.** The README mentions a library API, but `src/lib.rs` is essentially a re-export stub. If external tools should be able to use Rusty Wire as a library (which the README implies), the public API needs an intentional design: what types are stable, what is semver-versioned, and what are implementation details. Without this, any consumer of the library is depending on internals.

**The fnec integration is structurally optional but architecturally unspecified.** `fnec_validation.rs` (364 lines) is an optional cross-check tool. It is wired into the app layer but the interface between Rusty Wire's internal results and fnec's output format is not documented. If fnec-rust changes its output format, the integration will break silently in tests (because there is no CI job that runs with fnec present). Consider at minimum documenting the expected interface and mocking it in tests.

**Shadow types add overhead without a documented policy.** The project correctly uses separate `CliAntennaModel`/`AntennaModel` types to prevent clap from leaking into the domain. This is good. However, there is no document that states the policy ("CLI types live in `cli.rs`, domain types in `app.rs`; `From<>` conversions are the only bridge") so it is easy for a new contributor to accidentally add clap derives to a domain type.

**`docs/steering.md` should steer the project, not an AI assistant.** The current file is a GitHub Copilot session instruction — it tells an AI agent what model to claim to be and how to format responses. This should be replaced with a real steering document: what are the design goals, what trade-offs are non-negotiable, what is out of scope. This information currently lives in fragments across the roadmap, backlog, and README.

**Suggestions:**
- Break `app.rs` into at least three focused modules: `app/orchestration.rs` (request dispatch), `app/display.rs` (view-model generation), and `app/transformer.rs` (balun/unun logic). This does not require a workspace split and can be done incrementally.
- Plan the workspace split before the `iced` GUI is started. Adding a third binary after the workspace is established is far easier than migrating 13,000 lines into a workspace mid-development.
- Define the public library API explicitly. Add `#[doc(hidden)]` to internal types and write a brief `docs/library-api.md` that states what is stable.
- Replace `docs/steering.md` with a real project-steering document and remove the Copilot session configuration from the repository history.
- Add a short `CONTRIBUTING.md` that codifies the shadow-type policy, the layer contract (I/O-free app layer), and the test requirements for new features.

---

## Summary Table

| Area | Strength | Primary gap |
|---|---|---|
| **Requirements** | Feature scope is well understood by the author | No requirements doc; no precision contracts; `steering.md` is an AI config file |
| **Research** | Math is cited and transparent | Key constants (conductor model, scoring weights) are unvalidated against real NEC data |
| **Testing** | 255 tests, pre-push hook, shell regressions | Coverage untracked, no property tests, regression scripts not in CI, no mutation testing |
| **Architecture** | Clean I/O-free app layer, shadow types, layer separation | `app.rs` too large, monolithic crate, library API is a stub, fnec interface undocumented |
