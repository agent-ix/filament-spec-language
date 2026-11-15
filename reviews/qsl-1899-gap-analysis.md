---
id: SR-2693002
title: "gap analysis of FR-576 operation body build and carrier (PR 2693)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2693 diff origin/main...HEAD at ee377de43: FR-576 AC-1 to AC-10, TC-1011, FR-302-AC-8 against qsl-package/src/emit/tests/operation_bodies.rs, qsl-replay/tests/dispatch_link_compile.rs and the production changes in qsl-semantics and qsl-package"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1011
    type: reviews
---

## Summary

Ticket: QSL-1899. Planless gap analysis. Plan completion: not assessed. This
maps each FR-576 acceptance criterion to the tagged tests in the PR diff and
to the code that implements it.

| AC | Tagged test | Status |
| --- | --- | --- |
| FR-576-AC-1 | `a_called_body_is_referenced_by_its_row` | covered, weak oracle (FND-002) |
| FR-576-AC-2 | `an_uncalled_body_has_the_node_id_it_has_when_called`, `an_uncalled_body_is_checked_for_its_type_and_arity` | covered |
| FR-576-AC-3 | `an_operation_without_a_body_has_a_no_body_row_and_no_node` | covered |
| FR-576-AC-4 | `same_named_operations_of_two_owners_have_separate_rows` | covered |
| FR-576-AC-5 | `an_inherited_operation_has_no_row_and_a_redefinition_has_its_own` | covered |
| FR-576-AC-6 | `two_aliases_of_one_document_emit_one_row_per_operation` | covered |
| FR-576-AC-7 | `a_candidates_row_carries_the_tables_operation_body_identity` | covered |
| FR-576-AC-8 | `an_uncalled_body_with_a_dispatched_call_refuses_operator_ineligible` | covered |
| FR-576-AC-9 | `a_called_body_is_referenced_by_its_row` | covered, weak oracle (FND-002) |
| FR-576-AC-10 | none | gap, and not implemented (FND-001) |

Every new production function has an owning requirement (FR-576, FR-094,
FR-302). No stubs were found. All rows in the tests are read from the
in-memory `Emission` field rather than the bytes. That is recorded as
FND-005 in SR-2693001 and not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-576-AC-10 (TC-1011 steps 5-7) has no test and no implementation; the resultless-body case compiles and withholds the member | spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md:129 |
| FND-002 | medium | The AC-1/AC-9 test checks only that the node's body JSON contains the string `"body"`; it never asserts the node yields the authored expression `41` for that operation | qsl-package/src/emit/tests/operation_bodies.rs:194-198 |

## Verdict

AC-1 to AC-9 have tagged tests that pass on the reviewed head. AC-10, which
#2694 merged into the spec on this branch, is uncovered and unimplemented, so
the AC set is not complete. FND-002 is a weak oracle: a node that carries the
`body` clause binding but the wrong expression passes the test.

## Scope and evidence

- FR-576-AC-1 to FR-576-AC-9, examined, each mapped above.
- FR-576-AC-10, examined: Operation `A.touch` declares no result type and has an `operation-body`: the compile refuses `ill_typed`/`type-mismatch` located at that body and emits no package. The same unit with the body removed emits a `no_body` row; with a declared result and a matching body it emits a row referencing the body function.
- TC-1011 (spec/test-cases/TC-1011-unit-compile-checks-body-arity-and-type-against-the-operation.md), examined: steps 1-4 covered by the qsl-replay tests, steps 5-7 uncovered.
- FR-302-AC-8, context only: the existing called-body test plus the new uncalled cases.

## Dispositions

Round 1, against `git diff c25623b81..81d4e9e93`. The focused tests pass
(see SR-2693001 Dispositions).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | AC-10 is implemented. It is tagged by `a_body_on_an_operation_without_a_result_refuses_type_mismatch` (qsl-package: refusal, `no_body` without the body, function row with a declared result) and `a_body_on_an_operation_without_a_declared_result_refuses_type_mismatch` (qsl-replay: refusal located at the body, no package). |
| FND-002 | fixed | The AC-1/AC-9 test now asserts that the body node yields the authored expression `41`, read from the package bytes. |

After excerpts:

- FND-001: `#[trace("FR-576-AC-10", "TC-1011")] fn a_body_on_an_operation_without_a_result_refuses_type_mismatch() { ... assert_eq!(refusal.cause.cause(), Some("type-mismatch")); ... }`
- FND-002: `assert!(serde_json::to_string(&node["body"]).unwrap().contains("41"), "the node yields the authored expression 41: {node}");`
