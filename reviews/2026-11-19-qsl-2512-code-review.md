---
id: SR-2026111919340478491681405
title: "Model-based result accounting delta review"
type: SpecReview
analysis: code-review
scope: "FR-752; fsl-eval/src/simulation/model/model_based_test.rs; model.rs; operation_coverage.rs; model_based_run.rs; model_operation_coverage.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-752
    type: references
---

## Summary

Author review of the result-accounting delta over the existing partial implementation. The delta funds Text count tables before allocation, records accounting stops, counts evaluated and skipped points, retains ungenerated partitions, and excludes reachability-only partitions from case counts. It uses the existing scalar meter and native oracle.

## Verdict

CONDITIONAL. This is a partial result-generation slice; public-entry readiness and independent PR review remain outstanding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The existing public runner still takes a prebuilt ModelSystem and returns runtime errors separately from records. The package/subject entry, admission double seam, and complete refusal/settlement split require the separately owned entry integration. | FR-752 Entry, FR-752-AC-8, FR-752-AC-11 |
| FND-002 | medium | The plan reader carries and echoes both bounds, but enforcing missing consumed members with the first consuming step and package-identity precedence requires the coordinator integration. | FR-753, FSpec FR-824 |

## Coverage

Reviewed the five evaluator production/test files in this delta against merged FR-752 and FSpec FR-190, preserving the separately owned operation-coverage stubs and public entry. No applicable AssuranceProfile was found. No CI workflow or qualification trigger changed. Checks reuse the existing kernel meter; no alternate evaluator, copied contracts, or new implementation language was introduced.

Focused verification is recorded in the PR body. No aggregate gate was run. This author review does not replace independent PR review.

