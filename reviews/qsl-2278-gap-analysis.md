---
id: SR-2136
title: "Gap analysis of QSL-2278 PR #2737"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2737 diff origin/main...0640c7ffd7e8d0773cf26326a4ca0d67eebf29b6; Ticket QSL-2278; full repository matrix; focused NFR-002-AC-1..4 and TC-920"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-002
    type: reviews
---

## Summary

Ticket QSL-2278. Planless gap analysis of the full repository matrix and the behavior changed in PR #2737. Plan completion: not assessed. The four NFR-002 acceptance criteria are tagged; the new native-first, path-order-independent launcher selection and native-only diagnostic tests bind NFR-002-AC-3 and TC-920. The focused source behavior maps to NFR-002's scoped structural-validation gate and TC-920's installed module discovery and refusal controls. No changed-source stub or untraced behavior was found. The full repository matrix reports substantial pre-existing coverage gaps outside this PR: 5,172 criteria, 3,623 tagged, 1,120 untagged, 318 tagged only by ignored tests, 98 method-without-symbol, and 13 retired. The matrix reports no run evidence across the repository. This repository-wide gap set is not attributed to this PR.

## Verdict

**FAIL** — the full repository matrix contains untagged and ignored-only criteria, which meet gap-analysis's fail rule. The changed NFR-002-AC-1..4 criteria are all tagged, and this PR adds no gap within its changed behavior. Plan completion: not assessed.

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
