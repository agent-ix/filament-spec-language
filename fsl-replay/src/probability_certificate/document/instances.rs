// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-875: an `on each` reachability claim settled instance by instance.
//!
//! Every instance of the claim's trigger binder is read over its own phase
//! graph: the model graph from one initial state, each state in the phase
//! `waiting` (the guard has not yet held) or `active` (the guard held at or
//! before the state). An instance is read from its first activating position
//! and conditioned on activating (FR-196 "Conditioned readings"), so its
//! value is `Pr(eventually target and activated) / Pr(activated)` and an
//! instance with `Pr(activated) = 0` holds vacuously.
//!
//! The certificate is the FR-201 `conditional` certificate in FSpec FR-412's
//! spelling: mode `ratio` with an `exact` numerator and denominator part, or
//! mode `vacuous` when no instance activates. The checker rebuilds every
//! phase graph from the checked subject with its own graph code (the
//! `rebuilt` section below) and verifies each row against its successors;
//! neither part is trusted from the document.
//!
//! Known refusals (settled `unsupported`, never a verdict): a workload with
//! free delays, statistical evidence, a form other than unbounded
//! `eventually`, timed claims and fairness sets.

use super::*;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{ScheduledIdentity, TransitionId, WeightedStep};
use fsl_semantics::check::{ActivationGuard, ClaimActivation, StatePredicate};
use quire_canonical::Encode;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub(super) const PROFILE: &str = "filament.probability-objective/on-each-reachability/v1";

type Key = ProbabilityProductKey;

/// One positive-probability step of an instance's phase graph.
#[derive(Clone)]
struct Edge {
    to: Key,
    probability: Rational,
    action: ScheduledIdentity,
    transition: ModelTransition,
    draw: Vec<filament_exact::Value>,
}

/// One state of an instance's phase graph.
#[derive(Clone)]
struct WalkState {
    /// The activation guard has held at or before this state.
    active: bool,
    /// `Some(true)` for an active state at the target, `Some(false)` for a
    /// state with no successor, `None` for a state with successors.
    terminal: Option<bool>,
    edges: Vec<Edge>,
    choices: Vec<Vec<(Key, Rational)>>,
    choice_ids: Vec<Option<ScheduledIdentity>>,
}

/// The phase graph of one instance from one initial state.
#[derive(Clone)]
struct Walk {
    root: Key,
    states: BTreeMap<Key, WalkState>,
    models: BTreeMap<Key, ModelSimState>,
}

/// One instance read from one initial state.
struct Root {
    initial_index: usize,
    over: Option<(ModelInstance, ObjectReference)>,
    reference: ObjectReference,
    start: ModelSimState,
    walk: Walk,
}

fn exact_binding(instance: &ModelInstance) -> crate::ExactBinding {
    crate::ExactBinding {
        population: instance.population.node.clone(),
        key: instance.key.clone(),
    }
}

/// The entry's `binding` and `instance` members (FR-741 "Binding"): the
/// `over` object, null when the claim has no `over` parameter, and the
/// `on each` object.
fn entry_binding(
    over: Option<&(ModelInstance, ObjectReference)>,
    instance: &ModelInstance,
) -> (Option<crate::ExactBinding>, crate::ExactBinding) {
    (
        over.map(|(over, _)| exact_binding(over)),
        exact_binding(instance),
    )
}

/// The limits one item spends across every instance's graph.
#[derive(Default)]
struct Spent {
    states: usize,
    edges: usize,
    storage: u64,
}

impl Spent {
    fn state(
        &mut self,
        key: &Key,
        limits: ProbabilityDocumentLimits,
    ) -> Result<(), ProbabilityDocumentError> {
        self.states = self.states.saturating_add(1);
        if self.states > limits.states {
            return Err(ExactProbabilityGraphError::LimitReached.into());
        }
        let actual = self
            .storage
            .saturating_add(u64::try_from(key.as_bytes().len()).unwrap_or(u64::MAX));
        if actual > limits.storage_bytes {
            return Err(exceeded(
                ProbabilityDocumentLimit::StorageBytes,
                limits.storage_bytes,
                actual,
            ));
        }
        self.storage = actual;
        Ok(())
    }

    fn edge(&mut self, limits: ProbabilityDocumentLimits) -> Result<(), ProbabilityDocumentError> {
        self.edges = self.edges.saturating_add(1);
        if self.edges > limits.edges {
            return Err(ExactProbabilityGraphError::EdgeLimitReached.into());
        }
        Ok(())
    }
}

struct Subject<'a> {
    claim: &'a CheckedProbabilisticClaim,
    guard: Option<ActivationGuard<'a>>,
    target: &'a fsl_semantics::check::CheckedPredicate,
    bound: &'a Bound,
    workload: Option<&'a Workload>,
    instance_type: &'a filament_exact::ValueType,
}

/// The `on each` claim `item` names among `claims`, which must be the
/// subject's own claims.
fn each_claim<'a>(
    system: &ModelSystem<'_>,
    claims: &'a [CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    cancel: &Cancel,
) -> Result<&'a CheckedProbabilisticClaim, ProbabilityDocumentError> {
    if !std::ptr::eq(system.input().probabilistic.claims(), claims) {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cancel)?;
    if !matches!(claim.activation, ClaimActivation::Each { .. }) {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    Ok(claim)
}

fn subject<'a>(
    system: &ModelSystem<'a>,
    claims: &'a [CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    cancel: &Cancel,
) -> Result<Subject<'a>, ProbabilityDocumentError> {
    admit(system, each_claim(system, claims, item, cancel)?)
}

/// The `on-each-reachability/v1` profile's admission of `claim` (FR-875
/// "Profile boundary"): anything outside it is `Unsupported`.
fn admit<'a>(
    system: &ModelSystem<'a>,
    claim: &'a CheckedProbabilisticClaim,
) -> Result<Subject<'a>, ProbabilityDocumentError> {
    use fsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
    let ProbForm::Reach {
        bound,
        target: ReachTarget::Eventually(target),
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let ClaimActivation::Each { value_type, .. } = &claim.activation else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    if !claim.fairness.is_empty()
        || fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(fsl_semantics::check::TemporalProfile::InfiniteTrace)
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let workload = claim_workload(system, claim)?;
    if workload.is_none() && !claim.fairness.is_empty() {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    Ok(Subject {
        claim,
        guard: claim.activation.guard(),
        target,
        bound,
        workload,
        instance_type: value_type,
    })
}

fn instances_of(
    system: &ModelSystem<'_>,
    value_type: &filament_exact::ValueType,
) -> Result<Vec<(ModelInstance, ObjectReference)>, ProbabilityDocumentError> {
    let filament_exact::ValueType::Reference(object) = value_type else {
        return Err(ExactProbabilityGraphError::UnsupportedBinding.into());
    };
    system
        .instances(value_type)
        .map_err(|error| match error {
            fsl_eval::simulation::NotSimulated::RequiresBound(_) => {
                ExactProbabilityGraphError::RequiresBound
            }
            fsl_eval::simulation::NotSimulated::Extent(
                fsl_semantics::family::ClassifyFailure::Limit(_),
            ) => ExactProbabilityGraphError::ExtentLimit,
            _ => ExactProbabilityGraphError::ExpansionRefused,
        })?
        .into_iter()
        .map(|instance| {
            let reference = system
                .bound_reference(object.clone(), &instance)
                .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
            Ok((instance, reference))
        })
        .collect()
}

fn holds<P: StatePredicate>(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    predicate: &P,
    over: Option<&ObjectReference>,
    instance: &ObjectReference,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    use fsl_eval::simulation::model::PredicateReading;
    let readings = system.state_predicates_bound_with_cancel(
        state,
        &[(
            predicate,
            over,
            vec![filament_exact::Value::Reference(instance.clone())],
        )],
        cancel,
    );
    poll(cancel)?;
    let reading = readings
        .map_err(|stop| {
            ProbabilityDocumentError::Build(Box::new(crate::ExactProbBuildIssue::Expansion(stop)))
        })?
        .pop()
        .ok_or(ProbabilityDocumentError::Malformed)?;
    match reading {
        PredicateReading::True => Ok(true),
        PredicateReading::False => Ok(false),
        PredicateReading::Undefined { cause, .. } => {
            Err(ProbabilityDocumentError::UndefinedPredicate(cause))
        }
        PredicateReading::Refused(outcome) => Err(ProbabilityDocumentError::Build(Box::new(
            crate::ExactProbBuildIssue::Predicate(outcome),
        ))),
    }
}

/// The product key of `state` in `phase` for one instance.
fn phase_key(
    system: &ModelSystem<'_>,
    over: Option<&ObjectReference>,
    instance: &ObjectReference,
    state: &ModelSimState,
    phase: ActivationPhase,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Key, ProbabilityDocumentError> {
    phase_key_with_root(system, over, instance, state, phase, None, limits, cancel)
}

fn phase_key_with_root(
    system: &ModelSystem<'_>,
    over: Option<&ObjectReference>,
    instance: &ObjectReference,
    state: &ModelSimState,
    phase: ActivationPhase,
    root: Option<usize>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Key, ProbabilityDocumentError> {
    let io = ProbabilityDocumentIoLimits {
        bytes: limits.bytes,
        storage_bytes: limits.storage_bytes,
    };
    let (model, _) = system
        .state_key(state)
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    let key = ProbabilityProductKey::for_model_state_with_limits(over, &model, io, cancel)?
        .with_activation(instance, phase, io, cancel)?;
    match root {
        Some(root) => key::with_root_index(&key, root, io, cancel),
        None => Ok(key),
    }
}

fn phase_of(active: bool) -> ActivationPhase {
    if active {
        ActivationPhase::Active
    } else {
        ActivationPhase::Waiting
    }
}

/// The positive-probability steps of `state` under the workload; none for a
/// state that only stutters.
fn moves(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    workload: Option<&Workload>,
    cancel: &Cancel,
) -> Result<Vec<WeightedStep>, ProbabilityDocumentError> {
    use fsl_eval::simulation::model::Unsteppable;
    let steps = if let Some(workload) = workload {
        let stepped = system.weighted_steps_with_cancel(state, workload, cancel);
        poll(cancel)?;
        stepped
            .map_err(|stop| {
                ProbabilityDocumentError::Build(Box::new(crate::ExactProbBuildIssue::Expansion(
                    stop,
                )))
            })?
            .map_err(|stop| {
                ProbabilityDocumentError::Build(Box::new(match stop {
                    Unsteppable::NotMarkov(cause) => crate::ExactProbBuildIssue::NotMarkov(cause),
                    Unsteppable::UndecidedSuccessor(_) => {
                        crate::ExactProbBuildIssue::UndecidedSuccessor
                    }
                }))
            })?
    } else {
        let actions = system
            .actions_with_cancel(state, cancel)
            .map_err(|stop| {
                ProbabilityDocumentError::Build(Box::new(crate::ExactProbBuildIssue::Expansion(
                    stop,
                )))
            })?
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let mut steps = Vec::new();
        for action in actions {
            for draw in action.draws {
                let [(transition, post)] = draw.successors.as_slice() else {
                    return Err(ProbabilityDocumentError::Unsupported);
                };
                steps.push(WeightedStep {
                    transition: TransitionId::Operation(transition.clone()),
                    identity: Some(action.identity.clone()),
                    random: draw.random.clone(),
                    post: post.clone(),
                    probability: draw.probability.clone(),
                    rewards: draw.rewards.clone(),
                });
            }
        }
        steps
    };
    if steps
        .iter()
        .any(|step| matches!(step.transition, TransitionId::Stutter))
    {
        return if steps.len() == 1 {
            Ok(Vec::new())
        } else {
            Err(ProbabilityDocumentError::Unsupported)
        };
    }
    Ok(steps
        .into_iter()
        .filter(|step| step.probability != rational(0))
        .collect())
}

fn walk(
    system: &ModelSystem<'_>,
    subject: &Subject<'_>,
    start: &ModelSimState,
    binding: (Option<&ObjectReference>, &ObjectReference),
    limits: ProbabilityDocumentLimits,
    spent: &mut Spent,
    cancel: &Cancel,
) -> Result<Walk, ProbabilityDocumentError> {
    let (over, instance) = binding;
    let key = |state: &ModelSimState, active: bool| {
        phase_key(
            system,
            over,
            instance,
            state,
            phase_of(active),
            limits,
            cancel,
        )
    };
    let activates = |state: &ModelSimState| match &subject.guard {
        Some(guard) => holds(system, state, guard, over, instance, cancel),
        None => Ok(true),
    };
    let first = activates(start)?;
    let root = key(start, first)?;
    spent.state(&root, limits)?;
    let mut states = BTreeMap::new();
    let mut models = BTreeMap::from([(root.clone(), start.clone())]);
    let mut seen = BTreeSet::from([root.clone()]);
    let mut pending = VecDeque::from([(root.clone(), start.clone(), first)]);
    while let Some((at, state, active)) = pending.pop_front() {
        poll(cancel)?;
        if active && holds(system, &state, subject.target, over, instance, cancel)? {
            states.insert(
                at,
                WalkState {
                    active,
                    terminal: Some(true),
                    edges: Vec::new(),
                    choices: Vec::new(),
                    choice_ids: Vec::new(),
                },
            );
            continue;
        }
        let steps = moves(system, &state, subject.workload, cancel)?;
        if steps.is_empty() {
            states.insert(
                at,
                WalkState {
                    active,
                    terminal: Some(false),
                    edges: Vec::new(),
                    choices: Vec::new(),
                    choice_ids: Vec::new(),
                },
            );
            continue;
        }
        let mut edges = Vec::with_capacity(steps.len());
        let mut choices: Vec<(Option<ScheduledIdentity>, Vec<(Key, Rational)>)> = Vec::new();
        for step in steps {
            poll(cancel)?;
            let next_active = active || activates(&step.post)?;
            let to = key(&step.post, next_active)?;
            spent.edge(limits)?;
            let (TransitionId::Operation(transition), Some(action)) =
                (step.transition, step.identity)
            else {
                return Err(ProbabilityDocumentError::Malformed);
            };
            if seen.insert(to.clone()) {
                spent.state(&to, limits)?;
                models.insert(to.clone(), step.post.clone());
                pending.push_back((to.clone(), step.post, next_active));
            }
            let choice_identity = subject.workload.is_none().then(|| action.clone());
            edges.push(Edge {
                to: to.clone(),
                probability: step.probability.clone(),
                action,
                transition,
                draw: step.random,
            });
            if let Some((_, choice_edges)) = choices
                .iter_mut()
                .find(|(known, _)| *known == choice_identity)
            {
                choice_edges.push((to, step.probability));
            } else {
                choices.push((choice_identity, vec![(to, step.probability)]));
            }
        }
        states.insert(
            at,
            WalkState {
                active,
                terminal: None,
                edges,
                choice_ids: choices
                    .iter()
                    .map(|(identity, _)| identity.clone())
                    .collect(),
                choices: choices.into_iter().map(|(_, edges)| edges).collect(),
            },
        );
    }
    Ok(Walk {
        root,
        states,
        models,
    })
}

/// The instance set of an `on each` claim (FR-875 "Instances"): its initial
/// states, `over` instances (one `None` without an `over` parameter) and
/// activation objects, each expanded once.
struct Instances {
    initials: Vec<ModelSimState>,
    overs: Vec<Option<(ModelInstance, ObjectReference)>>,
    each: Vec<(ModelInstance, ObjectReference)>,
}

fn canonical_initials(
    system: &ModelSystem<'_>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<ModelSimState>, ProbabilityDocumentError> {
    let mut ordered = BTreeMap::new();
    let mut storage = 0_u64;
    for state in system.initial() {
        poll(cancel)?;
        let (key, _) = system
            .state_key(&state)
            .map_err(|_| ProbabilityDocumentError::Malformed)?;
        if !ordered.contains_key(&key) {
            let states = ordered.len().saturating_add(1);
            if states > limits.states {
                return Err(ExactProbabilityGraphError::LimitReached.into());
            }
            let actual = storage.saturating_add(u64::try_from(key.len()).unwrap_or(u64::MAX));
            if actual > limits.storage_bytes {
                return Err(exceeded(
                    ProbabilityDocumentLimit::StorageBytes,
                    limits.storage_bytes,
                    actual,
                ));
            }
            storage = actual;
            ordered.insert(key, state);
        }
    }
    Ok(ordered.into_values().collect())
}

impl Instances {
    /// Read the populations only; the claim's profile is not consulted.
    fn of(
        system: &ModelSystem<'_>,
        claim: &CheckedProbabilisticClaim,
    ) -> Result<Self, ProbabilityDocumentError> {
        let ClaimActivation::Each { value_type, .. } = &claim.activation else {
            return Err(ProbabilityDocumentError::Unsupported);
        };
        let overs = match &claim.over {
            None => vec![None],
            Some(over) => instances_of(system, &over.value_type)?
                .into_iter()
                .map(Some)
                .collect(),
        };
        Ok(Self {
            initials: system.initial(),
            overs,
            each: instances_of(system, value_type)?,
        })
    }

    fn is_empty(&self) -> bool {
        self.initials.is_empty() || self.overs.is_empty() || self.each.is_empty()
    }
}

fn roots(
    system: &ModelSystem<'_>,
    subject: &Subject<'_>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<Root>, ProbabilityDocumentError> {
    let instances = Instances::of(system, subject.claim)?;
    roots_over(system, subject, &instances, limits, cancel)
}

fn roots_over(
    system: &ModelSystem<'_>,
    subject: &Subject<'_>,
    instances: &Instances,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<Root>, ProbabilityDocumentError> {
    let mut spent = Spent::default();
    let mut found = Vec::new();
    for (initial_index, start) in canonical_initials(system, limits, cancel)?.iter().enumerate() {
        for over in &instances.overs {
            for (_, reference) in &instances.each {
                let walked = walk(
                    system,
                    subject,
                    start,
                    (over.as_ref().map(|(_, reference)| reference), reference),
                    limits,
                    &mut spent,
                    cancel,
                )?;
                found.push(Root {
                    initial_index,
                    over: over.clone(),
                    reference: reference.clone(),
                    start: start.clone(),
                    walk: walked,
                });
            }
        }
    }
    Ok(found)
}

#[derive(Clone, Copy)]
enum Part {
    Numerator,
    Denominator,
}

struct Arith<'a> {
    meter: &'a mut filament_exact::Meter,
    cancel: &'a Cancel,
    limits: ProbabilityDocumentLimits,
}

impl Arith<'_> {
    fn apply(
        &mut self,
        operation: filament_exact::RationalArithmetic<'_>,
    ) -> Result<Rational, ProbabilityDocumentError> {
        let value = arithmetic(operation, self.meter, self.cancel)?;
        check_scalar(&value, self.limits)?;
        Ok(value)
    }

    /// The value a state must have when each successor has the value
    /// `value` gives it.
    fn expected<'k>(
        &mut self,
        active: bool,
        terminal: Option<bool>,
        edges: impl Iterator<Item = (&'k Key, &'k Rational)>,
        part: Part,
        value: &dyn Fn(&Key) -> Result<Rational, ProbabilityDocumentError>,
    ) -> Result<Rational, ProbabilityDocumentError> {
        match (part, active, terminal) {
            (Part::Denominator, true, _) | (Part::Numerator, true, Some(true)) => Ok(rational(1)),
            (_, _, Some(_)) => Ok(rational(0)),
            (_, _, None) => {
                let mut sum = rational(0);
                let mut mass = rational(0);
                for (target, probability) in edges {
                    let next = value(target)?;
                    let product = self.apply(filament_exact::RationalArithmetic::Multiply(
                        probability,
                        &next,
                    ))?;
                    sum = self.apply(filament_exact::RationalArithmetic::Add(&sum, &product))?;
                    mass =
                        self.apply(filament_exact::RationalArithmetic::Add(&mass, probability))?;
                }
                if mass != rational(1) {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                Ok(sum)
            }
        }
    }
}

/// Successors before predecessors. A cycle has no ordering and is refused:
/// the rows below are the unique solution only on an acyclic graph.
fn postorder<'a>(
    walk: &'a Walk,
    cancel: &Cancel,
) -> Result<Vec<&'a Key>, ProbabilityDocumentError> {
    let mut order = Vec::with_capacity(walk.states.len());
    let mut done: BTreeSet<&Key> = BTreeSet::new();
    let mut open: BTreeSet<&Key> = BTreeSet::new();
    let mut stack: Vec<(&Key, usize)> = vec![(&walk.root, 0)];
    open.insert(&walk.root);
    while let Some((at, next)) = stack.pop() {
        poll(cancel)?;
        let state = walk
            .states
            .get(at)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        match state.edges.get(next) {
            None => {
                open.remove(at);
                done.insert(at);
                order.push(at);
            }
            Some(edge) => {
                stack.push((at, next + 1));
                if open.contains(&edge.to) {
                    return Err(ExactProbabilityGraphError::Cycle.into());
                }
                if !done.contains(&edge.to) {
                    open.insert(&edge.to);
                    stack.push((&edge.to, 0));
                }
            }
        }
    }
    Ok(order)
}

type Rows = BTreeMap<Key, Rational>;

fn solve(
    roots: &[Root],
    arith: &mut Arith<'_>,
) -> Result<(Rows, Rows, BTreeMap<Key, u64>), ProbabilityDocumentError> {
    let mut numerator = Rows::new();
    let mut denominator = Rows::new();
    let mut heights: BTreeMap<Key, u64> = BTreeMap::new();
    let row = |rows: &Rows, target: &Key| {
        rows.get(target)
            .cloned()
            .ok_or(ProbabilityDocumentError::Malformed)
    };
    for root in roots {
        for key in postorder(&root.walk, arith.cancel)? {
            if numerator.contains_key(key) {
                continue;
            }
            let state = root
                .walk
                .states
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let upper = arith.expected(
                state.active,
                state.terminal,
                state.edges.iter().map(|edge| (&edge.to, &edge.probability)),
                Part::Numerator,
                &|target| row(&numerator, target),
            )?;
            let lower = arith.expected(
                state.active,
                state.terminal,
                state.edges.iter().map(|edge| (&edge.to, &edge.probability)),
                Part::Denominator,
                &|target| row(&denominator, target),
            )?;
            // Height rank: terminal 0, else one plus the largest successor.
            let height = if state.terminal.is_some() {
                0
            } else {
                let mut most = 0;
                for edge in &state.edges {
                    let next = heights
                        .get(&edge.to)
                        .ok_or(ProbabilityDocumentError::Malformed)?;
                    most = most.max(*next);
                }
                most + 1
            };
            heights.insert(key.clone(), height);
            numerator.insert(key.clone(), upper);
            denominator.insert(key.clone(), lower);
        }
    }
    Ok((numerator, denominator, heights))
}

/// A root's conditioned value, absent when it never activates.
fn conditioned(
    root: &Key,
    numerator: &Rows,
    denominator: &Rows,
    arith: &mut Arith<'_>,
) -> Result<Option<Rational>, ProbabilityDocumentError> {
    let (Some(upper), Some(lower)) = (numerator.get(root), denominator.get(root)) else {
        return Err(ProbabilityDocumentError::Malformed);
    };
    if *lower == rational(0) {
        return Ok(None);
    }
    arith
        .apply(filament_exact::RationalArithmetic::Divide(upper, lower))
        .map(Some)
}

fn meets(bound: &Bound, value: &Rational) -> bool {
    match bound {
        Bound::AtLeast(threshold) => value >= threshold,
        Bound::AtMost(threshold) => value <= threshold,
    }
}

fn rows_of(rows: &Rows) -> Vec<ExactProbabilityValue> {
    rows.iter()
        .map(|(state, value)| ExactProbabilityValue {
            state: state.clone(),
            value: value.clone(),
        })
        .collect()
}

/// What `on each` settles to.
pub enum OnEachOutcome {
    /// The population has no instance (FR-875-AC-1).
    NoInstance,
    /// Every instance holds: the unchecked `conditional` certificate bytes.
    Holds {
        /// Canonical certificate document bytes.
        document: Vec<u8>,
    },
    /// The first instance in key order that fails, with its evidence.
    Fails {
        /// Unreplayed path-set evidence naming the instance.
        evidence: Box<ProbabilisticCounterexample>,
    },
}

/// Whether the `on each` claim `item` has an instance over `system`: an
/// initial state, an `over` instance when the claim has an `over` parameter,
/// and an object of the activation's type (FR-875 "Instances").
///
/// This reads only the populations, before the claim's profile is admitted,
/// so an empty instance set settles `no-instance` whatever the claim's form
/// or evidence kind.
///
/// No product graph is built, so no state or edge limit is reached.
///
/// # Errors
/// `claims` that are not the subject's own, a claim whose activation is not
/// `on each`, a population that has no universe or does not expand, or the
/// supplied cancellation.
pub fn on_each_has_instance(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    let claim = each_claim(system, claims, item, cancel)?;
    Ok(!Instances::of(system, claim)?.is_empty())
}

/// Settle every instance of the `on each` claim `item` over `system`.
/// `max_paths` bounds a refutation's path set (`exact_prob.max_witness_paths`).
///
/// # Errors
/// An unsupported claim form or subject, a refused expansion or predicate,
/// the configured limits or the supplied cancellation.
pub fn on_each_outcome(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    limits: ProbabilityDocumentLimits,
    max_paths: u64,
    cancel: &Cancel,
) -> Result<OnEachOutcome, ProbabilityDocumentError> {
    let claim = each_claim(system, claims, item, cancel)?;
    let instances = Instances::of(system, claim)?;
    // An empty instance set is `no-instance` before the profile is admitted.
    if instances.is_empty() {
        return Ok(OnEachOutcome::NoInstance);
    }
    let subject = admit(system, claim)?;
    let found = roots_over(system, &subject, &instances, limits, cancel)?;
    if subject.workload.is_none() {
        return reset_optimum_outcome(system, item, &subject, &found, limits, cancel);
    }
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut arith = Arith {
        meter: &mut meter,
        cancel,
        limits,
    };
    let (numerator, denominator, heights) = solve(&found, &mut arith)?;
    let ranking: Vec<(Key, u64)> = heights.into_iter().collect();
    let mut activating = false;
    for root in &found {
        let value = conditioned(&root.walk.root, &numerator, &denominator, &mut arith)?;
        activating |= value.is_some();
        if value
            .as_ref()
            .is_some_and(|value| !meets(subject.bound, value))
        {
            let evidence = evidence(system, root, subject.bound, item, max_paths, &mut arith)?;
            return Ok(OnEachOutcome::Fails {
                evidence: Box::new(evidence),
            });
        }
    }
    let document = if activating {
        encode(
            item,
            Some(Parts {
                numerator: &rows_of(&numerator),
                denominator: &rows_of(&denominator),
                ranking: &ranking,
            }),
            limits,
            cancel,
        )?
    } else {
        encode(item, None, limits, cancel)?
    };
    Ok(OnEachOutcome::Holds { document })
}

fn reset_optimum_outcome(
    system: &ModelSystem<'_>,
    item: &ObligationIdentity,
    subject: &Subject<'_>,
    roots: &[Root],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<OnEachOutcome, ProbabilityDocumentError> {
    let minimum = matches!(subject.bound, Bound::AtLeast(_));
    let mut post_values = BTreeMap::new();
    let mut post_ranks = BTreeMap::new();
    let mut reset_values = BTreeMap::new();
    let mut reset_ranks = BTreeMap::new();
    let mut reset_policy = BTreeMap::new();
    let mut any_activation = false;
    let mut reset_meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut reset_process = false;
    for root in roots {
        let starts_waiting = root
            .walk
            .states
            .get(&root.walk.root)
            .is_some_and(|state| !state.active);
        if starts_waiting && can_activate(&root.walk, &root.walk.root, cancel)? {
            reset_process = true;
        }
    }
    let initial_count = roots
        .iter()
        .map(|root| root.initial_index)
        .collect::<BTreeSet<_>>()
        .len();
    let root_qualified = initial_count > 1 && reset_process;
    for root in roots {
        poll(cancel)?;
        let starts_waiting = root
            .walk
            .states
            .get(&root.walk.root)
            .is_some_and(|state| !state.active);
        if starts_waiting && !can_activate(&root.walk, &root.walk.root, cancel)? {
            continue;
        }
        let walk = if root_qualified {
            indexed_walk(&root.walk, root.initial_index, limits, cancel)?
        } else {
            root.walk.clone()
        };
        let (active_memo, active_ranks) = post_graph_optimum(
            &walk,
            minimum,
            limits,
            &mut reset_meter,
            cancel,
        )?;
        if walk
            .states
            .get(&walk.root)
            .is_some_and(|state| state.active)
        {
            merge_root_rows(&mut post_values, active_memo)?;
            merge_root_rows(&mut post_ranks, active_ranks)?;
            any_activation = true;
            continue;
        }
        let result = reset_graph_optimum(
            system,
            &walk,
            &walk.root,
            &active_memo,
            minimum,
            limits,
            &mut reset_meter,
            cancel,
        )?;
        merge_root_rows(&mut post_values, active_memo)?;
        merge_root_rows(&mut post_ranks, active_ranks)?;
        if result.reaches_activation {
            any_activation = true;
        }
        merge_root_rows(&mut reset_values, result.values)?;
        merge_root_rows(&mut reset_ranks, result.ranks)?;
        merge_root_rows(&mut reset_policy, result.policy)?;
    }
    if !any_activation {
        return Ok(OnEachOutcome::Holds {
            document: encode(item, None, limits, cancel)?,
        });
    }
    let post_rows = rows_of(&post_values);
    let post_rank_rows = post_ranks.into_iter().collect::<Vec<_>>();
    let reset_rows = rows_of(&reset_values);
    let reset_rank_rows = reset_ranks.into_iter().collect::<Vec<_>>();
    let reset_policy_rows = reset_policy
        .into_iter()
        .map(|(key, action)| {
            let bytes = action
                .to_bytes()
                .map_err(|_| ProbabilityDocumentError::Malformed)?;
            let value =
                serde_json::from_slice(&bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
            Ok((key, value))
        })
        .collect::<Result<Vec<_>, ProbabilityDocumentError>>()?;
    let optimization = if minimum { "minimum" } else { "maximum" };
    let document = encode_optimum(
        item,
        optimization,
        &post_rows,
        &post_rank_rows,
        &reset_rows,
        &reset_rank_rows,
        &reset_policy_rows,
        limits,
        cancel,
    )?;
    Ok(OnEachOutcome::Holds { document })
}

fn indexed_walk(
    source: &Walk,
    initial_index: usize,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Walk, ProbabilityDocumentError> {
    let io = ProbabilityDocumentIoLimits {
        bytes: limits.bytes,
        storage_bytes: limits.storage_bytes,
    };
    let mut mapped = BTreeMap::new();
    for key in source.states.keys() {
        poll(cancel)?;
        mapped.insert(key.clone(), key::with_root_index(key, initial_index, io, cancel)?);
    }
    let remap = |key: &Key| {
        mapped
            .get(key)
            .cloned()
            .ok_or(ProbabilityDocumentError::Malformed)
    };
    let mut states = BTreeMap::new();
    for (key, state) in &source.states {
        poll(cancel)?;
        let mut state = state.clone();
        for edge in &mut state.edges {
            edge.to = remap(&edge.to)?;
        }
        for choice in &mut state.choices {
            for (to, _) in choice {
                *to = remap(to)?;
            }
        }
        states.insert(remap(key)?, state);
    }
    let models = source
        .models
        .iter()
        .map(|(key, model)| Ok((remap(key)?, model.clone())))
        .collect::<Result<BTreeMap<_, _>, ProbabilityDocumentError>>()?;
    Ok(Walk {
        root: remap(&source.root)?,
        states,
        models,
    })
}

fn merge_root_rows<T: Eq>(
    rows: &mut BTreeMap<Key, T>,
    additions: BTreeMap<Key, T>,
) -> Result<(), ProbabilityDocumentError> {
    for (key, value) in additions {
        if rows.get(&key).is_some_and(|previous| previous != &value) {
            // A canonical product row cannot encode two different root-local
            // reset values or policies for the same key.
            return Err(ProbabilityDocumentError::Unsupported);
        }
        rows.insert(key, value);
    }
    Ok(())
}

fn post_graph_optimum(
    walk: &Walk,
    minimum: bool,
    limits: ProbabilityDocumentLimits,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<(BTreeMap<Key, Rational>, BTreeMap<Key, u64>), ProbabilityDocumentError> {
    let active = walk
        .states
        .iter()
        .filter_map(|(key, state)| state.active.then_some(key.clone()))
        .collect::<BTreeSet<_>>();
    let mut choices = BTreeMap::new();
    let mut targets = BTreeSet::new();
    let mut zero = BTreeSet::new();
    let mut edges = 0_u64;
    for key in &active {
        poll(cancel)?;
        let state = walk
            .states
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        match state.terminal {
            Some(true) => {
                targets.insert(key.clone());
            }
            Some(false) => {
                zero.insert(key.clone());
            }
            None => {
                let mut state_choices = Vec::new();
                for choice in &state.choices {
                    poll(cancel)?;
                    let mut mass = rational(0);
                    for (to, probability) in choice {
                        poll(cancel)?;
                        if !active.contains(to) || probability.is_zero() {
                            return Err(ProbabilityDocumentError::Malformed);
                        }
                        mass = mass.add(probability);
                        edges = edges.saturating_add(1);
                    }
                    if mass != rational(1) {
                        return Err(ProbabilityDocumentError::Malformed);
                    }
                    state_choices.push(choice.clone());
                }
                if state_choices.is_empty() {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                choices.insert(key.clone(), state_choices);
            }
        }
    }
    if active.len() > limits.states || edges > limits.edges as u64 {
        return Err(ProbabilityDocumentError::Limit {
            limit: if active.len() > limits.states {
                ProbabilityDocumentLimit::States
            } else {
                ProbabilityDocumentLimit::Edges
            },
            bound: if active.len() > limits.states {
                limits.states as u64
            } else {
                limits.edges as u64
            },
            actual: if active.len() > limits.states {
                active.len() as u64
            } else {
                edges
            },
        });
    }
    // Compute qualitative zero states first. For minimum reachability, a
    // state has value zero when some action can remain entirely in a
    // non-target set; start from every non-target state to retain end
    // components such as a self-loop that avoids the target forever.
    // for maximum, it is zero when no action graph path reaches a target.
    if minimum {
        zero.extend(active.difference(&targets).cloned());
        loop {
            poll(cancel)?;
            let mut removed = Vec::new();
            for (key, actions) in &choices {
                poll(cancel)?;
                if !zero.contains(key) {
                    continue;
                }
                if !actions
                    .iter()
                    .any(|action| action.iter().all(|(to, _)| zero.contains(to)))
                {
                    removed.push(key.clone());
                }
            }
            if removed.is_empty() {
                break;
            }
            for key in removed {
                zero.remove(&key);
            }
        }
    } else {
        let mut can_reach = targets.clone();
        loop {
            poll(cancel)?;
            let mut changed = false;
            for (key, actions) in &choices {
                poll(cancel)?;
                if can_reach.contains(key)
                    || actions
                        .iter()
                        .any(|action| action.iter().any(|(to, _)| can_reach.contains(to)))
                {
                    changed |= can_reach.insert(key.clone());
                }
            }
            if !changed {
                break;
            }
        }
        zero.extend(active.difference(&can_reach).cloned());
    }
    let positive = active
        .difference(&zero)
        .filter(|key| !targets.contains(*key))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut values = zero
        .iter()
        .cloned()
        .map(|key| (key, rational(0)))
        .collect::<BTreeMap<_, _>>();
    values.extend(targets.iter().cloned().map(|key| (key, rational(1))));
    let mut ranks = zero
        .iter()
        .chain(targets.iter())
        .cloned()
        .map(|key| (key, 0_u64))
        .collect::<BTreeMap<_, _>>();
    if positive.is_empty() {
        return Ok((values, ranks));
    }

    // Build a proper seed strategy from CE-3 progress ranks. Every action
    // at a positive minimum state must progress; maximum needs one such action.
    let mut policy = BTreeMap::new();
    loop {
        poll(cancel)?;
        let mut changed = false;
        for key in &positive {
            poll(cancel)?;
            if ranks.contains_key(key) {
                continue;
            }
            let actions = choices.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            let selected = actions.iter().enumerate().find_map(|(index, action)| {
                let has_progress = action.iter().any(|(to, probability)| {
                    !probability.is_zero()
                        && (positive.contains(to) || targets.contains(to))
                        && ranks.contains_key(to)
                });
                has_progress.then_some(index)
            });
            if let Some(index) = selected {
                let progress_rank = actions[index]
                    .iter()
                    .filter_map(|(to, probability)| {
                        (!probability.is_zero()
                            && (positive.contains(to) || targets.contains(to)))
                            .then(|| ranks.get(to).copied())
                            .flatten()
                    })
                    .min()
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                if !minimum
                    || actions.iter().all(|action| {
                        action.iter().any(|(to, probability)| {
                            !probability.is_zero()
                                && ranks.contains_key(to)
                                && (positive.contains(to) || targets.contains(to))
                        })
                    })
                {
                    ranks.insert(key.clone(), progress_rank.saturating_add(1));
                    policy.insert(key.clone(), index);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    if positive.iter().any(|key| !policy.contains_key(key)) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let positive_choices = choices
        .iter()
        .filter(|(key, _)| positive.contains(*key))
        .map(|(key, actions)| (key.clone(), actions.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut iterations = 0_u64;
    loop {
        poll(cancel)?;
        iterations = iterations.saturating_add(1);
        meter
            .charge(
                filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                    .work(filament_exact::Integer::from(edges.max(1))),
            )
            .map_err(|error| match cancel.tripped() {
                Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                None => ProbabilityDocumentError::Accounting(Box::new(error)),
            })?;
        if iterations > edges.saturating_mul(10_000).max(10_000) {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Edges,
                bound: limits.edges as u64,
                actual: iterations,
            });
        }
        values = evaluate_reset_policy(
            &positive,
            &positive_choices,
            &policy,
            &values,
            limits,
            meter,
            cancel,
        )?;
        let current = values.clone();
        let mut improved = false;
        for key in &positive {
            poll(cancel)?;
            let selected = *policy.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            let current_value = expectation_value(&choices[key][selected], &current)?;
            for (candidate, action) in choices[key].iter().enumerate() {
                poll(cancel)?;
                let candidate_value = expectation_value(action, &current)?;
                let better = if minimum {
                    candidate_value < current_value
                } else {
                    candidate_value > current_value
                };
                if !better {
                    continue;
                }
                let mut candidate_policy = policy.clone();
                candidate_policy.insert(key.clone(), candidate);
                if let Some(candidate_ranks) = proper_policy_ranks(
                    &positive,
                    &positive_choices,
                    &candidate_policy,
                    &targets,
                    meter,
                    cancel,
                )? {
                    policy = candidate_policy;
                    if !minimum {
                        ranks.extend(candidate_ranks);
                    }
                    improved = true;
                    break;
                }
            }
        }
        if !improved {
            break;
        }
    }
    for key in &positive {
        poll(cancel)?;
        let rank = ranks.get(key).copied().ok_or(ProbabilityDocumentError::Malformed)?;
        let progresses = |action: &Vec<(Key, Rational)>| {
            action.iter().any(|(to, probability)| {
                !probability.is_zero()
                    && values.get(to).is_some_and(|value| !value.is_zero())
                    && ranks.get(to).is_some_and(|next| *next < rank)
            })
        };
        if (minimum && !choices[key].iter().all(progresses))
            || (!minimum
                && !choices[key].iter().enumerate().any(|(index, action)| {
                    policy.get(key) == Some(&index)
                        && progresses(action)
                        && expectation_value(action, &values)
                            .is_ok_and(|value| value == values[key])
                }))
        {
            return Err(ProbabilityDocumentError::Malformed);
        }
    }
    Ok((values, ranks))
}

struct ResetSolution {
    reaches_activation: bool,
    values: BTreeMap<Key, Rational>,
    ranks: BTreeMap<Key, u64>,
    policy: BTreeMap<Key, crate::ProductAction>,
}

fn reset_graph_optimum(
    system: &ModelSystem<'_>,
    walk: &Walk,
    root: &Key,
    post: &BTreeMap<Key, Rational>,
    minimum: bool,
    limits: ProbabilityDocumentLimits,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<ResetSolution, ProbabilityDocumentError> {
    let mut waiting = BTreeSet::from([root.clone()]);
    let mut stack = vec![root.clone()];
    while let Some(key) = stack.pop() {
        poll(cancel)?;
        let state = walk
            .states
            .get(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        for edge in &state.edges {
            if walk.states.get(&edge.to).is_some_and(|next| !next.active)
                && waiting.insert(edge.to.clone())
            {
                stack.push(edge.to.clone());
            }
        }
    }
    let mut activates = BTreeMap::new();
    for key in &waiting {
        activates.insert(key.clone(), can_activate(walk, key, cancel)?);
    }
    let reaches_activation = activates.get(root).copied().unwrap_or(false);
    let mut choices: BTreeMap<Key, Vec<Vec<(Key, Rational)>>> = BTreeMap::new();
    let mut choice_ids: BTreeMap<Key, Vec<Option<ScheduledIdentity>>> = BTreeMap::new();
    for key in &waiting {
        let state = walk
            .states
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let owner = root;
        if !activates[key] {
            choices.insert(key.clone(), vec![vec![(owner.clone(), rational(1))]]);
            choice_ids.insert(key.clone(), vec![None]);
            continue;
        }
        let actions = state.choices.clone();
        let ids = state.choice_ids.clone();
        choices.insert(key.clone(), actions);
        choice_ids.insert(key.clone(), ids);
    }
    let reset_eligible = waiting_end_component_members(&waiting, &choices, meter, cancel)?;
    for key in &reset_eligible {
        if !activates[key] {
            continue;
        }
        choices
            .get_mut(key)
            .ok_or(ProbabilityDocumentError::Malformed)?
            .push(vec![(root.clone(), rational(1))]);
        choice_ids
            .get_mut(key)
            .ok_or(ProbabilityDocumentError::Malformed)?
            .push(None);
    }
    let targets = waiting
        .iter()
        .flat_map(|key| {
            walk.states
                .get(key)
                .into_iter()
                .flat_map(|state| state.edges.iter())
                .filter(|edge| walk.states.get(&edge.to).is_some_and(|next| next.active))
                .map(|edge| edge.to.clone())
        })
        .collect::<BTreeSet<_>>();
    let mut ranks = BTreeMap::new();
    let mut policy = BTreeMap::new();
    for target in &targets {
        ranks.insert(target.clone(), 0_u64);
    }
    loop {
        let mut changed = false;
        for key in &waiting {
            if ranks.contains_key(key) {
                continue;
            }
            let Some(actions) = choices.get(key) else {
                return Err(ProbabilityDocumentError::Malformed);
            };
            let selected = actions.iter().enumerate().find_map(|(index, action)| {
                action
                    .iter()
                    .filter_map(|(to, _)| ranks.get(to))
                    .min()
                    .copied()
                    .map(|rank| (index, rank))
            });
            if let Some((index, rank)) = selected {
                ranks.insert(key.clone(), rank.saturating_add(1));
                policy.insert(key.clone(), index);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if !waiting.iter().all(|key| ranks.contains_key(key)) {
        return Err(ExactProbabilityGraphError::Cycle.into());
    }
    let mut values;
    // Proper policy iteration: each single-state improvement is retained
    // only when the selected policy still has a decreasing progress rank.
    for _ in 0..10_000 {
        values = evaluate_reset_policy(&waiting, &choices, &policy, post, limits, meter, cancel)?;
        let mut improved = false;
        for key in &waiting {
            let current = *policy.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            let current_value = expectation_value(&choices[key][current], &values)?;
            for (candidate, action) in choices[key].iter().enumerate() {
                let next_value = expectation_value(action, &values)?;
                let better = if minimum {
                    next_value < current_value
                } else {
                    next_value > current_value
                };
                if !better {
                    continue;
                }
                let mut candidate_policy = policy.clone();
                candidate_policy.insert(key.clone(), candidate);
                if let Some(candidate_ranks) = proper_policy_ranks(
                    &waiting,
                    &choices,
                    &candidate_policy,
                    &targets,
                    meter,
                    cancel,
                )? {
                    policy = candidate_policy;
                    ranks.extend(candidate_ranks);
                    improved = true;
                    break;
                }
            }
        }
        if !improved {
            break;
        }
    }
    values = evaluate_reset_policy(&waiting, &choices, &policy, post, limits, meter, cancel)?;
    let action_map = waiting
        .iter()
        .map(|key| {
            let selected = *policy.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            let ids = choice_ids
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let action = if ids.get(selected).is_some_and(Option::is_none) {
                crate::ProductAction::Reset
            } else {
                crate::ProductAction::Identity(
                    crate::Schedule::of(
                        system,
                        ids.get(selected)
                            .and_then(Option::as_ref)
                            .ok_or(ProbabilityDocumentError::Malformed)?,
                    )
                    .map_err(|_| ProbabilityDocumentError::Malformed)?,
                )
            };
            Ok((key.clone(), action))
        })
        .collect::<Result<BTreeMap<_, _>, ProbabilityDocumentError>>()?;
    Ok(ResetSolution {
        reaches_activation,
        values: waiting
            .iter()
            .filter_map(|key| values.get(key).cloned().map(|value| (key.clone(), value)))
            .collect(),
        ranks: waiting
            .iter()
            .filter_map(|key| ranks.get(key).copied().map(|rank| (key.clone(), rank)))
            .collect(),
        policy: action_map,
    })
}

fn evaluate_reset_policy(
    waiting: &BTreeSet<Key>,
    choices: &BTreeMap<Key, Vec<Vec<(Key, Rational)>>>,
    policy: &BTreeMap<Key, usize>,
    post: &BTreeMap<Key, Rational>,
    limits: ProbabilityDocumentLimits,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<BTreeMap<Key, Rational>, ProbabilityDocumentError> {
    let keys = waiting.iter().cloned().collect::<Vec<_>>();
    let indices = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let size = keys.len();
    let cells = size
        .checked_mul(size.checked_add(1).ok_or(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::StorageBytes,
            bound: limits.storage_bytes,
            actual: u64::MAX,
        })?)
        .ok_or(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::StorageBytes,
            bound: limits.storage_bytes,
            actual: u64::MAX,
        })?;
    let matrix_storage = u64::try_from(cells)
        .ok()
        .and_then(|cells| cells.checked_mul(std::mem::size_of::<Rational>() as u64))
        .ok_or(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::StorageBytes,
            bound: limits.storage_bytes,
            actual: u64::MAX,
        })?;
    let row_storage = u64::try_from(size)
        .ok()
        .and_then(|rows| rows.checked_mul(std::mem::size_of::<Vec<Rational>>() as u64))
        .ok_or(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::StorageBytes,
            bound: limits.storage_bytes,
            actual: u64::MAX,
        })?;
    let actual_storage = matrix_storage
        .checked_add(row_storage)
        .ok_or(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::StorageBytes,
            bound: limits.storage_bytes,
            actual: u64::MAX,
        })?;
    if actual_storage > limits.storage_bytes {
        return Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::StorageBytes,
            bound: limits.storage_bytes,
            actual: actual_storage,
        });
    }
    let n = u64::try_from(size).unwrap_or(u64::MAX);
    let elimination_work = n
        .checked_mul(n)
        .and_then(|work| work.checked_mul(n.saturating_add(1)))
        .ok_or(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::Edges,
            bound: u64::try_from(limits.edges).unwrap_or(u64::MAX),
            actual: u64::MAX,
        })?;
    meter
        .charge(
            filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                .work(filament_exact::Integer::from(elimination_work)),
        )
        .map_err(|error| match cancel.tripped() {
            Some(cause) => ProbabilityDocumentError::Cancelled(cause),
            None => ProbabilityDocumentError::Accounting(Box::new(error)),
        })?;
    let mut matrix = vec![vec![rational(0); size + 1]; size];
    for (row, key) in keys.iter().enumerate() {
        poll(cancel)?;
        matrix[row][row] = rational(1);
        let selected = *policy.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let action = choices
            .get(key)
            .and_then(|actions| actions.get(selected))
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let mut mass = rational(0);
        for (to, probability) in action {
            mass = mass.add(probability);
            if let Some(column) = indices.get(to) {
                matrix[row][*column] = matrix[row][*column].sub(probability);
            } else {
                let terminal = post.get(to).ok_or(ProbabilityDocumentError::Malformed)?;
                matrix[row][size] = matrix[row][size].add(&probability.mul(terminal));
            }
        }
        if mass != rational(1) {
            return Err(ProbabilityDocumentError::Malformed);
        }
    }
    for column in 0..size {
        poll(cancel)?;
        let pivot = (column..size)
            .find(|row| !matrix[*row][column].is_zero())
            .ok_or(ExactProbabilityGraphError::Cycle)?;
        matrix.swap(column, pivot);
        let divisor = matrix[column][column].clone();
        for cell in column..=size {
            poll(cancel)?;
            matrix[column][cell] = matrix[column][cell]
                .div(&divisor)
                .ok_or(ProbabilityDocumentError::Malformed)?;
        }
        for row in 0..size {
            poll(cancel)?;
            if row == column || matrix[row][column].is_zero() {
                continue;
            }
            let factor = matrix[row][column].clone();
            for cell in column..=size {
                matrix[row][cell] = matrix[row][cell].sub(&factor.mul(&matrix[column][cell]));
            }
        }
    }
    let mut values = post.clone();
    for (row, key) in keys.into_iter().enumerate() {
        values.insert(key, matrix[row][size].clone());
    }
    Ok(values)
}

fn proper_policy_ranks(
    waiting: &BTreeSet<Key>,
    choices: &BTreeMap<Key, Vec<Vec<(Key, Rational)>>>,
    policy: &BTreeMap<Key, usize>,
    terminal: &BTreeSet<Key>,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<Option<BTreeMap<Key, u64>>, ProbabilityDocumentError> {
    let mut ranks = terminal
        .iter()
        .cloned()
        .map(|key| (key, 0_u64))
        .collect::<BTreeMap<_, _>>();
    loop {
        poll(cancel)?;
        let mut changed = false;
        for key in waiting {
            poll(cancel)?;
            meter
                .charge(
                    filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                        .work(filament_exact::Integer::from(1_u64)),
                )
                .map_err(|error| match cancel.tripped() {
                    Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                    None => ProbabilityDocumentError::Accounting(Box::new(error)),
                })?;
            if ranks.contains_key(key) {
                continue;
            }
            let Some(index) = policy.get(key) else {
                return Ok(None);
            };
            let Some(action) = choices.get(key).and_then(|actions| actions.get(*index)) else {
                return Ok(None);
            };
            let next = action
                .iter()
                .filter(|(_, probability)| !probability.is_zero())
                .filter_map(|(to, _)| ranks.get(to))
                .min()
                .copied();
            if let Some(next) = next {
                ranks.insert(key.clone(), next.saturating_add(1));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    Ok(waiting.iter().all(|key| ranks.contains_key(key)).then(|| {
        ranks
            .into_iter()
            .filter(|(key, _)| waiting.contains(key))
            .collect()
    }))
}

fn waiting_end_component_members(
    waiting: &BTreeSet<Key>,
    choices: &BTreeMap<Key, Vec<Vec<(Key, Rational)>>>,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<BTreeSet<Key>, ProbabilityDocumentError> {
    fn reachable(
        start: &Key,
        choices: &BTreeMap<Key, Vec<Vec<(Key, Rational)>>>,
        candidate: &BTreeSet<Key>,
        meter: &mut filament_exact::Meter,
        cancel: &Cancel,
    ) -> Result<BTreeSet<Key>, ProbabilityDocumentError> {
        let mut seen = BTreeSet::from([start.clone()]);
        let mut stack = vec![start.clone()];
        while let Some(key) = stack.pop() {
            poll(cancel)?;
            charge_reset_graph_work(meter, cancel)?;
            for action in choices
                .get(&key)
                .ok_or(ProbabilityDocumentError::Malformed)?
            {
                charge_reset_graph_work(meter, cancel)?;
                if action.is_empty()
                    || action.iter().any(|(next, probability)| {
                        !probability.is_zero() && !candidate.contains(next)
                    })
                {
                    continue;
                }
                for (next, probability) in action {
                    charge_reset_graph_work(meter, cancel)?;
                    if !probability.is_zero() && seen.insert(next.clone())
                    {
                        stack.push(next.clone());
                    }
                }
            }
        }
        Ok(seen)
    }

    let mut eligible = BTreeSet::new();
    let mut candidates = vec![waiting.clone()];
    while let Some(mut candidate) = candidates.pop() {
        poll(cancel)?;
        loop {
            let mut retained = BTreeSet::new();
            for key in &candidate {
                poll(cancel)?;
                charge_reset_graph_work(meter, cancel)?;
                let actions = choices
                    .get(key)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                let mut has_closed_action = false;
                for action in actions {
                    let mut closed = !action.is_empty();
                    for (next, probability) in action {
                        charge_reset_graph_work(meter, cancel)?;
                        if !probability.is_zero() && !candidate.contains(next) {
                            closed = false;
                        }
                    }
                    has_closed_action |= closed;
                }
                if has_closed_action {
                    retained.insert(key.clone());
                }
            }
            if retained == candidate {
                break;
            }
            candidate = retained;
            if candidate.is_empty() {
                break;
            }
        }
        if candidate.is_empty() {
            continue;
        }
        let mut remaining = candidate.clone();
        let mut components = Vec::new();
        while let Some(seed) = remaining.iter().next().cloned() {
            let forward = reachable(&seed, choices, &candidate, meter, cancel)?;
            let mut component = BTreeSet::new();
            for key in &remaining {
                let back = reachable(key, choices, &candidate, meter, cancel)?;
                if forward.contains(key) && back.contains(&seed) {
                    component.insert(key.clone());
                }
            }
            if component.is_empty() {
                component.insert(seed);
            }
            remaining.retain(|key| !component.contains(key));
            components.push(component);
        }
        if components.len() == 1 && components[0] == candidate {
            eligible.extend(candidate);
        } else {
            candidates.extend(components);
        }
    }
    Ok(eligible)
}

fn charge_reset_graph_work(
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    meter
        .charge(
            filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                .work(filament_exact::Integer::one()),
        )
        .map_err(|error| match cancel.tripped() {
            Some(cause) => ProbabilityDocumentError::Cancelled(cause),
            None => ProbabilityDocumentError::Accounting(Box::new(error)),
        })
}

fn can_activate(
    walk: &Walk,
    root: &Key,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    let mut seen = BTreeSet::from([root.clone()]);
    let mut stack = vec![root.clone()];
    while let Some(key) = stack.pop() {
        poll(cancel)?;
        for edge in &walk
            .states
            .get(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?
            .edges
        {
            if walk.states.get(&edge.to).is_some_and(|state| state.active) {
                return Ok(true);
            }
            if seen.insert(edge.to.clone()) {
                stack.push(edge.to.clone());
            }
        }
    }
    Ok(false)
}

fn expectation_value(
    edges: &[(Key, Rational)],
    values: &BTreeMap<Key, Rational>,
) -> Result<Rational, ProbabilityDocumentError> {
    let mut mass = rational(0);
    let mut sum = rational(0);
    for (key, probability) in edges {
        sum =
            sum.add(&probability.mul(values.get(key).ok_or(ProbabilityDocumentError::Malformed)?));
        mass = mass.add(probability);
    }
    if mass != rational(1) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(sum)
}

/// The keys from which an active state is reachable, the state itself
/// included.
fn activatable<'a>(
    walk: &'a Walk,
    cancel: &Cancel,
) -> Result<BTreeSet<&'a Key>, ProbabilityDocumentError> {
    let mut set: BTreeSet<&Key> = BTreeSet::new();
    for key in postorder(walk, cancel)? {
        let state = walk
            .states
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if state.active || state.edges.iter().any(|edge| set.contains(&edge.to)) {
            set.insert(key);
        }
    }
    Ok(set)
}

/// How a path ending at a state counts toward FSpec FR-413's two masses.
#[derive(Eq, PartialEq)]
enum Leaf {
    /// Activated, and the event is false (`>= θ`) or true (`<= θ`).
    Violating,
    /// No activating state is reachable from here.
    NotActivated,
    /// Activated and not violating: left out of the path set.
    Other,
}

fn leaf_of(bound: &Bound, state: &WalkState, activates: bool) -> Option<Leaf> {
    if !activates {
        return Some(Leaf::NotActivated);
    }
    if state.active && state.terminal.is_some() {
        let violating = match bound {
            Bound::AtLeast(_) => state.terminal == Some(false),
            Bound::AtMost(_) => state.terminal == Some(true),
        };
        return Some(if violating {
            Leaf::Violating
        } else {
            Leaf::Other
        });
    }
    None
}

/// FSpec FR-413: the evidence refutes when `V > (1 - θ) · (1 - N)` for
/// `>= θ` and `V > θ · (1 - N)` for `<= θ`, strictly.
fn refutes(
    bound: &Bound,
    violating: &Rational,
    not_activated: &Rational,
    arith: &mut Arith<'_>,
) -> Result<bool, ProbabilityDocumentError> {
    let rest = arith.apply(filament_exact::RationalArithmetic::Subtract(
        &rational(1),
        not_activated,
    ))?;
    let share = match bound {
        Bound::AtLeast(threshold) => arith.apply(filament_exact::RationalArithmetic::Subtract(
            &rational(1),
            threshold,
        ))?,
        Bound::AtMost(threshold) => threshold.clone(),
    };
    let goal = arith.apply(filament_exact::RationalArithmetic::Multiply(&share, &rest))?;
    Ok(*violating > goal)
}

fn paths_limit(max_paths: u64) -> ProbabilityDocumentError {
    ProbabilityDocumentError::Build(Box::new(crate::ExactProbBuildIssue::Limit {
        limit: crate::ModelCheckLimit::ExactProbMaxWitnessPaths,
        value: max_paths,
    }))
}

/// A frontier node of the most-probable-first path search.
struct Frontier {
    probability: Rational,
    order: usize,
    at: Key,
    trail: Vec<Edge>,
}

impl PartialEq for Frontier {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Frontier {}
impl PartialOrd for Frontier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Frontier {
    fn cmp(&self, other: &Self) -> Ordering {
        self.probability
            .cmp(&other.probability)
            .then(other.order.cmp(&self.order))
    }
}

fn evidence(
    system: &ModelSystem<'_>,
    root: &Root,
    bound: &Bound,
    item: &ObligationIdentity,
    max_paths: u64,
    arith: &mut Arith<'_>,
) -> Result<ProbabilisticCounterexample, ProbabilityDocumentError> {
    let reachable = activatable(&root.walk, arith.cancel)?;
    let mut paths: Vec<Vec<Edge>> = Vec::new();
    let mut violating = rational(0);
    let mut not_activated = rational(0);
    let mut order = 0_usize;
    let mut frontier = BinaryHeap::from([Frontier {
        probability: rational(1),
        order,
        at: root.walk.root.clone(),
        trail: Vec::new(),
    }]);
    // Most probable first; the graph is acyclic, so every trail ends.
    while let Some(node) = frontier.pop() {
        poll(arith.cancel)?;
        let state = root
            .walk
            .states
            .get(&node.at)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        match leaf_of(bound, state, reachable.contains(&node.at)) {
            Some(Leaf::Other) => continue,
            Some(leaf) => {
                if u64::try_from(paths.len()).unwrap_or(u64::MAX) >= max_paths {
                    return Err(paths_limit(max_paths));
                }
                let mass = if leaf == Leaf::Violating {
                    &mut violating
                } else {
                    &mut not_activated
                };
                *mass = arith.apply(filament_exact::RationalArithmetic::Add(
                    mass,
                    &node.probability,
                ))?;
                paths.push(node.trail);
                if refutes(bound, &violating, &not_activated, arith)? {
                    break;
                }
            }
            None => {
                for edge in &state.edges {
                    order += 1;
                    let chance = arith.apply(filament_exact::RationalArithmetic::Multiply(
                        &node.probability,
                        &edge.probability,
                    ))?;
                    let mut trail = node.trail.clone();
                    trail.push(edge.clone());
                    frontier.push(Frontier {
                        probability: chance,
                        order,
                        at: edge.to.clone(),
                        trail,
                    });
                }
            }
        }
    }
    if !refutes(bound, &violating, &not_activated, arith)? {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let paths = paths
        .iter()
        .map(|trail| path_of(system, root, trail))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ProbabilisticCounterexample {
        item: item.clone(),
        binding: root.over.as_ref().map(|(_, reference)| reference.clone()),
        instance: Some(root.reference.clone()),
        initial: root.start.clone(),
        initial_key: root.walk.root.clone(),
        scheduler: Vec::new(),
        paths,
        subsystem: None,
    })
}

fn path_of(
    system: &ModelSystem<'_>,
    root: &Root,
    trail: &[Edge],
) -> Result<ProbabilityWitnessPath, ProbabilityDocumentError> {
    let steps = trail
        .iter()
        .map(|edge| {
            let post = root
                .walk
                .models
                .get(&edge.to)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let schedule = crate::Schedule::of(system, &edge.action)
                .map_err(|_| ProbabilityDocumentError::Malformed)?;
            Ok(ProbabilityWitnessStep {
                choice: crate::ProductAction::Identity(schedule),
                action: Some(edge.action.clone()),
                transition: Some(edge.transition.clone()),
                draw: Some(edge.draw.clone()),
                post: post.clone(),
                post_key: edge.to.clone(),
                elapsed: rational(0),
            })
        })
        .collect::<Result<Vec<_>, ProbabilityDocumentError>>()?;
    Ok(ProbabilityWitnessPath { steps })
}

/// Reproduce a path-set refutation of one instance: rebuild the instance's
/// phase graph from the checked subject, walk every path through that graph,
/// compare each step's identity, transition, draw and post-state, classify its
/// end as violating or not activated, recompute `V` and `N` and apply FSpec
/// FR-413's bound. Nothing in `evidence` is trusted beyond naming the instance
/// and the paths.
///
/// # Errors
/// An unsupported subject, a refused expansion or predicate, the configured
/// limits or the supplied cancellation.
pub fn replay_on_each_witness(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    evidence: &ProbabilisticCounterexample,
    limits: ProbabilityDocumentLimits,
    max_paths: u64,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityDocumentError> {
    let subject = subject(system, claims, item, cancel)?;
    if &evidence.item != item || evidence.subsystem.is_some() {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    let Some(named) = &evidence.instance else {
        return Ok(ProbabilityWitnessReplay::Parity);
    };
    if u64::try_from(evidence.paths.len()).unwrap_or(u64::MAX) > max_paths {
        return Err(paths_limit(max_paths));
    }
    let found = roots(system, &subject, limits, cancel)?;
    let Some(root) = found.iter().find(|root| {
        &root.reference == named
            && root.walk.root == evidence.initial_key
            && root.over.as_ref().map(|(_, reference)| reference) == evidence.binding.as_ref()
            && system.state_key(&root.start).ok().map(|(model, _)| model)
                == system
                    .state_key(&evidence.initial)
                    .ok()
                    .map(|(model, _)| model)
    }) else {
        return Ok(ProbabilityWitnessReplay::Parity);
    };
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut arith = Arith {
        meter: &mut meter,
        cancel,
        limits,
    };
    let rebuilt = rebuilt::build(system, &subject, limits, cancel)?;
    let mut violating = rational(0);
    let mut not_activated = rational(0);
    let mut trails: BTreeSet<Vec<(Key, usize)>> = BTreeSet::new();
    for path in &evidence.paths {
        poll(cancel)?;
        let mut at = &root.walk.root;
        let mut probability = rational(1);
        let mut trail = Vec::new();
        for step in &path.steps {
            let Some(state) = root.walk.states.get(at) else {
                return Ok(ProbabilityWitnessReplay::Parity);
            };
            let Some((index, edge)) = state.edges.iter().enumerate().find(|(_, edge)| {
                edge.to == step.post_key
                    && step.action.as_ref() == Some(&edge.action)
                    && step.transition.as_ref() == Some(&edge.transition)
                    && step.draw.as_ref().is_some_and(|draw| {
                        fsl_eval::simulation::model::same_values(draw, &edge.draw)
                    })
                    && crate::Schedule::of(system, &edge.action).is_ok_and(|schedule| {
                        step.choice == crate::ProductAction::Identity(schedule)
                    })
                    && step.elapsed == rational(0)
            }) else {
                return Ok(ProbabilityWitnessReplay::Parity);
            };
            let named_state = system.state_key(&step.post).map(|(model, _)| model);
            let walked = root
                .walk
                .models
                .get(&edge.to)
                .map(|model| system.state_key(model).map(|(model, _)| model));
            if !matches!((named_state, walked), (Ok(left), Some(Ok(right))) if left == right) {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
            probability = arith.apply(filament_exact::RationalArithmetic::Multiply(
                &probability,
                &edge.probability,
            ))?;
            trail.push((edge.to.clone(), index));
            at = &edge.to;
        }
        // A not-activated path ends where the checker's own search, over the
        // graph it rebuilds, finds no activating state reachable.
        let activates = rebuilt::reaches_activation(&rebuilt, at, cancel)?;
        let leaf = root
            .walk
            .states
            .get(at)
            .and_then(|state| leaf_of(subject.bound, state, activates));
        let mass = match leaf {
            Some(Leaf::Violating) => &mut violating,
            Some(Leaf::NotActivated) => &mut not_activated,
            Some(Leaf::Other) | None => return Ok(ProbabilityWitnessReplay::Parity),
        };
        *mass = arith.apply(filament_exact::RationalArithmetic::Add(mass, &probability))?;
        if !trails.insert(trail) {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
    }
    // Sorted trails put every extension of a trail right after it, so a
    // trail that prefixes another is adjacent to one.
    let ordered: Vec<&Vec<(Key, usize)>> = trails.iter().collect();
    if ordered.windows(2).any(|pair| pair[1].starts_with(pair[0])) {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    if !refutes(subject.bound, &violating, &not_activated, &mut arith)? {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    Ok(ProbabilityWitnessReplay::Reproduced(Box::new(
        ReproducedProbabilityWitness::reproduced(evidence.clone(), violating),
    )))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireProfile {
    profile: String,
    optimization: String,
}
/// An `exact` part of the `ratio` certificate (FSpec FR-412).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WirePart {
    identity: WireIdentity,
    objective: WireProfile,
    kind: String,
    values: Vec<WireValue>,
    ranking: Vec<WireRank>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    policy: Option<Vec<WirePolicy>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WirePolicy {
    state: String,
    choice: Box<serde_json::value::RawValue>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireConditional {
    mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    numerator: Option<WirePart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    denominator: Option<WirePart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    post: Option<WirePart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reset: Option<WirePart>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireOnEach {
    identity: WireIdentity,
    objective: WireProfile,
    kind: String,
    /// Present, and empty, only in a `vacuous` document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    values: Option<Vec<WireValue>>,
    conditional: Option<WireConditional>,
}

/// The rows of the `ratio` parts and their shared ranks.
struct Parts<'a> {
    numerator: &'a [ExactProbabilityValue],
    denominator: &'a [ExactProbabilityValue],
    ranking: &'a [(Key, u64)],
}

struct Events<'a> {
    identity: &'a ObligationIdentity,
    /// The `ratio` parts, or `None` for the `vacuous` mode.
    parts: Option<Parts<'a>>,
    optimization: &'a str,
}

impl Events<'_> {
    fn head<S: quire_canonical::Sink + ?Sized>(
        &self,
        kind: &str,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.name("identity")?;
        writer.begin_object()?;
        writer.name("node_id")?;
        fsl_foundation::node_wire::encode_node_key(self.identity.node(), writer)?;
        writer.name("model")?;
        match self.identity.model() {
            Some(model) => writer.string(model.as_str())?,
            None => writer.null()?,
        }
        writer.name("ordinal")?;
        writer.integer(i128::from(self.identity.ordinal()))?;
        writer.end_object()?;
        writer.name("objective")?;
        writer.begin_object()?;
        writer.name("profile")?;
        writer.string(PROFILE)?;
        writer.name("optimization")?;
        writer.string(self.optimization)?;
        writer.end_object()?;
        writer.name("kind")?;
        writer.string(kind)
    }

    fn part<S: quire_canonical::Sink + ?Sized>(
        &self,
        kind: &str,
        rows: &[ExactProbabilityValue],
        ranking: &[(Key, u64)],
        policy: Option<&[(Key, serde_json::Value)]>,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_object()?;
        self.head(kind, writer)?;
        writer.name("values")?;
        writer.begin_array()?;
        for row in rows {
            writer.begin_object()?;
            writer.name("state")?;
            writer.string(&URL_SAFE_NO_PAD.encode(row.state.as_bytes()))?;
            writer.name("value")?;
            writer.begin_object()?;
            writer.name("numerator")?;
            writer.string(&row.value.numerator().to_string())?;
            writer.name("denominator")?;
            writer.string(&row.value.denominator().to_string())?;
            writer.end_object()?;
            writer.end_object()?;
        }
        writer.end_array()?;
        if let Some(policy) = policy {
            writer.name("policy")?;
            writer.begin_array()?;
            for (state, choice) in policy {
                writer.begin_object()?;
                writer.name("state")?;
                writer.string(&URL_SAFE_NO_PAD.encode(state.as_bytes()))?;
                writer.name("choice")?;
                choice.encode_into(writer)?;
                writer.end_object()?;
            }
            writer.end_array()?;
        }
        writer.name("ranking")?;
        writer.begin_array()?;
        for (state, rank) in ranking {
            writer.begin_object()?;
            writer.name("rank")?;
            writer.integer(i128::from(*rank))?;
            writer.name("state")?;
            writer.string(&URL_SAFE_NO_PAD.encode(state.as_bytes()))?;
            writer.end_object()?;
        }
        writer.end_array()?;
        writer.end_object()
    }
}

impl quire_canonical::Encode for Events<'_> {
    fn encode_into<S: quire_canonical::Sink + ?Sized>(
        &self,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_object()?;
        self.head("conditional", writer)?;
        if self.parts.is_none() {
            writer.name("values")?;
            writer.begin_array()?;
            writer.end_array()?;
        }
        writer.name("conditional")?;
        writer.begin_object()?;
        writer.name("mode")?;
        match &self.parts {
            Some(parts) => {
                writer.string("ratio")?;
                writer.name("numerator")?;
                self.part("exact", parts.numerator, parts.ranking, None, writer)?;
                writer.name("denominator")?;
                self.part("exact", parts.denominator, parts.ranking, None, writer)?;
            }
            None => writer.string("vacuous")?,
        }
        writer.end_object()?;
        writer.end_object()
    }
}

fn encode(
    identity: &ObligationIdentity,
    parts: Option<Parts<'_>>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let admission = RefCell::new(writer::Admission::with_used(
        limits.storage_bytes,
        cancel,
        0,
    ));
    let events = Events {
        identity,
        parts,
        optimization: "workload",
    };
    let result = writer::encode_reserved(&events, limits.bytes, &admission);
    if let Some(error) = admission.borrow_mut().take_stop() {
        return Err(error);
    }
    result
}

struct OptimumEvents<'a> {
    identity: &'a ObligationIdentity,
    optimization: &'a str,
    post: &'a [ExactProbabilityValue],
    post_ranks: &'a [(Key, u64)],
    reset: &'a [ExactProbabilityValue],
    reset_ranks: &'a [(Key, u64)],
    reset_policy: &'a [(Key, serde_json::Value)],
}

impl OptimumEvents<'_> {
    fn head<S: quire_canonical::Sink + ?Sized>(
        &self,
        kind: &str,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.name("identity")?;
        writer.begin_object()?;
        writer.name("node_id")?;
        fsl_foundation::node_wire::encode_node_key(self.identity.node(), writer)?;
        writer.name("model")?;
        match self.identity.model() {
            Some(model) => writer.string(model.as_str())?,
            None => writer.null()?,
        }
        writer.name("ordinal")?;
        writer.integer(i128::from(self.identity.ordinal()))?;
        writer.end_object()?;
        writer.name("objective")?;
        writer.begin_object()?;
        writer.name("profile")?;
        writer.string(PROFILE)?;
        writer.name("optimization")?;
        writer.string(self.optimization)?;
        writer.end_object()?;
        writer.name("kind")?;
        writer.string(kind)
    }

    fn part<S: quire_canonical::Sink + ?Sized>(
        &self,
        kind: &str,
        rows: &[ExactProbabilityValue],
        ranking: &[(Key, u64)],
        policy: Option<&[(Key, serde_json::Value)]>,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_object()?;
        self.head(kind, writer)?;
        writer.name("values")?;
        writer.begin_array()?;
        for row in rows {
            writer.begin_object()?;
            writer.name("state")?;
            writer.string(&URL_SAFE_NO_PAD.encode(row.state.as_bytes()))?;
            writer.name("value")?;
            writer.begin_object()?;
            writer.name("numerator")?;
            writer.string(&row.value.numerator().to_string())?;
            writer.name("denominator")?;
            writer.string(&row.value.denominator().to_string())?;
            writer.end_object()?;
            writer.end_object()?;
        }
        writer.end_array()?;
        if let Some(policy) = policy {
            writer.name("policy")?;
            writer.begin_array()?;
            for (state, choice) in policy {
                writer.begin_object()?;
                writer.name("state")?;
                writer.string(&URL_SAFE_NO_PAD.encode(state.as_bytes()))?;
                writer.name("choice")?;
                choice.encode_into(writer)?;
                writer.end_object()?;
            }
            writer.end_array()?;
        }
        writer.name("ranking")?;
        writer.begin_array()?;
        for (state, rank) in ranking {
            writer.begin_object()?;
            writer.name("rank")?;
            writer.integer(i128::from(*rank))?;
            writer.name("state")?;
            writer.string(&URL_SAFE_NO_PAD.encode(state.as_bytes()))?;
            writer.end_object()?;
        }
        writer.end_array()?;
        writer.end_object()
    }
}

impl quire_canonical::Encode for OptimumEvents<'_> {
    fn encode_into<S: quire_canonical::Sink + ?Sized>(
        &self,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_object()?;
        self.head("conditional", writer)?;
        writer.name("conditional")?;
        writer.begin_object()?;
        writer.name("mode")?;
        writer.string("optimum")?;
        writer.name("post")?;
        self.part("exact", self.post, self.post_ranks, None, writer)?;
        writer.name("reset")?;
        let reset_kind = if matches!(self.optimization, "minimum" | "workload-minimum") {
            "lower"
        } else {
            "upper"
        };
        self.part(
            reset_kind,
            self.reset,
            self.reset_ranks,
            Some(self.reset_policy),
            writer,
        )?;
        writer.end_object()?;
        writer.end_object()
    }
}

fn encode_optimum(
    identity: &ObligationIdentity,
    optimization: &str,
    post: &[ExactProbabilityValue],
    post_ranks: &[(Key, u64)],
    reset: &[ExactProbabilityValue],
    reset_ranks: &[(Key, u64)],
    reset_policy: &[(Key, serde_json::Value)],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let admission = RefCell::new(writer::Admission::with_used(
        limits.storage_bytes,
        cancel,
        0,
    ));
    let events = OptimumEvents {
        identity,
        optimization,
        post,
        post_ranks,
        reset,
        reset_ranks,
        reset_policy,
    };
    let result = writer::encode_reserved(&events, limits.bytes, &admission);
    if let Some(error) = admission.borrow_mut().take_stop() {
        return Err(error);
    }
    result
}

/// One decoded `exact` part: its value rows and rank rows, in the order
/// written.
struct PartRows {
    values: Vec<ExactProbabilityValue>,
    ranks: Vec<(Key, u64)>,
    policy: Vec<(Key, serde_json::Value)>,
}

enum Mode {
    Ratio {
        numerator: PartRows,
        denominator: PartRows,
    },
    Vacuous,
    Optimum {
        post: PartRows,
        reset: PartRows,
    },
}

struct Decoded {
    identity: ObligationIdentity,
    mode: Mode,
}

fn identity_of(wire: WireIdentity) -> Result<ObligationIdentity, ProbabilityDocumentError> {
    let node = fsl_foundation::node_wire::node_key_from_text(wire.node_id.get())
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    Ok(ObligationIdentity::new(
        node,
        wire.model.map(crate::ModelIdentity::new),
        wire.ordinal,
    ))
}

fn member_error(
    error: ProbabilityDocumentError,
    member: &'static str,
) -> ProbabilityDocumentError {
    match error {
        ProbabilityDocumentError::Malformed
        | ProbabilityDocumentError::Noncanonical
        | ProbabilityDocumentError::Unsupported => {
            ProbabilityDocumentError::MalformedMember(member)
        }
        other => other,
    }
}

fn decode(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<Decoded, ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    let wire: WireOnEach =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if canonical(&wire, limits.bytes)? != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    if wire.objective.profile != PROFILE {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    if wire.kind != "conditional" {
        return Err(ProbabilityDocumentError::MalformedMember("kind"));
    }
    let Some(conditional) = wire.conditional else {
        return Err(ProbabilityDocumentError::MalformedMember("conditional"));
    };
    let workload = wire.objective.optimization == "workload";
    // A `ratio` document has no top-level `values`; a `vacuous` one has them
    // empty (FR-201).
    let values_fit = match conditional.mode.as_str() {
        "ratio" => wire.values.is_none(),
        "vacuous" => wire.values.as_ref().is_some_and(Vec::is_empty),
        "optimum" => wire.values.is_none(),
        _ => true,
    };
    if !values_fit {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    }
    let identity = identity_of(wire.identity)?;
    let optimization = wire.objective.optimization.as_str();
    let mode = match conditional.mode.as_str() {
        "ratio" if workload && conditional.post.is_none() && conditional.reset.is_none() => {
            let mut part = |name: &'static str, part: Option<WirePart>| {
                let part = part.ok_or(ProbabilityDocumentError::MalformedMember(name))?;
                if part.kind != "exact"
                    || part.objective.profile != PROFILE
                    || part.objective.optimization != "workload"
                    || identity_of(part.identity)? != identity
                    || part.policy.is_some()
                {
                    return Err(ProbabilityDocumentError::MalformedMember(name));
                }
                let mut values = Vec::with_capacity(part.values.len());
                for row in part.values {
                    poll(cancel)?;
                    values.push(ExactProbabilityValue {
                        state: decode_key_with(&row.state, limits, true)
                            .map_err(|error| member_error(error, "values"))?,
                        value: ratio(row.value, limits.scalar_bits, meter, cancel)
                            .map_err(|error| member_error(error, "values"))?,
                    });
                }
                ordered(values.iter().map(|row| row.state.clone()))
                    .map_err(|_| ProbabilityDocumentError::MalformedMember("values"))?;
                let mut ranks = Vec::with_capacity(part.ranking.len());
                for row in part.ranking {
                    poll(cancel)?;
                    if row.rank > MAX_COUNT {
                        return Err(ProbabilityDocumentError::MalformedMember("ranking"));
                    }
                    ranks.push((
                        decode_key_with(&row.state, limits, true)
                            .map_err(|error| member_error(error, "ranking"))?,
                        row.rank,
                    ));
                }
                ordered(ranks.iter().map(|(state, _)| state.clone()))
                    .map_err(|_| ProbabilityDocumentError::MalformedMember("ranking"))?;
                Ok::<_, ProbabilityDocumentError>(PartRows {
                    values,
                    ranks,
                    policy: Vec::new(),
                })
            };
            let numerator = part("numerator", conditional.numerator)?;
            let denominator = part("denominator", conditional.denominator)?;
            Mode::Ratio {
                numerator,
                denominator,
            }
        }
        "vacuous"
            if workload
                && conditional.numerator.is_none()
                && conditional.denominator.is_none()
                && conditional.post.is_none()
                && conditional.reset.is_none() =>
        {
            Mode::Vacuous
        }
        "optimum"
            if matches!(
                optimization,
                "minimum" | "maximum" | "workload-minimum" | "workload-maximum"
            ) && conditional.numerator.is_none()
                && conditional.denominator.is_none() =>
        {
            let mut decode_part = |name: &'static str,
                                   part: Option<WirePart>,
                                   expected_kind: &'static str|
             -> Result<PartRows, ProbabilityDocumentError> {
                let part = part.ok_or(ProbabilityDocumentError::MalformedMember(name))?;
                if part.kind != expected_kind
                    || part.objective.profile != PROFILE
                    || part.objective.optimization != optimization
                    || identity_of(part.identity)? != identity
                    || (expected_kind == "exact" && part.policy.is_some())
                    || (expected_kind != "exact" && part.policy.is_none())
                {
                    return Err(ProbabilityDocumentError::MalformedMember(name));
                }
                let mut values = Vec::with_capacity(part.values.len());
                for row in part.values {
                    poll(cancel)?;
                    values.push(ExactProbabilityValue {
                        state: decode_key_with(&row.state, limits, true)
                            .map_err(|error| member_error(error, "values"))?,
                        value: ratio(row.value, limits.scalar_bits, meter, cancel)
                            .map_err(|error| member_error(error, "values"))?,
                    });
                }
                ordered(values.iter().map(|row| row.state.clone()))
                    .map_err(|_| ProbabilityDocumentError::MalformedMember("values"))?;
                let mut ranks = Vec::with_capacity(part.ranking.len());
                for row in part.ranking {
                    poll(cancel)?;
                    if row.rank > MAX_COUNT {
                        return Err(ProbabilityDocumentError::MalformedMember("ranking"));
                    }
                    ranks.push((
                        decode_key_with(&row.state, limits, true)
                            .map_err(|error| member_error(error, "ranking"))?,
                        row.rank,
                    ));
                }
                ordered(ranks.iter().map(|(state, _)| state.clone()))
                    .map_err(|_| ProbabilityDocumentError::MalformedMember("ranking"))?;
                let mut policy = Vec::new();
                for row in part.policy.unwrap_or_default() {
                    poll(cancel)?;
                    let state = decode_key_with(&row.state, limits, true)
                        .map_err(|error| member_error(error, "policy"))?;
                    let choice: serde_json::Value = serde_json::from_str(row.choice.get())
                        .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
                    let bytes = serde_json::to_vec(&choice)
                        .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
                    crate::ProductAction::parse(&bytes)
                        .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
                    policy.push((state, choice));
                }
                ordered(policy.iter().map(|(state, _)| state.clone()))
                    .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
                Ok(PartRows {
                    values,
                    ranks,
                    policy,
                })
            };
            let post = decode_part("post", conditional.post, "exact")?;
            let reset_kind = if matches!(optimization, "minimum" | "workload-minimum") {
                "lower"
            } else {
                "upper"
            };
            let reset = decode_part("reset", conditional.reset, reset_kind)?;
            Mode::Optimum { post, reset }
        }
        _ => return Err(ProbabilityDocumentError::MalformedMember("mode")),
    };
    Ok(Decoded { identity, mode })
}

pub(super) fn identity(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ObligationIdentity, ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    Ok(decode(bytes, limits, cancel, &mut meter)?.identity)
}

/// The checker's own phase graph, rebuilt from the checked subject. Its
/// construction, cycle test and activation search share no traversal or
/// ordering code with the producer above. What is shared is the model's step
/// relation, the predicate reader, the key encoder and the row semantics
/// (`Arith::expected`, `conditioned` and `meets`), which state the
/// part equations, the ratio and the threshold test once. The witness replay
/// classifies a not-activated path with this graph's search and reads each
/// step's action, transition and draw from the producer's walk, which is
/// where the step relation is evaluated.
mod rebuilt {
    use super::*;

    pub(super) struct Node {
        pub(super) active: bool,
        pub(super) leaf: Option<bool>,
        pub(super) next: Vec<(Key, Rational)>,
        pub(super) choices: Vec<Vec<(Key, Rational)>>,
        pub(super) choice_ids: Vec<Option<ScheduledIdentity>>,
    }

    pub(super) struct RootRef {
        pub(super) initial: u32,
        pub(super) key: Key,
        pub(super) binding: Option<crate::ExactBinding>,
        pub(super) instance: Option<crate::ExactBinding>,
    }

    pub(super) struct Graph {
        pub(super) nodes: BTreeMap<Key, Node>,
        pub(super) roots: Vec<RootRef>,
    }

    pub(super) fn build(
        system: &ModelSystem<'_>,
        subject: &Subject<'_>,
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
    ) -> Result<Graph, ProbabilityDocumentError> {
        build_with_root_index(system, subject, limits, cancel, false)
    }

    pub(super) fn build_with_root_index(
        system: &ModelSystem<'_>,
        subject: &Subject<'_>,
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
        root_indexed: bool,
    ) -> Result<Graph, ProbabilityDocumentError> {
        let claim = subject.claim;
        let overs: Vec<Option<(ModelInstance, ObjectReference)>> = match &claim.over {
            None => vec![None],
            Some(over) => instances_of(system, &over.value_type)?
                .into_iter()
                .map(Some)
                .collect(),
        };
        let instances = instances_of(system, subject.instance_type)?;
        let mut spent = Spent::default();
        let mut graph = Graph {
            nodes: BTreeMap::new(),
            roots: Vec::new(),
        };
        for (position, start) in canonical_initials(system, limits, cancel)?.into_iter().enumerate() {
            let initial =
                u32::try_from(position).map_err(|_| ProbabilityDocumentError::Malformed)?;
            for over in &overs {
                let over_ref = over.as_ref().map(|(_, reference)| reference);
                for (instance, reference) in &instances {
                    let key = explore(
                        system,
                        subject,
                        &start,
                        over_ref,
                        reference,
                        limits,
                        &mut spent,
                        &mut graph.nodes,
                        root_indexed.then_some(initial as usize),
                        cancel,
                    )?;
                    let (binding, bound_instance) = entry_binding(over.as_ref(), instance);
                    graph.roots.push(RootRef {
                        initial,
                        key,
                        binding,
                        instance: Some(bound_instance),
                    });
                }
            }
        }
        Ok(graph)
    }

    /// Depth-first from `start`, adding every reachable node; the root's key.
    #[allow(clippy::too_many_arguments)]
    fn explore(
        system: &ModelSystem<'_>,
        subject: &Subject<'_>,
        start: &ModelSimState,
        over: Option<&ObjectReference>,
        instance: &ObjectReference,
        limits: ProbabilityDocumentLimits,
        spent: &mut Spent,
        nodes: &mut BTreeMap<Key, Node>,
        root_index: Option<usize>,
        cancel: &Cancel,
    ) -> Result<Key, ProbabilityDocumentError> {
        let guard_holds = |state: &ModelSimState| match &subject.guard {
            Some(guard) => holds(system, state, guard, over, instance, cancel),
            None => Ok(true),
        };
        let first = guard_holds(start)?;
        let root = phase_key_with_root(
            system,
            over,
            instance,
            start,
            phase_of(first),
            root_index,
            limits,
            cancel,
        )?;
        let mut visited: BTreeSet<Key> = BTreeSet::from([root.clone()]);
        spent.state(&root, limits)?;
        let mut stack = vec![(root.clone(), start.clone(), first)];
        while let Some((key, state, active)) = stack.pop() {
            poll(cancel)?;
            if nodes.contains_key(&key) {
                continue;
            }
            if active && holds(system, &state, subject.target, over, instance, cancel)? {
                nodes.insert(
                    key,
                    Node {
                        active,
                        leaf: Some(true),
                        next: Vec::new(),
                        choices: Vec::new(),
                        choice_ids: Vec::new(),
                    },
                );
                continue;
            }
            let steps = moves(system, &state, subject.workload, cancel)?;
            let mut next = Vec::with_capacity(steps.len());
            let mut choices: Vec<(Option<ScheduledIdentity>, Vec<(Key, Rational)>)> = Vec::new();
            for step in steps {
                poll(cancel)?;
                let next_active = active || guard_holds(&step.post)?;
                let to = phase_key_with_root(
                    system,
                    over,
                    instance,
                    &step.post,
                    phase_of(next_active),
                    root_index,
                    limits,
                    cancel,
                )?;
                spent.edge(limits)?;
                if visited.insert(to.clone()) {
                    spent.state(&to, limits)?;
                    stack.push((to.clone(), step.post, next_active));
                }
                next.push((to.clone(), step.probability.clone()));
                let action = if subject.workload.is_some() {
                    None
                } else {
                    step.identity.clone()
                };
                let choice = choices.iter_mut().find(|(known, _)| *known == action);
                if let Some((_, edges)) = choice {
                    edges.push((to.clone(), step.probability.clone()));
                } else {
                    choices.push((action, vec![(to, step.probability)]));
                }
            }
            let leaf = next.is_empty().then_some(false);
            nodes.insert(
                key,
                Node {
                    active,
                    leaf,
                    next,
                    choice_ids: choices
                        .iter()
                        .map(|(identity, _)| identity.clone())
                        .collect(),
                    choices: choices.into_iter().map(|(_, edges)| edges).collect(),
                },
            );
        }
        Ok(root)
    }

    /// Whether the graph has a cycle, by repeatedly removing nodes with no
    /// remaining successor.
    pub(super) fn has_cycle(
        graph: &Graph,
        cancel: &Cancel,
    ) -> Result<bool, ProbabilityDocumentError> {
        let mut waiting: BTreeMap<&Key, usize> = graph
            .nodes
            .iter()
            .map(|(key, node)| (key, node.next.len()))
            .collect();
        let mut predecessors: BTreeMap<&Key, Vec<&Key>> = BTreeMap::new();
        for (key, node) in &graph.nodes {
            for (to, _) in &node.next {
                predecessors.entry(to).or_default().push(key);
            }
        }
        let mut ready: Vec<&Key> = waiting
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(key, _)| *key)
            .collect();
        let mut removed = 0_usize;
        while let Some(key) = ready.pop() {
            poll(cancel)?;
            removed += 1;
            for before in predecessors.get(key).into_iter().flatten() {
                if let Some(count) = waiting.get_mut(before) {
                    *count -= 1;
                    if *count == 0 {
                        ready.push(before);
                    }
                }
            }
        }
        Ok(removed != graph.nodes.len())
    }

    /// Whether an active node is reachable from `root`.
    pub(super) fn reaches_activation(
        graph: &Graph,
        root: &Key,
        cancel: &Cancel,
    ) -> Result<bool, ProbabilityDocumentError> {
        let mut seen: BTreeSet<&Key> = BTreeSet::from([root]);
        let mut stack = vec![root];
        while let Some(key) = stack.pop() {
            poll(cancel)?;
            let node = graph
                .nodes
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            if node.active {
                return Ok(true);
            }
            for (to, _) in &node.next {
                if seen.insert(to) {
                    stack.push(to);
                }
            }
        }
        Ok(false)
    }
}

fn reject(
    rule: ProbabilityCertificateRule,
    at: ProbabilityCertificateLocus,
) -> ExactProbabilityCertificateCheck {
    ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection { rule, at })
}

fn part_name(part: Part) -> &'static str {
    match part {
        Part::Numerator => "numerator",
        Part::Denominator => "denominator",
    }
}

fn part_locus(key: &Key, part: Part) -> ProbabilityCertificateLocus {
    ProbabilityCertificateLocus::ProductPart {
        key: key.clone(),
        part: match part {
            Part::Numerator => ProbabilityCertificatePart::Numerator,
            Part::Denominator => ProbabilityCertificatePart::Denominator,
        },
    }
}

fn check_cyclic_post_values(
    graph: &rebuilt::Graph,
    values: &BTreeMap<Key, Rational>,
    minimum: bool,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<Option<ProbabilityCertificateRejection>, ProbabilityDocumentError> {
    let active = graph
        .nodes
        .iter()
        .filter_map(|(key, node)| node.active.then_some(key.clone()))
        .collect::<BTreeSet<_>>();
    let mut targets = BTreeSet::new();
    for key in &active {
        poll(cancel)?;
        charge_post_check(meter, cancel)?;
        if graph.nodes.get(key).is_some_and(|node| node.leaf == Some(true)) {
            targets.insert(key.clone());
        }
    }
    let mut zero = BTreeSet::new();
    if minimum {
        zero.extend(active.difference(&targets).cloned());
        loop {
            poll(cancel)?;
            let mut removed = Vec::new();
            for key in &zero {
                poll(cancel)?;
                charge_post_check(meter, cancel)?;
                let node = graph.nodes.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
                if node.leaf == Some(false) {
                    continue;
                }
                let remains_zero = node.choices.iter().any(|choice| {
                    choice
                        .iter()
                        .all(|(to, probability)| probability.is_zero() || zero.contains(to))
                });
                if !remains_zero {
                    removed.push(key.clone());
                }
            }
            if removed.is_empty() {
                break;
            }
            for key in removed {
                zero.remove(&key);
            }
        }
    } else {
        let mut can_reach = targets.clone();
        loop {
            poll(cancel)?;
            let mut added = Vec::new();
            for key in &active {
                poll(cancel)?;
                charge_post_check(meter, cancel)?;
                if can_reach.contains(key) {
                    continue;
                }
                let node = graph.nodes.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
                if node.choices.iter().any(|choice| {
                    choice
                        .iter()
                        .any(|(to, probability)| !probability.is_zero() && can_reach.contains(to))
                }) {
                    added.push(key.clone());
                }
            }
            if added.is_empty() {
                break;
            }
            can_reach.extend(added);
        }
        zero.extend(active.difference(&can_reach).cloned());
    }
    for key in &active {
        poll(cancel)?;
        charge_post_check(meter, cancel)?;
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let node = graph.nodes.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let valid = if node.leaf == Some(true) {
            value == &rational(1)
        } else if zero.contains(key) {
            value.is_zero()
        } else {
            !value.is_zero()
        };
        if !valid {
            return Ok(Some(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::InequalityFails,
                at: ProbabilityCertificateLocus::ProductPart {
                    key: key.clone(),
                    part: ProbabilityCertificatePart::Post,
                },
            }));
        }
    }
    Ok(None)
}

fn charge_post_check(
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    meter
        .charge(
            filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                .work(filament_exact::Integer::from(1_u64)),
        )
        .map_err(|error| match cancel.tripped() {
            Some(cause) => ProbabilityDocumentError::Cancelled(cause),
            None => ProbabilityDocumentError::Accounting(Box::new(error)),
        })
}

fn check_optimum_document(
    system: &ModelSystem<'_>,
    subject: &Subject<'_>,
    graph: &rebuilt::Graph,
    mode: Mode,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let Mode::Optimum { post, reset } = mode else {
        return Err(ProbabilityDocumentError::MalformedMember("mode"));
    };
    let post_values = post
        .values
        .iter()
        .map(|row| (row.state.clone(), row.value.clone()))
        .collect::<BTreeMap<_, _>>();
    let reset_values = reset
        .values
        .iter()
        .map(|row| (row.state.clone(), row.value.clone()))
        .collect::<BTreeMap<_, _>>();
    let post_ranks = post.ranks.iter().cloned().collect::<BTreeMap<_, _>>();
    let reset_ranks = reset.ranks.iter().cloned().collect::<BTreeMap<_, _>>();
    let policy = reset.policy.iter().cloned().collect::<BTreeMap<_, _>>();
    let active = graph
        .nodes
        .iter()
        .filter_map(|(key, node)| node.active.then_some(key.clone()))
        .collect::<BTreeSet<_>>();
    let waiting = graph
        .nodes
        .iter()
        .filter_map(|(key, node)| (!node.active).then_some(key.clone()))
        .collect::<BTreeSet<_>>();
    fn same_keys<T>(rows: &BTreeMap<Key, T>, expected: &BTreeSet<Key>) -> bool {
        rows.len() == expected.len() && rows.keys().eq(expected.iter())
    }
    if !same_keys(&post_values, &active) || !same_keys(&post_ranks, &active) {
        return Ok(reject(
            ProbabilityCertificateRule::Malformed,
            ProbabilityCertificateLocus::Member("post"),
        ));
    }
    if !same_keys(&reset_values, &waiting) {
        return Ok(reject(
            ProbabilityCertificateRule::Malformed,
            ProbabilityCertificateLocus::Member("values"),
        ));
    }
    if !same_keys(&reset_ranks, &waiting) {
        return Ok(reject(
            ProbabilityCertificateRule::Malformed,
            ProbabilityCertificateLocus::Member("ranking"),
        ));
    }
    if !same_keys(&policy, &waiting) {
        return Ok(reject(
            ProbabilityCertificateRule::Malformed,
            ProbabilityCertificateLocus::Member("policy"),
        ));
    }

    let minimum = matches!(subject.bound, Bound::AtLeast(_));
    // The independent cyclic Exact pass rejects arbitrary positive fixed
    // points in active end components before reset values are read.
    if let Some(rejection) =
        check_cyclic_post_values(graph, &post_values, minimum, meter, cancel)?
    {
        return Ok(ExactProbabilityCertificateCheck::Rejected(rejection));
    }
    let mut arith = Arith {
        meter,
        cancel,
        limits,
    };

    // Re-derive post's Bellman value at every active state, independently of
    // the producer. Active terminal states are fixed by the target predicate.
    for (key, node) in graph.nodes.iter().filter(|(_, node)| node.active) {
        poll(cancel)?;
        let offered = post_values
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let expected = match node.leaf {
            Some(true) => rational(1),
            Some(false) => rational(0),
            None => {
                let mut candidates = Vec::new();
                for choice in &node.choices {
                    candidates.push(expectation(choice, &post_values, &mut arith)?);
                }
                choose_extreme(candidates, minimum)?
            }
        };
        if &expected != offered {
            return Ok(reject(
                ProbabilityCertificateRule::InequalityFails,
                ProbabilityCertificateLocus::ProductPart {
                    key: key.clone(),
                    part: ProbabilityCertificatePart::Post,
                },
            ));
        }
    }
    for (key, node) in graph.nodes.iter().filter(|(_, node)| node.active) {
        poll(cancel)?;
        let value = post_values
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let rank = post_ranks
            .get(key)
            .copied()
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let is_target = node.leaf == Some(true);
        if is_target || value.is_zero() {
            if rank != 0 {
                return Ok(reject(
                    ProbabilityCertificateRule::RankBroken,
                    ProbabilityCertificateLocus::ProductPart {
                        key: key.clone(),
                        part: ProbabilityCertificatePart::Post,
                    },
                ));
            }
            continue;
        }
        if rank == 0 {
            return Ok(reject(
                ProbabilityCertificateRule::RankBroken,
                ProbabilityCertificateLocus::ProductPart {
                    key: key.clone(),
                    part: ProbabilityCertificatePart::Post,
                },
            ));
        }
        let progresses = |choice: &Vec<(Key, Rational)>| {
            choice.iter().any(|(next, probability)| {
                !probability.is_zero()
                    && post_values
                        .get(next)
                        .is_some_and(|next_value| !next_value.is_zero())
                    && post_ranks
                        .get(next)
                        .is_some_and(|next_rank| *next_rank < rank)
            })
        };
        let progress = if minimum {
            !node.choices.is_empty() && node.choices.iter().all(progresses)
        } else {
            let mut found = false;
            for choice in &node.choices {
                if expectation(choice, &post_values, &mut arith)? == *value && progresses(choice) {
                    found = true;
                    break;
                }
            }
            found
        };
        if !progress {
            return Ok(reject(
                ProbabilityCertificateRule::RankBroken,
                ProbabilityCertificateLocus::ProductPart {
                    key: key.clone(),
                    part: ProbabilityCertificatePart::Post,
                },
            ));
        }
    }

    // Assign every reachable waiting key to its independently discovered
    // initial key. The reset action always returns to that key.
    let mut owners: BTreeMap<Key, BTreeSet<Key>> = BTreeMap::new();
    let mut root_actions: BTreeMap<Key, BTreeMap<Key, Vec<(Vec<(Key, Rational)>, bool)>>> =
        BTreeMap::new();
    for root in &graph.roots {
        let mut reached = BTreeSet::from([root.key.clone()]);
        let mut stack = vec![root.key.clone()];
        while let Some(key) = stack.pop() {
            poll(cancel)?;
            for (next, _) in graph
                .nodes
                .get(&key)
                .ok_or(ProbabilityDocumentError::Malformed)?
                .next
                .iter()
            {
                if reached.insert(next.clone()) {
                    stack.push(next.clone());
                }
            }
        }
        let partition = reached
            .iter()
            .filter(|key| graph.nodes.get(*key).is_some_and(|node| !node.active))
            .cloned()
            .collect::<BTreeSet<_>>();
        let partition_choices = partition
            .iter()
            .map(|key| {
                graph
                    .nodes
                    .get(key)
                    .map(|node| (key.clone(), node.choices.clone()))
                    .ok_or(ProbabilityDocumentError::Malformed)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let reset_eligible =
            waiting_end_component_members(&partition, &partition_choices, arith.meter, cancel)?;
        for key in &partition {
            owners
                .entry(key.clone())
                .or_default()
                .insert(root.key.clone());
            let node = graph
                .nodes
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let actions = if !rebuilt::reaches_activation(graph, key, cancel)? {
                vec![(vec![(root.key.clone(), rational(1))], true)]
            } else {
                let mut actions = node
                    .choices
                    .iter()
                    .cloned()
                    .map(|choice| (choice, false))
                    .collect::<Vec<_>>();
                if reset_eligible.contains(key) {
                    actions.push((vec![(root.key.clone(), rational(1))], true));
                }
                actions
            };
            root_actions
                .entry(key.clone())
                .or_default()
                .insert(root.key.clone(), actions);
        }
    }

    // Every action, including the synthetic reset at a waiting end component,
    // must respect the proper optimum's submartingale/supermartingale bound.
    for key in &waiting {
        let value = reset_values
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let owners = owners.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        for owner in owners {
            let actions = root_actions
                .get(key)
                .and_then(|by_owner| by_owner.get(owner))
                .ok_or(ProbabilityDocumentError::Malformed)?;
            if actions.is_empty() {
                return Err(ProbabilityDocumentError::Malformed);
            }
            for (action, _) in actions {
                let expectation =
                    expectation_lookup(action, &post_values, &reset_values, graph, &mut arith)?;
                let respects = if minimum {
                    value <= &expectation
                } else {
                    &expectation <= value
                };
                if !respects {
                    return Ok(reject(
                        ProbabilityCertificateRule::InequalityFails,
                        ProbabilityCertificateLocus::ProductPart {
                            key: key.clone(),
                            part: ProbabilityCertificatePart::Reset,
                        },
                    ));
                }
            }
        }
    }

    // The selected deterministic policy must satisfy its equations and make
    // progress to an activating terminal copy under the offered rank.
    for key in &waiting {
        let choice_value = policy.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let choice_bytes = serde_json::to_vec(choice_value)
            .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
        let action = crate::ProductAction::parse(&choice_bytes)
            .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
        let node = graph
            .nodes
            .get(key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let selected_choice = match action {
            crate::ProductAction::Reset => None,
            crate::ProductAction::Identity(schedule) => Some(
                node.choice_ids
                    .iter()
                    .enumerate()
                    .find_map(|(index, identity)| {
                        let identity = identity.as_ref()?;
                        let known = crate::Schedule::of(system, identity).ok()?;
                        (known == schedule).then_some(index)
                    })
                    .ok_or(ProbabilityDocumentError::MalformedMember("policy"))?,
            ),
            _ => return Err(ProbabilityDocumentError::MalformedMember("policy")),
        };
        let owners = owners.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        for owner in owners {
            let actions = root_actions
                .get(key)
                .and_then(|by_owner| by_owner.get(owner))
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let (selected_edges, _) = match selected_choice {
                Some(index) => actions
                    .get(index)
                    .filter(|(_, is_reset)| !is_reset)
                    .ok_or(ProbabilityDocumentError::MalformedMember("policy"))?,
                None => actions
                    .iter()
                    .find(|(_, is_reset)| *is_reset)
                    .ok_or(ProbabilityDocumentError::MalformedMember("policy"))?,
            };
            let expected = expectation_lookup(
                selected_edges,
                &post_values,
                &reset_values,
                graph,
                &mut arith,
            )?;
            if reset_values.get(key) != Some(&expected) {
                return Ok(reject(
                    ProbabilityCertificateRule::InequalityFails,
                    ProbabilityCertificateLocus::ProductPart {
                        key: key.clone(),
                        part: ProbabilityCertificatePart::Reset,
                    },
                ));
            }
            let rank = reset_ranks.get(key).copied().unwrap_or(0);
            let progresses = selected_edges.iter().any(|(to, probability)| {
                !probability.is_zero()
                    && if graph.nodes.get(to).is_some_and(|node| node.active) {
                        0 < rank
                    } else {
                        reset_ranks.get(to).is_some_and(|next| *next < rank)
                    }
            });
            if !progresses {
                return Ok(reject(
                    ProbabilityCertificateRule::RankBroken,
                    ProbabilityCertificateLocus::ProductPart {
                        key: key.clone(),
                        part: ProbabilityCertificatePart::Reset,
                    },
                ));
            }
        }
    }

    let side = match subject.bound {
        Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
        Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
    };
    let mut coverage = Vec::new();
    let mut bindings = Vec::new();
    let mut instances = Vec::new();
    let mut least: Option<Rational> = None;
    for root in &graph.roots {
        let node = graph
            .nodes
            .get(&root.key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let value = if node.active {
            post_values.get(&root.key)
        } else if rebuilt::reaches_activation(graph, &root.key, cancel)? {
            reset_values.get(&root.key)
        } else {
            continue;
        }
        .ok_or(ProbabilityDocumentError::Malformed)?;
        if !meets(subject.bound, value) {
            return Ok(reject(
                ProbabilityCertificateRule::ThresholdMissed,
                ProbabilityCertificateLocus::ProductPart {
                    key: root.key.clone(),
                    part: ProbabilityCertificatePart::Reset,
                },
            ));
        }
        if least.as_ref().is_none_or(|known| {
            matches!(subject.bound, Bound::AtLeast(_)) && value < known
                || matches!(subject.bound, Bound::AtMost(_)) && value > known
        }) {
            least = Some(value.clone());
        }
        coverage.push((root.initial, Some(value.clone())));
        bindings.push(root.binding.clone());
        instances.push(root.instance.clone());
    }
    let Some(value) = least else {
        return Err(ProbabilityDocumentError::Malformed);
    };
    Ok(ExactProbabilityCertificateCheck::Accepted(
        ExactProbabilityAcceptance {
            initial_values: Some(CheckedInitialValues {
                values: coverage,
                bindings,
                instances,
            }),
            bound: None,
            policy: Vec::new(),
            value,
            side,
            certificate: ExactProbabilityCertificate {
                values: rows_of(&reset_values),
            },
            document: Some(bytes.into()),
        },
    ))
}

fn expectation(
    edges: &[(Key, Rational)],
    values: &BTreeMap<Key, Rational>,
    arith: &mut Arith<'_>,
) -> Result<Rational, ProbabilityDocumentError> {
    let mut mass = rational(0);
    let mut sum = rational(0);
    for (key, probability) in edges {
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let product = arith.apply(filament_exact::RationalArithmetic::Multiply(
            probability,
            value,
        ))?;
        sum = arith.apply(filament_exact::RationalArithmetic::Add(&sum, &product))?;
        mass = arith.apply(filament_exact::RationalArithmetic::Add(&mass, probability))?;
    }
    if mass != rational(1) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(sum)
}

fn expectation_lookup(
    edges: &[(Key, Rational)],
    post: &BTreeMap<Key, Rational>,
    reset: &BTreeMap<Key, Rational>,
    graph: &rebuilt::Graph,
    arith: &mut Arith<'_>,
) -> Result<Rational, ProbabilityDocumentError> {
    let values = edges
        .iter()
        .map(|(key, probability)| {
            let value = if graph.nodes.get(key).is_some_and(|node| node.active) {
                post.get(key)
            } else {
                reset.get(key)
            }
            .ok_or(ProbabilityDocumentError::Malformed)?;
            Ok((key.clone(), value.clone(), probability.clone()))
        })
        .collect::<Result<Vec<_>, ProbabilityDocumentError>>()?;
    let mut mass = rational(0);
    let mut sum = rational(0);
    for (_, value, probability) in values {
        let product = arith.apply(filament_exact::RationalArithmetic::Multiply(
            &probability,
            &value,
        ))?;
        sum = arith.apply(filament_exact::RationalArithmetic::Add(&sum, &product))?;
        mass = arith.apply(filament_exact::RationalArithmetic::Add(&mass, &probability))?;
    }
    if mass != rational(1) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(sum)
}

fn choose_extreme(
    mut candidates: Vec<Rational>,
    minimum: bool,
) -> Result<Rational, ProbabilityDocumentError> {
    let Some(first) = candidates.pop() else {
        return Err(ProbabilityDocumentError::Malformed);
    };
    Ok(candidates.into_iter().fold(first, |best, value| {
        if (minimum && value < best) || (!minimum && value > best) {
            value
        } else {
            best
        }
    }))
}

pub(super) fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let document = decode(bytes, limits, cancel, &mut meter)?;
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    // A `ratio` document offered for a claim under every scheduler or a free
    // delay is malformed at `mode`: the profile is a workload's (FR-201).
    if matches!(document.mode, Mode::Ratio { .. }) && free_scheduler(system, claims, item, cancel)?
    {
        return Ok(reject(
            ProbabilityCertificateRule::Malformed,
            ProbabilityCertificateLocus::Member("mode"),
        ));
    }
    let subject = subject(system, claims, item, cancel)?;
    let preliminary = rebuilt::build(system, &subject, limits, cancel)?;
    let mut reset_process = false;
    if matches!(&document.mode, Mode::Optimum { .. }) {
        for root in &preliminary.roots {
            let starts_waiting = preliminary
                .nodes
                .get(&root.key)
                .is_some_and(|node| !node.active);
            if starts_waiting
                && rebuilt::reaches_activation(&preliminary, &root.key, cancel)?
            {
                reset_process = true;
                break;
            }
        }
    }
    let initial_count = preliminary
        .roots
        .iter()
        .map(|root| root.initial)
        .collect::<BTreeSet<_>>()
        .len();
    let root_indexed = reset_process && initial_count > 1;
    let graph = if root_indexed {
        rebuilt::build_with_root_index(system, &subject, limits, cancel, true)?
    } else {
        preliminary
    };
    if graph.roots.is_empty() {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    if matches!(&document.mode, Mode::Optimum { .. }) {
        return check_optimum_document(
            system,
            &subject,
            &graph,
            document.mode,
            bytes,
            limits,
            cancel,
            &mut meter,
        );
    }
    if rebuilt::has_cycle(&graph, cancel)? {
        return Err(ExactProbabilityGraphError::Cycle.into());
    }
    let side = match subject.bound {
        Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
        Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
    };
    // FR-201: the first initial complete product key in canonical order.
    let first = graph
        .roots
        .iter()
        .map(|root| &root.key)
        .min()
        .ok_or(ProbabilityDocumentError::Malformed)?;
    // The first initial complete product key, in canonical order, from which
    // the checker's own search reaches a first activating state.
    let mut activating: Option<&Key> = None;
    for root in &graph.roots {
        if rebuilt::reaches_activation(&graph, &root.key, cancel)?
            && activating.is_none_or(|known| root.key < *known)
        {
            activating = Some(&root.key);
        }
    }
    let (numerator, denominator) = match document.mode {
        Mode::Vacuous => {
            return Ok(match activating {
                Some(key) => reject(
                    ProbabilityCertificateRule::InequalityFails,
                    ProbabilityCertificateLocus::ProductState(key.clone()),
                ),
                None => ExactProbabilityCertificateCheck::AcceptedVacuous(
                    ProbabilityVacuityAcceptance {
                        document: bytes.into(),
                        side,
                    },
                ),
            });
        }
        Mode::Ratio {
            numerator,
            denominator,
        } => (numerator, denominator),
        Mode::Optimum { .. } => return Err(ProbabilityDocumentError::Malformed),
    };
    let mut arith = Arith {
        meter: &mut meter,
        cancel,
        limits,
    };
    // Shape and coverage of both parts, before any arithmetic.
    let mut covered: Vec<(Part, Rows, BTreeMap<Key, u64>)> = Vec::with_capacity(2);
    for (part, rows) in [
        (Part::Numerator, numerator),
        (Part::Denominator, denominator),
    ] {
        let values: Rows = rows
            .values
            .iter()
            .map(|row| (row.state.clone(), row.value.clone()))
            .collect();
        let ranks: BTreeMap<Key, u64> = rows.ranks.iter().cloned().collect();
        if values.len() != graph.nodes.len()
            || ranks.len() != graph.nodes.len()
            || !graph
                .nodes
                .keys()
                .all(|key| values.contains_key(key) && ranks.contains_key(key))
        {
            return Ok(reject(
                ProbabilityCertificateRule::Malformed,
                ProbabilityCertificateLocus::Member(part_name(part)),
            ));
        }
        covered.push((part, values, ranks));
    }
    for (part, values, ranks) in &covered {
        let part = *part;
        for (key, node) in &graph.nodes {
            poll(cancel)?;
            let expected = arith.expected(
                node.active,
                node.leaf,
                node.next.iter().map(|(to, probability)| (to, probability)),
                part,
                &|target| {
                    values
                        .get(target)
                        .cloned()
                        .ok_or(ProbabilityDocumentError::Malformed)
                },
            )?;
            let Some(value) = values.get(key).filter(|value| **value == expected) else {
                return Ok(reject(
                    ProbabilityCertificateRule::InequalityFails,
                    part_locus(key, part),
                ));
            };
            // CE-3: a positive value that is not a target of the part has a
            // positive-valued successor of smaller rank.
            let target = node.active
                && match part {
                    Part::Denominator => true,
                    Part::Numerator => node.leaf == Some(true),
                };
            if *value > rational(0) && !target {
                let rank = ranks.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
                let progress = node.next.iter().any(|(to, _)| {
                    values.get(to).is_some_and(|next| *next > rational(0))
                        && ranks.get(to).is_some_and(|next| next < rank)
                });
                if !progress {
                    return Ok(reject(
                        ProbabilityCertificateRule::RankBroken,
                        part_locus(key, part),
                    ));
                }
            }
        }
    }
    let (Some((_, denominator, _)), Some((_, numerator, _))) = (covered.pop(), covered.pop())
    else {
        return Err(ProbabilityDocumentError::Malformed);
    };
    // Vacuity: a numeric certificate where no initial key activates is
    // rejected; it is certified only through `vacuous`.
    if activating.is_none() {
        return Ok(reject(
            ProbabilityCertificateRule::InequalityFails,
            ProbabilityCertificateLocus::ProductState(first.clone()),
        ));
    }
    let mut coverage = Vec::with_capacity(graph.roots.len());
    let mut bindings = Vec::with_capacity(graph.roots.len());
    let mut instances = Vec::with_capacity(graph.roots.len());
    let mut missed: Option<Key> = None;
    let mut least: Option<Rational> = None;
    for root in &graph.roots {
        let value = conditioned(&root.key, &numerator, &denominator, &mut arith)?;
        match &value {
            // A zero denominator is vacuity only when the checker's own
            // search finds no activation reachable.
            None if rebuilt::reaches_activation(&graph, &root.key, cancel)? => {
                return Ok(reject(
                    ProbabilityCertificateRule::InequalityFails,
                    part_locus(&root.key, Part::Denominator),
                ));
            }
            None => {}
            Some(value) => {
                if !meets(subject.bound, value)
                    && missed.as_ref().is_none_or(|known| root.key < *known)
                {
                    missed = Some(root.key.clone());
                }
                let worse = match (&least, subject.bound) {
                    (None, _) => true,
                    (Some(known), Bound::AtLeast(_)) => value < known,
                    (Some(known), Bound::AtMost(_)) => value > known,
                };
                if worse {
                    least = Some(value.clone());
                }
            }
        }
        coverage.push((root.initial, value));
        bindings.push(root.binding.clone());
        instances.push(root.instance.clone());
    }
    if let Some(key) = missed {
        return Ok(reject(
            ProbabilityCertificateRule::ThresholdMissed,
            ProbabilityCertificateLocus::ProductState(key),
        ));
    }
    poll(cancel)?;
    let Some(least) = least else {
        return Err(ProbabilityDocumentError::Malformed);
    };
    Ok(ExactProbabilityCertificateCheck::Accepted(
        ExactProbabilityAcceptance {
            initial_values: Some(CheckedInitialValues {
                values: coverage,
                bindings,
                instances,
            }),
            bound: None,
            policy: Vec::new(),
            value: least,
            side,
            certificate: ExactProbabilityCertificate {
                values: rows_of(&numerator),
            },
            document: Some(bytes.into()),
        },
    ))
}

/// Whether the claim `item` names is read under a scheduler choice rather
/// than a workload that leaves none.
fn free_scheduler(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    let claim = bind_claim(claims, item, cancel)?;
    Ok(claim_workload(system, claim)?.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn test_walk(states: BTreeMap<Key, WalkState>, root: Key) -> Walk {
        Walk {
            root,
            states,
            models: BTreeMap::new(),
        }
    }

    fn fraction(numerator: u64, denominator: u64) -> Rational {
        Rational::new(
            filament_exact::Integer::from(numerator),
            filament_exact::Integer::from(denominator),
        )
        .expect("nonzero test denominator")
    }

    fn test_limits() -> ProbabilityDocumentLimits {
        ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 1_000_000,
            states: 10_000,
            edges: 100_000,
            scalar_bits: 512,
            accounting: crate::AccountingLimits::default().0,
        }
    }

    fn active_state(choices: Vec<Vec<(Key, Rational)>>, terminal: Option<bool>) -> WalkState {
        WalkState {
            active: true,
            terminal,
            edges: Vec::new(),
            choices,
            choice_ids: Vec::new(),
        }
    }

    #[trace("FR-196-AC-16")]
    #[test]
    fn reset_candidates_cover_each_member_of_a_waiting_end_component() {
        let u = ProbabilityProductKey::from_canonical_bytes(b"u".to_vec());
        let v = ProbabilityProductKey::from_canonical_bytes(b"v".to_vec());
        let active = ProbabilityProductKey::from_canonical_bytes(b"active".to_vec());
        let waiting = BTreeSet::from([u.clone(), v.clone()]);
        let choices = BTreeMap::from([
            (
                u.clone(),
                vec![
                    vec![(v.clone(), rational(1))],
                    vec![(active.clone(), rational(1))],
                ],
            ),
            (
                v.clone(),
                vec![
                    vec![(u.clone(), rational(1))],
                    vec![(active, rational(1))],
                ],
            ),
        ]);
        assert_eq!(
            waiting_end_component_members(
                &waiting,
                &choices,
                &mut filament_exact::Meter::new(crate::AccountingLimits::default().0),
                &Cancel::new(),
            )
                .expect("the two-state SCC is finite"),
            waiting,
            "u and v each have a closed action inside the same waiting SCC"
        );
    }

    #[trace("FR-196-AC-16")]
    #[test]
    fn reset_candidates_keep_separate_closed_components_and_skip_nonprogress() {
        let u = ProbabilityProductKey::from_canonical_bytes(b"u".to_vec());
        let v = ProbabilityProductKey::from_canonical_bytes(b"v".to_vec());
        let w = ProbabilityProductKey::from_canonical_bytes(b"w".to_vec());
        let dead = ProbabilityProductKey::from_canonical_bytes(b"dead".to_vec());
        let active = ProbabilityProductKey::from_canonical_bytes(b"active".to_vec());
        let waiting = BTreeSet::from([u.clone(), v.clone(), w.clone(), dead.clone()]);
        let choices = BTreeMap::from([
            (u.clone(), vec![vec![(v.clone(), rational(1))], vec![(active.clone(), rational(1))]]),
            (v.clone(), vec![vec![(u.clone(), rational(1))], vec![(active.clone(), rational(1))]]),
            (w.clone(), vec![vec![(w.clone(), rational(1))], vec![(active.clone(), rational(1))]]),
            (dead.clone(), vec![vec![(active, rational(1))]]),
        ]);
        assert_eq!(
            waiting_end_component_members(
                &waiting,
                &choices,
                &mut filament_exact::Meter::new(crate::AccountingLimits::default().0),
                &Cancel::new(),
            )
            .expect("the closed supports are finite"),
            BTreeSet::from([u, v, w]),
            "u/v form one multi-state component, w a separate closed component, and dead has no closed support"
        );
    }

    #[trace("FR-196-AC-16")]
    #[test]
    fn reset_policy_storage_is_refused_before_dense_matrix_allocation() {
        let u = ProbabilityProductKey::from_canonical_bytes(b"u".to_vec());
        let v = ProbabilityProductKey::from_canonical_bytes(b"v".to_vec());
        let waiting = BTreeSet::from([u.clone(), v.clone()]);
        let choices = BTreeMap::from([
            (u.clone(), vec![vec![(v.clone(), rational(1))]]),
            (v.clone(), vec![vec![(u.clone(), rational(1))]]),
        ]);
        let policy = BTreeMap::from([(u.clone(), 0), (v.clone(), 0)]);
        let post = BTreeMap::new();
        let limits = ProbabilityDocumentLimits {
            bytes: 1_000,
            storage_bytes: 0,
            states: 10,
            edges: 10,
            scalar_bits: 64,
            accounting: crate::AccountingLimits::default().0,
        };
        assert!(matches!(
            evaluate_reset_policy(
                &waiting,
                &choices,
                &policy,
                &post,
                limits,
                &mut filament_exact::Meter::new(limits.accounting),
                &Cancel::new(),
            ),
            Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::StorageBytes,
                bound: 0,
                actual,
            }) if actual > 0
        ));
    }

    #[trace("FR-196-AC-16")]
    #[test]
    fn reset_policy_cancellation_is_observed_before_dense_matrix_allocation() {
        let u = ProbabilityProductKey::from_canonical_bytes(b"u".to_vec());
        let v = ProbabilityProductKey::from_canonical_bytes(b"v".to_vec());
        let waiting = BTreeSet::from([u.clone(), v.clone()]);
        let choices = BTreeMap::from([
            (u.clone(), vec![vec![(v.clone(), rational(1))]]),
            (v.clone(), vec![vec![(u.clone(), rational(1))]]),
        ]);
        let policy = BTreeMap::from([(u.clone(), 0), (v.clone(), 0)]);
        let post = BTreeMap::new();
        let limits = ProbabilityDocumentLimits {
            bytes: 1_000,
            storage_bytes: 8_000,
            states: 10,
            edges: 10,
            scalar_bits: 64,
            accounting: crate::AccountingLimits::default().0,
        };
        let cancel = Cancel::new();
        cancel.cancel(filament_exact::CancelCause::Deadline);
        assert!(matches!(
            evaluate_reset_policy(
                &waiting,
                &choices,
                &policy,
                &post,
                limits,
                &mut filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone()),
                &cancel,
            ),
            Err(ProbabilityDocumentError::Cancelled(
                filament_exact::CancelCause::Deadline
            ))
        ));
    }

    #[trace("FR-196-AC-16")]
    #[test]
    fn exact_post_rejects_a_forged_positive_active_self_loop() {
        let state = ProbabilityProductKey::from_canonical_bytes(b"active-self-loop".to_vec());
        let graph = rebuilt::Graph {
            nodes: BTreeMap::from([(
                state.clone(),
                rebuilt::Node {
                    active: true,
                    leaf: None,
                    next: vec![(state.clone(), rational(1))],
                    choices: vec![vec![(state.clone(), rational(1))]],
                    choice_ids: vec![None],
                },
            )]),
            roots: Vec::new(),
        };
        let forged = BTreeMap::from([(state.clone(), rational(1))]);
        let limits = test_limits();
        let mut meter = filament_exact::Meter::new(limits.accounting);
        let rejection = check_cyclic_post_values(
            &graph,
            &forged,
            false,
            &mut meter,
            &Cancel::new(),
        )
            .expect("the finite cyclic checker settles")
            .expect("a positive fixed point in a closed non-target component is rejected");
        assert_eq!(
            rejection.rule,
            ProbabilityCertificateRule::InequalityFails,
            "the forged one violates exact qualitative reachability"
        );
        assert_eq!(
            rejection.at,
            ProbabilityCertificateLocus::ProductPart {
                key: state,
                part: ProbabilityCertificatePart::Post,
            },
            "the failure is attributed to post before reset checks"
        );
    }

    #[trace("FR-196-AC-16")]
    #[test]
    fn exact_post_solves_cyclic_minimum_and_maximum_values_with_ranks() {
        let x = ProbabilityProductKey::from_canonical_bytes(b"x".to_vec());
        let target = ProbabilityProductKey::from_canonical_bytes(b"target".to_vec());
        let dead = ProbabilityProductKey::from_canonical_bytes(b"dead".to_vec());
        let half = fraction(1, 2);
        let quarter = fraction(1, 4);
        let three_quarters = fraction(3, 4);
        let mut states = BTreeMap::from([
            (
                target.clone(),
                active_state(Vec::new(), Some(true)),
            ),
            (dead.clone(), active_state(Vec::new(), Some(false))),
            (
                x.clone(),
                active_state(
                    vec![
                        vec![(target.clone(), half.clone()), (x.clone(), half.clone())],
                        vec![
                            (target.clone(), quarter.clone()),
                            (dead.clone(), three_quarters),
                        ],
                        vec![(x.clone(), rational(1))],
                    ],
                    None,
                ),
            ),
        ]);
        let walk = test_walk(states.clone(), x.clone());
        let limits = test_limits();
        let mut meter = filament_exact::Meter::new(limits.accounting);
        let (maximum, maximum_ranks) =
            post_graph_optimum(&walk, false, limits, &mut meter, &Cancel::new())
                .expect("maximum on the active cycle is finite");
        assert_eq!(maximum[&x], rational(1));
        assert_eq!(maximum[&target], rational(1));
        assert_eq!(maximum[&dead], rational(0));
        assert_eq!(maximum_ranks[&x], 1);
        assert_eq!(maximum_ranks[&target], 0);
        let mut rebuilt_graph = rebuilt::Graph {
            nodes: BTreeMap::from([
                (
                    target.clone(),
                    rebuilt::Node {
                        active: true,
                        leaf: Some(true),
                        next: Vec::new(),
                        choices: Vec::new(),
                        choice_ids: Vec::new(),
                    },
                ),
                (
                    dead.clone(),
                    rebuilt::Node {
                        active: true,
                        leaf: Some(false),
                        next: Vec::new(),
                        choices: Vec::new(),
                        choice_ids: Vec::new(),
                    },
                ),
                (
                    x.clone(),
                    rebuilt::Node {
                        active: true,
                        leaf: None,
                        next: Vec::new(),
                        choices: walk.states[&x].choices.clone(),
                        choice_ids: Vec::new(),
                    },
                ),
            ]),
            roots: Vec::new(),
        };
        assert!(check_cyclic_post_values(
            &rebuilt_graph,
            &maximum,
            false,
            &mut meter,
            &Cancel::new(),
        )
        .expect("independent maximum post check")
        .is_none());

        let (minimum, minimum_ranks) =
            post_graph_optimum(&walk, true, limits, &mut meter, &Cancel::new())
                .expect("minimum closed-cycle value is finite");
        assert_eq!(minimum[&x], rational(0));
        assert_eq!(minimum_ranks[&x], 0);
        assert_eq!(minimum_ranks[&target], 0);
        let zero_minimum_check = check_cyclic_post_values(
            &rebuilt_graph,
            &minimum,
            true,
            &mut meter,
            &Cancel::new(),
        )
        .expect("independent zero minimum post check");
        assert_eq!(zero_minimum_check, None, "minimum cycle is exact");

        states.get_mut(&x).expect("x state").choices.pop();
        let positive_minimum_walk = test_walk(states, x.clone());
        rebuilt_graph
            .nodes
            .get_mut(&x)
            .expect("rebuilt x")
            .choices = positive_minimum_walk.states[&x].choices.clone();
        let (positive_minimum, positive_minimum_ranks) = post_graph_optimum(
            &positive_minimum_walk,
            true,
            limits,
            &mut meter,
            &Cancel::new(),
        )
        .expect("minimum positive cyclic reachability value is finite");
        assert_eq!(positive_minimum[&x], quarter);
        assert_eq!(positive_minimum_ranks[&x], 1);
        assert!(check_cyclic_post_values(
            &rebuilt_graph,
            &positive_minimum,
            true,
            &mut meter,
            &Cancel::new(),
        )
        .expect("independent positive minimum post check")
        .is_none());
    }
}
