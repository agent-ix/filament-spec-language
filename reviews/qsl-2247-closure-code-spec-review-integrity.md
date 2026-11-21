---
id: SR-2026112122101534856500
title: "spec-review/integrity of the PR3378 fix-round edits to FR-128-AC-11 and FR-1030-AC-6/7"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR3378 4c9cfbd385be4faef17d13a13dfed5424ca93dd6 over base cf8d85826db560569cd8806cce07007d4b493e0c; spec/functional/FR-128-replay-a-model-counterexample.md (AC-11 row), spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md (AC-6, AC-7 rows), spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md as context"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1030
    type: references
  - target: ix://agent-ix/filament-spec-language/TC-2920
    type: references
---

## Summary

Ticket: QSL-2247. This integrity review covers only the three acceptance rows that the fix round changed, as the disposition brief allows, run inside the same disposition invocation. I checked each changed row against the architecture rulings, against the FR-128 Behavior text that is still in force, and against the test case that verifies it. FR-128-AC-11 now strikes only the unreachable hybrid case and names the public typed task entry, which matches the hybrid ruling. FR-1030-AC-6/7 now give exactly the three parts the transfer ruling names: the slot is refuted through the production replay gate, the written and read lasso are equal without loss, and the read-back lasso replays after the read over the source and package routes. They also say the transfer outcome carries no lasso. TC-2920 was not updated with them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-2920 step 6 still says to read the slot 1 witness back through FR-128's reader and replay it, and its expected result still names a lasso in the slot, which contradicts reworded FR-1030-AC-6's statement that the transfer outcome carries no lasso | spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md:22 |
| FND-002 | low | TC-2920 step 7 still calls PendingTwo "proposed and buildable, not yet in the fixture, not run", but TRANSFER_DECLARATIONS now declares it and FR-1030-AC-7's test runs it | spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md:23 |
| FND-003 | low | FR-128-AC-11 says the timed, task-derived and protocol cases run "with the flag selected on the request side", but the task-derived case it names runs the public typed task-product entry, which takes no request and no flag | spec/functional/FR-128-replay-a-model-counterexample.md:382 |

FND-001 brings back the phantom requirement the transfer ruling removed: anyone following TC-2920 step 6 would look for a witness in the transfer output. The fix is to reword step 6 and its expected result (line 32) to match AC-6's three parts.

## Verdict

FR-128-AC-11 and FR-1030-AC-6/7 are internally consistent with the rulings and with FR-128 Behavior: hybrid still "SHALL NOT gain the alternative", and stating that it has no entry is consistent with that. The defects are in the test case that was not updated, plus one ambiguous clause.

## Scope and evidence

- FR-128-AC-11: examined, FND-003
- FR-1030-AC-6: examined, consistent with the ruling; TC drift is FND-001
- FR-1030-AC-7: examined, consistent with the ruling; TC drift is FND-002
- TC-2920 steps 6-7 and expected results: context_only, read to check verification consistency

## Dispositions

Disposition pass 1, checked against frozen head 806445b30b8d5e07073efe9e68bd5920f8604e14 (parent 4c9cfbd385, base cf8d85826d), against the FR-1030 transfer ruling, the flagless-task-entry ruling and the replay_task_product_trace signature, which takes a product and a counterexample but no request or flag.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | TC-2920 step 6 and its expected result now state that the outcome carries no lasso, that slot 1 is refuted only through the production replay gate, and that the written/read equality and post-read replay are asserted by the FR-128-AC-14/15 tests; this matches FR-1030-AC-6 |
| FND-002 | fixed | step 7 now uses PendingTwo as declared in TRANSFER_DECLARATIONS, without "proposed, not yet in the fixture, not run"; plan_export_surface::a_closure_violation_after_a_real_step_refutes_its_premise runs it (tag TC-2920/FR-1030-AC-7) |
| FND-003 | fixed | FR-128-AC-11 now limits "with it selected" to the timed and protocol entries, which carry the request's flag, and says the typed task-product entry takes no flag and refuses a closure step categorically, which matches the source |
