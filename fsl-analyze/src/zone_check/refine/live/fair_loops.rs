// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-899 "Timed subjects": the exact decision of a hidden-field refinement's
//! liveness half over every concrete fair loop of a timed component.
//!
//! A fair component holds many loops, and the fair run of one of them says
//! nothing of another (FR-899-AC-7). This is the explicit-state walk-summary
//! search of the untimed subject, read over the zone product's vertices: a
//! closed walk's summary holds its concrete fairness marks, which of the
//! graph's acceptance sets (a divergence, so a loop of positive total
//! delay, and a position) it meets, and the relation it induces on the
//! hidden candidates. A closed walk that meets every acceptance set and is
//! concretely fair, whose candidate relation has no fair cycle, refutes the
//! refinement. Summaries are kept as an antichain under subsumption, so the
//! search ends; the budget is charged as symbolic transitions, so a search
//! that reaches it stops and never holds.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::fairness::FairnessConstraint;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::fairness::{EdgeLabel, Identity, Vertex};
use fsl_eval::simulation::model::ModelFinding;
use fsl_eval::simulation::ExpansionLimit;
use fsl_replay::{AbstractClass, ModelCheckLimit, ReachedLimit, Taken, Undetermined};

use super::super::{Product, TimedRefinementStop, TimedRefinementUndecided};
use super::{Decided, FairScope, Live};
use crate::model_check::fair_summary::{
    compose_candidate_relations, compose_fair_labels, concrete_label_is_fair,
    idempotent_has_fair_candidate, insert_relation_label, insert_walk_summary, neutral_label,
    CandidateRelation, FairLabel, FairMark, WalkSummary,
};
use crate::model_check::fair_walk::FairModel;
use crate::zone_check::search::{Halt, Run, ZoneCheckRefusal};

/// A fair component as the search reads it.
pub(super) struct ComponentView<'a> {
    pub(super) model: &'a FairModel,
    /// The product's arcs kept from each vertex, in the graph's edge order.
    pub(super) kept: &'a [Vec<usize>],
    pub(super) members: &'a BTreeSet<Vertex>,
    /// The edge identities the component's refinement left out.
    pub(super) removed: &'a BTreeSet<Identity>,
}

/// A closed walk the abstract subject has no fair run along: the vertex it
/// starts and ends at and its arcs.
pub(super) struct UnfairLoop {
    pub(super) entry: usize,
    pub(super) cycle: Vec<usize>,
}

/// One hidden valuation of a refinement position and the abstract
/// transitions enabled from it.
struct Candidate {
    key: Vec<u8>,
    enabled: Vec<ModelTransition>,
}

impl Product<'_, '_> {
    /// The first closed walk of `component` that is concretely fair, meets
    /// every acceptance set and has no fair candidate run, or `None` when
    /// every such walk has one.
    ///
    /// # Errors
    ///
    /// The run's budgets and cancellation, as a stop, and a fault when the
    /// product disagrees with the candidate check of the walk found.
    pub(super) fn unfair_candidate_loop(
        &self,
        run: &mut Run<'_, '_, '_>,
        live: &Live,
        component: &ComponentView<'_>,
        classes: &[AbstractClass],
        fairness: &[FairnessConstraint],
    ) -> Result<Result<Option<UnfairLoop>, Decided>, Halt> {
        let mut candidates = BTreeMap::new();
        for vertex in component.members {
            let q = live
                .nodes
                .get(vertex.index())
                .ok_or(ZoneCheckRefusal::Fault("component-vertex"))?
                .q;
            if let std::collections::btree_map::Entry::Vacant(slot) = candidates.entry(q) {
                match self.candidates_at(run, q)? {
                    Ok(here) => {
                        slot.insert(here);
                    }
                    Err(ends) => return Ok(Err(ends)),
                }
            }
        }
        let graph = &component.model.graph;
        let constraints = &component.model.constraints;
        let acceptance = graph.acceptance_sets();
        let mut relations: BTreeMap<usize, CandidateRelation> = BTreeMap::new();
        for &start in component.members {
            run.poll()?;
            let start_candidates = candidates_of(live, &candidates, start.index())?;
            let mut identity = CandidateRelation::new();
            for candidate in start_candidates {
                insert_relation_label(
                    &mut identity,
                    (candidate.key.clone(), candidate.key.clone()),
                    neutral_label(classes.len()),
                );
            }
            let neutral = WalkSummary {
                concrete: neutral_label(constraints.len() + acceptance.len()),
                candidates: identity,
            };
            let mut seen: BTreeMap<Vertex, Vec<WalkSummary>> = BTreeMap::new();
            seen.entry(start).or_default().push(neutral.clone());
            let mut queue = VecDeque::from([(start, neutral, Vec::<usize>::new())]);
            while let Some((at, summary, path)) = queue.pop_front() {
                run.poll()?;
                let enabled = graph.enabled(at).cloned().unwrap_or_default();
                for (position, edge) in graph.edges_from(at).iter().enumerate() {
                    let removed = matches!(
                        edge.label,
                        EdgeLabel::Transition(identity) if component.removed.contains(&identity)
                    );
                    if removed || !component.members.contains(&edge.target) {
                        continue;
                    }
                    let arc_index = component
                        .kept
                        .get(at.index())
                        .and_then(|out| out.get(position))
                        .copied()
                        .ok_or(ZoneCheckRefusal::Fault("walk-edge"))?;
                    if !relations.contains_key(&arc_index) {
                        let arc = live
                            .arcs
                            .get(arc_index)
                            .ok_or(ZoneCheckRefusal::Fault("walk-edge"))?;
                        let relation = arc_relation(
                            arc.taken.as_ref(),
                            candidates_of(live, &candidates, arc.from)?,
                            candidates_of(live, &candidates, arc.to)?,
                            classes,
                        )?;
                        relations.insert(arc_index, relation);
                    }
                    let edge_relation = relations
                        .get(&arc_index)
                        .ok_or(ZoneCheckRefusal::Fault("walk-edge"))?;
                    let (candidate_relation, compositions) =
                        compose_candidate_relations(&summary.candidates, edge_relation);
                    charge(run, compositions.saturating_add(1))?;
                    if candidate_relation.is_empty() {
                        continue;
                    }
                    let edge_label: FairLabel = constraints
                        .iter()
                        .map(|constraint| {
                            let here = constraint.is_enabled_in(&enabled).unwrap_or(false);
                            FairMark {
                                taken: constraint.is_taken_by(edge.label),
                                enabled_somewhere: here,
                                enabled_everywhere: here,
                            }
                        })
                        .chain(acceptance.iter().map(|set| FairMark {
                            taken: set.contains(&at),
                            enabled_somewhere: false,
                            enabled_everywhere: true,
                        }))
                        .collect();
                    let next = WalkSummary {
                        concrete: compose_fair_labels(&summary.concrete, &edge_label),
                        candidates: candidate_relation,
                    };
                    let mut next_path = path.clone();
                    next_path.push(arc_index);
                    if edge.target == start {
                        let (fair, accepted) = next.concrete.split_at(constraints.len());
                        if concrete_label_is_fair(fair, constraints)
                            && accepted.iter().all(|mark| mark.taken)
                        {
                            let idempotent = idempotent_power(run, &next.candidates)?;
                            if !idempotent_has_fair_candidate(&idempotent, classes, fairness) {
                                // The walk's own candidate graph agrees.
                                let fair_run = self.candidate_loop_has_fair_run(
                                    run,
                                    live,
                                    &next_path,
                                    FairScope::Through(fairness.len().saturating_sub(1)),
                                )?;
                                return match fair_run {
                                    Ok(false) => Ok(Ok(Some(UnfairLoop {
                                        entry: start.index(),
                                        cycle: next_path,
                                    }))),
                                    Ok(true) => Err(ZoneCheckRefusal::Fault(
                                        "candidate-summary-agrees-with-lasso-check",
                                    )
                                    .into()),
                                    Err(ends) => Ok(Err(ends)),
                                };
                            }
                        }
                    }
                    if insert_walk_summary(seen.entry(edge.target).or_default(), next.clone()) {
                        queue.push_back((edge.target, next, next_path));
                    }
                }
            }
        }
        Ok(Ok(None))
    }

    /// The hidden valuations of position `q` with the abstract transitions
    /// enabled from each.
    ///
    /// This reads the untimed abstract state only, which is all the
    /// liveness half admits: `check_timed_refinement` refuses it for a hidden
    /// field under a timed abstract side. The seam for a provider of
    /// per-candidate clock valuations and guard-split enabledness is the
    /// prefill of `candidates` in `unfair_candidate_loop`, which the rest of
    /// the search reads only through `candidates_of`, by vertex.
    fn candidates_at(
        &self,
        run: &Run<'_, '_, '_>,
        q: usize,
    ) -> Result<Result<Vec<Candidate>, Decided>, Halt> {
        let mut here = Vec::new();
        for state in &self.positions[q].abstract_states {
            run.poll()?;
            let (key, _) = self
                .abstract_system
                .state_key_of(state)
                .map_err(|_| ZoneCheckRefusal::Fault("candidate-loop-state-key"))?;
            let expansion = match self.abstract_system.expand_state(state) {
                Ok(expansion) => expansion,
                Err(stop) if stop.is_internal_failure() => {
                    return Ok(Err(Decided::Fault {
                        fault: InternalFault::new("refinement-liveness", "abstract-expansion"),
                        stats: run.stats(),
                    }));
                }
                Err(stop) => {
                    let reached = match stop.limit {
                        Some(ExpansionLimit::EvaluationMeter(counter)) => ReachedLimit {
                            limit: ModelCheckLimit::EvaluationMeter { counter },
                            value: fsl_replay::meter_bound(&self.abstract_system.meter(), counter),
                        },
                        Some(ExpansionLimit::MaxCandidates) | None => ReachedLimit {
                            limit: ModelCheckLimit::MaxCandidates,
                            value: self.abstract_system.max_candidates(),
                        },
                    };
                    return Ok(Err(Decided::Stopped {
                        cause: TimedRefinementStop::Step(reached),
                        stats: run.stats(),
                    }));
                }
            };
            if expansion
                .findings
                .iter()
                .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
            {
                return Ok(Err(Decided::Undecided {
                    cause: TimedRefinementUndecided::Undetermined(Undetermined::UndecidedSuccessor),
                    stats: run.stats(),
                }));
            }
            here.push(Candidate {
                key,
                enabled: expansion
                    .successors
                    .into_iter()
                    .map(|(transition, _)| transition)
                    .collect(),
            });
        }
        Ok(Ok(here))
    }
}

/// The candidates of vertex `node`'s position.
fn candidates_of<'a>(
    live: &Live,
    candidates: &'a BTreeMap<usize, Vec<Candidate>>,
    node: usize,
) -> Result<&'a [Candidate], ZoneCheckRefusal> {
    let q = live
        .nodes
        .get(node)
        .ok_or(ZoneCheckRefusal::Fault("component-vertex"))?
        .q;
    candidates
        .get(&q)
        .map(Vec::as_slice)
        .ok_or(ZoneCheckRefusal::Fault("component-candidates"))
}

/// The pairs of source and target candidates an arc relates, with the fair
/// label of each: a step by its rows, a delay or a stutter by keeping the
/// candidate.
fn arc_relation(
    taken: Option<&Taken>,
    source: &[Candidate],
    target: &[Candidate],
    classes: &[AbstractClass],
) -> Result<CandidateRelation, ZoneCheckRefusal> {
    let mark = |candidate: &Candidate, taken: &dyn Fn(&AbstractClass) -> bool| -> FairLabel {
        classes
            .iter()
            .map(|class| {
                let enabled = class.is_enabled_in(&candidate.enabled);
                FairMark {
                    taken: taken(class),
                    enabled_somewhere: enabled,
                    enabled_everywhere: enabled,
                }
            })
            .collect()
    };
    let mut relation = CandidateRelation::new();
    match taken {
        Some(Taken::CandidateSteps(rows)) => {
            for row in rows {
                let candidate = source
                    .iter()
                    .find(|candidate| candidate.key == row.pre_state)
                    .ok_or(ZoneCheckRefusal::Fault("candidate-step-source"))?;
                if !target
                    .iter()
                    .any(|candidate| candidate.key == row.post_state)
                {
                    return Err(ZoneCheckRefusal::Fault("candidate-step-target"));
                }
                let label = mark(candidate, &|class| {
                    row.transition
                        .as_ref()
                        .is_some_and(|transition| class.covers(transition))
                });
                insert_relation_label(
                    &mut relation,
                    (row.pre_state.clone(), row.post_state.clone()),
                    label,
                );
            }
        }
        Some(Taken::Stutter) | None => {
            for candidate in source {
                if target.iter().any(|held| held.key == candidate.key) {
                    let label = mark(candidate, &|_| false);
                    insert_relation_label(
                        &mut relation,
                        (candidate.key.clone(), candidate.key.clone()),
                        label,
                    );
                }
            }
        }
        Some(Taken::Abstract(_)) => {
            return Err(ZoneCheckRefusal::Fault(
                "hidden-model-edge-retains-candidate-steps",
            ));
        }
        Some(Taken::Protocol(_) | Taken::ProtocolTraces(_)) => {
            return Err(ZoneCheckRefusal::Fault(
                "hidden-model-edge-is-not-a-protocol-edge",
            ));
        }
    }
    Ok(relation)
}

/// The least power of `relation` that is idempotent, charged and polled as
/// the run requires.
fn idempotent_power(
    run: &mut Run<'_, '_, '_>,
    relation: &CandidateRelation,
) -> Result<CandidateRelation, Halt> {
    idempotent_power_by(relation, |amount| {
        run.poll()?;
        charge(run, amount)
    })
}

/// The least power `R^k`, `k >= 1`, of `relation` with `R^k . R^k = R^k`,
/// calling `pay` with the compositions each step costs before it is taken.
///
/// The relations over a finite candidate set and finite labels are a finite
/// monoid, so the sequence `R, R^2, R^3, ...` is eventually periodic and
/// some power in it is idempotent, which this reaches. Squaring instead
/// (`R, R^2, R^4, ...`) need not: over a candidate 3-cycle `R^4 = R`, so it
/// alternates between `R` and `R^2` and never meets `R^3`.
fn idempotent_power_by<E>(
    relation: &CandidateRelation,
    mut pay: impl FnMut(usize) -> Result<(), E>,
) -> Result<CandidateRelation, E> {
    let mut power = relation.clone();
    loop {
        let (squared, work) = compose_candidate_relations(&power, &power);
        pay(work.max(1))?;
        if squared == power {
            return Ok(power);
        }
        let (next, work) = compose_candidate_relations(&power, relation);
        pay(work.max(1))?;
        power = next;
    }
}

/// Charge `amount` compositions as symbolic transitions, which stops the
/// run at `max_symbolic_transitions`.
fn charge(run: &mut Run<'_, '_, '_>, amount: usize) -> Result<(), Halt> {
    for _ in 0..amount {
        run.charge_edge()?;
        run.escape_edge();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn cycle_of_three() -> CandidateRelation {
        let mut relation = CandidateRelation::new();
        for (from, to) in [(b"a", b"b"), (b"b", b"c"), (b"c", b"a")] {
            insert_relation_label(
                &mut relation,
                (from.to_vec(), to.to_vec()),
                neutral_label(1),
            );
        }
        relation
    }

    /// FR-899: over a candidate 3-cycle with equal fair labels the powers by
    /// squaring alternate between `R` and `R^2` for ever; consecutive powers
    /// reach `R^3`, the identity on the three candidates.
    #[trace("TC-2536")]
    #[test]
    fn a_candidate_three_cycle_reaches_its_idempotent_power() {
        let relation = cycle_of_three();
        let idempotent =
            idempotent_power_by(&relation, |_| Ok::<(), ()>(())).expect("no budget to exhaust");
        let pairs: Vec<_> = idempotent.keys().cloned().collect();
        assert_eq!(
            pairs,
            [
                (b"a".to_vec(), b"a".to_vec()),
                (b"b".to_vec(), b"b".to_vec()),
                (b"c".to_vec(), b"c".to_vec()),
            ]
        );
    }

    /// FR-899: the power search pays for every composition and ends with the
    /// payer's refusal when it cannot.
    #[trace("TC-2536")]
    #[test]
    fn the_power_search_ends_when_its_budget_does() {
        let mut left = 4_usize;
        let ended = idempotent_power_by(&cycle_of_three(), |amount| {
            left = left.checked_sub(amount).ok_or("budget")?;
            Ok(())
        });
        assert_eq!(ended, Err("budget"));
    }
}
