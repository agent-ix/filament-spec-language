---
id: SR-3608003
title: "spec-review of PR 3608 FR-895 enabledness rules 5, 19 and 20"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-895, spec/test-cases/TC-2531 at 115a613f78ca072982125b5cb8e38c801c453d70"
review_set: subset
---

## Summary

Ticket: QSL-2998. `quire validate` passes on FR-895 and TC-2531. Rule 5 now defers to rule 19 consistently; AC-29 to AC-31 are in order.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Rule 20 makes a branch step `violated` when the join "is reached by the other branches finished causally before the step", but ADR-027 ST-7 cancels at the join step, which may fire later; a branch step between the join becoming enabled and the join step is a legitimate behaviour (CI-3). Proposed: "A step of a branch of a `parallel` under `outstanding cancel` SHALL make the verdict `violated`, cause `unexpected`, when an observation of a step after the `parallel` (which needs its join) causally precedes it" | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:182-186 |
| FND-002 | medium | Rule 19's "a predicate that cannot be evaluated SHALL leave the verdict `pending`" makes an obligation no observation can decide and gives a closed recording `open-history`; an engine that cannot evaluate a declared predicate should refuse (or settle `unsupported`) naming the join, not report the recording as open. The rule sets no bound on the joined labels it evaluates | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:176-181 |
| FND-003 | low | Rules 19 and 20 each hold two requirements (the reach condition and the unevaluable outcome; the `cancel` violation and the `continue` negation) against "each rule below is one requirement" | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:176-186 |

## Verdict

Rule 19's policy thresholds and FR-218 argument order are right. Rule 20 is unsound as written (FND-001) and rule 19's unevaluable
outcome needs a decision (FND-002).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Rule 27 ("an observation of a step after the `parallel` ... causally precedes it") does not say "of the same iteration", so a `parallel` in a `repeat` is cancelled by an earlier iteration's post-join step (SR-3608001 FND-004). Rule 25 specifies a refusal no admitted function can reach (SR-3608001 FND-005) | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:191-202 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | rules 27 to 29: cancel only after an observed post-join step or finish; join enabled is not cancelled (ADR-027 CI-3) |
| FND-002 | fixed | rules 24 to 26 refuse unsupported naming the join and the bound; no pending outcome |
| FND-003 | fixed | rules 19 to 29 one requirement each |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | rule 26 requires the same iteration of every repeat the parallel runs in; old rule 25 deleted and the package selection refusal stated as rule 25; one requirement per rule; AC-37, AC-38 in order. The code's nested-repeat gap against rule 26 is SR-3608001 FND-008 |

## New findings (disposition pass 3, final)

Head 16961b0ca6dbaf413ef426d9f87b5416e1fe2659. `quire validate` on FR-895 and TC-2531: exit 0. Rules 19 to 32 hold one requirement each;
rule 5 cites rules 19 to 23 correctly; rules 27, 30 and 31 cite rule 26 correctly; AC-37 to AC-39 are in order. Undetermined-not-counted
is monotone across prefixes (counts and ranks use only observations causally before the step, all present in its prefix), and the last
prefix equals batch in every probe.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | Rules 28 and 29 do not determine the iteration soundly, so rule 30's "a missed violation, never a false one" is false: rule 28's own-node rank miscounts when a cancelled branch skips an iteration, and rule 29's "most times any step before that other `repeat`" over-counts steps inside a `repeat` and under-counts steps split across `case`s (SR-3608001 FND-010 to FND-012). Proposed: take the iteration from a step that runs once in every iteration of that `repeat` (on its body's spine, not under a `case`, a branch or another `repeat`), and call every other case undetermined | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:205-215 |
| FND-006 | low | Rule 30 says an iteration is undetermined "when no step precedes the inner `repeat` in the body"; the code also leaves it undetermined when such steps exist but none was observed causally before the step (count 0) | spec/functional/FR-895-assess-a-recorded-protocol-execution-against-its-obligations.md:213-215 |

## Dispositions (round 3)

No round-2 spec finding was open; FND-004 stays fixed (rules 26 and 27 now split the same-iteration condition, one requirement each).

## Dispositions (round 4)

Head e2c54e6b4c369708c08bc1da650bee5f25ec44b9. `quire validate` on FR-895 and TC-2531: exit 0. Rules 28 to 31 match `repeat_iteration`
(main-sequence steps at or before the path; undetermined when none, unobserved, or disagreeing); one requirement each; rules 27 and 31
cite rule 26, rule 5 cites rules 19 to 23; AC-37 to AC-41 in order; TC-2531 scope reaches AC-41.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | rules 28 to 31 take the iteration only from once-per-iteration main-sequence steps; probes give no false cancel violation |
| FND-006 | fixed | rule 29 names all three undetermined cases: no such step, not observed at or before, counts disagree |

## Dispositions (trailing commit)

| Finding | Disposition |
| --- | --- |
| SR-3608001 FND-013 (`scan` takes all children when `on_path` is missing) | Fixed: `repeat_iteration` takes the position of `on_path` in the body's children and returns undetermined when it is missing, then reads `children[..=end]`. |
| SR-3608002 FND-004 (FND-012 untested) | Fixed: test `a_choice_before_an_inner_repeat_does_not_count_toward_the_iteration` (FR-895-AC-42): choice `a` then `b` before the inner repeat, no `P`; only the second iteration's `L` and `R` batch verdicts are asserted `satisfied`, under cancel and continue. The first iteration is not asserted (QSL-3009). |
