---
id: SR-2026112001350000000001
title: "gap analysis of QSL-2562 case binder names (FSL #3115)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language PR #3115 at 7539972662b2f075a09402ceb3703a86bbfd3eac; fsl-command/src/command/tl/canonical.rs, fsl-semantics/src/check/{check,check/typing,sum_case}.rs, fsl-package/src/reader_protocol_projection.rs, fsl-replay/src/spine/clause.rs, fsl-eval/tests/it/package_binder_names.rs, tests/it/request_surface_s1_16.rs; criteria FR-931-AC-13, FR-931-AC-14, FR-400-AC-13, FR-397-AC-29"
review_set: subset
---

## Summary

Manual acceptance-criteria-to-tests check of the QSL-2562 code PR, scoped to `git diff origin/main...HEAD`. The quoin matrix was not run; criteria were mapped to tagged tests by reading the diff. Plan completion: not assessed. Ticket: QSL-2562.

## Examined scope

- FR-931-AC-13 (source route): `case_binders_print_under_their_declared_names_without_a_model_read` runs live and asserts nested arms, `_`, member-declaration order, outer `let` and a rename. The model-field twin is ignored on QSL-2629 (see code-review FND-002). (examined)
- FR-400-AC-13 (package route): `the_reader_gives_case_arm_and_let_binders_their_declared_names` runs live over emitted bytes; `the_reader_projects_nested_let_binders_into_distinct_slots` asserts `let` names. Byte-equal both-routes label test is ignored on QSL-2627 (Backlog). (examined)
- FR-931-AC-14: four canonical.rs unit tests cover zero/one/two operand binders, a stray `#ieee_operand` let and an author's `ieee_operand_0`. The end-to-end test is ignored on QSL-2628 (Backlog). (examined)
- FR-397-AC-29: `the_checker_names_its_ieee_operand_binders_with_a_hash` is un-ignored and asserts levels [2,3], [2], []. (examined)
- Code with no owning requirement: none found; the union-payload widening that lacked an owner was reverted. (examined)
- Stubs and coverage inflation: no `todo!`/`unimplemented!` added; ignored tests each cite a ticket. (examined)

## Verdict

Every QSL-2562 criterion has at least one live tagged test in this PR. The remaining end-to-end gaps are tracked by QSL-2627, QSL-2628 and QSL-2632. Order on the package route is not asserted (code-review FND-011). No gap blocks the merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
