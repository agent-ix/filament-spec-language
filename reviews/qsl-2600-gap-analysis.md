---
id: SR-60053
title: "gap-analysis of filament-spec-language PR 3230, FR-710-AC-1..5"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3230 head 089952824309ead35c2797dc83ae98c378ba66d9; FR-710-AC-1..5 against quoin matrix and the changed tests"
review_set: subset
---

## Summary

Ticket: QSL-2600. A scoped audit of the acceptance criteria against tests, using `quoin matrix --json` (quoin 0.36.8) plus a read of each test's oracle. Plan completion: not assessed. The full-repository reverse-gap and stub sweep was not run (scope = PR diff).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-710-AC-2's `reaches(a, b, next)` clause has no test whose oracle depends on edge direction or uses two distinct nodes. The only evaluation test is reflexive over a symmetric graph (see SR-60052 FND-002). | fsl-command/tests/it/model_unit_request.rs:653 |
| FND-002 | low | FR-710-AC-2's "with its ends and multiplicity" is checked only for the target lower bound. No test asserts end roles, source-end multiplicity or an inverse end. | fsl-semantics/tests/it/model_unit_input.rs:296 |

## Scope and evidence

quoin matrix binds tests to every AC from FR-710-AC-1 to AC-9. The AC-1..5 binders include fsl-cst/tests/it/model_unit.rs, fsl-semantics/tests/it/model_unit_input.rs and fsl-command/tests/it/model_unit_request.rs. The 17 AC-3 tags include the new complete_grammar ModelUnit/Domain* vectors.

- FR-710-AC-1: examined. compile_model_unit_returns_its_identity_and_stable_document_bytes and related tests. Clean.
- FR-710-AC-2: examined. a_model_unit_compiles_its_relationship_and_population and reaches_checks_and_evaluates_over_a_model_unit_relationship. FND-001/002.
- FR-710-AC-3: examined. Per-form admit tests plus one_model_unit_compiles_and_admits_every_declaration_form. Clean.
- FR-710-AC-4: examined. The refusal tests in fsl-cst and fsl-semantics. Clean.
- FR-710-AC-5: examined. a_supplied_model_unit_is_admitted_as_its_compiled_document and a_model_unit_for_another_identity_is_refused_wrong_model_selection. Clean.

## Verdict

AC-1, 3, 4 and 5 are covered by tests whose oracles can fail. AC-2's evaluation clause is weakly covered.
