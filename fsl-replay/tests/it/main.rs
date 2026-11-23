// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests that reach `fsl-replay` through its public facade only.

#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;

mod admitted_record_chain;
mod aggregate_dispatch;
mod application_live_occurrences;
mod category_facade;
mod checked_body_failure;
mod compile_ieee_profile;
mod compile_protocol_profile;
mod compile_text_literals;
mod conditioned_product_key;
mod declaration_only_unit;
mod format_outcome_facade;
mod hyper_finite_replay;
mod hyper_identity;
mod hyper_per_trace_clock;
mod hyper_query;
mod hyper_smt_settle;
mod indexed_read_stage;
mod intake_refusal_outcome;
mod integer_division_check;
mod emit_admit_agreement;
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
mod replay_result_rows;
#[path = "../../../tests/support/ring_world.rs"]
mod ring_world;
mod run_axes;
mod script_search;
mod source_diagnostic_outcome;
mod static_subject;
mod tool_failure;
mod undeclared_variant_case_message;
mod value_family_end_to_end;
mod vault_world;
mod witness_entry_name;
