---
id: SR-766
title: "Gap analysis — QSL-2811 root evaluator dependency edge"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository matrix, Cargo.toml, src/population_fixtures.rs, examples/population_v2_fixtures.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Planless gap analysis for QSL-2811 / PR #3493. The changed behavior is owned by FR-100-AC-7 and uses the existing port correctly, but the repository-wide traceability matrix contains untagged and ignored-only criteria; a complete repository assurance PASS is not supported.

## Verdict

**FAIL** — the matrix has 1,080 untagged criteria and 68 criteria whose only evidence is ignored. These are repository-wide results and are not introduced by this PR; FR-100-AC-7 itself is tagged by six non-ignored tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-001-AC-7 is reported `untagged` with no binder; it is one of 1,080 such matrix rows. | FR-001-AC-7; quire matrix |
| FND-002 | high | FR-056-AC-60 is backed only by an ignored test; it is one of 68 ignored-only rows. | FR-056-AC-60; fsl-eval/tests/it/model_simulation.rs::a_float_field_model_explores_by_its_ieee_bits |

## Coverage

- Reconciliation: `quire matrix --scope <repo> --format json` (quire CLI); `quoin matrix --repo <repo> --json` exceeded 15 seconds without output, so run evidence was not read.
- Plan completion: not assessed
- Criteria: 5,024 tagged · 1,080 untagged · 68 tagged-by-ignored-test · 108 method-without-symbol · 15 retired (6,295 total).
- FR-100-AC-7 is tagged by six non-ignored tests, including `fsl-replay/src/spine/call/tests.rs::tc_452_seven_completes_and_agrees_with_compile`.
- The root evaluator edge is tied to FR-100-AC-7; package checking calls the supplied `RewardEvaluator`, and the example/tests pass the real S6a evaluator. No scoped reverse gap or stub was found in the changed source.
- Repository-wide reverse-gap and stub inspection was not completed; the coverage extractor reports 6,213 untracked symbols and 23,507 diagnostics, including 22,866 `trace-id-outside-marker` diagnostics. These signals limit broad conclusions and need separate triage.
- Semantic review: skipped (not opted in).
