// SPDX-License-Identifier: AGPL-3.0-or-later
//! Analyze API, CLI, and package integration tests.

mod analysis_view;
mod analyze;
mod analyze_cli;
mod analyze_cli_support;
mod analyze_composition_cli;
mod analyze_last_stage;
mod analyze_liveness_cli;
mod analyze_package_deadlock_models;
mod analyze_package_protocol;
mod analyze_package_provider;
mod analyze_package_replay_conformance;
mod analyze_package_schedulability;
mod analyze_package_value;
mod analyze_protocol_subject;
mod analyze_reduction_cli;
mod analyze_request_wire;
mod analyze_state_graph_cli;
mod analyze_tp3_package;
mod analyze_wire;
mod clause_item;
mod closure_certificate;
mod counter;
#[path = "dense_route_support.rs"]
mod dense_route;
#[path = "derived_request_support.rs"]
mod derived_request;
mod fill;
mod fixture_ids;
mod fr114_process_attempt;
mod graph_world;
mod hybrid_provider;
mod hybrid_reference;
mod hybrid_requests;
mod hyper;
mod hyper_projected;
mod hyper_world;
mod litmus;
mod manifest_registry;
mod model_settle_support;
mod model_world;
mod observation_mapping;
mod package_enum;
mod package_loop_invariant;
mod package_model_relation;
mod package_recommendation_bridge;
mod package_refusal_route;
mod package_simulation;
mod package_step_relation_parity;
mod plan;
mod plan_search;
mod prob_world;
mod protocol_bounded_support;
mod protocol_composition;
mod protocol_counted_support;
mod protocol_item_bounds;
mod provider_external_contract;
mod provision_contract;
#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;
mod reference_literal_binding;
#[path = "../../../tests/support/refinement_cas.rs"]
mod refinement_fixture;
mod related_by_support;
mod relation_claims;
mod statistical_witness_replay;
mod statistical_witness_support;
mod timed_world;
mod units;
mod value_validity;
