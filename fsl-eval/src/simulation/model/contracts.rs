// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Contract clauses" and "Invariants": evaluating a state's
//! invariants and the conjunction of an application's `pre` or `post`
//! clauses through FR-107.

use filament_eval::{FamilyOutcome, FamilyResult};
use filament_exact::{Outcome, Value};
use filament_semantic_value::catalog::CatalogCode;
use fsl_semantics::model::observation::{AdmissionUsage, AdmittedObservations, SelfKind};

use super::walk::{exhausted, rows_of, stop, Row, Walk};
use super::{ClauseInfo, ClauseOutcome, ModelFinding, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::{ExpansionLimit, ExpansionStop};
use crate::value::ClauseSetOutcome;

/// How one clause evaluation decided.
pub(crate) enum Verdict {
    True,
    False,
    Undecided(ClauseOutcome),
}

/// Read a clause evaluation's outcome as a verdict.
///
/// A `match` over [`FamilyOutcome`] would have to name its probe-only variant
/// (FR-063), so the two real arms are tested in turn.
pub(crate) fn verdict(outcome: &FamilyOutcome<Value>) -> Verdict {
    if let FamilyOutcome::Evaluated(kernel) = outcome {
        return match kernel {
            Outcome::Completed(Value::Boolean(true)) => Verdict::True,
            Outcome::Completed(Value::Boolean(false)) => Verdict::False,
            Outcome::Completed(_) => Verdict::Undecided(ClauseOutcome::NotBoolean),
            Outcome::Undefined(undefined) => {
                Verdict::Undecided(ClauseOutcome::KernelUndefined(undefined.clone()))
            }
            Outcome::Refused(refusal) => {
                Verdict::Undecided(ClauseOutcome::KernelRefused(refusal.clone()))
            }
            Outcome::Incomplete(incomplete) => {
                Verdict::Undecided(ClauseOutcome::Incomplete(incomplete.clone()))
            }
        };
    }
    if let FamilyOutcome::FamilyEvaluated(family) = outcome {
        return Verdict::Undecided(match family {
            FamilyResult::Refused(cause) => ClauseOutcome::FamilyRefused(cause.catalog_code()),
            FamilyResult::Undefined(cause) => {
                ClauseOutcome::FamilyUndefined(cause.undefined_record().reason)
            }
        });
    }
    Verdict::Undecided(ClauseOutcome::NotBoolean)
}

/// The observations of an invariant over one object of the state.
pub(super) fn invariant_observations(
    walk: &Walk<'_, '_>,
    clause: &ClauseInfo,
    row: usize,
) -> AdmittedObservations {
    AdmittedObservations {
        usage: AdmissionUsage::default(),
        clause: clause.identity.clone(),
        current: Some(walk.observation.clone()),
        pre: None,
        post: None,
        self_object: walk.rows[row].reference.clone(),
        self_population: walk
            .observation
            .environment
            .population_node(&walk.rows[row].population.node)
            .cloned(),
        invocation: None,
        self_kind: SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: Vec::new(),
        value_origins: Default::default(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    }
}

/// Every invariant of the package, in ascending name order, for each object
/// of the state whose most-specific type conforms to the clause's context, in
/// ascending reference-key order (FR-120 "Invariants").
pub(super) fn invariants(walk: &Walk<'_, '_>) -> Result<Vec<ModelFinding>, ExpansionStop> {
    evaluate_invariants(walk, false, None)
}

/// [`invariants`] for a state evaluated for its findings alone (FR-634
/// "Invariant findings"): a clause evaluation that breaks reports that
/// invariant undetermined for that object, with the broken evaluation's
/// cause, and the other clauses and objects are still evaluated.
pub(super) fn invariants_alone(walk: &Walk<'_, '_>) -> Result<Vec<ModelFinding>, ExpansionStop> {
    evaluate_invariants(walk, true, None)
}

/// [`invariants_alone`] for one selected invariant, so a named model-based
/// oracle does not evaluate unrelated clauses.
pub(super) fn invariant_alone(
    walk: &Walk<'_, '_>,
    name: &str,
) -> Result<Vec<ModelFinding>, ExpansionStop> {
    evaluate_invariants(walk, true, Some(name))
}

/// The rows of `walk` in ascending reference order.
fn reference_order(rows: &[Row]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|left, right| rows[*left].reference.cmp(&rows[*right].reference));
    order
}

fn evaluate_invariants(
    walk: &Walk<'_, '_>,
    alone: bool,
    selected: Option<&str>,
) -> Result<Vec<ModelFinding>, ExpansionStop> {
    let system = walk.system;
    let order = reference_order(&walk.rows);
    let mut findings = Vec::new();
    for clause in &system.clauses.invariants {
        if selected.is_some_and(|name| clause.name != name) {
            continue;
        }
        for row in &order {
            let reference = &walk.rows[*row].reference;
            if !system
                .types
                .conforms(reference.object_type().clone(), clause.context.clone())
            {
                continue;
            }
            let observations = invariant_observations(walk, clause, *row);
            let evaluation = match system.evaluator().evaluate_clause(
                &clause.qualified,
                &observations,
                &mut walk.meter(),
            ) {
                Ok(evaluation) => evaluation,
                Err(_) if alone => {
                    findings.push(ModelFinding::InvariantUndetermined {
                        clause: clause.name.clone(),
                        self_object: reference.clone(),
                        anchor: walk.state.anchor.clone(),
                        evaluation: ClauseOutcome::Stopped(BROKEN),
                    });
                    continue;
                }
                Err(_) => return Err(stop(BROKEN)),
            };
            if walk
                .cancel
                .as_ref()
                .is_some_and(|cancel| cancel.tripped().is_some())
            {
                return Err(exhausted(ExpansionLimit::EvaluationMeter(
                    filament_exact::LimitKind::WorkUnits,
                )));
            }
            match verdict(&evaluation.evaluation.outcome) {
                Verdict::True => {}
                Verdict::False => findings.push(ModelFinding::InvariantViolated {
                    clause: clause.name.clone(),
                    self_object: reference.clone(),
                    anchor: walk.state.anchor.clone(),
                }),
                Verdict::Undecided(outcome) => findings.push(ModelFinding::InvariantUndetermined {
                    clause: clause.name.clone(),
                    self_object: reference.clone(),
                    anchor: walk.state.anchor.clone(),
                    evaluation: outcome,
                }),
            }
        }
    }
    findings.extend(bound_findings(system, walk.state, &walk.rows, &order));
    Ok(findings)
}

/// FR-634 "Invariant findings": the findings of `state` when its evaluation
/// for findings alone stopped with `cause` before deciding: each invariant
/// undetermined, with `cause`, for each object its context admits, in
/// [`invariants`]' order, then the member-bound findings, which read the
/// state's values and evaluate no clause.
///
/// # Errors
///
/// `runtime_invariant`/`established-invariant-broken` when the state's
/// objects cannot be referenced, so no object can be named.
pub(super) fn stopped_invariants(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    cause: CatalogCode,
) -> Result<Vec<ModelFinding>, ExpansionStop> {
    let rows = rows_of(system, &state.state)?;
    let order = reference_order(&rows);
    let mut findings = Vec::new();
    for clause in &system.clauses.invariants {
        for row in &order {
            let reference = &rows[*row].reference;
            if system
                .types
                .conforms(reference.object_type().clone(), clause.context.clone())
            {
                findings.push(ModelFinding::InvariantUndetermined {
                    clause: clause.name.clone(),
                    self_object: reference.clone(),
                    anchor: state.anchor.clone(),
                    evaluation: ClauseOutcome::Stopped(cause),
                });
            }
        }
    }
    findings.extend(bound_findings(system, state, &rows, &order));
    Ok(findings)
}

/// FSpec FR-154 "Unit bounds": a state whose value violates a bound
/// violates the constraint as an invariant, recorded rather than pruned.
fn bound_findings(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    rows: &[Row],
    order: &[usize],
) -> Vec<ModelFinding> {
    let mut findings = Vec::new();
    for bound in &system.member_bounds {
        for row in order {
            let row = &rows[*row];
            let violated = state
                .state
                .populations
                .get(&row.population)
                .and_then(|objects| objects.get(&row.key))
                .and_then(|object| object.fields.get(&bound.member))
                .and_then(|value| bound.violation(value, system.types))
                .is_some();
            if violated {
                findings.push(ModelFinding::InvariantViolated {
                    clause: bound.constraint.node.clone(),
                    self_object: row.reference.clone(),
                    anchor: state.anchor.clone(),
                });
            }
        }
    }
    findings
}

/// The conjunction of a set of clauses of one kind (FR-120 "Contract
/// clauses").
pub(super) enum Conjunction {
    /// Every clause is `true`, or there are none.
    True,
    /// Some clause is `false`; this is the first such clause in evaluation
    /// order (ascending name).
    False(String),
    /// No clause is `false` and these did not decide, in name order.
    Undecided(Vec<(String, ClauseOutcome)>),
}

/// Evaluate `clauses`, each over the observations `observe` builds for it,
/// each with a fresh meter and in the order given (ascending name).
///
/// # Errors
///
/// An `Incomplete` evaluation stops the expansion with
/// `resource_exhausted`/`insufficient-next-charge`; a call failure stops it
/// with `runtime_invariant`/`established-invariant-broken`.
pub(super) fn conjunction(
    walk: &Walk<'_, '_>,
    clauses: &[&ClauseInfo],
    observe: impl Fn(&ClauseInfo) -> AdmittedObservations,
) -> Result<Conjunction, ExpansionStop> {
    if clauses.is_empty() {
        return Ok(Conjunction::True);
    }
    let observations: Vec<AdmittedObservations> =
        clauses.iter().map(|info| observe(info)).collect();
    let set: Vec<_> = clauses
        .iter()
        .zip(&observations)
        .map(|(info, observations)| (info.qualified.clone(), observations))
        .collect();
    let evaluation = walk
        .system
        .evaluator()
        .evaluate_clause_set(&set, walk.system.meter, walk.cancel.as_ref())
        .map_err(|_| stop(BROKEN))?;
    match evaluation.outcome {
        ClauseSetOutcome::Incomplete => {
            let counter = evaluation
                .results
                .iter()
                .find_map(
                    |entry| match verdict(&entry.evaluation.evaluation.outcome) {
                        Verdict::Undecided(ClauseOutcome::Incomplete(incomplete)) => {
                            Some(incomplete.limit_kind)
                        }
                        _ => None,
                    },
                )
                .ok_or_else(|| stop(BROKEN))?;
            Err(exhausted(ExpansionLimit::EvaluationMeter(counter)))
        }
        ClauseSetOutcome::True => Ok(Conjunction::True),
        ClauseSetOutcome::False => {
            let entry = evaluation
                .results
                .iter()
                .find(|entry| {
                    matches!(
                        verdict(&entry.evaluation.evaluation.outcome),
                        Verdict::False
                    )
                })
                .ok_or_else(|| stop(BROKEN))?;
            let name = clauses
                .iter()
                .find(|info| info.qualified == entry.name)
                .map(|info| info.name.clone())
                .ok_or_else(|| stop(BROKEN))?;
            Ok(Conjunction::False(name))
        }
        ClauseSetOutcome::Undecided => {
            let mut undecided = Vec::new();
            for entry in &evaluation.results {
                if let Verdict::Undecided(outcome) = verdict(&entry.evaluation.evaluation.outcome) {
                    let name = clauses
                        .iter()
                        .find(|info| info.qualified == entry.name)
                        .map(|info| info.name.clone())
                        .ok_or_else(|| stop(BROKEN))?;
                    undecided.push((name, outcome));
                }
            }
            Ok(Conjunction::Undecided(undecided))
        }
    }
}
