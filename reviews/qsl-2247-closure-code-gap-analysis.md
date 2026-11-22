---
id: SR-2026112120444515239801
title: "Gap analysis of PR3378 closure-stutter witness and replay consumer (FR-128-AC-10..15, FR-1030-AC-6/7)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3378 f092056d7021d55ee93179645fb3f02eb806e315 over base dab8c60624b4427e2290efe7ebaa3827f5ae0842; FR-128-AC-10..15, FR-1030-AC-6..7 against fsl-analyze/tests/it/{closure_replay,clause_item,model_settle,protocol_bounded_lasso,refinement_replay,zone_replay}.rs, fsl-replay unit tests, tests/it/plan_export_surface.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: references
---

## Summary

Ticket: QSL-2247. This is a planless gap analysis bounded to the PR's diff, with plan completion not assessed. I checked each AC against the tests tagged to it, read in full, and against the production path those tests exercise. The computed `quoin matrix` sweep was not run because the brief bounds this review to the diff, so this is a manual AC-to-test audit of the eight ACs and not a whole-repository matrix. No test has been run (the coder reports compile checks only), so every binding below is an intent check and not green evidence.

QSL-2247's own four acceptance criteria are about `previous` in refinement (`fsl-semantics/tests/it/refinement_previous.rs`, `fsl-analyze/tests/it/refinement_previous.rs`). This PR touches none of them, and neither file exists at this head. This PR therefore gives no evidence toward closing QSL-2247.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-128-AC-11's task-derived case has no test on any task-derived entry; the model entry refuses a task item with TraceAlphabet{required,supplied} with no pointer, and the typed task entry does not refuse at all | fsl-replay/src/outcome/counterexample.rs:116 |
| FND-002 | medium | FR-128-AC-11's hybrid case cannot be built: no hybrid entry replays a TemporalCounterexample (none in fsl-hybrid-reference/src or fsl-replay/src/hybrid_replay), so the AC needs a spec ruling, and closure_alphabet_tests is a helper test, not entry evidence | spec/functional/FR-128-replay-a-model-counterexample.md:382 |
| FND-003 | medium | FR-1030-AC-6's test asserts only the slot outcome; that the written lasso reads back through FR-128's reader and replays to a reproduction is not asserted for StuckTwo, and in-process slot replay uses the typed envelope without a writer/reader round trip | tests/it/plan_export_surface.rs:1995 |
| FND-004 | low | FR-1030-AC-7's test does not assert that slot 1 is refuted through a closure-only lasso with a one-step real prefix, only the transfer outcome | tests/it/plan_export_surface.rs:2017 |
| FND-005 | low | FR-128-AC-11's timed case never selects the flag on the request side as the AC states; the protocol case loops over both values, the timed case uses the fixture's false | fsl-analyze/tests/it/zone_replay.rs:250 |
| FND-006 | low | FR-128-AC-13's undefined-atom case asserts the replay NoValue cause but not "the item cause ReplayParity" | fsl-analyze/tests/it/closure_replay.rs:448 |

FND-001: the coder's report says this honestly. Two fixes are possible: test `replay_task_product_trace`/`read_task_temporal_document` with a `closure-stutter` step at `/loop/0/transition/type`, which the reader already refuses, after fixing code-review FND-001; or get a ruling that the `TraceAlphabet` item refusal satisfies the case.

FND-002: this is a bounded blocker that needs a spec owner's ruling. The AC marks these cases "proposed" on the FR-193 fixtures. Either the hybrid case is struck, or the spec names the entry it means. Do not satisfy it with a helper test.

FND-003: clause_item.rs `a_closure_violation_settles_refuted_through_its_written_and_replayed_lasso` shows the mechanism on the Counter world and says so in its doc comment, but it is not tagged FR-1030-AC-6 and is not the CasRefinesCounter transfer.

## Verdict

FR-128-AC-10, AC-12, AC-14 and AC-15 have direct, correctly bound tests over the production entries, including both routes and the reference-literal route. In AC-15 the package route is a positive replay and not a refusal. AC-11's model, reference-literal, protocol, timed, refinement and invariant cases are covered. Its task case is missing (FND-001) and its hybrid case cannot be built (FND-002). AC-13 is covered apart from one item-level assertion. FR-1030-AC-6/7 have outcome-level tests only (FND-003, FND-004). Nothing has been run.

## Scope and evidence

- FR-128-AC-10: examined, bound to closure_replay::a_closure_lasso_reproduces_over_an_empty_and_a_non_empty_stem
- FR-128-AC-11: examined, FND-001, FND-002, FND-005
- FR-128-AC-12: examined, bound to closure_replay::a_closure_lasso_is_fair_only_where_the_constraint_is_disabled
- FR-128-AC-13: examined, FND-006
- FR-128-AC-14: examined, bound to the two outcome::counterexample unit tests and spine::request::tests::an_observed_model_step_refuses_the_closure_stutter
- FR-128-AC-15: examined, bound to clause_item::a_closure_lasso_replays_alike_on_the_source_and_the_package_routes
- FR-1030-AC-6: examined, FND-003
- FR-1030-AC-7: examined, FND-004

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | The two original previous-transfer acceptance tests now run on a unit with BoundedTwo removed, which routes around a NotEmitted failure on main and does not fix it: since #3329 (1ccd5cfc, after the TP3 merge 158cc432, at which the root evidence report records these tests passing on the full unit), a transfer whose unit declares an unselected configured clause ends resource_exhausted, even when that clause is not transferred | tests/it/plan_export_surface.rs:2057 |
| FND-008 | low | The FR-128-AC-11 task-derived case is asserted inside missed_deadline_settles_only_with_a_replayed_positive_time_lasso, which is tagged only TC-3008/FR-249-AC-9/FR-237-AC-8, so the matrix binds no test of that case to FR-128-AC-11 | fsl-analyze/tests/it/task_product.rs:526 |

FND-007: this is grounded in the source only. I did not run anything on main. `git log -S hold_item 158cc432..cf8d8582` shows #3329, whose message says it holds "the whole item (counterexample, record, ...)", and an unselected `ev` clause makes `emit_checked` refuse (fixture report: MissingTemporalClockSelection). The assertions are unchanged and the omitted clause is unused, so the previous-transfer rule is still tested. The product behaviour stays defective: an unrelated configured clause makes any held-id transfer fail. The full-unit `BoundedTwo` case is exposed to the same thing. A discriminator for the root: run the two tests on main cf8d with the full unit. If they fail, the regression belongs to #3329/QSL-2744 and should be deferred there; if they pass, this PR caused it.

## Dispositions

Disposition pass 1, checked against frozen head 4c9cfbd385be4faef17d13a13dfed5424ca93dd6 over base cf8d85826db560569cd8806cce07007d4b493e0c. I read the final source, the assertions and the spec rows. Per the root's results file, the tests behind FND-001/005/006 and both clause_item round trips passed at 8ae3989f (Analyze it 7/7). The two root closure transfers and the emit/read control passed there too (3/3).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | task_product.rs now replays a typed task counterexample with a closure step at Prefix(0) and at Loop(0) through the public replay_task_product_trace and asserts TraceAlphabet at `<site>/transition/type`, against a wrong-key control (the trace tag is FND-008) |
| FND-002 | fixed | under the architecture ruling, FR-128-AC-11 strikes only the unreachable hybrid case and says plainly that no hybrid entry accepts a temporal counterexample; no API was added, and the helper test is not used as evidence |
| FND-003 | fixed | under the architecture ruling, FR-1030-AC-6 now rests on three parts: slot 1 refuted only through the production replay gate, lossless written and read equality (clause_item asserts read == unbound, whole struct), and the post-read replay over the source and package routes; the package engine is Refuted with record equality; the phantom transfer-output lasso assertion was removed |
| FND-004 | fixed | FR-1030-AC-7 is reworded the same way; clause_item::a_closure_violation_after_a_real_step_settles_refuted_through_its_replayed_lasso (tagged FR-1030-AC-7) asserts a stem of at least 1 and the same round trip; the root PendingTwo slot-1 refuted assertion is kept |
| FND-005 | fixed | zone_replay::a_timed_request_selecting_the_flag_still_refuses_a_closure_step runs replay_model_trace with stutter_closure false and true, and both refuse Alphabet at /loop/0/transition/type |
| FND-006 | fixed | closure_replay::a_closure_lasso_the_evaluator_does_not_refute_disagrees now settles the item through settle_model_check_over_stutter_closure and asserts TerminalValue::Inconclusive(ReplayParity(NoValue)) with no counterexample |

Runtime cutoff (root Titan results file, read at 14:12 PST): 13/13 unique tests passed across mixed heads, 11 at 8ae3989f and the two original previous-transfer root tests at 4c9cfbd385; 8ae..4c9 differs in tests/it/plan_export_surface.rs alone.

## Dispositions (pass 2)

Disposition pass 2, narrow: it covers only the new findings FND-007 and FND-008, checked against frozen head 806445b30b8d5e07073efe9e68bd5920f8604e14 (parent 4c9cfbd385, base cf8d85826d unchanged). The diff from 4c9 to 806 is one test trace attribute, two Markdown files and the three SR copies; no production code or test body changed.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-007 | deferred | Architecture ruled that ownership of the full-unit BoundedTwo, MissingTemporalClockSelection and held-ids NotEmitted interaction lies with the existing QSL-2744 (#3329), whatever the causality runs show. #3378 merges on the valid emittable fixture, with the previous-transfer assertions unchanged and its 13 focused tests passing. The deferral is recorded once on QSL-2744 in comment 2f361160-1684-4fc1-8db5-b1520a83b6e3. |
| FND-008 | fixed | missed_deadline_settles_only_with_a_replayed_positive_time_lasso now carries #[trace("TC-523", "FR-128-AC-11")] beside its existing tags, and the body is byte-identical |
