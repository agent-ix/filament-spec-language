// SPDX-License-Identifier: AGPL-3.0-or-later
//! The consumer cannot reach package internals through a public module.
//!
//! A consumer can derive a protocol clause's causal relation from an admitted package:
//!
//! ```
//! use fsl_package::protocol_view::{causal_relation, CausalRelation, ViewRefusal};
//! use filament_contract_model::{CheckedNodeId, CheckedPackageV2};
//!
//! fn relation(
//!     package: &CheckedPackageV2,
//!     clause: &CheckedNodeId,
//! ) -> Result<CausalRelation, ViewRefusal> {
//!     causal_relation(package, clause)
//! }
//! ```
//!
//! ```compile_fail,E0603
//! fn private_relation_helper_is_not_a_consumer_api() {
//!     let _ = fsl_package::protocol_view::relation_over;
//! }
//! ```
