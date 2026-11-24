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

Manually checked the owning criteria, their tagged tests, and the changed package route and row-emission code. The matrix command did not return within 30 seconds, so this is a partial, scoped traceability review rather than an engine-backed repository audit.

## Verdict

**CONDITIONAL** — the scoped ACs have tagged tests, but package candidate resolution lacks an assertion for the release-backed path, and the computed matrix and repository-wide reverse-gap/stub checks remain incomplete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new package route's release-backed candidate branch has no test that resolves a package request; the marker test checks only emitted package rows. | fsl-analyze/tests/it/analyze_package_schedulability.rs:150-191; fsl-analyze/src/analyze/document/package_run/schedule.rs:205-214 |

## Coverage

- FR-246-AC-4: tagged package/source parity regression for named `en-6` on a non-release-backed claim; the existing TC-2961 tests cover the requirement-record path.
- FR-246-AC-5: tagged package-row test asserts schedulable `true` and response `false`; the existing source-route test covers release-backed classification and candidate routing.
- FR-075-AC-23: existing tagged source-route tests cover the specified release-backed and non-release-backed candidate sets; the new package test covers only the non-release-backed named case.
- `quoin matrix --repo . --json` and `quire matrix --scope . --format json` each timed out after 30 seconds without output. Manual scoped tag inspection found tags for all three owning ACs, but this is not a computed matrix result.
- Reverse-gap and stub checks were limited to the changed production files: no new unowned behavior or source stubs were found. A repository-wide audit was not completed.
- Plan completion: not assessed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The new test reads the package row's `release_backed` value, then directly calls `Registry::candidates` and `Registry::restrict_to_task_product`. It does not invoke the package analysis route (`ClaimReads::schedule`), so it still passes if that production path ignores the release-backed row. Add an assertion over a named EN-6 package request's actual candidate output. |
