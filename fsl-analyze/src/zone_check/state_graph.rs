// SPDX-License-Identifier: AGPL-3.0-or-later
//! State-graph forms over the symbolic timed graph (FSpec FR-712).
//!
//! The claim's predicates read model state, so they are evaluated per node
//! here; what they decide over the timed states is
//! [`super::state_graph_zones`]'s.

use filament_exact::{Cancel, ObjectReference};
use fsl_eval::simulation::model::{ModelSystem, PredicateReading, TimedSystem};
use fsl_replay::{ModelPath, ModelStep};
use fsl_semantics::check::{
    CheckedStateGraphClaim, CheckedStatePredicate, PredicateRole, StateGraphForm,
};
use ix_trace_rs::implements;

use super::search::{TimedStateGraph, ZoneCheckLimits, ZoneCheckRefusal};
use super::state_graph_zones::Zones;

/// A state-graph decision over a timed zone graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedStateGraphDecision {
    /// The claim holds. `witness_paths` is populated for `possible`; `ef_nodes`
    /// is populated for `always possible`.
    Holds {
        /// The concretised witness path for each initial state.
        witness_paths: Vec<ModelPath>,
        /// The nodes whose whole zone lies in `E(P)`.
        ef_nodes: Vec<usize>,
    },
    /// The claim is refuted at one initial or reachable timed state.
    Refuted {
        /// The path to the deciding timed state, which may be reached from
        /// the path's end by waiting.
        stem: ModelPath,
        /// The node that holds the deciding timed state.
        node: usize,
        /// The nodes whose whole zone lies in `E(P)`, when the form is
        /// `always possible`.
        ef_nodes: Vec<usize>,
        /// Two distinct paths from the stem's end for a `unique path`
        /// refutation; absent when no path reaches the target.
        paths: Option<(Vec<ModelStep>, Vec<ModelStep>)>,
    },
}

/// Decide a state-graph claim over the reachable zone graph.
///
/// # Errors
///
/// [`ZoneCheckRefusal`] when evaluating a predicate does not produce a
/// Boolean reading, when a path does not concretise, and
/// [`ZoneCheckRefusal::Stopped`] when `limits` or `cancel` stops the
/// decision, which then has no verdict.
#[implements("FR-852-AC-1", "FR-852-AC-2", "FR-852-AC-3")]
pub fn decide_timed_state_graph(
    timed: &TimedSystem<'_, '_>,
    graph: &TimedStateGraph,
    claim: &CheckedStateGraphClaim,
    over: Option<&ObjectReference>,
    limits: ZoneCheckLimits,
    cancel: &Cancel,
) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
    let system = timed.system();
    let mut zones = Zones::new(timed, graph, limits, cancel)?;
    let target = match claim.form() {
        StateGraphForm::Possible { target } | StateGraphForm::AlwaysPossible { target, .. } => {
            target
        }
        StateGraphForm::UniquePath { .. } => {
            return decide_unique_path(system, &mut zones, graph, claim, over, cancel);
        }
    };
    let p = graph
        .nodes
        .iter()
        .map(|node| {
            predicate(
                system,
                target,
                claim,
                PredicateRole::Target,
                &node.state,
                over,
                cancel,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    match claim.form() {
        StateGraphForm::Possible { .. } => zones.possible(&p),
        StateGraphForm::AlwaysPossible { from, .. } => {
            let q = graph
                .nodes
                .iter()
                .map(|node| {
                    predicate(
                        system,
                        from,
                        claim,
                        PredicateRole::From,
                        &node.state,
                        over,
                        cancel,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            zones.always_possible(&q, &p)
        }
        StateGraphForm::UniquePath { .. } => unreachable!("handled above"),
    }
}

fn decide_unique_path(
    system: &ModelSystem<'_>,
    zones: &mut Zones<'_, '_, '_>,
    graph: &TimedStateGraph,
    claim: &CheckedStateGraphClaim,
    over: Option<&ObjectReference>,
    cancel: &Cancel,
) -> Result<TimedStateGraphDecision, ZoneCheckRefusal> {
    let StateGraphForm::UniquePath { from, to } = claim.form() else {
        return Err(ZoneCheckRefusal::Fault("state-graph-form"));
    };
    let x = graph
        .nodes
        .iter()
        .map(|node| {
            predicate(
                system,
                from,
                claim,
                PredicateRole::From,
                &node.state,
                over,
                cancel,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let y = graph
        .nodes
        .iter()
        .map(|node| {
            predicate(
                system,
                to,
                claim,
                PredicateRole::Target,
                &node.state,
                over,
                cancel,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    zones.unique_path(&x, &y)
}

fn predicate(
    system: &ModelSystem<'_>,
    predicate: &CheckedStatePredicate,
    claim: &CheckedStateGraphClaim,
    role: PredicateRole,
    state: &fsl_eval::simulation::model::ModelSimState,
    over: Option<&ObjectReference>,
    cancel: &Cancel,
) -> Result<bool, ZoneCheckRefusal> {
    if predicate.is_constant_true() {
        return Ok(true);
    }
    let readings = if let Some(body) = predicate.predicate_body() {
        system.state_predicates_with_cancel(state, &[(body, over)], cancel)
    } else {
        system.state_graph_predicates(state, &[(claim, role, over)])
    }
    .map_err(|stop| ZoneCheckRefusal::Atom(format!("{stop:?}")))?;
    match readings.as_slice() {
        [PredicateReading::True] => Ok(true),
        [PredicateReading::False] => Ok(false),
        [PredicateReading::Undefined { .. }] => {
            Err(ZoneCheckRefusal::Fault("state-graph-undefined-predicate"))
        }
        [PredicateReading::Refused(_)] => Err(ZoneCheckRefusal::Fault("state-graph-predicate")),
        _ => Err(ZoneCheckRefusal::Fault("state-graph-predicate-count")),
    }
}
