---
id: SR-967175026302197768900474181941935084798
title: "Gap analysis of QSL-2951 qualitative-threshold test rewrite"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3570 head 28472bfdbfd5086742d13900bef6f8a21bbd82b8; FR-201-AC-39, FR-201-AC-41, FR-201 cyclic-MDP threshold clause, FR-186-AC-2 against tools/fsl-probability-check/tests/it/cyclic_mdp.rs"
review_set: subset
---

## Summary

Ticket: QSL-2951. A planless gap analysis scoped to the criteria the changed test claims or touches. The full `quire matrix` was not run: it took 15 minutes in an earlier review of this repo, and the host is overloaded. A grep index of `#[trace(...)]` tags over the PR head stands in for the matrix.

The changed test `strict_rows_boundaries_and_quantitative_source_scope_are_closed` keeps its tag `#[trace("TC-1404", "FR-201-AC-39", "FR-201-AC-41")]`. The tag is honest: its malformed-row, foreign-binding and boundary assertions are FR-201-AC-41 content on an AC-39 source. The new S3 assertions belong to FR-186-AC-2 and to FR-201's clause "Existing S3 endpoint admission is unchanged: source thresholds 0 and 1 are refused". That clause has no AC of its own. FR-186-AC-2 ("`probability >= 1 [E]` and `probability >= 0 [E]` are refused `qualitative-threshold` naming the qualitative form") is already tagged in five tests: `fsl-semantics/tests/it/probabilistic_declarations.rs` (3), `fsl-replay/src/spine.rs` (2), plus `tests/it/probability_source_routes.rs`. The test does not claim it, which is acceptable.

Coverage comparison with the pre-a3588a41ad test: the checker-level assertion (`run(&outside)` exits non-zero at thresholds 0 and 1) is kept, with the request built exactly as before. No criterion lost a test. Reverse gap: the diff adds no production code (the xtask change is gate tooling under the existing merge-gate rules). There are no stubs and no coverage inflation.

## Verdict

**PASS**: no untagged criterion, no coverage loss, tags honest.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

- Matrix source: grep index of trace tags (declared fallback; `quire matrix` not run).
- In-scope criteria with a tagged test: 3 of 3 (FR-201-AC-39, FR-201-AC-41, FR-186-AC-2).
- Reverse gaps: none. Stubs: none.
- Plan completion: not assessed

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | Placeholder row for a clean review; there is no defect to disposition. Round 1 changed only xtask/docs/reviews; the test and its trace tags are unchanged (a rebase onto 2665967f only). |
