---
id: SR-2026111919340478491681410
title: "Gap analysis — optional operation parameters"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository-wide matrix plus changed optional-parameter paths"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

The repository-wide `quoin matrix` reports 1,216 untagged criteria and 257 criteria tagged only by ignored tests, with no stored run evidence for 5,962 criteria. In the changed feature, the local criteria FR-056-AC-65 and FR-103-AC-12 are untagged because the new tests carry `FSpec-*` identifiers rather than the FSL criterion IDs; the optional-operation test requirements therefore have no matrix-bound test evidence.

## Verdict

**FAIL** — the repository matrix contains untagged criteria, including the two local acceptance criteria directly addressed by this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-056-AC-65 has no test bound to its FSL criterion ID | FR-056-AC-65, fsl-semantics/src/model/intake.rs::tests::operation_parameter_presence_is_required_and_rejects_unknown_values |
| FND-002 | high | FR-103-AC-12 has no test bound to its FSL criterion ID | FR-103-AC-12, fsl-semantics/src/check/state_model.rs::tests::dispatch_calls_may_omit_only_a_trailing_optional_parameter |

## Coverage

- Reconciliation: `quoin matrix --repo /home/peter/dev/worktrees/qsl-1983 --json` (quoin 0.36.8; quire 0.52.0).
- Plan completion: not assessed.
- Criteria: tagged 4,372; untagged 1,216; tagged-by-ignored-test 257; method-without-symbol 104; retired 13.
- Evidence: no run evidence for all 5,962 criteria; bound 0; stale 0; suspect 0; undischarged 0.
- `quire coverage --scope /home/peter/dev/worktrees/qsl-1983 --json`: 4,803 untracked trace symbols and 20,783 diagnostics. The four new feature tests use external `FSpec-*` ids; the matrix does not bind them to the two local FSL criteria.
- Reverse-gap/stub pass: inspected all changed production and test paths, including the new optional-parameter branches. No source/test stub or unrelated untraced behavior was found in those changed paths. The audit did not independently inventory every unchanged repository behavior.
- Semantic review: skipped (not opted in).

## Scope

- `FR-056-AC-65` — examined: "Over FSpec TC-2054's `test/refunds` package, intake admits `Order/note` with `firstApprover` `required` and `secondApprover` `optional`, and the type environment gives `firstApprover` `Reference<R::Staff>` and `secondApprover` `Option<Reference<R::Staff>>`. With `secondApprover`'s `presence` `\"maybe\"`, `null` or absent, intake refuses `invalid_model_binding`/`malformed-declaration` at `Order/note`, path `Operation.params[1].presence`; with `secondApprover` typed by the option value type `OptStaff` and presence `optional` it refuses the same pair at the same path; with `OptStaff` and presence `required` it admits `secondApprover` as `Option<Reference<R::Staff>>`."
- `FR-103-AC-12` — examined: optional operation parameter clause checks, presence obligations, `deref` type refusal, implicit trailing omission lowering, and invalid arity/type refusals.
- The other 1,214 untagged and 257 ignored-only criteria are counted in the matrix rollup; this PR review did not inspect their individual requirement/test semantics.
