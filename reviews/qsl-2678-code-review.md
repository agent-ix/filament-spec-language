---
id: SR-2026112013172122111220103
title: "Code review — QSL-2678 PR #3199 test corrections"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze/tests/it/race_freedom.rs, fsl-forms/tests/it/clause_text.rs, fsl-forms/tests/it/sum_case_forms.rs"
review_set: subset
evaluated_revision: "11b1c8ed60c2dd9410faec8524f37f073b09025c"
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Reviewed PR #3199 at head `11b1c8e` against FSL main `1af5220`, including its three test-only changes and their owning criteria. The fixture corrections match the requirements and retain behavioral checks; no code defect was found.

## Verdict

**PASS** — the five race-freedom observations now list the individual decision fact, the parser test still checks the required syntax refusal, and the clause-text comparison now uses identical wrappers.

## Scope and skill provenance

The diff changes only `fsl-analyze/tests/it/race_freedom.rs`, `fsl-forms/tests/it/clause_text.rs`, and `fsl-forms/tests/it/sum_case_forms.rs`. The code-review and Rust-review lanes ran by inspection. No applicable AssuranceProfile was found. There are no source or workflow changes, so the separate production-code gap-analysis lane was not applicable. No code or test files were edited during review.

## Rust review

| Check | Result |
| --- | --- |
| Repository conventions and traceability | Read `AGENTS.md`, `CLAUDE.md`, and Rust style/testing guidance. Existing traced test functions remain bound to their FR/TC criteria. |
| Behavior and assertion strength | Race tests still exercise source analysis and replay. Clause-text parity still compares the parsed expression to the same unit-clause production. The malformed-case test still asserts `InvalidSyntax`, the offending `}`, and the S2 `RecoveringCst` refusal. |
| Concrete failure scenario | A `visible` composite such as `bLy and true` discloses neither operand individually; listing `bLy` matches FR-218-AC-34 and TC-1056 rows 31/33. |
| Public API, safety, concurrency, wire and persistence | Not applicable; this PR changes tests only. |
| Integrity and gates | `git diff --check origin/main...HEAD` passed. No CI workflow or gate was changed. The required scoped matrix shows all six affected criteria tagged; the evidence store reports no run evidence. |

## Coverage

The repository matrix was run from this single FSL worktree root. The affected criteria FR-218-AC-34, FR-224-AC-1/2/4, FR-227-AC-3, FR-103-AC-8, and FR-313-AC-3 are tagged; the matrix evidence store has no run evidence. This is not a full repository gap-analysis. The ticket's latest comment records the targeted race-freedom run at 37 passed and the two corrected fsl-forms tests passing. I did not independently rerun them: `locked-build.sh` reported that the user systemd bus was unavailable and that its fallback had no cgroup memory cap, so I stopped the fsl-analyze run rather than run without the required cap. No aggregate gate was run under the owner's targeted-check policy.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Findings data

```yaml
clean: true
scope:
  - id: FR-218-AC-34
    path: spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md
    role: examined
    excerpt: A bare eligible decision fact checks and is individually visible, and a composite expression containing a literal or capture checks without making its operands individually visible.
  - id: TC-1056-row-31
    path: spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md
    role: examined
    excerpt: A composite visible entry such as true and got.ready discloses none of its operands, so the guard read of got.ready is refused as unlisted.
  - id: FR-224-AC-1
    path: spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md
    role: examined
    excerpt: ADR-025 §10 message-passing shape with release/acquire under ra carries one RaceFreedom item and settles proved.
  - id: FR-224-AC-2
    path: spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md
    role: examined
    excerpt: Relaxed flag accesses under ra and tso refute the item with the named data race counterexample.
  - id: FR-224-AC-4
    path: spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md
    role: examined
    excerpt: The authored claim retains its Formula counterexample and verdict when race-freedom is checked.
  - id: FR-227-AC-3
    path: spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md
    role: examined
    excerpt: Replay of a race counterexample recomputes the race over the replayed prefix and refuses forged race data.
  - id: FR-103-AC-8
    path: spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md
    role: examined
    excerpt: Clause text parses to the expression produced by a unit pre/post clause body, and malformed or multi-declaration text refuses.
  - id: TC-993
    path: spec/test-cases/TC-993-model-operations-keep-and-check-requires-and-ensures.md
    role: context_only
    excerpt: K2 operation-clause text preserves its source identity; a requires pre-read refuses, and an ensures clause reading pre(self.placed) checks.
  - id: FR-313-AC-3
    path: spec/functional/FR-313-build-union-declaration-and-case-forms.md
    role: examined
    excerpt: An unparenthesized record value is not parsed as a case scrutinee; the parenthesized form parses and protocol choice cases remain distinct.
  - id: TC-816
    path: spec/test-cases/TC-816-s1-parses-a-case-scrutinee-without-a-top-level-record-and-keeps-protoc.md
    role: context_only
    excerpt: The unparenthesized record case fails inside the arm list with a parser diagnostic; only required syntax and form refusal behavior are asserted.
findings: []
```
