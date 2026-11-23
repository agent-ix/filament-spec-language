---
id: SR-204
title: "Base specification review — QSL-2910 FSL caller limits"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; FR-255 lifecycle caller-limit coverage and TC-721"
review_set: subset
---

## Summary

Reviewed the added caller-limit contract and its test procedure for requirement identity, operation availability, exact names, boundary/error coverage, links and criterion coverage. One medium gap remains: the new criterion has no implementation-test binder in the current Test Matrix.

## Verdict

**CONDITIONAL** — the criterion is currently untagged; bind its implementation tests when the code change lands.

## Assurance Context

No applicable `AssuranceProfile` exists in this repository's specification scope.

## Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-255-AC-24 | examined | The analyze, monitor and replay command entry points accept exactly the operation-relevant caller-limit settings enumerated in Lifecycle caller-limit coverage; each accepted setting changes only its matching bound and unavailable settings refuse before work. |
| FR-255 monitor rows | examined | Adds monitor.retained_positions, monitor.retained_identities, monitor.preparation_work_units and monitor.preparation_storage with explicit owners/defaults. |
| FR-255 setting expansions | examined | Expands grouped accounting, temporal_evidence, model_check, exact_prob, statistical, zone_check, smt_encoding and certificate-check names into exact spellings. |
| TC-721 step 11 | examined | Exercises each operation-specific setting, monitor preparation bounds, operation-unavailable settings and replay.input_bytes' reader mapping. |
| FR-255-AC-3, AC-4 and AC-12 | context_only | Existing mapping uniqueness, applicable-entry-point and exact-name requirements. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-255-AC-24 is untagged in the computed Test Matrix | FR-255-AC-24; TC-721 step 11 |

The matrix reports FR-255-AC-24 as `untagged` with no binder. TC-721 describes the future test and lists the trace tag to add, but a test-case artifact does not bind the AC in the computed matrix. Add the implementation test trace(s) when the API implementation lands so the new operation-specific contract has executable evidence.


## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | The current QSL-2910 PR is specification-only. Its implementation work will add the code-test trace binding for FR-255-AC-24 when the caller-limit API and its tests land. |
