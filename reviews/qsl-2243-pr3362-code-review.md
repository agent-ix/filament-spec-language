---
id: SR-202611210022442313144108538
title: "code-review of PR3362 abstract clock guards decide enabledness in timed refinement liveness"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3362 head 8ec7835e9b463e1680ea7afef4062eb092e9d4e8; fsl-analyze/src/zone_check/{liveness.rs,refine.rs,refine/live.rs,search.rs}, fsl-analyze/src/analyze/items.rs, fsl-analyze/tests/it/{timed_refinement.rs,timed_refinement_liveness.rs}, fsl-replay/src/refinement/clock_enabled.rs, fsl-replay/src/model_check/refinement_replay.rs"
review_set: subset
---

## Summary

Ticket: QSL-2243. This is a soundness review of the diff, with the rust-review lane folded in. The producer's per-vertex enabledness is exact on its cells: guard atoms come from the F_A operations' FR-232 enabling parts, target invariants included. A three-way split at each constant covers both strict and non-strict bounds, and the atoms are the union over all candidates at the position. Replay's stationary reading judges the given lasso exactly for its infinite tail, so it cannot reproduce a lasso that is not a counterexample. No soundness defect was found. The remaining findings are two weakened test oracles, a new typed refusal, and missing unit tests for the new replay reader.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The renamed test accepts Holds or Undecided, so it pins neither outcome | fsl-analyze/tests/it/timed_refinement.rs:482-503 |
| FND-002 | medium | The hidden guard-disables test accepts Holds or Undecided(LassoNotConcretized); it should pin the one outcome the current code gives, naming FR-899-AC-9's unbuilt per-component check | fsl-analyze/tests/it/timed_refinement_liveness.rs:1153-1172 |
| FND-003 | medium | TimedAbstractLivenessTickSource is a new unsupported refusal of a timed-subject liveness half, which FR-899 says is never unsupported; it has no test and is not named as remaining scope | fsl-analyze/src/zone_check/refine.rs:329, fsl-analyze/src/zone_check/search.rs:634-638 |
| FND-004 | medium | The new 290-line replay reader clock_enabled.rs has no unit test, and the end-to-end tests do not reach its point segments, unreset-clock upper bounds, carries, terminal stutter or Unreadable paths | fsl-replay/src/refinement/clock_enabled.rs |
| FND-005 | low | The lasso concretiser's ceiling reads only the concrete model's max constant, while cells now carry abstract guard constants; this can only cost concretisation (an Undecided), not soundness | fsl-analyze/src/zone_check/refine/live.rs:1366 |
| FND-006 | low | candidate_loop_has_fair_run rebuilds the abstract view per arc with build_view instead of the cached abstract_view | fsl-analyze/src/zone_check/refine/live.rs:872 |

## Scope and evidence

I read the full diff, plus live.rs live_product bounds, search.rs include_auxiliary_timed (the abstract bounds are in the aM bounds), the concretiser in liveness.rs (lines 1466-1600), and the settle.rs liveness arms. Governing text: FR-899 "Timed subjects", FR-430 "Strong fairness over time" and its tick routing (lines 94-97 and 147), FR-236, and ADR-020 CT-4. No builds or test runs, per the brief. The quoin gap-analysis method was not run. The new tests carry bare FR-899 tags (correct). The diff contains no Python and no logic-bearing shell.

One pre-existing item, not a finding: the producer imports fsl_replay::candidate_lasso_has_fair_run, so the hidden candidate-graph acceptance is judged by code shared with replay. The clock reading this PR adds is separate in each.

## Verdict

Mergeable after FND-001, FND-002 and FND-004 are fixed and FND-003 is named as remaining scope. There is no soundness block.
