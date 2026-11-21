---
id: SR-20261121152319
title: "spec review of PR 3324 FR-488 reserved slot-name region sentence"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR 3324 head c50d5340c14486339e7d5cfa8931917de0855e6f; spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md"
review_set: subset
---

## Summary

Ticket: QSL-2581. The rewritten FR-488 sentence states system behaviour (no change-log wording) and matches the code for every kind it names, including the FR-103 model-operation case. AC-29 was not updated, so the new token-region rule has no criterion that can fail, and the sentence's list of binder kinds is narrower than the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-29 still says "at the parameter's declaring source region", which the old whole-form location also satisfied; the new identifier-token tests are tagged to a criterion that does not state their behaviour | spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md:548 |
| FND-002 | low | The region sentence lists binder kinds but omits reduce, collection count/sum and population-form binders that the code also locates at the token, and drops the SHALL of the sentence it replaces | spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md:336-340 |

## Verdict

Fix round needed for FND-001 (spec-only edit). FR-103 cross-reference checked: Origin::ModelOperationClause resolves to no region (fsl-semantics/src/check/region.rs:183-186), matching "the refusal names no region for it".
