---
id: SR-967175026302197768900474181941935084865
title: "Gap analysis — filament-spec-language repository matrix and PR scope"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository static matrix; fsl-replay/src/hybrid_replay/class.rs, tests/it/hybrid_class_recursion.rs, tests/it/main.rs; NFR-141-AC-27"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Audited the repository's static requirement-to-test matrix and the changed PR scope at head `519c072bdba7b21ee0e31bc64251aa8140f5ec1d`. The changed `NFR-141-AC-27` criterion is tagged by the existing class-walk tests and the new isolated recursion test; the repository-wide matrix nevertheless reports substantial untagged and ignored-only criteria.

## Verdict

**FAIL** — the static repository matrix reports untagged criteria and criteria backed only by ignored tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `NFR-141-AC-1` has no test bound to its criterion in the repository matrix. | NFR-141-AC-1 |

## Coverage

- Reconciliation: `quire matrix --scope /home/peter/dev/worktrees/qsl-3390 --format json` (quire 0.52.0) — static matrix only, no run evidence read. `quoin matrix --repo /home/peter/dev/worktrees/qsl-3390 --json` was attempted first and exited 1 with `CORE_REFUSED: request exceeds the accepted size`; the documented static fallback completed with exit 0.
- Criteria statuses from the static matrix: tagged 5,643; untagged 985; tagged-by-ignored-test 24; method-without-symbol 109; retired 15 (6,776 matrix rows). Among criterion-id rows ending `-AC-N`, the corresponding counts are tagged 5,599; untagged 929; tagged-by-ignored-test 22; method-without-symbol 77; retired 15 (6,642 rows).
- `NFR-141-AC-27` is tagged by `fsl-replay/src/hybrid_replay/class.rs` tests `bounded_branch_conditional_and_call_walks_match_expected_outputs` and `hybrid_class_walks_scale_and_support_one_hundred_thousand_depth`, and by `tests/it/hybrid_class_recursion.rs::recursive_depth_failure_is_isolated_in_a_child_process`.
- Changed-source reverse check: no new untraced production behavior or source/test stubs found. The full repository reverse-gap inventory was not completed; this artifact does not claim that portion of a repository-wide gap audit.
- Plan completion: not assessed
- Semantic review: skipped (not opted into)

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | `NFR-141-AC-1` is a repository-wide baseline gap for the QSL-1919 package/emit benchmark. PR #3958 addresses hybrid class-walk instrumentation and recursion-test isolation only; this unrelated criterion requires separate follow-up work. |
