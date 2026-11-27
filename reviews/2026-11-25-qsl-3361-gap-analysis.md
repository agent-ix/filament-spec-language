---
id: SR-967175026302197768900474181941935084873
title: "Gap analysis — filament-spec-language repository"
type: SpecReview
analysis: gap-analysis
scope: "spec/, source tree, tests/"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Planless static traceability analysis of the filament-spec-language repository. The computed matrix reports 989 untagged criteria; repository plan completion was not assessed, and the full reverse-gap/stub inventory was not completed in this run.

## Verdict

FAIL — the static matrix contains 989 live untagged criteria. The added FR-621-AC-27 package-route and FR-621-AC-28 source-route criteria are tagged and bind to their tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | 989 criteria in the repository-wide matrix are untagged and therefore lack a bound source test. | quire matrix; 989 criterion rows |
| FND-002 | medium | Coverage reports 6,613 untracked tagged symbols; these tags do not bind to criteria minted in this repository's spec corpus. The corpus includes separately prefixed FSpec identities, so this requires ownership reconciliation before treating individual tags as defects. | quire coverage; 6,613 symbols |

## Coverage

- Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2) — no run evidence read. `quoin matrix --repo . --json` refused with `CORE_REFUSED: request exceeds the accepted size` (quoin 0.36.8).
- Plan completion: not assessed
- Criteria: tagged 5,658 · untagged 989 · tagged-by-ignored-test 24 · method-without-symbol 110 · retired 15
- Evidence: no run evidence read
- Untracked symbols: 6,613; unmatched tags: 7,603; extraction diagnostics: 24,977
- Untraced behaviors / stubs: not inventoried in this run
- Semantic review: skipped
- QSL-3361 check: FR-621-AC-27 and FR-621-AC-28 are `tagged`, bound to the corresponding package-route and source-route tests in `fsl-replay/tests/it/probability_certificate.rs`.
