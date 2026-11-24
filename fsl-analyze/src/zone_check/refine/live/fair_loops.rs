// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-899 "Timed subjects": the exact decision of a hidden-field refinement's
//! liveness half over every concrete fair loop of a timed component.
//!
//! A fair component holds many loops, and the fair run of one of them says
//! nothing of another. This is the explicit-state walk-summary search of the
//! untimed subject, read over the zone product's vertices: a closed walk's
//! summary holds its concrete fairness marks, which of the graph's
//! acceptance sets (a divergence, so a loop of positive total delay, and a
//! position) it meets, and the relation it induces on the hidden candidates.
//! A closed walk that meets every acceptance set and is concretely fair,
//! whose candidate relation has no fair cycle, is a refuting loop candidate.
//! Summaries are kept as an antichain under subsumption, so the search ends;
//! its work is charged as symbolic transitions, so a search that reaches the
//! budget stops and never holds.
//!
//! Each vertex's candidates are those of
//! [`Product::vertex_candidates`], the guard-aware reading the lasso check
//! uses too, so the summary and the candidate check read one enabledness.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_semantic_value::fairness::FairnessConstraint;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::fairness::{EdgeLabel, Identity, Vertex};
use fsl_replay::{AbstractClass, TemporalCounterexample};

use super::{Decided, FairScope, Live};
use crate::model_check::fair_summary::{
    candidate_steps_relation, candidate_stutter_relation, compose_candidate_relations,
    compose_fair_labels, concrete_label_is_fair, idempotent_has_fair_candidate,
    insert_relation_label, insert_walk_summary, neutral_label, CandidateEnables, CandidateRelation,
    FairLabel, FairMark, WalkSummary,
};
use crate::model_check::fair_walk::FairModel;
use crate::zone_check::refine::Product;
use crate::zone_check::search::{Halt, Run, ZoneCheckRefusal};
use fsl_replay::Taken;

/// A fair component as the search reads it.
pub(super) struct ComponentView<'a> {
    pub(super) model: &'a FairModel,
    /// The product's arcs kept from each vertex, in the graph's edge order.
    pub(super) kept: &'a [Vec<usize>],
    pub(super) members: &'a BTreeSet<Vertex>,
    /// The edge identities the component's refinement left out.
    pub(super) removed: &'a BTreeSet<Identity>,
}

/// What the search decided for one component.
pub(super) enum Refuted {
    /// A closed walk with no fair candidate run, concretised.
    By(TemporalCounterexample),
    /// Such walks exist and every one examined fails to concretise; walks
    /// under a prefix whose summary a retained one subsumes are not tried,
    /// so the half is undecided.
    NotConcretised,
    /// Every concrete fair loop has a fair candidate run.
    Not,
    /// The half ends with this outcome.
    Ends(Decided),
}

/// One hidden valuation of a vertex's position and the abstract transitions
/// enabled from it.
type Candidates = Vec<(Vec<u8>, Vec<ModelTransition>)>;

impl Product<'_, '_> {
    /// Decide `component` over every closed walk that is concretely fair and
    /// meets every acceptance set: a walk whose candidate relation has no
    /// fair cycle is refuting when its lasso concretises (FR-241), and the
    /// search goes on past one that does not. The search ends with the first
    /// that does, or with every retained walk examined.
    ///
    /// # Errors
    ///
    /// The run's budgets and cancellation, as a stop, and a fault when the
    /// summary and the candidate check of a walk disagree.
    pub(super) fn refute_hidden_component(
        &self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        component: &ComponentView<'_>,
        classes: &[AbstractClass],
        fairness: &[FairnessConstraint],
    ) -> Result<Refuted, Halt> {
        let mut candidates: BTreeMap<usize, Candidates> = BTreeMap::new();
        for vertex in component.members {
            match self.vertex_candidates(run, live, vertex.index())? {
                Ok(here) => {
                    candidates.insert(vertex.index(), here);
                }
                Err(ends) => return Ok(Refuted::Ends(ends)),
            }
        }
        let graph = &component.model.graph;
        let constraints = &component.model.constraints;
        let acceptance = graph.acceptance_sets();
        let through_all = FairScope::Through(fairness.len().saturating_sub(1));
        let mut relations: BTreeMap<usize, CandidateRelation> = BTreeMap::new();
        let mut not_concretised = false;
        for &start in component.members {
            run.poll()?;
            let mut identity = CandidateRelation::new();
            for (key, _) in vertex_of(&candidates, start.index())? {
                insert_relation_label(
                    &mut identity,
                    (key.clone(), key.clone()),
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
                            &enables(vertex_of(&candidates, arc.from)?),
                            &enables(vertex_of(&candidates, arc.to)?),
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
                            let summary_fair =
                                idempotent_has_fair_candidate(&idempotent, classes, fairness);
                            // The walk's own candidate graph decides the same.
                            let run_fair = match self.candidate_loop_has_fair_run(
                                run,
                                live,
                                &next_path,
                                through_all,
                            )? {
                                Ok(fair) => fair,
                                Err(ends) => return Ok(Refuted::Ends(ends)),
                            };
                            if summary_fair != run_fair {
                                return Err(ZoneCheckRefusal::Fault(
                                    "candidate-summary-agrees-with-lasso-check",
                                )
                                .into());
                            }
                            if !summary_fair {
                                match self.concretise_unfair_walk(
                                    run,
                                    live,
                                    classes,
                                    start.index(),
                                    &next_path,
                                )? {
                                    Refuted::By(found) => return Ok(Refuted::By(found)),
                                    Refuted::NotConcretised => not_concretised = true,
                                    Refuted::Not => {}
                                    ends @ Refuted::Ends(_) => return Ok(ends),
                                }
                            }
                        }
                    }
                    if insert_walk_summary(seen.entry(edge.target).or_default(), next.clone()) {
                        queue.push_back((edge.target, next, next_path));
                    }
                }
            }
        }
        Ok(if not_concretised {
            Refuted::NotConcretised
        } else {
            Refuted::Not
        })
    }

    /// The lasso of the closed walk `cycle` from `entry`, which no candidate
    /// run satisfies `F_A` along, named by FR-899's rule, when it concretises.
    fn concretise_unfair_walk(
        &self,
        run: &mut Run<'_, '_, '_>,
        live: &mut Live,
        classes: &[AbstractClass],
        entry: usize,
        cycle: &[usize],
    ) -> Result<Refuted, Halt> {
        let named = match self.first_unfair_constraint(run, live, cycle)? {
            Ok(index) => index,
            Err(ends) => return Ok(Refuted::Ends(ends)),
        };
        let class = classes
            .first()
            .ok_or(ZoneCheckRefusal::Fault("hidden-component-has-a-class"))?;
        Ok(match self.lasso(run, live, class, named, entry, cycle)? {
            Some(found) => Refuted::By(found),
            None => Refuted::NotConcretised,
        })
    }
}

/// The candidates of vertex `node`.
fn vertex_of(
    candidates: &BTreeMap<usize, Candidates>,
    node: usize,
) -> Result<&Candidates, ZoneCheckRefusal> {
    candidates
        .get(&node)
        .ok_or(ZoneCheckRefusal::Fault("component-candidates"))
}

fn enables(candidates: &Candidates) -> Vec<CandidateEnables<'_>> {
    candidates
        .iter()
        .map(|(key, enabled)| (key.as_slice(), enabled.as_slice()))
        .collect()
}

/// The pairs of source and target candidates an arc relates, with the fair
/// label of each: a step by its rows, a delay or a stutter by keeping the
/// candidate.
fn arc_relation(
    taken: Option<&Taken>,
    source: &[CandidateEnables<'_>],
    target: &[CandidateEnables<'_>],
    classes: &[AbstractClass],
) -> Result<CandidateRelation, ZoneCheckRefusal> {
    match taken {
        Some(Taken::CandidateSteps(rows)) => candidate_steps_relation(
            rows.iter().map(|row| {
                (
                    row.pre_state.as_slice(),
                    row.transition.as_ref(),
                    row.post_state.as_slice(),
                )
            }),
            source,
            target,
            classes,
        )
        .map_err(ZoneCheckRefusal::Fault),
        Some(Taken::Stutter | Taken::Failing) | None => Ok(candidate_stutter_relation(source, target, classes)),
        Some(Taken::Abstract(_)) => Err(ZoneCheckRefusal::Fault(
            "hidden-model-edge-retains-candidate-steps",
        )),
        Some(Taken::Protocol(_) | Taken::ProtocolTraces(_)) => Err(ZoneCheckRefusal::Fault(
            "hidden-model-edge-is-not-a-protocol-edge",
        )),
    }
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
