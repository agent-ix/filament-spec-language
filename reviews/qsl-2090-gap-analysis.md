---
id: SR-1500
title: "gap-analysis of QSL-2090 lowering and model intake"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2090; frozen 7ebffebe932cbf9a111b5156d1a912e5a9fe07e6; .agent/receipts/qsl-2090/scope.md, .agent/receipts/qsl-2090/validation.md, fsl-analyze/tests/it/main.rs, fsl-analyze/tests/it/source_model_boundaries.rs, tests/it/lowering_intake_boundaries.rs, tests/it/lowering_intake_boundaries/calls.rs, tests/it/lowering_intake_boundaries/lowering.rs, tests/it/lowering_intake_boundaries/profiles.rs, tests/it/lowering_intake_boundaries/replay_bindings.rs, tests/it/lowering_intake_boundaries/state_clauses.rs, tests/it/main.rs"
review_set: subset
---

## Summary

Ticket: QSL-2090. Reviewed the eleven-file frozen test-only diff and the criterion facets listed below. Twenty new active functions have trusted focused passing receipts; four strict QSL-2709/QSL-2714 cases are pending and are not passing coverage.

## Verdict

**PASS** — no defects found in the changed tests or their scoped trace claims. This is a diff review, not whole-repository assurance or completion of the historical ticket inventory. Review does not claim all facets of each compound criterion are tested.

## Assurance Context

The dispatch limits this review to the changed tests and necessary current contracts. No assurance-profile inventory, matrix regeneration, unrelated gate, or execution recheck was performed. Exact harness model id is unavailable; no model id is invented.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Changed files examined:

- `.agent/receipts/qsl-2090/scope.md`
- `.agent/receipts/qsl-2090/validation.md`
- `fsl-analyze/tests/it/main.rs`
- `fsl-analyze/tests/it/source_model_boundaries.rs`
- `tests/it/lowering_intake_boundaries.rs`
- `tests/it/lowering_intake_boundaries/calls.rs`
- `tests/it/lowering_intake_boundaries/lowering.rs`
- `tests/it/lowering_intake_boundaries/profiles.rs`
- `tests/it/lowering_intake_boundaries/replay_bindings.rs`
- `tests/it/lowering_intake_boundaries/state_clauses.rs`
- `tests/it/main.rs`

The lowering assertions observe actual producer bytes, then the independent public typed reader; resolved structural operand keys distinguish parameter order and nested unary operands without conflating node-held slots with document indexes. Literal occurrences, local and quantified binders, conversion identities and flat-map form have concrete expected outputs. Calls, profile source spans and replay parameter identity exercise real source compilation, public call-site location and evaluation. State clauses select real pre/post observations. The active model case derives and replays the real Counter lasso with independently expected values and a distinct terminal-stutter identity.

The four ignored desired oracles are explicitly excluded. QSL-2709 owns widening producer/reader agreement; QSL-2714 owns the bounded source-model internal failures. No root cause, unexecuted Short-item result or successful proof is fabricated. Broad historical FR families, untouched operator-table rows, full CLI/direct parity and unrelated profile/dependency paths are not claimed as newly complete by this slice.

No source/config/dependency/gate changes, vendored external files, semantic mocks, source-grep evidence, warning suppression or new compatibility layer were added. Existing fixture helpers are reused within this repository. Known lint limitations remain disclosed. Author execution receipts are accepted without repetition. The one final affected-component gate remains the lead's next stage.

## Examined units and bindings

The excerpts identify the exact examined facets; compound criteria retain unexamined facets outside this diff review. Pending bindings describe desired tests and contribute no passing evidence.

```yaml
{
  "clean": true,
  "scope": [
    {
      "id": "FR-093-AC-1",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "examined",
      "excerpt": "The literal node has three `expression` occurrences, ordinals 0 to 2 in source order."
    },
    {
      "id": "FR-093-AC-2",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "examined",
      "excerpt": "No node is built for a read of `a`, `b` or `y`: each is a `reference` to that parameter's node."
    },
    {
      "id": "FR-093-AC-3",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "examined",
      "excerpt": "For each row of the application table whose checked node the `Value` family builds in a function body"
    },
    {
      "id": "FR-093-AC-4",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "examined",
      "excerpt": "`function c1 using v(x: Int[0, 9]): Int[0, 10] pure { x }` builds no convert node: `c1`'s `body` binding references `x`'s parameter node."
    },
    {
      "id": "FR-093-AC-5",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "examined",
      "excerpt": "In `function q using v(s: Sequence<Int[0, 9]>[0, 5]): Boolean pure { forall(x in s: exists(y in s: x = y)) and exists(z in s: true) }`, `x`'s parameter node has level 1, `y`'s level 2 and `z`'s level 1."
    },
    {
      "id": "FR-100-AC-4",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "An argument naming `y`, `x` bound twice, and no argument each refuse `invalid_runtime_input` at stage `call`, with `details` `{\"parameter\": \"y\"}`, `{\"parameter\": \"x\"}` and `{\"parameter\": \"x\"}`."
    },
    {
      "id": "FR-100-AC-5",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "`function` `nope`, `module.seven`, `\"\"`, `seven.`, and `7x` each refuse `missing_declaration` at stage `call`, exit 20, no result document, with `details` `{\"function\": <that string>}`."
    },
    {
      "id": "FR-100-AC-6",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "`seven` with `call.accounting` `{\"work_units\": 0}` returns a result document whose outcome is `{\"kind\": \"incomplete\", \"limit\": \"work_units\"}`, exit 22."
    },
    {
      "id": "FR-100-AC-7",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "`fsl_replay::spine::run` called directly over each AC-1, AC-4, AC-5 and AC-6 input returns the same outcome category, value, code, reason, counter, and parameter name or position `command`'s result document or refusal envelope carries for it."
    },
    {
      "id": "FR-110-AC-1",
      "path": "spec/functional/FR-110-resolve-header-profile-selections-at-e3.md",
      "role": "examined",
      "excerpt": "A unit whose one header profile names `filament.value.complete/v1` compiles through spine `compile`. Its emitted package has empty `profile_selections`."
    },
    {
      "id": "FR-110-AC-2",
      "path": "spec/functional/FR-110-resolve-header-profile-selections-at-e3.md",
      "role": "examined",
      "excerpt": "The same unit with the profile identity `test:unknown-profile` refuses `unknown_profile`/`unsupported-selection` at the span of the identity literal, naming alias `v`, the selection and required role `root`, and produces no package."
    },
    {
      "id": "FR-110-AC-3",
      "path": "spec/functional/FR-110-resolve-header-profile-selections-at-e3.md",
      "role": "examined",
      "excerpt": "The `edition` row's identity (`ix:native`) refuses the same way."
    },
    {
      "id": "FR-098-AC-1",
      "path": "spec/functional/FR-098-execute-a-replay-request.md",
      "role": "examined",
      "excerpt": "A request whose byte provision carries its one source and, under their package identities, the domain packages the source selects recompiles from those bytes alone and replays."
    },
    {
      "id": "FR-098-AC-2",
      "path": "spec/functional/FR-098-execute-a-replay-request.md",
      "role": "examined",
      "excerpt": "The selection resolves by `QualifiedName` in the recompiled package"
    },
    {
      "id": "FR-121-AC-1",
      "path": "spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md",
      "role": "examined",
      "excerpt": "For a unit declaring one Boolean predicate `p(x: Int[0, 9]): Boolean`, `call_site` over `p` returns `parameters` holding exactly one pair, its `Identifier` equal to `x`, and its `WireNodeId` is the one `replay` accepts: a `ReplayRequestWire` keyed by that exact pair does not refuse `UnknownParameter` or `UnboundParameter`."
    },
    {
      "id": "FR-107-AC-2",
      "path": "spec/functional/FR-107-evaluate-state-clauses-at-s6a.md",
      "role": "examined",
      "excerpt": "a postcondition `pre(present(self.parent) implies deref(value(self.parent)).versionNumber = 1)` over the same invocation is `Completed(true)`."
    },
    {
      "id": "FR-104-AC-1",
      "path": "spec/functional/FR-104-check-state-clauses.md",
      "role": "examined",
      "excerpt": "`ParentOrder`'s reads are all `current`; in each equality of `VersionAdvances` the left read is `post` and the right `pre`, `result` is `Boolean`"
    },
    {
      "id": "FR-125-AC-1",
      "path": "spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md",
      "role": "examined",
      "excerpt": "Under infinite-trace it continues with the terminal stutter step repeating position 3, whose transition identity differs from every `inc` identity."
    },
    {
      "id": "FR-125-AC-2",
      "path": "spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md",
      "role": "examined",
      "excerpt": "Over that behaviour, `always[0,5] holds(c.value <= 3)` from position 0 evaluates `false` under event-position false-extension (positions 4 and 5 are past closure) and `true` under infinite-trace (the stutter repeats `value = 3`); `eventually holds(c.value = 3)` under infinite-trace evaluates `true`, and `always eventually holds(c.value = 0)` evaluates `false`."
    },
    {
      "id": "FR-126-AC-2",
      "path": "spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md",
      "role": "examined",
      "excerpt": "Over the `Counter` subject (FR-124-AC-1, no `terminal` declaration), `eventually[0,3] holds(c.value = 3)` under event-position false-extension `on each` returns `Holds{Exhaustive}`, and `eventually[0,1] holds(c.value = 3)` `on each` returns `Violated` with the prefix `0 -inc-> 1`."
    },
    {
      "id": "FR-126-AC-3",
      "path": "spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md",
      "role": "examined",
      "excerpt": "Under infinite-trace, `always eventually holds(c.value = 0)` returns `Violated` whose loop is the terminal stutter step at value 3, with the stutter marker set."
    }
  ],
  "findings": [],
  "bindings": [
    {
      "test_id": "lowering_intake_boundaries::lowering::a_conditional_shares_its_literal_but_keeps_three_source_occurrences",
      "ac_id": "FR-093-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::lowering::local_reads_reference_parameters_and_a_let_keeps_its_bound_slot",
      "ac_id": "FR-093-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::lowering::arithmetic_and_boolean_applications_keep_operator_family_and_operand_order",
      "ac_id": "FR-093-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::lowering::widening_a_parameter_needs_no_conversion_and_the_resulting_package_admits",
      "ac_id": "FR-093-AC-4",
      "trace": "correct",
      "evidence": "ignored-owner-pending"
    },
    {
      "test_id": "lowering_intake_boundaries::lowering::a_bounded_literal_lowers_to_numeric_narrow_over_its_literal",
      "ac_id": "FR-093-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::lowering::a_rational_conversion_and_flat_map_keep_their_distinct_operations",
      "ac_id": "FR-093-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::lowering::sibling_quantifiers_reset_their_level_after_a_nested_binder",
      "ac_id": "FR-093-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::calls::the_named_call_binds_by_parameter_identity_instead_of_request_position",
      "ac_id": "FR-100-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::calls::the_named_call_binds_by_parameter_identity_instead_of_request_position",
      "ac_id": "FR-100-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::calls::missing_duplicate_and_foreign_argument_names_keep_the_offending_parameter",
      "ac_id": "FR-100-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::calls::missing_duplicate_and_foreign_argument_names_keep_the_offending_parameter",
      "ac_id": "FR-100-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::calls::malformed_and_unknown_function_selections_are_typed_call_refusals",
      "ac_id": "FR-100-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::calls::malformed_and_unknown_function_selections_are_typed_call_refusals",
      "ac_id": "FR-100-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::calls::zero_evaluation_work_is_incomplete_while_the_same_call_with_work_completes",
      "ac_id": "FR-100-AC-6",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::calls::zero_evaluation_work_is_incomplete_while_the_same_call_with_work_completes",
      "ac_id": "FR-100-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::profiles::a_value_profile_resolves_without_inventing_a_clause_profile_selection",
      "ac_id": "FR-110-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::profiles::unknown_and_wrong_role_header_selections_refuse_at_the_exact_identity_literal",
      "ac_id": "FR-110-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::profiles::unknown_and_wrong_role_header_selections_refuse_at_the_exact_identity_literal",
      "ac_id": "FR-110-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::replay_bindings::located_parameter_ids_bind_the_actual_predicate_in_a_source_only_provision",
      "ac_id": "FR-121-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::replay_bindings::located_parameter_ids_bind_the_actual_predicate_in_a_source_only_provision",
      "ac_id": "FR-098-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::replay_bindings::located_parameter_ids_bind_the_actual_predicate_in_a_source_only_provision",
      "ac_id": "FR-098-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::replay_bindings::input_assignments_bind_by_node_identity_in_either_request_order",
      "ac_id": "FR-098-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::replay_bindings::boolean_input_assignments_require_boolean_values_instead_of_integer_aliases",
      "ac_id": "FR-098-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::replay_bindings::omitting_a_located_parameter_is_a_typed_unbound_refusal",
      "ac_id": "FR-121-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::replay_bindings::omitting_a_located_parameter_is_a_typed_unbound_refusal",
      "ac_id": "FR-098-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::replay_bindings::another_declared_functions_parameter_cannot_bind_the_selected_predicate",
      "ac_id": "FR-098-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::state_clauses::selecting_a_new_postcondition_reads_pre_and_post_from_their_distinct_snapshots",
      "ac_id": "FR-104-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::state_clauses::selecting_a_new_postcondition_reads_pre_and_post_from_their_distinct_snapshots",
      "ac_id": "FR-107-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "lowering_intake_boundaries::state_clauses::pre_selects_the_whole_guarded_parent_expression_before_a_changed_invocation",
      "ac_id": "FR-107-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "source_model_boundaries::source_atoms_distinguish_sufficient_and_short_bounded_each_horizons",
      "ac_id": "FR-126-AC-2",
      "trace": "correct",
      "evidence": "ignored-owner-pending"
    },
    {
      "test_id": "source_model_boundaries::a_bounded_state_predicate_is_false_past_the_terminal_closure",
      "ac_id": "FR-125-AC-2",
      "trace": "correct",
      "evidence": "ignored-owner-pending"
    },
    {
      "test_id": "source_model_boundaries::a_bounded_operator_reads_true_over_the_infinite_terminal_stutter",
      "ac_id": "FR-125-AC-2",
      "trace": "correct",
      "evidence": "ignored-owner-pending"
    },
    {
      "test_id": "source_model_boundaries::a_real_terminal_lasso_repeats_three_with_a_distinct_stutter_transition",
      "ac_id": "FR-125-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "source_model_boundaries::a_real_terminal_lasso_repeats_three_with_a_distinct_stutter_transition",
      "ac_id": "FR-125-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "source_model_boundaries::a_real_terminal_lasso_repeats_three_with_a_distinct_stutter_transition",
      "ac_id": "FR-126-AC-3",
      "trace": "correct"
    }
  ]
}
```

