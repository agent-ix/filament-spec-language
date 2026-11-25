---
id: SR-967175026302197768900474181941935084826
title: "Gap analysis — filament-spec-language QSL-2832 package schedulability"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-246-AC-4, FR-246-AC-5, FR-075-AC-23; package schedulability route and tagged integration tests"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-246
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-075
    type: reviews
---

## Summary

Manually checked the owning criteria, their tagged tests, and the changed package route and row-emission code. Follow-up disposition is based on the QSL-2832 branch rebased onto `origin/main` `d4ad338ea0`; the new release-backed route test asserts the exact later task-product refusal. The confined matrix now completes, but the broad ACs still include cases outside this ticket's tests.

## Verdict

**Disposition update: FND-001 addressed in the test source.** The release-backed route is exercised and asserts `en-6` plus `Pending(TaskProduct)`. Its focused rerun is paused by the host build guard, so the new assertion's execution is pending. Scoped matrix coverage remains partial for the broader ACs; repository-wide reverse-gap/stub checks were not performed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The package route's release-backed candidate branch lacked a test that resolved a package request. | fsl-analyze/tests/it/analyze_package_schedulability.rs; fsl-analyze/src/analyze/document/package_run/schedule.rs |

## Coverage

- FR-246-AC-4: tagged package/source parity regression covers the named non-release-backed `en-6` case; the release-backed package route test also checks candidate output and its later refusal. The broader several-claim and ordering cases remain outside this ticket's package test.
- FR-246-AC-5: tagged package-row checks assert schedulable `true` and response `false`; source-route tests cover release-backed classification. Package route coverage asserts the release-backed candidate and later task-product refusal.
- FR-075-AC-23: tagged source-route tests cover release-backed and non-release-backed candidate sets; package tests cover the named no-release and release-backed `en-6` route cases. Registration-order invariance remains outside this ticket's package test.
- `quire matrix --scope . --format tsv` completed with exit 0 after rebasing. It reports FR-246-AC-4, FR-246-AC-5, and FR-075-AC-23 as `uncovered tagged` because remaining cases are broader than these ticket-scoped tests; tag presence alone is not full AC coverage.
- Reverse-gap and stub checks were limited to the changed production files: no new unowned behavior or source stubs were found. A repository-wide audit was not completed.
- Plan completion: not assessed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The package route is exercised for a named release-backed EN-6 request; literal assertions require captured candidates `['en-6']` and `PendingFamily::TaskProduct` from the later refusal. |
