---
id: SR-2026112102472602
title: "Gap analysis — QSL-2240 PR #3244 FR-418 task-product certificate checker (slice S4)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3244 diff origin/main...b61333c7eb: fsl-analyze/tests/it/task_product_certificate.rs, fsl-replay/src/certificate/zone_certificate/task_check.rs"
review_set: subset
evaluated_revision: "b61333c7eb6a6f3ecc16dce49d45d621ab5dfe99"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-245
    type: references
---

## Summary

Ticket: QSL-2240. Plan completion: not assessed. This is a scoped check of FSpec FR-418's ACs against the tagged tests in this PR. The FSL pin on this branch is FSpec 6ad873d5, which carries AC-18 to AC-21. The coder tagged only AC-12, AC-13 and AC-14, because their checkout lacked AC-18 to AC-21.

| Criterion | Test | Result |
| --- | --- | --- |
| FR-418-AC-12 | `a_forged_template_leaves_the_successor_missing`, `a_reversed_ready_order_is_not_the_recomputed_key`, `an_edf_tie_dispatches_the_canonically_first_task` | covered |
| FR-418-AC-13 | `the_recomputed_periodic_certificate_is_accepted`, `an_out_of_range_activation_rank_leaves_the_successor_missing`, slot half of `a_forged_template...` | covered for the checker; the replay half belongs to FR-364 |
| FR-418-AC-14 | `a_no_release_closure_is_accepted_only_when_no_release_is_reachable` | partial: the reachable-release refusal is vacuous, and scheduler clocks and task edge are untested (SR-2026112102472601 FND-003) |
| FR-418-AC-18 | none | gap: U1 two-region union untested |
| FR-418-AC-19 | `the_certificate_is_the_reachable_closure` (tagged AC-13) | covered, mis-tagged |
| FR-418-AC-20 | `a_work_limit_below_the_graph_is_incomplete` (tagged AC-13) | covered at checker level, mis-tagged |
| FR-418-AC-21 | `a_mixed_criticality_set_is_certificate_unchecked` (tagged AC-12) | covered at checker level, mis-tagged |

## Verdict

**FAIL.** AC-18 has no test. Six bindings are wrong. AC-14 is only partly verified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-418-AC-18 (U1: target `Z` a union of two whole regions accepts; reduced to the other region, `successor-missing`) has no test. The builder's per-key hulls never exercise a deliberate two-region union against a one-region reduction. | fsl-analyze/tests/it/task_product_certificate.rs |
| FND-002 | medium | Wrong trace tags. `the_certificate_is_the_reachable_closure` is AC-19 (tagged AC-13). `a_work_limit_below_the_graph_is_incomplete` is AC-20 (tagged AC-13). `a_mixed_criticality_set_is_certificate_unchecked` is AC-21 (tagged AC-12). `divergence_and_the_admitted_witness_are_replayed_on_the_quotient` is AC-2-shaped `bad-state`/`witness-fails` (tagged AC-13). `a_node_that_does_not_cover_is_rejected` is AC-2-shaped (tagged AC-12). `a_member_that_differs_from_its_recomputation_is_malformed` is member recompute, which no AC covers (tagged AC-13). | fsl-analyze/tests/it/task_product_certificate.rs:363, 435, 588, 689, 855, 884, 907 |
