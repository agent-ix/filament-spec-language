// SPDX-License-Identifier: AGPL-3.0-or-later
//! Required selections and synthetic completion output for the process double.

pub(super) const CHECKS: [(&str, &str); 11] = [
    ("fspec_finite_window::conformance_fspec_finite_window_source_windows", "conformance: 77 finite-window source windows matched\nconformance: 15 tick-cover vectors matched, 8 through source and 7 by the evaluator comparison only"),
    ("fspec_state_core::conformance_fspec_state_core_source_cases", "conformance: 28 state-core source cases matched"),
    ("checked_v2::tests::conformance_c14_source_map_lookup_over_fspec_positive_fixtures", "conformance: 1 source-map entries over 1 positive fixtures"),
    ("checked_v2::tests::conformance_i2_read_over_fspec_checked_package_v2_fixtures", "conformance: 1 positive fixtures through FSL's full I2 read\nconformance: 1 adverse mutations refused"),
    ("checked_v2::tests::conformance_fr340_frame_mutations_match_fspec_vectors", "conformance: 1 frame-body mutation vectors matched"),
    ("checked_v2::tests::conformance_node_index_vectors_refuse_malformed_wire", "conformance: 1 node-index vector outcomes matched"),
    ("checked_v2::tests::conformance_dependency_selection_vectors", "conformance: 1 dependency-selection entry mutations and 1 order vectors"),
    ("emit::tests::golden::conformance_emitted_application_nodes_match_fspec_positive_fixtures", "conformance: 1 emitted application nodes match FSpec's positive fixtures"),
    ("complete_value_lock::conformance_admit_selection_matches_fspec_vectors", "conformance: 1 accepted and 1 refused selection vectors"),
    ("variant_type::conformance_clock_tuple_union_intake_vectors_are_admitted", "conformance: 1 Clock tuple/union intake vectors admitted"),
    ("checked_v2::tests::protocol_corpus::conformance_protocol_vector_positive_packages_are_admitted", "conformance: 2 protocol vector positive packages admitted"),
];
