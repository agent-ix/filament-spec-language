---
id: SR-9999999999999999999999999999999999992
title: "Gap analysis — QSL-3108 Set/Bag min/max"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-701-AC-1/2/3; TC-1907; changed code and tests in PR #3701 at c04ac782a1e8a97ab3d9e7ec34cf19230e51c65d; QSL-3108"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-701", type: references}
---

## Summary

Planless, diff-bounded assurance review of the QSL-3108 implementation chain from FR-701 through TC-1907, trace tags, and changed checker/evaluator code. The changed criterion is tagged and its core Int Set/Bag paths are exercised, but the AC's bounded Rational Bag reduction variant lacks direct coverage; plan completion was not assessed.

## Verdict

**CONDITIONAL** — the computed matrix binds FR-701-AC-3 to changed tests, while the explicit bounded Rational Bag reduction case is not represented in those tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test checks or evaluates `reduce<R>` with `min`/`max` over the AC's bounded Rational Bag; the checker tests reductions only for Int Set, and the P5 reduction property uses bounded Int collections. | FR-701-AC-3; TC-1907 |

## Coverage

- Reconciliation: `quire matrix --scope . --format json` (Quire 0.36.10; Quoin 0.36.10) — matrix reports FR-701-AC-1, AC-2 and AC-3 tagged. The matrix reports changed checker tests as binders for AC-3; their `#[trace("TC-1907", "FR-701-AC-3")]` attributes were inspected.
- Scope: FR-701-AC-1, AC-2, AC-3 and TC-1907; changed grammar, checker, form, checker test, and evaluator test files.
- Reverse-gap/stub inspection: no changed production behavior outside FR-701 was found; no new stub was found. Broader repository reverse-gap findings were not assessed beyond the changed files.
- Semantic review: skipped as a separate pass; code/test intent was inspected for the changed paths.
- Plan completion: not assessed
- Evidence: no run evidence read

## Dispositions

Disposition pass 1 at head `b24fd2f15b1c6ee73cd240b76c9951c40de5ff5e`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The checker case now includes bounded Rational Bag min/max reductions, and the evaluator test exercises both reductions across original, reversed and rotated input orders. |
