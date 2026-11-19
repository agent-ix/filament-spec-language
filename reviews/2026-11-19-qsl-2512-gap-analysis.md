---
id: SR-2026111919340478491681406
title: "Model-based result record traceability gaps"
type: SpecReview
analysis: gap-analysis
scope: "FR-752; FR-753; fsl-eval model-based run tests; fsl-command plan request and outcome"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-752
    type: references
---

## Summary

Repository-driven traceability check of the model-based result slice. The computed matrix retains public-entry and admission gaps; the result-generation tests alone do not establish those criteria.

## Verdict

FAIL for complete FR-752 acceptance. The partial result-generation slice is independently reviewable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-8 has no trace-bound run-level admission-double test. Equal-input record comparisons now exercise cases, checks, partitions and their summary, but do not establish the separately owned admission seam. | FR-752-AC-8 |
| FND-002 | medium | AC-11 has no trace-bound test through the specified package/subject entry. The internal regression covers a coverage-limited run's case invariant and tested settlement only. | FR-752-AC-11 |
| FND-003 | medium | AC-3 and the plan half of AC-7 require the coordinator adapter; internal runner tests do not establish whole-plan bounds, disposition mapping or blocked dependents. | FR-752-AC-3, FR-752-AC-7, FR-753-AC-5 |

## Coverage

Reconciliation: quoin matrix --repo /Users/peter/dev/qsl-2512-code --json, quoin 0.36.7, quire 0.52.0 (engine 0.62.2). FR-752 has nine tagged criteria and two untagged criteria (AC-8 and AC-11). No recorded run evidence was bound in the matrix.

Plan completion: not assessed

The retained ignored public-entry/operation-coverage stubs were inspected as separately owned work, not counted as passing evidence. The Text limit test is tagged to AC-10 but uses the existing internal runner; the public-entry part remains dependent on its owner. Source behavior is owned by FR-752 and FSpec FR-190; no new requirement was invented. Optional semantic review was not run.

