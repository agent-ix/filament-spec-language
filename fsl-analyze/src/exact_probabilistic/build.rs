// SPDX-License-Identifier: AGPL-3.0-or-later
//! The breadth-first construction of the probabilistic product (FR-196
//! "Behavior"): the product, its state numbering and its edge order are
//! functions of the subject, the claim and the limits.

use fsl_eval::simulation::model::{
    ModelKeyBudget, ModelKeyError, ModelKeyExpansionError, UndecidedSuccessor,
};
use fsl_foundation::IdentityLimits;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_exact::{
    Cancel, Identifier, Integer, ObjectReference, Rational, Value, ValueType, WorkScope,
};
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::automaton::{AutomatonError, Monitor, PropertyAutomaton};
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::model::{
    same_rewards, Action, ModelSimState, ModelSimulationRequest, ModelSystem, NotMarkov,
    PredicateReading, ScheduledIdentity, Unsteppable,
};
use fsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
use fsl_eval::simulation::StateKey;
use fsl_eval::simulation::TransitionSystem;
use fsl_eval::temporal::UndefinedCause;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_replay::ModelCheckLimit;
use fsl_semantics::check::{CheckedPredicate, Workload};
use ix_trace_rs::implements;

use super::plan::{Plan, PlanFault, Reads};
use super::{
    Accumulator, FairnessOperationIdentity, PathStep, Pending, ProbBuild, ProbEdge, ProbKey,
    ProbProduct, ProbStateId, ProductAction, ProductKind, ProductRefusal, ProductRequest,
    ProductStop, UndefinedPath, UndefinedRead,
};
use super::{FormulaState, MonitorState};

/// Build the product of `request`'s claim over its subject. A product whose
/// successor `max_states` denied is [`ProbBuild::Partial`], the explored
/// prefix partial evidence reads (FR-196, FSpec FR-411 "Evidence on a partial
/// product").
///
/// # Errors
///
/// [`ProductRefusal`] when the subject does not admit, a state does not
/// expand or step, the event has no monitor, `max_states` denies an initial
/// state, or another budget or the cancellation ends the build (FR-196,
/// FR-203).
#[implements(
    "FR-196-AC-1",
    "FR-196-AC-2",
    "FR-196-AC-3",
    "FR-196-AC-4",
    "FR-196-AC-5",
    "FR-196-AC-7",
    "FR-196-AC-8",
    "FR-196-AC-9",
    "FR-196-AC-11",
    "FR-196-AC-12",
    "FR-196-AC-13"
)]
pub fn build_product(
    request: ProductRequest<'_>,
    cancel: &Cancel,
) -> Result<ProbBuild, ProductRefusal> {
    build_retaining_monitor(request, cancel).map(|(build, _)| build)
}

/// Build a product under a caller-owned item work scope.
#[cfg(feature = "test-support")]
#[implements("FR-203-AC-29")]
pub fn build_product_with_work_scope(
    request: ProductRequest<'_>,
    work: WorkScope,
    cancel: &Cancel,
) -> Result<ProbBuild, ProductRefusal> {
    build_retaining_monitor_in_scope(request, work, cancel).map(|(build, _)| build)
}

/// Retain the reached semantic owner for certificate key projection. The
/// monitor cache moves once after exploration, never once per product node.
pub(crate) fn build_retaining_monitor(
    request: ProductRequest<'_>,
    cancel: &Cancel,
) -> Result<(ProbBuild, Option<Monitor<PropertyAutomaton>>), ProductRefusal> {
    let work = WorkScope::new(request.subject.simulation.meter);
    build_retaining_monitor_in_scope(request, work, cancel)
}

/// [`build_retaining_monitor`] under the item's `work` allowance, so that a
/// later reading of the same item spends only what the build left (FR-203).
#[implements("FR-203-AC-29")]
pub(crate) fn build_retaining_monitor_in_scope(
    request: ProductRequest<'_>,
    work: WorkScope,
    cancel: &Cancel,
) -> Result<(ProbBuild, Option<Monitor<PropertyAutomaton>>), ProductRefusal> {
    match build_configured(request, work, cancel, None) {
        Ok(result) => Ok(result),
        Err(BudgetedProductError::Product(error)) => Err(error),
        Err(BudgetedProductError::Key(_) | BudgetedProductError::Undefined(_)) => {
            Err(fault("legacy-builder-does-not-use-key-admission"))
        }
    }
}

/// One qualified build's original product failure or identity-owner stop.
#[derive(Debug, thiserror::Error)]
pub enum BudgetedProductError {
    /// Existing product admission/evaluation semantics.
    #[error(transparent)]
    Product(#[from] ProductRefusal),
    /// Raw identity failure, with a denied caller cause held by its budget.
    #[error(transparent)]
    Key(#[from] ModelKeyError),
    /// Proof-only admission cannot create a native undefined refutation path.
    #[error("the checked predicate is undefined: {0:?}")]
    Undefined(UndefinedCause),
}

struct KeyPolicy<'a> {
    identity: IdentityLimits,
    budget: RefCell<&'a mut dyn ModelKeyBudget>,
}

/// Retain the semantic monitor while every key uses the item's work allowance.
#[implements("FR-203-AC-29")]
pub(crate) fn build_retaining_monitor_with_budget_in_scope(
    request: ProductRequest<'_>,
    identity: IdentityLimits,
    budget: &mut dyn ModelKeyBudget,
    work: WorkScope,
    cancel: &Cancel,
) -> Result<(ProbBuild, Option<Monitor<PropertyAutomaton>>), BudgetedProductError> {
    let policy = KeyPolicy {
        identity,
        budget: RefCell::new(budget),
    };
    build_configured(request, work, cancel, Some(&policy))
}

/// Exercise the production budgeted builder without changing public requests.
#[cfg(feature = "test-support")]
pub fn build_product_with_key_budget(
    request: ProductRequest<'_>,
    identity: IdentityLimits,
    budget: &mut dyn ModelKeyBudget,
    cancel: &Cancel,
) -> Result<ProbBuild, BudgetedProductError> {
    let work = WorkScope::new(request.subject.simulation.meter);
    build_retaining_monitor_with_budget_in_scope(request, identity, budget, work, cancel)
        .map(|(build, _)| build)
}

/// Admit `simulation` under the item's `work` allowance.
///
/// # Errors
///
/// [`ProductRefusal`] when the cancellation is tripped, the subject does not
/// admit, or the allowance cannot pay a reading admission makes.
pub(crate) fn admit_in_scope<'a>(
    simulation: ModelSimulationRequest<'a>,
    work: WorkScope,
    cancel: &Cancel,
) -> Result<ModelSystem<'a>, ProductRefusal> {
    ModelSystem::new_with_work_scope(simulation, work, cancel).map_err(|failure| {
        if let Some(cause) = cancel.tripped() {
            ProductRefusal::Stopped(ProductStop::Cancelled(cause))
        } else {
            match failure {
                fsl_eval::simulation::model::ProbabilityAdmissionFailure::Admission(failure) => {
                    ProductRefusal::Admission(failure)
                }
                fsl_eval::simulation::model::ProbabilityAdmissionFailure::Evaluation(
                    incomplete,
                ) => ProductRefusal::Expansion(fsl_eval::simulation::ExpansionStop {
                    cause: filament_semantic_value::catalog::CatalogCode::new(
                        "resource_exhausted",
                        "insufficient-next-charge",
                    ),
                    limit: Some(fsl_eval::simulation::ExpansionLimit::EvaluationMeter(
                        incomplete.limit_kind,
                    )),
                }),
            }
        }
    })
}

#[implements("FR-125-AC-9")]
fn build_configured(
    request: ProductRequest<'_>,
    work: WorkScope,
    cancel: &Cancel,
    key_policy: Option<&KeyPolicy<'_>>,
) -> Result<(ProbBuild, Option<Monitor<PropertyAutomaton>>), BudgetedProductError> {
    let ProductRequest {
        subject,
        claim,
        limits,
    } = request;
    let limits_all = limits;
    let limits = limits_all.model;
    let probabilistic = subject.simulation.input.probabilistic;
    let system = admit_in_scope(subject.simulation, work, cancel)?;
    match TimedSystem::new(&system, &claim.subject.alias) {
        Ok(timed) => {
            if key_policy.is_some() {
                return Err(
                    ModelKeyError::Unsupported("untimed budgeted probabilistic product").into(),
                );
            }
            return super::digital::build(&system, timed, claim, &limits_all, cancel)
                .map(|build| (build, None))
                .map_err(BudgetedProductError::Product);
        }
        Err(TimedSubjectRefusal::NoTimedModel { .. }) => {}
        Err(TimedSubjectRefusal::NotSimulated(refusal)) => {
            return Err(ProductRefusal::NotSimulated(refusal).into());
        }
        Err(TimedSubjectRefusal::ParameterizedGuard { .. }) => {
            return Err(fault("timed-guard-reads-no-operation-parameter").into());
        }
        Err(TimedSubjectRefusal::NestedUnboundedCollection { .. }) => {
            return Err(fault("timed-clocks-nest-no-unbounded-collections").into());
        }
        Err(TimedSubjectRefusal::Unbounded { .. }) => {
            return Err(fault("timed-clock-owners-have-universes").into());
        }
    }
    let (kind, workload) = match &claim.scheduler {
        fsl_semantics::check::ProbScheduler::Workload(name) => {
            let workload = probabilistic
                .workload(name)
                .ok_or_else(|| fault("claim-workload-is-declared"))?;
            (ProductKind::Dtmc, Some(workload))
        }
        fsl_semantics::check::ProbScheduler::Every => (ProductKind::Mdp, None),
    };
    let (plan, reads) = Plan::of(claim, limits.max_automaton_states)
        .map_err(|fault| plan_refusal(fault, claim.occurrence(), claim.timed_form_member_path()))?;
    let bindings = match &claim.over {
        None => vec![(None, None)],
        Some(over) => {
            let ValueType::Reference(object) = &over.value_type else {
                return Err(fault("over-is-an-object-type").into());
            };
            system
                .instances(&over.value_type)
                .map_err(ProductRefusal::NotSimulated)?
                .into_iter()
                .map(|instance| {
                    let reference = system
                        .written_reference(object.clone(), &instance)
                        .map_err(ProductRefusal::Internal)?;
                    Ok((Some(instance), Some(reference)))
                })
                .collect::<Result<Vec<_>, ProductRefusal>>()?
        }
    };
    let builder = Builder {
        system: &system,
        key_policy,
        target_mode: if key_policy.is_some()
            && matches!(
                claim.form,
                fsl_semantics::check::ProbForm::ExpectedReward { .. }
            ) {
            CandidateTargetMode::FirstRewardTarget
        } else if key_policy.is_some()
            && matches!(claim.scheduler, fsl_semantics::check::ProbScheduler::Every)
            && matches!(
                claim.form,
                fsl_semantics::check::ProbForm::Reach {
                    target: fsl_semantics::check::ReachTarget::Eventually(_),
                    ..
                }
            )
        {
            CandidateTargetMode::FirstMdpTarget
        } else {
            CandidateTargetMode::Continue
        },
        workload,
        kind,
        plan,
        event_horizon: match &claim.form {
            fsl_semantics::check::ProbForm::Probability {
                event: fsl_semantics::check::Event::Formula { horizon, .. },
                ..
            } => Some(*horizon),
            _ => None,
        },
        measure_meter: system.fresh_meter().with_cancel(cancel.clone()),
        reads,
        bindings,
        max_states: limits.limits.max_states,
        max_transitions: limits.limits.max_transitions,
        cancel,
        nodes: Vec::new(),
        index: BTreeMap::new(),
        pending_keys: BTreeSet::new(),
        readings: BTreeMap::new(),
        edges: Vec::new(),
        queue: VecDeque::new(),
        fairness_operations: claim
            .fairness
            .iter()
            .filter_map(|constraint| {
                system
                    .operation_text(
                        constraint.operation().declaring_type(),
                        constraint.operation().name(),
                    )
                    .map(|text| FairnessOperationIdentity {
                        operation: constraint.operation().clone(),
                        text: text.to_owned(),
                    })
            })
            .collect(),
        fairness: claim.fairness.clone(),
        partial: false,
        omitted: Vec::new(),
    };
    builder.run()
}

fn fault(invariant: &'static str) -> ProductRefusal {
    ProductRefusal::Internal(InternalFault::new("exact-probabilistic", invariant))
}

fn plan_refusal(
    fault_: PlanFault,
    claim: Option<&OccurrenceKey>,
    member: Option<fsl_semantics::check::ClaimMemberPath>,
) -> ProductRefusal {
    match fault_ {
        PlanFault::Automaton(error) => automaton_refusal(error),
        PlanFault::TimedFormShape { span } => ProductRefusal::TimedFormShape {
            span,
            claim: claim.cloned(),
            member,
        },
        PlanFault::Measure(fsl_eval::probabilistic::MeasureError::Undefined(cause)) => {
            ProductRefusal::PredicateRefused(
                fsl_eval::simulation::model::ClauseOutcome::KernelUndefined(*cause),
            )
        }
        PlanFault::Measure(fsl_eval::probabilistic::MeasureError::Accounting(error)) => {
            ProductRefusal::PredicateRefused(
                fsl_eval::simulation::model::ClauseOutcome::Incomplete(*error),
            )
        }
        PlanFault::Broken(invariant)
        | PlanFault::Measure(fsl_eval::probabilistic::MeasureError::Broken(invariant)) => {
            fault(invariant)
        }
    }
}

/// An automaton that reached its state limit is a budget; any other refusal
/// is the event's.
pub(crate) fn automaton_refusal(error: AutomatonError) -> ProductRefusal {
    match error {
        AutomatonError::StateLimit { limit } => ProductRefusal::Stopped(ProductStop::Limit {
            limit: ModelCheckLimit::ExactProbMaxAutomatonStates,
            value: limit,
        }),
        other => ProductRefusal::Translation(other),
    }
}

#[cfg(test)]
mod tests {
    use super::{lower_completion, plan_refusal, OmittedEdge, PlanFault, ProductRefusal};
    use crate::exact_probabilistic::{
        ProbEdge, ProbKey, ProbProduct, ProbStateId, ProductAction, ProductKind,
    };
    use filament_exact::{Integer, Rational};

    /// FR-204-AC-4 (TC-639): the exact planner keeps the timed form refusal's source span.
    #[ix_trace_rs::trace("TC-639", "FR-204-AC-4")]
    #[test]
    fn timed_form_shape_preserves_its_span() {
        let span = fsl_foundation::Span { start: 12, end: 34 };
        let ProductRefusal::TimedFormShape { span: actual, .. } =
            plan_refusal(PlanFault::TimedFormShape { span }, None, None)
        else {
            panic!("a timed form shape refusal");
        };

        assert_eq!(actual, span);

        // The plan path carries the selected claim's identity.
        let key = fsl_foundation::source::provenance::OccurrenceKey::generated(
            filament_exact::NodeKey::new(
                "p",
                vec![filament_exact::PathSegment::Text("k".into())],
                0,
            )
            .expect("a valid key"),
        );
        let ProductRefusal::TimedFormShape { claim, .. } =
            plan_refusal(PlanFault::TimedFormShape { span }, Some(&key), None)
        else {
            panic!("a timed form shape refusal");
        };
        assert_eq!(claim, Some(key));
    }

    #[test]
    fn partial_completion_preserves_omitted_successor_mass() {
        let key = |model| ProbKey {
            binding: None,
            model: vec![model],
            monitor: None,
            accumulator: None,
            pending: None,
            certificate_state: None,
        };
        let half = Rational::new(Integer::one(), Integer::from(2_i64)).unwrap();
        let edge = |from, probability| ProbEdge {
            from: ProbStateId(from),
            choice: ProductAction::TerminalStutter,
            action: None,
            draw: None,
            to: ProbStateId(1),
            probability,
            elapsed: Rational::from_integer(Integer::zero()),
            rewards: Vec::new(),
        };
        let product = ProbProduct {
            states: vec![key(0), key(1)],
            kind: ProductKind::Dtmc,
            edges: vec![
                edge(0, half.clone()),
                edge(1, Rational::from_integer(Integer::one())),
            ],
            initial: vec![(0, None, ProbStateId(0))],
            decided: Vec::new(),
            labels: vec![vec![false], vec![true]],
            fairness_operations: Vec::new(),
            fairness: Vec::new(),
        };
        let omitted = OmittedEdge {
            kept_before: 1,
            choice: ProductAction::TerminalStutter,
            action: None,
            draw: None,
            probability: half,
            rewards: Vec::new(),
        };
        let completed = lower_completion(&product, vec![(0, omitted)]);
        // `outgoing` reads the row, so it holds only when the omitted edge
        // stands before state 1's edge.
        let source_mass = completed
            .outgoing(ProbStateId(0))
            .iter()
            .fold(Rational::from_integer(Integer::zero()), |mass, edge| {
                mass.add(&edge.probability)
            });
        assert_eq!(source_mass, Rational::from_integer(Integer::one()));
        assert_eq!(completed.states.len(), 3);
        assert_eq!(completed.labels[2], vec![false]);
        assert_eq!(completed.edges[1].to, ProbStateId(2));
        assert_eq!(completed.edges[2].from, ProbStateId(1));
        assert_eq!(completed.edges.len(), 3);
    }
}

/// Why the exploration ended before it was exhausted.
enum Halt {
    Refusal(ProductRefusal),
    Key(ModelKeyError),
    UndefinedReading(UndefinedCause),
    Undefined(Box<UndefinedPath>),
}

impl From<ProductRefusal> for Halt {
    fn from(refusal: ProductRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

impl From<PlanFault> for Halt {
    fn from(fault: PlanFault) -> Self {
        Self::Refusal(plan_refusal(fault, None, None))
    }
}

type Step<T> = Result<T, Halt>;

/// What a draw with several post-states awaits: the post-states in canonical
/// order, and the step's rewards.
struct Resolution {
    successors: Vec<ModelSimState>,
    rewards: Vec<(Identifier, Value)>,
}

/// One discovered product state.
struct Node {
    key: ProbKey,
    model: ModelSimState,
    /// The truth of the claim's predicates at the model state.
    truths: Vec<bool>,
    /// The state it was first reached from, and the model step that reached
    /// it: none for a resolution, which is no model step.
    parent: Option<(usize, Option<TraceStep>)>,
    /// The subject initial state that first discovered this node.
    initial: u32,
    /// Set at an intermediate state.
    resolution: Option<Resolution>,
}

/// A target of a model step, before it is a product state.
struct Target {
    key: ProbKey,
    /// What numbering the key reads. None when `max_states` denied the key a
    /// slot, so that no representation of it is constructed (FR-203-AC-14).
    state: Option<TargetState>,
}

impl Target {
    /// Count the planned edge to this target, if the product keeps it.
    fn observe_edge(&self) {
        if self.state.is_some() {
            super::observe_edge_construction();
        }
    }
}

/// An admitted target's model state and control reading.
struct TargetState {
    model: ModelSimState,
    truths: Vec<bool>,
    /// Set for an intermediate state.
    resolution: Option<Resolution>,
}

/// Only the control and key needed to decide target admission.
struct Arrival {
    key: ProbKey,
    truths: Vec<bool>,
}

/// An edge of the state being expanded, before its target is numbered.
struct Planned {
    choice: ProductAction,
    action: Option<ScheduledIdentity>,
    draw: Option<Vec<Value>>,
    target: Target,
    probability: Rational,
    rewards: Vec<(Identifier, Value)>,
    /// The model step the edge takes.
    step: Option<TraceStep>,
}

/// The solver edge for a successor omitted from partial exploration.
struct OmittedEdge {
    /// How many edges the product held when this one was omitted: its place
    /// in exploration order.
    kept_before: usize,
    choice: ProductAction,
    action: Option<ScheduledIdentity>,
    draw: Option<Vec<Value>>,
    probability: Rational,
    rewards: Vec<(Identifier, Value)>,
}

/// The step retained during exploration. Its complete product target is
/// filled from the discovered path when an undefined read is reported.
#[derive(Clone)]
struct TraceStep {
    action: Option<ScheduledIdentity>,
    draw: Option<Vec<Value>>,
    probability: Rational,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CandidateTargetMode {
    Continue,
    FirstRewardTarget,
    FirstMdpTarget,
}

struct Builder<'r, 'c, 'p, 'k> {
    system: &'r ModelSystem<'r>,
    key_policy: Option<&'p KeyPolicy<'k>>,
    // Qualified reward and eventual-MDP candidates stop at their first target.
    // Independent replay still admits the complete original source model.
    target_mode: CandidateTargetMode,
    workload: Option<&'r Workload>,
    kind: ProductKind,
    plan: Plan<'c>,
    /// The horizon of a bounded event: the last position it reads.
    event_horizon: Option<u64>,
    measure_meter: filament_exact::Meter,
    reads: Reads<'c>,
    /// Each `over` binding, with the reference its predicates are read for.
    bindings: Vec<(Option<ModelInstance>, Option<ObjectReference>)>,
    max_states: usize,
    max_transitions: usize,
    cancel: &'r Cancel,
    nodes: Vec<Node>,
    index: BTreeMap<ProbKey, usize>,
    /// Distinct new keys already reserved by this expansion's planned edges.
    pending_keys: BTreeSet<ProbKey>,
    /// The predicates' truths at a model state, by binding and state key.
    readings: BTreeMap<usize, BTreeMap<Vec<u8>, Vec<bool>>>,
    edges: Vec<ProbEdge>,
    queue: VecDeque<usize>,
    fairness_operations: Vec<FairnessOperationIdentity>,
    fairness: Vec<filament_semantic_value::fairness::FairnessConstraint>,
    partial: bool,
    omitted: Vec<(usize, OmittedEdge)>,
}

/// One model state the sweep beyond decided control reached.
struct Swept {
    model: ModelSimState,
    /// The product key, built only when the sweep can end the build with a
    /// path (not under key admission).
    key: Option<ProbKey>,
    parent: Option<usize>,
    step: Option<TraceStep>,
    /// The model steps from the decided node.
    depth: u64,
}

/// Why reading a swept state's steps stopped.
enum SweptStop {
    /// Its expansion found a contract conjunction with no value, at the state
    /// with this key.
    Undefined(fsl_eval::simulation::model::UndefinedContract, StateKey),
    Halt(Halt),
}

/// A predicate that has no value at a model state.
struct Hit {
    read: UndefinedRead,
    cause: UndefinedCause,
    state_key: StateKey,
    model: Vec<u8>,
}

/// Complete a solver-only copy with an absorbing least-value state. Each
/// omitted edge keeps its original probability and action, so rows are never
/// renormalized and each omitted successor contributes exactly its original
/// mass at the least continuation value.
#[implements("FR-1042-AC-25")]
fn lower_completion(product: &ProbProduct, omitted: Vec<(usize, OmittedEdge)>) -> ProbProduct {
    let mut completed = product.clone();
    if omitted.is_empty() {
        return completed;
    }
    let sink = completed.states.len();
    let mut key = completed.states[0].clone();
    key.model.extend_from_slice(b"partial-lower-sink");
    key.pending = None;
    key.certificate_state = None;
    key.monitor = match key.monitor {
        Some(MonitorState::Formula(_)) => Some(MonitorState::Formula(FormulaState::Decided(
            super::MonitorVerdict::Reject,
        ))),
        Some(MonitorState::Measure { .. }) => Some(MonitorState::Measure {
            remaining: 0,
            phase: fsl_eval::probabilistic::MeasurePhase::Complete,
        }),
        monitor => monitor,
    };
    completed.states.push(key);
    if !completed.labels.is_empty() {
        let width = completed.labels.first().map_or(0, Vec::len);
        completed.labels.push(vec![false; width]);
    }
    completed
        .decided
        .push((ProbStateId(sink), super::MonitorVerdict::Reject));
    let to_sink = |(from, edge): (usize, OmittedEdge)| ProbEdge {
        from: ProbStateId(from),
        choice: edge.choice,
        action: edge.action,
        draw: edge.draw,
        to: ProbStateId(sink),
        probability: edge.probability,
        elapsed: Rational::from_integer(Integer::from(0_i64)),
        rewards: edge.rewards,
    };
    // Each omitted edge returns to its place in exploration order, so the
    // edges stay ordered by source and one choice's edges stay adjacent.
    let kept = std::mem::take(&mut completed.edges);
    let mut omitted = omitted.into_iter().peekable();
    for (position, edge) in kept.into_iter().enumerate() {
        while let Some(missing) = omitted.next_if(|(_, missing)| missing.kept_before <= position) {
            completed.edges.push(to_sink(missing));
        }
        completed.edges.push(edge);
    }
    completed.edges.extend(omitted.map(to_sink));
    completed
}

impl Builder<'_, '_, '_, '_> {
    fn run(
        mut self,
    ) -> Result<(ProbBuild, Option<Monitor<PropertyAutomaton>>), BudgetedProductError> {
        match self.explore() {
            Ok(initial) => {
                let partial = self.partial;
                let omitted = std::mem::take(&mut self.omitted);
                let (product, monitor) = self.finish(initial);
                let build = if partial {
                    let lower_product = lower_completion(&product, omitted);
                    ProbBuild::Partial {
                        product,
                        lower_product,
                    }
                } else {
                    ProbBuild::Built(product)
                };
                Ok((build, monitor))
            }
            Err(Halt::Refusal(refusal)) => Err(BudgetedProductError::Product(refusal)),
            Err(Halt::Key(error)) => Err(BudgetedProductError::Key(error)),
            Err(Halt::UndefinedReading(cause)) => Err(BudgetedProductError::Undefined(cause)),
            Err(Halt::Undefined(path)) => {
                let monitor = match self.plan {
                    Plan::Formula(monitor) => Some(*monitor),
                    _ => None,
                };
                Ok((ProbBuild::Undefined(*path), monitor))
            }
        }
    }

    /// Number the initial states, then every state they reach, breadth
    /// first. Returns the initial states' handles.
    fn explore(&mut self) -> Step<Vec<(u32, Option<ModelInstance>, ProbStateId)>> {
        self.poll()?;
        let mut starts = Vec::new();
        for (position, state) in self.system.initial().into_iter().enumerate() {
            let at = u32::try_from(position).map_err(|_| fault("initial-states-count"))?;
            for binding in 0..self.bindings.len() {
                let truths = match self.truths(&state, binding)? {
                    Ok(truths) => truths,
                    Err(hit) => {
                        let (monitor, accumulator) =
                            self.plan.initial_control(&mut self.measure_meter)?;
                        let key = ProbKey {
                            binding: self.bindings[binding].0.clone(),
                            model: hit.model.clone(),
                            monitor,
                            accumulator,
                            pending: None,
                            certificate_state: None,
                        };
                        return Err(self.undefined(None, None, None, at, key, hit));
                    }
                };
                let (monitor, accumulator) = self.plan.start(&truths, &mut self.measure_meter)?;
                let key = self.key_of(&state, binding, monitor, accumulator, None)?;
                self.admit_state(&key)?;
                super::observe_target_construction();
                let admitted = TargetState {
                    model: state.clone(),
                    truths,
                    resolution: None,
                };
                let id = self.intern(key, admitted, None, None, at)?;
                starts.push((at, self.bindings[binding].0.clone(), ProbStateId(id)));
            }
        }
        while let Some(at) = self.queue.pop_front() {
            self.poll()?;
            self.expand(at)?;
        }
        self.sweep()?;
        Ok(starts)
    }

    fn finish(
        self,
        initial: Vec<(u32, Option<ModelInstance>, ProbStateId)>,
    ) -> (ProbProduct, Option<Monitor<PropertyAutomaton>>) {
        let decided = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| match node.key.monitor {
                Some(MonitorState::Formula(FormulaState::Decided(verdict))) => {
                    Some((ProbStateId(index), verdict))
                }
                _ if self.target_mode != CandidateTargetMode::Continue
                    && node.truths.first() == Some(&true) =>
                {
                    Some((ProbStateId(index), super::MonitorVerdict::Accept))
                }
                _ => None,
            })
            .collect();
        let labels = if matches!(self.plan, Plan::Labels) {
            self.nodes.iter().map(|node| node.truths.clone()).collect()
        } else {
            Vec::new()
        };
        let monitor = match self.plan {
            Plan::Formula(monitor) => Some(*monitor),
            _ => None,
        };
        (
            ProbProduct {
                states: self.nodes.into_iter().map(|node| node.key).collect(),
                kind: self.kind,
                edges: self.edges,
                initial,
                decided,
                labels,
                fairness_operations: self.fairness_operations,
                fairness: self.fairness,
            },
            monitor,
        )
    }

    fn poll(&self) -> Step<()> {
        if let Some(cause) = self.cancel.tripped() {
            return Err(ProductRefusal::Stopped(ProductStop::Cancelled(cause)).into());
        }
        if self.cancel.poll() {
            if let Some(cause) = self.cancel.cause() {
                return Err(ProductRefusal::Stopped(ProductStop::Cancelled(cause)).into());
            }
        }
        Ok(())
    }

    fn stopped(limit: ModelCheckLimit, value: usize) -> Halt {
        Halt::Refusal(ProductRefusal::Stopped(ProductStop::Limit {
            limit,
            value: u64::try_from(value).unwrap_or(u64::MAX),
        }))
    }

    fn key_bytes(&self, state: &ModelSimState) -> Step<Vec<u8>> {
        match self.key_policy {
            Some(policy) => self
                .system
                .state_key_bytes_with_budget(
                    state,
                    policy.identity,
                    &mut **policy.budget.borrow_mut(),
                    self.cancel,
                )
                .map_err(Halt::Key),
            None => self
                .system
                .state_key(state, fsl_foundation::IdentityLimits::default())
                .map(|(bytes, _)| bytes)
                .map_err(|_| fault("state-key-encodes").into()),
        }
    }

    fn reserve_key(&self, bytes: usize) -> Step<()> {
        if let Some(policy) = self.key_policy {
            policy
                .budget
                .borrow_mut()
                .reserve(bytes)
                .map_err(|_| Halt::Key(ModelKeyError::Denied))?;
        }
        Ok(())
    }

    fn expansion_error(error: ModelKeyExpansionError) -> Halt {
        match error {
            ModelKeyExpansionError::Key(error) => Halt::Key(error),
            ModelKeyExpansionError::Expansion(stop) => ProductRefusal::Expansion(stop).into(),
        }
    }

    /// The truth of the claim's predicates at `state` under binding
    /// `binding`, or the first that has no value.
    fn truths(&mut self, state: &ModelSimState, binding: usize) -> Step<Result<Vec<bool>, Hit>> {
        let (model, state_key) = if self.key_policy.is_some() {
            (self.key_bytes(state)?, None)
        } else {
            let (model, key) = self
                .system
                .state_key(state, fsl_foundation::IdentityLimits::default())
                .map_err(|_| fault("state-key-encodes"))?;
            (model, Some(key))
        };
        if let Some(known) = self
            .readings
            .get(&binding)
            .and_then(|by_model| by_model.get(model.as_slice()))
        {
            return Ok(Ok(known.clone()));
        }
        let over = self.bindings[binding].1.as_ref();
        let predicates: Vec<(&CheckedPredicate, Option<&ObjectReference>)> = self
            .reads
            .iter()
            .map(|(_, predicate)| (*predicate, over))
            .collect();
        let readings = self
            .system
            .state_predicates_with_cancel(state, &predicates, self.cancel);
        self.poll()?;
        let readings = readings.map_err(|stop| Halt::Refusal(ProductRefusal::Expansion(stop)))?;
        let mut truths = Vec::with_capacity(readings.len());
        for ((read, _), reading) in self.reads.iter().zip(readings) {
            match reading {
                PredicateReading::True => truths.push(true),
                PredicateReading::False => truths.push(false),
                PredicateReading::Undefined { cause, .. } => {
                    if self.key_policy.is_some() {
                        return Err(Halt::UndefinedReading(cause));
                    }
                    let state_key =
                        state_key.ok_or_else(|| fault("legacy-reading-retains-state-key"))?;
                    return Ok(Err(Hit {
                        read: read.clone(),
                        cause,
                        state_key,
                        model,
                    }));
                }
                PredicateReading::Refused(outcome) => {
                    return Err(ProductRefusal::PredicateRefused(outcome).into());
                }
            }
        }
        self.reserve_key(std::mem::size_of::<(Vec<u8>, Vec<bool>)>().saturating_add(truths.len()))?;
        self.readings
            .entry(binding)
            .or_default()
            .insert(model, truths.clone());
        Ok(Ok(truths))
    }

    #[implements("FR-196-AC-6", "FR-196-AC-10")]
    fn key_of(
        &self,
        state: &ModelSimState,
        binding: usize,
        monitor: Option<MonitorState>,
        accumulator: Option<Accumulator>,
        pending: Option<Pending>,
    ) -> Step<ProbKey> {
        let model = self.key_bytes(state)?;
        Ok(self.key_from(model, binding, monitor, accumulator, pending))
    }

    /// The product key of the model state whose canonical bytes are `model`.
    fn key_from(
        &self,
        model: Vec<u8>,
        binding: usize,
        monitor: Option<MonitorState>,
        accumulator: Option<Accumulator>,
        pending: Option<Pending>,
    ) -> ProbKey {
        ProbKey {
            binding: self.bindings[binding].0.clone(),
            model,
            monitor,
            accumulator,
            pending,
            certificate_state: None,
        }
    }

    fn admit_state(&mut self, key: &ProbKey) -> Step<()> {
        if !self.index.contains_key(key) && !self.pending_keys.contains(key) {
            if self.nodes.len().saturating_add(self.pending_keys.len()) >= self.max_states {
                return Err(Self::stopped(
                    ModelCheckLimit::ExactProbMaxStates,
                    self.max_states,
                ));
            }
            self.pending_keys.insert(key.clone());
        }
        Ok(())
    }

    fn admit_edge(&self, pending: usize) -> Step<()> {
        self.poll()?;
        if self.edges.len().saturating_add(pending) >= self.max_transitions {
            return Err(Self::stopped(
                ModelCheckLimit::ExactProbMaxTransitions,
                self.max_transitions,
            ));
        }
        Ok(())
    }

    fn target(&mut self, arrival: Arrival, model: &ModelSimState, pending: usize) -> Step<Target> {
        self.admit_edge(pending)?;
        let state = self.admit_partial_state(&arrival.key)?.then(|| {
            super::observe_target_construction();
            TargetState {
                model: model.clone(),
                truths: arrival.truths,
                resolution: None,
            }
        });
        Ok(Target {
            key: arrival.key,
            state,
        })
    }

    fn admit_partial_state(&mut self, key: &ProbKey) -> Step<bool> {
        if self.index.contains_key(key) || self.pending_keys.contains(key) {
            return Ok(true);
        }
        if self.nodes.len().saturating_add(self.pending_keys.len()) >= self.max_states {
            self.partial = true;
            return Ok(false);
        }
        self.pending_keys.insert(key.clone());
        Ok(true)
    }

    /// Whether node `at` has decided control: it is never expanded for
    /// successors; [`Self::sweep`] reads what it reaches.
    fn is_decided(&self, at: usize) -> bool {
        let node = &self.nodes[at];
        node.key.monitor.is_some_and(MonitorState::is_decided)
            || (self.target_mode != CandidateTargetMode::Continue
                && node.truths.first() == Some(&true))
    }

    /// The state of `key`, numbered at its first discovery. A state with
    /// decided control is never expanded.
    fn intern(
        &mut self,
        key: ProbKey,
        target: TargetState,
        parent: Option<usize>,
        step: Option<TraceStep>,
        initial: u32,
    ) -> Step<usize> {
        if let Some(known) = self.index.get(&key) {
            return Ok(*known);
        }
        if self.nodes.len() >= self.max_states {
            return Err(Self::stopped(
                ModelCheckLimit::ExactProbMaxStates,
                self.max_states,
            ));
        }
        let id = self.nodes.len();
        let initial = parent
            .map(|parent| self.nodes[parent].initial)
            .unwrap_or(initial);
        self.reserve_key(
            key.model
                .len()
                .saturating_add(std::mem::size_of::<ProbKey>())
                .saturating_add(std::mem::size_of::<Node>())
                .saturating_add(std::mem::size_of::<usize>()),
        )?;
        self.index.insert(key.clone(), id);
        self.pending_keys.remove(&key);
        self.nodes.push(Node {
            key,
            model: target.model,
            truths: target.truths,
            parent: parent.map(|parent| (parent, step)),
            initial,
            resolution: target.resolution,
        });
        if !self.is_decided(id) {
            self.queue.push_back(id);
        }
        Ok(id)
    }

    /// The path to the state `hit` names: the steps to node `parent`, then
    /// `step`. A resolution has no step of its own: `resolved` is the model
    /// state it chose, which the path records on the draw step before it.
    fn undefined(
        &self,
        parent: Option<usize>,
        step: Option<&TraceStep>,
        resolved: Option<ProbKey>,
        initial: u32,
        key: ProbKey,
        hit: Hit,
    ) -> Halt {
        let mut steps: Vec<PathStep> = step
            .map(|step| PathStep {
                action: step.action.clone(),
                draw: step.draw.clone(),
                probability: step.probability.clone(),
                target: Box::new(key.clone()),
                resolved: None,
            })
            .into_iter()
            .collect();
        let mut chosen = resolved;
        let mut cursor = parent;
        let mut initial = initial;
        while let Some(at) = cursor {
            initial = self.nodes[at].initial;
            let Some((before, taken)) = &self.nodes[at].parent else {
                break;
            };
            match taken {
                Some(taken) => {
                    let resolution = chosen.take();
                    steps.push(PathStep {
                        action: taken.action.clone(),
                        draw: taken.draw.clone(),
                        probability: taken.probability.clone(),
                        target: Box::new(
                            resolution
                                .clone()
                                .unwrap_or_else(|| self.nodes[at].key.clone()),
                        ),
                        resolved: resolution.map(Box::new),
                    });
                }
                // The edge into `at` resolved a draw: `at` is the chosen
                // post-state.
                None => chosen = Some(self.nodes[at].key.clone()),
            }
            cursor = Some(*before);
        }
        steps.reverse();
        let probability = steps
            .iter()
            .fold(Rational::from_integer(Integer::one()), |total, step| {
                total.mul(&step.probability)
            });
        Halt::Undefined(Box::new(UndefinedPath {
            initial,
            binding: key.binding.clone(),
            position: u64::try_from(steps.len()).unwrap_or(u64::MAX),
            steps,
            probability,
            state_key: hit.state_key,
            key: Box::new(key),
            read: hit.read,
            cause: hit.cause,
        }))
    }

    fn binding_of(&self, at: usize) -> Step<usize> {
        self.bindings
            .iter()
            .position(|(instance, _)| *instance == self.nodes[at].key.binding)
            .ok_or_else(|| fault("node-binding-is-a-binding").into())
    }

    /// The state reached from node `at` by a model step to `post` with
    /// `rewards`, its control and amounts advanced once. `step` is the model
    /// step the path gains; a resolution gains none.
    fn advance(
        &mut self,
        at: usize,
        post: &ModelSimState,
        rewards: &[(Identifier, Value)],
        step: Option<&TraceStep>,
    ) -> Step<Arrival> {
        let binding = self.binding_of(at)?;
        let truths = match if self.plan.reads_post(self.nodes[at].key.monitor) {
            self.truths(post, binding)?
        } else {
            Ok(Vec::new())
        } {
            Ok(truths) => truths,
            Err(hit) => {
                let source = &self.nodes[at].key;
                let key = self.key_of(
                    post,
                    binding,
                    source.monitor,
                    source.accumulator.clone(),
                    None,
                )?;
                let resolved = step.is_none().then(|| key.clone());
                return Err(self.undefined(
                    Some(at),
                    step,
                    resolved,
                    self.nodes[at].initial,
                    key,
                    hit,
                ));
            }
        };
        let source = &self.nodes[at];
        let advanced = self.plan.step(
            source.key.monitor,
            source.key.accumulator.as_ref(),
            &source.truths,
            &truths,
            rewards,
            &mut self.measure_meter,
        );
        let (monitor, accumulator) = match advanced {
            Ok(next) => next,
            Err(PlanFault::Measure(fsl_eval::probabilistic::MeasureError::Undefined(cause))) => {
                let cause = UndefinedCause::Kernel(*cause);
                if self.key_policy.is_some() {
                    return Err(Halt::UndefinedReading(cause));
                }
                let key = self.key_of(
                    post,
                    binding,
                    source.key.monitor,
                    source.key.accumulator.clone(),
                    None,
                )?;
                let hit = Hit {
                    read: UndefinedRead::FractionValue,
                    cause,
                    state_key: StateKey::from_canonical_bytes(key.model.clone()),
                    model: key.model.clone(),
                };
                let resolved = step.is_none().then(|| key.clone());
                return Err(self.undefined(Some(at), step, resolved, source.initial, key, hit));
            }
            Err(error) => return Err(error.into()),
        };
        let key = self.key_of(post, binding, monitor, accumulator, None)?;
        Ok(Arrival { key, truths })
    }

    /// What a state whose expansion left its successors unknown halts the
    /// build with: an undefined contract conjunction refutes (ADR-018 UE-6)
    /// with the path to the state; only a refused one is undecided.
    #[implements("FR-196-AC-21")]
    fn undecided_successor(&self, at: usize, undecided: UndecidedSuccessor) -> Halt {
        // The replay of a bounded measure's undefined path (`measure.rs`)
        // does not yet read a contract site: that form stays undecided.
        let contract = undecided
            .undefined
            .clone()
            .filter(|_| !matches!(self.plan, Plan::Bounded(_)));
        let Some(contract) = contract else {
            return ProductRefusal::UndecidedSuccessor(undecided).into();
        };
        if self.key_policy.is_some() {
            return Halt::UndefinedReading(contract.cause);
        }
        let node = &self.nodes[at];
        let hit = Hit {
            cause: contract.cause.clone(),
            state_key: undecided.state,
            model: node.key.model.clone(),
            read: UndefinedRead::Contract(Box::new(contract)),
        };
        self.undefined(Some(at), None, None, node.initial, node.key.clone(), hit)
    }

    /// The number of model steps from an initial state to node `at`: its
    /// position in the claim's trace.
    fn position_of(&self, at: usize) -> u64 {
        let mut position = 0_u64;
        let mut cursor = at;
        while let Some((parent, taken)) = &self.nodes[cursor].parent {
            position += u64::from(taken.is_some());
            cursor = *parent;
        }
        position
    }

    /// The model states a state of decided control reaches, each read once for
    /// an undefined contract conjunction (ADR-018 UE-6, ADR-028 XV-8): a form
    /// that reads every position (an unbounded one, under admission) reads
    /// every reachable state, and a bounded event reads the states at the
    /// positions below its horizon. A decided node is never expanded, so the
    /// guard of a state beyond it is read here. The sweep adds no node or edge.
    /// Under admission it returns only that a conjunction has no value, for
    /// the caller to rebuild without admission; there it ends the build with
    /// the path to the state, the model steps beyond the decided node carrying
    /// its decided control. A refused conjunction changes nothing, as the
    /// decided verdict never read it. A bounded measure has no contract replay
    /// yet and stays undecided.
    #[implements("FR-196-AC-21")]
    fn sweep(&mut self) -> Step<()> {
        let horizon = match &self.plan {
            Plan::Labels if self.key_policy.is_some() => None,
            Plan::Formula(_) => Some(
                self.event_horizon
                    .ok_or_else(|| Halt::from(fault("an-event-has-a-horizon")))?,
            ),
            _ => return Ok(()),
        };
        let roots: Vec<usize> = (0..self.nodes.len())
            .filter(|at| {
                self.is_decided(*at)
                    && self.nodes[*at].resolution.is_none()
                    && horizon.is_none_or(|horizon| self.position_of(*at) < horizon)
            })
            .collect();
        if roots.is_empty() {
            return Ok(());
        }
        // The model states read, with the lowest trace position each was read
        // at: a state read at a position is read to the horizon from there, so
        // only a nearer position reads it again. An unbounded form reads every
        // state once, those a node expanded already included.
        let mut visited: BTreeMap<Vec<u8>, u64> = BTreeMap::new();
        if horizon.is_none() {
            for at in 0..self.nodes.len() {
                if !self.is_decided(at) {
                    visited.insert(self.nodes[at].key.model.clone(), 0);
                }
            }
        }
        // Under admission the sweep only reports that a conjunction has no
        // value, so no product key is built for a swept state.
        let keyed = self.key_policy.is_none();
        for root in roots {
            let first = self.position_of(root);
            let binding = self.binding_of(root)?;
            let (monitor, accumulator) = (
                self.nodes[root].key.monitor,
                self.nodes[root].key.accumulator.clone(),
            );
            let root_bytes = self.nodes[root].key.model.clone();
            if !self.admit_swept(&mut visited, &root_bytes, first)? {
                continue;
            }
            let mut swept = vec![Swept {
                model: self.nodes[root].model.clone(),
                key: keyed.then(|| self.nodes[root].key.clone()),
                parent: None,
                step: None,
                depth: 0,
            }];
            let mut next = 0;
            while next < swept.len() {
                self.poll()?;
                if horizon.is_some_and(|horizon| first + swept[next].depth >= horizon) {
                    next += 1;
                    continue;
                }
                let model = swept[next].model.clone();
                match self.swept_steps(&model) {
                    Ok(steps) => {
                        let depth = swept[next].depth + 1;
                        for (taken, post) in steps {
                            let bytes = self.key_bytes(&post)?;
                            if !self.admit_swept(&mut visited, &bytes, first + depth)? {
                                continue;
                            }
                            let key = keyed.then(|| {
                                self.key_from(bytes, binding, monitor, accumulator.clone(), None)
                            });
                            swept.push(Swept {
                                model: post,
                                key,
                                parent: Some(next),
                                step: Some(taken),
                                depth,
                            });
                        }
                    }
                    Err(SweptStop::Undefined(contract, state)) => {
                        if self.key_policy.is_some() {
                            return Err(Halt::UndefinedReading(contract.cause));
                        }
                        return Err(self.swept_path(root, &swept, next, contract, state));
                    }
                    Err(SweptStop::Halt(halt)) => return Err(halt),
                }
                next += 1;
            }
        }
        Ok(())
    }

    /// The undefined path to the contract conjunction found at swept state
    /// `last`: the path to decided node `root`, then the model steps the
    /// sweep took, which carry the decided control unchanged.
    fn swept_path(
        &self,
        root: usize,
        swept: &[Swept],
        last: usize,
        contract: fsl_eval::simulation::model::UndefinedContract,
        state: StateKey,
    ) -> Halt {
        let node = &self.nodes[root];
        let Some(last_key) = swept[last].key.clone() else {
            return fault("a-swept-path-has-keys").into();
        };
        let hit = Hit {
            cause: contract.cause.clone(),
            state_key: state,
            model: last_key.model.clone(),
            read: UndefinedRead::Contract(Box::new(contract)),
        };
        let Halt::Undefined(mut path) =
            self.undefined(Some(root), None, None, node.initial, node.key.clone(), hit)
        else {
            return fault("an-undefined-path-is-undefined").into();
        };
        let mut trail = Vec::new();
        let mut cursor = last;
        while let (Some(parent), Some(taken)) = (swept[cursor].parent, swept[cursor].step.clone()) {
            let Some(target) = swept[cursor].key.clone() else {
                return fault("a-swept-path-has-keys").into();
            };
            trail.push(PathStep {
                action: taken.action,
                draw: taken.draw,
                probability: taken.probability,
                target: Box::new(target),
                resolved: None,
            });
            cursor = parent;
        }
        trail.reverse();
        for step in trail {
            path.probability = path.probability.mul(&step.probability);
            path.steps.push(step);
        }
        path.position = u64::try_from(path.steps.len()).unwrap_or(u64::MAX);
        path.key = Box::new(last_key);
        Halt::Undefined(path)
    }

    /// Whether the model state `bytes` is new to the sweep, counted against
    /// the state limit like a product state.
    fn admit_swept(
        &mut self,
        visited: &mut BTreeMap<Vec<u8>, u64>,
        bytes: &[u8],
        position: u64,
    ) -> Step<bool> {
        if let Some(known) = visited.get_mut(bytes) {
            if *known <= position {
                return Ok(false);
            }
            *known = position;
            return Ok(true);
        }
        if visited.len() >= self.max_states {
            return Err(Self::stopped(
                ModelCheckLimit::ExactProbMaxStates,
                self.max_states,
            ));
        }
        self.reserve_key(bytes.len().saturating_add(std::mem::size_of::<Vec<u8>>()))?;
        visited.insert(bytes.to_vec(), position);
        Ok(true)
    }

    /// The model steps out of `model` with their post-states, or the
    /// undefined contract conjunction its expansion found.
    fn swept_steps(
        &mut self,
        model: &ModelSimState,
    ) -> Result<Vec<(TraceStep, ModelSimState)>, SweptStop> {
        let mut steps = Vec::new();
        match self.kind {
            ProductKind::Dtmc => {
                let workload = self
                    .workload
                    .ok_or_else(|| SweptStop::Halt(fault("a-dtmc-has-a-workload").into()))?;
                let result = match self.key_policy {
                    Some(policy) => self
                        .system
                        .weighted_steps_with_budget(
                            model,
                            workload,
                            policy.identity,
                            &mut **policy.budget.borrow_mut(),
                            self.cancel,
                        )
                        .map_err(Self::expansion_error),
                    None => self
                        .system
                        .weighted_steps_with_cancel(model, workload, self.cancel)
                        .map_err(|stop| Halt::Refusal(ProductRefusal::Expansion(stop))),
                };
                self.poll().map_err(SweptStop::Halt)?;
                match result.map_err(SweptStop::Halt)? {
                    Ok(weighted) => {
                        for step in weighted {
                            if let Some(identity) = step.identity {
                                steps.push((
                                    TraceStep {
                                        action: Some(identity),
                                        draw: Some(step.random),
                                        probability: step.probability,
                                    },
                                    step.post,
                                ));
                            }
                        }
                    }
                    Err(Unsteppable::UndecidedSuccessor(undecided)) => {
                        if let Some(contract) = undecided.undefined {
                            return Err(SweptStop::Undefined(contract, undecided.state));
                        }
                    }
                    Err(Unsteppable::NotMarkov(_)) => {}
                }
            }
            ProductKind::Mdp => {
                let result = match self.key_policy {
                    Some(policy) => self
                        .system
                        .actions_with_budget(
                            model,
                            policy.identity,
                            &mut **policy.budget.borrow_mut(),
                            self.cancel,
                        )
                        .map_err(Self::expansion_error),
                    None => self
                        .system
                        .actions_with_cancel(model, self.cancel)
                        .map_err(|stop| Halt::Refusal(ProductRefusal::Expansion(stop))),
                };
                self.poll().map_err(SweptStop::Halt)?;
                match result.map_err(SweptStop::Halt)? {
                    Ok(actions) => {
                        for action in actions {
                            for draw in action.draws {
                                for (_, post) in draw.successors {
                                    steps.push((
                                        TraceStep {
                                            action: Some(action.identity.clone()),
                                            draw: Some(draw.random.clone()),
                                            probability: draw.probability.clone(),
                                        },
                                        post,
                                    ));
                                }
                            }
                        }
                    }
                    Err(undecided) => {
                        if let Some(contract) = undecided.undefined {
                            return Err(SweptStop::Undefined(contract, undecided.state));
                        }
                    }
                }
            }
        }
        Ok(steps)
    }

    fn expand(&mut self, at: usize) -> Step<()> {
        self.pending_keys.clear();
        // A full transition prefix never constructs another successor relation.
        self.admit_edge(0)?;
        let planned = match self.nodes[at].resolution.take() {
            Some(resolution) => self.resolve(at, &resolution)?,
            None => {
                let model = self.nodes[at].model.clone();
                match self.kind {
                    ProductKind::Dtmc => self.weighted(at, &model)?,
                    ProductKind::Mdp => self.choices(at, &model)?,
                }
            }
        };
        for edge in planned {
            let Some(target) = edge.target.state else {
                self.partial = true;
                self.omitted.push((
                    at,
                    OmittedEdge {
                        kept_before: self.edges.len(),
                        choice: edge.choice,
                        action: edge.action,
                        draw: edge.draw,
                        probability: edge.probability,
                        rewards: edge.rewards,
                    },
                ));
                continue;
            };
            if self.edges.len() >= self.max_transitions {
                return Err(Self::stopped(
                    ModelCheckLimit::ExactProbMaxTransitions,
                    self.max_transitions,
                ));
            }
            let initial = self.nodes[at].initial;
            let to = self.intern(edge.target.key, target, Some(at), edge.step, initial)?;
            self.edges.push(ProbEdge {
                from: ProbStateId(at),
                choice: edge.choice,
                action: edge.action,
                draw: edge.draw,
                to: ProbStateId(to),
                probability: edge.probability,
                elapsed: Rational::from_integer(Integer::from(0_i64)),
                rewards: edge.rewards,
            });
        }
        Ok(())
    }

    /// The edges out of an intermediate state: each post-state of the draw,
    /// with probability 1.
    fn resolve(&mut self, at: usize, resolution: &Resolution) -> Step<Vec<Planned>> {
        let mut planned = Vec::new();
        for post in &resolution.successors {
            self.admit_edge(planned.len())?;
            let arrival = self.advance(at, post, &resolution.rewards, None)?;
            let target = self.target(arrival, post, planned.len())?;
            target.observe_edge();
            planned.push(Planned {
                choice: ProductAction::PostState(Box::new(target.key.clone())),
                action: None,
                draw: None,
                target,
                probability: Rational::from_integer(Integer::one()),
                rewards: self.system.zero_rewards(),
                step: None,
            });
        }
        Ok(planned)
    }

    /// A DTMC state's edges: FR-187's weighted steps.
    fn weighted(&mut self, at: usize, model: &ModelSimState) -> Step<Vec<Planned>> {
        super::observe_relation_construction();
        let workload = self
            .workload
            .ok_or_else(|| fault("a-dtmc-has-a-workload"))?;
        let result = match self.key_policy {
            Some(policy) => self
                .system
                .weighted_steps_with_budget(
                    model,
                    workload,
                    policy.identity,
                    &mut **policy.budget.borrow_mut(),
                    self.cancel,
                )
                .map_err(Self::expansion_error),
            None => self
                .system
                .weighted_steps_with_cancel(model, workload, self.cancel)
                .map_err(|stop| Halt::Refusal(ProductRefusal::Expansion(stop))),
        };
        self.poll()?;
        let steps = match result? {
            Ok(steps) => steps,
            Err(Unsteppable::NotMarkov(not_markov)) => {
                return Err(ProductRefusal::NotMarkov(not_markov).into());
            }
            Err(Unsteppable::UndecidedSuccessor(undecided)) => {
                return Err(self.undecided_successor(at, undecided));
            }
        };
        let mut planned = Vec::new();
        for step in steps {
            self.admit_edge(planned.len())?;
            let Some(identity) = step.identity else {
                planned.push(self.terminal(at, model, step.rewards, planned.len())?);
                continue;
            };
            let taken = TraceStep {
                action: Some(identity.clone()),
                draw: Some(step.random.clone()),
                probability: step.probability.clone(),
            };
            let arrival = self.advance(at, &step.post, &step.rewards, Some(&taken))?;
            let target = self.target(arrival, &step.post, planned.len())?;
            target.observe_edge();
            planned.push(Planned {
                choice: ProductAction::Identity(identity.clone()),
                action: Some(identity),
                draw: Some(step.random),
                target,
                probability: step.probability,
                rewards: step.rewards,
                step: Some(taken),
            });
        }
        Ok(planned)
    }

    /// An MDP state's edges: FR-187's actions, each draw an edge, or an
    /// intermediate state when it has several post-states (SCH-2).
    fn choices(&mut self, at: usize, model: &ModelSimState) -> Step<Vec<Planned>> {
        super::observe_relation_construction();
        let result = match self.key_policy {
            Some(policy) => self
                .system
                .actions_with_budget(
                    model,
                    policy.identity,
                    &mut **policy.budget.borrow_mut(),
                    self.cancel,
                )
                .map_err(Self::expansion_error),
            None => self
                .system
                .actions_with_cancel(model, self.cancel)
                .map_err(|stop| Halt::Refusal(ProductRefusal::Expansion(stop))),
        };
        self.poll()?;
        let actions = match result? {
            Ok(actions) => actions,
            Err(undecided) => {
                return Err(self.undecided_successor(at, undecided));
            }
        };
        if actions.is_empty() {
            let rewards = self.system.zero_rewards();
            return Ok(vec![self.terminal(at, model, rewards, 0)?]);
        }
        let mut planned = Vec::new();
        for action in actions {
            self.action(at, model, action, &mut planned)?;
        }
        Ok(planned)
    }

    /// The edges of one action of an MDP state.
    fn action(
        &mut self,
        at: usize,
        model: &ModelSimState,
        action: Action,
        planned: &mut Vec<Planned>,
    ) -> Step<()> {
        let first = planned.len();
        for draw in action.draws {
            let taken = TraceStep {
                action: Some(action.identity.clone()),
                draw: Some(draw.random.clone()),
                probability: draw.probability.clone(),
            };
            let mut successors = draw.successors;
            match successors.len() {
                0 => {
                    let state = self.key_bytes(model)?;
                    return Err(ProductRefusal::NotMarkov(Box::new(NotMarkov {
                        state: fsl_eval::simulation::StateKey::from_canonical_bytes(state),
                        transition: action.identity,
                        drawn_transition: draw.transition,
                        random: draw.random,
                        post_states: 0,
                        post_state_examples: Vec::new(),
                    }))
                    .into());
                }
                1 => {
                    let (_, post) = successors.remove(0);
                    let arrival = self.advance(at, &post, &draw.rewards, Some(&taken))?;
                    // Draws of one action that reach the same post-state with
                    // the same rewards are one edge with their summed
                    // probability.
                    if let Some(merged) = planned[first..].iter_mut().find(|edge| {
                        edge.target.key == arrival.key && same_rewards(&edge.rewards, &draw.rewards)
                    }) {
                        merged.probability = merged.probability.add(&draw.probability);
                        merged.draw = None;
                        merged.step = Some(TraceStep {
                            action: taken.action,
                            draw: None,
                            probability: merged.probability.clone(),
                        });
                        continue;
                    }
                    let target = self.target(arrival, &post, planned.len())?;
                    target.observe_edge();
                    planned.push(Planned {
                        choice: ProductAction::Identity(action.identity.clone()),
                        action: Some(action.identity.clone()),
                        draw: Some(draw.random),
                        target,
                        probability: draw.probability,
                        rewards: draw.rewards,
                        step: Some(taken),
                    });
                }
                _ => {
                    let identity_bytes = self
                        .system
                        .transition_key(&successors[0].0)
                        .map_err(|_| fault("transition-key-encodes"))?;
                    let (monitor, accumulator) = {
                        let source = &self.nodes[at].key;
                        (source.monitor, source.accumulator.clone())
                    };
                    let binding = self.binding_of(at)?;
                    let key = self.key_of(
                        model,
                        binding,
                        monitor,
                        accumulator,
                        Some(Pending {
                            identity: action.identity.clone(),
                            random: draw.random.clone(),
                            identity_bytes,
                        }),
                    )?;
                    // The intermediate keeps the source's model state, so its
                    // predicates read as the source's; the monitor and the
                    // accumulator advance at the resolution, once.
                    self.admit_edge(planned.len())?;
                    let state = self.admit_partial_state(&key)?.then(|| {
                        super::observe_target_construction();
                        TargetState {
                            model: model.clone(),
                            truths: self.nodes[at].truths.clone(),
                            resolution: Some(Resolution {
                                successors: successors.into_iter().map(|(_, post)| post).collect(),
                                rewards: draw.rewards.clone(),
                            }),
                        }
                    });
                    let target = Target { key, state };
                    target.observe_edge();
                    planned.push(Planned {
                        choice: ProductAction::Identity(action.identity.clone()),
                        action: Some(action.identity.clone()),
                        draw: Some(draw.random),
                        target,
                        probability: draw.probability,
                        rewards: draw.rewards,
                        step: Some(taken),
                    });
                }
            }
        }
        Ok(())
    }

    /// The edge of a terminal model state. A bounded event closes under the
    /// profile's false extension into a decided state; anything else takes
    /// the stutter step, which repeats the state.
    fn terminal(
        &mut self,
        at: usize,
        model: &ModelSimState,
        rewards: Vec<(Identifier, Value)>,
        pending: usize,
    ) -> Step<Planned> {
        let probability = Rational::from_integer(Integer::one());
        let taken = TraceStep {
            action: None,
            draw: None,
            probability: probability.clone(),
        };
        let target = if self.plan.closes_at_terminal() {
            let Some(MonitorState::Formula(FormulaState::Running(running))) =
                self.nodes[at].key.monitor
            else {
                return Err(fault("a-terminal-event-state-is-undecided").into());
            };
            let verdict = self.plan.close(running)?;
            let binding = self.binding_of(at)?;
            let key = self.key_of(
                model,
                binding,
                Some(MonitorState::Formula(FormulaState::Decided(verdict))),
                None,
                None,
            )?;
            self.target(
                Arrival {
                    key,
                    truths: self.nodes[at].truths.clone(),
                },
                model,
                pending,
            )?
        } else {
            let arrival = self.advance(at, model, &rewards, Some(&taken))?;
            self.target(arrival, model, pending)?
        };
        target.observe_edge();
        Ok(Planned {
            choice: ProductAction::TerminalStutter,
            action: None,
            draw: None,
            target,
            probability,
            rewards,
            step: Some(taken),
        })
    }
}
