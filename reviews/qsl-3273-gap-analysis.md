---
id: SR-96050
title: "gap analysis of QSL-3273/QSL-3295 field-member node identity (filament-spec-language PR 3923)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@cf69938b7357ec6a3cac2669223b89829c0fa798; criteria FR-135-AC-4, FR-137-AC-1, FR-147-AC-9, FR-147-AC-16, FR-306-AC-2, FR-092-AC-2, FR-092-AC-11; code fsl-semantics/src/check/lowering.rs walked_operands, fsl-package/src/emit/monitor_plan.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-135
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-147
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-092
    type: references
---

# Gap analysis of QSL-3273/QSL-3295 field-member node identity (PR 3923)

## Summary

Ticket: QSL-3273 (with QSL-3295). Planless run: no plan was supplied. The run covers the computed matrix (`quire matrix --scope . --format json` at head), a reverse-gap check over the changed production code, and a stub check. The semantic review was not run.

Matrix state of the criteria the PR touches:
- FR-135-AC-4 is tagged by `a_field_row_changes_the_refinement_node_body_and_not_its_key`.
- FR-137-AC-1 is tagged by `the_fairness_rows_form_the_two_sets_in_source_order`.
- FR-147-AC-9 is tagged by `the_internal_list_changes_the_node_body_and_not_its_key`.
- FR-306-AC-2 is tagged by `a_rebinding_changes_the_relation_body`.
- FR-092-AC-2 and FR-092-AC-11 are tagged.
- FR-147-AC-16 is tagged only by fsl-replay `a_refused_dependency_protocol_precedes_the_importing_units_s3_check`.

The new `FSpec-` tags (FR-375-AC-22/23/26/27/28, TC-310) raise no grammar warning. Each new or changed test asserts literal values; none is a stub or a tautology.

Reverse gap: the production change in `walked_operands` and in the monitor_plan re-key belongs to FSL FR-092, which delegates derived paths to FSpec FR-322, and to FSpec FR-487 "call closure" renames. No unowned behaviour was added.

The one gap is a direct consequence of the same delegation. FR-092 requires FR-322 derived paths, and FR-322's member operands are applied to the `field` member only. The other member kinds are recorded in SR-20101 FND-001 to FND-004, not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-147-AC-16's local OnImpl clause ("with `Spec` local, a protocol `OnImpl` ... refuses `abstract Spec::OnImpl` the same way, at `OnImpl`") lost its only binding: this PR removed the tag from the test that asserts it. The dependency `OnOther` clause had no binding before or after. The matrix still marks AC-16 tagged through the replay test, so the loss is invisible in the computed status. | fsl-semantics/tests/it/refinement_protocol.rs:434-439 |

## Verdict

CONDITIONAL. Every criterion this PR touches is tagged by a test that asserts literal values. One low trace regression remains (FND-001, the same defect as SR-20101 FND-005). No untagged criterion was introduced, and no stub or coverage inflation was found in the changed tests.

## Coverage

- Criteria examined: FR-135-AC-4, FR-137-AC-1, FR-147-AC-9, FR-147-AC-16, FR-306-AC-2, FR-092-AC-2, FR-092-AC-11.
- Reverse gap: `walked_operands` member arms; `rename_projections` and the `emit_monitor_plan` re-key.
- Stub and tautology check: the 9 changed test files.
- Semantic review: skipped.
- Plan completion: not assessed

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
