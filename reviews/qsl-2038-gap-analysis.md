---
id: SR-2026111518064800000005
title: "Gap analysis — QSL-2038 source corpus runner"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2038; PR #2722; full spec matrix at candidate e483e670612fcb0e87ae8c81e56293bc3d42ee75; changed source and test files"
review_set: subset
---

## Summary

Ran the repository's evidence-backed matrix and manually mapped QSL-2038's acceptance criteria to the changed code and tests. The focused runner cases are present, but the required negative check that proves CI fails when corpus tests are deselected is missing; the full matrix also exposes preexisting repository-wide untagged and ignored-only criteria.

## Verdict

**FAIL** — the full matrix has untagged and ignored-only criteria, and QSL-2038's CI-selection acceptance check is not covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The full matrix reports 1,129 criteria without a trace-tagged test across the 452-requirement repository; this is a repository-wide preexisting gap, not introduced by PR #2722. | quoin matrix at candidate e483e670612fcb0e87ae8c81e56293bc3d42ee75 |
| FND-002 | high | FR-518-AC-11 is bound only to an ignored test, so no test runs for this criterion. | FR-518-AC-11; tests/it/checked_handoff.rs::a_differing_declared_clause_name_is_refused_on_clause_name |
| FND-003 | medium | QSL-2038's negative CI-selection criterion is uncovered: deleting the \`source_corpus\` module registration leaves the root integration target nonempty, while the xtask tests exercise only the empty-area helper. | tests/it/main.rs:35; xtask/src/test_suites.rs:1712-1742 |

## Coverage

- Reconciliation: \`quoin matrix --repo /home/peter/dev/worktrees/qsl-2038 --json\` (quoin 0.35.0; quire 0.45.0, engine 0.60.0).
- Matrix rollup at the candidate: 4,812 criteria across 452 requirements; tagged 3,573; untagged 1,129; tagged-by-ignored-test 1; method-without-symbol 96; retired 13. Evidence status: no run evidence for all 4,812 criteria.
- Manually mapped QSL-2038's matching-expectation, exit mismatch, code/cause mismatch, orphan, flip-report, public-entry-point and empty-area criteria to named tests. The negative CI-selection check has no test.
- Reverse-gap inventory: the changed runner and xtask guard are internal test infrastructure owned by the specced QSL-2038 ticket; no new user-facing API, stub or TODO was found.
- \`spec/spec.md\` has no artifact id, so no spec-root relationship was asserted. No applicable AssuranceProfile was found.
- Plan completion: not assessed
- Semantic review: skipped
