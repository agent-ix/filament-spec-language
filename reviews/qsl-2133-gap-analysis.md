---
id: SR-2026111608414702
title: "Gap analysis of filament-spec-language PR 2741 (QSL-2133)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-641, FR-645, FR-330-AC-16 against tests/it/run_wire.rs, fsl-analyze/tests/it/model_trace_run.rs, fsl-replay/src/spine/request.rs tests, fsl-replay/src/spine/clause/tests at 617dafe9585eacb4289256ca6fc0f2f9de7d76e1"
review_set: subset
evaluated_revision: "617dafe9585eacb4289256ca6fc0f2f9de7d76e1"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-645
    type: reviews
---

## Summary

Ticket: QSL-2133. This analysis read `quoin matrix --json` (quoin 0.29.4) for
FR-641, FR-645 and FR-330-AC-16, and then read each tagged test against its
AC. Where the ticket's older test locations disagree with the merged spec's TC
mapping, the TC mapping is used. FR-641-AC-1..5, FR-645-AC-1, -2, -3, -5 and
FR-330-AC-16 are `tagged`. FR-645-AC-4 is `tagged-by-ignored-test` until
QSL-2254 (lead ruling), and so is the ticket's AC-7. No production code in the
diff lacks an owning requirement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-641-AC-1's test neither compares the result with the typed `ClauseRunRequest` byte for byte nor asserts FR-501-AC-4's outcome | tests/it/run_wire.rs:279-299 |
| FND-002 | medium | FR-641-AC-2 has no fixed-sample `progressed` case and asserts no FR-504-AC-1/AC-2 outcome | tests/it/run_wire.rs:310-364 |
| FND-003 | medium | FR-330-AC-16's test asserts `decisive.position >= 1`, not position 2, and does not check the state sequence 0, 1, 2 or the atom read there | fsl-analyze/tests/it/model_trace_run.rs:95-112 |

### FND-001

FR-641-AC-1 says the `timestamped` request "reports the outcome FR-501-AC-4
states for the same input built as a typed `ClauseRunRequest`,
byte-identical." `a_timestamped_trace_runs_from_a_request_file_and_returns_the_ledger`
checks only stage `evaluate`, exit 0 and the ledger's `format`. A wrong
verdict, or a wire-to-typed mapping that drops `cover` or `order_key`, passes.
Fix: build the same trace as a typed request, write both results, and
`assert_eq!` the bytes. Also assert FR-501-AC-4's verdict.

### FND-002

FR-641-AC-2 covers "a `progressed` trace of an event-position clause and of a
fixed-sample clause". The only progressed wire test is event-position, and it
asserts the ledger and the AC-3 regression, not FR-504-AC-1/AC-2's outcomes.
The ticket's AC-2 ("advances time through silence and settles a deadline") is
not asserted anywhere on the wire. Fix: add the fixed-sample case, and assert
both FR-504 outcomes, including the deadline that settles through silence.

### FND-003

FR-330-AC-16 asks for "a `decisive` of `position` 2, the atom evaluated over
the reconstructed state of position 2" over `c.value` 0, then loop states 1
and 2. The test uses whatever lasso EN-1 finds and asserts only
`position >= 1`. A decisive record off by one loop state, or one that never
evaluated the atom (code-review FND-004), passes. Fix: build the 0 -> 1 -> 2
lasso explicitly, and assert `position == Some(2)` and the decisive value path.

## Verdict

Partial coverage. Every AC in scope carries a tag, but three tagged tests are
weaker than their ACs: FR-641-AC-1, FR-641-AC-2 and FR-330-AC-16. The trace
mis-tags (FSpec-FR-541-AC-3 and FR-645-AC-3) and the model-trace
`Pending`-for-empty-loop test are recorded in the companion code review
(SR-2026111608414701, FND-003, FND-009, FND-010).
