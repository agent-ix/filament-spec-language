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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | No test puts a cancel `parallel` inside a `repeat` (SR-3608001 FND-004) | fsl-analyze/tests/it/protocol_replay_mode.rs:1617-2140 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-32 (L, R under cancel; concurrent R), AC-33 (nested), AC-34 (label bound in library, batch and incremental), AC-35 (contradiction), AC-36 (package route) tests |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | No test puts a cancel `parallel` in a `repeat` nested in another `repeat`, the case rule 26 names ("every `repeat`") and SR-3608001 FND-008 breaks | fsl-analyze/tests/it/protocol_replay_mode.rs:2140-2340 |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | AC-37 (cancel parallel in a repeat body, batch and every prefix) and AC-38 (repeat inside a cancelling branch) tests |

## Dispositions (fix round 3)

Closed by FR-895-AC-39; see `qsl-2998-code-review.md` "Dispositions (fix round 3)".
