---
id: SR-2026111612172400000041
title: "Gap analysis of QSL-2430 PR #2928"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository matrix and reverse-gap scan; PR #2928 changed request, protocol-analysis and test files; QSL-2430"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-636
    type: references
---

## Summary

Ticket: QSL-2430. The changed CLI route and tests are trace-bound to FR-636-AC-2 and AC-5..7, and the implementation changes map to the existing FR-636/FR-570/FR-571/FR-574 protocol-analysis obligations. However, TC-2988 exercises only zero as an out-of-range count and does not cover the declared upper bound, while the full repository matrix contains 1,160 untagged criteria overall.

## Verdict

FAIL for the repository-wide planless gap audit: the computed matrix reports 1,160 untagged criteria, 268 tagged only by ignored tests, 103 method-without-symbol criteria, and 13 retired criteria among 5,719 unique criteria. Within this PR's FR-636 scope, all examined ACs are statically tagged to active tests; the upper-bound test gap is identified below. The repository-wide untagged population is pre-existing and not caused by this PR. Plan completion: not assessed.

## Coverage

Ran `quoin matrix --repo . --json` with Quoin 0.36.6. Across 5,719 unique criteria: 4,175 are tagged; 1,160 are untagged; 268 are tagged only by ignored tests; 103 declare a method without a symbol; and 13 are retired. FR-636-AC-2 is tagged to `protocol_deadlock_agrees_with_projection`; AC-5, AC-6 and AC-7 are tagged to the three active tests in `protocol_item_bounds.rs`. The targeted tests passed. Inspected all nine changed files for requirement ownership and stubs; the production behavior is covered by FR-636, FR-570, FR-571, FR-574 and FR-575, and the removed ignored test was replaced by an active CLI-path test. Optional semantic review was skipped because it was not requested. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-2988 and its tagged AC-7 test cover zero bounds but not values above 2^53 - 1, leaving the upper-range refusal requirement without executable evidence. | spec/test-cases/TC-2988-protocol-analysis-item-profile-and-bounds.md:17-25; spec/functional/FR-636-request-protocol-analysis-items-and-replay-supplied-protocol-traces.md:131-139 |
| FND-002 | high | The repository-wide matrix has 1,160 untagged criteria out of 5,719 unique criteria; these have no tagged executable evidence in the computed matrix. This is a pre-existing repository baseline, not introduced by PR #2928. | spec/ |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | deferred | This is the pre-existing repository-wide traceability baseline and is outside the QSL-2430 implementation scope. |
