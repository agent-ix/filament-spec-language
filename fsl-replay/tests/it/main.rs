// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests that reach `fsl-replay` through its public facade only.

#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;

mod admitted_composite_facade;
mod admitted_composite_root_only;
mod admitted_context_facade;
mod admitted_record_chain;
mod admitted_root_only;
mod admitted_unit_facade;
mod admitted_unit_root_only;
mod admitted_wire_lookup;
mod aggregate_dispatch;
mod application_live_occurrences;
mod call_replay_context;
mod call_replay_context_root_only;
mod call_requirements_facade;
mod call_requirements_root_only;
mod category_facade;
mod checked_body_failure;
mod checked_nominal_declaration;
mod clause_evaluation_facade;
mod clause_evaluation_root_only;
mod compile_ieee_profile;
mod compile_package_facade;
mod compile_protocol_profile;
mod compile_text_literals;
mod conditioned_product_key;
mod declaration_only_unit;
mod declared_domain_facade;
mod dispatch_link_compile;
mod emit_admit_agreement;
mod format_outcome_facade;
mod hyper_finite_replay;
mod hyper_identity;
mod hyper_per_trace_clock;
mod hyper_query;
mod hyper_smt_settle;
mod imported_function_replay;
mod imported_function_site;
mod indexed_read_stage;
mod intake_refusal_outcome;
mod integer_division_check;
mod large_check_node_limit;
mod native_fence_failure;
mod native_run_result_reader;
mod outcome_facade;
mod parse_outcome_facade;
mod probability_certificate;
mod quantity_namespace_source;
mod quantity_type_source;
mod query_proof;
#[path = "../../../tests/support/refinement_aux.rs"]
mod refinement_aux;
#[path = "../../../tests/support/refinement_cas.rs"]
mod refinement_fixture;
mod refinement_hidden;
mod refinement_history;
mod refinement_mapping;
#[path = "../../../tests/support/refinement_smt_proofs.rs"]
mod refinement_proofs;
mod refinement_query_facade;
mod refinement_ring;
mod refinement_smt;
mod refinement_step;
mod refinement_unmapped;
mod replay_accounting_limit;
mod replay_cancellation_boundary;
mod replay_result_rows;
mod request_clocks_for_replay;
#[path = "../../../tests/support/ring_world.rs"]
mod ring_world;
mod run_axes;
mod script_search;
mod smt_setting_operation;
mod source_diagnostic_outcome;
mod stage_limit_named_root_only;
mod static_subject;
mod terminal_record_facade;
mod tool_failure;
mod undeclared_variant_case_message;
mod value_family_end_to_end;
mod vault_world;
mod witness_entry_name;
