---
id: SR-2026111518064800000006
title: "Code review — QSL-2262 native Quire launcher gate"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2262; PR #2727; Makefile, xtask/src/ci_spec_validation_tests.rs, spec/non-functional/NFR-002-reproduce-native-builds.md, spec/non-functional/NFR-005-rust-verification-paths.md, spec/test-cases/TC-920-native-spec-validation-precedes-ci-cargo.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/NFR-002", type: references }
---

## Summary

Reviewed the frozen QSL-2262 PR #2727 against `origin/main` at `9462d4b41`, including the Make target, its regression test, NFR-002 and TC-920. Resolving Node before clearing `PATH` and invoking it with the Quire launcher is sound; the new Rust test is missing the repository's trace attribute.

## Verdict

**CONDITIONAL** — one low-severity traceability finding; the targeted regression and formatting checks pass. The aggregate gate was not run in this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new regression test has only a provenance comment and no canonical `#[trace]` binding, so its evidence is absent from the repository traceability model. | xtask/src/ci_spec_validation_tests.rs:96-99 |

## Review notes

The Makefile resolves both `quire` and `node` while the caller's `PATH` is available, then invokes the resolved Node executable with the Quire launcher and arguments while keeping the child `PATH` empty. This preserves the gate's plugin-discovery isolation. The new test compiles a Rust recorder and verifies the actual Make invocation, empty child `PATH`, launcher path and arguments. The assertions are behavioral and the focused test passed. No changed API, dependency, stub or unowned behavior was found.

Verification: `cargo test --locked -p xtask --lib spec_validation_uses_node_without_exposing_quoin_to_path -- --nocapture` passed (1/1); `cargo fmt --all -- --check`, `cargo clippy --locked -p xtask --lib --tests -- -D warnings`, and `CARGO_HOME=/tmp/qsl-2262-cargo-home cargo deny check` passed. The aggregate gate was not run in this review.
