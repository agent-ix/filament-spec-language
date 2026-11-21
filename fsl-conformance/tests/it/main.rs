// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests of the lifecycle conformance matrix.

mod analyze_row;
mod catalog_coverage;
mod execute_row;
mod front_end_rows;
mod fspec_finite_window;
mod fspec_vectors;
mod matrix_shape;
mod monitor_row;
mod plugin_invariance;
mod prove_row;
#[path = "../../../tests/support/published_fspec.rs"]
pub(crate) mod published_fspec;
mod support;
mod type_decision_cases;
