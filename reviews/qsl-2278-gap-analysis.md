---
id: SR-2136
title: "Gap analysis of QSL-2278 PR #2737"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2737 diff origin/main@40f224a4d12017022b0e78256d3d1d87609d9820...a44531223fd81376c09c7f2bc88a03a3436a7cda; Ticket QSL-2278; full repository matrix; focused NFR-002-AC-1..4 and TC-920"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-002
    type: reviews
---

## Summary

Ticket QSL-2278. Planless gap analysis of the full repository matrix and the behavior changed in PR #2737. Plan completion: not assessed. The four NFR-002 acceptance criteria are tagged; the new native-first, path-order-independent launcher selection and native-only diagnostic tests bind NFR-002-AC-3 and TC-920. The focused source behavior maps to NFR-002's scoped structural-validation gate and TC-920's installed module discovery and refusal controls. No changed-source stub or untraced behavior was found. The previous full repository matrix run reported substantial pre-existing coverage gaps outside this PR: 5,172 criteria, 3,623 tagged, 1,120 untagged, 318 tagged only by ignored tests, 98 method-without-symbol, and 13 retired. That run reported no evidence across the repository. This repository-wide gap set is not attributed to this PR.

## Review Refresh

Rechecked at PR head `a44531223fd81376c09c7f2bc88a03a3436a7cda` against `origin/main` `40f224a4d12017022b0e78256d3d1d87609d9820`. The source/test patch is unchanged. The previous full-repository matrix result remains the evidence for FND-001; refreshed `quoin matrix` and `quire matrix` commands did not complete in the review environment, so the old repository-wide counts are not asserted as current. The focused NFR-002 tests remain unchanged and tagged. FND-001 remains deferred to the pre-existing QSL-2083..QSL-2098 coverage backlog; this finding does not identify a gap introduced by QSL-2278.

## Verdict

**FAIL (carried forward)** — the previous full repository matrix run contained untagged and ignored-only criteria, which meet gap-analysis's fail rule. Current matrix commands did not complete, so the repository-wide count was not refreshed. The changed NFR-002-AC-1..4 criteria are all tagged, and this PR adds no gap within its changed behavior. Plan completion: not assessed.

## Coverage

- Reconciliation: `quoin matrix --repo . --json` (quoin 0.35.0, quire 0.45.0 / engine 0.60.0).
- Full repository static statuses: 3,623 tagged; 1,120 untagged; 318 tagged-by-ignored-test; 98 method-without-symbol; 13 retired (5,172 total).
- Full repository evidence status: no run evidence.
- Focused obligations: NFR-002-AC-1 tagged by `full_ci_composes_the_unrestricted_scoped_validator`; AC-2 by `invalid_native_spec_prevents_actual_cargo_calls_even_in_parallel`; AC-3 by the five NFR-002 gate controls including all three new launcher tests; AC-4 by `native_fixtures_use_workspace_default_relative_and_absolute_targets`.
- Reverse-gap inventory for the changed surface: 3 changed files reviewed; helper behavior is covered by the NFR-002 gate criteria and TC-920. No source stubs or test stubs found. Plan completion: not assessed.
- `quire coverage` did not finish within the reviewer run window and was interrupted; untracked-symbol census is not claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The repository-wide computed matrix has 1,120 untagged criteria and 318 criteria backed only by ignored tests. These are existing repository-wide gaps, not introduced by PR #2737; they cause the planless gap-analysis verdict to fail. The four NFR-002 acceptance criteria in this PR are tagged. | NFR-002-M-1, NFR-002-M-2, NFR-002-M-3 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | The repository-wide coverage backlog is tracked by QSL-2083 through QSL-2098 and is outside QSL-2278. The four NFR-002 acceptance criteria relevant to this PR are tagged. |
