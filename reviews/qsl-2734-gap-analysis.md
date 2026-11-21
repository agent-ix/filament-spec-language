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

## Dispositions

Round 1, fix head 4219c63abebbce1dd776d930e126e778c4bd5d96 (original head 1d666e7daa977b75f02e1bdefbd89b361d0449fd). The reason column records the evidence; after-excerpts are in the round-1 dispositions comment.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | quoin matrix (0.36.8) at the fix head extracts 23 FR-533 criteria. FR-533-AC-23 is tagged, with binders a_package_class_conversion_is_typed_unsupported_until_reconstruction and bare_class_products_and_noncanonical_powers_refuse_before_kernel_charges. |
| FND-002 | fixed | New test bare_class_products_and_noncanonical_powers_refuse_before_kernel_charges refuses the bare product and the noncanonical power. It picks the noncanonical member from the projected root relation, checks span and no unit charges under five meters, and keeps an ordinary Length power true and caller-metered (an integer_bits=0 stop at UnitIdentityRead). Runtime outcome is not claimed: draco's fix-head runs are queued. |
| FND-003 | fixed | The unrelated-dimension negative is now tagged FSpec-FR-142-AC-2 / FSpec-TC-187, matching quantities.rs's existing incompatible-dimension tests. It is no longer tagged FR-093-AC-46. |
| FND-004 | fixed | The single-package positive keeps FR-093-AC-46 only. The matrix no longer binds it to FR-091-AC-60, whose sole binder is dimension_equivalences_are_decided_by_fr_142_steps. |

Round 2, final head 7eb36c927d92a4b7637c05c608d09db5e819f2b8 (delta from 4219c63abebbce1dd776d930e126e778c4bd5d96). Each round-1 fix was re-verified at this head; none of the fixed files or spec rows changed in the delta. Delta gap check, no new findings. let_binder tests (FR-093-AC-68, FSpec-FR-322-AC-512, TC-3137) assert the binder link, the dependency, the reference identity, unread-binder identity and per-IEEE-operand binders on strictly read packages. FSpec-FR-142-AC-2 / TC-187 text is unchanged between FSpec 6ad873d5 and the pinned 247a3941; that FSpec range changes only its tests/*.rs (no lib, API or Cargo change). Draco runs for the 12 focused tests and the replay check are queued, not green; no runtime outcome is claimed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. quoin matrix (0.36.8) at the fix head extracts 23 FR-533 criteria. FR-533-AC-23 is tagged, with binders a_package_class_conversion_is_typed_unsupported_until_reconstruction and bare_class_products_and_noncanonical_powers_refuse_before_kernel_charges. |
| FND-002 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. New test bare_class_products_and_noncanonical_powers_refuse_before_kernel_charges refuses the bare product and the noncanonical power. It picks the noncanonical member from the projected root relation, checks span and no unit charges under five meters, and keeps an ordinary Length power true and caller-metered (an integer_bits=0 stop at UnitIdentityRead). Runtime outcome is not claimed: draco's fix-head runs are queued. |
| FND-003 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. The unrelated-dimension negative is now tagged FSpec-FR-142-AC-2 / FSpec-TC-187, matching quantities.rs's existing incompatible-dimension tests. It is no longer tagged FR-093-AC-46. |
| FND-004 | fixed | Round 2: still fixed at 7eb36c927d, unchanged by the delta. The single-package positive keeps FR-093-AC-46 only. The matrix no longer binds it to FR-091-AC-60, whose sole binder is dimension_equivalences_are_decided_by_fr_142_steps. |
