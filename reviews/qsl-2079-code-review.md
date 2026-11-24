---
id: SR-1481
title: "QSL-2079 code-review at 283c5a6558"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2079; five changed paths and integration registration"
review_set: subset
---

## Summary

Independent diff review of PR 3163 at 283c5a6558e08e30bb08a6a9bfd42001b235e768. Rust review and docs accuracy/style inspection are folded into this review.

## Verdict

CONDITIONAL. Confined findings below require disposition. Real public entry points replace the removed source-grep assertions; no production API, diagnostic variant, workflow or real execution test is removed. Four ignored cases remain excluded from passing coverage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The QSL-2687 regression only inspects the first diagnostic. It passes if InvalidEscape is moved first while the unrelated selection-identity diagnostic remains. Inspect the full diagnostic list and reject the unrelated cascade, retaining the located lexical assertion. | tests/it/error_producer_inputs.rs:322-335 |

## Coverage

Plan completion: not assessed

Inspected all changed Rust tests, registration, removed source-inspection tests and both changed documents. Source/host refusal assertions execute genuine parser/editor calls; model/population and invocation cases use the existing authoritative same-repository fixture; replay uses emitted package bytes and real checked identities; protocol emission retains its actual source refusal. No fabricated diagnostic establishes reachability. Protected formatter/monitor guards and higher host serialization are explicitly outside this feature's claim.

Accepted author evidence: 32 active producer groups passed focused runs; four ignored defects have owners QSL-2687, QSL-2688, QSL-2692 and QSL-2693. No tests, lint, builds, matrix, product probes or broad audits were run by this reviewer. The single final producer-component gate remains with the original coder after review. Known unchanged lint blockers were accepted from the author. Repository-wide matrix/reverse-gap assurance and optional broad semantic audit were not run, per dispatch and efficiency rules. This scoped result is not a full gap-analysis PASS. No applicable committed AssuranceProfile declaration was found.

## Examined units

The structured scope below lists every reviewed function/file, including clean units. FR-122 identity inputs were context for the stale-clause test; its AC-3 was examined for the false tag. Other source tags were inspected against their directly corresponding existing criteria where needed; no complete-criterion or catalogue-completeness claim is made.

```yaml
{
  "scope": [
    {
      "id": "identity",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn identity() -> SourceIdentity {"
    },
    {
      "id": "source",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn source(body: &str) -> String {"
    },
    {
      "id": "parsed",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn parsed(text: &str) -> ParsedSource {"
    },
    {
      "id": "refused",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn refused(text: &[u8], limits: Limits) -> CompleteDiagnostic {"
    },
    {
      "id": "produced",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn produced("
    },
    {
      "id": "editor_source",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn editor_source() -> ParsedSource {"
    },
    {
      "id": "catalog",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn catalog() -> ProfileCatalog {"
    },
    {
      "id": "binding",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn binding() -> DocumentBinding {"
    },
    {
      "id": "source_refusals_reach_their_typed_producers_and_documented_rows",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn source_refusals_reach_their_typed_producers_and_documented_rows() {"
    },
    {
      "id": "an_unselected_declaration_form_is_a_real_unsupported_construct",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_unselected_declaration_form_is_a_real_unsupported_construct() {"
    },
    {
      "id": "the_diagnostic_limit_is_reached_by_independent_source_errors",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn the_diagnostic_limit_is_reached_by_independent_source_errors() {"
    },
    {
      "id": "source_preflight_inputs_reach_retained_host_causes_before_parsing",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn source_preflight_inputs_reach_retained_host_causes_before_parsing() {"
    },
    {
      "id": "real_host_edits_reach_predecessor_and_range_causes",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn real_host_edits_reach_predecessor_and_range_causes() {"
    },
    {
      "id": "real_editor_requests_reach_revision_profile_and_cancellation_producers",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn real_editor_requests_reach_revision_profile_and_cancellation_producers() {"
    },
    {
      "id": "a_cancelled_real_parse_stops_at_its_public_entry",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn a_cancelled_real_parse_stops_at_its_public_entry() {"
    },
    {
      "id": "an_invalid_escape_reaches_its_lexical_producer_before_selection_cascades",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_invalid_escape_reaches_its_lexical_producer_before_selection_cascades() {"
    },
    {
      "id": "a_real_node_from_another_parse_reaches_the_foreign_node_host_cause",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn a_real_node_from_another_parse_reaches_the_foreign_node_host_cause() {"
    },
    {
      "id": "exact_definition_inputs_reach_bundle_refusal_producers",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn exact_definition_inputs_reach_bundle_refusal_producers() {"
    },
    {
      "id": "public_compile_intake_inputs_reach_io_json_and_wire_codes",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn public_compile_intake_inputs_reach_io_json_and_wire_codes() {"
    },
    {
      "id": "compile_original",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn compile_original(body: &str) -> Result<Vec<u8>, Box<fsl_command::command::RunError>> {"
    },
    {
      "id": "compile_text",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn compile_text(text: &str) -> Result<Vec<u8>, Box<fsl_command::command::RunError>> {"
    },
    {
      "id": "public_compile_semantic_inputs_keep_missing_typing_and_definedness_codes",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn public_compile_semantic_inputs_keep_missing_typing_and_definedness_codes() {"
    },
    {
      "id": "actual_model_document_inputs_reach_malformed_and_noncanonical_codes",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn actual_model_document_inputs_reach_malformed_and_noncanonical_codes() {"
    },
    {
      "id": "source_selection_inputs_reach_language_and_identifier_codes",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn source_selection_inputs_reach_language_and_identifier_codes() {"
    },
    {
      "id": "source_lexer_and_parser_limits_reach_actual_stage_limit_causes",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn source_lexer_and_parser_limits_reach_actual_stage_limit_causes() {"
    },
    {
      "id": "real_bundle_budget_and_authority_inputs_keep_distinct_codes",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn real_bundle_budget_and_authority_inputs_keep_distinct_codes() {"
    },
    {
      "id": "population_domain",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn population_domain() -> fsl_semantics::model::domain_package::DomainPackage {"
    },
    {
      "id": "population_view",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn population_view("
    },
    {
      "id": "real_population_inputs_reach_foreign_incomplete_and_cardinality_codes",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn real_population_inputs_reach_foreign_incomplete_and_cardinality_codes() {"
    },
    {
      "id": "an_actual_abstract_instance_produces_invalid_runtime_input",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_actual_abstract_instance_produces_invalid_runtime_input() {"
    },
    {
      "id": "duplicate_real_model_selections_produce_duplicate_selection",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn duplicate_real_model_selections_produce_duplicate_selection() {"
    },
    {
      "id": "real_invocations_distinguish_recorded_delta_from_frame_permission",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn real_invocations_distinguish_recorded_delta_from_frame_permission() {"
    },
    {
      "id": "real_snapshot_provisions_distinguish_absence_from_wrong_role",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn real_snapshot_provisions_distinguish_absence_from_wrong_role() {"
    },
    {
      "id": "original_extraction_requests_reach_combination_refusal_producers",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn original_extraction_requests_reach_combination_refusal_producers() {"
    },
    {
      "id": "an_original_extraction_request_preserves_quire_context_diagnostics",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_original_extraction_request_preserves_quire_context_diagnostics() {"
    },
    {
      "id": "a_source_clock_without_time_reaches_the_timed_model_producer",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn a_source_clock_without_time_reaches_the_timed_model_producer() {"
    },
    {
      "id": "an_original_zero_weight_reaches_the_probabilistic_model_producer",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_original_zero_weight_reaches_the_probabilistic_model_producer() {"
    },
    {
      "id": "an_original_sequence_bound_reaches_fixed_representability_refusal",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_original_sequence_bound_reaches_fixed_representability_refusal() {"
    },
    {
      "id": "original_duplicate_operation_bodies_reach_conflicting_declaration",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn original_duplicate_operation_bodies_reach_conflicting_declaration() {"
    },
    {
      "id": "a_real_called_family_without_a_body_reaches_ambiguous_dispatch",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn a_real_called_family_without_a_body_reaches_ambiguous_dispatch() {"
    },
    {
      "id": "an_original_call_request_reaches_unsupplied_execution_capability",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_original_call_request_reaches_unsupplied_execution_capability() {"
    },
    {
      "id": "an_admitted_complete_population_distinguishes_dangling_protocol_seeds",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_admitted_complete_population_distinguishes_dangling_protocol_seeds() {"
    },
    {
      "id": "selected_model_protocol_seeds_require_complete_present_targets",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn selected_model_protocol_seeds_require_complete_present_targets() {"
    },
    {
      "id": "a_host_that_loses_the_checked_model_input_reaches_a_real_internal_fault",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn a_host_that_loses_the_checked_model_input_reaches_a_real_internal_fault() {"
    },
    {
      "id": "an_unknown_request_capability_keeps_the_required_feature_classification",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_unknown_request_capability_keeps_the_required_feature_classification() {"
    },
    {
      "id": "an_original_replay_rejects_a_stale_clause_before_missing_observation_admission",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_original_replay_rejects_a_stale_clause_before_missing_observation_admission() {"
    },
    {
      "id": "an_original_definition_closure_reaches_missing_required_capability",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn an_original_definition_closure_reaches_missing_required_capability() {"
    },
    {
      "id": "real_protocol_refusals_require_the_result_preserving_projection_entry",
      "path": "tests/it/error_producer_inputs.rs",
      "role": "examined",
      "excerpt": "fn real_protocol_refusals_require_the_result_preserving_projection_entry() {"
    },
    {
      "id": "tests/it/main.rs",
      "path": "tests/it/main.rs",
      "role": "examined",
      "excerpt": "mod error_producer_inputs;"
    },
    {
      "id": "fsl-foundation/tests/code_catalogue.rs",
      "path": "fsl-foundation/tests/code_catalogue.rs",
      "role": "examined",
      "excerpt": "fsl-foundation/tests/code_catalogue.rs"
    },
    {
      "id": "docs/native-error-codes.md",
      "path": "docs/native-error-codes.md",
      "role": "examined",
      "excerpt": "docs/native-error-codes.md"
    },
    {
      "id": "docs/diagnostic-producer-validation.md",
      "path": "docs/diagnostic-producer-validation.md",
      "role": "examined",
      "excerpt": "docs/diagnostic-producer-validation.md"
    },
    {
      "id": "FR-122-AC-3",
      "path": "spec/functional/FR-122-replay-a-state-clause-counterexample.md",
      "role": "examined",
      "excerpt": "Missing name: a `state_clause` naming `Absent`, and one naming the function `sameIdentity`, each refuse `missing_declaration`/`missing-name` naming the target."
    },
    {
      "id": "FR-001-AC-11",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "With both the identity and the authority blank, admission refuses `invalid_source_identity`/`blank-label` with `label` `authority`. With the identity blank and the path empty, it refuses `blank-label` with `label` `identity`, not `empty-path`. With both labels non-blank and the path empty, it refuses `invalid_source_identity`/`empty-path` with no field."
    },
    {
      "id": "FR-001-AC-12",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "An incremental edit whose expected source identity differs, and a complete-editor request bound to another source revision, each refuse with `invalid_source_map`, keeping host causes `EditPredecessor` and `RequestRevision` respectively, and naming no region; neither refuses with `invalid_source_identity`. The edit and the request name the one `Source` instance they act on."
    },
    {
      "id": "FR-002-AC-6",
      "path": "spec/functional/FR-002-parse-native-units.md",
      "role": "examined",
      "excerpt": "The bounded malformed/truncation corpus returns a `ParsedSource` or a located diagnostic from `fsl_cst::parse` without panic; any admissible variant retains its exact submitted bytes."
    },
    {
      "id": "FR-010-AC-8",
      "path": "spec/functional/FR-010-report-native-outcomes.md",
      "role": "examined",
      "excerpt": "A native Diagnostic propagates through a standard Error-based caller; every stable code round-trips through its catalog lookup."
    },
    {
      "id": "FR-031-AC-6",
      "path": "spec/functional/FR-031-run-extracted-native-source.md",
      "role": "examined",
      "excerpt": "With every model file the request names absent, a request carrying `extraction` and `package` refuses `extraction-package-conflict` from the request reader; one with no `clauses` binding and one with two refuse `extraction-clause-count` with `details` `{\"actual\": 0}` and `{\"actual\": 2}`; one carrying `call` in place of `clause`, and a compile request carrying `extraction`, refuse `extraction-requires-run`; each at stage `request`, exit 20, empty stdout, with no file read beyond the request. A `null` `extraction`, a positional `body` and an unknown member in `body` each refuse `invalid-request`"
    },
    {
      "id": "FR-111-AC-3",
      "path": "spec/functional/FR-111-link-a-complete-v1-definition-bundle.md",
      "role": "examined",
      "excerpt": "Two roots, or a root and a reached definition, selecting one identity under two authorities refuse `ambiguous_declaration`/`conflicting-authority` naming both; a dependency cycle refuses `invalid_package`/`definition-cycle` naming the cycle in path order."
    },
    {
      "id": "FR-255-AC-1",
      "path": "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md",
      "role": "examined",
      "excerpt": "For each row of the setting table, driving its stage or operation past that limit at a configured bound `B` returns the code Behavior 1's table gives for the limit's carrying type, with the row's limit kind, bound `B`, the count the refused charge would have reached, and the row's setting name, and, for a `stage_limit_exceeded` row, its catalog record carries fields `kind`, `bound`, `actual` and `setting` holding those values; the caller-budget rows are as Behavior 1 states. The `explore.*` rows return their `Outcome::Bounded` instead, and the supplied-value ceilings, which are not setting-tab"
    },
    {
      "id": "FR-276-AC-1",
      "path": "spec/functional/FR-276-cancel-a-lifecycle-operation.md",
      "role": "examined",
      "excerpt": "Each of `parse`, `select`, `check`, `package`, `analyze`, `monitor`, `replay`, `inspect` and `render`, called over its FR-275-AC-1 input with a handle already cancelled with `Requested`, returns `StageFailure::Cancelled(Requested)` and no output; `execute` returns `CallFailure::Cancelled(Requested)`."
    },
    {
      "id": "FR-296-AC-2",
      "path": "spec/functional/FR-296-select-the-execution-backend-in-the-request.md",
      "role": "examined",
      "excerpt": "An `execute` request choosing `jit` with `SuppliedBackends::default()` returns `CallFailure::Unsupported(UnsupportedEntry { backend: Jit, cause: NotSupplied })`, category unsupported (exit 21), and no stage runs."
    },
    {
      "id": "FR-302-AC-7",
      "path": "spec/functional/FR-302-link-dispatch-during-unit-compile.md",
      "role": "examined",
      "excerpt": "A single body for each of `A.size` and `B.size` joins with retained expressions and unit pre/post clauses to admitted operations by resolved `DeclarationKey`, independent of body labels and declaration order; `has_body` is true for those resolved operations and false for an operation without a body, whose called family is not applicable under FR-151. Two body forms resolving to the same `DeclarationKey` refuse as `conflicting_declaration`/`duplicate-declaration`, naming both forms and producing no checked package or partial table. TC-986 is the single-body control; TC-987 covers the duplicate."
    },
    {
      "id": "FR-762-AC-6",
      "path": "spec/functional/FR-762-report-every-independent-syntax-error-of-a-unit.md",
      "role": "examined",
      "excerpt": "A unit of 150 `record` declarations, each missing a field type, under the default limits gives 101 diagnostics: 100 `unexpected-token` diagnostics, then one `invalid_syntax`/`diagnostic-list-cut` diagnostic located at the 101st record's offending token whose message names `s1.diagnostics` and the bound 100. The outcome is a refusal, not `stage_limit_exceeded`."
    }
  ]
}
```

## Dispositions

Round 1, reviewed fix diff at 4c3dda6556a3555303deebdbd54d721799f57010.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The entire diagnostic list now rejects selection and identity cascades while retaining the lexical locus assertion. |

Disposition verdict: PASS for the confined fixes; no new findings. Original findings and their original review verdict remain unchanged above. Inspected new bindings against their corresponding current criteria and FSpec FR-046-AC-3/FR-376-AC-2 plus FR-151 abstract-instance behavior. The ignored QSL-2692 snapshot now correctly omits model and uses source names parcels and Parcel, matching FR-569-AC-9; this does not claim the known source-universe fault is fixed. No tests, lint, builds or fault probes ran.

Additional examined unit: FR-569-AC-9, source snapshot representation. Excerpt: S selects no domain package: a snapshot with no `model` member admits and one with a `model` member refuses `invalid_runtime_input`/`unknown-member` naming `model`.
