// SPDX-License-Identifier: AGPL-3.0-or-later
//! refinement integration tests.

mod analyze_cli_support;
#[allow(
    dead_code,
    reason = "shared fixture support spans split integration test binaries"
)]
mod counter;
mod dense_route_support;
mod derived_request_support;
mod fill;
mod fixture_ids;
mod flag_world;
mod graph_world;
mod hyper_world;
mod letters;
mod litmus;
mod model_settle_support;
mod partial_order;
mod por_world;
mod prob_world;
mod protocol_footprint_support;
mod refinement_protocol_support;
mod relation_claims_support;
mod source_value_support;
mod units;
mod weak_memory_support;

mod load_buffering;
mod operation_contract;
#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;
mod quotient;
mod race_freedom;
mod rc11_oracle;
#[path = "../support/rc11.rs"]
mod rc11_support;
mod reduction;
mod reduction_differential;
mod reduction_manifest;
mod reduction_proptest;
mod reduction_world;
#[path = "../../../tests/support/refinement_aux.rs"]
mod refinement_aux;
mod refinement_auxiliary;
mod refinement_budget;
mod refinement_certificate;
#[path = "../../../tests/support/refinement_cas.rs"]
mod refinement_fixture;
mod refinement_liveness;
mod refinement_liveness_support;
mod refinement_protocol;
mod refinement_reads;
mod refinement_replay;
mod refinement_request;
mod refinement_safety;
mod relation_certificate;
mod relation_vs_smt;
mod release_acquire;
#[path = "../../../tests/support/ring_world.rs"]
mod ring_world;
mod smt_query;
mod source_model_boundaries;
mod tied_item_resolution;
mod weak_memory;
