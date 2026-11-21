---
id: SR-20261121063602
title: "Gap analysis — QSL-2240 PR #3271 FR-418 S5 task-product symbolic producer, emission and settle"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #3271 diff origin/main...190f1b57f1: fsl-analyze/tests/it/task_product_producer.rs, fsl-analyze/tests/it/task_automaton_lowering.rs, fsl-analyze/tests/it/zone_certificate.rs against FSpec FR-418 AC-2/11/19/20/21 and FSL FR-249 AC-7/AC-8"
review_set: subset
evaluated_revision: "190f1b57f1929bc34c736a1e4a33dbbdfab119cb"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-249
    type: references
---

## Summary

Ticket: QSL-2240. This is a manual AC-to-test check of the PR's tagged tests against FSpec FR-418 and FSL FR-249. It is scoped to the diff. Plan completion: not assessed.

**FR-249-AC-7 is verified.** `a_two_task_set_lowers_with_its_declared_periods_and_deadlines` passes un-ignored. It settles `proved` through an accepted certificate and checks the queue-bound rows.

**FR-249-AC-8 stays `#[ignore]`.** The stated reason is that the set B region quotient exceeds the 100M default `max_check_work`, measured at 791 s in release. That is consistent with the code: the work limit stops the pass, and the item would settle `incomplete`, never a wrong verdict, so the ignore does not hide a soundness bug. It does leave the ticket's Buttazzo AC unmet (FND-006). The test was not run, per the brief.

## Verdict

**CONDITIONAL.** Four tags overclaim (FND-001 to FND-004). AC-20 and AC-21 are not verified (FND-004, FND-005). AC-8 is deferred honestly but remains open, so QSL-2240 cannot close on this PR.

U6 probe (throwaway worktree): the dual-WCET item settles `incomplete` with cause `bounded-search-incomplete`/`unresolved-dynamics` and a `task_search` frontier, not `certificate-unchecked`. That makes FND-005 a real gap: no test reaches the mode-switch refusal, and the analyze item does not carry the cause AC-21 requires.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `the_engine_quotient_equals_the_checker_quotient` is tagged FR-418-AC-19. AC-19 is the checker's `unreached-node`/`extra-edge` rejection, and this test is an engine/checker agreement test that exercises neither rule. | fsl-analyze/tests/it/task_product_producer.rs:189-202 |
| FND-002 | low | `a_tampered_emitted_certificate_is_rejected` is tagged FR-418-AC-19, but no case adds an unreached node or an extra edge. The four tampers it makes are member recompute (malformed), a dropped edge, a retargeted edge and squeezed zones. | fsl-analyze/tests/it/task_product_producer.rs:251-305 |
| FND-003 | low | `no_certificate_is_built_for_a_reachable_miss` is tagged FR-418-AC-2. AC-2 names the checker's rejection rules (`bad-state` etc.), and this test checks that the producer refuses to build a certificate (`is_err`). | fsl-analyze/tests/it/task_product_producer.rs:307-321 |
| FND-004 | medium | `the_symbolic_pass_stops_at_its_work_limit_and_on_cancel` is tagged FR-418-AC-20. AC-20 needs `zone_check.max_check_work` set below the needed work to settle `incomplete` `limit-reached` naming the setting, and the default to settle `proved`. The test calls `explore_task_symbolic` with a literal 10, and the setting is not plumbed (SR-20261121063601 FND-002). | fsl-analyze/tests/it/task_product_producer.rs:323-343 |
| FND-005 | medium | FR-418-AC-21 / U6 is not verified. `a_dual_wcet_set_settles_incomplete_never_proved` declares no `mode … hi on` switch, and its task has `wcet` 2 / `wcet_hi` 3, so `certification()` refuses `NotFixedWork`, not `DualWcetModeSwitch`. The test also does not assert the `certificate-unchecked` cause that AC-21 and the FR-418 behaviour text require. An `Uncertified` pass falls through to the concrete search, so the cause is whatever that search ends with. A probe of this test's item shows `unresolved-dynamics` with `task_search`. | fsl-analyze/tests/it/task_automaton_lowering.rs:867-886; fsl-analyze/src/analyze.rs:4100-4101 |
| FND-006 | medium | FR-249-AC-8 is still unmet. It is ignored with a measured reason, pending an arch ruling. The ticket AC "the zone engine's deadline verdict equals the EN-7 response-time verdict on the Buttazzo example" is therefore not done. The refute arm would also first spend the full symbolic budget (SR-20261121063601 FND-006). | fsl-analyze/tests/it/task_automaton_lowering.rs:307-310 |
| FND-007 | low | The FR-249-AC-7 assertions are partial. The AC lists `taskset` and `max_jitter` in each row, and also the lowered subject's task-timing rows. The test reads neither. | fsl-analyze/tests/it/task_automaton_lowering.rs:248-290 |

## Dispositions

Round 1 is checked at e950d4d4ec. The PR tests named below pass (see SR-20261121063601 Dispositions for the runs and mutations). On NotFixedWork, FR-418 does not ask for an analyze-level test: AC-21 covers only dual-WCET with a mode switch, and FR-249 keeps variable demand on bounded exploration. That path is already covered by `an_unequal_budget_product_never_proves_and_stops_with_task_search_evidence`, which passes.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | deferred | FR-249-AC-8 stays ignored, with its measured reason, pending the arch ruling on the abstraction or the AC wording. QSL-2240's Buttazzo AC remains open, so the ticket must not close on this PR. |
| FND-007 | fixed | - |

### Disposition pass 2

Checked the final fix at `f20c14a6a3fc79b54d6425e30bc5744bd7a00b3d`. No gap finding had a latest `still-open` or missing outcome before this pass, so no new disposition rows are required. The original round-1 outcomes remain current. No new gap was found in the bounded fix diff.

The renamed concrete-first refutation regression retains its `TC-371` tag and requires `refuted` plus reproduced replay. The new witness read-back regression also claims only `TC-371` and asserts both successful codec traversal and a changed decoded witness before requiring guard refusal. No acceptance criterion is newly overclaimed.

Observed local logs report 14/14 focused integration tests and 7/7 focused library tests passing; this reviewer ran no Cargo, Python or gate. The default-budget Pair proof remains a separate pending verification. FR-249-AC-8 remains ignored with the measured reason already recorded in round 1, and remains open on QSL-2240 pending architecture's abstraction/criterion ruling. This disposition does not claim that the Buttazzo acceptance criterion is complete.
