---
id: SR-2026112102472601
title: "Code review — QSL-2240 PR #3244 FR-418 task-product certificate checker (slice S4)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3244 diff origin/main...b61333c7eb: fsl-replay/src/certificate/zone_certificate/task_check.rs, fsl-replay/src/certificate/zone_certificate.rs, fsl-replay/src/certificate/zone.rs, fsl-replay/src/certificate/task_product/key.rs, fsl-eval/src/simulation/model/task_key.rs, fsl-analyze/tests/it/task_product_certificate.rs"
review_set: subset
evaluated_revision: "b61333c7eb6a6f3ecc16dce49d45d621ab5dfe99"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-245
    type: references
---

## Summary

Ticket: QSL-2240. Code review with the rust-review lane folded in. The authority is FSpec FR-418 at filament-specification 6ad873d5, which is also the FSL pin on this branch, so AC-18 to AC-21 are pinned. The review also used design §3 and arch rulings U1 to U6.

What holds: identity and policy come from the checked claim. Every `task_product` member is compared with its recomputation. U1 coverage is sound: `UnionOfRegions` is set only after `region_covers(Z, Z, Arbitrary) == Covered`, and when that holds, plain inclusion is a correct coverage test. The pair fixpoint cannot accept while a recomputed successor lacks an edge. `canonical` enforces U5. Work exhaustion, cancel, Uncertified, overflow and dual-WCET/jitter/equal-priority/not-fixed-work all settle incomplete. A reachable miss settles `bad-state`, or `certificate-unchecked` when the node is locked, and never `proved`. A throwaway probe confirmed this: an over-utilised fixed-priority pair (2/3 + 2/3) had 16 missed vertices and was rejected `bad-state`. The multi-release v1 edge shape is sound because both the concrete FR-419 product and the lowering fire all activations of one source step at once (FND-009).

Mutation experiments ran in a separate worktree, with each mutation behind an environment switch. With the release-ops check removed (M1), all PR tests pass. With delay coverage skipped (M3), all pass. With the miss check removed (M4), all pass and only the reviewer's probe fails. With the overflow check removed (M5), all pass. With urgency ignored in the shared explorer (M2), 5 tests fail, and the 10,922-vertex EEdf quotient ran for about 20 minutes in debug.

## Verdict

**FAIL.** Not mergeable. FND-001 is a real soundness gap in the TD-6 vacuity guard: Zeno region cycles are accepted as time divergence. FND-002 leaves the slice's central safety rule untested. FND-003 to FND-005 are medium gaps a fix round should close in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Time divergence is "a quotient cycle through a time edge" (divergent SCC marking, `repeats`). That accepts Zeno cycles: a never-reset clock bounded by an invariant stays in one open region while another clock is reset each round. The module-doc claim "every clock is then reset or saturated" is false. Design §3 step 5 required `advancing_cycle`, a fixed per-round advance. Result: a forged `admitted` cycle on a Zeno-only subject settles a vacuous schedulability claim `proved`. | fsl-replay/src/certificate/zone_certificate/task_check.rs:14-18, 1159-1191, 1260-1269 |
| FND-002 | high | No PR test reaches a deadline miss. The `here.missed` rule (`bad-state`, or `certificate-unchecked` when locked) is the property a schedulability proof certifies, and deleting it (M4) leaves every PR test green. The "locked" test locks a non-miss node. | fsl-replay/src/certificate/zone_certificate/task_check.rs:1025-1030; fsl-analyze/tests/it/task_product_certificate.rs:855-880 |
| FND-003 | medium | The ZC-28 test is vacuous for the no-release check. Its certificate has no edges, so the ordinary successor-missing rule already rejects it; removing the `release_ops` check (M1) passes. AC-14's other refusals are untested: scheduler clock metadata, a `task_transition` edge on the no-release route. | fsl-replay/src/certificate/zone_certificate.rs:2244-2254; fsl-analyze/tests/it/task_product_certificate.rs:1046-1058 |
| FND-004 | medium | The bounded-subtraction side condition is assumed, not proved. FR-418 says the checker "proves that every reachable subtraction of c from x is guarded by c <= x <= M_x". `subtract_bounded` silently intersects the zone with that range, so any reachable valuation outside it loses its completion successor without a diagnosis. The check should test that post ⊆ range and stop Uncertified otherwise. | fsl-replay/src/certificate/zone_certificate/task_check.rs:487-494; fsl-replay/src/certificate/zone.rs:889-890 |
| FND-005 | medium | Common mode: every accepted certificate is built from `explore_task_product`, so an explorer completeness bug is invisible to tamper tests. Nothing independent is accepted, such as a hand-built ZC-19 or ZC-23 certificate or a 2-node union-of-regions certificate. Delay-coverage tampering (M3) is also undetected. | fsl-analyze/tests/it/task_product_certificate.rs:96-211 |
| FND-006 | medium | U3 metering gaps. These loops are uncharged or charged once for unbounded work: divergent marking O(time edges × V); `Matcher::step` scans all quotient edges for each vertex of the witness frontier; `Divergence::close`; the `leaving` scan for mismatched labels; the `delays` `minus` pieces; `compare_subtractions` O(n²). The finite-product route also returns before `Checker::budgets`, so `max_certificate_edges` and `max_certificate_bytes` are never applied. | fsl-replay/src/certificate/zone_certificate/task_check.rs:577-588, 1045-1056, 1138-1157, 1205-1217, 1260-1269; fsl-replay/src/certificate/zone_certificate.rs:1008-1013 |
| FND-007 | low | U5 canonicity gaps. The task route ignores `lu_bounds` and does not require it to be empty. It accepts `locked` on non-miss nodes. Quotient `initials` skip empty initial states, so positions can shift from FR-361 order. | fsl-replay/src/certificate/zone_certificate/task_check.rs:433-466, 744-795 |
| FND-008 | low | Weak assertions in `a_node_that_does_not_cover_is_rejected`: it accepts either InitialMissing or Malformed, and the column-swap case asserts only `!= UnreachedNode`. Dead `let _ = positive;` in `Divergence::close`. Cancellation and overflow (M5) are untested. | fsl-analyze/tests/it/task_product_certificate.rs:689-739; fsl-replay/src/certificate/zone_certificate/task_check.rs:1213 |
| FND-009 | low | Multi-release v1 shape, pending arch: one `Source` edge for a multi-activation op, which carries no per-activation template. This is sound, because there is no missing interleaving: the concrete FR-419 step (`task_product.rs:878-992`) and the lowering are both atomic. It diverges from the ZC-19 template-per-activation shape, though, and the ruling is still open. | fsl-replay/src/certificate/zone_certificate/task_check.rs:20-28, 241-252 |

## Dispositions

Round 1 is checked against fix commit 3a1234b428. These mutation runs were made in a throwaway worktree at that head:

| Mutation | Result |
| --- | --- |
| M4: miss check removed | caught by `a_reachable_deadline_miss_is_bad_state_or_unchecked_when_locked` |
| MZ: `advances` always true | caught by `a_zeno_region_cycle_is_not_an_admitted_witness` |
| M1: `release_ops` check removed | survives |
| M3: delay coverage skipped | survives |
| MW: `within_range` skipped | survives |

The PR tests pass: 20/20 and 105/105.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | still-open | M1 still survives. The certificate in `a_reachable_release_is_the_only_defect_of_a_no_release_closure` has no `fire_a` source edge, so ordinary successor-missing rejects it whether or not the release check exists. The test needs a source edge for the release operation with a covering target, so that the release check is the only thing that can reject. The task-edge case asserts Malformed OR SuccessorMissing. A probe on the release-free source shows the code does refuse it as Malformed at `{member: edges}`, so that assertion can be exact. |
| FND-004 | fixed | - |
| FND-005 | still-open | The hand-written 4-node certificate is a real independent oracle and closes the common-mode half. M3 still survives: no test tampers a node so that it is not closed under delay within its key. |
| FND-006 | fixed | - |
| FND-007 | accepted-no-change | `lu_bounds` must now be empty (`shape`). Locked on a non-miss node stays: FR-418 bars badness without `locked`, not `locked` without badness. Skipping empty initial states stays: such a state has no valuation to cover. Both are low and have no soundness effect. |
| FND-008 | fixed | - |
| FND-009 | accepted-no-change | Pending the arch ruling. The v1 shape is sound: the concrete FR-419 step and the lowering are both atomic. |
