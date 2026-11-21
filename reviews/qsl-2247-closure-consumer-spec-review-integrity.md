---
id: SR-2026112118004026158502
title: "spec-review integrity of PR3360 closure-stutter replay consumer contract"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR3360 78e47c7ccb06352af03673036e9e24a37ecb1bb1 over base 158cc4328568a0c864b90b07105a4276f19d6f0e; spec/functional/FR-128-replay-a-model-counterexample.md, spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md, spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md"
review_set: subset
---

## Summary

Ticket: QSL-2247. New AC ids are contiguous: FR-128-AC-10..14 and FR-1030-AC-6..7. TC-2920's scope is updated to AC-1..AC-7. The relationships resolve. Two consistency gaps remain: the test case that is meant to verify the new FR-128 criteria was not updated, and the FR-128 Inputs prose still contradicts the new identity input.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-128-AC-10..14 cite TC-523, but TC-523's Scope is still FR-128-AC-1 to AC-4 and FR-330-AC-20, and it has no procedure for them | spec/test-cases/TC-523-replay-a-model-counterexample.md:21 |
| FND-002 | low | FR-128 Inputs still lists loop steps as transitions or terminal-stutter only, and says replay reads only the clause node, the occurrence key and the fairness set, against the new stutter_closure identity input | spec/functional/FR-128-replay-a-model-counterexample.md:98-110 |

## Scope and evidence

- FR-128-AC-10..AC-14 (spec/functional/FR-128-replay-a-model-counterexample.md): examined, the id sequence and the TC column
- TC-523 (spec/test-cases/TC-523-replay-a-model-counterexample.md): examined, not in the diff, FND-001
- FR-128 Inputs (spec/functional/FR-128-replay-a-model-counterexample.md): examined, FND-002
- TC-2920 (spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md): examined, the scope line and steps 6 and 7 are consistent with FR-1030-AC-6..7

## Dispositions

Disposition pass 1, checked against the frozen fix 101574eb2b8a5a7102740deee9ef6dad08f5cc67.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The existing TC-523 (relationship: verifies FR-128) now scopes FR-128-AC-10 to AC-15, with procedure step 7 and an expected result for step 7. |
| FND-002 | fixed | The Inputs step list now names closure-stutter for a flag item, and the "reads none of these" sentence now says the flag comes from the admitted item, not the envelope. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | TC-523's new Step 7 expected result is placed before the Step 6 bullet, out of order with the procedure | spec/test-cases/TC-523-replay-a-model-counterexample.md:70-71 |
