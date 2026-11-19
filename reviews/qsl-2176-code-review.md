---
id: SR-2026111612172400000048
title: "QSL-2176 code review — PR 2967"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/assemble/tests.rs, fsl-semantics/tests/it/model_intake.rs; QSL-2176 PR 2967"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-056", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-103", type: references}
---

## Summary

Ticket QSL-2176, PR filament-spec-language#2967. The Rust review covered the changed intake paths and tests against FR-056's float, bare-member and multiplicity criteria and FR-103-AC-11 after W57. Removing `AssemblyCause::FloatingType` leaves two `fsl-replay` matches uncompilable; the required `make ci` gate stops at that compile error.

## Verdict

**FAIL** — the workspace gate cannot compile `fsl-replay` because two callers still match the removed enum variant.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Removing `AssemblyCause::FloatingType` leaves two exhaustive `fsl-replay` matches uncompilable; `make ci` fails with E0599 at both remaining references. | fsl-semantics/src/check/assemble.rs:347; fsl-replay/src/spine/refinement.rs:243; fsl-replay/src/spine.rs:1028 |

## Coverage

Rust lane: `rust-review` and `rust-style` applied to the changed Rust files. Reviewed FR-056-AC-10, AC-20, AC-43, AC-53 through AC-56, and FR-103-AC-11; FR-056-AC-57 through AC-59 were context only because checker-side IEEE behavior remains deferred behind #2854. The live intake tests assert exact Float32/Float64 types for fields, operation signatures, tuple positions and union payloads; bare Text/Rational/Decimal assembly; typed refusal for positive-lower unbounded multiplicity; and systems records under W57. No CI workflow files changed. No applicable AssuranceProfile was found for these paths. No new unsafe code, concurrency behavior, or public entry points were added.

`make ci` was run with `CARGO_HOME=/home/peter/.cache/cargo` and `CARGO_TARGET_DIR=/tmp/qsl2176-pr2967-target`; spec validation and the pin check began, then compilation stopped at the two E0599 errors in `fsl-replay`. A first attempt using the default Cargo home failed because the sandbox could not write `/home/peter/.cargo/registry/cache`; the writable cache override allowed the gate to proceed. No later CI lanes ran. The focused `fsl-semantics` intake run (`--features test-support --test it model_intake::`) passed 39 tests with 3 checker-side tests ignored as deferred behind #2854; Cargo initially ran offline without cached `alloca v0.4.0`, then passed after `cargo fetch --locked`. Direct rustfmt checking of all three changed Rust files passed. Workspace `cargo fmt --all -- --check` failed on formatting diffs outside the PR diff, including `fsl-analyze` and `fsl-replay/src/spine.rs`. Focused Clippy over `fsl-semantics` failed with 565 `result_large_err`/`large_enum_variant` diagnostics; none referenced the three changed files.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Removed both stale replay match arms in 17d35ec312; `cargo check -p fsl-replay --all-targets` passed. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |4-
            IllFormedBounds(TypeFormFault),
            /// An alias's resolution reaches that alias again.
            AliasCycle {
                /// The cycle's dependency edges, `(alias, alias it names)`.
                edges: Vec<(String, String)>,
            },
```
