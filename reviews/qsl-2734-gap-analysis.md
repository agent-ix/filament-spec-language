---
id: SR-202611210022442313144108507
title: "QSL-2734 PR3286 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR3286 1d666e7daa977b75f02e1bdefbd89b361d0449fd vs merge-base 8afc37d8585d4acb6ebd42e3e129dad6203a432a; FR-093-AC-46, FR-533-AC-23, FR-497-AC-2; fsl-eval/src/value/expression/evaluate.rs, fsl-semantics/src/check/assemble.rs, fsl-semantics/src/check/mod.rs, fsl-semantics/src/check/refusal.rs, fsl-eval/tests/it/quantity_class_convert.rs, fsl-eval/tests/it/quantity_literals.rs, fsl-semantics/tests/it/dimension_equivalence.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-093
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-533
    type: references
---

## Summary

Ticket: QSL-2734. Planless gap-analysis, so plan completion was not assessed. The matrix came from `quoin matrix --json` (quoin 0.36.8) at the frozen head. Run evidence is not bound in the matrix, which reports "no run evidence"; the coder reported targeted runs, which this review did not reproduce. No applicable AssuranceProfile was found. Package/BareWorld class reconstruction is out of scope by explicit arch ruling (FR-093, FR-533 boundary), so its absence is not counted as a gap.

## Verdict

CONDITIONAL. FR-093-AC-46 is tagged by six binders that cover u1/u2/u3 exact values under both root ratios, lowered conversion nodes, the u4 dimensionless quotient, u5 refusals, five meter bounds with the denied edge's span, and affine. FR-497-AC-2 is tagged, and its rewritten oracle is exact. No stubs or coverage inflation were found in the new code. Gaps: FR-533-AC-23 is absent from the matrix, the product and integer-power bare refusals have no test, and two trace bindings are imprecise.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-533-AC-23 is not extracted into the computed matrix (FR-533 criteria end at AC-22), so the bare-route test tagged to it backs no criterion and the bounded-refusal obligation is untracked. | spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md:497-498; fsl-eval/tests/it/quantity_class_convert.rs:341-344 |
| FND-002 | medium | FR-533's boundary requires bare refusal for product, quotient and integer power, but only a quotient (`6 s / 3 beat`) is tested; the Multiply guard and the QuantityPower guard (which has the ordering defect in the code-review artifact's FND-001) have no test. | fsl-eval/tests/it/quantity_class_convert.rs:363-367; fsl-eval/src/value/expression/evaluate.rs:3315-3349; fsl-eval/src/value/expression/evaluate.rs:3426-3446 |
| FND-003 | low | `convert_between_unrelated_dimensions_refuses_type_mismatch` is tagged FR-093-AC-46, but AC-46 has no unrelated-dimension clause; the refusal it asserts is FR-093 dimension-equivalence prose. | fsl-semantics/tests/it/dimension_equivalence.rs:1347 |
| FND-004 | low | `convert_across_a_dimension_equivalence_class_checks` is tagged FR-091-AC-60, a cross-package dimension-equivalence decision criterion, though it checks single-package conversion admission (FR-093-AC-46). | fsl-semantics/tests/it/dimension_equivalence.rs:1325 |

FND-001 has the same root cause as the spec-integrity artifact's FND-001 (a blank line splits the AC table). It is recorded here as the coverage escape.

Reverse gaps: `AssemblyCause::DimensionClassProjection` and `KeyFault::DimensionClassProjection` are invariant paths with no test. Assembly admits classes before projecting them, so neither path is reachable from source text. This is not counted as a finding.

## Scope

- FR-093-AC-46 (examined): unequal roots, u1–u5 checks, lowering and values under ratios 1/4 and 1.
- FR-533-AC-23 (examined): bare unsupported refusal for cross-class convert, affine, quotient; ordinary/affine same-class positives.
- FR-497-AC-2 (examined): carried quantity not an arithmetic operand.
- FR-093 dimension-equivalence prose, lines 359-371 (context_only).
- FR-533 boundary prose, lines 296-313 (context_only).
