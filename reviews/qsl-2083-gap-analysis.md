---
id: SR-1484
title: "gap-analysis of QSL-2083 operation intake and body tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-semantics/tests/it/operation_uniqueness.rs, fsl-semantics/tests/it/main.rs, fsl-package/src/emit/tests/operation_bodies/source_contracts.rs, fsl-package/src/emit/tests/operation_bodies.rs, reviews/qsl-2083-focused.json"
review_set: subset
---

## Summary

Ticket: QSL-2083. Closed PR: filament-spec-language#3164. Frozen branch: codex/qsl-2083-operation-intake-bodies. Reviewed SHA: 5aee8e7f72b13f6b7ea1c82ffe19302335a57e6b. The PR was closed solely to preserve one open PR per repository; it was not reopened.

Review of operation intake refusals and semantic body recovery found 2 confined test-oracle or coverage defects.

## Verdict

**CONDITIONAL** — resolve the findings in the existing feature branch before its final gate. Existing tracked production failures are not new review findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The refusal-reordering case exercises only duplicate operation identity (AC-4). The required equal-name, parameter identity/name, reserved-result and precedence refusals are never compared across unrelated-record reorderings. A reader path that retargets those loci after a type moves would pass the new suite; add the remaining required refusal variants, retaining tracked ignores where their underlying production bug is still open. | fsl-semantics/tests/it/operation_uniqueness.rs:618 |
| FND-002 | medium | The strict pending equal-operation-name oracle checks the second name locus and refusal codes, but discards the returned refusal without asserting that both colliding operations are named. A diagnostic that names only the second operation satisfies this oracle even after QSL-2696 is fixed. Assert both collision participants through the documented structured cause or retained diagnostic in this case and its precedence variant. | fsl-semantics/tests/it/operation_uniqueness.rs:94 |

## Coverage

This is a review of the exact five-file test/receipt diff, not a repository assurance audit. No production changes or CI/gate changes are present. The new tests reuse existing model and emission helpers and exercise real model intake, dispatch/check, emission, and strict read_v2 over emission.package.bytes(). Original model evidence supplies domain input; it does not supply the authored body.

Author receipts are accepted: 22 authored intake cases and six body cases; eight strict pending cases are excluded from passing coverage and tracked under QSL-2696 and QSL-2697. No assertion is made that an unexecuted loop iteration or frame kind passed. QSL-1899 remains Done; QSL-2347 remains the existing owner for IR source-owned shapes. The existing called/uncalled node-id comparison remains in operation_bodies.rs.

No tests, builds, lint, probes, matrix, inventory or baseline checks ran. The 595 unchanged semantics lint findings are accepted external blockers. The original coder owns one final complete affected-subsystem gate after fixes and review. No global AssuranceProfile search was performed; no applicable literal profile pointer was supplied, so broader assurance context is outside this diff. Plan completion: not assessed.

For gap-analysis, matrix computation and repository-wide reverse-gap/stub scans were explicitly excluded by the assignment; the performed checks are confined criterion-to-test binding, behavioral-oracle, and reverse-ownership inspection of the changed tests. This is not an engine-backed repository PASS.

## Examined units and bindings

```yaml
{
  "scope": [
    {
      "id": "FR-512-AC-1",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "In the ConfigVersion package, an `operations` member on the population `config_history` is refused by the semantic-IR reader, whose diagnostic is retained, and no declaration is admitted."
    },
    {
      "id": "FR-512-AC-2",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "An operation held by an event type refuses `invalid_model_binding`/`malformed-declaration` at the operation; the same operation held by an object type is admitted."
    },
    {
      "id": "FR-512-AC-3",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "An operation held by a record value type refuses `unsupported_construct`/`declaration-form` at the operation."
    },
    {
      "id": "FR-512-AC-4",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "Two `Counter` operations that share the identity `.../Counter/bump` refuse `invalid_model_binding`/`conflicting-binding` at that identity, path `TypeDefinition.operations[1]`."
    },
    {
      "id": "FR-512-AC-5",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "Two `Counter` operations named `bump` with the distinct identities `.../Counter/bump` and `.../Counter/bump2` refuse `ambiguous_declaration`/`ambiguous-name` at the second operation's `name`, naming both."
    },
    {
      "id": "FR-512-AC-6",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "`Sub.bump` redefining `Counter.bump` through `redefines` is admitted."
    },
    {
      "id": "FR-512-AC-7",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "Parameters `a` and `a` with one identity refuse `invalid_model_binding`/`conflicting-binding` at the shared identity, path `TypeDefinition.operations[k].params[1]`, with the reader's `DUPLICATE_PARAM` retained and no other refusal."
    },
    {
      "id": "FR-512-AC-8",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "Parameters both named `a` with the identities `.../a` and `.../b` give the reader's `DUPLICATE_PARAM` at `/ir/types/i/operations/k/params/1/name` and no FSL intake refusal and no declaration."
    },
    {
      "id": "FR-512-AC-9",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "A parameter named `x` with identity `.../b`, and no equal name among the operation's parameters, refuses `invalid_model_binding`/`malformed-declaration` at its `name`."
    },
    {
      "id": "FR-512-AC-10",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "A parameter named `result` on an operation that declares `returns` refuses `ambiguous_declaration`/`ambiguous-name` at its `name`."
    },
    {
      "id": "FR-512-AC-11",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "A parameter named `result` on an operation with no `returns` refuses `ambiguous_declaration`/`ambiguous-name` at its `name`."
    },
    {
      "id": "FR-512-AC-12",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "`modifies [Gauge/g, Gauge/g]` and `modifies_receiver [Gauge/g, Gauge/g]` are each admitted with an effect equal to the one-entry list; `modifies [Gauge/g, Gauge/h]` holds both in declared order."
    },
    {
      "id": "FR-512-AC-13",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "`creates [A, A]`, `deletes [B, B]` and `deletes_parameters [p, p]` (for a `Reference` parameter `p`) are each admitted with an effect equal to the one-entry list."
    },
    {
      "id": "FR-512-AC-14",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "`creates [A]` with `deletes [A]` is admitted; `modifies [Gauge/g]` with `modifies_receiver [Gauge/g]` refuses `invalid_model_binding`/`conflicting-binding` (FR-154-AC-4)."
    },
    {
      "id": "FR-512-AC-15",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "An operation with repeated parameters `a`, `a` and an unresolved `modifies` entry reports the `conflicting-binding` refusal only; one with a name/identity-mismatched parameter and an unresolved `modifies` entry reports the `malformed-declaration` refusal only; one with a `result` parameter and an unresolved `modifies` entry reports the `ambiguous-name` refusal only."
    },
    {
      "id": "FR-512-AC-16",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "A `Counter` with two operations named `bump` of distinct identities, the second also having a parameter `result`, reports the operation-name refusal only; an event type holding two operations of one name reports the owner refusal only; a record value type holding two operations of one name reports the owner refusal only."
    },
    {
      "id": "FR-512-AC-17",
      "path": "spec/functional/FR-512-keep-operation-names-parameters-and-frame-entries-unique-at-intake.md",
      "role": "examined",
      "excerpt": "Admitting the ConfigVersion package with its operations and frame entries reordered in the document still gives equal `PackageDeclarations` (FR-103-AC-5), and the refusals of AC-4 to AC-11 and AC-15 to AC-16 name the same entries under the reordering of unrelated records."
    },
    {
      "id": "FR-576-AC-1",
      "path": "spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md",
      "role": "examined",
      "excerpt": "A unit with `A.size` having an `operation-body` and a dispatched call that reaches it emits an `operation_bodies` row for `A.size` whose `body` references the FR-094 body function node; reading that node yields the authored expression with its source provenance."
    },
    {
      "id": "FR-576-AC-2",
      "path": "spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md",
      "role": "examined",
      "excerpt": "Operation `A.empty` has a body and no call in the unit reaches it: the package still holds its body function node and a row referencing it. The same source with a wrong-typed `A.empty` body, and again with a body whose parameter count differs from `A.empty`'s signature, each refuses `ill_typed`/`type-mismatch` at that body and emits no package."
    },
    {
      "id": "FR-576-AC-3",
      "path": "spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md",
      "role": "examined",
      "excerpt": "An operation without an `operation-body` has exactly one row whose `body` is `no_body`, and the package holds no body function node, default value or empty body for it."
    },
    {
      "id": "FR-576-AC-9",
      "path": "spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md",
      "role": "examined",
      "excerpt": "Each row, together with the graph node it names, yields the operation identity and the body expression with source provenance by lookup alone: for the AC-1 package, resolving the row's `body` in the package graph returns the authored body for the row's operation, and for a `no_body` row no node lookup is needed."
    },
    {
      "id": "FR-576-AC-10",
      "path": "spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md",
      "role": "examined",
      "excerpt": "Operation `A.touch` declares no result type and has an `operation-body`: the compile refuses `ill_typed`/`type-mismatch` located at that body and emits no package. The same unit with the body removed emits a `no_body` row for `A.touch` in `operation_bodies`; with a declared result and a matching body it emits a row referencing the body function."
    }
  ],
  "bindings": [
    {
      "test_id": "duplicate_operation_identity_refuses_at_the_second_operation",
      "ac_id": "FR-512-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "equal_operation_names_with_distinct_identities_refuse_at_the_second_name",
      "ac_id": "FR-512-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "duplicate_parameter_identity_preempts_an_unresolved_frame",
      "ac_id": "FR-512-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "duplicate_parameter_identity_preempts_an_unresolved_frame",
      "ac_id": "FR-512-AC-15",
      "trace": "correct"
    },
    {
      "test_id": "duplicate_parameter_names_preserve_the_readers_exact_repeated_name",
      "ac_id": "FR-512-AC-8",
      "trace": "correct"
    },
    {
      "test_id": "mismatched_parameter_name_preempts_frame_resolution",
      "ac_id": "FR-512-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "mismatched_parameter_name_preempts_frame_resolution",
      "ac_id": "FR-512-AC-15",
      "trace": "correct"
    },
    {
      "test_id": "result_parameter_is_reserved_with_and_without_a_return_type",
      "ac_id": "FR-512-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "result_parameter_is_reserved_with_and_without_a_return_type",
      "ac_id": "FR-512-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "result_parameter_is_reserved_with_and_without_a_return_type",
      "ac_id": "FR-512-AC-15",
      "trace": "correct"
    },
    {
      "test_id": "repeated_frame_entries_deduplicate_without_losing_distinct_entries",
      "ac_id": "FR-512-AC-12",
      "trace": "correct"
    },
    {
      "test_id": "receiver_frame_duplicates_deduplicate",
      "ac_id": "FR-512-AC-12",
      "trace": "correct"
    },
    {
      "test_id": "creation_frame_duplicates_deduplicate",
      "ac_id": "FR-512-AC-13",
      "trace": "correct"
    },
    {
      "test_id": "deletion_frame_duplicates_deduplicate",
      "ac_id": "FR-512-AC-13",
      "trace": "correct"
    },
    {
      "test_id": "parameter_deletion_frame_duplicates_deduplicate",
      "ac_id": "FR-512-AC-13",
      "trace": "correct"
    },
    {
      "test_id": "operation_and_frame_order_do_not_change_the_selected_declarations",
      "ac_id": "FR-512-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "an_explicit_redefinition_keeps_its_own_operation_identity",
      "ac_id": "FR-512-AC-6",
      "trace": "correct"
    },
    {
      "test_id": "an_explicit_redefinition_keeps_its_own_operation_identity",
      "ac_id": "FR-512-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "repeated_operation_name_preempts_the_second_operations_reserved_parameter",
      "ac_id": "FR-512-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "repeated_operation_name_preempts_the_second_operations_reserved_parameter",
      "ac_id": "FR-512-AC-16",
      "trace": "correct"
    },
    {
      "test_id": "different_frame_scopes_conflict_but_creation_and_deletion_can_share_a_type",
      "ac_id": "FR-512-AC-12",
      "trace": "correct"
    },
    {
      "test_id": "different_frame_scopes_conflict_but_creation_and_deletion_can_share_a_type",
      "ac_id": "FR-512-AC-14",
      "trace": "correct"
    },
    {
      "test_id": "a_population_cannot_acquire_an_operations_member",
      "ac_id": "FR-512-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "event_owner_refuses_before_operation_name_collisions",
      "ac_id": "FR-512-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "event_owner_refuses_before_operation_name_collisions",
      "ac_id": "FR-512-AC-16",
      "trace": "correct"
    },
    {
      "test_id": "record_owner_refuses_before_operation_name_collisions",
      "ac_id": "FR-512-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "record_owner_refuses_before_operation_name_collisions",
      "ac_id": "FR-512-AC-16",
      "trace": "correct"
    },
    {
      "test_id": "unrelated_objects_can_declare_the_same_operation_name",
      "ac_id": "FR-512-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "result_parameter_without_a_return_type_refuses_before_frame_resolution",
      "ac_id": "FR-512-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "result_parameter_without_a_return_type_refuses_before_frame_resolution",
      "ac_id": "FR-512-AC-15",
      "trace": "correct"
    },
    {
      "test_id": "two_distinct_modifies_grants_keep_their_declared_order",
      "ac_id": "FR-512-AC-12",
      "trace": "correct"
    },
    {
      "test_id": "unrelated_record_order_does_not_retarget_a_duplicate_operation_identity",
      "ac_id": "FR-512-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "unrelated_record_order_does_not_retarget_a_duplicate_operation_identity",
      "ac_id": "FR-512-AC-17",
      "trace": "correct"
    },
    {
      "test_id": "strict_body_lookup_recovers_called_integer_and_uncalled_boolean_values",
      "ac_id": "FR-576-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "strict_body_lookup_recovers_called_integer_and_uncalled_boolean_values",
      "ac_id": "FR-576-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "strict_body_lookup_recovers_called_integer_and_uncalled_boolean_values",
      "ac_id": "FR-576-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "a_bodyless_void_operation_has_explicit_absence_beside_a_real_body",
      "ac_id": "FR-576-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "a_bodyless_void_operation_has_explicit_absence_beside_a_real_body",
      "ac_id": "FR-576-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "uncalled_wrong_type_body_refuses_at_its_body",
      "ac_id": "FR-576-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "uncalled_wrong_arity_body_refuses_at_its_body",
      "ac_id": "FR-576-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "void_operation_body_refuses_at_its_body",
      "ac_id": "FR-576-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "a_parameterized_conditional_body_keeps_its_branches_in_order",
      "ac_id": "FR-576-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "a_parameterized_conditional_body_keeps_its_branches_in_order",
      "ac_id": "FR-576-AC-9",
      "trace": "correct"
    }
  ]
}
```

## Dispositions

Round 1 inspected only fix diff 5aee8e7f72b13f6b7ea1c82ffe19302335a57e6b..51b9efb986727c478ce7a60543bb099223a9c011, on the same closed PR3164 and branch.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Inspected fix at 51b9efb986727c478ce7a60543bb099223a9c011; see after excerpt below. |
| FND-002 | fixed | Inspected fix at 51b9efb986727c478ce7a60543bb099223a9c011; see after excerpt below. |

The fix covers all remaining required refusal-reordering families with precise operation/parameter identities and member paths. AC-8 calculates the source owner index after each permutation. Known namespace/result-name failures remain tracked ignores. Both equal-name cases and the reordered variant invoke the shared oracle for the two distinct qualified operation identities, rather than merely their repeated display name.

```yaml
{
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "after_excerpt": "fn unrelated_record_order_keeps_duplicate_parameter_identity_and_precedence_loci() {\n    for bad_frame in [false, true] {\n        let parameter = operation_parameter(\"advance\", \"amount\", INTEGER);\n        let mut operation = query(\"advance\", json!([parameter.clone(), parameter]));\n        if bad_frame {\n            operation[\"frame\"][\"modifies\"] = json!([key(\"missingField\")]);\n        }\n        for document in unrelated_record_permutations(&input(vec![operation])) {"
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "after_excerpt": "fn assert_collision_participants(found: &fsl_semantics::model::normalize::ModelRefusal) {\n    assert!(\n        found.detail.contains(&key(\"advance\")) && found.detail.contains(&key(\"other\")),\n        \"both distinct colliding operation identities must be retained: {found:?}\"\n    );\n}"
    }
  ]
}
```

## Current disposition verdict

**PASS** — all findings resolved; no new finding in the confined fix diff. This is a source review verdict, not a full-gate claim. Author reports three supported reordered cases passed, three known ignored, and the strengthened conditional passed; these results are accepted without rerunning. Known QSL-2696/QSL-2697 remain unprobed. No tests, lint, builds, product probes, matrix, profile or cache searches ran. The original coder owns the one affected component gate after queue release.
