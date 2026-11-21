---
id: SR-2026112118004026158504
title: "spec-review dependency of PR3360 closure-stutter replay consumer contract"
type: SpecReview
analysis: dependency
scope: "agent-ix/filament-spec-language; PR3360 78e47c7ccb06352af03673036e9e24a37ecb1bb1 over base 158cc4328568a0c864b90b07105a4276f19d6f0e; spec/functional/FR-128-replay-a-model-counterexample.md, spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md, spec/test-cases/TC-2920-request-a-refinement-transfer-from-a-native-run.md"
review_set: subset
---

## Summary

Ticket: QSL-2247. Three new edges point the right way and add no cycle. FR-1030 depends_on FR-128 because the transfer consumes FR-128's writer, reader and replay. FR-128 depends_on FSpec FR-362 for fairness with no taken identity, and on FSpec FR-368 for the stutter_closure item member and obligation identity. FSpec owns the wire and semantics; FSL owns the replay and adapters. No upstream API, pin or request member is added, and FSpec #795 is merged. The TP-3 on each and protocol modes are left to a separate related ticket, not to this requirement. The closure code follows this contract's merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope and evidence

- FR-1030 relationships (spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md): examined, depends_on FR-128
- FR-128 relationships (spec/functional/FR-128-replay-a-model-counterexample.md): examined, depends_on FSpec FR-362 and FR-368
