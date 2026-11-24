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


# Follow-up review (reviewed at e2c54e6b and the trailing commit)

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

## Dispositions (round 3)

Head 16961b0ca6dbaf413ef426d9f87b5416e1fe2659. No new gap-analysis finding: the missing coverage for SR-3608001 FND-010 to FND-012
(a skipped cancelled branch in a `repeat`, a `case` or a second inner `repeat` before or after the inner `repeat`) goes with those fixes.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | AC-39 test a_cancel_in_nested_repeats_cancels_per_iteration_of_each: legit two outer iterations (batch, every prefix, monotone, last equals batch), the violated R, and the undetermined no-P case |

## New findings (disposition pass 4, final)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | No test covers SR-3608001 FND-012 (steps under a `case` before the inner `repeat`). A test need not hit QSL-3009: over the probe C3 shape (choice a then b before the inner `repeat`, no `P`) assert only the second outer iteration's `L` and `R` batch verdicts `satisfied`, as under `continue`; the round-3 code gave them `violated`. Leave out the first iteration's verdicts and the all-obligation monotonicity helper, which QSL-3009 breaks | fsl-analyze/tests/it/protocol_replay_mode.rs:2540-2640 |

## Dispositions (round 4)

No open finding from earlier rounds. AC-40 (`a_skipped_branch_step_does_not_shift_the_iteration`) and AC-41
(`a_step_inside_an_inner_repeat_does_not_count_toward_the_iteration`) back SR-3608001 FND-010 and FND-011.
## Dispositions (fix round 4)

No new finding in this pass; the iteration derivation changed as stated in the code-review dispositions (FND-010, FND-011, FND-012).
