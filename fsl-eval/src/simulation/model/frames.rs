// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Post-states": the applications of one object's operations and the
//! candidate post-states inside each operation's frame, generated lazily.
//!
//! A candidate is one choice of three things, made in this order: which
//! objects the operation deletes, which unused universe keys it creates and
//! under which type, and a value for every field the frame lets it write. Each
//! choice is an odometer, so a candidate is built and dropped one at a time
//! and a product too large to hold is never materialized.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::rc::Rc;

use filament_checked::key::DeclarationKey;
use filament_exact::{Integer, ObjectReference, OrderingOperator, Value, ValueType};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_semantics::model::domain_package::OperationEffect;
use fsl_semantics::model::observation::{
    AdmissionUsage, AdmittedObservations, Observation, SelfKind,
};
use fsl_semantics::model::population::{decide_frame, FrameDecision, FrameObject, FrameStop};
use fsl_semantics::model::state::{ModelState, StateField, StateObject};
use quire_canonical::Limits as CanonicalLimits;

pub(super) mod judge;
#[cfg(test)]
mod narrowing_tests;

use super::contracts::{conjunction, operation_conjunction, Conjunction};
use super::domain::{Domain, Odometer};
use super::encode::{self, value_enc, values_equal, Typing};
use super::footprints::ReadCheck;
use super::walk::{stop, Applied, Budget, ContractFinding, FindingCase, Row, Selected, Walk};
use super::{
    handler_anchor, identity_limits, state_document, Application, ClauseInfo, ClauseOutcome,
    FieldChange, ModelFinding, ModelSystem, StepEffect, WrittenSlots, BROKEN, EXHAUSTED,
};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::footprint::Footprint;
use crate::simulation::key;
use crate::value::PostTarget;
use crate::value::{Provenance, WrittenCollection};
use filament_semantic_value::catalog::CatalogCode;
use fsl_semantics::check::CollectionWrite;

/// The most values a result domain may hold and still be put in canonical
/// order; a larger one stops the expansion.
const RESULT_ORDER_LIMIT: u128 = 1 << 16;

/// The identity limit every canonical text of a step is written under.
pub(super) fn text_limits() -> CanonicalLimits {
    CanonicalLimits::new(identity_limits().input_bytes)
}

/// `value`'s canonical text.
pub(super) fn text_of(value: &Value, typing: Typing<'_>) -> Result<String, ExpansionStop> {
    value_enc(value, typing)
        .canonical_text(text_limits())
        .map_err(|_| stop(BROKEN))
}

/// The transition identity of `application` applied to `args` at `row`
/// (FR-181): the operation, the receiver apart from the arguments, and one
/// canonical text per declared parameter.
fn transition_of(
    row: &Row,
    application: &Application,
    args: &[(String, Value)],
    typing: Typing<'_>,
) -> Result<ModelTransition, ExpansionStop> {
    let receiver = encode::reference_enc(&row.reference)
        .canonical_text(text_limits())
        .map_err(|_| stop(BROKEN))?;
    let arguments = args
        .iter()
        .map(|(_, value)| text_of(value, typing))
        .collect::<Result<Vec<_>, _>>()?;
    ModelTransition::new(
        application.text.clone(),
        Some(receiver),
        arguments,
        text_limits(),
    )
    .map_err(|_| stop(BROKEN))
}

/// The parameter domains of `application` over the objects of `walk`'s
/// state, one per declared parameter: a random parameter's support, else
/// its type's domain.
fn argument_domains(
    walk: &Walk<'_, '_>,
    application: &Application,
) -> Result<Vec<Domain>, ExpansionStop> {
    let system = walk.system;
    let mut references = walk.references();
    if walk.candidate_visitor.is_some() {
        // Planning uses the complete typed reference identity order, which
        // may differ from population row order or canonical JSON text order.
        for at in 1..references.len() {
            let mut next = at;
            while next > 0 {
                walk.meter()
                    .charge(filament_exact::Charge::new(
                        filament_exact::ChargePoint::CollectionVisit,
                    ))
                    .map_err(|incomplete| {
                        super::walk::exhausted(
                            crate::simulation::expansion::ExpansionLimit::EvaluationMeter(
                                incomplete.limit_kind,
                            ),
                        )
                    })?;
                if references[next - 1] <= references[next] {
                    break;
                }
                references.swap(next - 1, next);
                next -= 1;
            }
        }
        references.dedup();
    }
    application
        .declaration
        .parameters()
        .iter()
        .enumerate()
        .map(|(position, (_, value_type))| {
            match system.probabilistic.support(
                application.declaring.clone(),
                &application.name,
                position,
            ) {
                Some(support) => Some(Domain::Listed(support.values())),
                None => walk.value_domain(value_type, &references),
            }
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| stop(BROKEN))
}

/// The argument vector `digits` selects from `domains`, by parameter name.
fn arguments_of(
    application: &Application,
    domains: &[Domain],
    digits: &[u128],
) -> Result<Vec<(String, Value)>, ExpansionStop> {
    application
        .declaration
        .parameters()
        .iter()
        .zip(domains)
        .zip(digits)
        .map(|(((name, _), domain), index)| domain.nth(*index).map(|value| (name.clone(), value)))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| stop(BROKEN))
}

/// Every transition identity of `application` at `row`, enabled or not
/// (FR-156 "Candidates"): one per argument vector of its parameter domains,
/// each counted against `budget`.
///
/// # Errors
///
/// An [`ExpansionStop`] when the identities cannot be listed within the
/// candidate bound.
pub(super) fn identities(
    walk: &Walk<'_, '_>,
    row: &Row,
    application: &Application,
    budget: &mut Budget,
    out: &mut Vec<ModelTransition>,
) -> Result<(), ExpansionStop> {
    let domains = argument_domains(walk, application)?;
    let mut arguments = Odometer::new(domains.iter().map(Domain::len).collect());
    while let Some(digits) = arguments.next_digits() {
        budget.charge()?;
        let args = arguments_of(application, &domains, &digits)?;
        out.push(transition_of(
            row,
            application,
            &args,
            walk.system.typing(),
        )?);
    }
    Ok(())
}

/// Apply `application` to `row` for every argument vector of its parameter
/// domains under the `pre` and `post` clauses `clauses` selects: check its
/// effective precondition, and for an enabled application generate its
/// successors. A random parameter ranges over its support in declared order
/// (FR-187, ADR-024 PM-1). This is FR-120's one application rule:
/// `ModelSystem` calls it with every applicable clause, a protocol attempt
/// with the clauses its `contracts` select (ADR-027 TS-3). When `enabled` is
/// given, each argument vector whose precondition holds is recorded there
/// with the successors it generated, none included.
///
/// # Errors
///
/// An [`ExpansionStop`] when the successor set cannot be known.
pub(super) fn apply(
    walk: &Walk<'_, '_>,
    row: &Row,
    application: &Application,
    clauses: &Selected<'_>,
    budget: &mut Budget,
    mut outputs: Outputs<'_>,
) -> Result<(), ExpansionStop> {
    let domains = argument_domains(walk, application)?;
    let frames = ArgumentFrames::default();
    let mut arguments = Odometer::new(domains.iter().map(Domain::len).collect());
    while let Some(digits) = arguments.next_digits() {
        let args = arguments_of(application, &domains, &digits)?;
        apply_argument(
            walk,
            ApplicationContext {
                row,
                application,
                clauses,
            },
            args,
            &frames,
            budget,
            Outputs {
                contract: &mut *outputs.contract,
                successors: &mut *outputs.successors,
                enabled: outputs.enabled.as_deref_mut(),
                enabled_transitions: outputs.enabled_transitions.as_deref_mut(),
                recommendations: outputs.recommendations.as_deref_mut(),
                selected_transitions: outputs.selected_transitions,
                charged: outputs.charged.as_deref_mut(),
            },
        )?;
    }
    Ok(())
}

/// The receiver, operation and selected clauses shared by its argument vectors.
struct ApplicationContext<'a, 'c> {
    row: &'a Row,
    application: &'a Application,
    clauses: &'a Selected<'c>,
}

/// Apply one typed argument vector through the same native relation used by
/// every model caller. Enumeration and source-plan cursors share this path.
fn apply_argument(
    walk: &Walk<'_, '_>,
    context: ApplicationContext<'_, '_>,
    args: Vec<(String, Value)>,
    frames: &ArgumentFrames,
    budget: &mut Budget,
    outputs: Outputs<'_>,
) -> Result<(), ExpansionStop> {
    let ApplicationContext {
        row,
        application,
        clauses,
    } = context;
    let Outputs {
        contract,
        successors,
        enabled,
        enabled_transitions,
        recommendations,
        selected_transitions,
        charged,
    } = outputs;
    let system = walk.system;
    let preconditions = &clauses.preconditions;
    let postconditions = &clauses.postconditions;
    // Every argument vector counts, enabled or not: a false or undecided
    // precondition would otherwise let one expansion run unbounded.
    budget.charge()?;
    let transition = transition_of(row, application, &args, walk.system.typing())?;
    if selected_transitions.is_some_and(|selected| !selected.contains(&transition)) {
        return Ok(());
    }
    let transition_bytes =
        quire_canonical::to_vec(&transition, text_limits()).map_err(|_| stop(BROKEN))?;
    let footprint = walk
        .enforcement
        .filter(|_| recommendations.is_none())
        .map(|enforcement| {
            enforcement.footprint_of(system, application, &row.reference, &args, &transition)
        })
        .transpose()?;

    let check = walk
        .enforcement
        .zip(footprint.as_ref())
        .map(|(enforcement, footprint)| {
            ReadCheck::new(&system.model, enforcement, footprint, &transition)
        });
    let verdict = conjunction(walk, preconditions, |info| {
        let observations = pre_observations(walk, info, row, &args);
        match &check {
            Some(check) => check.attach(observations),
            None => observations,
        }
    })?;
    if let Some(check) = &check {
        check.verify()?;
    }
    let verdict = if clauses.operation_preconditions.is_empty() {
        verdict
    } else {
        operation_conjunction(
            walk,
            &clauses.operation_preconditions,
            &fsl_semantics::model::observation::OperationContractObservations {
                pre: walk.observation.clone(),
                post: walk.observation.clone(),
                self_reference: row.reference.clone(),
                parameters: args.clone(),
                result: None,
            },
            verdict,
        )?
    };
    match verdict {
        Conjunction::True => {}
        Conjunction::False(_) => return Ok(()),
        Conjunction::Undecided(undecided) => {
            for (clause, evaluation) in undecided {
                if walk.candidate_visitor.is_some() {
                    walk.visit(super::NativeCandidateEvent::Undetermined {
                        transition: &transition,
                        outcome: &evaluation,
                    })?;
                    continue;
                }
                contract.push(ContractFinding {
                    transition: transition_bytes.clone(),
                    candidate: None,
                    clause: clause.clone(),
                    finding: ModelFinding::ContractUndetermined {
                        transition: transition.clone(),
                        candidate: None,
                        clause,
                        evaluation,
                    },
                    case: Box::new(FindingCase {
                        receiver: row.reference.clone(),
                        arguments: args.iter().map(|(_, value)| value.clone()).collect(),
                        post: None,
                    }),
                });
            }
            return Ok(());
        }
    }

    if let Some(recommendations) = recommendations {
        recommendations.push(super::application::RecommendedApplication {
            operation: application.text.clone(),
            receiver: row.reference.clone(),
            arguments: args.into_iter().map(|(_, value)| value).collect(),
        });
        return Ok(());
    }

    let application_run = Run {
        walk,
        row,
        application,
        args: &args,
        transition: &transition,
        transition_bytes: &transition_bytes,
        postconditions,
        operation_postconditions: &clauses.operation_postconditions,
        footprint: footprint.as_ref(),
        references: reference_arguments(&args),
        result_order_cache: RefCell::new(None),
        frames,
    };

    // FR-220 access classification needs only whether this transition
    // completes when no attempt constraint reads a completed result. Keep
    // the transition only if at least one candidate post-state succeeds.
    let before = budget.used();
    if let Some(transitions) = enabled_transitions {
        let mut first_successor = Vec::new();
        let completes = application_run.first_candidate(budget, contract, &mut first_successor)?;
        if let Some(charged) = charged {
            charged.push((transition.clone(), budget.used() - before));
        }
        if completes {
            transitions.push(transition);
        }
        return Ok(());
    }

    let first = successors.len();
    application_run.candidates(budget, contract, successors)?;
    if let Some(charged) = charged {
        charged.push((transition.clone(), budget.used() - before));
    }
    if let Some(enabled) = enabled {
        enabled.push(EnabledVector {
            transition,
            receiver: row.reference.clone(),
            declaring: application.declaring.clone(),
            name: application.name.clone(),
            operation: application.text.clone(),
            args,
            successors: first..successors.len(),
        });
    }
    Ok(())
}

/// Declaration -> receiver -> typed argument order, reusing apply_argument.
/// The supported scalar domains are indexed lazily rather than collected.
pub(super) fn visit_operation(
    walk: &Walk<'_, '_>,
    declaring: &filament_exact::EffectiveId,
    name: &str,
    budget: &mut Budget,
) -> Result<(), ExpansionStop> {
    let mut rows: Vec<_> = walk
        .rows
        .iter()
        .filter_map(|row| {
            let application =
                walk.system
                    .applications
                    .get(&row.type_id())?
                    .iter()
                    .find(|application| {
                        &application.declaring == declaring && application.name == name
                    })?;
            Some((row, application))
        })
        .collect();
    for at in 1..rows.len() {
        let mut next = at;
        while next > 0 {
            budget.charge()?;
            if !rows[next - 1]
                .0
                .reference
                .cmp(&rows[next].0.reference)
                .is_gt()
            {
                break;
            }
            rows.swap(next - 1, next);
            next -= 1;
        }
    }
    for (row, application) in rows {
        // Receiver-scoped frame caches must never be reused by another row.
        let frames = ArgumentFrames::default();
        if application
            .declaration
            .parameters()
            .iter()
            .any(|(_, value_type)| {
                !matches!(
                    value_type,
                    ValueType::Boolean
                        | ValueType::Int(_)
                        | ValueType::Reference(_)
                        | ValueType::Enum(_)
                )
            })
        {
            return Err(stop(BROKEN));
        }
        let domains = argument_domains(walk, application)?;
        let mut arguments = Odometer::new(domains.iter().map(Domain::len).collect());
        while let Some(digits) = arguments.next_digits() {
            let args = arguments_of(application, &domains, &digits)?;
            let transition = transition_of(row, application, &args, walk.system.typing())?;
            match walk.visit(super::NativeCandidateEvent::Begin(&transition))? {
                super::NativeCandidateControl::Skip => {
                    // Replay selection still accounts for walking the finite
                    // invocation identities that precede its selected one.
                    budget.charge()?;
                    continue;
                }
                super::NativeCandidateControl::Continue => {}
                super::NativeCandidateControl::Stop => return Err(stop(BROKEN)),
            }
            walk.retained_occurrences
                .replace(filament_exact::Integer::one());
            let clauses = walk.model_selected(row, application, |_| true);
            let mut contract = Vec::new();
            let mut successors = Vec::new();
            apply_argument(
                walk,
                ApplicationContext {
                    row,
                    application,
                    clauses: &clauses,
                },
                args,
                &frames,
                budget,
                Outputs {
                    contract: &mut contract,
                    successors: &mut successors,
                    enabled: None,
                    enabled_transitions: None,
                    recommendations: None,
                    selected_transitions: None,
                    charged: None,
                },
            )?;
            debug_assert!(contract.is_empty());
            // Sort only this already accounted invocation's outcomes. At
            // most two bounded canonical state keys are temporary at once.
            for at in 1..successors.len() {
                let mut next = at;
                while next > 0 {
                    budget.charge()?;
                    let left = &successors[next - 1];
                    let right = &successors[next];
                    let result = match (&left.result, &right.result) {
                        (None, None) => std::cmp::Ordering::Equal,
                        (Some(left), Some(right)) => crate::value::compare_map_keys(left, right)
                            .ok_or_else(|| stop(BROKEN))?,
                        (None, Some(_)) | (Some(_), None) => return Err(stop(BROKEN)),
                    };
                    let order = if result.is_eq() {
                        let left_key = walk
                            .system
                            .state_key(&left.state)
                            .map_err(|_| stop(BROKEN))?
                            .0;
                        let right_key = walk
                            .system
                            .state_key(&right.state)
                            .map_err(|_| stop(BROKEN))?
                            .0;
                        left_key.cmp(&right_key)
                    } else {
                        result
                    };
                    if !order.is_gt() {
                        break;
                    }
                    successors.swap(next - 1, next);
                    next -= 1;
                }
            }
            // The ordinal names a distinct (result, post-state) outcome,
            // not duplicate raw assignments that computed the same value.
            let mut at = 0;
            while at + 1 < successors.len() {
                budget.charge()?;
                let left = &successors[at];
                let right = &successors[at + 1];
                let same_result = match (&left.result, &right.result) {
                    (None, None) => true,
                    (Some(left), Some(right)) => crate::value::compare_map_keys(left, right)
                        .ok_or_else(|| stop(BROKEN))?
                        .is_eq(),
                    (None, Some(_)) | (Some(_), None) => return Err(stop(BROKEN)),
                };
                let same_state = same_result
                    && walk
                        .system
                        .state_key(&left.state)
                        .map_err(|_| stop(BROKEN))?
                        .0
                        == walk
                            .system
                            .state_key(&right.state)
                            .map_err(|_| stop(BROKEN))?
                            .0;
                if same_state {
                    budget.charge_many(successors.len() - at - 1)?;
                    successors.remove(at + 1);
                } else {
                    at += 1;
                }
            }
            if successors.is_empty() {
                walk.visit(super::NativeCandidateEvent::Disabled(&transition))?;
            }
            let multiple = successors.len() > 1;
            for (index, applied) in successors.iter().enumerate() {
                let outcome = multiple
                    .then(|| u64::try_from(index))
                    .transpose()
                    .map_err(|_| stop(BROKEN))?;
                walk.visit(super::NativeCandidateEvent::Completed {
                    transition: &applied.transition,
                    receiver: &applied.receiver,
                    arguments: &applied.arguments,
                    state: &applied.state,
                    outcome,
                })?;
            }
        }
    }
    Ok(())
}

/// ADR-027 PD-4: the transition identity of the first argument vector of
/// `application` at `row`, in canonical order, with the first of `clauses`'
/// preconditions, in clause order, that is not true there (`None` when
/// every one is true). `None` for an application with no argument vector.
///
/// # Errors
///
/// An [`ExpansionStop`] when a precondition's evaluation stops.
pub(super) fn first_precondition(
    walk: &Walk<'_, '_>,
    row: &Row,
    application: &Application,
    clauses: &Selected<'_>,
) -> Result<Option<(ModelTransition, Option<String>)>, ExpansionStop> {
    let domains = argument_domains(walk, application)?;
    let mut arguments = Odometer::new(domains.iter().map(Domain::len).collect());
    let Some(digits) = arguments.next_digits() else {
        return Ok(None);
    };
    let args = arguments_of(application, &domains, &digits)?;
    let transition = transition_of(row, application, &args, walk.system.typing())?;
    for clause in &clauses.preconditions {
        let verdict = conjunction(walk, &[*clause], |info| {
            pre_observations(walk, info, row, &args)
        })?;
        if !matches!(verdict, Conjunction::True) {
            return Ok(Some((transition, Some(clause.name.clone()))));
        }
    }
    Ok(Some((transition, None)))
}

/// One argument vector of an application and how its one precondition
/// read there (FSpec FR-485 "Clause"): `None` when it decided a Boolean,
/// else the outcome that decided none.
pub(super) struct PreconditionCase {
    /// The FR-120 transition identity.
    pub(super) transition: ModelTransition,
    /// The arguments, in parameter order.
    pub(super) arguments: Vec<Value>,
    /// Whether the precondition decided true.
    pub(super) satisfied: bool,
    /// The evaluation that decided no Boolean, if any.
    pub(super) undecided: Option<ClauseOutcome>,
}

/// FSpec FR-485 "Clause": the precondition `clause` of `application` at
/// `row` evaluated alone at every argument vector, each counted against
/// `budget`, with no candidate post-state built.
///
/// # Errors
///
/// An [`ExpansionStop`] when the budget runs out or an evaluation stops.
pub(super) fn precondition_cases(
    walk: &Walk<'_, '_>,
    row: &Row,
    application: &Application,
    clause: &ClauseInfo,
    budget: &mut Budget,
) -> Result<Vec<PreconditionCase>, ExpansionStop> {
    let domains = argument_domains(walk, application)?;
    let mut arguments = Odometer::new(domains.iter().map(Domain::len).collect());
    let mut cases = Vec::new();
    while let Some(digits) = arguments.next_digits() {
        let args = arguments_of(application, &domains, &digits)?;
        budget.charge()?;
        let transition = transition_of(row, application, &args, walk.system.typing())?;
        let verdict = conjunction(walk, &[clause], |info| {
            pre_observations(walk, info, row, &args)
        })?;
        let (satisfied, undecided) = match verdict {
            Conjunction::True => (true, None),
            Conjunction::False(_) => (false, None),
            Conjunction::Undecided(mut undecided) => {
                (false, undecided.pop().map(|(_, outcome)| outcome))
            }
        };
        cases.push(PreconditionCase {
            transition,
            arguments: args.into_iter().map(|(_, value)| value).collect(),
            satisfied,
            undecided,
        });
    }
    Ok(cases)
}

/// Where [`apply`] records what one application found: its contract
/// findings, its successors and, when given, its enabled argument vectors.
pub(super) struct Outputs<'o> {
    /// Preconditions-only calls; when present, no post-state is computed.
    pub(super) recommendations: Option<&'o mut Vec<super::application::RecommendedApplication>>,
    /// The contract findings.
    pub(super) contract: &'o mut Vec<ContractFinding>,
    /// The successors.
    pub(super) successors: &'o mut Vec<Applied>,
    /// Each enabled argument vector, when the caller records them.
    pub(super) enabled: Option<&'o mut Vec<EnabledVector>>,
    /// Enabled transition identities when the caller needs access
    /// classification without enumerating every candidate post-state.
    pub(super) enabled_transitions: Option<&'o mut Vec<ModelTransition>>,
    /// An exact set of candidate transition identities to check. Candidate
    /// enumeration and its budget charges still cover the full domain.
    pub(super) selected_transitions: Option<&'o std::collections::BTreeSet<ModelTransition>>,
    /// The candidates each enabled argument vector charged beyond its own
    /// count, by transition identity, when the caller records them.
    pub(super) charged: Option<&'o mut Vec<(ModelTransition, u64)>>,
}

/// One argument vector of an application whose effective precondition holds
/// at the expanded state, and the successors it generated (FR-187).
pub(super) struct EnabledVector {
    /// The complete identity, retained even when the vector has no successor.
    pub(super) transition: ModelTransition,
    /// The receiver.
    pub(super) receiver: ObjectReference,
    /// The operation's declaring type.
    pub(super) declaring: filament_exact::EffectiveId,
    /// The operation's name.
    pub(super) name: String,
    /// The operation's declaration identity text.
    pub(super) operation: String,
    /// One value per declared parameter, random ones included.
    pub(super) args: Vec<(String, Value)>,
    /// The positions of its successors in the expansion's successor list.
    pub(super) successors: std::ops::Range<usize>,
}

/// The transition identity of an application of the operation `operation`
/// to `receiver` with `arguments`, which may omit some parameters: a
/// scheduled identity's canonical order key (FR-187).
pub(super) fn identity_bytes(
    operation: &str,
    receiver: &ObjectReference,
    arguments: &[Value],
    typing: Typing<'_>,
) -> Result<Vec<u8>, ExpansionStop> {
    let receiver = encode::reference_enc(receiver)
        .canonical_text(text_limits())
        .map_err(|_| stop(BROKEN))?;
    let arguments = arguments
        .iter()
        .map(|value| text_of(value, typing))
        .collect::<Result<Vec<_>, _>>()?;
    let transition = ModelTransition::new(
        operation.to_owned(),
        Some(receiver),
        arguments,
        text_limits(),
    )
    .map_err(|_| stop(BROKEN))?;
    quire_canonical::to_vec(&transition, text_limits()).map_err(|_| stop(BROKEN))
}

/// The observations of a precondition: the state as `pre`, the receiver as
/// `self` and the arguments by declared name (FR-120 "Frame and observation
/// seam").
fn pre_observations(
    walk: &Walk<'_, '_>,
    clause: &ClauseInfo,
    row: &Row,
    args: &[(String, Value)],
) -> AdmittedObservations {
    AdmittedObservations {
        usage: AdmissionUsage::default(),
        clause: clause.identity.clone(),
        current: None,
        pre: Some(walk.observation.clone()),
        post: None,
        self_object: row.reference.clone(),
        self_population: walk
            .observation
            .environment
            .population_node(&row.population.node)
            .cloned(),
        invocation: None,
        self_kind: SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: args.to_vec(),
        value_origins: Default::default(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    }
}

/// One enabled application and what its candidates need.
struct Run<'w, 's, 'a> {
    walk: &'w Walk<'s, 'a>,
    row: &'w Row,
    application: &'w Application,
    args: &'w [(String, Value)],
    transition: &'w ModelTransition,
    transition_bytes: &'w [u8],
    postconditions: &'w [&'w ClauseInfo],
    operation_postconditions: &'w [&'w fsl_semantics::check::CheckedOperationClause],
    /// The footprint the clauses are held to, when the expansion enforces
    /// footprints.
    footprint: Option<&'w Footprint>,
    /// The argument vector's reference-valued arguments, by parameter name:
    /// the only arguments the frame decision reads.
    references: References,
    /// Canonical result order for the current post-state object universe.
    result_order_cache: RefCell<Option<(Vec<ObjectReference>, Vec<u128>)>>,
    /// What the frame decision found for the application's argument
    /// vectors.
    frames: &'w ArgumentFrames,
}

/// The reference-valued arguments of `args`, by parameter name.
pub(super) fn reference_arguments(args: &[(String, Value)]) -> References {
    args.iter()
        .filter_map(|(name, value)| match value {
            Value::Reference(bound) => Some((name.clone(), bound.clone())),
            _ => None,
        })
        .collect()
}

/// What the frame decision found for one application at one state, by the
/// reference-valued arguments of an argument vector (the only arguments it
/// reads), so argument vectors that agree on them share it: the fields the
/// frame lets the application write, and the decision for each population
/// a candidate leaves as the state holds it.
#[derive(Default)]
pub(super) struct ArgumentFrames {
    writable: RefCell<BTreeMap<References, Rc<WritableFields>>>,
    unchanged: RefCell<UnchangedPopulations>,
}

/// An argument vector's reference-valued arguments, by parameter name.
type References = Vec<(String, ObjectReference)>;

/// The decision for each population a candidate leaves as the state holds
/// it, by reference-valued arguments.
type UnchangedPopulations = BTreeMap<(References, DeclarationKey), PopulationDecided>;

/// The frame decision for one population: the keys the candidate creates
/// and deletes there, or the refusal.
type PopulationDecided = Result<(Vec<String>, Vec<String>), FrameRefused>;

/// What one application's candidates share: the keys it may create objects
/// under and the fields it may write.
struct Plan<'p> {
    slots: &'p [Slot],
    writable: &'p WritableFields,
    stop_after_first: bool,
    /// Whether the checked postcondition targets prove that result values do
    /// not affect either set of operation postconditions.
    result_independent: bool,
    /// What the postconditions say about the receiver's fields, when the
    /// application is solved and narrowed rather than enumerated.
    narrowing: Option<&'p Narrowing>,
}

/// FR-120 "Directly solved assignments" and FR-851 "Narrowing": what the
/// postconditions of one application say about each written field of its
/// receiver, by field name.
type Narrowing = BTreeMap<String, FieldTargets>;

/// The equation and bound conjuncts on one field, each with its `e`'s
/// value (`None` when it was not `Completed`).
#[derive(Default)]
struct FieldTargets {
    /// Each equation's value.
    equations: Vec<Option<Value>>,
    /// Each bound, the field as its left operand.
    bounds: Vec<(OrderingOperator, Option<Value>)>,
}

/// Which positions of a field's domain a candidate takes, in ascending
/// order.
enum Allowed {
    /// Every position: the field is enumerated.
    All,
    /// The `len` positions from `start`: an integer field inside its bounds.
    Range { start: u128, len: u128 },
    /// These positions: a solved field.
    Listed(Vec<u128>),
}

impl Allowed {
    /// The positions of `domain`, the domain of a required field the frame
    /// writes on the receiver, that `targets` admits; `None` when the
    /// application is enumerated instead: two equations assign the field,
    /// or an `e` the field reads has no value (FR-120 "Value", FR-851
    /// "Narrowing").
    fn of(domain: &Domain, targets: &FieldTargets) -> Option<Self> {
        // FR-851: a bound narrows only an integer field with a finite range;
        // on any other field it is a conjunct like any other.
        let integers = match domain {
            Domain::Int { lower, len } => Some((lower, u64::try_from(*len).ok()?)),
            _ => None,
        };
        let mut bounds = Vec::new();
        if integers.is_some() {
            for (operator, value) in &targets.bounds {
                let Some(Value::Integer(bound)) = value else {
                    return None;
                };
                bounds.push((*operator, bound));
            }
        }
        let admits = |value: &Integer| {
            bounds
                .iter()
                .all(|(operator, bound)| operator.holds(value.cmp(bound)))
        };
        match targets.equations.as_slice() {
            [] => {}
            [Some(equal)] => {
                let positions: Vec<u128> = match (integers, equal) {
                    (Some((lower, len)), Value::Integer(value)) => value
                        .sub(lower)
                        .to_u64()
                        .filter(|offset| *offset < len && admits(value))
                        .map(u128::from)
                        .into_iter()
                        .collect(),
                    (Some(_), _) => Vec::new(),
                    (None, Value::Integer(_)) if matches!(domain, Domain::SizeInteger { .. }) => {
                        domain.index_of(equal).into_iter().collect()
                    }
                    (None, _) => (0..domain.len())
                        .filter(|index| {
                            domain
                                .nth(*index)
                                .is_some_and(|candidate| values_equal(&candidate, equal))
                        })
                        .collect(),
                };
                return Some(Self::Listed(positions));
            }
            _ => return None,
        }
        let Some((lower, len)) = integers.filter(|_| !bounds.is_empty()) else {
            return Some(Self::All);
        };
        let (mut least, mut most) = (
            lower.clone(),
            lower.add(&Integer::from(len)).sub(&Integer::one()),
        );
        for (operator, bound) in bounds {
            match operator {
                OrderingOperator::Less => most = most.min(bound.sub(&Integer::one())),
                OrderingOperator::LessOrEqual => most = most.min(bound.clone()),
                OrderingOperator::Greater => least = least.max(bound.add(&Integer::one())),
                OrderingOperator::GreaterOrEqual => least = least.max(bound.clone()),
            }
        }
        if least > most {
            return Some(Self::Listed(Vec::new()));
        }
        Some(Self::Range {
            start: u128::from(least.sub(lower).to_u64()?),
            len: u128::from(most.sub(&least).add(&Integer::one()).to_u64()?),
        })
    }
}

/// An unused universe key an operation may create an object under.
struct Slot {
    population: DeclarationKey,
    key: String,
    types: Vec<DeclarationKey>,
}

/// One object of a candidate's object set.
struct FrameMember {
    population: DeclarationKey,
    key: String,
    type_key: DeclarationKey,
    reference: ObjectReference,
    created: bool,
}

/// One field a candidate chooses a value for.
struct Dimension<'f> {
    population: DeclarationKey,
    key: String,
    field: &'f StateField,
    domain: Domain,
    allowed: Allowed,
}

impl Dimension<'_> {
    fn radix(&self) -> u128 {
        match &self.allowed {
            Allowed::All => self
                .domain
                .len()
                .saturating_add(u128::from(self.field.optional)),
            Allowed::Range { len, .. } => *len,
            Allowed::Listed(positions) => positions.len() as u128,
        }
    }

    /// The value at `index`: `Some(None)` for an absent `?` field.
    fn value(&self, index: u128) -> Option<Option<Value>> {
        match &self.allowed {
            Allowed::All => {}
            Allowed::Range { start, .. } => {
                return self.domain.nth(start.checked_add(index)?).map(Some);
            }
            Allowed::Listed(positions) => {
                let position = positions.get(usize::try_from(index).ok()?)?;
                return self.domain.nth(*position).map(Some);
            }
        }
        if self.field.optional {
            match index.checked_sub(1) {
                None => Some(None),
                Some(position) => self.domain.nth(position).map(Some),
            }
        } else {
            self.domain.nth(index).map(Some)
        }
    }
}

/// A refusal by the frame decision, with its catalog code.
#[derive(Clone)]
struct FrameRefused(CatalogCode);

/// A candidate's observation. Its state key is encoded only for a finding
/// that names it.
type Evidence = Observation;

/// The (population, object key, field) triples a frame lets an application
/// write.
type WritableFields = BTreeSet<(DeclarationKey, String, DeclarationKey)>;

/// What the frame decision found: the created and deleted objects, by
/// population and key.
struct Delta {
    created: Vec<(DeclarationKey, String)>,
    deleted: Vec<(DeclarationKey, String)>,
}

struct CompletionContext<'a> {
    candidate: (ModelState, Vec<WrittenSlots>),
    delta: &'a Delta,
    post_references: &'a [ObjectReference],
    budget: &'a mut Budget,
    contract: &'a mut Vec<ContractFinding>,
    successors: &'a mut Vec<Applied>,
    result_independent: bool,
}

impl Run<'_, '_, '_> {
    fn effect(&self) -> &OperationEffect {
        self.application.declaration.effect()
    }

    /// The objects the frame lets this application delete: a type-wide
    /// `deletes` entry, `deletes self` for the receiver, and `deletes p` for
    /// the argument bound to `p`.
    fn deletable(&self) -> Vec<&Row> {
        let system = self.walk.system;
        let effect = self.effect();
        self.walk
            .rows
            .iter()
            .filter(|candidate| {
                let by_type = effect.deletes.iter().any(|grant| {
                    system
                        .model
                        .type_identity(grant)
                        .is_some_and(|target| system.types.conforms(candidate.type_id(), target))
                });
                let is_receiver = candidate.reference == self.row.reference;
                let by_argument = self.args.iter().any(|(name, value)| {
                    effect.deletes_arguments.contains(name)
                        && matches!(value, Value::Reference(bound) if *bound == candidate.reference)
                });
                by_type || (effect.deletes_receiver && is_receiver) || by_argument
            })
            .collect()
    }

    /// The unused universe keys this application may create objects under,
    /// each with the non-abstract types a created object may have: a type
    /// that conforms to a `creates` entry and to a member type of the
    /// population (FR-120 "Post-states").
    fn slots(&self) -> Vec<Slot> {
        let system = self.walk.system;
        let creatable = system.creatable_types(self.effect());
        let mut slots = Vec::new();
        for (population, keys) in &system.universes {
            let members: Vec<_> = system
                .populations
                .iter()
                .filter(|declared| declared.key == *population)
                .flat_map(|declared| declared.member_types.iter())
                .filter_map(|member| system.model.type_identity(member))
                .collect();
            let types: Vec<DeclarationKey> = creatable
                .iter()
                .filter(|candidate| {
                    system
                        .model
                        .type_identity(candidate)
                        .is_some_and(|identity| {
                            members.iter().any(|member| {
                                system.types.conforms(identity.clone(), member.clone())
                            })
                        })
                })
                .cloned()
                .collect();
            if types.is_empty() {
                continue;
            }
            let used = self.walk.state.state.populations.get(population);
            for key in keys {
                if used.is_some_and(|objects| objects.contains_key(key)) {
                    continue;
                }
                slots.push(Slot {
                    population: population.clone(),
                    key: key.clone(),
                    types: types.clone(),
                });
            }
        }
        slots
    }

    /// FR-120 "Directly solved assignments" and FR-851 "Narrowing": what
    /// this application's postconditions say about its receiver's fields,
    /// each conjunct's `e` evaluated once with a fresh meter, or `None` when
    /// the application is enumerated. It is enumerated when its frame
    /// deletes or creates, a clause names `result`, two equations assign one
    /// field, or an `e` has no value (`Undefined`, `Refused`, `Incomplete`,
    /// or a non-integer bound), so that its findings and stops are the
    /// enumeration's.
    fn narrowing(&self) -> Result<Option<Narrowing>, ExpansionStop> {
        let effect = self.effect();
        if self.walk.system.shares_evaluation_meter()
            || self.postconditions.is_empty()
            || !effect.creates.is_empty()
            || !effect.deletes.is_empty()
            || effect.deletes_receiver
            || !effect.deletes_arguments.is_empty()
        {
            return Ok(None);
        }
        // `e` reads no `result` and no post-state: the clause is bound over
        // the pre-state as its post-state, with a result of the operation's
        // type standing in for the one it never reads.
        let result = match self.application.declaration.result() {
            None => None,
            Some(result_type) => {
                let domain = self.walk.value_domain(result_type, &self.walk.references());
                match domain.and_then(|domain| domain.nth(0)) {
                    Some(value) => Some(value),
                    None => return Ok(None),
                }
            }
        };
        let mut narrowing = Narrowing::new();
        for info in self.postconditions {
            let observations = AdmittedObservations {
                usage: AdmissionUsage::default(),
                clause: info.identity.clone(),
                current: None,
                pre: Some(self.walk.observation.clone()),
                post: Some(self.walk.observation.clone()),
                self_object: self.row.reference.clone(),
                self_population: self
                    .walk
                    .observation
                    .environment
                    .population_node(&self.row.population.node)
                    .cloned(),
                invocation: None,
                self_kind: SelfKind::Object,
                receiver_pre: None,
                receiver_post: None,
                parameters: self.args.to_vec(),
                value_origins: Default::default(),
                result: result.clone(),
                created: Vec::new(),
                deleted: Vec::new(),
            };
            let found = self.walk.system.evaluator().evaluate_post_targets(
                &info.qualified,
                &observations,
                &mut || {
                    let meter = self.walk.system.fresh_meter();
                    match &self.walk.cancel {
                        Some(cancel) => meter.with_cancel(cancel.clone()),
                        None => meter,
                    }
                },
            );
            let Ok(found) = found else {
                return Ok(None);
            };
            if found.names_result {
                return Ok(None);
            }
            for target in found.targets {
                match target {
                    PostTarget::Equation { field, value } => {
                        let size_bits = self
                            .walk
                            .system
                            .max_value_bits
                            .load(std::sync::atomic::Ordering::Relaxed);
                        if size_bits > 0
                            && self
                                .walk
                                .system
                                .fields
                                .values()
                                .flatten()
                                .find(|candidate| candidate.name == field)
                                .is_some_and(|candidate| {
                                    matches!(&candidate.value_type, ValueType::Integer)
                                })
                        {
                            let field_type = self
                                .walk
                                .system
                                .fields
                                .values()
                                .flatten()
                                .find(|candidate| candidate.name == field)
                                .map(|candidate| &candidate.value_type);
                            if field_type
                                .and_then(|field_type| {
                                    self.walk.value_domain(field_type, &self.walk.references())
                                })
                                .is_some_and(|domain| match value.as_ref() {
                                    Some(value @ Value::Integer(_)) => {
                                        domain.index_of(value).is_none()
                                    }
                                    _ => false,
                                })
                            {
                                self.walk.system.record_value_bound_cut(
                                    &self.application.text,
                                    &format!("{:?}", self.walk.state.state()),
                                );
                            }
                        }
                        narrowing.entry(field).or_default().equations.push(value);
                    }
                    PostTarget::Bound {
                        field,
                        operator,
                        value,
                    } => {
                        narrowing
                            .entry(field)
                            .or_default()
                            .bounds
                            .push((operator, value));
                    }
                }
            }
        }
        Ok(Some(narrowing))
    }

    /// Generate every candidate post-state of this application, one at a
    /// time, in canonical order, counting each against the bound.
    fn candidates(
        &self,
        budget: &mut Budget,
        contract: &mut Vec<ContractFinding>,
        successors: &mut Vec<Applied>,
    ) -> Result<(), ExpansionStop> {
        self.generate_candidates(budget, contract, successors, false)
    }

    /// Find the first completed candidate post-state, if one exists. Access
    /// classification needs to know whether an application completes, but
    /// without an attempt constraint it does not need every post-state.
    fn first_candidate(
        &self,
        budget: &mut Budget,
        contract: &mut Vec<ContractFinding>,
        successors: &mut Vec<Applied>,
    ) -> Result<bool, ExpansionStop> {
        let initial = successors.len();
        self.generate_candidates(budget, contract, successors, true)?;
        Ok(successors.len() > initial)
    }

    fn generate_candidates(
        &self,
        budget: &mut Budget,
        contract: &mut Vec<ContractFinding>,
        successors: &mut Vec<Applied>,
        stop_after_first: bool,
    ) -> Result<(), ExpansionStop> {
        let deletable = self.deletable();
        let slots = self.slots();
        let writable = self.writable()?;
        let narrowing = self.narrowing()?;
        let result_independent = narrowing.is_some() && self.operation_postconditions.is_empty();
        let plan = Plan {
            slots: &slots,
            writable: &writable,
            stop_after_first,
            result_independent,
            narrowing: narrowing.as_ref(),
        };
        let mut deletions = Odometer::new(vec![2; deletable.len()]);
        while let Some(deleted) = deletions.next_digits() {
            let deleted: BTreeSet<(&DeclarationKey, &str)> = deletable
                .iter()
                .zip(&deleted)
                .filter(|(_, chosen)| **chosen == 1)
                .map(|(row, _)| (&row.population, row.key.as_str()))
                .collect();
            let mut creations = Odometer::new(
                slots
                    .iter()
                    .map(|slot| 1 + slot.types.len() as u128)
                    .collect(),
            );
            while let Some(created) = creations.next_digits() {
                let found =
                    self.fields_of_choice(&deleted, &plan, &created, budget, contract, successors)?;
                if stop_after_first && found {
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// The post-state object set for one deletion and creation choice, and
    /// every assignment of values to the fields the frame lets the operation
    /// write.
    fn fields_of_choice(
        &self,
        deleted: &BTreeSet<(&DeclarationKey, &str)>,
        plan: &Plan<'_>,
        created: &[u128],
        budget: &mut Budget,
        contract: &mut Vec<ContractFinding>,
        successors: &mut Vec<Applied>,
    ) -> Result<bool, ExpansionStop> {
        let initial = successors.len();
        let system = self.walk.system;
        let mut members: Vec<FrameMember> = Vec::new();
        for row in &self.walk.rows {
            if deleted.contains(&(&row.population, row.key.as_str())) {
                continue;
            }
            let type_key = system
                .model
                .type_key(row.type_id())
                .cloned()
                .ok_or_else(|| stop(BROKEN))?;
            members.push(FrameMember {
                population: row.population.clone(),
                key: row.key.clone(),
                type_key,
                reference: row.reference.clone(),
                created: false,
            });
        }
        for (slot, choice) in plan.slots.iter().zip(created) {
            let Some(position) = choice.checked_sub(1) else {
                continue;
            };
            let type_key = usize::try_from(position)
                .ok()
                .and_then(|position| slot.types.get(position))
                .cloned()
                .ok_or_else(|| stop(BROKEN))?;
            let reference = system
                .model
                .object_reference(&type_key, &slot.key)
                .map_err(|_| stop(BROKEN))?;
            members.push(FrameMember {
                population: slot.population.clone(),
                key: slot.key.clone(),
                type_key,
                reference,
                created: true,
            });
        }
        members.sort_by(|left, right| {
            (&left.population, &left.key).cmp(&(&right.population, &right.key))
        });
        let post_references: Vec<ObjectReference> = members
            .iter()
            .map(|member| member.reference.clone())
            .collect();

        let mut dimensions = Vec::new();
        let mut enumerated = false;
        for member in &members {
            let Some(fields) = system.fields.get(&member.type_key) else {
                return Err(stop(BROKEN));
            };
            for field in fields {
                if !member.created
                    && !plan.writable.contains(&(
                        member.population.clone(),
                        member.key.clone(),
                        field.key.clone(),
                    ))
                {
                    continue;
                }
                let domain = self
                    .walk
                    .field_domain(&member.type_key, field, &post_references)
                    .ok_or_else(|| stop(BROKEN))?;
                // `None` marks a field whose targets send the whole
                // application back to enumeration.
                let allowed = match plan
                    .narrowing
                    .and_then(|narrowing| narrowing.get(&field.name))
                {
                    Some(targets)
                        if !member.created
                            && !field.optional
                            && member.reference == self.row.reference =>
                    {
                        Allowed::of(&domain, targets)
                    }
                    _ => Some(Allowed::All),
                };
                enumerated |= allowed.is_none();
                dimensions.push(Dimension {
                    population: member.population.clone(),
                    key: member.key.clone(),
                    field,
                    domain,
                    allowed: allowed.unwrap_or(Allowed::All),
                });
            }
        }
        if enumerated {
            for dimension in &mut dimensions {
                dimension.allowed = Allowed::All;
            }
        }

        // The populations a candidate of this choice may differ from the
        // state in; `build` changes no other.
        let touched: BTreeSet<&DeclarationKey> = deleted
            .iter()
            .map(|(population, _)| *population)
            .chain(
                plan.slots
                    .iter()
                    .zip(created)
                    .filter(|(_, choice)| **choice != 0)
                    .map(|(slot, _)| &slot.population),
            )
            .chain(dimensions.iter().map(|dimension| &dimension.population))
            .collect();
        let mut values = Odometer::new(dimensions.iter().map(Dimension::radix).collect());
        while let Some(digits) = values.next_digits() {
            budget.charge()?;
            let candidate =
                self.build(deleted, &members, plan.slots, created, &dimensions, &digits)?;
            if !is_closed(&candidate, &post_references, system)? {
                continue;
            }
            let Ok(delta) = self.decide_touched(&candidate, Some(&touched))? else {
                // Candidates are built inside the frame, so a refusal here is
                // a broken internal invariant.
                return Err(stop(BROKEN));
            };
            self.complete(
                CompletionContext {
                    candidate: self.collection_writes(candidate, &delta)?,
                    delta: &delta,
                    post_references: &post_references,
                    budget,
                    contract,
                    successors,
                    result_independent: plan.result_independent,
                },
            )?;
            if plan.stop_after_first && successors.len() > initial {
                return Ok(true);
            }
        }
        Ok(successors.len() > initial)
    }

    /// The candidate state one choice describes.
    fn build(
        &self,
        deleted: &BTreeSet<(&DeclarationKey, &str)>,
        members: &[FrameMember],
        slots: &[Slot],
        created: &[u128],
        dimensions: &[Dimension<'_>],
        digits: &[u128],
    ) -> Result<ModelState, ExpansionStop> {
        let state = &self.walk.state.state;
        let mut populations = state.populations.clone();
        for (population, key) in deleted {
            if let Some(objects) = populations.get_mut(*population) {
                objects.remove(*key);
            }
        }
        for (slot, choice) in slots.iter().zip(created) {
            if *choice == 0 {
                continue;
            }
            let member = members
                .iter()
                .find(|member| {
                    member.created && member.population == slot.population && member.key == slot.key
                })
                .ok_or_else(|| stop(BROKEN))?;
            populations
                .entry(slot.population.clone())
                .or_default()
                .insert(
                    slot.key.clone(),
                    StateObject {
                        type_identity: member.type_key.clone(),
                        fields: BTreeMap::new(),
                    },
                );
        }
        for (dimension, digit) in dimensions.iter().zip(digits) {
            let value = dimension.value(*digit).ok_or_else(|| stop(BROKEN))?;
            let object = populations
                .get_mut(&dimension.population)
                .and_then(|objects| objects.get_mut(&dimension.key))
                .ok_or_else(|| stop(BROKEN))?;
            match value {
                Some(value) => {
                    object.fields.insert(dimension.field.key.clone(), value);
                }
                None => {
                    object.fields.remove(&dimension.field.key);
                }
            }
        }
        let mut extents = state.extents.clone();
        let removed: Vec<&ObjectReference> = self
            .walk
            .rows
            .iter()
            .filter(|row| deleted.contains(&(&row.population, row.key.as_str())))
            .map(|row| &row.reference)
            .collect();
        for extent in &mut extents {
            extent.instances.retain(|instance| {
                !removed.contains(&&instance.source) && !removed.contains(&&instance.target)
            });
        }
        Ok(ModelState {
            populations,
            extents,
        })
    }

    /// The frame decision for `candidate` against the state, per population
    /// in ascending order (FR-120 "Frame and observation seam").
    fn decide(&self, candidate: &ModelState) -> Result<Result<Delta, FrameRefused>, ExpansionStop> {
        self.decide_touched(candidate, None)
    }

    /// [`Self::decide`], where every population outside `touched`, when
    /// given, is the state's own: its decision is the state's against
    /// itself, made once per reference-valued arguments, and `candidate`
    /// need not hold it.
    fn decide_touched(
        &self,
        candidate: &ModelState,
        touched: Option<&BTreeSet<&DeclarationKey>>,
    ) -> Result<Result<Delta, FrameRefused>, ExpansionStop> {
        let pre = &self.walk.state.state;
        let empty = BTreeMap::new();
        let populations: BTreeSet<&DeclarationKey> = pre
            .populations
            .keys()
            .chain(candidate.populations.keys())
            .collect();
        let mut delta = Delta {
            created: Vec::new(),
            deleted: Vec::new(),
        };
        for population in populations {
            let before = pre.populations.get(population).unwrap_or(&empty);
            let decided = if touched.is_some_and(|touched| !touched.contains(population)) {
                self.unchanged(population, before)?
            } else {
                let after = candidate.populations.get(population).unwrap_or(&empty);
                self.decide_population(population, before, after)?
            };
            match decided {
                Err(refused) => return Ok(Err(refused)),
                Ok((created, deleted)) => {
                    delta
                        .created
                        .extend(created.into_iter().map(|key| (population.clone(), key)));
                    delta
                        .deleted
                        .extend(deleted.into_iter().map(|key| (population.clone(), key)));
                }
            }
        }
        Ok(Ok(delta))
    }

    /// The decision for `population` left as the state holds it (`before`),
    /// once per reference-valued arguments.
    fn unchanged(
        &self,
        population: &DeclarationKey,
        before: &BTreeMap<String, StateObject>,
    ) -> Result<PopulationDecided, ExpansionStop> {
        let key = (self.references.clone(), population.clone());
        if let Some(decided) = self.frames.unchanged.borrow().get(&key) {
            return Ok(decided.clone());
        }
        let decided = self.decide_population(population, before, before)?;
        self.frames
            .unchanged
            .borrow_mut()
            .insert(key, decided.clone());
        Ok(decided)
    }

    /// The frame decision for `population` from `before` to `after`.
    fn decide_population(
        &self,
        population: &DeclarationKey,
        before: &BTreeMap<String, StateObject>,
        after: &BTreeMap<String, StateObject>,
    ) -> Result<PopulationDecided, ExpansionStop> {
        let system = self.walk.system;
        let created: Vec<String> = after
            .keys()
            .filter(|key| !before.contains_key(*key))
            .cloned()
            .collect();
        let deleted: Vec<String> = before
            .keys()
            .filter(|key| !after.contains_key(*key))
            .cloned()
            .collect();
        let arguments: Vec<(String, String)> = self
            .args
            .iter()
            .filter_map(|(name, value)| match value {
                Value::Reference(bound)
                    if before.get(bound.object().as_str()).is_some_and(|object| {
                        system.model.type_identity(&object.type_identity).as_ref()
                            == Some(bound.object_type())
                    }) =>
                {
                    Some((name.clone(), bound.object().as_str().to_owned()))
                }
                _ => None,
            })
            .collect();
        let index = system
            .model
            .population_index(population)
            .ok_or_else(|| stop(BROKEN))?;
        let frame = FrameDecision {
            index,
            ancestor_steps: system.model.ancestor_steps(),
            effect: self.effect(),
            declared_created: &created,
            declared_deleted: &deleted,
            receiver: (self.row.population == *population).then_some(self.row.key.as_str()),
            arguments: &arguments,
        };
        let outcome = decide_frame(
            &frame,
            &frame_objects(before),
            &frame_objects(after),
            |_, left, right| match (left, right) {
                (None, None) => true,
                (Some(left), Some(right)) => values_equal(left, right),
                (Some(_), None) | (None, Some(_)) => false,
            },
            &mut |_| Ok::<(), Infallible>(()),
        );
        match outcome {
            Err(FrameStop::Refused(refusal)) => Ok(Err(FrameRefused(refusal.catalog_code()))),
            Err(FrameStop::Denied(never)) => match never {},
            Ok(found) => Ok(Ok((
                found.created.into_iter().collect(),
                found.deleted.into_iter().collect(),
            ))),
        }
    }

    /// [`Self::writable_fields`], decided once per application for the
    /// argument vectors that agree on their reference-valued arguments.
    fn writable(&self) -> Result<Rc<WritableFields>, ExpansionStop> {
        if let Some(writable) = self.frames.writable.borrow().get(&self.references) {
            return Ok(Rc::clone(writable));
        }
        let writable = Rc::new(self.writable_fields()?);
        self.frames
            .writable
            .borrow_mut()
            .insert(self.references.clone(), Rc::clone(&writable));
        Ok(writable)
    }

    /// Every (population, object, field) the frame lets this application
    /// write, decided by [`decide_frame`] alone: a probe state changes that
    /// one field, and the frame decision accepts or refuses it.
    fn writable_fields(&self) -> Result<WritableFields, ExpansionStop> {
        let system = self.walk.system;
        let pre = &self.walk.state.state;
        let references = self.walk.references();
        let mut writable = BTreeSet::new();
        for row in &self.walk.rows {
            let type_key = system
                .model
                .type_key(row.type_id())
                .ok_or_else(|| stop(BROKEN))?;
            let fields = system.fields.get(type_key).ok_or_else(|| stop(BROKEN))?;
            for field in fields {
                let current = pre
                    .populations
                    .get(&row.population)
                    .and_then(|objects| objects.get(&row.key))
                    .and_then(|object| object.fields.get(&field.key));
                // A field with no finite domain (an `Integer`, a string, an
                // unbounded collection) is bounded by no frame that grants it,
                // so it cannot be written here.
                let Some(domain) = self.walk.value_domain(&field.value_type, &references) else {
                    continue;
                };
                let mut different: Option<Option<Value>> = None;
                if field.optional && current.is_some() {
                    different = Some(None);
                } else {
                    for index in 0..domain.len().min(8) {
                        let value = domain.nth(index).ok_or_else(|| stop(BROKEN))?;
                        if current.is_none_or(|held| !values_equal(held, &value)) {
                            different = Some(Some(value));
                            break;
                        }
                    }
                }
                // A field whose domain holds no other value cannot change.
                let Some(different) = different else {
                    continue;
                };
                // The probe holds only the row's population, the one it
                // changes; every other is decided as the state's own.
                let mut objects = pre
                    .populations
                    .get(&row.population)
                    .cloned()
                    .ok_or_else(|| stop(BROKEN))?;
                let object = objects.get_mut(&row.key).ok_or_else(|| stop(BROKEN))?;
                match different {
                    Some(value) => {
                        object.fields.insert(field.key.clone(), value);
                    }
                    None => {
                        object.fields.remove(&field.key);
                    }
                }
                let probe = ModelState {
                    populations: BTreeMap::from([(row.population.clone(), objects)]),
                    extents: Vec::new(),
                };
                let touched = BTreeSet::from([&row.population]);
                if self.decide_touched(&probe, Some(&touched))?.is_ok() {
                    writable.insert((row.population.clone(), row.key.clone(), field.key.clone()));
                }
            }
        }
        Ok(writable)
    }

    /// The effect of the step to `candidate`, with `result` as its result.
    fn effect_of(
        &self,
        candidate: &ModelState,
        delta: &Delta,
    ) -> Result<StepEffect, ExpansionStop> {
        let system = self.walk.system;
        let pre = &self.walk.state.state;
        let reference = |state: &ModelState, population: &DeclarationKey, key: &str| {
            state
                .populations
                .get(population)
                .and_then(|objects| objects.get(key))
                .ok_or_else(|| stop(BROKEN))
                .and_then(|object| {
                    system
                        .model
                        .object_reference(&object.type_identity, key)
                        .map_err(|_| stop(BROKEN))
                })
        };
        let mut created = delta
            .created
            .iter()
            .map(|(population, key)| reference(candidate, population, key))
            .collect::<Result<Vec<_>, _>>()?;
        let mut deleted = delta
            .deleted
            .iter()
            .map(|(population, key)| reference(pre, population, key))
            .collect::<Result<Vec<_>, _>>()?;
        created.sort();
        deleted.sort();

        let mut changed: Vec<FieldChange> = Vec::new();
        for (population, objects) in &pre.populations {
            for (key, before) in objects {
                let Some(after) = candidate
                    .populations
                    .get(population)
                    .and_then(|survivors| survivors.get(key))
                else {
                    continue;
                };
                let fields = system
                    .fields
                    .get(&before.type_identity)
                    .ok_or_else(|| stop(BROKEN))?;
                for field in fields {
                    let (left, right) =
                        (before.fields.get(&field.key), after.fields.get(&field.key));
                    let same = match (left, right) {
                        (None, None) => true,
                        (Some(left), Some(right)) => values_equal(left, right),
                        (Some(_), None) | (None, Some(_)) => false,
                    };
                    if same {
                        continue;
                    }
                    changed.push(FieldChange {
                        object: reference(pre, population, key)?,
                        field: field.name.clone(),
                        pre: left
                            .map(|value| text_of(value, system.typing()))
                            .transpose()?,
                        post: right
                            .map(|value| text_of(value, system.typing()))
                            .transpose()?,
                    });
                }
            }
        }
        changed.sort_by(|left, right| left.object.cmp(&right.object));
        Ok(StepEffect {
            created,
            deleted,
            changed,
            result: None,
            written: Vec::new(),
        })
    }

    /// The result domain over the post-state and its values' positions in
    /// ascending order of their typed canonical bytes; `None` for an
    /// operation with no result. Only the order is held, never the values:
    /// each pair is read lazily and counted when it is evaluated.
    ///
    /// A result domain with more than [`RESULT_ORDER_LIMIT`] values cannot be
    /// ordered and stops the expansion.
    fn result_order(
        &self,
        post_references: &[ObjectReference],
    ) -> Result<Option<(Domain, Vec<u128>)>, ExpansionStop> {
        let Some(result_type) = self.application.declaration.result() else {
            return Ok(None);
        };
        let domain = self
            .walk
            .value_domain(result_type, post_references)
            .ok_or_else(|| stop(BROKEN))?;
        if domain.len() > RESULT_ORDER_LIMIT {
            return Err(stop(EXHAUSTED));
        }
        let known = self
            .result_order_cache
            .borrow()
            .as_ref()
            .filter(|(references, _)| references == post_references)
            .map(|(_, order)| order.clone());
        let order = match known {
            Some(order) => order,
            None => {
                let mut keyed = Vec::new();
                for index in 0..domain.len() {
                    let value = domain.nth(index).ok_or_else(|| stop(BROKEN))?;
                    let bytes = quire_canonical::to_vec(
                        &value_enc(&value, self.walk.system.typing()),
                        text_limits(),
                    )
                    .map_err(|_| stop(BROKEN))?;
                    keyed.push((bytes, index));
                }
                keyed.sort();
                let order: Vec<u128> = keyed.into_iter().map(|(_, index)| index).collect();
                *self.result_order_cache.borrow_mut() =
                    Some((post_references.to_vec(), order.clone()));
                order
            }
        };
        Ok(Some((domain, order)))
    }

    /// The conjunction of this application's postconditions over `candidate`
    /// with `result`, reading the candidate's key and observation from
    /// `evidence`, which it fills on first use.
    fn post_conjunction(
        &self,
        evidence: &mut Option<Evidence>,
        candidate: &ModelState,
        effect: &StepEffect,
        result: Option<&Value>,
    ) -> Result<Conjunction, ExpansionStop> {
        if self.postconditions.is_empty() && self.operation_postconditions.is_empty() {
            return Ok(Conjunction::True);
        }
        let system = self.walk.system;
        if evidence.is_none() {
            *evidence = Some(self.post_observation(candidate)?);
        }
        let post = evidence.as_ref().ok_or_else(|| stop(BROKEN))?;
        let check = self
            .walk
            .enforcement
            .zip(self.footprint)
            .map(|(enforcement, footprint)| {
                ReadCheck::new(&system.model, enforcement, footprint, self.transition)
            });
        let verdict = conjunction(self.walk, self.postconditions, |info| {
            let observations = AdmittedObservations {
                usage: AdmissionUsage::default(),
                clause: info.identity.clone(),
                current: None,
                pre: Some(self.walk.observation.clone()),
                post: Some(post.clone()),
                self_object: self.row.reference.clone(),
                self_population: self
                    .walk
                    .observation
                    .environment
                    .population_node(&self.row.population.node)
                    .cloned(),
                invocation: None,
                self_kind: SelfKind::Object,
                receiver_pre: None,
                receiver_post: None,
                parameters: self.args.to_vec(),
                value_origins: Default::default(),
                result: result.cloned(),
                created: effect.created.clone(),
                deleted: effect.deleted.clone(),
            };
            match &check {
                Some(check) => check.attach(observations),
                None => observations,
            }
        })?;
        if let Some(check) = &check {
            check.verify()?;
        }
        if self.operation_postconditions.is_empty() {
            return Ok(verdict);
        }
        operation_conjunction(
            self.walk,
            self.operation_postconditions,
            &fsl_semantics::model::observation::OperationContractObservations {
                pre: self.walk.observation.clone(),
                post: post.clone(),
                self_reference: self.row.reference.clone(),
                parameters: self.args.to_vec(),
                result: result.cloned(),
            },
            verdict,
        )
    }

    /// `candidate` with every clock collection the application's
    /// postconditions write whole (FR-230 "Collection-valued writes") set to
    /// the value its write computes, and the provenance of each written
    /// slot. A record collection has no value domain to enumerate (FR-120),
    /// so the write is computed rather than chosen; a frame that does not
    /// grant the field leaves it to the postcondition to decide.
    ///
    /// # Errors
    ///
    /// A [`COLLECTION_WRITE`] stop when a write cannot be computed from the
    /// pre-state: it reads the collection it carries from in the post-state
    /// (STD-493 item 28), an evaluation it needs has no value, or its value
    /// changes with the written field. Such a step is never dropped silently.
    fn collection_writes(
        &self,
        mut candidate: ModelState,
        delta: &Delta,
    ) -> Result<(ModelState, Vec<WrittenSlots>), ExpansionStop> {
        let system = self.walk.system;
        let mut written: Vec<WrittenSlots> = Vec::new();
        for info in self.postconditions {
            let writes = system
                .input()
                .timed_models
                .iter()
                .flat_map(|model| model.resets())
                .filter(|resets| resets.clause() == info.name)
                .flat_map(|resets| resets.writes());
            for write in writes {
                if written.iter().any(|done| done.field == write.field()) {
                    continue;
                }
                let field_key = system
                    .fields
                    .get(
                        system
                            .model
                            .type_key(self.row.type_id())
                            .ok_or_else(|| stop(BROKEN))?,
                    )
                    .and_then(|fields| fields.iter().find(|field| field.name == write.field()))
                    .map(|field| field.key.clone())
                    .ok_or_else(|| stop(BROKEN))?;
                let (value, _) = self.computed(&candidate, delta, info, write)?;
                let mut next = candidate.clone();
                next.populations
                    .get_mut(&self.row.population)
                    .and_then(|objects| objects.get_mut(&self.row.key))
                    .ok_or_else(|| stop(BROKEN))?
                    .fields
                    .insert(field_key, value.clone());
                if self.decide(&next)?.is_err() {
                    continue;
                }
                // Computed again over the state it writes: a write that is a
                // function of the pre-state computes the same value.
                let (again, occurrences) = self.computed(&next, delta, info, write)?;
                if !values_equal(&again, &value) {
                    return Err(stop(COLLECTION_WRITE));
                }
                let provenance =
                    slot_provenance(&value, occurrences).ok_or_else(|| stop(BROKEN))?;
                written.push(WrittenSlots {
                    field: write.field().to_owned(),
                    provenance,
                });
                candidate = next;
            }
        }
        Ok((candidate, written))
    }

    /// The observation of `candidate` as a post-state, with
    /// the presence of each guarded clock place it holds, which a
    /// postcondition's `present(self.v)` reads (FSpec FR-322 `CW-05`).
    fn post_observation(&self, candidate: &ModelState) -> Result<Observation, ExpansionStop> {
        let mut observation = self
            .walk
            .system
            .model
            .observation(candidate, state_document())
            .map_err(|_| stop(BROKEN))?;
        if let Some(presence) = self.walk.post_presence {
            observation
                .environment
                .set_place_presence(presence(candidate));
        }
        Ok(observation)
    }

    /// The observations of the postcondition `info` with `candidate` as
    /// the post-state.
    fn post_observations(
        &self,
        candidate: &ModelState,
        delta: &Delta,
        info: &ClauseInfo,
    ) -> Result<AdmittedObservations, ExpansionStop> {
        let post = self.post_observation(candidate)?;
        let effect = self.effect_of(candidate, delta)?;
        Ok(AdmittedObservations {
            usage: AdmissionUsage::default(),
            clause: info.identity.clone(),
            current: None,
            pre: Some(self.walk.observation.clone()),
            post: Some(post),
            self_object: self.row.reference.clone(),
            self_population: self
                .walk
                .observation
                .environment
                .population_node(&self.row.population.node)
                .cloned(),
            invocation: None,
            self_kind: SelfKind::Object,
            receiver_pre: None,
            receiver_post: None,
            parameters: self.args.to_vec(),
            value_origins: Default::default(),
            result: None,
            created: effect.created,
            deleted: effect.deleted,
        })
    }

    /// The value `write` of the postcondition `info` computes with
    /// `candidate` as the post-state, and its occurrences.
    fn computed(
        &self,
        candidate: &ModelState,
        delta: &Delta,
        info: &ClauseInfo,
        write: &CollectionWrite,
    ) -> Result<(Value, Vec<(Value, Provenance)>), ExpansionStop> {
        let system = self.walk.system;
        let observations = self.post_observations(candidate, delta, info)?;
        match system.evaluator().evaluate_collection_write(
            &info.qualified,
            &observations,
            write.field(),
            write.written(),
            &mut self.walk.meter(),
        ) {
            Ok(Some(WrittenCollection::Computed { value, occurrences })) => {
                Ok((value, occurrences))
            }
            Ok(None) => Err(stop(BROKEN)),
            Ok(Some(WrittenCollection::ReadsPostState | WrittenCollection::Undecided(_)))
            | Err(_) => Err(stop(COLLECTION_WRITE)),
        }
    }

    /// Pair `candidate` with each result and keep it as a successor when some
    /// pair's effective postcondition is true, its effect naming the
    /// collections `written`.
    fn complete(&self, context: CompletionContext<'_>) -> Result<(), ExpansionStop> {
        let CompletionContext {
            candidate: (candidate, written),
            delta,
            post_references,
            budget,
            contract,
            successors,
            result_independent,
        } = context;
        let mut effect = self.effect_of(&candidate, delta)?;
        effect.written = written;
        let plain = if self.walk.candidate_visitor.is_some() {
            self.application
                .declaration
                .result()
                .map(|result_type| {
                    self.walk
                        .value_domain(result_type, post_references)
                        .ok_or_else(|| stop(BROKEN))
                })
                .transpose()?
        } else {
            None
        };
        let ordered = if self.walk.candidate_visitor.is_some() {
            None
        } else {
            self.result_order(post_references)?
        };
        let count = plain
            .as_ref()
            .map(Domain::len)
            .or_else(|| ordered.as_ref().map(|(_, order)| order.len() as u128))
            .unwrap_or(1);
        let mut evidence: Option<Evidence> = None;
        let mut independent_verdict: Option<Conjunction> = None;
        for pick in 0..count {
            let result = match (&plain, &ordered) {
                (Some(domain), None) => Some(domain.nth(pick).ok_or_else(|| stop(BROKEN))?),
                (None, Some((domain, order))) => Some(
                    domain
                        .nth(
                            *order
                                .get(usize::try_from(pick).map_err(|_| stop(BROKEN))?)
                                .ok_or_else(|| stop(BROKEN))?,
                        )
                        .ok_or_else(|| stop(BROKEN))?,
                ),
                (None, None) => None,
                (Some(_), Some(_)) => return Err(stop(BROKEN)),
            };
            if self.application.declaration.result().is_some() {
                budget.charge()?;
            }
            let verdict = match (&independent_verdict, result_independent) {
                (Some(verdict), true) => verdict.clone(),
                _ => {
                    let verdict =
                        self.post_conjunction(&mut evidence, &candidate, &effect, result.as_ref())?;
                    if result_independent {
                        independent_verdict = Some(verdict.clone());
                    }
                    verdict
                }
            };
            match verdict {
                Conjunction::True => {
                    effect.result = result
                        .as_ref()
                        .map(|value| text_of(value, self.walk.system.typing()))
                        .transpose()?;
                    let applied = Applied {
                        transition: self.transition.clone(),
                        state: self.walk.state.reached(
                            candidate.clone(),
                            handler_anchor(&self.application.text),
                            effect.clone(),
                        ),
                        receiver: self.row.reference.clone(),
                        arguments: self.args.iter().map(|(_, value)| value.clone()).collect(),
                        result,
                    };
                    if self.walk.candidate_visitor.is_some() {
                        retain_outcome(self.walk, &applied)?;
                        successors.try_reserve(1).map_err(|_| stop(EXHAUSTED))?;
                        successors.push(applied);
                        continue;
                    }
                    match self.walk.result_policy {
                        super::walk::ResultPolicy::FirstResult => {
                            successors.push(applied);
                            return Ok(());
                        }
                        super::walk::ResultPolicy::AllResults => successors.push(applied),
                    }
                }
                Conjunction::False(_) => {}
                Conjunction::Undecided(undecided) => {
                    let state_key = match self.walk.key_policy {
                        Some(policy) => policy.key(self.walk.system, &candidate),
                        None => key::canonical_state_key(
                            &encode::model_key(
                                &self.walk.system.model,
                                &candidate,
                                self.walk.system.typing(),
                            ),
                            identity_limits(),
                        )
                        .map_err(|_| stop(BROKEN)),
                    }?;
                    for (clause, evaluation) in undecided {
                        if self.walk.candidate_visitor.is_some() {
                            self.walk.visit(super::NativeCandidateEvent::Undetermined {
                                transition: self.transition,
                                outcome: &evaluation,
                            })?;
                            continue;
                        }
                        super::key_budget::reserve(self.walk.key_policy, contract, 1)?;
                        contract.push(ContractFinding {
                            transition: self.transition_bytes.to_vec(),
                            candidate: Some(super::key_budget::copy_bytes(
                                self.walk.key_policy,
                                state_key.as_bytes(),
                            )?),
                            clause: clause.clone(),
                            finding: ModelFinding::ContractUndetermined {
                                transition: self.transition.clone(),
                                candidate: Some(crate::simulation::StateKey::from_canonical_bytes(
                                    super::key_budget::copy_bytes(
                                        self.walk.key_policy,
                                        state_key.as_bytes(),
                                    )?,
                                )),
                                clause,
                                evaluation,
                            },
                            case: Box::new(FindingCase {
                                receiver: self.row.reference.clone(),
                                arguments: self
                                    .args
                                    .iter()
                                    .map(|(_, value)| value.clone())
                                    .collect(),
                                post: Some((candidate.clone(), result.clone())),
                            }),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

/// Bound the retained outcome collection before allocating another entry.
/// This is one implicit sequence per invocation: its highwater size includes
/// the sequence root, each state/population/object container, every field value
/// occurrence and each result. ValueOccurrences bounds that one collection;
/// ResultRetain charges newly retained occurrences cumulatively. Duplicate raw
/// outcomes count until deduplication, so sorting cannot hide retained memory.
fn retain_outcome(walk: &Walk<'_, '_>, next: &Applied) -> Result<(), ExpansionStop> {
    let occurrences = |applied: &Applied| {
        let mut count = filament_exact::Integer::from(2_i64);
        for argument in &applied.arguments {
            count = count.add(&argument.occ());
        }
        for objects in applied.state.state.populations.values() {
            count = count.add(&filament_exact::Integer::one());
            for object in objects.values() {
                count = count.add(&filament_exact::Integer::one());
                for value in object.fields.values() {
                    count = count.add(&value.occ());
                }
            }
        }
        if let Some(result) = &applied.result {
            count = count.add(&result.occ());
        }
        count
    };
    let next_size = occurrences(next);
    let size = walk.retained_occurrences.borrow().add(&next_size);
    walk.meter()
        .charge(
            filament_exact::Charge::new(filament_exact::ChargePoint::CollectionResultRetain)
                .exact_size(filament_exact::LimitKind::ValueOccurrences, size.clone())
                .exact_results(next_size),
        )
        .map_err(|incomplete| {
            super::walk::exhausted(
                crate::simulation::expansion::ExpansionLimit::EvaluationMeter(
                    incomplete.limit_kind,
                ),
            )
        })?;
    walk.retained_occurrences.replace(size);
    Ok(())
}

/// `objects` as the frame decision reads them.
fn frame_objects(objects: &BTreeMap<String, StateObject>) -> Vec<FrameObject<'_, &Value>> {
    objects
        .iter()
        .map(|(key, object)| FrameObject {
            object: key.as_str(),
            type_identity: object.type_identity.clone(),
            fields: object
                .fields
                .iter()
                .map(|(field, value)| (field.clone(), value))
                .collect(),
        })
        .collect()
}

/// Whether candidate references name present objects or declared clock places
/// owned by those objects. The frame decision still checks each field's type.
fn is_closed(
    candidate: &ModelState,
    references: &[ObjectReference],
    system: &ModelSystem<'_>,
) -> Result<bool, ExpansionStop> {
    let mut named = Vec::new();
    for object in candidate.populations.values().flat_map(BTreeMap::values) {
        for value in object.fields.values() {
            collect_references(value, &mut named)?;
        }
    }
    // Most candidates name no object: the set of present references is built
    // only when one does.
    if named.is_empty() {
        return Ok(true);
    }
    let present: BTreeSet<&ObjectReference> = references.iter().collect();
    let clocks = match system.input.clock_declarations {
        Some(table) => {
            fsl_semantics::model::clock_reference::DeclaredClockPlaces::with_declaration_table(
                system.input.timed_models,
                table,
            )
        }
        None => fsl_semantics::model::clock_reference::DeclaredClockPlaces::new(
            system.input.timed_models,
        ),
    };
    Ok(named.into_iter().all(|reference| {
        present.contains(reference)
            || references
                .iter()
                .any(|owner| clocks.contains(reference, owner, system.types))
    }))
}

/// Every object reference `value` holds.
pub(super) fn collect_references<'v>(
    value: &'v Value,
    out: &mut Vec<&'v ObjectReference>,
) -> Result<(), ExpansionStop> {
    match value {
        Value::Reference(reference) => out.push(reference),
        Value::Option(option) => {
            if let Some(payload) = option.payload() {
                collect_references(payload, out)?;
            }
        }
        Value::Collection(collection) => {
            for element in collection.elements() {
                collect_references(element, out)?;
            }
        }
        Value::Map(map) => {
            let Some(entries) = crate::value::ordered_map_entries(map) else {
                return Err(stop(BROKEN));
            };
            for (_, value) in entries {
                collect_references(value, out)?;
            }
        }
        Value::Boolean(_)
        | Value::Integer(_)
        | Value::Rational(_)
        | Value::Decimal(_)
        | Value::Float(_)
        | Value::Quantity(_)
        | Value::Text(_)
        | Value::String(_)
        | Value::Bytes(_)
        | Value::JsonObject(_)
        | Value::Uuid(_)
        | Value::Timestamp(_)
        | Value::Enum(_)
        | Value::ClockReference(_)
        | Value::Population(_)
        | Value::Composite(_)
        | Value::Union(_) => {}
    }
    Ok(())
}

/// A collection write that cannot be computed from the pre-state (FR-230
/// "Collection-valued writes"): the step is refused, never dropped.
pub(crate) const COLLECTION_WRITE: CatalogCode =
    CatalogCode::new("unsupported_construct", "collection-write-not-computed");

/// The provenance of each slot of `value`: the `k`-th slot equal to a value
/// is the `k`-th occurrence equal to it, so a `Set` or `OrderedSet` keeps
/// the first of equal occurrences and a `Bag` or `Sequence` keeps them all,
/// by value and never by spelling. `None` when a slot is no occurrence.
fn slot_provenance(
    value: &Value,
    mut occurrences: Vec<(Value, Provenance)>,
) -> Option<Vec<Provenance>> {
    let Value::Collection(collection) = value else {
        return None;
    };
    collection
        .elements()
        .iter()
        .map(|element| {
            let position = occurrences
                .iter()
                .position(|(occurrence, _)| values_equal(occurrence, element))?;
            Some(occurrences.remove(position).1)
        })
        .collect()
}

#[cfg(test)]
mod clock_reference_tests {
    use super::collect_references;
    use crate::test_clock_reference::clock_reference;
    use ix_trace_rs::trace;

    /// FR-230-AC-71: a clock-place reference is not an `ObjectReference`, so
    /// the collector gathers nothing from it.
    #[trace("FR-230-AC-71", "TC-3084")]
    #[test]
    fn a_clock_reference_collects_no_object_reference() {
        let value = clock_reference();
        let mut out = Vec::new();
        collect_references(&value, &mut out).expect("a leaf value");
        assert!(out.is_empty());
    }
}
