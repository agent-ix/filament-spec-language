---
id: SR-34920
title: "QSL-3492 code review of filament-spec-language#4060 (shared timed outcome layer)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@a5695763d68a7aba58b7f9edc110d7e9bde606bb; commits 39f5e9d275, 8e10bc95c0, f8175515a7, a5695763d6 and merge resolutions; fsl-eval/src/simulation/model/outcome.rs, fsl-analyze/src/model_check/digital.rs, fsl-analyze/src/zone_check/search.rs, fsl-analyze/src/zone_check/settle.rs, fsl-replay/src/certificate/zone_certificate.rs, fsl-replay/src/model_check/outcome.rs, fsl-analyze/tests/it/{analyze_timed_cli,zone_dense,zone_search}.rs"
review_set: subset
---

## Summary

Ticket: QSL-3492 (also QSL-3484, QSL-3485). Independent review with the rust-review lane (rust-style loaded). Read-only; no cargo was run, so every failure scenario below is a hand trace, unrun. The merged claim_instances commits belong to another lane and were not reviewed.

## Verdict

Not mergeable. FND-001 is a wrong refutation on EN-1 in the case the PR says it fixes (an undefined reading at a time-lock), for a lock reached after a delay. FND-002 is a wrong certified proof on EN-6 that the consolidation leaves in place. FND-003 is a route split the PR introduces.

Checked and clean: claim_start order (extent refusal, no initial state, no instance) against FSpec FR-416 "Outcomes decided before a run is read" and FR-125-AC-13; EN-1, EN-6 and the zone certificate checker all take the instance set through claim_start; one TimedVacuity mapping into each of UndecidedCause and ZoneUndecided; the zone_dense change from Holds to Undecided(NoInstance) is what FR-125-AC-7 requires (an empty universe settles inconclusive, never proved); the main merge at 70f583c809 dropped only the superseded second vacuity block.

Checker independence: FSL FR-921 and ADR-038 RL-7 govern the timed-product (EN-5) checker, which this PR does not touch. The zone checker falls under FSL FR-1051; claim_start lives in fsl_eval::simulation::model, which row B-6 trusts. The checker imports claim_start and TimedVacuity only; admitted_start and admitted_refutation are not shared with it, and no zone, DBM or product code is shared. No finding.

Author-reported items: (a) conflicting-identity checked by ModelSystem::new before no-initial-state: fine, an empty initial list has no member records to conflict; the only reorderable pair is requires-bound against conflicting-identity, which is ModelSystem::new's and not this PR's. (b) "both routes test divergence only": not accurate for the vacuity test (EN-1 and EN-6 liveness do read fairness); it is a code defect where fairness is not read (FND-002), not spec wording. FSL FR-232 and FR-240 say fair; the FSL FR-239 "Vacuity" bullet says only time-divergent and should be aligned with FR-240.

Paired and dependent code: EN-1 and EN-6 and the zone checker (changed); zone_check/liveness.rs and refine.rs (read for fairness, unchanged); analyze/state_graph_item.rs, certificate/state_graph.rs, EN-4 and EN-5 keep their own ordering and are not on claim_start.

Gates: not run by this review. The PR records it_monitor_protocol 151 passed, 39 failed on an earlier head 87e5252f5e, attributed to QSL-3314 and not run on main; the merge check lets a failure pass only when it also fails on main, so that comparison is still owed. The PR is a draft until #4041 and the claim_instances PR merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | EN-1 divergence test counts a Zeno trap reached after a delay as time-divergent, so an undefined reading on a time-locked state still refutes | fsl-analyze/src/model_check/digital.rs:1339-1363 |
| FND-002 | high | EN-6 reads no fairness for a TT-1 or TT-2 claim, so a subject with no fair time-divergent behaviour is certified proved where EN-1 settles no-admitted-behaviour | fsl-analyze/src/zone_check/search.rs:1581-1596 |
| FND-003 | medium | With fairness and no fair behaviour, EN-1 now settles an undefined reading no-admitted-behaviour while EN-6 still refutes it; no test covers the change | fsl-analyze/src/model_check/digital.rs:1014-1020 |

### FND-001

search_divergence makes the acceptance set from the target vertex of every delay edge, wherever the edge starts, so a component holding such a target and any zero-time loop is "divergent" with no delay inside it. The doc comment says "a cycle through a delay". The product path is right (its divergent flag is part of the vertex key); EN-6 uses a divergence clock. The function body is unchanged from main, but it is the evidence this PR hands to admitted_refutation and the QSL-3485 behaviour.

Hand trace, unrun. Operations block (phase 0 to 2), spin (phase 2 to 2), go (0 to 1); time invariant "when self.phase = 2 { x <= 1 ms }"; claim "always holds(mod_defined(c.phase) and c.phase <= 1)" with mod_defined from the new tests. block reaches (2, x=0), undefined. search_divergence: (2,0) -spin-> (2,0), -delay-> (2,1), (2,1) -spin-> (2,1). Component {(2,1)} has a self-loop and is the target of the delay edge, so diverges is true and EN-1 returns refuted undefined-evaluation. Time never passes 1 ms from phase 2, so it is a time-lock: expected proved on both routes (FSpec FR-416 "How claims read a time-lock"). With "and c.phase = 0" instead, EN-1 reports the undefined reading in place of the go violation.

Test: in analyze_timed_cli.rs, that model through partial_world's source with "spin" added to the document operations, asserting HoldsReduced from digital_on. Command: cargo test --locked -p fsl-analyze --test it_model_check -- analyze_timed_cli::. Fix direction: require a delay edge with both ends in one component.

### FND-002

The would-be proof is admitted by node_escape(root).diverges, which is divergence only; admits_fair runs only inside the liveness product. The certificate's admitted part proves divergence only. FSL FR-240 "Vacuity" requires a fair time-divergent cycle or a quiescent state and says FR-239 runs the same check before every TT-1 and TT-2 proof; FSL FR-232 "Vacuity" and FSpec FR-161-AC-15 say the same. Pre-existing on main; the PR routes this site through the shared layer without supplying fairness evidence.

Hand trace, unrun. phase 0 under "x <= 1 ms"; idle (0 to 0, resets x); go (0 to 2, resets x); phase 2 under "x <= 0 ms"; "always holds(c.phase != 1)" with "fair { weak Call::go }". go is enabled at every instant of every divergent run (the idle loop) and never taken, so no fair divergent run exists and no state is quiescent. EN-1: inconclusive no-admitted-behaviour. EN-6: proved zone-certified. The same missing premise lets a TT-1 violation that lies only on unfair runs refute on EN-6 while EN-1, which requires a fair cycle, does not.

### FND-003

On main EN-1 built the claim product first, so an undefined reading refuted before vacuity; the PR classifies vacuity first. EN-6 returns on an undefined reading as soon as time diverges, both in judge (TT-1) and in the liveness decide, before admits_fair. EN-1's new answer matches FSpec FR-161 (fairness restricts the admitted behaviours), but the two routes now disagree, the PR body names EN-1 only, and no added test exercises it. admitted_refutation also takes divergence only on both routes, so where some behaviour is fair, an undefined reading lying only on unfair runs still refutes.
