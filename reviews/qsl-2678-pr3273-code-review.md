---
id: SR-202611210022442313144108500
title: "QSL-2678 unignore sweep code-review for PR 3273"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3273 changed test files and .agent/receipts, QSL-2678"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Frozen head: `9b4f776c61f79f38f2c99b41bb9bb0b41407cba0`. Reviewed merge base `a731dbb5ce53111cc3b6142c6d74347f8a85597c` through HEAD, as `origin/main...HEAD`; current origin/main contains later work and is not used as the old-file source. Ticket: QSL-2678 (Done; retained unchanged with existing has-review label). PR 3273 is MERGEABLE, mergeStateStatus BLOCKED at inspection. No builds, Cargo invocations or tests were run by this reviewer under the occupied-lane restriction. `git diff --check origin/main...HEAD` passed.

Attribute-aware old/new function audit confirms 90 enables, 49 reason edits, 5 test deletions and 24 receipt deletions across 72 changed files. All 90 enabled names occur as `... ok` in the inherited per-crate seq logs; all 49 retained ignores occur as FAILED. These logs predate rebase and are not final-head execution evidence. The supplied manifest requests 90 enabled tests plus three ignored-reason gate tests (93 expected), which still must run through the authorized test lane before merge.

The 157 selected checklist names reconcile exactly: 144 changed tests, six intentionally unchanged split stubs (four W83c conditioned-key/certificate stubs and two Shifted-row stubs), and seven already enabled at the merge base (two plan tests and five decimal tests). Notes remain only on origin/draco/unignore-sweep-notes and are absent from the PR. No assertion or production behavior was changed. Five deletions match the explicit D ruling: QSL-2699 x1, QSL-2703 x3 and QSL-2329 x1. The QSL-2329 tracker comment 6af5039c confirms no product defect and a corrected fixture/control in PR 2792. Guarded direct-assignment/reference tests and their shared assertion helpers remain. Only the now-dead World::try_with_mode wrapper and the deleted test's imports were removed; TIMED_CLOCK remains used.

Four reasons cite In Progress QSL-2158 for source-route probabilistic settlement and precisely retain runtime_invariant/established-invariant-broken as the observed failure; that is feature ownership, not a proven root cause. One cites Backlog QSL-2497 for duplicate hyperproperty variables. Forty-four say the historical feature ticket is completed and the current blocker has no assigned owner. This does not pretend a Done ticket is an active dependency, and no successor ownership is fabricated. The syntactic ignored-reason gate accepts a recognized ticket identifier, independent of tracker state; its implementation is unchanged. Emitter/reader code and architecture-owned shapes remain untouched.

## Verdict

**CONDITIONAL** — two low findings: rewritten failure reasons contradict the inherited per-name evidence. Correct the reasons or substantiate them with newer per-test evidence, and obtain final-head focused test counts before merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The rewritten ignore says this unit is refused missing_declaration at compile, but inherited seq-filament-spec-language.log reaches the wrapper-equality assertion and reports two distinct body references. Remove the unsupported compile-refusal claim and retain the observed wrapper mismatch, or supply newer per-test evidence for the changed failure. | tests/it/clock_derivation.rs:548 |
| FND-002 | low | The rewritten ignore says compilation refuses unexpected-token before replay starts, but inherited seq-filament-spec-language.log reaches replay and reports settlement inconclusive with disagreement cause verdicts (proved violation, replayed success), versus expected reproduced-without-witness. Describe that observed settlement mismatch, or supply newer per-test evidence for the claimed compile failure. | tests/it/replay_request_file.rs:323 |

## Coverage

Plan completion: not assessed

Reconciliation: no fresh quoin matrix or quire coverage scan was run, per dispatch's instruction to avoid an unnecessary full scan. The existing PR 3270 engine outputs were inspected only as historical context, not attributed to this head: quoin 0.36.8 / quire 0.52.0 (engine 0.62.2), 6093 rows (4659 tagged, 1161 untagged, 156 tagged-by-ignored-test, 104 method-without-symbol, 13 retired), all without run evidence. They describe a different branch and cannot establish current coverage. The bounded source audit above is not an engine-backed repository assurance PASS. Repository-wide reverse-gap and stub completeness are not claimed. No new production behavior, trace tag, stub, public API, dependency, wire shape or CI gate appears in this diff. Removed tests were already ignored; their deletion is explicitly authorized and does not create a new loss of default-run coverage. Existing unmet obligations are not declared satisfied.

Optional semantic-review fan-out was not run. Ordinary code/test-intent review and exact checklist/evidence reconciliation ran under the explicit review scope. No applicable AssuranceProfile document was found (`type: AssuranceProfile` scan under spec); ordinary Rust/testing conventions apply. Available installed tools resolve to /home/peter/.npm-global/bin/quoin 0.36.8 and /home/peter/.npm-global/bin/quire 0.52.0 (engine 0.62.2), newer than the historical release notice. No CLI failure or fallback binary was used.

## Examined scope

```yaml
scope:
- id: global_conformance_is_decided_over_every_behaviour
  path: fsl-analyze/tests/it/analyze_cli.rs
  role: examined
  excerpt: fn global_conformance_is_decided_over_every_behaviour() {
- id: time_forms_over_a_timed_model_check
  path: fsl-analyze/tests/it/digital_clocks.rs
  role: examined
  excerpt: fn time_forms_over_a_timed_model_check() {
- id: parallel_join_overflow_and_message_budget
  path: fsl-analyze/tests/it/protocol_composition.rs
  role: examined
  excerpt: fn parallel_join_overflow_and_message_budget() {
- id: a_bad_recording_identity_refuses_the_request
  path: fsl-analyze/tests/it/protocol_replay_mode.rs
  role: examined
  excerpt: fn a_bad_recording_identity_refuses_the_request() {
- id: a_check_refusal_names_the_supplied_file
  path: fsl-command/tests/it/check_source_identity.rs
  role: examined
  excerpt: fn a_check_refusal_names_the_supplied_file() {
- id: each_protocol_refusal_names_its_subject
  path: fsl-command/tests/it/protocol_refusal_names.rs
  role: examined
  excerpt: fn each_protocol_refusal_names_its_subject() {
- id: index_pair
  path: fsl-conformance/tests/it/type_decision_cases.rs
  role: examined
  excerpt: fn index_pair() {
- id: tuple_position_pair
  path: fsl-conformance/tests/it/type_decision_cases.rs
  role: examined
  excerpt: fn tuple_position_pair() {
- id: some_pair
  path: fsl-conformance/tests/it/type_decision_cases.rs
  role: examined
  excerpt: fn some_pair() {
- id: scale_pair
  path: fsl-conformance/tests/it/type_decision_cases.rs
  role: examined
  excerpt: fn scale_pair() {
- id: every_corpus_unit_formats_idempotently_with_its_tokens_and_tree
  path: fsl-cst/tests/it/complete_grammar.rs
  role: examined
  excerpt: fn every_corpus_unit_formats_idempotently_with_its_tokens_and_tree() {
- id: a_weight_zero_workload_keeps_every_step_inside_the_state_graph
  path: fsl-eval/src/simulation/model/probabilistic/tests.rs
  role: examined
  excerpt: fn a_weight_zero_workload_keeps_every_step_inside_the_state_graph() {
- id: a_float_field_is_refused_field_sort_by_name
  path: fsl-eval/tests/it/smt_query.rs
  role: examined
  excerpt: fn a_float_field_is_refused_field_sort_by_name() {
- id: an_emitted_await_selects_exactly_one_temporal_profile
  path: fsl-package/src/emit/tests/protocol_controls.rs
  role: examined
  excerpt: fn an_emitted_await_selects_exactly_one_temporal_profile() {
- id: an_await_and_a_fence_are_structural
  path: fsl-package/src/emit/tests/protocol_occurrence.rs
  role: examined
  excerpt: fn an_await_and_a_fence_are_structural() {
- id: nested_repeats_zero_and_missing_maximums_and_branches_outside_a_repeat
  path: fsl-package/src/emit/tests/protocol_occurrence.rs
  role: examined
  excerpt: fn nested_repeats_zero_and_missing_maximums_and_branches_outside_a_repeat()
    {
- id: w_holds_no_runtime_value_and_parity_holds_across_variants
  path: fsl-package/src/emit/tests/protocol_occurrence.rs
  role: examined
  excerpt: fn w_holds_no_runtime_value_and_parity_holds_across_variants() {
- id: requirements_are_exactly_the_subject_kinds_of_the_operator_table
  path: fsl-package/src/emit/tests/protocol_occurrence.rs
  role: examined
  excerpt: fn requirements_are_exactly_the_subject_kinds_of_the_operator_table() {
- id: the_causal_relation_equals_the_checked_clauses_edge_relation
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn the_causal_relation_equals_the_checked_clauses_edge_relation() {
- id: the_protocol_view_lists_the_nodes_the_checked_clause_rebuilds
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn the_protocol_view_lists_the_nodes_the_checked_clause_rebuilds() {
- id: an_await_after_a_compensation_is_anchored_at_the_forward_effect
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn an_await_after_a_compensation_is_anchored_at_the_forward_effect() {
- id: awaits_after_one_template_and_a_template_without_a_commit_check
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn awaits_after_one_template_and_a_template_without_a_commit_check() {
- id: a_control_held_twice_is_refused_not_listed_twice
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn a_control_held_twice_is_refused_not_listed_twice() {
- id: an_unknown_or_non_clause_node_refuses_without_a_partial_result
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn an_unknown_or_non_clause_node_refuses_without_a_partial_result() {
- id: the_node_views_return_the_map_successors_and_refuse_a_removed_member
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn the_node_views_return_the_map_successors_and_refuse_a_removed_member()
    {
- id: the_channel_and_queue_views_return_delivery_and_capacity
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn the_channel_and_queue_views_return_delivery_and_capacity() {
- id: the_locus_and_the_clock_binding_are_the_packages_entries
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn the_locus_and_the_clock_binding_are_the_packages_entries() {
- id: the_lock_views_return_the_selections_and_refuse_an_absent_one
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn the_lock_views_return_the_selections_and_refuse_an_absent_one() {
- id: the_package_views_take_the_admitted_package_and_return_node_ids
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn the_package_views_take_the_admitted_package_and_return_node_ids() {
- id: types_values_and_numbers_are_the_terms_the_emitter_built
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn types_values_and_numbers_are_the_terms_the_emitter_built() {
- id: the_attempts_maximum_equals_the_authored_count_at_the_boundaries
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn the_attempts_maximum_equals_the_authored_count_at_the_boundaries() {
- id: a_compensation_without_a_maximum_refuses_missing_member
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn a_compensation_without_a_maximum_refuses_missing_member() {
- id: a_node_that_is_no_compensation_refuses_naming_the_id
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn a_node_that_is_no_compensation_refuses_naming_the_id() {
- id: a_zero_or_non_integer_maximum_refuses_malformed_member
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn a_zero_or_non_integer_maximum_refuses_malformed_member() {
- id: the_await_anchor_is_the_kind_and_target_the_node_carries
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn the_await_anchor_is_the_kind_and_target_the_node_carries() {
- id: a_channels_ends_and_message_type_are_the_nodes_it_references
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn a_channels_ends_and_message_type_are_the_nodes_it_references() {
- id: a_choices_owner_and_visible_set_are_the_nodes_it_references
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn a_choices_owner_and_visible_set_are_the_nodes_it_references() {
- id: a_channels_ordering_and_fifo_key_are_the_nodes_it_references
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn a_channels_ordering_and_fifo_key_are_the_nodes_it_references() {
- id: a_queues_overflow_policy_is_what_the_checked_clause_holds
  path: fsl-package/src/emit/tests/protocol_view.rs
  role: examined
  excerpt: fn a_queues_overflow_policy_is_what_the_checked_clause_holds() {
- id: native_result_two_frame_refuses_top_level_separating_witness
  path: fsl-replay/src/spine/clause/tests/frame.rs
  role: examined
  excerpt: fn native_result_two_frame_refuses_top_level_separating_witness() {
- id: private_witness_and_codec_sources_keep_format_identity
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn private_witness_and_codec_sources_keep_format_identity() {
- id: native_result_two_reader_rejects_invalid_witness_scalar_and_path
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn native_result_two_reader_rejects_invalid_witness_scalar_and_path() {
- id: native_result_two_writes_decisive_closed_and_unavailable_reports
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn native_result_two_writes_decisive_closed_and_unavailable_reports() {
- id: native_result_two_member_inventory_refuses_independent_edits
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn native_result_two_member_inventory_refuses_independent_edits() {
- id: native_result_two_writer_refuses_inconsistent_basis_and_witness
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn native_result_two_writer_refuses_inconsistent_basis_and_witness() {
- id: native_result_two_reader_refuses_witness_basis_and_version_edits
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn native_result_two_reader_refuses_witness_basis_and_version_edits() {
- id: native_result_two_reader_keeps_deciding_element_beyond_128_levels
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn native_result_two_reader_keeps_deciding_element_beyond_128_levels()
    {
- id: native_result_two_retains_selection_refusal_record
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn native_result_two_retains_selection_refusal_record() {
- id: tc_740_a_refuted_forall_derives_its_record_and_visits_nothing_after_the_stop
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_740_a_refuted_forall_derives_its_record_and_visits_nothing_after_the_stop()
    {
- id: tc_740_a_float_ordering_clause_ends_the_path_at_closed_scope
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_740_a_float_ordering_clause_ends_the_path_at_closed_scope() {
- id: tc_740_a_float_ordering_under_not_ends_the_path_at_closed_scope
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_740_a_float_ordering_under_not_ends_the_path_at_closed_scope() {
- id: tc_740_a_float_ordering_under_let_ends_the_path_at_closed_scope
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_740_a_float_ordering_under_let_ends_the_path_at_closed_scope() {
- id: tc_740_a_float_ordering_as_a_walked_or_operand_ends_the_path_at_closed_scope
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_740_a_float_ordering_as_a_walked_or_operand_ends_the_path_at_closed_scope()
    {
- id: tc_740_a_float_ordering_as_a_walked_and_operand_ends_the_path_at_closed_scope
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_740_a_float_ordering_as_a_walked_and_operand_ends_the_path_at_closed_scope()
    {
- id: tc_740_a_float_ordering_as_an_implies_consequent_ends_the_path_at_closed_scope
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_740_a_float_ordering_as_an_implies_consequent_ends_the_path_at_closed_scope()
    {
- id: tc_740_a_nested_forall_records_the_outer_occurrence
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_740_a_nested_forall_records_the_outer_occurrence() {
- id: tc_740_computed_domains_record_the_source_position
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_740_computed_domains_record_the_source_position() {
- id: tc_740_no_boolean_derives_no_basis
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_740_no_boolean_derives_no_basis() {
- id: tc_741_clause_reports_carry_the_derived_basis_and_witness
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_741_clause_reports_carry_the_derived_basis_and_witness() {
- id: tc_743_a_matching_record_that_separates_reproduces
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_743_a_matching_record_that_separates_reproduces() {
- id: tc_743_a_differing_or_missing_record_settles_inconclusive
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_743_a_differing_or_missing_record_settles_inconclusive() {
- id: a_protocol_record_or_a_quantifierless_record_fails_the_element_step
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn a_protocol_record_or_a_quantifierless_record_fails_the_element_step()
    {
- id: tc_743_the_separation_check_names_its_failing_step
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_743_the_separation_check_names_its_failing_step() {
- id: tc_744_an_undefined_step_maps_to_undefined_evaluation
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_744_an_undefined_step_maps_to_undefined_evaluation() {
- id: tc_743_an_oversized_record_refuses_and_replay_is_deterministic
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_743_an_oversized_record_refuses_and_replay_is_deterministic() {
- id: tc_744_a_mismatch_holds_both_records_and_no_result_record
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_744_a_mismatch_holds_both_records_and_no_result_record() {
- id: tc_744_the_witness_cause_round_trips_and_reads_strictly
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_744_the_witness_cause_round_trips_and_reads_strictly() {
- id: tc_744_a_failed_separation_check_disagrees_naming_its_step
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_744_a_failed_separation_check_disagrees_naming_its_step() {
- id: a_100000_deep_deciding_element_round_trips_on_a_small_stack
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn a_100000_deep_deciding_element_round_trips_on_a_small_stack() {
- id: tc_744_every_deciding_element_kind_round_trips
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_744_every_deciding_element_kind_round_trips() {
- id: tc_744_member_option_value_and_field_steps_round_trip
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_744_member_option_value_and_field_steps_round_trip() {
- id: tc_744_a_refusal_reads_only_catalog_spellings
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_744_a_refusal_reads_only_catalog_spellings() {
- id: tc_743_long_text_or_object_identity_exceeds_the_reader_bound
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_743_long_text_or_object_identity_exceeds_the_reader_bound() {
- id: a_state_record_round_trips_unchanged_and_a_protocol_record_omits_quantifier_and_index
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn a_state_record_round_trips_unchanged_and_a_protocol_record_omits_quantifier_and_index()
    {
- id: tc_1040_the_elements_of_a_built_list_are_read_at_their_index
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: fn tc_1040_the_elements_of_a_built_list_are_read_at_their_index() {
- id: unbounded_and_ordered_ends_navigate_to_their_collection_types
  path: fsl-semantics/tests/it/checker_rulings.rs
  role: examined
  excerpt: fn unbounded_and_ordered_ends_navigate_to_their_collection_types() {
- id: a_multiplicity_less_end_refuses_as_operator_ineligible
  path: fsl-semantics/tests/it/checker_rulings.rs
  role: examined
  excerpt: fn a_multiplicity_less_end_refuses_as_operator_ineligible() {
- id: reaches_admits_every_destination_multiplicity
  path: fsl-semantics/tests/it/checker_rulings.rs
  role: examined
  excerpt: fn reaches_admits_every_destination_multiplicity() {
- id: null_outside_a_record_field_is_ill_typed_at_check
  path: fsl-semantics/tests/it/null_value.rs
  role: examined
  excerpt: fn null_outside_a_record_field_is_ill_typed_at_check() {
- id: a_null_field_and_the_empty_option_spellings_still_check
  path: fsl-semantics/tests/it/null_value.rs
  role: examined
  excerpt: fn a_null_field_and_the_empty_option_spellings_still_check() {
- id: the_program_package_records_its_library_clock_selections
  path: tests/it/clock_derivation.rs
  role: examined
  excerpt: fn the_program_package_records_its_library_clock_selections() {
- id: a_recovered_unit_refuses_at_s1_with_both_diagnostics
  path: tests/it/parse_operation.rs
  role: examined
  excerpt: fn a_recovered_unit_refuses_at_s1_with_both_diagnostics() {
- id: case_binders_print_under_their_declared_names
  path: tests/it/request_surface_s1_16.rs
  role: examined
  excerpt: fn case_binders_print_under_their_declared_names() {
- id: clocks_member_compiles_event_position_fixed_sample_and_timestamped_clauses
  path: tests/it/run_wire.rs
  role: examined
  excerpt: fn clocks_member_compiles_event_position_fixed_sample_and_timestamped_clauses()
    {
- id: clock_configuration_changes_package_identity_and_member_order_does_not
  path: tests/it/run_wire.rs
  role: examined
  excerpt: fn clock_configuration_changes_package_identity_and_member_order_does_not()
    {
- id: missing_unused_and_mismatched_clock_selections_refuse_naming_the_clock
  path: tests/it/run_wire.rs
  role: examined
  excerpt: fn missing_unused_and_mismatched_clock_selections_refuse_naming_the_clock()
    {
- id: a_timed_clause_over_untimed_samples_refuses_with_profile_and_kinds
  path: tests/it/run_wire.rs
  role: examined
  excerpt: fn a_timed_clause_over_untimed_samples_refuses_with_profile_and_kinds()
    {
- id: malformed_and_reserved_clock_members_refuse
  path: tests/it/run_wire.rs
  role: examined
  excerpt: fn malformed_and_reserved_clock_members_refuse() {
- id: clocks_rebuilt_from_package_rows_compile_the_same_package
  path: tests/it/run_wire.rs
  role: examined
  excerpt: fn clocks_rebuilt_from_package_rows_compile_the_same_package() {
- id: a_timed_trace_answers_a_timed_clause
  path: tests/it/run_wire.rs
  role: examined
  excerpt: fn a_timed_trace_answers_a_timed_clause() {
- id: an_image_outside_an_abstract_unit_bound_is_initial_not_abstract
  path: fsl-analyze/tests/it/analyze_family_cli.rs
  role: examined
  excerpt: '#[ignore = "QSL-2407 completed; current blocker has no assigned owner:
    the analyze item for an image outside an abstract unit bound carries no terminal
    record (null) instead of settling InitialNotAbstract"]'
- id: the_first_refuted_instance_is_reported_over_a_shorter_later_one
  path: fsl-analyze/tests/it/counterexample_instance.rs
  role: examined
  excerpt: '#[ignore = "QSL-2013 completed; current blocker has no assigned owner:
    the counterexample reports instance `b` where the test expects the first refuted
    instance `a`"]'
- id: a_weight_zero_operation_takes_its_turn_last_in_a_tie
  path: fsl-analyze/tests/it/digital_clocks.rs
  role: examined
  excerpt: '#[ignore = "QSL-1991 completed; current blocker has no assigned owner:
    exact-probabilistic-digital still fails with an internal fault (invariant workload-weights-sum)
    on a weight-0 tie"]'
- id: an_object_free_hyper_wrapper_ranges_over_the_model_root
  path: fsl-analyze/tests/it/hyper_object_free.rs
  role: examined
  excerpt: '#[ignore = "QSL-2464 completed; current blocker has no assigned owner:
    the test''s hyper source does not parse at S1 (invalid_syntax: expected identifier)"]'
- id: a_model_root_over_on_a_wrapper_that_reads_a_slot_is_refused
  path: fsl-analyze/tests/it/hyper_object_free.rs
  role: examined
  excerpt: '#[ignore = "QSL-2464 completed; current blocker has no assigned owner:
    the checked package has no hyperproperty node for this source"]'
- id: probabilistic_on_each_settles_per_instance
  path: fsl-analyze/tests/it/probability_from_source.rs
  role: examined
  excerpt: '#[ignore = "QSL-2158: source-route on-each probabilistic claims settle
    internal-failure (runtime_invariant, established-invariant-broken) before the
    expected per-instance verdict"]'
- id: probabilistic_on_each_empty_population
  path: fsl-analyze/tests/it/probability_from_source.rs
  role: examined
  excerpt: '#[ignore = "QSL-2158: source-route on-each probabilistic claims settle
    internal-failure (runtime_invariant, established-invariant-broken) before the
    expected per-instance verdict"]'
- id: probabilistic_on_each_never_activating_instance_is_vacuous
  path: fsl-analyze/tests/it/probability_from_source.rs
  role: examined
  excerpt: '#[ignore = "QSL-2158: source-route on-each probabilistic claims settle
    internal-failure (runtime_invariant, established-invariant-broken) before the
    expected per-instance verdict"]'
- id: probabilistic_on_each_names_first_failing_instance
  path: fsl-analyze/tests/it/probability_from_source.rs
  role: examined
  excerpt: '#[ignore = "QSL-2158: source-route on-each probabilistic claims settle
    internal-failure (runtime_invariant, established-invariant-broken) before the
    expected per-instance verdict"]'
- id: a_symbolic_capacity_settles_from_the_capacities_binding
  path: fsl-analyze/tests/it/protocol_analysis_differential.rs
  role: examined
  excerpt: '#[ignore = "QSL-2212 completed; current blocker has no assigned owner:
    the Flow claim is refused invalid_runtime_input/missing-member `temporal.profile`
    before the capacities binding is read"]'
- id: a_pair_on_a_causal_cycle_is_misordered
  path: fsl-analyze/tests/it/protocol_replay_mode.rs
  role: examined
  excerpt: '#[ignore = "QSL-2210 completed; current blocker has no assigned owner:
    the recording is refused invalid_runtime_input because its empty closure premises
    omit Flow::Done and Flow::Main::Decide::{no,yes}, before assessment"]'
- id: array_order_alone_never_misorders_a_recording
  path: fsl-analyze/tests/it/protocol_replay_mode.rs
  role: examined
  excerpt: '#[ignore = "QSL-2210 completed; current blocker has no assigned owner:
    the recording is refused invalid_runtime_input because its empty closure premises
    omit Flow::Done and Flow::Main::Decide::{no,yes}, before assessment"]'
- id: an_unexpected_send_violates_its_enabledness_obligation
  path: fsl-analyze/tests/it/protocol_replay_mode.rs
  role: examined
  excerpt: '#[ignore = "QSL-2210 completed; current blocker has no assigned owner:
    the recording is refused invalid_runtime_input because its empty closure premises
    omit Flow::Done and Flow::Main::Decide::{no,yes}, before assessment"]'
- id: a_late_compensation_attempt_violates_its_window_and_an_unread_one_is_pending
  path: fsl-analyze/tests/it/protocol_replay_mode.rs
  role: examined
  excerpt: '#[ignore = "QSL-2210 completed; current blocker has no assigned owner:
    the replay-mode item yields no verdict per obligation"]'
- id: a_recording_is_bound_by_its_stated_or_declared_identity
  path: fsl-analyze/tests/it/protocol_replay_mode.rs
  role: examined
  excerpt: '#[ignore = "QSL-2210 completed; current blocker has no assigned owner:
    the recording is refused invalid_runtime_input because its empty closure premises
    omit Flow::Done and Flow::Main::Decide::{no,yes}, before assessment"]'
- id: state_graph_identity_readback_uses_the_explicit_temporal_selection
  path: fsl-analyze/tests/it/state_graph_settle.rs
  role: examined
  excerpt: '#[ignore = "QSL-2563 completed; current blocker has no assigned owner:
    with the explicit clock selection, whole-package read-back now refuses the temporal
    clock selection as having no configured declaration instead of reaching the temporal
    formula reader"]'
- id: a_state_graph_certificate_over_a_zeno_only_graph_needs_no_divergence_witness
  path: fsl-analyze/tests/it/timed_state_graph.rs
  role: examined
  excerpt: '#[ignore = "QSL-2305 completed; current blocker has no assigned owner:
    the Replied claim is refused invalid_capability/inconsistent-candidates (contract-checker)
    instead of settling with a zone certificate"]'
- id: a_float_field_model_explores_by_its_ieee_bits
  path: fsl-eval/tests/it/model_simulation.rs
  role: examined
  excerpt: '#[ignore = "QSL-2176 completed; current blocker has no assigned owner:
    the explored float model yields no IEEE-bits state where the test expects [13830554455654793216]"]'
- id: a_protocol_reference_in_a_body_is_written
  path: fsl-package/src/emit/tests/protocol_controls.rs
  role: examined
  excerpt: '#[ignore = "QSL-2219 completed; current blocker has no assigned owner:
    the check still refuses a protocol reference in a protocol body with ProtocolReference(NotEmitted
    { construct: ProtocolReferenceInBody }) instead of writing it"]'
- id: a_self_referring_protocol_is_written_as_one_recursion_group
  path: fsl-package/src/emit/tests/protocol_controls.rs
  role: examined
  excerpt: '#[ignore = "QSL-2219 completed; current blocker has no assigned owner:
    the check still refuses a self-referring protocol with ProtocolReference(NotEmitted
    { construct: ProtocolReferenceInBody }) instead of writing a recursion group"]'
- id: an_ordered_end_navigates_in_ascending_position
  path: fsl-replay/src/spine/clause/tests/ordered_end_positions.rs
  role: examined
  excerpt: '#[ignore = "QSL-2179 completed; current blocker has no assigned owner:
    snapshot admission refuses the ordered-end `positions` member as invalid_runtime_input/unknown-member,
    so ordered navigation is not reached"]'
- id: malformed_positions_refuse_at_admission
  path: fsl-replay/src/spine/clause/tests/ordered_end_positions.rs
  role: examined
  excerpt: '#[ignore = "QSL-2179 completed; current blocker has no assigned owner:
    snapshot admission refuses the ordered-end `positions` member as unknown-member
    before the malformed-position refusals can be observed"]'
- id: a_violated_item_carries_a_witness_exactly_when_the_report_holds_one
  path: fsl-replay/src/spine/clause/tests/temporal.rs
  role: examined
  excerpt: '#[ignore = "QSL-2173 completed; current blocker has no assigned owner:
    a pending Reaches item still carries a witness where the test asserts none"]'
- id: snapshot_admission_refusal_witnesses_name_object_member_and_path
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: '#[ignore = "QSL-2173 completed; current blocker has no assigned owner:
    the missing-member snapshot refusal for the ConfigVersion fixture is not produced
    as the test expects; admission refuses with invalid_runtime_input/missing-member
    at a different member and path"]'
- id: tc_740_a_satisfied_exists_or_negated_forall_derives_a_decisive_witness
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: '#[ignore = "QSL-2173 completed; current blocker has no assigned owner:
    the float-ordering ConfigVersion clause derives a DecisiveCounterexample record
    where the test expects a ClosedScope derivation with no record"]'
- id: tc_740_the_decision_path_reaches_the_decisive_quantifier
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: '#[ignore = "QSL-2173 completed; current blocker has no assigned owner:
    the ConfigVersion clause derives a DecisiveCounterexample record where the test
    expects a ClosedScope derivation with no record"]'
- id: tc_744_a_declared_unit_quantity_round_trips
  path: fsl-replay/src/spine/clause/tests/witness.rs
  role: examined
  excerpt: '#[ignore = "QSL-2173 completed; current blocker has no assigned owner:
    the unit-table fixture package checks as an EmptyPackage refusal, so no unit quantity
    is built"]'
- id: a_replay_recompiles_a_library_that_uses_its_own_clock
  path: fsl-replay/tests/imported_function_replay.rs
  role: examined
  excerpt: '#[ignore = "QSL-2274 completed; current blocker has no assigned owner:
    replay still refuses a library whose bounded temporal rule `Late` has no selected
    clock configuration"]'
- id: a_value_type_bound_to_clock_by_target_types_a_clock_place
  path: fsl-semantics/tests/it/clock_places.rs
  role: examined
  excerpt: '#[ignore = "QSL-2176 completed; current blocker has no assigned owner:
    model intake refuses the by-target Clock alias (InvalidModelBinding, malformed
    `target` member of the type definition) before the check"]'
- id: a_value_type_bound_to_a_data_native_by_target_is_that_native
  path: fsl-semantics/tests/it/model_intake.rs
  role: examined
  excerpt: '#[ignore = "QSL-2176 completed; current blocker has no assigned owner:
    model intake refuses the by-target data-native alias (InvalidModelBinding, malformed
    `target` member of the type definition) before the check"]'
- id: a_protocol_slot_that_reads_model_state_names_a_population
  path: fsl-semantics/tests/it/named_population.rs
  role: examined
  excerpt: '#[ignore = "QSL-2216 completed; current blocker has no assigned owner:
    the test''s protocol source does not parse at S1 (invalid_syntax: expected end
    of source at the protocol slot)"]'
- id: a_profile_less_unit_refuses_another_layer_s_form_as_the_value_layer_does
  path: fsl-semantics/tests/it/profile_defaults.rs
  role: examined
  excerpt: '#[ignore = "QSL-2161 completed; current blocker has no assigned owner:
    the profile-less unit is refused with a Temporal TimedFacetRequired (Clock) cause,
    not the layer refusal the value layer gives"]'
- id: some_takes_the_expected_bounded_element_type
  path: fsl-semantics/tests/it/type_decisions.rs
  role: examined
  excerpt: '#[ignore = "QSL-2169 completed; current blocker has no assigned owner:
    `Some(3) = y` over Option<Int[0, 9]> is still refused IllTyped(TypeMismatch)"]'
- id: a_quantity_scaled_by_a_number_keeps_its_unit
  path: fsl-semantics/tests/it/type_decisions.rs
  role: examined
  excerpt: '#[ignore = "QSL-2168 completed; current blocker has no assigned owner:
    dividing a Quantity by an Integer is still refused with OperandTypeMismatch (Divide),
    so the scaling does not check"]'
- id: an_unproved_or_ineligible_scaling_is_refused
  path: fsl-semantics/tests/it/type_decisions.rs
  role: examined
  excerpt: '#[ignore = "QSL-2168 completed; current blocker has no assigned owner:
    dividing a Quantity by Int[0, 9] is refused IllTyped (OperandTypeMismatch) where
    the test expects UndefinedExpression"]'
- id: an_unknown_member_of_a_source_enum_refuses_type_mismatch_like_a_model_variant_case
  path: fsl-semantics/tests/it/variant_fields.rs
  role: examined
  excerpt: '#[ignore = "QSL-2170 completed; current blocker has no assigned owner:
    an unknown qualified member `Status::Pending` is still refused MissingQualifiedName,
    not type-mismatch"]'
- id: a_missing_model_names_the_identity_and_the_inputs_that_supply_it
  path: tests/it/check_imports.rs
  role: examined
  excerpt: '#[ignore = "QSL-1972 completed; current blocker has no assigned owner:
    the missing_import refusal does not yet name --model and the models member (fails
    at the run-result assertion)"]'
- id: reversed_prefix_hyper_clauses_share_one_body_wrapper
  path: tests/it/clock_derivation.rs
  role: examined
  excerpt: '#[ignore = "QSL-2400 completed; current blocker has no assigned owner:
    the unit is refused missing_declaration at compile; the slots of reversed prefixes
    also sit at different levels, so the two clauses get two body wrappers, not one"]'
- id: a_finite_domain_hyper_clause_emits_one_row_per_trace_on_its_domain_binder
  path: tests/it/clock_derivation.rs
  role: examined
  excerpt: '#[ignore = "QSL-2400 completed; current blocker has no assigned owner:
    the finite-domain hyper unit is refused invalid_syntax (unexpected-token) at compile,
    so no clock row per trace on role `t` is emitted"]'
- id: a_repeated_quantifier_variable_is_refused_at_the_hyperproperty_shape
  path: tests/it/clock_derivation.rs
  role: examined
  excerpt: '#[ignore = "QSL-2497: a repeated quantifier variable is not refused at
    the hyperproperty Shape step yet"]'
- id: a_float_model_field_exports_its_exact_bits
  path: tests/it/export_rulings.rs
  role: examined
  excerpt: '#[ignore = "QSL-2176 completed; current blocker has no assigned owner:
    a float model field compiles to a package that is refused invalid_package/operator-ineligible
    at result_type, so it cannot export yet"]'
- id: a_replay_request_file_replays_a_library_clock_from_the_package
  path: tests/it/replay_request_file.rs
  role: examined
  excerpt: '#[ignore = "QSL-2274 completed; current blocker has no assigned owner:
    the library-clock program is refused invalid_syntax (unexpected-token) at compile,
    so the package replay cannot start"]'
- id: check_markdown_names_each_fence_unit
  path: tests/it/request_surface_s1.rs
  role: examined
  excerpt: '#[ignore = "QSL-2019 completed; current blocker has no assigned owner:
    the check command reports no per-fence result for a Markdown document (check outcome
    is null instead of check)"]'
- id: check_markdown_refuses_doubled_and_headless_fences
  path: tests/it/request_surface_s1.rs
  role: examined
  excerpt: '#[ignore = "QSL-2019 completed; current blocker has no assigned owner:
    check rejects the Markdown operand as an invalid request instead of refusing doubled
    and headless fences"]'
- id: check_without_markdown_reads_native_source
  path: tests/it/request_surface_s1.rs
  role: examined
  excerpt: '#[ignore = "QSL-2019 completed; current blocker has no assigned owner:
    check of a non-native document does not refuse invalid_syntax on its native reading"]'
- id: a_bounded_quantity_refuses_out_of_range_bounds
  path: tests/it/run_value_spellings.rs
  role: examined
  excerpt: '#[ignore = "QSL-2171 completed; current blocker has no assigned owner:
    S1 does not parse Quantity<u>[min, max] (the run refuses with unexpected-token)"]'
- id: a_selected_package_with_a_population_collection_type_refuses_unsupported
  path: tests/it/run_value_spellings.rs
  role: examined
  excerpt: '#[ignore = "QSL-2182 completed; current blocker has no assigned owner:
    the fixture tests/fixtures/population-collection.checked-package.json is missing;
    canonicalization fails before any package request runs"]'
- id: a_run_resolves_its_function_in_a_selected_package
  path: tests/it/run_value_spellings.rs
  role: examined
  excerpt: '#[ignore = "QSL-2182 completed; current blocker has no assigned owner:
    the fixture tests/fixtures/population-collection.checked-package.json is missing;
    canonicalization fails before any package request runs"]'
- id: a_compile_request_over_its_ceiling_names_the_option
  path: tests/it/verb_limits.rs
  role: examined
  excerpt: '#[ignore = "QSL-2190 completed; current blocker has no assigned owner:
    compile exhausts at status 22 (resource_exhausted) instead of the request_bytes
    refusal naming --request-bytes (status 20)"]'
- id: a_kernel_world_applies_kernel_to_function_bodies_only
  path: fsl-analyze/tests/it/check_mode_record.rs
  role: examined
  excerpt: fn a_kernel_world_applies_kernel_to_function_bodies_only() {
- id: direct_partial_assignment_preserves_all_unsolved_values_and_effects
  path: fsl-eval/src/simulation/model/frames/narrowing_tests.rs
  role: examined
  excerpt: fn direct_partial_assignment_preserves_all_unsolved_values_and_effects()
    {
- id: direct_assignment_and_its_reference_preserve_full_canonical_exploration
  path: fsl-eval/src/simulation/model/frames/narrowing_tests.rs
  role: examined
  excerpt: fn direct_assignment_and_its_reference_preserve_full_canonical_exploration()
    {
- id: an_out_of_domain_direct_assignment_has_no_successor_or_finding
  path: fsl-eval/src/simulation/model/frames/narrowing_tests.rs
  role: examined
  excerpt: fn an_out_of_domain_direct_assignment_has_no_successor_or_finding() {
- id: a_delay_beside_an_untimed_source_clock_is_refused_as_untimed
  path: fsl-package/tests/source_probabilistic_emission.rs
  role: examined
  excerpt: fn a_delay_beside_an_untimed_source_clock_is_refused_as_untimed() {
- id: .agent/receipts/qsl-2084/final-gate.md
  path: .agent/receipts/qsl-2084/final-gate.md
  role: examined
  excerpt: '# QSL-2084 final affected-runtime gate'
- id: .agent/receipts/qsl-2084/scope.md
  path: .agent/receipts/qsl-2084/scope.md
  role: examined
  excerpt: '# QSL-2084 evaluator closure cases'
- id: .agent/receipts/qsl-2084/validation.md
  path: .agent/receipts/qsl-2084/validation.md
  role: examined
  excerpt: '# QSL-2084 authoring validation'
- id: .agent/receipts/qsl-2087/final-gate.md
  path: .agent/receipts/qsl-2087/final-gate.md
  role: examined
  excerpt: '# Final affected-component gate'
- id: .agent/receipts/qsl-2087/scope.md
  path: .agent/receipts/qsl-2087/scope.md
  role: examined
  excerpt: '# Certificate and witness behavior cases'
- id: .agent/receipts/qsl-2087/validation.md
  path: .agent/receipts/qsl-2087/validation.md
  role: examined
  excerpt: '# Focused validation'
- id: .agent/receipts/qsl-2088/final-gate.md
  path: .agent/receipts/qsl-2088/final-gate.md
  role: examined
  excerpt: '# QSL-2088 final affected-component gate'
- id: .agent/receipts/qsl-2090/final-gate.md
  path: .agent/receipts/qsl-2090/final-gate.md
  role: examined
  excerpt: '# QSL-2090 final affected-component gate'
- id: .agent/receipts/qsl-2090/scope.md
  path: .agent/receipts/qsl-2090/scope.md
  role: examined
  excerpt: '# QSL-2090 scope'
- id: .agent/receipts/qsl-2090/validation.md
  path: .agent/receipts/qsl-2090/validation.md
  role: examined
  excerpt: '# QSL-2090 authoring validation'
- id: .agent/receipts/qsl-2091/final-gate.md
  path: .agent/receipts/qsl-2091/final-gate.md
  role: examined
  excerpt: '# QSL-2091 final component gate'
- id: .agent/receipts/qsl-2091/scope.md
  path: .agent/receipts/qsl-2091/scope.md
  role: examined
  excerpt: '# QSL-2091 delivered behavior'
- id: .agent/receipts/qsl-2091/validation.md
  path: .agent/receipts/qsl-2091/validation.md
  role: examined
  excerpt: '# QSL-2091 focused validation'
- id: .agent/receipts/qsl-2092/final-gate.md
  path: .agent/receipts/qsl-2092/final-gate.md
  role: examined
  excerpt: '# QSL-2092 final affected-component gate'
- id: .agent/receipts/qsl-2092/scope.md
  path: .agent/receipts/qsl-2092/scope.md
  role: examined
  excerpt: '# QSL-2092: protocol and timed replay boundaries'
- id: .agent/receipts/qsl-2092/validation.md
  path: .agent/receipts/qsl-2092/validation.md
  role: examined
  excerpt: '# QSL-2092 authoring validation'
- id: .agent/receipts/qsl-2093/final-gate.md
  path: .agent/receipts/qsl-2093/final-gate.md
  role: examined
  excerpt: '# QSL-2093 final component gate'
- id: .agent/receipts/qsl-2093/scope.md
  path: .agent/receipts/qsl-2093/scope.md
  role: examined
  excerpt: '# QSL-2093 lifecycle boundary slice'
- id: .agent/receipts/qsl-2093/validation.md
  path: .agent/receipts/qsl-2093/validation.md
  role: examined
  excerpt: '# QSL-2093 focused validation'
- id: .agent/receipts/qsl-2094/final-gate.md
  path: .agent/receipts/qsl-2094/final-gate.md
  role: examined
  excerpt: '# QSL-2094 final terminal-component gate'
- id: .agent/receipts/qsl-2094/scope.md
  path: .agent/receipts/qsl-2094/scope.md
  role: examined
  excerpt: '# QSL-2094 terminal emission boundaries'
- id: .agent/receipts/qsl-2094/validation.md
  path: .agent/receipts/qsl-2094/validation.md
  role: examined
  excerpt: '# QSL-2094 authoring validation'
- id: .agent/receipts/qsl-2095/final-gate.md
  path: .agent/receipts/qsl-2095/final-gate.md
  role: examined
  excerpt: '# QSL-2095 final integration gate'
- id: .agent/receipts/qsl-2096/final-gate.md
  path: .agent/receipts/qsl-2096/final-gate.md
  role: examined
  excerpt: '# QSL-2096 canonical-types gate receipt'
- id: World::try_with_mode
  path: fsl-analyze/tests/it/prob_world.rs
  role: examined
  excerpt: 'pub(crate) fn try_with_mode(model: &Model, mode: CheckMode) -> Result<Self,
    Vec<CheckRefusal>> {'
- id: FR-488-AC-26
  path: spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md
  role: context_only
  excerpt: Two hyper clauses with equal bodies, `Fwd` over `forall trace a of V forall
    trace b of V` and `Rev` over `forall trace b of V forall trace a of V`, emit two
    `hyperproperty` nodes whose `body` references one wrapper, and binding rows `a`,
    `b` at `Fwd`'s node and `b`, `a` at `Rev`'s node, none at the wrapper; the package
    is admitted.
- id: FR-862-AC-2
  path: spec/functional/FR-862-read-a-replay-request-file-and-replay-it.md
  role: context_only
  excerpt: Through `command::run`, the `native-replay/1` request carrying FR-640-AC-11's
    replay of the function `entry` (its package with `test/plant` as its one `dependencies`
    entry and its sources in `byte_provision` files) settles `reproduced-without-witness`
    with the value the in-process `fsl_replay::replay` gives, exit 0; the same request
    with the package's `lib-ms` `dependency_clock_selections` entry removed refuses
    `missing-clock-selection` as FR-640-AC-11 states.
- id: the_verification_plan_member_selects_one_plan_or_refuses
  path: tests/it/request_surface_s1_16.rs
  role: examined
  excerpt: fn the_verification_plan_member_selects_one_plan_or_refuses() {
- id: a_plan_leaves_the_package_identity_unchanged_and_emits_its_document
  path: tests/it/request_surface_s1_16.rs
  role: examined
  excerpt: fn a_plan_leaves_the_package_identity_unchanged_and_emits_its_document()
    {
- id: probabilistic_on_each_reports_conditional_certificate_and_instance
  path: fsl-analyze/tests/it/probability_from_source.rs
  role: examined
  excerpt: fn probabilistic_on_each_reports_conditional_certificate_and_instance()
    {
- id: an_image_outside_the_unit_bound_is_initial_not_abstract
  path: fsl-analyze/tests/it/analyze_family_cli.rs
  role: examined
  excerpt: fn an_image_outside_the_unit_bound_is_initial_not_abstract() {
- id: a_mapping_that_does_not_conform_to_the_abstract_field_is_refused
  path: fsl-analyze/tests/it/analyze_family_cli.rs
  role: examined
  excerpt: fn a_mapping_that_does_not_conform_to_the_abstract_field_is_refused() {
- id: an_on_each_key_carries_its_instance_and_phase
  path: fsl-replay/tests/it/conditioned_product_key.rs
  role: examined
  excerpt: fn an_on_each_key_carries_its_instance_and_phase() {
- id: a_six_member_key_is_refused
  path: fsl-replay/tests/it/conditioned_product_key.rs
  role: examined
  excerpt: fn a_six_member_key_is_refused() {
- id: decimal_members_have_the_golden_policy_types
  path: fsl-semantics/tests/it/decimal_value_types.rs
  role: examined
  excerpt: fn decimal_members_have_the_golden_policy_types() {
- id: bounds_on_every_node_of_the_walk_narrow
  path: fsl-semantics/tests/it/decimal_value_types.rs
  role: examined
  excerpt: fn bounds_on_every_node_of_the_walk_narrow() {
- id: decimal_members_check_in_clauses
  path: fsl-semantics/tests/it/decimal_value_types.rs
  role: examined
  excerpt: fn decimal_members_check_in_clauses() {
- id: snapshot_decimals_admit_by_membership
  path: fsl-semantics/tests/it/decimal_value_types.rs
  role: examined
  excerpt: fn snapshot_decimals_admit_by_membership() {
- id: decimal_policy_defects_refuse_once
  path: fsl-semantics/tests/it/decimal_value_types.rs
  role: examined
  excerpt: fn decimal_policy_defects_refuse_once() {
- id: bounded_fair_keys_carry_all_eight_members
  path: tools/fsl-probability-check/tests/it/bounded_fair.rs
  role: examined
  excerpt: fn bounded_fair_keys_carry_all_eight_members() {
```

## Dispositions

Round 1, independently inspected at `d10554aae7d603545081e99f239660b18bb0f721`. The fix delta changes only the two ignored-reason strings plus byte-identical copies of the original review artifacts at `reviews/qsl-2678-pr3273-{code-review,gap-analysis}.md`. Both strings now match the inherited root log. Historical ticket identifiers QSL-2400/QSL-2274, completed status and explicit unassigned current owner remain honest; the unchanged ignored-reason gate still accepts their identifiers. Removing ignore lines makes both before/after test files byte-identical, confirming no test body, assertion, trace tag or enable change. No new findings.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Reason now records distinct compiled body wrappers versus the shared-wrapper assertion. |
| FND-002 | fixed | Reason now records inconclusive replay with verdict disagreement versus reproduced-without-witness. |

Current code-review outcome: PASS for reviewed defects; both findings resolved. Final-head focused execution remains pending at this disposition. Draco's requested 93-test run is queued at the original 9b4f776c61 source; the subsequent delta is review prose and ignored-reason strings only, so those execution counts can be reused when delivered. No Cargo, tests or matrix scan was run in this disposition. The original review/findings sections above are preserved as historical records.
