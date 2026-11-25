// SPDX-License-Identifier: AGPL-3.0-or-later
//! `fsl-eval`'s integration tests: the ones that exercise only ADR-011 §6.1
//! layer 5 and the layers below it, moved from the root crate's
//! `tests/it/`. One binary, one link step, the same shape as the root
//! crate's `it` target.

#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;
#[path = "../../../tests/support/ring_queue.rs"]
pub(crate) mod ring_queue;
pub(crate) mod support;

/// The package name test fixtures assemble under.
const TEST_PACKAGE_NAME: &str = "fsl-eval-integration-tests";

#[path = "../../../fsl-semantics/tests/support/tc235_k2.rs"]
#[allow(dead_code)]
mod tc235_k2;

mod absent_literal;
mod accounting_boundaries;
mod aggregate_samples;
mod application_observation;
mod automaton_bounded_each;
mod automaton_kernels;
mod automaton_key;
mod automaton_key_bounded;
mod call_verdicts;
mod canonical_node_probe;
mod checked_package_call;
mod checked_route_owner;
mod checked_route_trigger;
mod child_composite;
mod clause_read_paths;
mod clock_reference_v2;
mod collection_algebra;
mod collection_queries;
mod composite_values;
mod compound_conversion_frame;
mod decimal_source_forms;
mod deep_values;
mod dispatch_calls;
mod division_law_run;
mod duplicate_preserving_queries;
mod enumeration_findings;
mod equality_matrix;
mod evaluation_ignores_display_strings;
mod evaluation_rulings;
mod evaluator_closure_cases;
mod execution_coverage;
mod fairness_kernels;
mod family_sums;
mod finite_hyper;
mod finite_simulation;
mod finite_window;
mod float_callables;
mod float_rounding;
mod footprints;
mod hyper_tuple;
mod ieee_node_charges;
mod ieee_ordering_route_parity;
mod inherited_attributes;
mod int_arithmetic_expected_type;
mod kernel_refusal_payloads;
mod machine_contexts;
mod mltl_differential;
mod model_based_tests;
mod model_operation_coverage;
mod model_operations;
mod model_reference_queries;
mod model_simulation;
mod monitor_vs_infinite_trace;
mod named_population;
mod narrowing_conversions;
mod native_literals;
mod native_scalar_equality;
mod native_scalars;
mod observation_owner;
mod one_occurrence;
mod option_values;
mod package_aggregate_route;
mod package_binder_names;
mod property_automata;
mod protocol_delivery_interval;
mod protocol_duplicate_effect;
mod protocol_observe_only;
mod protocol_over;
mod protocol_population;
mod protocol_reference;
mod protocol_retry_bound;
mod protocol_slots;
mod qualification_enum_rows;
mod qualification_vectors_from_packages;
mod qualification_vectors_from_source;
mod qualified_bounded_monitor;
mod quantity_class_convert;
mod quantity_literals;
mod quantity_negation;
mod quantity_power;
mod reference_adapter;
mod refinement_populations;
mod refinement_row_clock_places;
mod relationship_navigation;
mod relationship_reachability;
mod removal_filter_decrease;
mod smt_query;
mod source_call;
mod source_object_probabilistic;
mod source_objects;
mod state_model_family;
mod supplied_phases;
mod tc235_business_meaning_runtime;
mod temporal_activation;
mod temporal_aggregate_precedence;
mod temporal_capture_activations;
mod temporal_causes;
mod temporal_evidence;
mod temporal_identity;
mod temporal_laws;
mod temporal_relation;
mod temporal_rulings;
mod temporal_stream;
mod temporal_trace;
mod text_size;
mod text_to_number;
mod tick_monitor;
mod timed_recorded_trace;
mod timed_subject;
mod timed_trace;
mod total_functions;
mod type_invariants;
mod typed_value_differential;
mod unbounded_set_call;
mod union_value_profile;
mod unit_diagnostic_vectors;
mod value_properties;
mod variant_fields;
mod weighted_sampler;
mod window_aggregates;
mod witnessed_environment;
