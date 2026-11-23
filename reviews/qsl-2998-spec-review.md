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

## Dispositions (fix round 2)

FND-004: rule 26 (was 27) adds "in the same iteration of every `repeat` the `parallel` runs in"; the old rule 25 is deleted and rules renumbered; rule 24 names the `stage_limit_exceeded` refusal; rule 25 states the package selection refusal; AC-37 and AC-38 test the iteration condition.
