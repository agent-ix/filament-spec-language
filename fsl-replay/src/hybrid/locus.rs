// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750: the FSpec FR-331 `Locus` of the term node an [`ExpressionLocus`]
//! names in the checked package, the locus an `unsupported` construct and an
//! `undefined-evaluation` counterexample cite.

use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::{CheckedGraph, HybridTermHome};
use serde_json::Value;

use crate::hybrid_replay::{ExpressionLocus, ExpressionSite};

/// The `{node, occurrence}` locus of the term node `locus` names in the
/// claim at position `claim` of `graph`, with the node spelled by `wire`.
/// `None` when the package has no such claim, expression or term.
pub fn hybrid_locus_wire(
    graph: &CheckedGraph,
    wire: &WireGraph,
    claim: usize,
    locus: &ExpressionLocus,
) -> Option<Value> {
    let model = graph.hybrid_claims().get(claim)?.model.get();
    let home = match locus.site {
        ExpressionSite::Initial { entry } => HybridTermHome::Initial { model, entry },
        ExpressionSite::Invariant { mode } => HybridTermHome::Invariant { model, mode },
        ExpressionSite::Rate { mode, variable } => HybridTermHome::Rate {
            model,
            mode,
            variable,
        },
        ExpressionSite::Guard { mode, transition } => HybridTermHome::Guard {
            model,
            mode,
            transition,
        },
        ExpressionSite::Reset {
            mode,
            transition,
            constraint,
        } => HybridTermHome::Reset {
            model,
            mode,
            transition,
            constraint,
        },
        ExpressionSite::Activation => HybridTermHome::Activation { claim },
        ExpressionSite::Capture { capture } => HybridTermHome::Capture { claim, capture },
        ExpressionSite::ClaimAtom { atom } => HybridTermHome::ClaimAtom { claim, atom },
    };
    let found = graph.hybrid_term_node(home, &locus.term.term)?;
    Some(serde_json::json!({
        "node": wire.wire_value(&found.node).ok()?,
        "occurrence": {
            "role": found.origin.role().as_str(),
            "ordinal": found.origin.ordinal(),
        },
    }))
}
