---
id: SR-2026111612172400000031
title: "Code review — QSL-2019 FSL command extraction route"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #2891 @1ff2b2beaa36076def73d9d596154804a65cc6bf; fsl-command/Cargo.toml, fsl-command/src/command.rs, tests/it/request_surface_s1.rs; FR-662-AC-3, FR-031-AC-6, TC-1744"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-662", type: references }
  - { target: "ix://agent-ix/filament-spec-language/FR-031", type: references }
---

## Summary

Reviewed PR #2891's default feature, command routing, and activated FR-662-AC-3 test at the specified head. The new extraction route misclassifies a request carrying `call` instead of `clause`; a changed helper also fails rustfmt 1.9 formatting.

## Verdict

**CONDITIONAL** — one medium request-contract defect and one low formatting finding; no high-severity finding. `cargo deny check` passed (with source-not-allowed warnings); package clippy did not reach a clean result because `fsl-command` and its dependencies have existing warnings outside this diff. The focused tests are reported passed in the PR description and were not rerun.

## Review Scope

- `fsl-command/Cargo.toml`: default `quire-extraction` feature.
- `fsl-command/src/command.rs`: `run_configured`, `run_extracted_clause_bytes`, and `read_clause_run_request_bytes`.
- `tests/it/request_surface_s1.rs`: `the_run_command_reaches_markdown_extraction_by_default`.
- FR-662-AC-3, FR-031-AC-6's extraction mode refusal, and TC-1744's two test procedure steps.
- Focused tests are reported passed in the PR description; no aggregate or package-wide tests were run.

## Gates

- `cargo fmt --all -- --check`: failed on numerous pre-existing workspace formatting differences and on the new closure at `command.rs:1584-1588` (FND-002).
- `cargo clippy --locked -p fsl-command --all-targets --no-deps -- -D warnings`: failed on existing findings in `wire.rs`, `items.rs`, and unchanged regions of `command.rs`; no diagnostic points into this PR diff. The dependency-inclusive clippy run stopped earlier on 275 `fsl-forms` `result_large_err` diagnostics.
- `cargo deny check` with the worktree cache: exit 0; reported existing `source-not-allowed` warnings for git dependencies.
- No aggregate or package-wide tests were run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | When an extraction request carries `call` instead of `clause`, the new `command::run` route invokes `read_clause_run_request_bytes`; the reader looks up `clause` first and returns the generic `ClauseRequest` missing-clause refusal before it can classify the extraction mode. FR-031-AC-6 requires `extraction-requires-run` before reading dependent files. | fsl-command/src/command.rs:1582-1583, 1858-1864; spec/functional/FR-031-run-extracted-native-source.md:132-134 |
| FND-002 | low | The changed `run_extracted_clause_bytes` closure is not formatted by the repository's Rust 1.98 toolchain rustfmt 1.9 (`rustfmt --check --edition 2021 --config skip_children=true fsl-command/src/command.rs` reports lines 1584-1588). | fsl-command/src/command.rs:1584-1588 |
