---
id: SR-2026111612172400000053
title: "Gap analysis — QSL-2241 text number conversion"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/test-cases/TC-1196-a-text-to-number-convert-lowers-to-its-text-identity.md, fsl-semantics/tests/it/text_to_number.rs, fsl-eval/src/value/expression/evaluate/text_conversion.rs, fsl-eval/tests/it/text_to_number.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: references
---

## Summary

Audited the QSL-2241 conversion slice against local FR-093 criteria, external FSpec FR-478 / TC-964, trace tags, and the computed matrix at PR head `c384a09679ef9f6af86e94fd7c59b1df136ad5e6`. The local checker criteria are tagged and the checker test passes; the evaluator test suite leaves material acceptance evidence gaps and uses unqualified external FSpec identifiers that the repository trace convention does not resolve.

## Verdict

**CONDITIONAL** — implementation is present, but external trace bindings and several explicit evaluator acceptance cases/accounting checks are not evidenced in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The evaluator test tags use bare external `TC-964` and `FR-478-AC-*` identifiers, so quire coverage reports them as untracked; repository convention requires the `FSpec-` prefix for external FSpec IDs. | fsl-eval/tests/it/text_to_number.rs:129 |
| FND-002 | medium | The tests do not assert the required accounting sequence or denied-charge outcomes for integer parsing, decimal rounding, IEEE rounding, and String normalization. | fsl-eval/tests/it/text_to_number.rs:129 |
| FND-003 | medium | IEEE acceptance coverage omits required alternate-width and alternate-rounding bit/flag cases and does not assert overflow/underflow flags or strict-exact overflow refusal with would-be flags. | fsl-eval/tests/it/text_to_number.rs:372 |
| FND-004 | medium | Quantity tests omit the specified domain-package unit case, including accepting its declared short name and rejecting the qualified spelling. | fsl-eval/tests/it/text_to_number.rs:316 |
| FND-005 | medium | Rational and integer/decimal coverage omits explicit acceptance vectors for negative bounded integers, unbounded Decimal retention, and unbounded Rational reduction. | fsl-eval/tests/it/text_to_number.rs:129 |

## Coverage

- `quoin matrix --repo <worktree> --json`: local FR-093-AC-35 and FR-093-AC-47 are tagged by the intended semantic integration tests. The matrix is repository-wide and reports existing global no-run-evidence / untagged counts; those are not attributed to this PR.
- `quire coverage`: PR evaluator traces `TC-964` and bare `FR-478-AC-*` are untracked in this repository. The semantic test traces `TC-1196` and FR-093 ACs are local and resolve.
- Focused test: `cargo test --locked -p fsl-semantics --test it text_to_number` passed (4 passed, 676 filtered). The evaluator suite could not be built under the known coordinated direct/transitive filament-exact pin mismatch; this known external pin blocker is not treated as an implementation defect.
- Reverse-gap inspection was limited to this conversion slice: new `ConvertText` checker/runtime branches are anchored to FR-093 and FSpec FR-478; no additional unowned behavior was identified.
- Semantic intent review: skipped (not requested as an opt-in).
- Plan completion: not assessed.
