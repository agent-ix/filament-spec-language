---
id: SR-2026112118004026158503
title: "spec-review EARS of PR3360 closure-stutter replay consumer contract"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR3360 78e47c7ccb06352af03673036e9e24a37ecb1bb1 over base 158cc4328568a0c864b90b07105a4276f19d6f0e; spec/functional/FR-128-replay-a-model-counterexample.md, spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md, spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md"
review_set: subset
---

## Summary

Ticket: QSL-2247. quire 0.52.0 (engine 0.62.2) warns on FR-128 lines 186, 196, 204 and 330. Lines 186, 196 and 204 are new in this diff. Line 330 is inherited: it is parent line 270, unchanged, and is not reported. FR-1030 and TC-2920 raise no EARS warnings. The other new statements use Where/When and the executor as subject.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | New FR-128 bullet at line 204 has three SHALL obligations (both routes read; an unread route keeps refusing; it SHALL NOT be reported) in one statement | spec/functional/FR-128-replay-a-model-counterexample.md:201-205 |
| FND-002 | low | New FR-128 sub-bullets at lines 186 and 196 use a non-canonical "After" trigger with the pronoun "it", and a subject-less "Every route ... SHALL" form | spec/functional/FR-128-replay-a-model-counterexample.md:186-197 |

## Scope and evidence

- FR-128 line 186 (spec/functional/FR-128-replay-a-model-counterexample.md): examined, new, FND-002
- FR-128 line 196: examined, new, FND-002
- FR-128 line 204: examined, new, FND-001
- FR-128 line 330: context_only, inherited from parent line 270
- FR-1030 closure bullet (spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md): examined, no warning

## Dispositions

Disposition pass 1, checked against the frozen fix 101574eb2b8a5a7102740deee9ef6dad08f5cc67. quire 0.52.0 on the four changed files reports only line 340, the inherited "before" warning (was 330, parent 270).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The routes bullet is split into separate statements, one obligation each. The ears:non-singular warning is gone. |
| FND-002 | fixed | The statements are rewritten as "When re-execution ends ..., the executor SHALL refuse" and "The routes that read a user trace ... SHALL keep refusing it". The warnings at the old lines 186 and 196 are gone. |
