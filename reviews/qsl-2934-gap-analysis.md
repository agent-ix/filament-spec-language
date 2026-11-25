---
id: SR-967175026302197768900474181941935084854
title: "Gap analysis of FSL PR #3892: package-route pre/post operation-contract items"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@34f048e03595db5bb0a5f93a5d2413f883077946; PR #3892 against origin/main; FR-940-AC-6, AC-7, AC-13, AC-28, AC-33 to AC-38, FR-962-AC-1, FR-489-AC-2"
review_set: subset
---

## Summary

Ticket: QSL-2934. I computed `quire matrix --scope . --format json` at head and saved it as `sr-3892/matrix.json`.

- FR-940-AC-28 and FR-940-AC-33 to AC-38 are untagged at head, as they are on main. They are the remaining scope of QSL-2934; this PR is a partial red-main fix and does not claim them.
- The PR's rewritten tests now assert part of AC-28: package/source record and exit equality for `requires_bound` and for the negotiation outcomes. They carry other tags (FR-489-AC-2, FR-940-AC-7), so AC-28 still shows untagged.
- FR-962-AC-1 (2 binders), FR-489-AC-2 (3), FR-940-AC-6 (117), AC-7 (2) and AC-13 (25) are tagged.
- Reverse gap: the production change, removing the pre/post early refusal, is owned by FR-940 "Requirement records" and FR-962-AC-1. It is not unspecified.
- Stub scan of the diff: no `todo!`, `unimplemented!` or placeholder arm. The evaluator gap is an `unadapted` item with no work, recorded in the code-review SR as FND-001.

Plan completion: not assessed

## Verdict

**FAIL for QSL-2934 completeness; not a merge blocker for this PR.** FR-940-AC-28 and AC-33 to AC-38 are untagged. They are the ticket's disclosed remaining scope, so QSL-2934 must not close on this PR. Semantic review was not requested, so it was skipped. Intent checks of the touched tests are in the code-review SR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-940-AC-28 and FR-940-AC-33 to AC-38 have no tagged test at head. AC-28's `requires_bound` and negotiation parity is asserted, but by tests tagged FR-489-AC-2 and FR-940-AC-7. Evaluation parity (AC-28's case-limited `ContractClaim` settlement, AC-34 to AC-37) has no test and no implementation on the package route. | spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md:898, 903-908; fsl-analyze/tests/it/analyze_package_selection.rs:719 |

## Coverage

- Matrix: computed by quire 0.36.10 at head. The rows for the criteria in scope are listed in Summary.
- Focused tests: see the code-review SR (58 passed, 0 failed at head).
- Semantic review: skipped (not requested).
