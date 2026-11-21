---
id: SR-202611210022442313144108528
title: "Independent QSL-2158 cumulative work contract base review"
type: SpecReview
analysis: base
scope: "FR-120, FR-191, FR-192, FR-203, FR-255, FR-741: d25728d7..3354b395d7739b3376cad577ddec0f4f667f1814"
review_set: subset
---

## Summary

PASS for the frozen contract's 123 added lines and eight removed lines, reviewed independently by Codex. No actionable defect found in the changed requirements. This is a specification verdict, not an implementation, runtime compatibility, release or merge verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | (placeholder) No issues found in the scoped contract. | FR-120, FR-191, FR-192, FR-203, FR-255, FR-741 |

## Examined scope

All added/edited Description, Inputs, Behavior, relationship and setting-table text in the six files was examined. New criteria examined: FR-120-AC-35/36, FR-191-AC-8, FR-192-AC-8, FR-203-AC-13/14, FR-255-AC-23, FR-741-AC-6. Context included surrounding clause/candidate rules, FR-107's caller-owned meter, FR-189's test/sample order, ADR-024's Health example, and the owning settlement tables.

Local max_candidates remains an independent per-expansion counter. Candidate units additionally consume cumulative work once per argument vector, candidate and evaluated result pair. Clause work consumes the same run allowance, including claim predicates; other counters keep their prior scopes. Local denial precedes cumulative denial on the same candidate. Configured accounting bounds and the two finite defaults remain distinct, and typed stop provenance survives settlement.

## Validation and limits

Installed quoin 0.36.8 and quire CLI 0.52.0 (engine 0.62.2) resolved from /home/peter/.npm-global/bin/. The installed SpecReview template and schema were fetched with quoin write. Independent scoped quire validation of all six FR files passed: 6/6 grammar-clean; zero grammar findings. git diff --check passed. No active AssuranceProfile or review_policy declaration was found under spec; no enforced review run was required by a discovered profile.

Changed IDs are unique and their local references resolve in the owning context. FR-203-AC-10 through AC-12 are reserved by pending parent 3285, expressly outside this frozen branch; AC-13/14 do not overwrite or renumber them. Their absence here is not a newly lost criterion.

Criterion methods are Test and the added criteria describe observable denial, accounting, defaults, concurrency and materialization probes. Boundary/error/state-transition review covers no-result pair charging, local/cumulative tie, independent runs, cross-test exhaustion, cancellation, reused state keys and outer capacity. No new digest/pin/tracking-record antipattern was introduced. Existing provenance is semantic statistical reproduction data, not a newly added repository tracking mechanism.

No broad matrix, implementation coverage sweep, cargo command, external model or source edit was run. Untagged new contract criteria remain implementation obligations; passing this review does not assert test discharge. Existing example runtime costs under the shared default are unmeasured. This review makes no request to raise budgets, change examples or alter thresholds.
