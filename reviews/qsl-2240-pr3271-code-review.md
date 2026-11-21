---
id: SR-20261121063601
title: "Code review — QSL-2240 PR #3271 FR-418 S5 task-product symbolic producer, emission and settle"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3271 diff origin/main...190f1b57f1: fsl-analyze/src/zone_check/task_symbolic.rs, fsl-analyze/src/zone_check/task_certificate.rs, fsl-analyze/src/analyze/task_product_item.rs, fsl-analyze/src/analyze.rs, fsl-analyze/src/analyze/document.rs, fsl-replay/src/certificate/zone_certificate/wire.rs, fsl-replay/src/certificate/zone_certificate.rs, fsl-replay/src/certificate/task_product/wire.rs, fsl-analyze/tests/it/task_product_producer.rs, fsl-analyze/tests/it/task_automaton_lowering.rs, fsl-analyze/tests/it/zone_certificate.rs"
review_set: subset
evaluated_revision: "190f1b57f1929bc34c736a1e4a33dbbdfab119cb"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-249
    type: references
---

## Summary

Ticket: QSL-2240. This is a code review with the rust-review lane folded in. The authorities are FSpec FR-418 (filament-specification `spec/objects/interfaces/FR-418-zone-certificate.md`, AC-1 to AC-21), the design notes in `/tmp/fr418-design.md` (§4 and rulings U1 to U7), FSL FR-249 (AC-7 and AC-8, and the "closed search without a certificate" behaviour) and FR-621 (`certificate` member).

**Tests run** (name-filtered, debug, `--test-threads=1`). There were 48 tests over `task_product_producer`, `task_product_certificate`, `zone_certificate::` and four `task_automaton_lowering` tests: 46 passed and 2 failed.
- `a_closed_search_without_a_certificate_settles_certificate_unchecked` fails because of this PR (FND-001).
- `zone_certificate::the_certificate_members_are_the_schema_fifty_one_names` fails with 61 vs 51 members. The member list comes from the contract-model pin, and this PR touches neither pins nor `CertificateMember`, so it is a red already on main. It is not on the known-baseline list.
- The ignored AC-8 test was not run.

**Mutation run** in a throwaway worktree: the S4 `advancing_cycle` change reverted, plus a probe that prints the U6 item's record. Results are under Verdict.

What holds:
- **Independence.** `task_symbolic.rs` and `task_certificate.rs` import only the S3 `TaskAutomaton` lowering, the engine `Dbm`, and fsl-replay data and wire types (`TaskProductKey`, `TaskTransitionTemplate`, `CertNode`/`CertEdge`/`Zone`, `TaskQueueBound::from_checked_task_set`, `closed_finite_product`, and the spelling-level `same_source_transition`). Neither file calls `explore_task_product`, `region_covers` or anything in `task_check.rs`. The checker quotient appears only in the agreement test.
- **Settlement.** `proved` is reached only through `ProofVerdict::Checked(Accepted)`, and that runs `check_zone_certificate` on the certificate decoded from the emitted JSON. A rejection settles `inconclusive`/`certificate-rejected` with the checker's rule and locus. Dual-WCET with a mode switch, not-fixed-work, jitter and equal priorities stop at `certification()` as `Uncertified` and never build a certificate. A reachable miss vertex refuses the certificate (`Missed`). In `settle`, a replayed counterexample takes precedence over every proof value. A rejected, stopped or uncertified pass falls through to the concrete search, so it cannot hide a refutation. Skipping the concrete search after an accepted proof hides one only if the checker accepts wrongly (FND-008).
- **Per-key hull (item 6).** A hull that is not a union of regions is refused by the checker's `shape_of`: `malformed` at `nodes`, which settles inconclusive. A hull larger than the reachable regions cannot hide a reachable state, because the checker computes reachability itself. This is a safe failure.
- **`advancing_cycle` (item 3).** The change is sound and only rejects more. `x > r` after `x := r` admits the Zeno run `δ_n = 2^-n`, so it was never a per-round lower bound. Both callers (`zone_certificate.rs:2777` and `divergence.rs:166`) use the result only to accept a witness cycle. Engine dense witnesses go through the `z`/`Diverge` early return and are unaffected.

## Verdict

**FAIL.** Not mergeable as it stands:
- FND-001: the PR leaves an FR-249 test red, and the FR-249 "closed search without a certificate" behaviour no longer has a passing test.
- FND-002: the symbolic pass is not plumbed to the published `zone_check.max_check_work` setting, while the PR tags a test FR-418-AC-20.
- FND-003 to FND-007 are medium gaps that belong in this PR's fix round.

Items 2 and 6 are sound. Item 3 is sound (see Summary). Mutation result: with the old strict-guard rule restored, `a_strict_guard_at_the_reset_value_is_not_an_advancing_cycle` fails at zone_certificate.rs:1078 (the Zeno ping witness is accepted), so the test kills the mutation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR turns `a_closed_search_without_a_certificate_settles_certificate_unchecked` red. Its `Calm` fixed-work single-task set now proves through the symbolic pass, so the run reports left `proved`, right `incomplete`. The FR-249 behaviour "a closed task-product search with no accepted FR-418 certificate settles `incomplete`/`certificate-unchecked`" therefore has no passing test. The test needs a set that closes but has no accepted certificate, for example equal priorities or a lo==hi dual-WCET set with a mode switch, and the proved Calm case should be asserted separately. | fsl-analyze/tests/it/task_automaton_lowering.rs:836-865 |
| FND-002 | high | `zone_check.max_check_work` is not plumbed. `prove` always uses `CertificateCheckLimits::default()`, both for the producer's work limit and for the checker's limits, so the request option cannot change either. FR-418-AC-20 (a setting below the needed work settles `incomplete`/`limit-reached` naming `zone_check.max_check_work`, never proved) cannot hold, yet the PR tags `the_symbolic_pass_stops_at_its_work_limit_and_on_cancel` FR-418-AC-20. | fsl-analyze/src/analyze/task_product_item.rs:495-499; fsl-analyze/tests/it/task_product_producer.rs:323-343 |
| FND-003 | medium | The canonical `admitted` decode can change the witness, and the guard in `prove` cannot see it. `read.to_wire(..) == wire` compares flattened edge lists, and `to_wire∘from_wire` is the identity on them. The producer's cycle is a vertex-level walk over per-key nodes, so it can pass through its own start node mid-cycle. `read_witness` then takes the later return point as the cycle and moves the rest into `path`, and the checker may reject a valid proof (`witness-fails` → inconclusive). Never unsound. Compare the decoded `(start, path, target)` with the built witness, or pick a node-simple cycle. | fsl-replay/src/certificate/zone_certificate/wire.rs:319-364; fsl-analyze/src/analyze/task_product_item.rs:554; fsl-analyze/src/zone_check/task_certificate.rs:264-360 |
| FND-004 | medium | The item's `certificate` member breaks FR-621's shape. FR-621 writes `certificate` for a proved record as `{identity, checker, accepted}`, and a hybrid item adds the re-checkable `form`. The PR instead merges `checker` and `accepted` into the whole FR-418 `ZoneCertificate` object. The result is neither a valid `ZoneCertificate` (that reader is closed) nor the FR-621 shape, and it is also written for non-proved, rejected records. FR-249-AC-7 needs `certificate.task_product`, so FR-621 needs a ruling: a `form`-like member, or an amendment. | fsl-analyze/src/analyze/document.rs:4156-4170 |
| FND-005 | medium | The S4 shape relaxation accepts a malformed reference. `effective_id` loops `while !current.is_null()`, so a top-level `object_type: null` passes, and `identity` is now any non-empty text. FR-181's node-id `object_type` is now refused. This is not a soundness weakening, because source edges are matched by spelling against recomputation, but the shape check now admits what FR-361-AC-13's writer never emits. Reuse `spine::identity_form::read_effective`, which refuses a null root. Track the FR-181 vs FR-361-AC-13 conflict with a ticket until arch rules. | fsl-replay/src/certificate/task_product/wire.rs:321-352 |
| FND-006 | medium | The symbolic pass runs before the concrete search on every EN-6 item and does not stop at a miss. A missed vertex is not expanded, but the pass keeps going until the whole quotient closes, and only then does `task_product_certificate` refuse `Missed`. On a refutable set (such as AC-8's `t3` `wcet` 4 under fixed priority), the pass can spend the full 100M work (measured at 791 s on set B) before the concrete search that refutes even starts. Stop with `Uncertified`/`Missed` at the first missed vertex. | fsl-analyze/src/zone_check/task_symbolic.rs:569-572; fsl-analyze/src/zone_check/task_certificate.rs:160-162; fsl-analyze/src/analyze/task_product_item.rs:105-139 |
| FND-007 | medium | The settle mapping is tested only at the explore level. These paths have no item-level test: Rejected → `inconclusive certificate-rejected`, `Checked(Stopped)`, the pass's own `limit-reached` (`ZoneMaxCheckWork`) and `cancelled` once the concrete search closes, and Fault → `failed`. The `closed` gate in `proof_value` (which decides when a pass stop overrides the concrete cause) is also untested. | fsl-analyze/src/analyze.rs:4068-4102; fsl-analyze/tests/it/task_product_producer.rs:323-343 |
| FND-008 | low | An accepted proof skips the concrete search. That removes the only run-time cross-check between the S3 `TaskAutomaton` lowering, which the producer and checker share, and the concrete FR-419 `TaskProductSystem`. The quotient-agreement test compares two explorers over the same lowering, so a lowering divergence would settle `proved` where the concrete search would refute. The S3 sampled cross-check is the only guard. | fsl-analyze/src/analyze/task_product_item.rs:121-139 |
| FND-009 | low | Any missed vertex refuses the certificate, including a time-locked miss that FR-418 permits as a `locked` node. This is a loss of completeness only. | fsl-analyze/src/zone_check/task_certificate.rs:160-162 |
| FND-010 | low | The test assertions are weak. `no_certificate_is_built_for_a_reachable_miss` asserts `is_err()`, not `Missed`. `a_tampered_emitted_certificate_is_rejected` accepts `SuccessorMissing \| Malformed` or just `!= Accepted`. `let _ = build;` is dead code. | fsl-analyze/tests/it/task_product_producer.rs:255-321, 342 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | high | Running the symbolic pass first breaks the FR-249-AC-9 refutation test, the ticket's negative AC. `an_unschedulable_set_refutes_with_its_first_missed_deadline` (set B, `t3` `wcet` 4, fixed priority) ran more than 60 min in debug at 12 GB RSS without finishing at e950d4d4ec, and was killed. With the symbolic pass skipped (main's behaviour, throwaway switch `nosym`), the same test passes in 0.67 s. Stopping at the first miss does not help: breadth-first region exploration of set B's quotient (more than 100M work, per AC-8) exhausts the budget before it reaches `t3`'s first deadline, and only then does the concrete search refute. | fsl-analyze/src/analyze/task_product_item.rs:121-139; fsl-analyze/src/zone_check/task_symbolic.rs:596-601; fsl-analyze/tests/it/task_automaton_lowering.rs:404-415 |

## Dispositions

Round 1 is checked at fix commit e950d4d4ec, which sits on the rebased S5 commits.

**Test runs.** At the head, name-filtered: the `task_automaton_lowering` tests passed 21 of 21, with the AC-9 test killed (FND-011) and AC-8 not run. The `fsl-analyze` lib tests `proof_value_tests` and `task_certificate::tests` passed 7 of 7. In the throwaway at the same head with no mutation active: `task_product_producer`, `task_product_certificate`, `zone_certificate::` and the remaining lowering tests passed 48 of 49. The one failure is `the_certificate_members_are_the_schema_fifty_one_names` (61 vs 51). It is a red already on main: its inputs (the contract-model and FSpec pins, and the test) are identical between origin/main ab98ea12d8 and this branch, and the branch changes no Cargo manifest.

**Mutations** (throwaway, runtime `RV_M` switch):

| Mutation | Result |
| --- | --- |
| `plumb`: setting ignored | caught by `a_small_max_check_work_settles_limit_reached` |
| `eff`: null `object_type` accepted | caught by `a_null_receiver_object_type_is_not_read` |
| `miss`: no stop at the first miss | caught by `no_certificate_is_built_for_a_reachable_miss` |
| `guard`: decoded-witness comparison removed | survives (8/8) |
| `erase`: `erase_loops` and the `canonical` filter skipped | survives (8/8) |

The last two survive because no current set builds a witness that the decode would change. Only the `canonical`/`erase_loops` unit tests exercise that logic.

**FND-002 placement.** Pinned FSpec 6ad873d5 FR-331 and `backend-provider-v1/schema.json` `TemporalLimits` both carry `max_check_work`, described as "setting zone_check.max_check_work, default 100000000". So `temporal.limits.max_check_work` is the specified request member, and the coder's note that the schema has no such member is mistaken. The `limit-reached` cause names `max_check_work` and `zone_check.max_check_work`, as FR-418-AC-20 requires.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | still-open | Stopping at the first miss covers only small sets. On set B the pass exhausts the 100M work before any miss vertex appears, so a refutable set still waits for the full symbolic budget (see FND-011). The coder's objection to running the concrete search first was that sporadic sets hit the cap. But the concrete search's own cap (`max_task_product_work`, default 1048576) bounds that cost. The concrete search could run first and refute, with the symbolic pass running only when it does not refute, or the symbolic pass could get a smaller first slice of budget. |
| FND-007 | fixed | - |
| FND-008 | accepted-no-change | Documented in the task_product_item module doc. The S3 sampled cross-check is the stated guard. This was low and is a design decision. |
| FND-009 | accepted-no-change | Documented as a completeness loss: the concrete search then decides, and a time-locked miss is never refuted, so the result is not unsound. |
| FND-010 | fixed | - |

### Disposition pass 2

Recovery disposition checked at `f20c14a6a3fc79b54d6425e30bc5744bd7a00b3d`, following the unavailable original reviewer's round 1. The final fix diff is `7747319762..f20c14a6a3`; the rebase preserves the accepted checker-only proof path. Original findings and previous outcomes above are unchanged.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-006 | fixed | The bounded concrete search now runs before the symbolic pass. A refuted result or a cancelled concrete search does not invoke `prove`; other stopped outcomes may attempt symbolic certification. A core-accepted certificate still controls `proved`. |
| FND-011 | fixed | The AC-9 negative case no longer waits for the symbolic quotient. `an_unschedulable_set_refutes_with_its_first_missed_deadline` passes in the final focused run, and `a_refutable_set_refutes_before_the_symbolic_pass` requires a replayed counterexample. |

The new witness regression requires an admitted witness in every producer fixture, successfully encodes and decodes a changed target, asserts that the decoded witness differs, and requires `read_back_unchanged` to refuse it. Removing the admitted-witness comparison would let the final assertion fail, so the test exercises that guard without a conditional/vacuous assertion. The production caller uses the same helper before invoking the core checker.

Evidence inspected, not run by this recovery reviewer: `tmp/recovery-fix2-focused-final.log` reports 14 passed, 0 failed, 0 ignored (37.46 s); `tmp/recovery-fix2-lib.log` reports 7 passed, 0 failed, 0 ignored; `tmp/recovery-fix2-witness.log` reports the corrected witness test passed 1/1. No Cargo, Python, mutation build or full gate ran in this disposition. The default-budget Pair test is separately queued on production-identical ancestor `83765b21c5`; only the witness test changed after that head. Its result remains pending.

**Current verdict:** no remaining static review blocker and no new finding in this fix round. Final mergeability remains conditional on the requested default-budget Pair result. FR-249-AC-8 remains the explicitly deferred gap on QSL-2240; the ticket must stay open.
