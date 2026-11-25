---
id: SR-2026112506280002
title: "Gap analysis — filament-spec-language QSL-3311 intake mapping"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec/, fsl-semantics/src/model/intake.rs, fsl-semantics/tests/it/model_intake.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Ran the repository-level matrix and coverage extraction for the QSL-3311 review. The change's three tests bind to `FR-056-AC-5`, and that criterion is tagged; the whole repository matrix still contains substantial pre-existing traceability and extraction gaps, so repository assurance does not pass.

## Verdict

**FAIL** — the computed repository matrix has 1,025 untagged criteria and 26 criteria tagged only by ignored tests; coverage also reports 6,579 unmatched test tags. These are repository-wide results and are not attributed to this diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed matrix contains 1,025 untagged criteria and 26 criteria whose only tagged tests are ignored, leaving live requirements without executable test evidence. | quire matrix: 1,025 untagged; 26 tagged-by-ignored-test |
| FND-002 | medium | Coverage extraction reports 6,579 untracked test symbols/tags, so those tags do not bind to declared criteria. | quire coverage: untracked_symbols=6579 |
| FND-003 | high | The whole-source reverse-gap inventory and source/test stub scan were not completed; only the changed intake path and its tests were inspected, so this is not a complete repository gap audit. | fsl-semantics/src/model/intake.rs; fsl-semantics/tests/it/model_intake.rs |

## Coverage

- Reconciliation: `quoin matrix --repo` refused with `CORE_REFUSED: request exceeds the accepted size`; used `quire matrix --scope ... --format json` (quire 0.52.0, engine 0.62.2), with no run evidence.
- Criteria: tagged 5,587; untagged 1,025; tagged-by-ignored-test 26; method-without-symbol 109; retired 15.
- `FR-056-AC-5` is tagged by 15 tests, including the three changed tests at `fsl-semantics/tests/it/model_intake.rs:205,235,253`; it has unbacked-claim diagnostics from existing tests but the changed tests themselves are bound. `FR-056-AC-42` is tagged by one test.
- Coverage extraction: untracked symbols 6,579; 24,747 diagnostics; 142 suspicions. These are repository-wide, not caused by this PR.
- Reverse-gap review was limited to the changed intake mapping and test helpers: one new mapping behavior has an owning requirement, and no new stubs or hollow tests were found there. A full manual inventory of all repository behaviors was not completed.
- Semantic review: skipped (not requested). Plan completion: not assessed.
- No applicable AssuranceProfile applies to the changed behavior.
