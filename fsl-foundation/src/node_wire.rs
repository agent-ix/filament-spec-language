// SPDX-License-Identifier: AGPL-3.0-or-later
//! Re-export the shared exact structural node-identity wire codec, plus the
//! FSL-specific borrowed projections retained for existing consumers.

pub use crate::node_identity::{
    encode_node_key, project_node_key, NodeIdentitySink, TextNodeIdentity,
};
pub use filament_exact::node_wire::*;
