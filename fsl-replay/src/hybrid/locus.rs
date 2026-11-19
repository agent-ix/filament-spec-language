// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750: the FSpec FR-331 `Locus` of the term node an [`ExpressionLocus`]
//! names in the checked package, the locus an `unsupported` construct and an
//! `undefined-evaluation` counterexample cite.

use fsl_semantics::check::wire_id::WireGraph;
use fsl_semantics::check::{CheckedGraph, HybridTermHome};
use serde_json::Value;

use crate::hybrid_replay::{ExpressionLocus, ExpressionSite, TermLocus};

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

/// The inverse of [`hybrid_locus_wire`]: the expression site and term path of
/// the term node the FSpec FR-331 `Locus` `locus` names, among the
/// expressions of mode `mode` of the model the claim at position `claim`
/// ranges over: the rate of `variable` when it is given, and otherwise the
/// mode's invariant, guards, reset values and the initial conditions that
/// name it. `None` when the locus names no term node of those expressions.
pub fn hybrid_locus_site(
    graph: &CheckedGraph,
    wire: &WireGraph,
    claim: usize,
    mode: usize,
    variable: Option<usize>,
    locus: &Value,
) -> Option<ExpressionLocus> {
    let model_index = graph.hybrid_claims().get(claim)?.model.get();
    let model = graph.hybrid_models().get(model_index)?;
    let target = wire.key_of_value(locus.get("node")?).ok()?;
    let checked = model.modes.get(mode)?;
    let mut homes = Vec::new();
    if let Some(variable) = variable {
        homes.push((
            HybridTermHome::Rate {
                model: model_index,
                mode,
                variable,
            },
            ExpressionSite::Rate { mode, variable },
        ));
    } else {
        homes.push((
            HybridTermHome::Invariant {
                model: model_index,
                mode,
            },
            ExpressionSite::Invariant { mode },
        ));
        for (transition, jump) in checked.transitions.iter().enumerate() {
            homes.push((
                HybridTermHome::Guard {
                    model: model_index,
                    mode,
                    transition,
                },
                ExpressionSite::Guard { mode, transition },
            ));
            for constraint in 0..jump.resets.len() {
                homes.push((
                    HybridTermHome::Reset {
                        model: model_index,
                        mode,
                        transition,
                        constraint,
                    },
                    ExpressionSite::Reset {
                        mode,
                        transition,
                        constraint,
                    },
                ));
            }
        }
        for (entry, initial) in model.initial.iter().enumerate() {
            if initial.mode.get() == mode {
                homes.push((
                    HybridTermHome::Initial {
                        model: model_index,
                        entry,
                    },
                    ExpressionSite::Initial { entry },
                ));
            }
        }
    }
    homes.into_iter().find_map(|(home, site)| {
        graph
            .hybrid_term_path(home, &target)
            .map(|term| ExpressionLocus {
                site,
                term: TermLocus {
                    term,
                    body: Vec::new(),
                },
            })
    })
}
