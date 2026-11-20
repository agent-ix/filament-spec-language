---
id: SR-1489
title: "code-review of QSL-2085 checker and value boundaries"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2085; tests/it/checker_value_boundaries.rs and its five child modules; tests/it/main.rs"
review_set: subset
---

## Summary

Reviewed frozen source 70ad3c157392731a3f75da225242c3666ea99dd1. The real public behavior cases have one medium test-oracle defect: generated guard checks can pass with no accepted formula.

## Verdict

**PASS** — FND-001 is fixed in f1b283aa03ee72446fc52e64b1422a86466a7317. All three TC-053 mandatory controls must check, with independent runtime Boolean expectations; conservative refusal remains permitted elsewhere.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The generated guard test permits every formula to return UndefinedExpression, so it passes with zero accepted clauses and omits the required impossible-path acceptance control. TC-053 explicitly requires direct-presence, impossible-path and common-fact positives and forbids vacuous refusal of all formulas. | tests/it/checker_value_boundaries/guards.rs:47; spec/test-cases/TC-053-prove-guard-fact-soundness.md |

## Scope

```json
[
  {
    "id": "checker_value_boundaries",
    "path": "tests/it/checker_value_boundaries.rs",
    "role": "examined",
    "excerpt": "fn check(body: &str, dependencies: &DependencyInput) -> Result<CheckedUnit, Box<CompileRefusal>> {"
  },
  {
    "id": "byte_concatenation_preserves_zero_octets_and_input_order",
    "path": "tests/it/checker_value_boundaries/evaluation.rs",
    "role": "examined",
    "excerpt": "fn byte_concatenation_preserves_zero_octets_and_input_order() {"
  },
  {
    "id": "native_json_and_raw_strings_preserve_distinct_payload_meanings",
    "path": "tests/it/checker_value_boundaries/evaluation.rs",
    "role": "examined",
    "excerpt": "fn native_json_and_raw_strings_preserve_distinct_payload_meanings() {"
  },
  {
    "id": "duration_addition_compares_after_explicit_unit_conversion",
    "path": "tests/it/checker_value_boundaries/evaluation.rs",
    "role": "examined",
    "excerpt": "fn duration_addition_compares_after_explicit_unit_conversion() {"
  },
  {
    "id": "raw_strings_keep_canonical_equivalents_distinct_and_never_become_uuid_values",
    "path": "tests/it/checker_value_boundaries/evaluation.rs",
    "role": "examined",
    "excerpt": "fn raw_strings_keep_canonical_equivalents_distinct_and_never_become_uuid_values() {"
  },
  {
    "id": "recursive_work_fuel_is_exact_at_the_last_required_charge",
    "path": "tests/it/checker_value_boundaries/evaluation.rs",
    "role": "examined",
    "excerpt": "fn recursive_work_fuel_is_exact_at_the_last_required_charge() {"
  },
  {
    "id": "nested_value_type_clone_compare_hash_debug_and_drop_use_bounded_stack",
    "path": "tests/it/checker_value_boundaries/evaluation.rs",
    "role": "examined",
    "excerpt": "fn nested_value_type_clone_compare_hash_debug_and_drop_use_bounded_stack() {"
  },
  {
    "id": "recursively_built_record_values_compare_clone_and_drop_at_real_depth",
    "path": "tests/it/checker_value_boundaries/evaluation.rs",
    "role": "examined",
    "excerpt": "fn recursively_built_record_values_compare_clone_and_drop_at_real_depth() {"
  },
  {
    "id": "duration_wire_payloads_keep_exact_magnitudes_and_require_the_declared_unit",
    "path": "tests/it/checker_value_boundaries/evaluation.rs",
    "role": "examined",
    "excerpt": "fn duration_wire_payloads_keep_exact_magnitudes_and_require_the_declared_unit() {"
  },
  {
    "id": "accepted_three_variable_guards_never_admit_an_absent_unwrap",
    "path": "tests/it/checker_value_boundaries/guards.rs",
    "role": "examined",
    "excerpt": "fn accepted_three_variable_guards_never_admit_an_absent_unwrap() {"
  },
  {
    "id": "an_unrelated_optional_presence_cannot_discharge_the_unwrap",
    "path": "tests/it/checker_value_boundaries/guards.rs",
    "role": "examined",
    "excerpt": "fn an_unrelated_optional_presence_cannot_discharge_the_unwrap() {"
  },
  {
    "id": "short_circuit_order_protects_only_the_following_optional_arithmetic",
    "path": "tests/it/checker_value_boundaries/guards.rs",
    "role": "examined",
    "excerpt": "fn short_circuit_order_protects_only_the_following_optional_arithmetic() {"
  },
  {
    "id": "a_let_binding_does_not_borrow_decimal_context_from_its_later_use",
    "path": "tests/it/checker_value_boundaries/guards.rs",
    "role": "examined",
    "excerpt": "fn a_let_binding_does_not_borrow_decimal_context_from_its_later_use() {"
  },
  {
    "id": "a_strict_finite_guard_discharges_only_the_admitted_successor_range",
    "path": "tests/it/checker_value_boundaries/guards.rs",
    "role": "examined",
    "excerpt": "fn a_strict_finite_guard_discharges_only_the_admitted_successor_range() {"
  },
  {
    "id": "failed_missing_linkage_does_not_poison_a_later_qualified_request",
    "path": "tests/it/checker_value_boundaries/imports.rs",
    "role": "examined",
    "excerpt": "fn failed_missing_linkage_does_not_poison_a_later_qualified_request() {"
  },
  {
    "id": "imported_parameter_boundary_matches_local_discharge",
    "path": "tests/it/checker_value_boundaries/imports.rs",
    "role": "examined",
    "excerpt": "fn imported_parameter_boundary_matches_local_discharge() {"
  },
  {
    "id": "qualified_same_named_exports_keep_their_distinct_function_bodies",
    "path": "tests/it/checker_value_boundaries/imports.rs",
    "role": "examined",
    "excerpt": "fn qualified_same_named_exports_keep_their_distinct_function_bodies() {"
  },
  {
    "id": "record_native_fields_distinguish_string_uuid_and_payload_kinds",
    "path": "tests/it/checker_value_boundaries/observations.rs",
    "role": "examined",
    "excerpt": "fn record_native_fields_distinguish_string_uuid_and_payload_kinds() {"
  },
  {
    "id": "nested_record_shape_defects_do_not_admit_partial_field_values",
    "path": "tests/it/checker_value_boundaries/observations.rs",
    "role": "examined",
    "excerpt": "fn nested_record_shape_defects_do_not_admit_partial_field_values() {"
  },
  {
    "id": "presence_comes_from_optional_field_and_not_sequence_cardinality",
    "path": "tests/it/checker_value_boundaries/observations.rs",
    "role": "examined",
    "excerpt": "fn presence_comes_from_optional_field_and_not_sequence_cardinality() {"
  },
  {
    "id": "scalar_expressions_cannot_replace_a_boolean_clause",
    "path": "tests/it/checker_value_boundaries/observations.rs",
    "role": "examined",
    "excerpt": "fn scalar_expressions_cannot_replace_a_boolean_clause() {"
  },
  {
    "id": "invocation_native_parameters_and_result_are_admitted_in_their_declared_roles",
    "path": "tests/it/checker_value_boundaries/observations.rs",
    "role": "examined",
    "excerpt": "fn invocation_native_parameters_and_result_are_admitted_in_their_declared_roles() {"
  },
  {
    "id": "state_keys",
    "path": "tests/it/checker_value_boundaries/state_keys.rs",
    "role": "examined",
    "excerpt": "fn deep_record_state_keys_preserve_each_link_and_the_absent_terminal()"
  },
  {
    "id": "registration",
    "path": "tests/it/main.rs",
    "role": "examined",
    "excerpt": "#[cfg(feature = \"checked-handoff\")]\nmod checker_value_boundaries;"
  },
  {
    "id": "TC-053",
    "path": "spec/test-cases/TC-053-prove-guard-fact-soundness.md",
    "role": "examined",
    "excerpt": "The direct-presence, impossible-path and common-fact\njoin controls are accepted; a guard that can be true with absent parent refuses.\nThe test records how many formulas and assignments were exercised. Conservative\nrefusal of another logically safe formula is permitted; vacuous refusal of all\nformulas cannot satisfy the mandatory positive controls."
  },
  {
    "id": "FR-262-AC-1",
    "path": "spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md",
    "role": "examined",
    "excerpt": "With `work_units` one below the run's measured spend, the same call returns `incomplete { limit_kind: work_units }` at the denied charge, and completes once `work_units` is raised to the measured spend."
  },
  {
    "id": "FR-262-AC-2",
    "path": "spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md",
    "role": "examined",
    "excerpt": "On a thread with a 512 KiB stack, a recursive list value 100,000 long (`record List { head: Int[0, 9]; tail: List?; }`), built by a recursive function, evaluates; it keys as simulation state, and its state-key bytes equal the RFC 8785 text of its canonical JSON form; it compares equal to its clone, and it and its clone drop."
  },
  {
    "id": "FR-056-AC-23",
    "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
    "role": "examined",
    "excerpt": "K2's native String value retains any Unicode scalar sequence exactly, including empty, ASCII, multibyte and UUID-looking text; equality is exact-sequence equality, its key is UTF-8 byte order, and Text normalization does not apply."
  },
  {
    "id": "FR-099-AC-3",
    "path": "spec/functional/FR-099-compile-against-supplied-libraries.md",
    "role": "examined",
    "excerpt": "With no library supplied as `test/geometry` the import refuses `missing_import`/`missing-selection` at the import's identity string, stage `intake`;"
  },
  {
    "id": "FR-099-AC-8",
    "path": "spec/functional/FR-099-compile-against-supplied-libraries.md",
    "role": "examined",
    "excerpt": "each call `g::f(e)` receives the same verdict, refusal code, cause and locus as `f2(e)` in the same body"
  },
  {
    "id": "FR-104-AC-16",
    "path": "spec/functional/FR-104-check-state-clauses.md",
    "role": "examined",
    "excerpt": "because a `let` binding supplies no expected type;"
  },
  {
    "id": "FR-106-AC-15",
    "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
    "role": "examined",
    "excerpt": "A record payload with a missing effective field refuses `invalid_runtime_input`/`missing-member`; one with an extra field refuses `unknown-member`; a `type` naming an object type, an unrelated record subtype, or no selected record refuses `wrong-value-kind`; and an object reference in place of a record refuses `wrong-value-kind`."
  },
  {
    "id": "FR-650-AC-10",
    "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
    "role": "examined",
    "excerpt": "and called with `unit` `\"s\"` refuses `invalid_runtime_input`/`wrong-value-kind` at `/unit`;"
  },
  {
    "id": "TC-895",
    "path": "spec/test-cases/TC-895-the-native-checker-reads-optionality-from-presence.md",
    "role": "examined",
    "excerpt": "Step 1: `x.f` types as a sequence of at most one Boolean, and\n  `present(x.f)` refuses `ill_typed` at `x.f`."
  },
  {
    "id": "TC-1231",
    "path": "spec/test-cases/TC-1231-native-bytes-jsonobject-and-duration-in-the-unit-language.md",
    "role": "context_only",
    "excerpt": "a Duration is the type\n  `Quantity<s>` and has no node of its own."
  }
]
```

## Coverage

Diff-scoped review only, under explicit lead/user scope. Nineteen active tests have accepted focused PASS receipts; three strict pending cases are not passing coverage (QSL-2610 source Bytes/JsonObject surface; QSL-2704 complete recursive-state journey stack overflow). No phase attribution is made for the abort. No tests, builds, lint, matrix, repository-wide inventory, or prior Done audit ran. The exact current criteria above are partial behavioral slices, not claims to discharge every subcase. No production code, wire vocabulary, gate policy or dependency changes are present; existing repository constructors and frontend are reused. No applicable AssuranceProfile was found in the narrow profile-name lookup; no profile was invented. The only concrete defect is the mandatory positive-control gap below. Plan completion: not assessed

## Required correction

Mark the current generated direct-presence and common-fact controls as required acceptance, add an impossible-path control grounded in TC-053, and require those controls to check. Keep the independent eight-assignment soundness oracle and typed refusal checks for the remaining formulas. Record actual generated formula/assignment counts. Run only this changed qualified test while fixing; one final affected-component gate belongs after review disposition.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, f1b283aa03ee72446fc52e64b1422a86466a7317: direct-presence, common-fact and impossible-path formulas carry mandatory acceptance labels; refusal of any labeled control fails. Counts require three accepted controls and 36 real evaluations against independently computed Boolean outcomes. The reported changed-test receipt passed 35 formulas, 280 assignments, 9 accepted and 26 permitted refusals. No execution was repeated. |

The original finding remains unchanged. This confined disposition inspected only the guard-test fix. No new finding; final affected-component gate remains the code lead's next step.

