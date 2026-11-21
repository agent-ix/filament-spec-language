// SPDX-License-Identifier: AGPL-3.0-or-later
//! The walk-summary algebra FR-899 decides a hidden-field refinement's
//! liveness half with (ADR-020 AX-5, AX-6): the per-label fairness marks a
//! walk accumulates, the candidate relation a walk induces on the hidden
//! valuations, and the subsumption order the retained summaries keep an
//! antichain under. The explicit-state product and the timed zone product
//! each search their own walks and share this algebra.

use std::collections::BTreeMap;

use filament_semantic_value::fairness::{FairnessConstraint, FairnessKind};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::fairness::{Constraint, Strength};
use fsl_replay::AbstractClass;

/// One transition-label summary component: taken on some edge, enabled at
/// some source, and enabled at every source.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct FairMark {
    pub(crate) taken: bool,
    pub(crate) enabled_somewhere: bool,
    pub(crate) enabled_everywhere: bool,
}

pub(crate) type FairLabel = Vec<FairMark>;
pub(crate) type CandidatePair = (Vec<u8>, Vec<u8>);
pub(crate) type CandidateRelation = BTreeMap<CandidatePair, Vec<FairLabel>>;

/// The concrete and candidate-path summaries of one retained walk.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct WalkSummary {
    pub(crate) concrete: FairLabel,
    pub(crate) candidates: CandidateRelation,
}

pub(crate) fn neutral_label(constraints: usize) -> FairLabel {
    vec![
        FairMark {
            taken: false,
            enabled_somewhere: false,
            enabled_everywhere: true,
        };
        constraints
    ]
}

pub(crate) fn compose_fair_labels(left: &[FairMark], right: &[FairMark]) -> FairLabel {
    left.iter()
        .zip(right)
        .map(|(left, right)| FairMark {
            taken: left.taken || right.taken,
            enabled_somewhere: left.enabled_somewhere || right.enabled_somewhere,
            enabled_everywhere: left.enabled_everywhere && right.enabled_everywhere,
        })
        .collect()
}

pub(crate) fn at_least_as_fair(left: &[FairMark], right: &[FairMark]) -> bool {
    left.iter().zip(right).all(|(left, right)| {
        (!right.taken || left.taken)
            && (!left.enabled_somewhere || right.enabled_somewhere)
            && (!left.enabled_everywhere || right.enabled_everywhere)
    })
}

pub(crate) fn insert_relation_label(
    relation: &mut CandidateRelation,
    pair: CandidatePair,
    label: FairLabel,
) {
    let labels = relation.entry(pair).or_default();
    if labels
        .iter()
        .any(|existing| at_least_as_fair(existing, &label))
    {
        return;
    }
    labels.retain(|existing| !at_least_as_fair(&label, existing));
    labels.push(label);
    labels.sort();
}

pub(crate) fn compose_candidate_relations(
    left: &CandidateRelation,
    right: &CandidateRelation,
) -> (CandidateRelation, usize) {
    let mut composed = CandidateRelation::new();
    let mut work = 0_usize;
    for ((start, middle), left_labels) in left {
        for ((next_middle, end), right_labels) in right {
            if middle != next_middle {
                continue;
            }
            for left_label in left_labels {
                for right_label in right_labels {
                    work = work.saturating_add(1);
                    insert_relation_label(
                        &mut composed,
                        (start.clone(), end.clone()),
                        compose_fair_labels(left_label, right_label),
                    );
                }
            }
        }
    }
    (composed, work)
}

pub(crate) fn relation_subsumes(left: &CandidateRelation, right: &CandidateRelation) -> bool {
    left.iter().all(|(pair, left_labels)| {
        right.get(pair).is_some_and(|right_labels| {
            left_labels.iter().all(|left_label| {
                right_labels
                    .iter()
                    .any(|right_label| at_least_as_fair(right_label, left_label))
            })
        })
    })
}

pub(crate) fn walk_summary_subsumes(left: &WalkSummary, right: &WalkSummary) -> bool {
    at_least_as_fair(&left.concrete, &right.concrete)
        && relation_subsumes(&left.candidates, &right.candidates)
}

pub(crate) fn insert_walk_summary(summaries: &mut Vec<WalkSummary>, summary: WalkSummary) -> bool {
    if summaries
        .iter()
        .any(|existing| walk_summary_subsumes(existing, &summary))
    {
        return false;
    }
    summaries.retain(|existing| !walk_summary_subsumes(&summary, existing));
    summaries.push(summary);
    true
}

pub(crate) fn concrete_label_is_fair(label: &[FairMark], constraints: &[Constraint]) -> bool {
    label
        .iter()
        .zip(constraints)
        .all(|(mark, constraint)| match constraint.strength() {
            Strength::Weak => mark.taken || !mark.enabled_everywhere,
            Strength::Strong => mark.taken || !mark.enabled_somewhere,
        })
}

/// The abstract constraints a candidate run must satisfy: the first
/// `through + 1` of them jointly, or one alone.
#[derive(Clone, Copy)]
pub(crate) enum FairScope {
    Through(usize),
    Only(usize),
}

impl FairScope {
    pub(crate) fn covers(self, constraint: usize) -> bool {
        match self {
            Self::Through(last) => constraint <= last,
            Self::Only(only) => constraint == only,
        }
    }
}

pub(crate) fn candidate_label_is_fair(
    label: &[FairMark],
    classes: &[AbstractClass],
    fairness: &[FairnessConstraint],
    scope: FairScope,
) -> bool {
    label.iter().zip(classes).all(|(mark, class)| {
        if !scope.covers(class.constraint()) {
            return true;
        }
        match fairness
            .get(class.constraint())
            .map(FairnessConstraint::kind)
        {
            Some(FairnessKind::Weak) => mark.taken || !mark.enabled_everywhere,
            Some(FairnessKind::Strong) => mark.taken || !mark.enabled_somewhere,
            None => false,
        }
    })
}

pub(crate) fn idempotent_has_fair_candidate(
    relation: &CandidateRelation,
    classes: &[AbstractClass],
    fairness: &[FairnessConstraint],
) -> bool {
    idempotent_has_fair_candidate_in(
        relation,
        classes,
        fairness,
        FairScope::Through(fairness.len().saturating_sub(1)),
    )
}

pub(crate) fn idempotent_has_fair_candidate_in(
    relation: &CandidateRelation,
    classes: &[AbstractClass],
    fairness: &[FairnessConstraint],
    scope: FairScope,
) -> bool {
    relation.iter().any(|((_, middle), _)| {
        relation
            .get(&(middle.clone(), middle.clone()))
            .is_some_and(|labels| {
                labels
                    .iter()
                    .any(|label| candidate_label_is_fair(label, classes, fairness, scope))
            })
    })
}

/// A hidden candidate's state key and the abstract transitions it enables.
pub(crate) type CandidateEnables<'a> = (&'a [u8], &'a [ModelTransition]);

fn candidate_label(
    enabled: &[ModelTransition],
    classes: &[AbstractClass],
    taken: impl Fn(&AbstractClass) -> bool,
) -> FairLabel {
    classes
        .iter()
        .map(|class| {
            let is_enabled = class.is_enabled_in(enabled);
            FairMark {
                taken: taken(class),
                enabled_somewhere: is_enabled,
                enabled_everywhere: is_enabled,
            }
        })
        .collect()
}

/// One candidate step row: the candidate before, the abstract transition
/// taken (`None` for a stutter) and the candidate after.
pub(crate) type CandidateRow<'a> = (&'a [u8], Option<&'a ModelTransition>, &'a [u8]);

/// The relation one edge's candidate step rows induce between its source
/// and target candidates, each pair with the fair label of its row. The
/// error names the invariant a row broke.
pub(crate) fn candidate_steps_relation<'a>(
    rows: impl IntoIterator<Item = CandidateRow<'a>>,
    source: &[CandidateEnables<'_>],
    target: &[CandidateEnables<'_>],
    classes: &[AbstractClass],
) -> Result<CandidateRelation, &'static str> {
    let mut relation = CandidateRelation::new();
    for (pre, transition, post) in rows {
        let (_, enabled) = source
            .iter()
            .find(|(key, _)| *key == pre)
            .ok_or("candidate-step-source-is-retained")?;
        if !target.iter().any(|(key, _)| *key == post) {
            return Err("candidate-step-target-is-retained");
        }
        let label = candidate_label(enabled, classes, |class| {
            transition.is_some_and(|transition| class.covers(transition))
        });
        insert_relation_label(&mut relation, (pre.to_vec(), post.to_vec()), label);
    }
    Ok(relation)
}

/// The relation a stutter, or a delay that takes no step, induces: each
/// source candidate that the target retains stays, taking no class.
pub(crate) fn candidate_stutter_relation(
    source: &[CandidateEnables<'_>],
    target: &[CandidateEnables<'_>],
    classes: &[AbstractClass],
) -> CandidateRelation {
    let mut relation = CandidateRelation::new();
    for (key, enabled) in source {
        if target.iter().any(|(held, _)| held == key) {
            insert_relation_label(
                &mut relation,
                (key.to_vec(), key.to_vec()),
                candidate_label(enabled, classes, |_| false),
            );
        }
    }
    relation
}
