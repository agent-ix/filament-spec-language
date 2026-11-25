// SPDX-License-Identifier: AGPL-3.0-or-later
//! model_check integration tests.

mod analyze_cli_support;
mod analyze_coverage_cli;
mod analyze_family_cli;
mod analyze_family_support;
mod analyze_package_selection;
mod analyze_reach_rulings;
mod analyze_timed_cli;
#[allow(
    dead_code,
    reason = "shared fixture support spans split integration test binaries"
)]
mod counter;
mod dense_route;
mod dense_route_support;
mod derived_request;
mod derived_request_support;
mod example_units;
mod hyper_world;
mod lasso_replay;
mod lasso_support;
mod letters;
mod litmus;
mod liveness;
mod liveness_support;
mod long_run_support;
mod model_settle;
mod model_settle_support;
mod native_correlation;
mod package_forms;
mod prob_world;
mod probabilistic_request_support;
mod protocol_analysis_differential;
mod protocol_bounded_lasso;
mod protocol_bounded_support;
mod protocol_compensation;
mod protocol_conformance;
mod protocol_constructs;
mod protocol_counted_steps;
mod protocol_counted_support;
mod protocol_docs_examples;
mod protocol_footprint_support;
mod protocol_footprints;
mod protocol_outcomes;
mod protocol_projection;
mod protocol_realizability;
mod protocol_recording;
mod protocol_replay_mode;
mod protocol_roles;
mod protocol_step_view;
mod protocol_system;
mod provider_schema_support;
#[expect(
    dead_code,
    reason = "some shared reduction fixture helpers are used by sibling test binaries"
)]
#[path = "reduction_world.rs"]
mod reduction_world;
#[path = "../../../tests/support/refinement_cas.rs"]
mod refinement_fixture;
#[path = "../../../tests/support/refinement_smt_proofs.rs"]
mod refinement_proofs;
mod refinement_protocol_support;
mod related_by;
mod related_by_support;
mod scheduler_fairness;
mod timed_refinement;
mod timed_world;
mod units;
mod value_validity_support;
mod weak_memory_support;
mod zone_dense;
mod zone_liveness;
mod zone_replay;
mod zone_search;
#[path = "zone_support.rs"]
mod zone_support;

mod certificate_rejection_support;
mod check_mode_record;
mod claim_family;
mod clause_route;
mod closure_replay;
mod component_certificate;
mod config_lasso_fairness;
mod counterexample_instance;
mod deadlock_item;
mod digital_clocks;
mod fill;
mod fixture_ids;
mod free_delays;
mod graph_core;
mod graph_world;
mod invariant_item;
mod loop_proof;
mod model_check;
mod model_check_captures;
mod model_check_input;
mod model_check_oracle;
mod model_check_safety;
mod model_instances;
mod model_trace_run;
mod model_world;
mod por_world;
mod provider_test_support;
#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;
mod repeat_guard;
mod schedulability;
mod schedulability_routing;
mod simulation_findings;
mod state_graph;
mod state_graph_certificate;
mod state_graph_settle;
mod step_simulation;
mod strong_fairness;
mod strong_hint;
mod task_automaton_crosscheck;
mod task_automaton_lowering;
mod task_product;
mod task_product_certificate;
mod task_product_producer;
mod terminal_two_models;
mod time_lock_item;
mod timed_hidden_candidates;
mod timed_product_certificate;
mod timed_refinement_certificate;
mod timed_refinement_liveness;
mod timed_state_graph;
mod tso_fairness;
mod universe_obligation;
mod zone_certificate;
mod zone_clock_collections;
mod zone_live_certificate;
mod zone_mitl;
