// SPDX-License-Identifier: AGPL-3.0-or-later
//! monitor_protocol integration tests.

mod hidden_choice_source;
mod monitor;
mod monitor_aggregate_rendering;
mod monitor_authority;
mod monitor_entries;
mod monitor_incremental;
mod monitor_inputs;
mod monitor_plan;
mod monitor_plan_vectors;
mod monitor_root;
mod protocol_analysis_differential;
mod protocol_bounded_lasso;
mod protocol_compensation;
mod protocol_composition;
mod protocol_conformance;
mod protocol_constructs;
mod protocol_counted_steps;
mod protocol_docs_examples;
mod protocol_footprints;
mod protocol_item_bounds;
mod protocol_outcomes;
mod protocol_projection;
mod protocol_realizability;
mod protocol_recording;
mod protocol_replay_mode;
mod protocol_roles;
mod protocol_step_view;
mod protocol_system;
mod provider_external_contract;
mod provider_request_seam;
mod provision_contract;
#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;
mod recording_population;
mod scheduler_fairness;
