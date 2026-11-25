---
id: SR-3112002
title: "Manual FR-563 acceptance-criteria-to-tests check for QSL-3112 (filament-spec-language PR 3800)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; spec FR-563 (AC-1..AC-9); fsl-conformance/tests/it/parity.rs; fsl-conformance/src/parity.rs; fsl-conformance/src/lib.rs; xtask/src/parity.rs; xtask/tests/qualify_parts.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-563
    type: reviews
---

# Manual FR-563 AC-to-tests check for QSL-3112

## Summary

Ticket: QSL-3112, head d60f04a225b7cb8fc04cee8874413db37ca5c6ff. This is a manual, scoped acceptance-criteria-to-tests check, not a full gap audit. Following the dispatch brief, `quoin matrix` and the whole-repo reverse-gap audit were not run, because they hung for earlier reviewers on this repo. Stub and reverse-trace checks covered only the PR diff. No test was run (cargo forbidden by the lead's orders).

Bindings, all read in source:
- AC-1: `interpreter_backend_passes_default_generated_scope`, correct but over an empty corpus (FND-001). `corpus_bytes_argument_is_decoded_for_selected_runtime_vector`, wrong (FND-002).
- AC-2: `comparison_member_differences_are_named_and_reproducible` and `kernel_undefined_differences_are_compared_against_the_interpreter`, correct. Together they cover all eight members plus the Fault case.
- AC-3: `alternating_backend_is_reported_nondeterministic`, correct.
- AC-4: `coverage_gap_depends_on_declared_forms`, correct.
- AC-5: `prepare_failure_and_all_unsupported_are_reported`, correct.
- AC-6: `reference_fault_and_preflight_refusals_are_distinct`, correct.
- AC-7: `finding_retention_is_bounded_without_stopping_the_run`, correct.
- AC-8: `unknown_option_is_a_named_refusal` plus the cfg_attr compile_fail/control doctests in fsl-conformance/src/lib.rs, correct. The inspection half holds: neither ParityInputs nor the xtask options have an allow/skip/retry/tolerance member or a reference selector.
- AC-9: `run_vector_is_selected_with_its_single_decoded_tuple` and `corpus_aggregate_arguments_use_the_shared_fr350_value_adapter`, correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-563-AC-1's corpus half has no test: the only AC-1 pass test uses an empty corpus, and the pinned complete-v1 fixture has no runtime/run vector, so a delegating backend has never been shown to pass a selected corpus case. | fsl-conformance/tests/it/parity.rs:734-752 |
| FND-002 | low | `corpus_bytes_argument_is_decoded_for_selected_runtime_vector` is tagged FR-563-AC-1 but asserts an all-unsupported run, which by AC-5 is an EmptyComparison failure. The correct binding is FR-563-AC-9 (corpus adapter) or untagged. | fsl-conformance/tests/it/parity.rs:925-949 |

## Verdict

Every FR-563 AC has at least one tagged test that asserts its main path. Two gaps remain: AC-1's corpus half is untested (FND-001) and one test is tagged to the wrong AC (FND-002). Matrix computation and the repo-wide reverse-gap audit were not run.

## Dispositions

Disposition pass 1 at head 1b8c75ead1c345bb748e947497934a0a52b2c218 (static only). The merged FSL #3887 adds FR-563-AC-10 to AC-16 and FR-567-AC-7 and AC-8. Each has a tagged test, read in source:
- FR-563-AC-10: `a_document_level_defect_refuses_before_any_unit_runs`
- FR-563-AC-11: `unbuildable_vectors_are_listed_and_found_while_the_rest_run`
- FR-563-AC-12: `an_expected_refusal_excuses_only_what_fsl_refuses`
- FR-563-AC-13: `refused_draws_beyond_the_bound_fail_the_gate`
- FR-563-AC-14: `reference_preparation_failures_are_findings`
- FR-563-AC-15: `a_prefix_call_is_compared`
- FR-563-AC-16: `cancellation_while_a_case_is_built_gives_no_report`
- FR-567-AC-7: `a_replacement_unit_count_sets_the_generated_units_and_has_no_option`
- FR-567-AC-8: `an_unbuildable_vector_fails_parity_and_a_duplicate_id_refuses`

FR-563-AC-8's new `--max-discarded 1` half is in `unknown_option_is_a_named_refusal`. No new gap was found.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 100405e1b2 added `interpreter_backend_passes_a_selected_corpus_vector`: a delegating backend compares the selected `inc` vector with no finding and exit 0. |
| FND-002 | fixed | The mis-tagged bytes test was retagged in 100405e1b2 and removed in f6da35c396 (`bytes` is no FR-336 `InputValue` tag), so the wrong binding no longer exists. |
