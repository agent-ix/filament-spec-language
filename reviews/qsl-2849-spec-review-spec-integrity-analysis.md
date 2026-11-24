---
id: SR-3639003
title: "spec-review (integrity, EARS) of PR 3639 new acceptance criteria"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR 3639 head bc7fa1b398; FR-126, FR-203, FR-247, FR-264 and TC-521, TC-638, TC-702, TC-738"
review_set: subset
---

## Summary

Ticket: QSL-2849. Spec review of the five new ACs, the edited FR-247 statement and use
case, and the four TC edits. `quire validate` on the nine changed spec files returned no
findings. Each AC is atomic, measurable (counted rather than timed) and matches the code.
FR-247's new SHALL ("count every recurrence step") agrees with FR-255's
`schedule.max_demand_points` row and FR-248's linear-check statement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-738's new procedure step 6 sits after the "Tag the tests" paragraph, outside the numbered procedure list, so it renders as a separate list after the tagging instructions | spec/test-cases/TC-738-a-deep-package-emits-reads-back-and-verifies.md:41-44 |

## Verdict

PASS with one low layout nit.

## Dispositions

Round 1, reviewed head 1946b4382f.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Step 6 now sits inside the numbered procedure, before the Tag paragraph |

Round 2 reviewed `1ac4da2a725290e8986f265eeb45bf73917e7d0d`. The PR requires another fix round.

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | FR-126-AC-26 explicitly expects two full initial-state constructions at max_states=1. TC-521 and protocol_system.rs:726 verify that behavior. The owner requires the second distinct state to remain unconstructed. Align the AC and its counter oracle, preserving duplicate coalescing and the typed failure ordering. |

Round 3 reviewed `bb649d2006b756bcbd7358a4fbd1474760ab5c86`. Open findings block merge.

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | FR-126-AC-26 and the deadlock-freedom counter now correctly require one constructed protocol root with four distinct roots at max_states=1. TC-521 step 12 still expects two initial states pulled/keyed by shared admission and allows reduced/sampled requests to generate the crossing candidate. Those candidates are full ModelSimState values: tests count two at max_states=1. The requirement and test oracle still omit the owner’s construction bound for model roots, and the protocol oracle does not exercise liveness. See SR-3639001 FND-006 and FND-013. |

Round 4 reviewed `b05ca318826032f4fb39bf61918f9e3993f51c72` against `608e201178a28f96a0b4e738114ce7227cb66c7b`. BLOCKED by the contradictory construction-count oracle.

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | TC-521 Procedure step 12 (lines 80–83) requires the second distinct root unconstructed at limit one. Expected Results (lines 160–169) still allows two pulls at one and one keyed state at zero. Four focused integration assertions fail on this head with the old expectations: reduction.rs:281 (1 vs 2), refinement_budget.rs:445 (0 vs 1), state_graph.rs:1493 and :1539 (both 1 vs 2). Preserve the separate coverage of the crossing-state defect in the nontrivial-symmetry precheck and mapped refinement (SR-3639001 FND-006). |

Round 5 reviewed `b0078a167be4aa35c8fce8d1ef8bfcbec4094c6e` against `dccaf11c3d00f51738761f26cb72ef3124415e02`. PASS. Both prior spec findings are closed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | TC-521 now consistently demands one materialized root at limit one and zero at limit zero. The four previously failing reduction/refinement/state-graph assertions now expect those counts and pass. |

Round 6 reviewed `2dc5f4817dfad4059ac12990eff97e0176311dba` against `16c724b092132cce8283860c9f67fd327f3db064`. PASS. Both prior spec findings remain closed. FR-203-AC-28 now owns the dense settlement contract.

Round 7 reviewed `e77768d387d01eee1ed13483835e60b2ce249000` against `ca74568372bdda843b1808c6fee4e3c5c56a8ee7`. PASS. Prior spec findings remain closed. FR-203-AC-28 and TC-638 govern dense storage and cancellation, and TC-521 retains the corrected distinct-root bound.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | FR-126-AC-26 explicitly expects two full initial-state constructions at max_states=1. TC-521 and protocol_system.rs:726 verify that behavior. The owner requires the second distinct state to remain unconstructed. Align the AC and its counter oracle, preserving duplicate coalescing and the typed failure ordering. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:608 |
