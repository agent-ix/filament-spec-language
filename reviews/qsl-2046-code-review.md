---
id: SR-1417
title: "Code and Rust review of type/operator conformance pairs"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2046; PR 3107; fsl-conformance/tests/it/type_decision_cases.rs and cases/type-operators/**"
review_set: subset
---

## Summary

Reviewed the frozen conformance diff. No findings.

## Verdict

**PASS for this admission/type-refusal slice.** QSL-2046 remains partial under QSL-2610 and QSL-2371; evaluator behavior and repository-wide assurance are not claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed all changed native fixture pairs and the existing assert_pair harness extension at frozen head dacbdf3ea4daa9efcd35e167b8867c9f05f7d8da. Each active accepted source is sent through the public compile_package path to S4; each rejected source must yield exactly one typed assembly or Linked-check refusal with the expected code and exact cause. The matrix macro expands real tests, and its file-set check is supporting bookkeeping, not language-behavior evidence. The same existing helper is reused rather than introducing a parallel runner. Its strengthened exact assembly cause comparison does not branch on human messages. No production API, wire decoder, compatibility layer, copied external implementation, gate or CI workflow is introduced.

The reviewed cases cover Boolean connectives/equality; Integer arithmetic, slash/div/rem/mod, negation, comparison; bounded Rational and Decimal arithmetic, conversion-mediated Decimal equality and defined refusal boundaries; both IEEE widths; quantity dimensions, power/scaling, unit resolution and duration equality in seconds; Text/String separation and ordering; native UUID/Timestamp identity and ordering refusals; enum declaration identity/order; Option guarded projection; record/tuple/union/float-leaf equality; the four collection kinds with size, contains and type separation; and object-reference equality/order refusal. Type admission is the behavior under test: no evaluator result, accounting schedule or complete satisfaction of an enclosing FR/AC is claimed.

The pending source signature cases for Bytes/JsonObject (QSL-2610, related to completed QSL-2242) and the bounded Rational division-by-one admission (QSL-2371) retain intended positive sources outside the active corpus. UUID/Timestamp and a wider valid Rational division remain independent active cases; neither is presented as satisfying those pending obligations. No new ignored test or old known-incorrect output is used as passing conformance.

Accepted the coder's reported focused evidence for 47 pairs / 94 active units across 13 families: initial 32 passing pairs plus the 15 corrected/new pairs and supporting completeness test. The initial failing-run receipt is retained. No reviewer tests, builds, Clippy, matrix, root gate or prior-corpus checks were run. The lead owns one final applicable gate; because assert_pair is shared, that gate should select the complete type_decision_cases module, including its pre-existing active callers.

Rust review is folded into this artifact. Applied repository AGENTS/CLAUDE instructions and Rust style, testing, implementation and boundary guidance. Test assertions use real public behavior, typed codes, deterministic owned fixtures and exact refusal causes. Spec review is inapplicable because no specification changed.

Plan completion: not assessed.

## Examined Scope

```json
{
  "clean": true,
  "scope": [
    {
      "id": "FSpec-FR-146",
      "path": "filament-specification/spec/functional/expressions/FR-146-check-total-pure-functions.md",
      "role": "examined",
      "excerpt": "When admitting a function declaration, the checker SHALL establish typed purity, total definedness and a well-founded termination argument for every reachable call cycle."
    },
    {
      "id": "FSpec-FR-140-AC-7",
      "path": "filament-specification/spec/functional/type-model/FR-140-evaluate-exact-decimals.md",
      "role": "examined",
      "excerpt": "A `Rational` to `Decimal` conversion follows the decimal division rule and schedule at target scale `smax`, decimal operands and result share one named type, and a `decimal(c,s)` literal without a unique expected type is `ill_typed` with cause `ambiguous-literal`."
    },
    {
      "id": "FSpec-FR-141-AC-5",
      "path": "filament-specification/spec/functional/type-model/FR-141-evaluate-text-and-enumerations.md",
      "role": "examined",
      "excerpt": "An unordered enumeration comparison other than equality, or any cross-declaration enumeration comparison, refuses with `ill_typed` before any charge, while an ordered enumeration follows declaration order only."
    },
    {
      "id": "FSpec-FR-142",
      "path": "filament-specification/spec/functional/type-model/FR-142-evaluate-quantities-and-units.md",
      "role": "examined",
      "excerpt": "Addition/comparison require compatible dimensions and an explicit common-unit conversion. Multiplication/division combine dimension exponents."
    },
    {
      "id": "FSpec-FR-145",
      "path": "filament-specification/spec/functional/expressions/FR-145-convert-and-query-collections.md",
      "role": "context_only",
      "excerpt": "When applying a collection conversion or query, the evaluator SHALL derive the result kind, order, uniqueness, multiplicity and bound from the selected operation contract."
    },
    {
      "id": "FSpec-FR-147-AC-13",
      "path": "filament-specification/spec/functional/expressions/FR-147-evaluate-integer-division-domains.md",
      "role": "examined",
      "excerpt": "Under every law, `a div b`, `a rem b` and `a mod b` with `b: Int[0, 12]` and no exact nonzero guard are each refused `undefined_expression`/`unproved-nonzero` at the operator before any range obligation; with `b: Int[1, 12]` or under the guard `b != 0` the nonzero obligation is discharged."
    },
    {
      "id": "FSpec-FR-149-AC-9",
      "path": "filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md",
      "role": "examined",
      "excerpt": "Top-level Boolean, exact numeric and reference equality charge exactly `equality.plan-form` and the one-pair equality schedule; a top-level pair of two floats of one type charges FR-148's `numericEqual` schedule (`ieee.operands`, `ieee.result-retain`); and `=` over a record, tuple, option, union, sequence, set, bag or ordered set holding a float leaf is admitted under the equality schedule, never `ill_typed`."
    },
    {
      "id": "FSpec-FR-149-AC-15",
      "path": "filament-specification/spec/functional/type-model/FR-149-apply-complete-equality-matrix.md",
      "role": "examined",
      "excerpt": "UUID versus Timestamp, either kind versus `Int`, `Integer`, `Text`, `String` or `Boolean`, and any attempted equality conversion between these kinds refuse `ill_typed`/`type-mismatch` before an equality charge and produce no Boolean. Direct kernel mixed-kind comparisons return the typed checked-invariant refusal and key comparison returns no key relation."
    },
    {
      "id": "assert_pair",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "fn assert_pair(accepted: (&str, &[u8]), refused: (&str, &[u8]), code: Code, cause: &str) {"
    },
    {
      "id": "matrix_case_files_have_executable_expectations",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "assert_eq!(actual, expected, \"every matrix source has an executable expectation\");"
    },
    {
      "id": "pending-admissions",
      "path": "fsl-conformance/cases/type-operators/pending/README.md",
      "role": "examined",
      "excerpt": "These original positive units are excluded from the active matrix and do not count as successful conformance."
    },
    {
      "id": "matrix_boolean_connectives",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_boolean_connectives: \"boolean\", \"boolean-connectives\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-146\";"
    },
    {
      "id": "matrix_boolean_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_boolean_equality: \"boolean\", \"boolean-equality\" => Code::IllTyped, \"operator-ineligible\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_integer_arithmetic",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_integer_arithmetic: \"integer\", \"integer-arithmetic\" => Code::UndefinedExpression, \"unproved-range\", \"FSpec-FR-146\";"
    },
    {
      "id": "matrix_integer_slash",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_integer_slash: \"integer\", \"integer-slash\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-146\";"
    },
    {
      "id": "matrix_integer_division",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_integer_division: \"integer\", \"integer-division\" => Code::UndefinedExpression, \"unproved-nonzero\", \"FSpec-FR-147\";"
    },
    {
      "id": "matrix_integer_modulo",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_integer_modulo: \"integer\", \"integer-modulo\" => Code::UndefinedExpression, \"unproved-nonzero\", \"FSpec-FR-147\";"
    },
    {
      "id": "matrix_integer_negation",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_integer_negation: \"integer\", \"integer-negation\" => Code::UndefinedExpression, \"unproved-range\", \"FSpec-FR-146\";"
    },
    {
      "id": "matrix_integer_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_integer_equality: \"integer\", \"integer-equality\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_integer_ordering",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_integer_ordering: \"integer\", \"integer-ordering\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_rational_arithmetic",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_rational_arithmetic: \"rational\", \"rational-arithmetic\" => Code::UndefinedExpression, \"unproved-range\", \"FSpec-FR-146\";"
    },
    {
      "id": "matrix_rational_division",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_rational_division: \"rational\", \"rational-division\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-146\";"
    },
    {
      "id": "matrix_rational_negation",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_rational_negation: \"rational\", \"rational-negation\" => Code::UndefinedExpression, \"unproved-range\", \"FSpec-FR-146\";"
    },
    {
      "id": "matrix_rational_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_rational_equality: \"rational\", \"rational-equality\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_rational_ordering",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_rational_ordering: \"rational\", \"rational-ordering\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_decimal_arithmetic",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_decimal_arithmetic: \"decimal\", \"decimal-arithmetic\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-140\";"
    },
    {
      "id": "matrix_decimal_division",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_decimal_division: \"decimal\", \"decimal-division\" => Code::UndefinedExpression, \"unproved-nonzero\", \"FSpec-FR-140\";"
    },
    {
      "id": "matrix_decimal_negation",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_decimal_negation: \"decimal\", \"decimal-negation\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-140\";"
    },
    {
      "id": "matrix_decimal_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_decimal_equality: \"decimal\", \"decimal-equality\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_decimal_ordering",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_decimal_ordering: \"decimal\", \"decimal-ordering\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-140\";"
    },
    {
      "id": "matrix_float_arithmetic",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_float_arithmetic: \"float\", \"float-arithmetic\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-148\";"
    },
    {
      "id": "matrix_float_division",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_float_division: \"float\", \"float-division\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-148\";"
    },
    {
      "id": "matrix_float_negation",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_float_negation: \"float\", \"float-negation\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-148\";"
    },
    {
      "id": "matrix_float_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_float_equality: \"float\", \"float-equality\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-148\";"
    },
    {
      "id": "matrix_float_ordering",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_float_ordering: \"float\", \"float-ordering\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-148\";"
    },
    {
      "id": "matrix_quantity_arithmetic",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_quantity_arithmetic: \"quantity\", \"quantity-arithmetic\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-142\";"
    },
    {
      "id": "matrix_quantity_division",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_quantity_division: \"quantity\", \"quantity-division\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-142\";"
    },
    {
      "id": "matrix_quantity_negation",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_quantity_negation: \"quantity\", \"quantity-negation\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-142\";"
    },
    {
      "id": "matrix_quantity_power",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_quantity_power: \"quantity\", \"quantity-power\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-142\";"
    },
    {
      "id": "matrix_quantity_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_quantity_equality: \"quantity\", \"quantity-equality\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-142\";"
    },
    {
      "id": "matrix_quantity_ordering",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_quantity_ordering: \"quantity\", \"quantity-ordering\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-142\";"
    },
    {
      "id": "matrix_text_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_text_equality: \"text\", \"text-equality\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_text_ordering",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_text_ordering: \"text\", \"text-ordering\" => Code::IllTyped, \"operator-ineligible\", \"FSpec-FR-141\";"
    },
    {
      "id": "matrix_text_size",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_text_size: \"text\", \"text-size\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-141\";"
    },
    {
      "id": "matrix_native_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_native_equality: \"native\", \"native-equality\" => Code::IllTyped, \"operator-ineligible\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_native_kind_separation",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_native_kind_separation: \"native\", \"native-kind-separation\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_enum_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_enum_equality: \"enum\", \"enum-equality\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-141\";"
    },
    {
      "id": "matrix_enum_ordering",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_enum_ordering: \"enum\", \"enum-ordering\" => Code::IllTyped, \"operator-ineligible\", \"FSpec-FR-141\";"
    },
    {
      "id": "matrix_option_presence",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_option_presence: \"option\", \"option-presence\" => Code::UndefinedExpression, \"unproved-presence\", \"FSpec-FR-146\";"
    },
    {
      "id": "matrix_option_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_option_equality: \"option\", \"option-equality\" => Code::IllTyped, \"operator-ineligible\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_composite_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_composite_equality: \"composite\", \"composite-equality\" => Code::IllTyped, \"operator-ineligible\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_collection_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_collection_equality: \"collection\", \"collection-equality\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_collection_size",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_collection_size: \"collection\", \"collection-size\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-145\";"
    },
    {
      "id": "matrix_collection_contains",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_collection_contains: \"collection\", \"collection-contains\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-145\";"
    },
    {
      "id": "matrix_reference_equality",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_reference_equality: \"reference\", \"reference-equality\" => Code::IllTyped, \"operator-ineligible\", \"FSpec-FR-149\";"
    },
    {
      "id": "matrix_quantity_scaling",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_quantity_scaling: \"quantity\", \"quantity-scaling\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-142\";"
    },
    {
      "id": "matrix_quantity_unit_resolution",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_quantity_unit_resolution: \"quantity\", \"quantity-unit-resolution\" => Code::MissingDeclaration, \"missing-name\", \"FSpec-FR-142\";"
    },
    {
      "id": "matrix_collection_arithmetic_refused",
      "path": "fsl-conformance/tests/it/type_decision_cases.rs",
      "role": "examined",
      "excerpt": "matrix_collection_arithmetic_refused: \"collection\", \"collection-arithmetic-refused\" => Code::IllTyped, \"type-mismatch\", \"FSpec-FR-145\";"
    }
  ],
  "findings": [],
  "bindings": [
    {
      "test_id": "matrix_boolean_connectives",
      "ac_id": "FSpec-FR-146",
      "trace": "correct"
    },
    {
      "test_id": "matrix_boolean_equality",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_integer_arithmetic",
      "ac_id": "FSpec-FR-146",
      "trace": "correct"
    },
    {
      "test_id": "matrix_integer_slash",
      "ac_id": "FSpec-FR-146",
      "trace": "correct"
    },
    {
      "test_id": "matrix_integer_division",
      "ac_id": "FSpec-FR-147",
      "trace": "correct"
    },
    {
      "test_id": "matrix_integer_modulo",
      "ac_id": "FSpec-FR-147",
      "trace": "correct"
    },
    {
      "test_id": "matrix_integer_negation",
      "ac_id": "FSpec-FR-146",
      "trace": "correct"
    },
    {
      "test_id": "matrix_integer_equality",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_integer_ordering",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_rational_arithmetic",
      "ac_id": "FSpec-FR-146",
      "trace": "correct"
    },
    {
      "test_id": "matrix_rational_division",
      "ac_id": "FSpec-FR-146",
      "trace": "correct"
    },
    {
      "test_id": "matrix_rational_negation",
      "ac_id": "FSpec-FR-146",
      "trace": "correct"
    },
    {
      "test_id": "matrix_rational_equality",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_rational_ordering",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_decimal_arithmetic",
      "ac_id": "FSpec-FR-140",
      "trace": "correct"
    },
    {
      "test_id": "matrix_decimal_division",
      "ac_id": "FSpec-FR-140",
      "trace": "correct"
    },
    {
      "test_id": "matrix_decimal_negation",
      "ac_id": "FSpec-FR-140",
      "trace": "correct"
    },
    {
      "test_id": "matrix_decimal_equality",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_decimal_ordering",
      "ac_id": "FSpec-FR-140",
      "trace": "correct"
    },
    {
      "test_id": "matrix_float_arithmetic",
      "ac_id": "FSpec-FR-148",
      "trace": "correct"
    },
    {
      "test_id": "matrix_float_division",
      "ac_id": "FSpec-FR-148",
      "trace": "correct"
    },
    {
      "test_id": "matrix_float_negation",
      "ac_id": "FSpec-FR-148",
      "trace": "correct"
    },
    {
      "test_id": "matrix_float_equality",
      "ac_id": "FSpec-FR-148",
      "trace": "correct"
    },
    {
      "test_id": "matrix_float_ordering",
      "ac_id": "FSpec-FR-148",
      "trace": "correct"
    },
    {
      "test_id": "matrix_quantity_arithmetic",
      "ac_id": "FSpec-FR-142",
      "trace": "correct"
    },
    {
      "test_id": "matrix_quantity_division",
      "ac_id": "FSpec-FR-142",
      "trace": "correct"
    },
    {
      "test_id": "matrix_quantity_negation",
      "ac_id": "FSpec-FR-142",
      "trace": "correct"
    },
    {
      "test_id": "matrix_quantity_power",
      "ac_id": "FSpec-FR-142",
      "trace": "correct"
    },
    {
      "test_id": "matrix_quantity_equality",
      "ac_id": "FSpec-FR-142",
      "trace": "correct"
    },
    {
      "test_id": "matrix_quantity_ordering",
      "ac_id": "FSpec-FR-142",
      "trace": "correct"
    },
    {
      "test_id": "matrix_text_equality",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_text_ordering",
      "ac_id": "FSpec-FR-141",
      "trace": "correct"
    },
    {
      "test_id": "matrix_text_size",
      "ac_id": "FSpec-FR-141",
      "trace": "correct"
    },
    {
      "test_id": "matrix_native_equality",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_native_kind_separation",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_enum_equality",
      "ac_id": "FSpec-FR-141",
      "trace": "correct"
    },
    {
      "test_id": "matrix_enum_ordering",
      "ac_id": "FSpec-FR-141",
      "trace": "correct"
    },
    {
      "test_id": "matrix_option_presence",
      "ac_id": "FSpec-FR-146",
      "trace": "correct"
    },
    {
      "test_id": "matrix_option_equality",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_composite_equality",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_collection_equality",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_collection_size",
      "ac_id": "FSpec-FR-145",
      "trace": "correct"
    },
    {
      "test_id": "matrix_collection_contains",
      "ac_id": "FSpec-FR-145",
      "trace": "correct"
    },
    {
      "test_id": "matrix_reference_equality",
      "ac_id": "FSpec-FR-149",
      "trace": "correct"
    },
    {
      "test_id": "matrix_quantity_scaling",
      "ac_id": "FSpec-FR-142",
      "trace": "correct"
    },
    {
      "test_id": "matrix_quantity_unit_resolution",
      "ac_id": "FSpec-FR-142",
      "trace": "correct"
    },
    {
      "test_id": "matrix_collection_arithmetic_refused",
      "ac_id": "FSpec-FR-145",
      "trace": "correct"
    }
  ]
}
```

