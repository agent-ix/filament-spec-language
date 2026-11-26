---
id: SR-5590835
title: "Code review (rust lane) of QSL-3506 changes in PR 4037"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-analyze, fsl-eval, fsl-replay (QSL-3506 changes of PR 4037)"
review_set: subset
---

## Summary

Ticket: QSL-3506. Reviewed the QSL-3506 changes of PR 4037 at af67c1c708 (diff against the merge of main 2fe67cd667 and titan/qsl-3478-zone-possible 2c318d6f62). The family-constant fixes and the checker-decision change are correct; a pre-existing hole in the same LU computation still lets EN-6 prove a false property that the checker accepts.

## Verdict

**FAIL** on FND-001 (high, pre-existing, outside the PR's claim). No defect found in the changes themselves above medium. Mutation check at head: with the family walk removed from fsl-replay timed::clock_constraints and fsl-eval compared_constraints, bounds_below_a_quantifier_family_constant_are_rejected and the three family_constants tests fail (4 of 4); reverted. The state-graph refutation now comes only from the checker's decision. No other reader found that takes a rejection rule as a refutation. A nested quantifier family is refused, as the PR says. Cargo gates were not rerun; the PR's reported results are the evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | LU bounds are collected per clock field from the constraints that name that field; a collection write that carries a clock from another field (self.ts = pre(self.us)) is not followed, so us.x has no L or U although its value is later compared as ts.x >= 5. The producer's scaled_bounds (fsl-analyze zone_check/search.rs:2750) does the same. Reproduced at head, nothing forged: dense model, us = [Timer], ts = []; alpha (y <= 0 ms) and go (y >= 5 ms) both enter phase 1 and reset y; copy writes self.ts = pre(self.us) into phase 2; work (forall e in ts: e.x >= 5 ms) enters phase 3; invariant y <= 1 ms in phases 1 and 2. 'always not phase 3' settles Holds under EN-6, check_zone_certificate returns Accepted with lower [5, None, 5], and replay_model_trace of go after 5, copy, work returns Reproduced Truth(false). Naming the late operation first (exploration order swapped) settles Violated. A false property is proved and certified. Not introduced by this PR (read from code: main's two readers have no carry step either); needs its own ticket if not fixed here. Probe: gates-cons-checkers/sr-3506-probe-carried-clock.rs, log sr-3506-probe5.log. | fsl-replay/src/certificate/zone_certificate.rs:4515-4560 |
| FND-002 | medium | The checker's lasso replay closes a loop with fsl-eval's TimedSystem::max_constant (fsl-replay model_check/replay/timed.rs:508), the same function the producers use for their concretisation ceiling (zone_check/liveness.rs:1473, zone_check/refine/live.rs:1634); it now rests on fsl-eval compared_constraints, not on the checker walk fsl-replay timed::clock_constraints this PR adds as 'apart from every engine's'. source_clock_maxima is likewise shared by the task product and the TaskAutomaton the task certificate checker reads. A constant one walk misses is missed by producer and checker together, which is the family defect this PR fixes: the lasso entered below the family constant was reproduced. The sharing predates the PR. | fsl-eval/src/simulation/model/timed/over_time.rs:442-463 |
| FND-003 | low | fsl-replay now holds two walks of the model's clock constants: this one, recursive over families, and the inline chain in model_check/digital_state_graph.rs grid (lines 185-220), which reads one family level. A change to what carries a constant must be made in both. grid gives no wrong result today because a nested quantifier family is refused ClockReadOutsideConstraint (confirmed by probe), but it should call this function. | fsl-replay/src/timed.rs:12-37 |
| FND-004 | low | No test fails if the producer goes back to reading a refutation from the rejection rule. a_coverage_rejection_is_no_state_graph_decision calls only the checker, and its forged case rejects in the node coverage pass before state_graph runs, so it also gives decision None under the previous checker code; it passed with the family walk mutated and asserts nothing about run_timed. The producer half of findings 5 and 7 is unguarded. | fsl-analyze/src/analyze/state_graph_item.rs:1136-1150 |

## Examined

- examined: fsl-replay/src/certificate/zone_certificate.rs
- examined: fsl-replay/src/certificate/zone_certificate/state_graph.rs
- examined: fsl-replay/src/timed.rs
- examined: fsl-replay/src/timed/explore.rs
- examined: fsl-replay/src/probability_certificate/document/timed.rs
- examined: fsl-replay/src/lib.rs
- examined: fsl-eval/src/simulation/model/timed/over_time.rs
- examined: fsl-eval/src/simulation/model/task_product.rs
- examined: fsl-analyze/src/analyze/state_graph_item.rs
- examined: fsl-analyze/tests/it/family_constants.rs
- examined: fsl-analyze/tests/it/zone_certificate.rs
- examined: fsl-analyze/tests/it/timed_state_graph.rs
- examined: fsl-analyze/tests/it/timed_world.rs
- examined: fsl-analyze/tests/it/zone_replay.rs
- examined: fsl-analyze/tests/it/model_check_binary.rs
- context_only: fsl-analyze/src/zone_check/search.rs
- context_only: fsl-replay/src/model_check/digital_state_graph.rs
- context_only: fsl-replay/src/model_check/replay/timed.rs
- context_only: fsl-replay/src/model_check/graph_replay.rs
- context_only: fsl-analyze/src/analyze/exact/measure.rs
