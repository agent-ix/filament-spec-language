---
id: SR-20261123070012001
title: "code review (with Rust lane) of PR 3560 object-free hyper clause"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3560 head e89d6e0dba29046bd13ab91c9c161ae82fe4e4f0 vs origin/main; fsl-cst/src/grammar.rs, fsl-cst/src/cst.rs, fsl-analyze/tests/it/hyper_object_free.rs, fsl-analyze/tests/it/hyper_world.rs, fsl-analyze/tests/it/analyze_family_cli.rs"
review_set: subset
---

## Summary

Ticket: QSL-2887. The grammar change (`behaviours ()` admitted by an inner `opt`) matches FR-488-AC-28, FR-631-AC-4, TC-3010 and FSpec FR-402-AC-11, which all spell the empty list. FR-171 says the domain holds "with or without object parameters". The only CST consumer, fsl-forms `hyper_clause.rs:171`, collects `Parameter` children into a Vec, so an empty list gives the same form as bare `behaviours`. The token formatter treats the `(` after `behaviours` as a non-call parenthesis (`format.rs:345`) and prints `behaviours ()`, so the source round-trips. The harness changes are correct: `package_with_clause`, the vault domain document given to the strict reader, operand-slot resolution, and the quantity snapshot form. Deleting `an_image_outside_the_unit_bound_is_initial_not_abstract` is correct. FR-630-AC-5 states that `Shifted` is refused at S3 `ill_typed`/`type-mismatch` and that "the empty completion set is FR-630-AC-6's". The sibling `a_mapping_that_does_not_conform_to_the_abstract_field_is_refused` asserts the code, the cause and `items: []`. Focused run on titan at e89d6e0: 4 passed, and the 2 ignored tests fail as their reasons describe (`a_nested_reference_row_checks_and_proves`: null terminal record; the unit-bound test: `initial` is 0, not "0"). fsl-cst `hyper` filter: 11 passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The refusal test asserts only `!admitted`. `admitted` is false for any `Refused`, `Incomplete` or `CatalogUnavailable` result. The edit puts a resolved-form id into the raw wire, so the test can pass on a refusal unrelated to the over binding. FR-488-AC-28 names `invalid_model_binding`/`malformed-declaration` at `/semantic_graph/nodes/{w}/body/arguments/0` | fsl-analyze/tests/it/hyper_object_free.rs:49-65,99-131 |
| FND-002 | medium | The ignore reason cites QSL-2887 and blames only the test source, which is invalid. The live blocker is that S3 refuses every nested-reference row as `nested-reference-row` (refinement.rs:3569), against FR-135, which says "support set SHALL hold nested-reference-row". QSL-2945 owns that. The reason should cite QSL-2945 | fsl-analyze/tests/it/analyze_family_cli.rs:338 |
| FND-003 | medium | The ignore reason calls this a spec gap, but the counterexample wire writes every `refinement_failure` member as a JSON number (counterexample.rs:1203), and refinement_safety.rs:310 asserts `position` as the number 1. FR-149's decimal string is the certificate verdict form (simulation.rs:250). So the expected `"0"` is the defect. The test should assert `0` and be un-ignored, or the reason should cite a live ticket. It cites QSL-2887, which this PR addresses | fsl-analyze/tests/it/analyze_family_cli.rs:1427,1458-1462 |
| FND-004 | low | The wrapper is now found as the first `temporal_clause` node, not through the hyperproperty's body member. That is correct only because the Secure vault declares no `temporal` clause; adding one would silently edit the wrong node | fsl-analyze/tests/it/hyper_object_free.rs:116-120 |
| FND-005 | low | `admitted` always supplies `document(false, false)`, but `package_with_clause` builds against `document(vault.busy(), false)`. For a Busy vault the reader would get a domain document that differs from the one the package was checked against | fsl-analyze/tests/it/hyper_object_free.rs:55; fsl-analyze/tests/it/hyper_world.rs:1053 |
| FND-006 | low | No S1 or formatter test covers the new `behaviours ()` alternative. It is exercised only end to end through fsl-analyze | fsl-cst/src/grammar.rs:3070-3077 |
| FND-007 | low | The test asserts `proved` or `refuted` for `always holds(true)`, which can only be `proved`. A wrong `refuted` would pass | fsl-analyze/tests/it/analyze_family_cli.rs:402 |

## Verdict

Not mergeable yet. Run one fix round for FND-001 to FND-003: assert the exact refusal code and pointer, and either correct or re-cite the two ignore reasons. Note that QSL-2887's acceptance ("the listed tests pass un-ignored") is not met, because two of the six tests stay ignored. Do not close the ticket on merge unless both ignores cite their live owners. The grammar change, the harness changes and the test deletion are sound. The Rust lane found no panics in non-test code and no API change. The only non-test edit is a grammar sequence and its doc comment.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | Under the arch ruling, the unit-bound ignore still cites QSL-2887, and no open Linear ticket tracks the FR-379 amendment or the writer change. Two fixes are possible: keep QSL-2887 open after merge (the PR says "Refs", not "Closes"), or file a ticket for the FR-379 amendment and the writer change and cite it | fsl-analyze/tests/it/analyze_family_cli.rs:1434 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | assert_root_over_refused asserts Refused, InvalidModelBinding, MalformedDeclaration and the pointer /semantic_graph/nodes/{at}/body/arguments/0. The edit is made on the resolved wire, and write_operand_rows rewrites the operand rows (ffc49aae) |
| FND-002 | fixed | The ignore cites QSL-2945 and the FR-630-AC-4 row rewrite |
| FND-003 | rejected | Overridden by an arch ruling: `initial` is a decimal string, so the expected "0" is correct and the writer is wrong. The test stays ignored until the FR-379 amendment lands. The residual citation is FND-008 |
| FND-004 | fixed | wrapper_index locates the wrapper through the hyperproperty's `body` member, by name |
| FND-005 | fixed | clause_documents(vault, second) feeds both package_with_clause and read |
| FND-006 | fixed | Added fsl-cst hyper_clause::a_clause_over_behaviours_with_no_object_parameter_parses and formatter_style::an_empty_behaviours_parameter_list_prints_canonically |
| FND-007 | fixed | assert_eq!(label(&run.item(0)), "proved") |
| FND-008 | fixed | Superseded at 8e1abb6d. The FR-379 writer change (967029648b) writes `initial` and `position` as decimal strings, matching FSpec FR-379 as amended by filament-specification #820 (U64String, FR-379-AC-14), and the ignore is removed. The test passes un-ignored, and its assertions are unchanged |
