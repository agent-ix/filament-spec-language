---
id: SR-20261107070951249607
title: "Gap analysis — QSL-2193 checked handoff command scope"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository-wide computed matrix and FR-665 implementation/tests"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/spec
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-665
    type: references
---

## Summary

Ran `quoin matrix --repo . --json` across the repository and inspected FR-665's linked implementation and tests. All three FR-665 criteria are tagged, but TC-1748 only partially asserts AC-2 and AC-3 as described in the code-review finding; the repository-wide matrix also contains substantial unrelated untagged criteria.

## Verdict

**FAIL** — the required planless repository audit finds 934 active-or-unmatched untagged criteria among 5,089 criteria; the raw matrix classifies 1,117 as untagged, including 183 rows marked RETIRED in their spec text. The targeted FR-665 rows are tagged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed repository matrix reports 934 active-or-unmatched untagged criteria (and 183 additional untagged rows whose spec text says RETIRED), so repository-wide traceability is incomplete. FR-665-AC-1..3 themselves are tagged. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:451 |

## Coverage

Plan completion: not assessed.

`quoin matrix --repo . --json` (Quoin 0.35.0, engine 0.60.0) returned 0. Across 5,089 criteria: 3,609 tagged, 1,117 untagged, 252 tagged only by ignored tests, 98 methods without symbols, and 13 retired. A text scan found 183 of the untagged rows explicitly marked RETIRED; excluding those leaves 934 active-or-unmatched untagged criteria. FR-402-AC-3 is one active example. This audit is planless; FR-665-AC-1, AC-2 and AC-3 each have a test binder in `tests/it/request_surface_s1.rs`. No plan was supplied. Semantic review was skipped because it is opt-in in the gap-analysis method.
