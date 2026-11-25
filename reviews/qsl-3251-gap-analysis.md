---
id: SR-967175026302197768900474181941935084845
title: "Gap analysis — filament-spec-language repository traceability"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; full spec and source/test tree"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Ran a planless, repository-wide static test-matrix audit for filament-spec-language. The evidence-backed `quoin matrix` refused the repository request size; the documented static fallback `quire matrix` succeeded and reported broad existing criterion gaps, while the four FR-282 criteria changed by PR #3861 are all tagged.

## Verdict

**FAIL** — the full static matrix reports 1,075 untagged criteria and 24 criteria tagged only by ignored tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The repository matrix reports 1,075 untagged criteria. This includes 877 whose statements are not visibly marked retired and 198 whose statement text is marked `RETIRED`; the representative active row FR-002-AC-9 has no bound test. | FR-002-AC-9; FR-001-AC-1; quire matrix |
| FND-002 | high | The repository matrix reports 24 criteria whose only binders are ignored tests; representative FR-056-AC-60 is bound only by `fsl-eval/tests/it/model_simulation.rs`. | FR-056-AC-60; quire matrix |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` (quoin 0.36.10, engine 0.62.2) refused with `CORE_REFUSED: request exceeds the accepted size`; static fallback `quire matrix --scope . --format json` (quire 0.52.1) succeeded. No run evidence was read.
- Plan completion: not assessed
- Criteria: tagged 5,496 · untagged 1,075 · tagged-by-ignored-test 24 · method-without-symbol 109 · qualified-record 0 · retired 15 (counts grouped from the computed matrix JSON; 6,719 criterion rows were emitted)
- Evidence: no run evidence
- `quire coverage` completed. It reported 6,519 untracked symbols and 24,624 diagnostics; the untracked set includes FSpec-prefixed cross-repository references and parent/TestCase trace ids. The new FR-282 tags are recognized as binders; no newly added tag in this PR is unmatched.
- Reverse-gap/stub check: all four changed behavior paths map to FR-282-AC-6/7/8/21; no new stub or untraced changed behavior was found. A full manual inventory of the repository's complete source surface was not completed in this review.
- Semantic review: skipped; no explicit opt-in was given.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Round 1 (Linear comment afc32fae): refreshed matrix 1,073 untagged; no broad follow-up linked. |
| FND-002 | still-open | Round 1 (Linear comment afc32fae): refreshed matrix 24 ignored-only; no broad follow-up linked. |
| FND-001 | deferred | Round 2 at 83d70d88e5e1da26b88a170866a09f3fb6497631: QSL-3110 (linked related) owns repository-wide untagged triage. Pre-existing baseline: origin/main 1df75ab47fc1fc64d6e87df8172903e449a78766 has 1,077 untagged, head 1,073; no criterion is untagged at head that is tagged on main; the PR only adds trace tags. FR-282-AC-6/7/8/21 are tagged by non-ignored tests at head. |
| FND-002 | deferred | Round 2 at 83d70d88e5e1da26b88a170866a09f3fb6497631: QSL-3110 scope extended by comment 97237f01 to the 24 ignored-only criteria. Identical 24-id set on origin/main and head; the PR adds no `#[ignore]`. |
