---
id: SR-2026112013172122111220106
title: "gap-analysis of QSL-2731 located refusal of a probabilistic claim over a source-object package"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2731; PR3203 head 4e94b698c1c310b3572a1a467171e9681571616f; FR-740-AC-6, FR-940-AC-6, FR-185-AC-12, FR-763; TC-3012, TC-2958, TC-2062; fsl-analyze/src/analyze/document/package_run.rs; fsl-command/src/command{.rs,/analyze.rs}"
review_set: subset
---

## Summary

Ticket: QSL-2731, PR filament-spec-language#3203. This is a diff-scoped check that the re-scoped ticket's behaviour has code and tests behind it. A package request that names a probabilistic claim of a source-object package must be refused at the claim with a non-null occurrence locus, using the FR-763 `unsupported_construct`/`not-yet-implemented` cause. Other items and packages without source objects must be unchanged. Plan completion: not assessed. Quoin 0.29.1; checked by hand against the diff and the spec text of FR-740-AC-6, FR-940-AC-6 and FR-185-AC-12.

## Verdict

COMPLETE for the re-scoped claim. Every branch has a test that passes:

- the model rebuilds: `a_probabilistic_claim_over_a_source_object_package_is_refused_at_the_claim` and `a_package_with_two_source_object_types_resolves_its_claim`;
- the model does not rebuild: `a_probabilistic_claim_whose_model_does_not_rebuild_is_refused_at_the_claim`;
- no probabilistic claim is named: `an_item_that_names_no_probabilistic_claim_keeps_its_source_object_refusal`;
- no source objects: `a_package_without_source_objects_is_not_refused_by_the_probabilistic_wiring`;
- the locus reaches the wire: fsl-command `wire_schema` and `probabilistic_package_locus`.

No engine steps a source-object model. That is the stated scope, and the refusal says so.

Base reds that this PR does not cause (verified on base 1af5220358 with identical failures):

- fsl-analyze `package_forms`: 7 tests, the strict reader refuses the emitted package at `/temporal_clause_names/1`;
- fsl-analyze `analyze_request_wire`: 3 tests;
- fsl-command `prepared_command::command_outcome_documents_include_category_and_last_stage`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
