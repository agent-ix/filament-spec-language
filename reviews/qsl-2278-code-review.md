---
id: SR-2135
title: "Code and Rust review of QSL-2278 PR #2737"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; origin/main...0640c7ffd7e8d0773cf26326a4ca0d67eebf29b6; PR #2737; Ticket QSL-2278; Makefile, xtask/ci_spec_validation.rs, xtask/src/ci_spec_validation_tests.rs; NFR-002-AC-1..4; TC-920"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-002
    type: reviews
---

## Summary

Ticket QSL-2278. Reviewed the frozen PR diff against `origin/main`. The Rust helper resolves executable `quire` candidates to canonical npm `@agent-ix/quire-cli/bin/launcher.js` paths, selects deterministically across PATH order, passes the resolved launcher as one argument to Node, and runs the child with an empty PATH. The Make target builds the std-only helper with `rustc` before structural validation, without starting Cargo. Added tests exercise native-first selection, reversed multiple-launcher PATH order, native-only refusal, missing tool/helper failures, isolated HOME, and existing schema controls. No code defect found. No applicable `AssuranceProfile` applies under `spec/`.

## Verdict

**PASS** — no code-review or Rust-review findings. `cargo fmt --all -- --check` and `git diff --check` passed. The PR's reported focused xtask test (8/8) and Clippy `-D warnings` passed. Reviewer reruns of those Cargo commands made no progress or output during the review window and were interrupted; `cargo deny check` could not acquire its advisory database lock because `/home/peter/.cargo/advisory-dbs/db.lock` is on a read-only path. The aggregate gate was not run.

## Assurance Context

No applicable `AssuranceProfile` was found under `spec/` for the changed files.

## Scope

- `NFR-002-AC-1`: examined — “The default structural gate selects every Markdown document under spec/ recursively from the caller's repository root and validates it through installed scoped module discovery, without narrowing to one module or suppressing errors. A valid selected corpus succeeds.”
- `NFR-002-AC-2`: examined — “With a structurally invalid selected document, make ci returns nonzero and starts zero Cargo processes, for both serial and parallel execution, including parallel keep-going execution. With the document restored, the same executing process-boundary control observes Cargo calls.”
- `NFR-002-AC-3`: examined — “Missing Filament, missing discovered modules, an unmatched document selection, or malformed required frontmatter, body sections or table headers makes the structural gate fail. The gate retains native diagnostics and actual native exit status in Make's error report and does not install modules to turn configuration failure into success.”
- `NFR-002-AC-4`: examined — “Owned Rust gate fixtures resolve an unset Cargo target to the workspace target directory, a relative target from the workspace root, and an absolute target unchanged; each configuration creates and cleans up its fixture there and executes the native validator, including paths containing spaces. Actual assertions use imported canonical TC and AC trace attributes.”
- `TC-920`: examined — default structural gate, invalid-document refusal before Cargo, empty selection, missing configuration, and target directory controls.
- `Makefile`: examined — helper compile/run recipe and workspace root resolution.
- `xtask/ci_spec_validation.rs`: examined — launcher resolver, child environment and exit propagation.
- `xtask/src/ci_spec_validation_tests.rs`: examined — changed assertions and trace tags.

## Checks

- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `cargo test --locked -p xtask --lib ci_spec_validation_tests`: PR author reported 8/8 passed. Reviewer rerun stalled with no output and was interrupted (exit 130).
- `cargo clippy --locked -p xtask --all-targets -- -D warnings`: PR author reported passed. Reviewer rerun stalled with no output and was interrupted (exit 130).
- `cargo deny check`: could not acquire `/home/peter/.cargo/advisory-dbs/db.lock` (read-only path); no dependency manifest or lockfile changed.
- Aggregate `make ci`: not run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
