---
type: status
---
# Temporal grammar test validation

The new temporal grammar cases passed focused checks. The single final component
gate at `3d0775109ebb335f32596f697463d2949200b499` exited 101 in an existing
formatter unit test; it did not reach integration or doc tests. Those two
unrun segments were then executed once separately. The original failure log
is retained, and this receipt does not report a full component pass.

## Commands and inputs

All commands used the existing ticket worktree target, `CARGO_BUILD_JOBS=2`,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_PROFILE_TEST_DEBUG=0`. Free space before the gate was 213 GiB.

```sh
cargo test --locked -p fsl-cst -- --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
cargo test --locked -p fsl-cst --test it -- --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
cargo test --locked -p fsl-cst --doc -- --test-threads=1 \
  --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart \
  --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Only the two previously unrun segments followed the failed original command.
The 15 passing unit cases and known failing unit were not rerun. No workspace,
product-binary, lint, formatter or review confidence gate followed.

## Unit disposition

The unit segment recorded 15 passes and one failure:
`format::tests::a_population_bound_ends_its_own_line`. Actual output joins
`M::orders[8]`; the existing required oracle is `M::orders [8]`, likewise for
`people [2]` and `staff [0]`.

FSL FR-003's canonical bracket rule requires a space outside its type-bound,
index, collection-value and temporal/window exceptions. FSpec's
`population-bound` production is none of these exceptions. The formatter
therefore has a product defect; the expected strings remain unchanged. The
unit is explicitly pending under QSL-2679 and excluded from coverage. Its
production formatter remains unchanged in this PR.

Local logs are `.agent/qsl-2076-final-cst.log`,
`.agent/qsl-2076-final-it.log` and `.agent/qsl-2076-final-doc.log`.

## Segmented results

| Segment | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Original unit segment, exit 101 | 15 | 1 | 0 |
| Previously unrun integration segment, exit 101 | 268 | 12 | 11 |
| Previously unrun doc segment, exit 0 | 1 | 0 | 0 |
| Total actual recorded results | 284 | 13 | 11 |

All seven new near-miss functions passed in the integration segment. Its
171.28-second run also recorded these existing failures:

- `complete_cst::header_recovery_resumes_at_the_next_declaration`
- `complete_grammar::complete_source_diagnostics_carry_their_catalogued_typed_cause`
- `complete_grammar::contextual_complete_words_remain_eligible_identifiers`
- `complete_grammar::every_named_production_has_positive_boundary_and_located_negative_vectors`
- `decimal_constructor::a_numeric_token_that_is_no_signed_int_or_uint_is_an_invalid_token`
- `nesting_levels::a_work_budget_refusal_names_the_work_limit`
- `nesting_levels::nested_trailing_comma_typo_recovery_points_at_the_offending_comma`
- `protocol_memory_refinement::a_history_row_refuses_a_non_literal_initial_value_at_its_first_token`
- `readme_snippets::readme_protocol_excerpt_parses_and_native_fences_parse`
- `reserved_declaration_names::lowercase_tuple_still_refuses_as_a_record_name`
- `selection::an_edition_other_than_1_draft_refuses_unknown_edition_at_its_literal`
- `syntax_messages::long_expected_sets_use_a_short_category_phrase`

The original unit segment took 39.53 seconds. The doc segment passed its one
compile-fail example in 0.03 seconds. No failed segment was repeated. The
existing integration failures and 11 ignored cases remain visible; none is
changed into passing coverage by this PR.
