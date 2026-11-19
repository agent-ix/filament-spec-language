---
id: SR-2026111919340478491681414
title: "Diff-scoped gap analysis of executable core parser cases"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2107; PR 3104; fsl-cst/tests/it/production_core.rs and main.rs"
review_set: subset
---

## Summary

Reviewed the two-file parser-test diff at frozen head 722d46eee41c7ff0ed81330b28f3603485b6dd58. No findings.

## Verdict

**PASS for this core parser slice.** QSL-2107 remains partial; later production families are not claimed covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Manually mapped this diff to the examined grammar criteria and executable test functions. The cases cover empty/header/profile boundaries, selection repetitions and UTF-8 offsets; declaration members, optional markers and malformed neighbours; type alternatives and nested container attachment; modes/profiles; function options/parameters; expression precedence, grouping, binding/branch boundaries, collection ordering, postfix receivers and located trailing-token refusals. Every claimed case invokes fsl_cst::parse and asserts production output or a refusal, so breaking recognition, attachment, precedence, spans or classifications can fail a test. This is partial coverage of FSpec-FR-339-AC-1/2/4/6/7 and FR-002-AC-5, not whole-criterion satisfaction: checking/packaging halves and later grammar families remain outside this slice. No new stub, name/count-only drift gate, ignored-only evidence or manufactured specification was found.

Accepted the coder-reported 15-test focused pass plus the corrected declaration test pass (16 tests total), scoped fsl-cst Clippy with warnings denied and formatting/whitespace pass. No reviewer tests, builds, gates, matrix or prior corpus reruns were performed. The owning grammar was read from the Cargo-pinned FSpec f32bca6 source. No applicable AssuranceProfile was identified. Rust review is included in code-review; spec review is inapplicable because no specification changed. Lead owns the final applicable gate.

Plan completion: not assessed.

## Examined Scope

```json
{
  "clean": true,
  "scope": [
    {
      "id": "FSpec-FR-339-AC-1",
      "path": "filament-specification/spec/functional/source-package/FR-339-define-complete-native-grammar.md",
      "role": "examined",
      "excerpt": "Every production in the base and complete-V1 facet grammar has positive, boundary and located-negative corpus vectors."
    },
    {
      "id": "FSpec-FR-339-AC-2",
      "path": "filament-specification/spec/functional/source-package/FR-339-define-complete-native-grammar.md",
      "role": "examined",
      "excerpt": "Parsing and lossless rendering reproduce accepted UTF-8 source bytes exactly, including comments and whitespace."
    },
    {
      "id": "FSpec-FR-339-AC-4",
      "path": "filament-specification/spec/functional/source-package/FR-339-define-complete-native-grammar.md",
      "role": "examined",
      "excerpt": "An undeclared keyword, operator or extension refuses at its exact source span without producing a checked declaration."
    },
    {
      "id": "FSpec-FR-339-AC-6",
      "path": "filament-specification/spec/functional/source-package/FR-339-define-complete-native-grammar.md",
      "role": "examined",
      "excerpt": "`language \"ix:native\" edition \"1-draft\";` alone, and the same header followed by `profile v = \"filament.value.complete/v1\";`, each parse with no `invalid_syntax` diagnostic and format unchanged; checking or compiling either refuses `missing_declaration`/`empty-package` at the `language` header, exit 20, with no checked package."
    },
    {
      "id": "FSpec-FR-339-AC-7",
      "path": "filament-specification/spec/functional/source-package/FR-339-define-complete-native-grammar.md",
      "role": "examined",
      "excerpt": "Beside `profile v = \"filament.value.complete/v1\";`, `function probe using v(): Boolean pure { true } garbage` refuses `unsupported_construct`/`declaration-form` naming `garbage` at its span, exit 21, with no checked package and no `invalid_syntax`; `function probe using v(): Boolean pure { true } }` refuses `invalid_syntax`/`unexpected-token` at the trailing `}`, exit 20."
    },
    {
      "id": "FR-002-AC-5",
      "path": "spec/functional/FR-002-parse-native-units.md",
      "role": "examined",
      "excerpt": "The CST node of a parenthesized expression spans from its `(` to its `)`, both included."
    },
    {
      "id": "unit_prelude_repetitions_preserve_selections_and_utf8_spans",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn unit_prelude_repetitions_preserve_selections_and_utf8_spans() {"
    },
    {
      "id": "incomplete_prelude_members_refuse_the_missing_member_position",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn incomplete_prelude_members_refuse_the_missing_member_position() {"
    },
    {
      "id": "value_declarations_keep_ordered_members_payloads_and_optional_markers",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn value_declarations_keep_ordered_members_payloads_and_optional_markers() {"
    },
    {
      "id": "nonempty_declaration_lists_and_member_separators_refuse_malformed_neighbours",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn nonempty_declaration_lists_and_member_separators_refuse_malformed_neighbours() {"
    },
    {
      "id": "type_reference_alternatives_retain_the_full_authored_type",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn type_reference_alternatives_retain_the_full_authored_type() {"
    },
    {
      "id": "nested_type_members_are_attached_to_their_own_container",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn nested_type_members_are_attached_to_their_own_container() {"
    },
    {
      "id": "rounding_and_text_profiles_are_exact_grammar_members",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn rounding_and_text_profiles_are_exact_grammar_members() {"
    },
    {
      "id": "malformed_type_arguments_refuse_at_the_unfilled_argument_or_separator",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn malformed_type_arguments_refuse_at_the_unfilled_argument_or_separator() {"
    },
    {
      "id": "function_options_and_parameter_repetitions_keep_their_own_nodes",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn function_options_and_parameter_repetitions_keep_their_own_nodes() {"
    },
    {
      "id": "expression_precedence_attaches_products_before_sums_and_implications_to_the_right",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn expression_precedence_attaches_products_before_sums_and_implications_to_the_right() {"
    },
    {
      "id": "powers_and_grouping_keep_their_authored_operand_spans",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn powers_and_grouping_keep_their_authored_operand_spans() {"
    },
    {
      "id": "let_if_and_case_members_preserve_binder_and_branch_boundaries",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn let_if_and_case_members_preserve_binder_and_branch_boundaries() {"
    },
    {
      "id": "collection_and_record_value_repetitions_preserve_element_order",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn collection_and_record_value_repetitions_preserve_element_order() {"
    },
    {
      "id": "postfix_calls_and_indices_stay_inside_their_receiver_chain",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn postfix_calls_and_indices_stay_inside_their_receiver_chain() {"
    },
    {
      "id": "malformed_expression_neighbours_refuse_the_offending_token",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn malformed_expression_neighbours_refuse_the_offending_token() {"
    },
    {
      "id": "trailing_words_and_punctuation_have_distinct_located_refusals",
      "path": "fsl-cst/tests/it/production_core.rs",
      "role": "examined",
      "excerpt": "fn trailing_words_and_punctuation_have_distinct_located_refusals() {"
    },
    {
      "id": "production-core-registration",
      "path": "fsl-cst/tests/it/main.rs",
      "role": "examined",
      "excerpt": "mod production_core;"
    }
  ],
  "findings": [],
  "bindings": []
}
```
