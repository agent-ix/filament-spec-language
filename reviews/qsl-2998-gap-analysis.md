---
id: SR-3608002
title: "gap-analysis of PR 3608 FR-895 AC-29 to AC-31"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-895 rules 5, 19, 20 and AC-29 to AC-31; fsl-analyze replay-mode tests at 115a613f78ca072982125b5cb8e38c801c453d70"
review_set: subset
---

## Summary

Ticket: QSL-2998. Plan completion: not assessed. `quoin matrix` not run; bindings read from trace attributes. AC-29, AC-30 and AC-31 each
have a traced test asserting batch verdicts, prefix sequences, monotonicity and last prefix equal to the item.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers a branch step taken after the join became reachable but before any step that needs the join (the FND-001 shape of SR-3608001), nor the `Unevaluable` predicate path (package route, more than 12 joined labels) | fsl-analyze/tests/it/protocol_replay_mode.rs:1617-1711 |

## Verdict

Coverage of the stated ACs is complete and exact; the missing cases are the ones where the implementation is wrong or silent.

## Dispositions (fix round 1)

Closed by FR-895-AC-32 to AC-36 and the tests named in `qsl-2998-code-review.md` "Dispositions (fix round 1)". The failed-evaluation refusal has no reachable fixture.
