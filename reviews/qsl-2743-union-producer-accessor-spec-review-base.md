---
id: SR-967175026302197768900474181941935084682
title: "QSL-2743 PR #3398 spec-review base: FR-531 AC-45..48"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@5f6b1a76e5358f75d38c5d401dd0479d2d82183f; PR #3398 spec delta; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md AC-45..AC-48 and their tagged tests"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-531", type: reviews }
---

## Summary

Ticket: QSL-2743. This is a base checklist review of the four FR-531 acceptance criteria added in fix round 1, checked against the code and the tests tagged to them. Review set: base plus integrity (SR-967175026302197768900474181941935084683). EARS conformance does not apply: no requirement statement was added or edited, only AC rows. The other analyses do not apply to a four-row AC addition.

## Verdict

**CONDITIONAL**: one medium and one low finding.
- IDs are sequential after AC-44 with no duplicates, and FR-531 validates (`quire validate`, rc=0).
- AC-46 and AC-48 match their tests exactly.
- AC-47 matches the classifier and its unit test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-531-AC-45 is compound, and two of its clauses have no test. The writer-fault clause covers a composite, sealed member or payload node that cannot be joined, which the emitter reports as `SourceMapInvariant`. The no-charge clause says rows are borrowed without a native budget charge. The only AC-45 test (`emission_model_union_rows_are_joined_by_the_native_monitor_closure`) asserts the row's document, its case rank/position/name/payload kind, closure admission and native projection. No test exercises the unjoinable-row refusal or asserts budget consumption around the getter. | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:1097; fsl-analyze/tests/it/monitor_root.rs:1331-1335; fsl-package/src/emit.rs:7900-7973 |
| FND-002 | low | AC-45 to AC-48 declare Verification `Test` with no TC. The neighbouring FR-531 ACs (AC-26 to AC-44) read `Test (TC-1331)`. The retagged union tests dropped their `TC-1331` tag, and TC-1331 does not describe the union cases, so the new ACs have no test-case procedure. | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:1097-1100; spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:141 |

## Coverage

Examined:
- AC-45 against `Emission::selected_model_union_rows` and `model_union_rows`. The clauses are true of the code.
- AC-46 against both closure tests. It is exact.
- AC-47 against `union_payload_form` and its test. Classification is true. The outcome wording is covered by the integrity finding.
- AC-48 against intake `position` and the reorder oracle. It is exact.

Trace bindings checked:
- AC-45: monitor_root.rs:1331.
- AC-46: monitor_root.rs:1331 and 1339.
- AC-47: lowering/model/tests.rs:2169.
- AC-48: variant_type.rs:316.

Computed matrix: not run. A prior attempt did not complete within 300 s, so the matrix axis is unassessed. Receipts read, not rerun: qsl2743fix1-c (2 passed), d (1), e (1), f (30), g (15).

## Dispositions

Disposition round 2. Head 7cfd1558f632b5f26841cdcc218f21b07638e1be. Same reviewer, run 59b28e57-7cbb-423a-b6ed-b607ed3538ff. This round changed spec text, TC text and four trace tags; no production code changed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-45 (`FR-531...md:1127`) no longer has the untested writer-fault and no-budget-charge clauses. It now states only the row and case contents. Those contents are asserted by `emission_model_union_rows_are_joined_by_the_native_monitor_closure`: the document, rank, position, name and payload kind directly, and the type handle through the projected `Composite(type_handle)`. The original case identity, settled key, sealed member and Data original type are checked by the native closure join against the package's selected model. The typed lowering faults moved to the atomic AC-49, verified by Inspection. |
| FND-002 | fixed | AC-45 to AC-48 now read `Test (TC-1331)`. TC-1331's scope names AC-45 to AC-48, and the procedure gains rows 25 to 27 plus a "Model union producer" paragraph. Row 26 matches the classifier test's eight inputs and outcomes, and is qualified as making no end-to-end admission claim. The four tests carry `TC-1331` with their union ACs. The AC-30 enum tags are unchanged. AC-49 reads `Inspection` with no TC while AC-11 and AC-25 read `Inspection (TC-1331)`. The TC-1331 paragraph names AC-49's inspection target, so I record this as a style difference, not a defect. |
