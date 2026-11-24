// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests of the lifecycle conformance matrix.

mod admitted_unit_equivalences;
mod analyze_row;
mod call_replay_package_binding;
mod call_replay_root_consumer;
mod catalog_coverage;
mod effective_declaration_preimages;
mod execute_row;
mod front_end_rows;
mod fspec_finite_window;
mod fspec_pending;
mod fspec_standard;
mod fspec_state_core;
mod fspec_vectors;
mod hyper_product_identity;
mod hyper_smt_encoding_limits;
mod matrix_shape;
mod monitor_row;
mod plugin_invariance;
mod prove_row;
#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;
mod s2_clock_quantity_type_positions;
mod support;
mod temporal_clock_identity;
mod temporal_evaluation;
mod timed_interval_stage_boundaries;
mod tso_certificate;
mod type_decision_cases;
mod unsupported_entry_contract;
#[path = "../../../fsl-replay/tests/it/vault_world.rs"]
mod vault_world;
