---
id: SR-202611210022442313144108502
title: "Gap analysis — PR #3269 QSL-2247 refinement transfer (FR-1020, FR-1030, FR-1031, FSpec FR-820)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; diff ed3f278e4e3ece4f49f6520019d5cc3f14c18d68...eec1d1c35dc06eb49d9ebe5853a29850e2c982b5; FR-1020-AC-1..2, FR-1030-AC-1..5, FR-1031-AC-1..4, FSpec FR-820-AC-1..5 and FSpec FR-380-AC-9 tags"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1031
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1020
    type: references
---

## Summary

Ticket: QSL-2247. Planless repository audit scoped to the PR's criteria. Matrix
from `quire matrix --scope /home/peter/dev/worktrees/qsl-2247 --format json`
(quire 0.52.0, engine 0.62.2): FR-1020-AC-1/2, FR-1030-AC-1..5 and
FR-1031-AC-1..3 are tagged by non-ignored tests; FR-1031-AC-4 is **untagged**.
Several tagged criteria are backed by assertions the code cannot satisfy or by
oracles too weak to fail. No test has run: tags are not runtime evidence.

## Verdict

**FAIL** — FR-1031-AC-4 untagged, and FR-1031-AC-3's binding test asserts
behaviour the implementation does not have (coverage inflation). Semantic review
(intent↔test↔code) was performed inline for the scoped criteria as part of this
reviewer pass. Clean bindings: FR-1020-AC-2 → `a_step_for_step_refinement_is_decided_from_its_step_rows_and_time_kind`
(stutter, any and dense each falsify step-for-step; requires `--features
test-support`), FR-1030-AC-2 → `previous_transfers_only_through_a_step_for_step_refinement`
(fragment reason, operator names, `row` arm, AtMostTwo proved, exit 22),
FR-1031-AC-1 and FR-1031-AC-2 → `a_refinement_transfer_request_is_refused_by_stage`
(call/items conflict, duplicate clause pointer, Ghost missing-name, missing
package at `compile`, Impl invalid-value naming Third).

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high | FR-1031-AC-4 is untagged (matrix status `untagged`, no binder) and unimplemented: a supplied claim premise over another abstract package is not refused at stage `admit`, and `CasRefinesCounter`'s never-refused case is not asserted by any AC-4 test. | spec/functional/FR-1031-refuse-a-refinement-transfer-request-by-stage.md:55; fsl-command/src/command/transfer.rs:701-749 |
| FND-002 | high | Coverage inflation on FR-1031-AC-3: the binding test asserts `type-mismatch` for the state clause `CanInc` and `invalid-value` for `OverThird`, but the code refuses `CanInc` with `missing-name` and has no `over`-subject check, so the tagged test is expected to fail; the AC's bounded-profile clause case is not exercised at all. | tests/it/plan_export_surface.rs:1857-1874; fsl-command/src/command/transfer.rs:161-182 |
| FND-003 | medium | Weak oracles. FR-1030-AC-5 (PrevStutter) asserts only `premise-unproved` and slot 1; it does not assert `cause.result` (`refuted`) or the stutter witness the 2026-11-20 ruling says it pins, so any non-proved slot-1 result passes. FR-1030-AC-3 cannot tell a consumed refinement premise from a re-settled one (both fold to `uncertified`). FR-1030-AC-1 asserts no certification, so the global NotTranslated→`uncertified` change (code-review FND-001) is unobserved by any test. | tests/it/plan_export_surface.rs:1668-1679, 1713-1744 |
| FND-004 | medium | FSpec FR-820 tags overclaim. `a_step_for_step_transfer_records_both_premises_and_is_proved` is tagged FSpec-FR-820-AC-1 but seeds `Proved(Uncertified)` directly instead of reading `certified` entries from bytes, and never asserts `fairness.implied` or `premise_form`; the FSpec-FR-820-AC-4 test checks `unsupported` at slot 0 while the AC requires `unsupported`, `incomplete` and `refuted` at slot 1 and the fairness reason; FR-820-AC-2's `any` row and `{"time":"dense"}` record and all of FR-820-AC-3 (admission) have no test. | fsl-replay/src/planner/transfer_tests.rs:45-103 |
| FND-005 | medium | Reverse gap: the new `Row::of_check` NotTranslated arm (settle.rs:803-811) and the analyze route's acceptance of `stutter_closure: true` for any source-form liveness request (clause_item.rs:189-199) change behaviour outside the transfer route with no owning FR-127/FR-529 criterion row and no direct test. | fsl-replay/src/model_check/settle.rs:803-811; fsl-analyze/src/analyze/clause_item.rs:189-236 |
| FND-006 | medium | Test-evidence gap: no test has run on this head. `refinement_declarations` is compiled only with `fsl-semantics --features test-support` (fsl-semantics/tests/it/main.rs:109-110); the handoff's TEST-RUN command (3) `cargo test -p fsl-semantics --test it -- refinement_declarations` omits the feature and would select zero tests, which is no coverage of FR-1020-AC-2. | fsl-semantics/tests/it/main.rs:109-110 |

## Coverage

- Criteria in scope: 11 FSL criteria; 10 tagged, 1 untagged (FR-1031-AC-4); 0
  tagged only by ignored tests (both former `#[ignore]` stubs were un-ignored).
- FSpec criteria tagged from this repo: FR-820-AC-1, -2, -4, -5 and FR-380-AC-9;
  FR-820-AC-3 untagged here.
- Runtime evidence: none (Draco TEST-RUN pending; compile-only checks reported by
  the coder).
- Plan completion: not assessed

## Dispositions

Round 1 — head c410fe3190700371f7a5dc2c5da261e90c7ea938, matrix recomputed with `quire matrix --scope /home/peter/dev/worktrees/qsl-2247` (quire 0.52.0). Measured runtime exists for eec1d1c35d only: root 24 pass / 4 fail (two transfer failures plus two compile_command cases reproduced identically on parent 37824e, so inherited), replay lib 4, semantics it (test-support) 31, semantics lib 26, analyze it 57 pass / 1 fail (empty certificate). The c410 runtime batch is pending.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-1031-AC-4 is now `tagged` by `an_inadmissible_snapshot_or_supplied_premise_is_refused_at_admit` (stage `admit`, `invalid-value`, the claims pointer, no `refinement_transfer` member). Its input is a wrong-slot identity rather than "another abstract package", but both go through the same `slot_key` mismatch branch. It also adds the FR-106 snapshot row. Runtime pending. |
| FND-002 | still-open | The `CanInc` and `OverThird` assertions now match the code (transfer.rs:430-466). FR-1031-AC-3's bounded-profile temporal clause case still has no test. The code path exists (transfer.rs:437-444), the fixture needs one `temporal … using <bounded profile>` declaration, and no ruling authorizes deferring it. |
| FND-003 | fixed | FR-1030-AC-5 asserts `cause.result` and `slots[1].value` `refuted` (plan_export_surface.rs:1727-1728). FR-1030-AC-3 now distinguishes consumption: a supplied `refuted` refinement premise gives slot 0 `refuted` where in-process would give `proved` (1799-1809). The AC-1 certification concern is gone with the fallback, and the empty-certificate test guards it. |
| FND-004 | still-open | Fixed: the FSpec-FR-820-AC-1 test now seeds `certified` and asserts `fairness.implied` and `premise_form`, and AC-4 now covers `unsupported`, `incomplete` and `refuted` at slot 1. Still missing: rule 2's `transfer-not-preserving{fairness}` reason; FR-820-AC-2's `any` row; a command-level test that `step_for_step_fact` emits `time: dense` (`a_dense_time_refinement_is_not_step_for_step_before_any_row` hand-writes the fact and only shows the planner echoing it). |
| FND-005 | fixed | The NotTranslated arm is removed. Analyze acceptance of `stutter_closure` belongs to FR-529's member, which FR-1030 Inputs cites. |
| FND-006 | fixed | Draco ran the feature-enabled `refinement_declarations` on eec1d1c35d: 31 passed. The coder's fix1 TEST-RUN command 3 carries `--features test-support`. |

## New findings (disposition pass 1)

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-007 | medium | FR-1030-AC-1 and FR-1020-AC-1 are now `tagged-by-ignored-test`: their only binder, `a_step_for_step_refinement_transfers_previous_and_interval_clauses_proved`, is `#[ignore]`d because no core checker certifies TP-3 past/interval forms. Arch ruling A keeps certifying them in scope on QSL-2247, which stays open. Both ACs are unmet on this PR; the expected disposition is `deferred: QSL-2247 remaining scope (ruling A)`, and the ticket must not be closed. | tests/it/plan_export_surface.rs:1634-1641 |
| FND-008 | medium | The negative oracles are too weak to catch the regressions they guard. PrevSafe and SoonTwo (plan_export_surface.rs:1703) and StuckTwo (1858) assert only `cause.result != "proved"`, not the typed outcome ruling A requires (`failed` for the uncertifiable TP-3 premise, `unsupported` from `ClosureStutterLoop`). No test feeds `check_components_over_stutter_closure` a component certificate that is valid over the plain graph but not over the closure (e.g. `Trivial` at a non-terminal state). Reverting code-review FND-002's checker to plain-graph checking would still pass every test, because EventuallyTwo stays `certified` either way. | tests/it/plan_export_surface.rs:1696-1704, 1830-1859; fsl-replay/src/certificate/closure.rs:322-341 |
