// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-375 refinement node identity on the S4 wire.

use crate::refinement_declarations::{cas_refines_counter, checked_counter, only_refinement};
use filament_exact::PathSegment;
use ix_trace_rs::trace;

/// FSpec-FR-375-AC-1: an admitted refinement is emitted as a
/// `relation`/`refinement` node.
#[trace("FSpec-FR-375-AC-1")]
#[test]
fn refinement_node_identity_uses_the_relation_refinement_path() {
    let graph = checked_counter(&cas_refines_counter());
    let emission = only_refinement(&graph)
        .v2_emission()
        .expect("a checked refinement has its S4 node");

    assert_eq!(
        emission.identity.path(),
        [
            PathSegment::Text("relation".to_owned()),
            PathSegment::Text("refinement".to_owned()),
            PathSegment::Text("CasRefinesCounter".to_owned()),
        ],
        "FSpec FR-375 keys a refinement by its relation/refinement path"
    );
}
