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
partial facts, activation mapping, the ledger's mapping onto the owner's
completeness revision check, and the separate optional activation history. Scope: FR-944-AC-1 to
FR-944-AC-7, FR-944-AC-14 and FR-944-AC-16.

## Test Procedure

1. Compare `ClockCover`'s `definitely_before` and `within_bound` with the owner's `relation_to` on quarter-tick intervals for readings 0 to 6 and bounds 0 to 6, under `Counter { uncertainty: 1 }` and `Exact`.
2. Admit FR-501-AC-13's trace through `filament_observation::admit` and read the atoms' partial facts.
3. Derive FR-524-AC-27 to AC-29's activation views.
4. Apply FR-944-AC-4's requests through the ledger under `ms` after a retained `Complete { through: 100 }` covering `p1` stamped 80: `Complete { through: 60 }`, `Incomplete`, a new identity stamped 80, a position equal to `p1` in content, stamp, order key, interval and arrival under a new identity, `p1` itself, `Complete { through: 100 }` and `{ through: 140 }`; then a covered position omitted by one request and re-supplied by the next at or before 100. Record the `check_revision` arguments and answer of each.
5. Map FR-944-AC-5's request `(0, 1]` and acks `[3, 4)` and `[2, 3)` to quarter ticks, read `within_bound(3, ...)` through `relation_to`, and run `holds(req) implies eventually[0,3] holds(ack)` over each.
6. Decide FR-944-AC-6's `at_least` and `at_most` cases on the scaled difference's endpoints and compare each with `ClockCover`'s eligibility.
7. Map FR-944-AC-7's maximal readings and uncertainty and its extreme bounds, and compare `within_bound` and `definitely_before` with `ClockCover`'s.
8. Build FR-944-AC-14's activation requests (no instance closed, open, with an incomplete trigger record, with `Missing` and with `Refused` evidence; one instance without and with captures `a` and `b` beside a false-guard delivery; one instance with a capture faulted `Missing`; two instances with equal captures), admit each, derive its closure, completeness (only for admitted, complete trigger evidence) and capture views over its own `Context`, and derive each scope's activation, recording every `derive` call's binding identity.
9. Apply FR-944-AC-16's requests: after an event-position `Incomplete` covering stamps 1 to 3, a new position stamped 0, with a watermark not below the retained one, with `Incomplete` and with `Complete { through: 5 }`; after a fixed-sample record-sample `Incomplete`, and again after `Complete { through: 5 }`, covering index 1 with fingerprint F, index 1 with F and with G; for a timestamped clause over record samples (a `Timestamped` record-sample trace, FR-993 Inputs, under `ms`), after a retained `Complete { through: 5 }` covering index 1 at stamp 1, index 1 at stamp 2 with changed and with the same content; as ledger tests, a held position with F listed with none, one with none listed with F, and one with none listed with none. Run every case a trace can express, and step 4's requests, in batch and in carried evaluation, the carried state passed through `to_bytes` and `from_bytes` between requests.

Test: `fsl-eval/tests/it/observation_owner.rs` and the `fsl-eval` ledger tests; step 9's carried evaluation in `fsl-replay`'s spine clause temporal tests.

## Expected Results

- Step 1: equal answers on every pair.
- Step 2: one record per position with its FR-106 identity; possibility sets and reasons of the Kleene values.
- Step 3: the evaluator's dispositions.
- Step 4: prior upper bound `Some(100)`, state `Complete`; `BoundaryRegressed`, `CompleteRegressed` at successor upper bound 100, `UncoveredMember` for both new identities, each reported as FR-502's `Completeness` contradiction with the ledger unchanged; `p1`, `{ through: 100 }` and `{ through: 140 }` accepted; the omitted position accepted when re-supplied and listed as covered throughout.
- Step 5: intervals `[1, 4]`, `[12, 15]` and `[8, 11]`; `Indeterminate` and `Indeterminate { reason: StraddlesBound }` for `[3, 4)`, `Completed(true)` for `[2, 3)`.
- Step 6: each eligible, ineligible and `Indeterminate` (`StraddlesBound`) decision FR-944-AC-6 states, equal to `ClockCover`'s; no unbounded owner interval.
- Step 7: the exact endpoints and scaled bounds FR-944-AC-7 states, and equal answers.
- Step 8: every request `Available` with `membership_complete` true; the closed scope's closure view `Closed` without refusal, its watermark the range's exclusive end; `Inactive`, then `Unknown` with evidence `Incomplete` (reported `Unknown(open)` twice, `Unknown(missing)`, `Unknown(refused)`) with no refusal; one request per instance and one empty request for the no-instance scope; the records, capture identities, values and order FR-944-AC-14 states, including `{"instance": X}` for `fsl-eval/instance` and `{"fault": "missing"}`; `Active` for both equal-capture instances with distinct binding, record and activation identities; the trace binding still `required`; no `derive` over the trace binding's identity.
- Step 9: prior upper bound `None` after `Incomplete` and both stamp-0 requests accepted; F accepted and G refused `MemberObservationChanged`, reported as `Completeness` naming the position without a fingerprint, after either retained state; index 1 at stamp 2 refused `MemberObservationChanged` with changed content and accepted as the same member with the same content, the returned ledger listing index 1 once, at stamp 1 with fingerprint F; both fingerprint-present-versus-absent cases refused `MemberObservationChanged` and the absent-absent case accepted; batch and carried answers and ledgers equal on every case run in both.
