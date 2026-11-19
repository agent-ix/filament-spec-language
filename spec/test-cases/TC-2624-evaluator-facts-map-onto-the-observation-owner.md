---
id: TC-2624
title: "Evaluator trace facts map onto the observation owner's contexts and views"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-944
    type: verifies
---
# TC-2624: Evaluator trace facts map onto the observation owner's contexts and views

## Description

Verify the quarter-tick interval mapping, its doubly-open strictness, the
one-sided eligibility comparisons and its exactness at the domain edge, trace admission,
partial facts, activation and repair mapping, and the completeness revision
rule, and the separate optional activation history. Scope: FR-944-AC-1 to
FR-944-AC-7 and FR-944-AC-14.

## Test Procedure

1. Compare `ClockCover`'s `definitely_before` and `within_bound` with the owner's `relation_to` on quarter-tick intervals for readings 0 to 6 and bounds 0 to 6, under `Counter { uncertainty: 1 }` and `Exact`.
2. Admit FR-501-AC-13's trace through `filament_observation::admit` and read the atoms' partial facts.
3. Derive FR-524-AC-27 to AC-29's activation views and FR-526-AC-2's repair plan.
4. Apply FR-502-AC-2's cases through `completeness::revise`.
5. Map FR-944-AC-5's request `(0, 1]` and acks `[3, 4)` and `[2, 3)` to quarter ticks, read `within_bound(3, ...)` through `relation_to`, and run `holds(req) implies eventually[0,3] holds(ack)` over each.
6. Decide FR-944-AC-6's `at_least` and `at_most` cases on the scaled difference's endpoints and compare each with `ClockCover`'s eligibility.
7. Map FR-944-AC-7's maximal readings and uncertainty and its extreme bounds, and compare `within_bound` and `definitely_before` with `ClockCover`'s.
8. Build FR-944-AC-14's activation requests (no instance closed, open, with an incomplete trigger record, with `Missing` and with `Refused` evidence; one instance without and with captures `a` and `b` beside a false-guard delivery; one instance with a capture faulted `Missing`; two instances with equal captures), admit each, derive its closure, completeness (only for admitted, complete trigger evidence) and capture views over its own `Context`, and derive each scope's activation, recording every `derive` call's binding identity.

Test: `fsl-eval/tests/it/observation_owner.rs`.

## Expected Results

- Step 1: equal answers on every pair.
- Step 2: one record per position with its FR-106 identity; possibility sets and reasons of the Kleene values.
- Step 3: the evaluator's dispositions; exactly the superseded result invalidated.
- Step 4: the refusals and acceptances FR-502's ledger gives.
- Step 5: intervals `[1, 4]`, `[12, 15]` and `[8, 11]`; `Indeterminate` and `Indeterminate { reason: StraddlesBound }` for `[3, 4)`, `Completed(true)` for `[2, 3)`.
- Step 6: each eligible, ineligible and `Indeterminate` (`StraddlesBound`) decision FR-944-AC-6 states, equal to `ClockCover`'s; no unbounded owner interval.
- Step 7: the exact endpoints and scaled bounds FR-944-AC-7 states, and equal answers.
- Step 8: every request `Available` with `membership_complete` true; `Inactive`, then `Unknown` with evidence `Incomplete` (reported `Unknown(open)` twice, `Unknown(missing)`, `Unknown(refused)`) with no refusal; one request per instance and one empty request for the no-instance scope; the records, capture identities, values and order FR-944-AC-14 states, including `{"instance": X}` for `fsl-eval/instance` and `{"fault": "missing"}`; `Active` for both equal-capture instances with distinct binding, record and activation identities; the trace binding still `required`; no `derive` over the trace binding's identity.
