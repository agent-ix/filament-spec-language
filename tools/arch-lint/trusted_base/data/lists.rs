// SPDX-License-Identifier: AGPL-3.0-or-later
//! The item-level lists of FR-1051: base recordings, loop-proof types, check
//! entries and codec functions.

pub(super) const B9: &[&str] = &[
    "fsl_eval::automaton::error::AutomatonError",
    "fsl_eval::automaton::letter::Atom",
    "fsl_eval::automaton::letter::Atom::new",
    "fsl_eval::automaton::letter::Letter",
    "fsl_eval::automaton::state::StateId",
    "fsl_eval::automaton::timed_claim::ClockedEdge",
    "fsl_eval::automaton::timed_claim::position_automaton",
    "fsl_eval::automaton::timed_claim::PositionAutomaton",
    "fsl_eval::automaton::timed_window::Read",
    "fsl_eval::automaton::timed_window::Stamp",
    "fsl_eval::automaton::timed_window::WindowClaim",
    "fsl_eval::automaton::timed_window::WindowClaim::new",
    "fsl_eval::automaton::timed_window::WindowRefusal",
];
pub(super) const B11: &[&str] = &[
    "fsl_replay::bounds::BoundExceeded::check",
    "fsl_replay::execute::recompile",
    "fsl_replay::execute::recompile_with_temporal_selections",
    "fsl_replay::execute::request_limits",
    "fsl_replay::identity::DefinitionSourceRef::from_wire",
    "fsl_replay::identity::QualifiedName::new",
    "fsl_replay::limits::AccountingLimits::setting_of",
    "fsl_replay::limits::CallerLimits::from_operands",
    "fsl_replay::model_check::limits::DEFAULT_MAX_AUTOMATON_STATES",
    "fsl_replay::model_check::limits::meter_bound",
    "fsl_replay::request::ByteProvision::admit",
    "fsl_replay::request::package::ReplayPackage::read",
    "fsl_replay::request::ReplayRequest::decode",
    "fsl_replay::request::ReplayRequest::decode_package_route",
    "fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT",
    "fsl_replay::spine::identity_form::is_effective_form",
    "fsl_replay::spine::lifecycle::check",
    "fsl_replay::spine::lifecycle::package",
    "fsl_replay::spine::lifecycle::parse",
    "fsl_replay::spine::lifecycle::select",
];
pub(super) const B12: &[&str] = &[
    "fsl_replay::outcome::counterexample::state_key_wire",
    "fsl_replay::outcome::counterexample::task_source_transition_wire",
    "fsl_replay::outcome::TimedStateWire::from",
    "fsl_replay::outcome::WrittenTransition::from",
    "fsl_replay::result::Verdict::from_category",
    "fsl_replay::result::wire::ProtocolWitnessWire::of",
    "fsl_replay::written_operation::WrittenOperation::from_wire",
];

/// The loop-proof types of `fsl_eval::simulation::protocol` (row S-9).
pub(super) const LOOP_PROOF_TYPES: &[&str] = &[
    "fsl_eval::simulation::protocol::ProtocolLoopProofKind",
    "fsl_eval::simulation::protocol::ProtocolLoopProofTarget",
    "fsl_eval::simulation::protocol::ProtocolLoopProofFailure",
    "fsl_eval::simulation::protocol::ProtocolLoopProofViolation",
];

/// R-1 to R-18: the EN-5 profiles and timed routes, through the document,
/// source and package entries.
const EN5: &[&str] = &[
    "R-1", "R-2", "R-3", "R-4", "R-5", "R-6", "R-7", "R-8", "R-9", "R-10", "R-11", "R-12", "R-13",
    "R-14", "R-15", "R-16", "R-17", "R-18",
];
const ZONE: &[&str] = &[
    "R-22", "R-23", "R-24", "R-25", "R-26", "R-27", "R-28", "R-29", "R-31",
];

pub(super) const CHECK_ENTRIES: &[(&str, &[&str])] = &[
    (
        "fsl_replay::probability_certificate::document::check_probability_certificate_document",
        EN5,
    ),
    (
        "fsl_replay::probability_certificate::document::check_probability_certificate_document_as",
        EN5,
    ),
    (
        "fsl_replay::probability_certificate::source::check_probability_certificate_source",
        EN5,
    ),
    (
        "fsl_replay::probability_certificate::source::check_probability_certificate_package",
        EN5,
    ),
    (
        "fsl_replay::probability_certificate::check_claim_exact_reachability",
        &["R-1"],
    ),
    (
        "fsl_replay::probability_certificate::document::check_cyclic_reward_probability_certificate_observing",
        &["R-9"],
    ),
    (
        "fsl_replay::probability_certificate::document::check_bounded_fair_probability_certificate_observing",
        &["R-10"],
    ),
    (
        "fsl_replay::probability_certificate::document::mdp::check_cyclic_mdp_probability_certificate_observing",
        &["R-6", "R-8", "R-9", "R-11"],
    ),
    (
        "fsl_replay::probability_certificate::document::mdp::check_cyclic_mdp_probability_certificate_with_identity",
        &["R-6", "R-8", "R-9", "R-11"],
    ),
    (
        "fsl_replay::probability_certificate::document::long_run::check_long_run_probability_certificate_observing",
        &["R-15", "R-16"],
    ),
    (
        "fsl_replay::probability_certificate::document::long_run::check_long_run_probability_certificate_with_identity",
        &["R-15", "R-16"],
    ),
    (
        "fsl_replay::probability_certificate::witness::replay_probabilistic_witness",
        &["R-19"],
    ),
    (
        "fsl_replay::probability_certificate::witness::replay_probabilistic_witness_observing",
        &["R-19"],
    ),
    (
        "fsl_replay::probability_certificate::witness::replay_probabilistic_witness_from_package",
        &["R-19"],
    ),
    (
        "fsl_replay::probability_certificate::witness::undefined::replay_probabilistic_undefined",
        &["R-20"],
    ),
    (
        "fsl_replay::probability_certificate::document::instances::replay_on_each_witness",
        &["R-21"],
    ),
    (
        "fsl_replay::certificate::zone_certificate::check_zone_certificate",
        ZONE,
    ),
    (
        "fsl_replay::certificate::zone_certificate::check_free_delay_zone_certificate",
        &["R-30"],
    ),
    (
        "fsl_replay::certificate::state_graph::check_state_graph",
        &["R-32"],
    ),
    ("fsl_replay::certificate::closure::check_closure", &["R-33"]),
    ("fsl_replay::certificate::closure::check_components", &["R-34"]),
    (
        "fsl_replay::certificate::closure::check_components_over_stutter_closure",
        &["R-35"],
    ),
    (
        "fsl_replay::certificate::closure::check_protocol_closure",
        &["R-36"],
    ),
    (
        "fsl_replay::certificate::closure::check_protocol_components",
        &["R-37"],
    ),
    (
        "fsl_replay::certificate::product::check_product_closure",
        &["R-38"],
    ),
    (
        "fsl_replay::certificate::product::check_model_relation_closure",
        &["R-39"],
    ),
    (
        "fsl_replay::certificate::product::check_package_hyper_closure",
        &["R-40"],
    ),
    (
        "fsl_replay::model_check::hyper_replay::check_match_witness",
        &["R-41"],
    ),
    ("fsl_replay::certificate::smt::check_smt_proof", &["R-42"]),
    (
        "fsl_replay::certificate::smt::check_refinement_smt_proof",
        &["R-43"],
    ),
    (
        "fsl_replay::refinement_query::RefinementQueryRequest::check_smt_proof",
        &["R-43"],
    ),
    (
        "fsl_replay::certificate::simulation::check_simulation_certificate",
        &["R-44"],
    ),
    (
        "fsl_replay::certificate::closed_form::check::check_closed_form",
        &["R-45"],
    ),
    (
        "fsl_replay::certificate::closed_form::recompute::recompute",
        &["R-45"],
    ),
    (
        "fsl_replay::certificate::closed_form::settle::settle",
        &["R-45"],
    ),
    (
        "fsl_replay::hybrid_replay::certificate::check_certificate",
        &["R-46"],
    ),
    ("fsl_replay::hybrid_replay::exact_replay", &["R-47"]),
    ("fsl_replay::hybrid_replay::static_replay", &["R-48"]),
    (
        "fsl_replay::model_check::replay::replay_model_trace",
        &["R-49", "R-50"],
    ),
    (
        "fsl_replay::model_check::replay::replay_model_trace_with_temporal_selections",
        &["R-49", "R-50"],
    ),
    (
        "fsl_replay::model_check::replay::replay_checked_source_model_trace",
        &["R-49"],
    ),
    (
        "fsl_replay::model_check::replay::replay_model_trace_from_package",
        &["R-50"],
    ),
    (
        "fsl_replay::model_check::replay::replay_protocol_trace",
        &["R-51"],
    ),
    (
        "fsl_replay::model_check::replay::replay_protocol_trace_with_temporal_selections",
        &["R-51"],
    ),
    (
        "fsl_replay::model_check::replay::replay_protocol_trace_from_package",
        &["R-51"],
    ),
    (
        "fsl_replay::model_check::replay::replay_timed_counterexample",
        &["R-52"],
    ),
    (
        "fsl_replay::outcome::counterexample::replay_task_product_trace",
        &["R-53"],
    ),
    (
        "fsl_replay::model_check::graph_replay::replay_model_graph",
        &["R-54"],
    ),
    (
        "fsl_replay::model_check::graph_replay::replay_model_graph_with_temporal_selections",
        &["R-54"],
    ),
    (
        "fsl_replay::model_check::graph_replay::replay_model_graph_with_cancel",
        &["R-54"],
    ),
    (
        "fsl_replay::model_check::graph_replay::replay_model_graph_with_cancel_and_temporal_selections",
        &["R-54"],
    ),
    (
        "fsl_replay::model_check::graph_replay::replay_model_graph_from_package",
        &["R-54"],
    ),
    (
        "fsl_replay::model_check::hyper_replay::replay_hyper_trace_tuple",
        &["R-55"],
    ),
    (
        "fsl_replay::model_check::hyper_replay::replay_model_relation_tuple",
        &["R-56"],
    ),
    (
        "fsl_replay::model_check::hyper_replay::replay_model_relation_input",
        &["R-56"],
    ),
    (
        "fsl_replay::model_check::finite_hyper_replay::replay_finite_hyper_refutation",
        &["R-57"],
    ),
    (
        "fsl_replay::model_check::finite_hyper_replay::reproduce_finite_hyper_refutation",
        &["R-57"],
    ),
    (
        "fsl_replay::model_check::refinement_replay::replay_refinement_trace",
        &["R-58"],
    ),
    (
        "fsl_replay::model_check::refinement_replay::replay_timed_refinement_trace",
        &["R-59"],
    ),
    (
        "fsl_replay::model_check::refinement_replay::replay_free_delay_refinement_trace",
        &["R-60"],
    ),
    (
        "fsl_replay::model_check::refinement_replay::replay_protocol_assumption_witness",
        &["R-61"],
    ),
    ("fsl_replay::contract_check::ContractTarget::replay", &["R-62"]),
];

pub(super) const CODEC: &[(&str, &str)] = &[
    ("fsl_replay::probability_certificate::ProbabilityProductKey::from_canonical_bytes", "C-1"),
    ("fsl_replay::probability_certificate::document::read_probability_certificate", "C-1"),
    ("fsl_replay::probability_certificate::document::write_probability_certificate", "C-1"),
    ("fsl_replay::probability_certificate::document::long_run::read_long_run_probability_certificate", "C-1"),
    ("fsl_replay::probability_certificate::document::long_run::read_weighted_long_run_probability_certificate", "C-1"),
    ("fsl_replay::probability_certificate::document::long_run::write_long_run_probability_certificate", "C-1"),
    ("fsl_replay::probability_certificate::document::long_run::write_weighted_long_run_probability_certificate", "C-1"),
    ("fsl_replay::probability_certificate::document::mdp::read_probability_proof_document", "C-1"),
    ("fsl_replay::probability_certificate::document::mdp::write_probability_proof_document", "C-1"),
    ("fsl_replay::probability_certificate::document::measure::read_measure_probability_certificate", "C-1"),
    ("fsl_replay::probability_certificate::document::measure::write_measure_probability_certificate", "C-1"),
    ("fsl_replay::probability_certificate::witness::ProbabilitySubsystemCertificate::from_document", "C-1"),
    ("fsl_replay::probability_certificate::witness::ProbabilitySubsystemCertificate::from_long_run", "C-1"),
    ("fsl_replay::probability_certificate::witness::ProbabilitySubsystemCertificate::from_weighted_long_run", "C-1"),
    ("fsl_replay::probability_certificate::document::measure::conditional_measure_probability_certificate", "C-2"),
    ("fsl_replay::probability_certificate::document::ProbabilityDocumentError::from", "C-4"),
    ("fsl_replay::probability_certificate::document::ProbabilityDocumentIoLimits::default", "C-4"),
    ("fsl_replay::probability_certificate::document::ProbabilityDocumentLimits::for_exact", "C-4"),
    ("fsl_replay::probability_certificate::document::ProbabilityDocumentLimits::for_interval", "C-4"),
    ("fsl_replay::probability_certificate::document::ProbabilityDocumentLimits::interval_scalar_setting", "C-4"),
    ("fsl_replay::probability_certificate::key_admission::ProbabilityKeyAdmission::new", "C-4"),
    ("fsl_replay::probability_certificate::witness::ProbabilityWitnessError::from", "C-4"),
    ("fsl_replay::certificate::zone_certificate::ZoneCertificate::from_wire", "C-1"),
    ("fsl_replay::certificate::zone_certificate::ZoneCertificate::from_wire_with_source", "C-1"),
    ("fsl_replay::certificate::zone_certificate::CertificateCheckLimits::default", "C-4"),
    ("fsl_replay::certificate::task_product::wire::ACTIONS", "C-1"),
    ("fsl_replay::certificate::task_product::wire::action_name", "C-1"),
    ("fsl_replay::hybrid_replay::certificate::wire::read_certificate", "C-1"),
    ("fsl_replay::hybrid_replay::trace::ExactInterval::new", "C-2"),
    ("fsl_replay::hybrid_replay::trace::ExactInterval::point", "C-2"),
    ("fsl_replay::hybrid_replay::HybridReplayMeter::new", "C-4"),
    ("fsl_replay::model_check::identity::node_occurrence", "C-3"),
];
