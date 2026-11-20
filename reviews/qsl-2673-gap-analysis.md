---
id: SR-2026112017124277723988804
title: "Repository gap analysis for FSL PR 3213"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository-wide specification, tests, and Rust source"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-642
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: references
---

## Summary

The production PR's timed evaluation and resumed-position behavior are represented in the current source and the focused criteria have tagged tests. The repository-wide assurance matrix remains incomplete: it reports 1,169 untagged criteria, 219 criteria tagged only by ignored tests, and 5,450 untracked test symbols. These findings are repository state, not evidence that PR 3213 introduced each gap.

## Verdict

FAIL under the repository-wide gap-analysis rule because the computed matrix has untagged and ignored-only criteria. The selected PR paths do not expose a production stub or an unowned behavior. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed repository matrix contains 1,169 untagged criteria; FR-643-AC-10 is among them and has no tagged test for the timed-session `assert_through` behavior. | FR-643-AC-10; spec/functional/FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md |
| FND-002 | high | The computed repository matrix contains 219 criteria whose only test tags are on ignored tests, so those obligations have no running test evidence. | computed matrix: tagged-by-ignored-test |
| FND-003 | medium | Coverage reconciliation reports 5,450 untracked symbols and 21,503 diagnostics, including stale trace IDs outside valid marker scopes; this prevents reliable traceability for those symbols. | quire coverage: untracked_symbols; diagnostics |

## Coverage

Reconciliation: quoin matrix (quoin 0.36.8, quire coverage 0.40.0); repository scope `/home/peter/dev/worktrees/qsl-2673`.

Matrix static statuses: 4,532 tagged; 1,169 untagged; 219 tagged-by-ignored-test; 104 method-without-symbol; 13 retired. Evidence status: 6,037 no run evidence. The 29 criteria under FR-642, FR-643 and FR-644 include one untagged row (FR-643-AC-10); the remaining 28 are tagged. Their evidence status is no run evidence because the repository evidence store has no bound runs.

`quire coverage` reports 5,450 untracked symbols, 21,503 diagnostics, and a Rust binding census of 12,620 candidates / 12,093 tagged / 11,624 bound. The dominant diagnostic is `trace-id-outside-marker` (21,092). A source scan found no `todo!` or `unimplemented!` in production Rust; the suspicious `todo!` matches were documentation examples. The changed tests contain behavioral assertions, and the changed production paths have owning requirements FR-283, FR-642, FR-643, and FR-644. No unowned behavior or hollow stub was found in the changed surface.

Reverse-gap inspection covered the changed timed evaluator, trace decoder, monitor admission/evaluation route, incremental-session state access, command stream position accounting, and associated tests. Semantic review was skipped; it was not requested. Plan completion: not assessed.

## Dispositions

### Round 1

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Arch's standing scope freeze parks owed-test sweeps; the missing FR-643-AC-10 timed-session coverage is not assigned to this PR. No ticket was created. |
| FND-002 | deferred | Arch's standing scope freeze parks owed-test sweeps; repository-wide ignored-only criteria are outside this PR's assigned scope. No ticket was created. |
| FND-003 | deferred | Arch's standing scope freeze parks traceability sweeps; repository-wide untracked symbols/diagnostics are outside this PR's assigned scope. No ticket was created. |
