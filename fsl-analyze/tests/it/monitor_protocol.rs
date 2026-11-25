// SPDX-License-Identifier: AGPL-3.0-or-later
//! monitor_protocol integration tests.

mod analyze_cli_support;
mod counter;
mod fixture_ids;
mod litmus;
mod units;

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
mod provider_request_seam;
#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;
mod recording_population;
