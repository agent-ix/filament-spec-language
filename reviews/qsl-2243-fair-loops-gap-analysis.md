---
id: SR-20261121181402002
title: "gap-analysis of PR3357 exact timed fair-loop search, targeted to FR-899"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3357 head e0487871c326d18465a53e299981acc0525a39f8; FR-899 criteria against fsl-analyze/tests/it/timed_refinement_liveness.rs and fsl-analyze/src/zone_check/refine/live/fair_loops.rs"
review_set: subset
---

## Summary

Ticket: QSL-2243. This is a targeted, planless gap analysis of the PR3357 diff. `quire matrix --scope . --format json` reports every FR-899 criterion as tagged. The diff adds bindings for FR-899-AC-7 (line 1068) and FR-899-AC-9 (lines 624, 682, 1068 and 1091). The two tests converted from Undecided to Holds now evidence AC-9's own fixtures. The two new two_loop_world tests are tagged to AC-7 and AC-9, but they use a different fixture from the one those criteria name. All new production code has an owning requirement, FR-899 "Timed subjects". There are no stubs or placeholder returns. Runtime evidence does not exist yet.

## Verdict

CONDITIONAL. There are only low findings, but the tests are qualified at compile level only. Nothing has been run, and the claim that the hiding test fails on base has not been established.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | a_fair_loop_of_a_timed_component_does_not_hide_an_unfair_one is tagged FR-899-AC-7. AC-7 names an untimed `{A, B}` fire1/fire2 fixture over AC-6's tick loop. The test evidences the joint-prefix naming rule on a timed poll/tock fixture, not that AC | fsl-analyze/tests/it/timed_refinement_liveness.rs:1066-1068 |
| FND-002 | low | The two two_loop_world tests are tagged FR-899-AC-9. AC-9 names FR-430-AC-5's Req and TReq fixture with send and poll. These tests evidence FR-899 "Timed subjects" over a different send/idle/poll/tock fixture | fsl-analyze/tests/it/timed_refinement_liveness.rs:1066-1068, fsl-analyze/tests/it/timed_refinement_liveness.rs:1089-1091 |

## Coverage

- Plan completion: not assessed
- Matrix: FR-899-AC-1 to AC-10 are all `tagged`, computed by `quire matrix`. No criterion lost a tag in this diff: the removed lib test only carried TC-2536.
- These bindings were checked and are correct: timed_hidden_free_initial_valuation_holds_over_every_fair_loop and timed_hidden_strong_send_with_invariant_holds are both bound to FR-899-AC-9. These were checked and are wrong: the two_loop_world tests bound to AC-7 and AC-9 (FND-001, FND-002).
- Reverse gap: fair_summary.rs, fair_loops.rs and the live.rs branch all belong to FR-899. No unowned behaviour was found.
- Stubs and inflation: none found. The two lib unit tests exercise idempotent_power_by directly, with a real 3-cycle relation and a real paying closure.
- Strength of the budget oracle: see SR-20261121181402001 FND-002.
- Semantic review: not run as a separate fan-out. The single-reviewer brief forbids subagents, and its intent-to-test-to-code judgement is in SR-20261121181402001.
- Runtime: none. All seven focused tests still need TEST-RUN qualification, ideally together with a run of the hiding test on base to show it fails there.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
