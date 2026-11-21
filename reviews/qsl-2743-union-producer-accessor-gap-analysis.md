---
id: SR-967175026302197768900474181941935084681
title: "QSL-2743 PR #3398 union producer accessor gap-analysis (scoped, partial matrix axis)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@d3be81c5f503996212c1d5594f5b9a5d22746c4d; PR #3398 diff (14 files); owning FR-531/TC-1331 lookup; spec/functional/FR-531, FR-093, FR-360"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-531", type: references }
---

## Summary

Ticket: QSL-2743. This is a gap analysis scoped to the PR #3398 production delta. It covers reverse ownership of new code, stub and coverage-inflation checks, and a direct lookup of the owning AC, TC and test tags. Plan completion: not assessed. The computed static matrix (`quire matrix --scope . --format json`, quire 0.36.2) did not complete within 300 s and produced no output. A retry was cancelled. No emitted matrix could be audited, so the full computed matrix axis is unassessed and no repository-wide assurance is claimed.

## Verdict

**CONDITIONAL** (scoped). There are no stubs, ignored tests or coverage inflation. The urgent proof is a genuine admitted-model integration test with a discriminating adverse control, and both pass. One medium reverse-gap/trace finding stands. This verdict does not cover the matrix axis, which remains unassessed. It is not a full gap-audit PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No FSL acceptance criterion states the new public model-union producer contract: `Emission::selected_model_union_rows`, `SemanticGraph::selected_model_unions` and `VariantRecord.position`, which carry rank, authored position, sealed members and the Data/Clock/None payload. The two new union tests are traced to `FR-531-AC-30`. That AC covers source-enum materialization and the conversion of model-enum shapes and selected collections; it does not cover model unions. The binding claims coverage the criterion does not state. The contract is grounded only in the upstream IR FR-830 and in ticket prose. | fsl-analyze/tests/it/monitor_root.rs:1331-1342; fsl-package/src/emit.rs:1504; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:1082 |

## Coverage

Reconciliation: quire matrix (quire 0.36.2). It did not complete within 300 s and gave no JSON; the retry was cancelled. Matrix axis: unassessed. No evidence bindings were read: the raw runner receipts are not Quoin bindings, and `quoin matrix` was not run. Plan completion: not assessed.

Direct lookup instead of the matrix:
- `FR-531-AC-30` has binders across fsl-package, `native_row_budget.rs` and `monitor_root.rs`, and its Verification cell reads Test.
- `TC-1331` exists at `spec/test-cases/TC-1331-*.md`.
- No FSL spec file mentions model unions or `SelectedModelUnion`. FR-360 calls payload-bearing variant types unions, but states no producer-row contract.

Reverse gap:
- New public items with no owning AC: FND-001.
- Fixture `position` literals and the `variant_type` oracle change are test support for the new field and stay under FR-056/FR-360 variant-type tests.

Stubs and inflation:
- No `todo!`, `unimplemented!` or placeholder rows.
- No empty stand-in: the getter returns lowered rows.
- The adverse test asserts the emission is non-empty before withholding rows.
- The positive test destructures exactly one row and asserts exact case tuples.

Focused evidence read: a1 enum closure (1 passed), a2 union closure plus adverse (2 passed), a3 semantics variant_type (30 passed), a6 package variant_types (15 passed). Semantic review (intent vs test vs code) is covered by the companion code-review SR-967175026302197768900474181941935084680. Repository baseline assurance failures are out of scope and unqualified here.

## Dispositions

Disposition round 1. Head 5f6b1a76e5358f75d38c5d401dd0479d2d82183f. Same reviewer, run 59b28e57-7cbb-423a-b6ed-b607ed3538ff. The matrix axis is still unassessed: no matrix was re-run, and no repository-wide assurance is claimed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-531 gains AC-45 (accessor and complete rows), AC-46 (native closure join and missing-row refusal), AC-47 (Clock classification) and AC-48 (`VariantRecord.position`). The union closure tests are retagged: `monitor_root.rs:1331` carries AC-45 and AC-46, and `monitor_root.rs:1339` carries AC-46. The classifier test (`lowering/model/tests.rs:2169`) carries AC-47, and the reorder oracle (`variant_type.rs:316`) adds AC-48. Neither union test is tagged FR-531-AC-30 any more. Remaining quality defects in the new ACs (no owning statement, an untested clause, outcome wording, TC binding) are recorded in the new spec-review SRs SR-967175026302197768900474181941935084682 and SR-967175026302197768900474181941935084683. |
