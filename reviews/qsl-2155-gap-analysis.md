---
id: SR-2026111612172400000047
title: "Gap analysis — filament-spec-language QSL-2155 Clopper–Pearson"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language repository audit; PR #2966 changed production and test files"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/spec
    type: references
---

## Summary

Ran the repository-wide Quoin Test Matrix and Quire coverage reconciliation for the production-code change. The matrix has pre-existing repository-wide untagged and ignored-only criteria; within this change, the coverage integration test tagged to FR-189-AC-16 does not test its stated fixed-vector requirement, while separate fixed-vector tests do bind that criterion.

## Verdict

**FAIL** — the repository-wide matrix contains 1,146 untagged criteria, and 294 additional criteria are backed only by ignored tests. The changed files introduce no untracked source symbols in the Quire coverage census.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The repository-wide Test Matrix leaves 1,146 criteria untagged and 294 tagged only by ignored tests. | quoin matrix: repository totals |
| FND-002 | medium | The newly added coverage test is tagged to FR-189-AC-16 but does not exercise that criterion's fixed N=231 decision vector; other tests do bind the criterion. | fsl-analyze/tests/it/statistical_coverage.rs:58-78; FR-189-AC-16 |

## Coverage

- Scope: the full `spec/`, source tree, and test tree under this repository, as required by planless gap analysis.
- Quoin matrix: 5,789 criteria total; 4,233 tagged, 294 tagged only by ignored tests, 1,146 untagged, 103 method-without-symbol rows, and 13 retired rows.
- Quire coverage: 5,702 criteria reconciled; 4,537 backed matrix rows out of 5,723, 1,141 unbacked rows, and no run evidence bound in the repository evidence store. The coverage census reported zero untracked source symbols in changed statistical source/test paths; it did report trace-id-outside-marker diagnostics across changed statistical files, including new tagged tests. Those diagnostics do not change the Quoin static statuses above.
- Changed criterion FR-189-AC-16 is statically tagged. The new integration coverage function's trace tag does not assert the AC's fixed N=231 vector; the new method unit tests `clopper_pearson_decisions_match_the_fixed_sample_vector` and `clopper_pearson_known_intervals` do bind that AC.
- Reverse-gap and stub inspection of the changed production symbols found no new unowned implementation surface or live-path stubs.
- Semantic review was skipped because it was not explicitly requested. Plan completion: not assessed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | This repository-wide baseline is outside QSL-2155 and does not block it; the planner routed it to luna:QSL:code. |
| FND-002 | fixed | The integration coverage test no longer carries the FR-189-AC-16 trace tag; separate method unit tests bind and exercise that fixed-vector criterion. |

### Disposition evidence

FND-002 after_excerpt:

```rust
#[trace("TC-624")]
#[test]
fn clopper_pearson_coverage() {
    let world = world(CLAIM);
    let true_probability = ratio(1, 2);
```

The other tagged unit tests continue to bind `FR-189-AC-16` and assert its N=231 fixed vector.
