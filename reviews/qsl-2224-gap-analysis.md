---
id: SR-2026111612172400000036
title: "Gap analysis — QSL-2224 protocol analyze routing"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-analyze protocol request routing and tests; FR-636-AC-1..4"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-636", type: references}
---

## Summary

Ran the repository Quoin matrix and inspected the FR-636 protocol request path and its tagged tests. Within FR-636, AC-1 and AC-3 have running tagged tests; AC-2 and AC-4 are tagged only by ignored tests, so those criteria remain without executable evidence.

## Verdict

**FAIL** — FR-636-AC-2 and FR-636-AC-4 have no running test evidence; the AC-4 check also fails when executed explicitly.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-636-AC-2 is backed only by an ignored test, leaving local projection and realizability unverified through the request path | fsl-analyze/tests/it/protocol_analysis_differential.rs:14 |
| FND-002 | high | FR-636-AC-4 is backed only by an ignored test; its `max_messages` case returns `refuted` rather than the required `incomplete` | FR-636-AC-4 |

## Coverage

- Reconciliation: `quoin matrix` (`quoin 0.36.6`); repository-wide result: 4,164 tagged, 1,165 untagged, 249 tagged-by-ignored-test, 103 method-without-symbol, 13 retired criteria across 621 requirements. In the FR-636 subset: AC-1 and AC-3 are tagged by running tests; AC-2 and AC-4 are tagged-by-ignored-test.
- Plan completion: not assessed
- Evidence: no run evidence read
- Untraced behaviors / stubs: no new source or test stub found in the reviewed protocol request-routing slice; reverse-gap inventory outside the PR scope was not assessed.
- Semantic review: skipped (not requested; optional under gap-analysis).
