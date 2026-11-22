// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Applications and transition identities": the expansion of one
//! model state. It evaluates the state's invariants, then every application
//! of every object's operations, collecting the successors and the findings.

use filament_checked::key::DeclarationKey;
use filament_exact::{EffectiveId, ObjectReference, Value};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_semantics::model::object_environment::PlacePresences;
use fsl_semantics::model::observation::Observation;
use fsl_semantics::model::state::ModelState;

use super::footprints::Enforcement;
use super::{
    contracts, frames, state_document, Application, ClauseInfo, ModelFinding, ModelSimState,
    ModelSystem, BROKEN, EXHAUSTED,
};
use crate::simulation::expansion::{Expansion, ExpansionLimit, ExpansionStop};

/// A stop with `cause`.
pub(super) const fn stop(cause: filament_semantic_value::catalog::CatalogCode) -> ExpansionStop {
    ExpansionStop { cause, limit: None }
}

/// `resource_exhausted`/`insufficient-next-charge` naming the budget `limit`
/// that ran out.
pub(super) const fn exhausted(limit: ExpansionLimit) -> ExpansionStop {
    ExpansionStop {
        cause: EXHAUSTED,
        limit: Some(limit),
    }
}

/// The candidates one expansion has generated, against its bound
/// (FR-120 "Post-states", ADR-016 G-4).
pub(crate) struct Budget {
    used: u64,
    max: Option<u64>,
    cancel: Option<filament_exact::Cancel>,
    request_meter: Option<std::sync::Arc<std::sync::Mutex<filament_exact::Meter>>>,
    work_meter: Option<filament_exact::Meter>,
}

impl Budget {
    /// A budget of `max` candidates, none used.
    pub(super) const fn new(max: u64) -> Self {
        Self {
            used: 0,
            max: Some(max),
            cancel: None,
            request_meter: None,
            work_meter: None,
        }
    }

    /// A lazy visitor retains at most one invocation's outcome collection.
    /// Its caller gates invocation work; raw frame/result trials charge
    /// evaluation work and poll cancellation independently of plan work.
    pub(super) fn visiting(
        cancel: Option<&filament_exact::Cancel>,
        system: &ModelSystem<'_>,
    ) -> Self {
        Self {
            used: 0,
            max: None,
            cancel: cancel.cloned(),
            request_meter: Some(system.shared_meter.clone()),
            work_meter: None,
        }
    }

    fn account_trials(&mut self, count: u64) -> Result<(), ExpansionStop> {
        if let Some(meter) = &self.request_meter {
            meter
                .lock()
                .expect("evaluation meter lock")
                .charge(
                    filament_exact::Charge::new(filament_exact::ChargePoint::CollectionVisit)
                        .work(filament_exact::Integer::from(count)),
                )
                .map_err(|incomplete| {
                    exhausted(ExpansionLimit::EvaluationMeter(incomplete.limit_kind))
                })?;
        }
        if let Some(meter) = &mut self.work_meter {
            meter
                .charge(
                    filament_exact::Charge::new(filament_exact::ChargePoint::ModelCandidate)
                        .work(filament_exact::Integer::from(count)),
                )
                .map_err(|incomplete| {
                    exhausted(ExpansionLimit::EvaluationMeter(incomplete.limit_kind))
                })?;
        }
        Ok(())
    }

    /// Count one candidate or (candidate, result) pair.
    ///
    /// # Errors
    ///
    /// `resource_exhausted`/`insufficient-next-charge` when the count would
    /// exceed the bound.
    pub(crate) fn charge(&mut self) -> Result<(), ExpansionStop> {
        self.charge_count(1)
    }
    /// The candidates counted so far.
    pub(super) const fn used(&self) -> u64 {
        self.used
    }

    /// Count `count` candidates at once.
    ///
    /// # Errors
    ///
    /// As [`Self::charge`].
    pub(super) fn charge_many(&mut self, count: usize) -> Result<(), ExpansionStop> {
        self.charge_count(u64::try_from(count).unwrap_or(u64::MAX))
    }

    fn charge_count(&mut self, count: u64) -> Result<(), ExpansionStop> {
        self.poll_cancel()?;
        if self
            .max
            .is_some_and(|max| count > max.saturating_sub(self.used))
        {
            return Err(exhausted(ExpansionLimit::MaxCandidates));
        }
        self.account_trials(count)?;
        self.used = self.used.saturating_add(count);
        Ok(())
    }

    pub(super) fn scoped(max: u64, system: &ModelSystem<'_>) -> Self {
        let mut budget = Self::new(max);
        budget.work_meter = system
            .probability_work
            .as_ref()
            .map(|_| system.fresh_meter());
        budget.cancel = system.probability_cancel.clone();
        budget
    }

    fn poll_cancel(&self) -> Result<(), ExpansionStop> {
        if self
            .cancel
            .as_ref()
            .is_some_and(filament_exact::Cancel::poll)
        {
            Err(exhausted(ExpansionLimit::EvaluationMeter(
                filament_exact::LimitKind::WorkUnits,
            )))
        } else {
            Ok(())
        }
    }
}

/// One object of the expanded state.
pub(super) struct Row {
    pub(super) population: DeclarationKey,
    pub(super) key: String,
    pub(super) reference: ObjectReference,
}

impl Row {
    /// The object's most-specific type.
    pub(super) fn type_id(&self) -> EffectiveId {
        self.reference.object_type().clone()
    }
}

/// What an expansion reads: the system, the expanded state, its observation
/// and its objects in ascending (population, key) order.
pub(super) struct Walk<'s, 'a> {
    pub(super) result_policy: ResultPolicy,
    pub(super) key_policy: Option<&'s dyn super::key_budget::KeyPolicy>,
    pub(super) cancel: Option<filament_exact::Cancel>,
    pub(super) system: &'s ModelSystem<'a>,
    pub(super) state: &'s ModelSimState,
    pub(super) observation: Observation,
    pub(super) rows: Vec<Row>,
    /// The footprints each step's clauses are held to, when the expansion
    /// enforces them.
    pub(super) enforcement: Option<&'s Enforcement<'s>>,
    /// Selector maxima from this timed subject's authenticated clock bound.
    pub(super) clock_selectors: Option<&'s super::domain::ClockSelectorMaxima>,
    /// The presence of each guarded clock place at a candidate post-state,
    /// which a postcondition's `present(self.v)` reads (FR-230 "Presence is
    /// derived", FSpec FR-322 `CW-05`); `None` outside a timed expansion.
    pub(super) post_presence: Option<PostPresence<'s>>,
    pub(super) candidate_visitor: Option<&'s dyn CandidateVisitor>,
    pub(super) visitor_stopped: std::cell::Cell<bool>,
    pub(super) retained_occurrences: std::cell::RefCell<filament_exact::Integer>,
}

pub(super) trait CandidateVisitor {
    fn event(&self, event: super::NativeCandidateEvent<'_>) -> super::NativeCandidateControl;
}
impl<F> CandidateVisitor for std::cell::RefCell<F>
where
    F: for<'event> FnMut(super::NativeCandidateEvent<'event>) -> super::NativeCandidateControl,
{
    fn event(&self, event: super::NativeCandidateEvent<'_>) -> super::NativeCandidateControl {
        (self.borrow_mut())(event)
    }
}

// A control-flow sentinel private to this module: visit_plan_candidates
// recognizes visitor_stopped and never exposes it as a native failure.
const VISITOR_STOP: filament_semantic_value::catalog::CatalogCode =
    filament_semantic_value::catalog::CatalogCode::new("cancelled", "caller-cancelled");

/// Whether an expansion retains the first satisfying result or every result.
#[derive(Clone, Copy, Default)]
pub(super) enum ResultPolicy {
    #[default]
    FirstResult,
    AllResults,
}

/// The presence of each guarded clock place a timed model's post-state
/// holds, which the timed subject derives from it (FR-230 "Presence is
/// derived").
pub(super) type PostPresence<'s> = &'s dyn Fn(&ModelState) -> PlacePresences;

impl Walk<'_, '_> {
    pub(super) fn value_domain(
        &self,
        value_type: &filament_exact::ValueType,
        objects: &[ObjectReference],
    ) -> Option<super::domain::Domain> {
        self.system
            .value_domain_with_clock_selectors(value_type, objects, self.clock_selectors)
    }

    pub(super) fn field_domain(
        &self,
        type_key: &DeclarationKey,
        field: &fsl_semantics::model::state::StateField,
        objects: &[ObjectReference],
    ) -> Option<super::domain::Domain> {
        self.system.field_domain_with_clock_selectors(
            type_key,
            field,
            objects,
            self.clock_selectors,
        )
    }

    pub(super) fn visit(
        &self,
        event: super::NativeCandidateEvent<'_>,
    ) -> Result<super::NativeCandidateControl, ExpansionStop> {
        let Some(visitor) = self.candidate_visitor else {
            return Ok(super::NativeCandidateControl::Continue);
        };
        let beginning = matches!(event, super::NativeCandidateEvent::Begin(_));
        let control = visitor.event(event);
        match control {
            super::NativeCandidateControl::Continue => Ok(control),
            super::NativeCandidateControl::Skip => {
                if beginning {
                    Ok(control)
                } else {
                    Err(stop(BROKEN))
                }
            }
            super::NativeCandidateControl::Stop => {
                self.visitor_stopped.set(true);
                Err(stop(VISITOR_STOP))
            }
        }
    }

    pub(super) fn meter(&self) -> super::EvaluationMeter<'_> {
        let meter = self.system.evaluation_meter();
        match (meter, &self.cancel) {
            (super::EvaluationMeter::Local(meter), Some(cancel)) => {
                super::EvaluationMeter::Local(Box::new((*meter).with_cancel(cancel.clone())))
            }
            (meter, _) => meter,
        }
    }
    /// The references of every object of the state, in row order.
    pub(super) fn references(&self) -> Vec<ObjectReference> {
        self.rows.iter().map(|row| row.reference.clone()).collect()
    }

    /// The `pre` and `post` clauses of `application` at `row` that
    /// `select` admits by their checked identity: every applicable clause
    /// for a model step, the clauses an attempt's `contracts` select for a
    /// protocol attempt (ADR-027 ST-1).
    pub(super) fn selected<'c>(
        &'c self,
        row: &Row,
        application: &Application,
        select: impl Fn(filament_exact::NodeKey) -> bool,
    ) -> Selected<'c> {
        let clauses = &self.system.clauses;
        let pick = |group: &'c [ClauseInfo]| {
            self.applicable(group, row, application.declaring.clone(), &application.name)
                .into_iter()
                .filter(|clause| select(clause.identity.clone()))
                .collect()
        };
        Selected {
            preconditions: pick(&clauses.preconditions),
            postconditions: pick(&clauses.postconditions),
            operation_preconditions: Vec::new(),
            operation_postconditions: Vec::new(),
        }
    }

    /// Source model steps additionally read the checked contracts of their
    /// actual operation. Protocol contract selection stays unit-scoped.
    pub(super) fn model_selected<'c>(
        &'c self,
        row: &Row,
        application: &Application,
        select: impl Fn(filament_exact::NodeKey) -> bool,
    ) -> Selected<'c> {
        use filament_semantic_value::location::OperationClauseKind;

        let mut selected = self.selected(row, application, select);
        let Some(package) = self.system.source_package() else {
            return selected;
        };
        for clause in package.graph().operation_clauses() {
            if clause.operation() != &application.key
                || clause.declaring() != &application.declaring
            {
                continue;
            }
            match clause.kind() {
                OperationClauseKind::Requires => selected.operation_preconditions.push(clause),
                OperationClauseKind::Ensures => selected.operation_postconditions.push(clause),
            }
        }
        selected
            .operation_preconditions
            .sort_by_key(|clause| clause.position());
        selected
            .operation_postconditions
            .sort_by_key(|clause| clause.position());
        selected
    }

    /// The clauses in `group` that apply to `row` as the receiver of the
    /// operation `name` declared by `declaring`: the clause resolves to that
    /// operation and the receiver's type conforms to the clause's context
    /// (FR-120 "Contract clauses").
    pub(super) fn applicable<'c>(
        &self,
        group: &'c [ClauseInfo],
        row: &Row,
        declaring: EffectiveId,
        name: &str,
    ) -> Vec<&'c ClauseInfo> {
        group
            .iter()
            .filter(|clause| {
                clause
                    .operation
                    .as_ref()
                    .is_some_and(|(owner, operation)| owner == &declaring && operation == name)
                    && self
                        .system
                        .types
                        .conforms(row.type_id(), clause.context.clone())
            })
            .collect()
    }
}

/// The `pre` and `post` clauses one application evaluates, each kind in
/// ascending name order.
pub(super) struct Selected<'c> {
    pub(super) preconditions: Vec<&'c ClauseInfo>,
    pub(super) postconditions: Vec<&'c ClauseInfo>,
    pub(super) operation_preconditions: Vec<&'c fsl_semantics::check::CheckedOperationClause>,
    pub(super) operation_postconditions: Vec<&'c fsl_semantics::check::CheckedOperationClause>,
}

/// A contract finding with the keys its position among its siblings follows:
/// ascending transition-identity bytes, then candidate state-key bytes (none
/// first), then clause name (FR-120 "Invariants").
pub(super) struct ContractFinding {
    pub(super) transition: Vec<u8>,
    pub(super) candidate: Option<Vec<u8>>,
    pub(super) clause: String,
    pub(super) finding: ModelFinding,
    /// The receiver, arguments and, for a postcondition, the candidate
    /// post-state and result the finding was made at (FSpec FR-485 reads
    /// them for a counterexample).
    pub(super) case: Box<FindingCase>,
}

/// Where a contract finding was made: the application's receiver and
/// arguments and, for a postcondition, the candidate post-state and result.
pub(super) struct FindingCase {
    pub(super) receiver: ObjectReference,
    pub(super) arguments: Vec<Value>,
    pub(super) post: Option<(fsl_semantics::model::state::ModelState, Option<Value>)>,
}

/// One successor of one application (FR-120 "Applications and transition
/// identities"): its transition identity, the state it reaches, and the
/// receiver, argument vector and result the application chose.
#[derive(Clone)]
pub(crate) struct Applied {
    /// The FR-120 transition identity.
    pub(crate) transition: ModelTransition,
    /// The post-state.
    pub(crate) state: ModelSimState,
    /// The receiver.
    pub(crate) receiver: ObjectReference,
    /// The arguments, in parameter order.
    pub(crate) arguments: Vec<Value>,
    /// The result; `None` for an operation with no result.
    pub(crate) result: Option<Value>,
}

/// The objects of `state`, in ascending (population, key) order.
pub(super) fn rows_of(
    system: &ModelSystem<'_>,
    state: &fsl_semantics::model::state::ModelState,
) -> Result<Vec<Row>, ExpansionStop> {
    let mut rows = Vec::new();
    for (population, objects) in &state.populations {
        for (object_key, object) in objects {
            let reference = system
                .model
                .object_reference(&object.type_identity, object_key)
                .map_err(|_| stop(BROKEN))?;
            rows.push(Row {
                population: population.clone(),
                key: object_key.clone(),
                reference,
            });
        }
    }
    Ok(rows)
}

/// What an expansion of `state` reads: its synthesized observation and its
/// objects (FR-120 "Frame and observation seam"), and the footprints its
/// clauses are held to when the expansion enforces them.
pub(super) fn walk_of<'s, 'a>(
    system: &'s ModelSystem<'a>,
    state: &'s ModelSimState,
    enforcement: Option<&'s Enforcement<'s>>,
) -> Result<Walk<'s, 'a>, ExpansionStop> {
    let observation = system
        .model
        .observation(&state.state, state_document())
        .map_err(|_| stop(BROKEN))?;
    Ok(Walk {
        result_policy: ResultPolicy::FirstResult,
        cancel: None,
        key_policy: None,
        system,
        state,
        observation,
        rows: rows_of(system, &state.state)?,
        enforcement,
        clock_selectors: None,
        post_presence: None,
        candidate_visitor: None,
        visitor_stopped: std::cell::Cell::new(false),
        retained_occurrences: std::cell::RefCell::new(filament_exact::Integer::one()),
    })
}

/// A source-plan walk does not retain the full node's invocation product.
/// It sorts only one invocation's accounted outcomes before advancing.
/// Operation order is supplied by admitted declarations.
pub(super) fn visit_plan_candidates(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    operations: &[(EffectiveId, String)],
    cancel: Option<&filament_exact::Cancel>,
    visitor: &mut dyn for<'event> FnMut(
        super::NativeCandidateEvent<'event>,
    ) -> super::NativeCandidateControl,
) -> Result<super::NativeCandidateVisit, ExpansionStop> {
    let visitor = std::cell::RefCell::new(visitor);
    let mut walk = walk_of(system, state, None)?;
    walk.cancel = cancel.cloned();
    walk.candidate_visitor = Some(&visitor);
    let mut budget = Budget::visiting(cancel, system);
    for (declaring, name) in operations {
        match frames::visit_operation(&walk, declaring, name, &mut budget) {
            Ok(()) => {}
            Err(_) if walk.visitor_stopped.get() => {
                return Ok(super::NativeCandidateVisit::Stopped)
            }
            Err(stop) => return Err(stop),
        }
    }
    Ok(super::NativeCandidateVisit::Complete)
}

/// One model state's expansion.
type ModelExpansion = Expansion<ModelTransition, ModelSimState, ModelFinding>;

/// Expand `state`: its findings and every successor (FR-120).
///
/// No post-state presences are supplied here: a postcondition's
/// `present(self.v)` read on this path fails closed with
/// `place-presence-unrecorded`. Dense timed models route only to EN-6,
/// whose timed subject expands through [`expand_enabled`] with them, so
/// nothing reaches that read here today.
pub(super) fn expand_state(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    enforcement: Option<&Enforcement<'_>>,
) -> Result<Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop> {
    let cancel = system
        .request_cancel
        .lock()
        .expect("request cancellation lock")
        .clone();
    expand(
        system,
        state,
        enforcement,
        None,
        ExpansionSelection::default(),
        cancel.as_ref(),
        None,
    )
}

/// Expand the existing model relation while retaining every satisfying result.
pub(super) fn expand_relation_state(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    cancel: &filament_exact::Cancel,
) -> Result<ModelExpansion, ExpansionStop> {
    expand(
        system,
        state,
        None,
        None,
        ExpansionSelection {
            result_policy: ResultPolicy::AllResults,
            ..ExpansionSelection::default()
        },
        Some(cancel),
        None,
    )
}

/// Expand a state while testing one selected operation clause independently
/// of the relation that generates cases.
pub(super) fn expand_state_omitting_clause(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    clause: &filament_exact::NodeKey,
) -> Result<Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop> {
    expand(
        system,
        state,
        None,
        None,
        ExpansionSelection {
            omitted_clause: Some(clause),
            ..ExpansionSelection::default()
        },
        None,
        None,
    )
}

/// [`expand_state`] with no footprint enforcement, also returning every
/// argument vector whose effective precondition holds, each with the
/// positions of its successors in the expansion's list (FR-187).
/// `post_presence`, when given, is the presence of each guarded clock place
/// at a candidate post-state, which a postcondition reads.
pub(super) fn expand_enabled<'s>(
    system: &'s ModelSystem<'_>,
    state: &'s ModelSimState,
    post_presence: Option<PostPresence<'s>>,
) -> Result<(ModelExpansion, Vec<frames::EnabledVector>), ExpansionStop> {
    expand_enabled_with_clock_selectors(system, state, post_presence, None)
}

pub(super) fn expand_enabled_with_clock_selectors<'s>(
    system: &'s ModelSystem<'_>,
    state: &'s ModelSimState,
    post_presence: Option<PostPresence<'s>>,
    clock_selectors: Option<&'s super::domain::ClockSelectorMaxima>,
) -> Result<(ModelExpansion, Vec<frames::EnabledVector>), ExpansionStop> {
    let mut enabled = Vec::new();
    let expansion = expand(
        system,
        state,
        None,
        Some(&mut enabled),
        ExpansionSelection {
            clock_selectors,
            post_presence,
            ..ExpansionSelection::default()
        },
        None,
        None,
    )?;
    Ok((expansion, enabled))
}

pub(super) fn expand_enabled_with_cancel<'s>(
    system: &'s ModelSystem<'_>,
    state: &'s ModelSimState,
    cancel: &filament_exact::Cancel,
) -> Result<(ModelExpansion, Vec<frames::EnabledVector>), ExpansionStop> {
    let mut enabled = Vec::new();
    let expansion = expand(
        system,
        state,
        None,
        Some(&mut enabled),
        ExpansionSelection::default(),
        Some(cancel),
        None,
    )?;
    Ok((expansion, enabled))
}

pub(super) fn expand_enabled_with_key_policy<'s>(
    system: &'s ModelSystem<'_>,
    state: &'s ModelSimState,
    cancel: &filament_exact::Cancel,
    key_policy: &'s dyn super::key_budget::KeyPolicy,
) -> Result<(ModelExpansion, Vec<frames::EnabledVector>), ExpansionStop> {
    let mut enabled = Vec::new();
    let expansion = expand(
        system,
        state,
        None,
        Some(&mut enabled),
        ExpansionSelection::default(),
        Some(cancel),
        Some(key_policy),
    )?;
    Ok((expansion, enabled))
}

#[derive(Default)]
struct ExpansionSelection<'s> {
    result_policy: ResultPolicy,
    clock_selectors: Option<&'s super::domain::ClockSelectorMaxima>,
    post_presence: Option<PostPresence<'s>>,
    omitted_clause: Option<&'s filament_exact::NodeKey>,
}

fn expand<'s>(
    system: &'s ModelSystem<'_>,
    state: &'s ModelSimState,
    enforcement: Option<&'s Enforcement<'s>>,
    mut enabled: Option<&mut Vec<frames::EnabledVector>>,
    selection: ExpansionSelection<'s>,
    cancel: Option<&filament_exact::Cancel>,
    key_policy: Option<&'s dyn super::key_budget::KeyPolicy>,
) -> Result<Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop> {
    // FR-120: every expansion entry point runs the bound check first, so an
    // expansion never meets a granted field with no finite domain.
    debug_assert!(
        match selection.clock_selectors {
            Some(selectors) => super::check_model_bounds_with_clock_selectors(
                system,
                crate::simulation::BOUND_POSITIONS,
                Some(selectors)
            )
            .is_ok(),
            None => system.bounded(),
        },
        "FR-120: a system the requires-bound check refuses is never expanded"
    );
    let mut walk = walk_of(system, state, enforcement)?;
    walk.result_policy = selection.result_policy;
    walk.cancel = cancel.cloned();
    walk.key_policy = key_policy;
    walk.post_presence = selection.post_presence;
    walk.clock_selectors = selection.clock_selectors;
    let mut budget = system.budget();
    budget.cancel = cancel
        .cloned()
        .or_else(|| system.probability_cancel.clone());
    if let (Some(meter), Some(cancel)) = (&mut budget.work_meter, cancel) {
        *meter = meter.clone().with_cancel(cancel.clone());
    }

    let mut findings = contracts::invariants(&walk)?;
    let mut contract = Vec::new();
    let mut successors = Vec::new();
    for row in &walk.rows {
        let Some(applications) = system.applications.get(&row.type_id()) else {
            continue;
        };
        for application in applications {
            let select = |identity| selection.omitted_clause != Some(&identity);
            let clauses = if selection.clock_selectors.is_none()
                && selection.post_presence.is_none()
                && system.input().timed_models.is_empty()
            {
                walk.model_selected(row, application, select)
            } else {
                walk.selected(row, application, select)
            };
            frames::apply(
                &walk,
                row,
                application,
                &clauses,
                &mut budget,
                frames::Outputs {
                    recommendations: None,
                    contract: &mut contract,
                    successors: &mut successors,
                    enabled: enabled.as_deref_mut(),
                    enabled_transitions: None,
                    selected_transitions: None,
                    charged: None,
                },
            )?;
        }
    }
    match key_policy {
        Some(policy) => {
            super::key_budget::sort(policy, &mut contract, |left, right| {
                let transition = policy.compare(&left.transition, &right.transition)?;
                if !transition.is_eq() {
                    return Ok(transition);
                }
                let candidate = match (&left.candidate, &right.candidate) {
                    (Some(left), Some(right)) => policy.compare(left, right)?,
                    (None, None) => std::cmp::Ordering::Equal,
                    (None, Some(_)) => std::cmp::Ordering::Less,
                    (Some(_), None) => std::cmp::Ordering::Greater,
                };
                if !candidate.is_eq() {
                    return Ok(candidate);
                }
                policy.compare(left.clause.as_bytes(), right.clause.as_bytes())
            })?;
            findings.extend(contract.into_iter().map(|entry| entry.finding));
        }
        None => findings.extend(sorted_contract_findings(contract)),
    }
    Ok(Expansion {
        successors: successors
            .into_iter()
            .map(|applied| (applied.transition, applied.state))
            .collect(),
        findings,
    })
}

/// Contract findings in the order their siblings follow (FR-120
/// "Invariants"): ascending transition-identity bytes, then candidate
/// state-key bytes (none first), then clause name.
pub(super) fn sorted_contract_findings(mut contract: Vec<ContractFinding>) -> Vec<ModelFinding> {
    contract.sort_by(|left: &ContractFinding, right: &ContractFinding| {
        left.transition
            .cmp(&right.transition)
            .then_with(|| left.candidate.cmp(&right.candidate))
            .then_with(|| left.clause.cmp(&right.clause))
    });
    contract.into_iter().map(|entry| entry.finding).collect()
}

/// Every transition identity of the subject at `state`, enabled or not, in
/// canonical transition order (FR-156 "Candidates", FR-155 "State-free"):
/// each application of the operations of every object the state holds and of
/// every object a universe key could name that it does not hold yet, to every
/// argument vector of its parameter domains. The objects not held are taken at
/// each concrete type their population admits, since a triple carries the
/// most-specific type.
pub(super) fn identities_of(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
) -> Result<Vec<ModelTransition>, ExpansionStop> {
    let mut walk = walk_of(system, state, None)?;
    walk.rows.extend(absent_rows(system, state));
    let mut budget = system.budget();
    budget.cancel = system
        .request_cancel
        .lock()
        .expect("request cancellation lock")
        .clone()
        .or_else(|| system.probability_cancel.clone());
    let mut found = Vec::new();
    for row in &walk.rows {
        let Some(applications) = system.applications.get(&row.type_id()) else {
            continue;
        };
        for application in applications {
            frames::identities(&walk, row, application, &mut budget, &mut found)?;
        }
    }
    found.sort();
    Ok(found)
}

/// One row for each universe key a population does not hold at `state`, at
/// each non-abstract object type that conforms to a member type of the
/// population.
pub(super) fn absent_rows(system: &ModelSystem<'_>, state: &ModelSimState) -> Vec<Row> {
    let mut rows = Vec::new();
    for population in &system.populations {
        let Some(keys) = system.universes.get(&population.key) else {
            continue;
        };
        let held = state.state.populations.get(&population.key);
        for key in keys
            .iter()
            .filter(|key| !held.is_some_and(|held| held.contains_key(*key)))
        {
            for declaration in system.types.object_types() {
                let Some(type_key) = system.model.type_key(declaration.key()) else {
                    continue;
                };
                let admitted = population.member_types.iter().any(|member| {
                    system
                        .model
                        .type_identity(member)
                        .is_some_and(|member| system.types.conforms(declaration.key(), member))
                });
                if system.model.is_abstract(type_key) || !admitted {
                    continue;
                }
                if let Ok(reference) = system.model.object_reference(type_key, key) {
                    rows.push(Row {
                        population: population.key.clone(),
                        key: key.clone(),
                        reference,
                    });
                }
            }
        }
    }
    rows
}
