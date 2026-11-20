---
id: SR-1420
title: "Diff-scoped coverage review of parser source-form cases"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2107; PR 3111; fsl-cst/tests/it/production_source_forms.rs and main.rs"
review_set: subset
---

## Summary

Reviewed the two-file parser test diff at frozen head dd5ad1ac9ef81d7675b1b033b14402fb0d97dc03. No findings.

## Verdict

**PASS for this source-form parser slice.** QSL-2107 remains partial.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | implementation-bug-despite-evidence |

The required placeholder row is not a finding; its Escape Cause cell is schema filler and asserts no defect or escape.

## Coverage

Compared this source-form slice with the Cargo-pinned FSpec f32bca6 complete grammar and FR-339 AC-1/AC-2/AC-4. Positive and malformed neighbours exercise exact numeric/float/reference forms, collection/population/model queries, object members/operations/frames, state bodies and population bounds, dense/tick clocks and urgency, quantified clock bounds, activation/fairness/capture forms, temporal intervals and operator precedence. Concrete nodes, child roles/order, source bytes and locations make the cases fail on real parser errors; there is no table-name/count-only gate, stub, duplicate grammar implementation, or false passing oracle. Syntax cases do not claim type-checking/runtime admission. QSL-2107 remains partial: production families beyond this changed slice are not claimed covered. This is a diff-scoped manual semantic/traceability gap analysis under the dispatch; no repository-wide matrix or assurance-completion claim is made.

Accepted the coder-reported 20 passing source-form tests plus the corrected bound declaration test pass (21 tests across focused runs), scoped fsl-cst Clippy with warnings denied, and formatting pass. No reviewer tests, builds, matrix, prior core/grammar corpus, or gate reruns were performed. The lead owns one final applicable gate. Plan completion: not assessed.

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
      "id": "exact_number_alternatives_keep_signed_components_and_scales",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn exact_number_alternatives_keep_signed_components_and_scales() {"
    },
    {
      "id": "bit_exact_float_constructors_keep_every_digit_inside_their_width",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn bit_exact_float_constructors_keep_every_digit_inside_their_width() {"
    },
    {
      "id": "incomplete_numeric_constructor_arguments_refuse_at_the_unfilled_slot",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn incomplete_numeric_constructor_arguments_refuse_at_the_unfilled_slot() {"
    },
    {
      "id": "reference_and_escaped_text_literals_keep_original_utf8_byte_spans",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn reference_and_escaped_text_literals_keep_original_utf8_byte_spans() {"
    },
    {
      "id": "collection_query_binders_projections_and_fold_identities_keep_their_roles",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn collection_query_binders_projections_and_fold_identities_keep_their_roles() {"
    },
    {
      "id": "population_queries_preserve_population_names_filters_and_projection_ranges",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn population_queries_preserve_population_names_filters_and_projection_ranges() {"
    },
    {
      "id": "model_lookup_absence_modes_and_typed_queries_keep_argument_boundaries",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn model_lookup_absence_modes_and_typed_queries_keep_argument_boundaries() {"
    },
    {
      "id": "incomplete_query_binders_and_lookup_modes_refuse_at_the_required_slot",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn incomplete_query_binders_and_lookup_modes_refuse_at_the_required_slot() {"
    },
    {
      "id": "source_objects_keep_inheritance_identity_fields_and_population_members",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn source_objects_keep_inheritance_identity_fields_and_population_members() {"
    },
    {
      "id": "operation_members_keep_optional_results_redefinitions_and_each_frame_kind",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn operation_members_keep_optional_results_redefinitions_and_each_frame_kind() {"
    },
    {
      "id": "malformed_object_lists_and_frames_refuse_the_missing_member",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn malformed_object_lists_and_frames_refuse_the_missing_member() {"
    },
    {
      "id": "state_clause_and_operation_body_alternatives_keep_targets_and_predicates",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn state_clause_and_operation_body_alternatives_keep_targets_and_predicates() {"
    },
    {
      "id": "population_bounds_and_terminal_alternatives_keep_their_subjects",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn population_bounds_and_terminal_alternatives_keep_their_subjects() {"
    },
    {
      "id": "incomplete_state_targets_and_population_bounds_refuse_the_offending_token",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn incomplete_state_targets_and_population_bounds_refuse_the_offending_token() {"
    },
    {
      "id": "dense_and_tick_time_declarations_and_urgency_keep_their_distinct_members",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn dense_and_tick_time_declarations_and_urgency_keep_their_distinct_members() {"
    },
    {
      "id": "clock_bound_conjunctions_keep_ordinary_and_quantified_bounds_separate",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn clock_bound_conjunctions_keep_ordinary_and_quantified_bounds_separate() {"
    },
    {
      "id": "incomplete_time_and_clock_bound_forms_refuse_at_their_required_slot",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn incomplete_time_and_clock_bound_forms_refuse_at_their_required_slot() {"
    },
    {
      "id": "activation_options_and_repeated_fairness_and_captures_keep_their_scope",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn activation_options_and_repeated_fairness_and_captures_keep_their_scope() {"
    },
    {
      "id": "temporal_unary_operators_keep_step_and_timed_interval_alternatives",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn temporal_unary_operators_keep_step_and_timed_interval_alternatives() {"
    },
    {
      "id": "temporal_binary_and_boolean_operators_keep_their_operand_precedence",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn temporal_binary_and_boolean_operators_keep_their_operand_precedence() {"
    },
    {
      "id": "incomplete_temporal_headers_guards_captures_and_intervals_refuse_locally",
      "path": "fsl-cst/tests/it/production_source_forms.rs",
      "role": "examined",
      "excerpt": "fn incomplete_temporal_headers_guards_captures_and_intervals_refuse_locally() {"
    },
    {
      "id": "production_source_forms",
      "path": "fsl-cst/tests/it/main.rs",
      "role": "examined",
      "excerpt": "mod production_source_forms;"
    }
  ],
  "findings": [],
  "bindings": []
}
```
