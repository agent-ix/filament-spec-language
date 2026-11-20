# QSL-2095 final integration gate

The independent review passed without findings: SR-1507 and SR-1508, validated with exit 0; tracker comments `9c8fa3c9` and `e591b527`. The unchanged artifacts are committed under `reviews/`.

This is a bounded partial slice of QSL-2095. Three new tests are active. The operator-locus oracle owned by QSL-2722 and conditional-equality oracle owned by QSL-2723 remain ignored and uncounted. Bytes-only evaluation is exercised in-process; fresh-process and historical-family completion are unclaimed.

Both nonoverlapping segments below tested `a84be8213530461304c93433dd0eac80d954ea0d`. Before the gate, `/` had 210G available. The owned warm target was reused with two jobs, dev/test debug 0, incremental compilation disabled and one test thread. No lint, matrix, workspace/proof/Kani, main refresh or confidence run was performed.

```sh
CARGO_TARGET_DIR=$PWD/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p filament-spec-language --test it --features checked-handoff checked_package_state_core:: -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Exit **0**: **1 new passed, 0 failed, 1 strict pending ignored**; 1265 filtered. Log: `/tmp/qsl-2095-final-state-core.log`.

```sh
CARGO_TARGET_DIR=$PWD/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p filament-spec-language --test it --features checked-handoff bare_recovery_binder:: -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Exit **101**: **2 new passed + 1 existing passed, 21 existing failed, 1 strict pending ignored**; 1242 filtered. Log: `/tmp/qsl-2095-final-bare-recovery.log`.

All 21 failures stop at the fixture's attempt-result type mismatch: `M::Workflow::apply` returns Boolean, while event-position recipe R binds `attempted: M::Notice` and reads `attempted.ready`. Nineteen fail in the handoff writer and two case-arm variants fail at their direct checker. No production evaluator failure is reached by those 21 paths. The new child fixture already uses the current Boolean contract; its two active tests passed.

The original failure receipt is retained. The confined correction changes the shared event-position attempt binder/read to Boolean and removes the new child's redundant normalization, leaving that child's final input bytes unchanged. No production code or substantive recovery assertion changes.

## Correction of the 21 failed inputs

Tested head: `ff98e8f219169e3914a130b8a51898cd1ecd9d56`.

```sh
mapfile -t qsl_failed_names < <(sed -n 's/^test \(bare_recovery_binder::[^ ]*\) \.\.\. FAILED$/\1/p' /tmp/qsl-2095-final-bare-recovery.log)
CARGO_TARGET_DIR=$PWD/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p filament-spec-language --test it --features checked-handoff -- --exact "${qsl_failed_names[@]}" --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

The selected names were exactly these previously failed existing cases under `bare_recovery_binder::`:

```text
a_binder_the_body_never_reads_is_no_source
a_case_arm_binder_in_a_recover_condition_is_admitted_and_evaluates
a_let_binder_is_no_source_and_the_predicate_evaluates
a_malformed_recovery_supply_is_refused
a_query_binder_is_no_source_of_the_slot
a_repeated_source_unit_row_refuses_the_bare_route
a_selected_union_with_an_unrepresented_payload_refuses_before_charging
a_source_unit_row_naming_no_defined_unit_refuses_the_bare_route
a_source_unit_row_naming_no_unit_node_refuses_the_bare_route
a_supply_for_an_introduced_binder_is_refused_as_unknown
a_supply_without_the_source_unit_rows_leaves_recovery_unrepresented
an_unrepresented_union_payload_does_not_poison_an_unrelated_slot
duplicate_queries_over_the_receipts_evaluate_from_the_handoff_files
exists_binder_is_no_source
filter_binder_is_no_source
forall_binder_is_no_source
full_recovery_predicate_evaluates_through_the_bare_route
map_binder_is_no_source
nested_query_binders_are_no_sources_and_the_predicate_evaluates
partial_recovery_predicate_evaluates_through_the_bare_route
the_unit_definition_read_is_charged_to_the_native_budget
```

Exit **101**: **20 existing passed, 1 existing failed**, 0 ignored, 1246 filtered. Log: `/tmp/qsl-2095-final-bare-recovery-correction.log`. The remaining duplicate-query case loads its own `unit-duplicate-queries.native`, which held the same invalid Notice result binder. No production evaluator failure was reached.

## Changed-input query-binder case

The already-passing query-binder case also reads the shared event-position fixture. Its final input therefore changed, and the lead authorized this exact validation of that actual change. Its AmbiguousName/targetFull oracle is unchanged.

Tested head: `ff98e8f219169e3914a130b8a51898cd1ecd9d56`.

```sh
CARGO_TARGET_DIR=$PWD/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p filament-spec-language --test it --features checked-handoff bare_recovery_binder::a_query_binder_named_as_a_source_is_refused_when_the_unit_is_checked -- --exact --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Exit **0**: **1 existing passed**, 0 failed, 0 ignored, 1266 filtered. Log: `/tmp/qsl-2095-final-query-binder-changed-input.log`.

## Remaining duplicate-query fixture correction

The separate duplicate-query fixture received only the same Boolean binder/read correction. Tested head: `da646c832c527bc8a612a375ebb4a42622296c16`.

```sh
CARGO_TARGET_DIR=$PWD/target CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p filament-spec-language --test it --features checked-handoff bare_recovery_binder::duplicate_queries_over_the_receipts_evaluate_from_the_handoff_files -- --exact --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated
```

Exit **0**: **1 existing passed**, 0 failed, 0 ignored, 1266 filtered. Log: `/tmp/qsl-2095-final-duplicate-queries-correction.log`.

## Final result and limits

Across the nonoverlapping gate and confined changed-input corrections, all **3 new active** and **22 existing** affected cases have passing receipts. The **2 strict pending** cases remain ignored and uncounted. No whole parent or passing unchanged input was repeated. The two previously passing new child cases were not rerun because their final input bytes are unchanged. All actual processes exited. The final receipt commit changes documentation only; no further gate is required for it.
