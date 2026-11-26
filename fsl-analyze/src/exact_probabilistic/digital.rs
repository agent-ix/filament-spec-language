// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-204 (ADR-028 TA-1 to TA-7): the digital MDP of a closed timed subject.
//!
//! A digital timed state is the discrete model state with an integer value
//! per clock, capped at one plus the clock's largest compared constant after
//! every constant is scaled to an integer, the optional deadline clock of a
//! `[0, D]` event, and TS-5's tail mode. Its moves are the unit delay, the
//! discrete steps with integer clocks, and, in a quiescent state, the idle
//! tail's enter-delay, delay and observation.
//!
//! Over every scheduler the scheduler chooses each delay and each scheduled
//! identity. Under a workload, a `discrete` delay distribution turns the race
//! of ADR-026 SD-3 into one probabilistic edge per outcome; a delay no
//! distribution gives stays the scheduler's choice: between a unit delay and
//! the workload's resolution of the identity where every racing delay is
//! free, and as the post-draw `FreeDelays` action where given and free delays
//! race together (FR-204).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use filament_exact::{
    Cancel, Identifier, Integer, ObjectReference, Quantity, Rational, Value, ValueType,
};
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::probabilistic::TimedOperatorState;
use fsl_eval::simulation::StateKey;
use fsl_eval::simulation::model::{
    Action, ClockValuation, GuardedStep, ModelSimState, ModelSystem, NotMarkov, PlaceConstraint,
    PredicateReading, ScheduledIdentity, TimedStop, TimedSystem, same_rewards,
};
use fsl_replay::{
    DigitalClockKey, DigitalDeadlineKey, DigitalProductKeyInput, DigitalTail, ModelCheckLimit,
    ProbabilityProductKey, RegionClockKey, RegionDeadlineKey, RegionKeyKind, RegionProductKeyInput,
    digital_probability_product_key, region_probability_product_key,
};
use fsl_semantics::check::{
    BinaryTemporal, BoundedMeasure, CheckedInterval, CheckedPredicate, CheckedProbabilisticClaim,
    ClockOp, ConvexPart, DelayDistribution, Event, FormulaNode, ProbForm, ProbScheduler,
    UnaryTemporal, Workload,
};
use ix_trace_rs::implements;
use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{One, ToPrimitive, Zero};
use serde_json::{Value as JsonValue, json};

use super::unbounded::{Choice, Graph, maximal_end_components_from, same_action};
use super::{
    DeadlineKind, DeadlinePhase, ExactProbLimits, FreeDelayRow, MeasurePhase, MonitorState,
    MonitorVerdict, PathStep, Pending, ProbBuild, ProbEdge, ProbKey, ProbProduct, ProbStateId,
    ProductAction, ProductKind, ProductRefusal, ProductStop, UndefinedPath, UndefinedRead,
};

pub(crate) const ELAPSED_REWARD: &str = "qsl_elapsed_time";

/// Build the digital product of a claim over a timed subject (FR-204).
///
/// # Errors
///
/// [`ProductRefusal`]: the admission causes of FR-204, a model that is not a
/// Markov decision process or chain at some digital state, a budget or the
/// cancellation.
#[implements("FR-204-AC-1")]
#[implements("FR-125-AC-9")]
pub(super) fn build<'s, 'a>(
    system: &'s ModelSystem<'a>,
    timed: TimedSystem<'s, 'a>,
    claim: &CheckedProbabilisticClaim,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbBuild, ProductRefusal> {
    let package = system
        .source_package()
        .ok_or_else(|| fault("the-product-reads-source-forms"))?;
    let workload = match &claim.scheduler {
        ProbScheduler::Workload(name) => Some(
            package
                .graph()
                .probabilistic_model()
                .workload(name)
                .ok_or_else(|| fault("claim-workload-is-declared"))?,
        ),
        ProbScheduler::Every => None,
    };
    let form = Form::of(claim)?;
    admit(system, package, claim)?;
    let (base_scale, subdivisions) = scale_of(system, &timed, claim, &form)?;
    let scale = base_scale
        .checked_mul(subdivisions)
        .ok_or_else(|| fault("region-scale-fits-u64"))?;
    let caps = caps_of(&timed, &scale)?;
    let bindings = match &claim.over {
        None => vec![(None, None)],
        Some(over) => {
            let ValueType::Reference(object) = &over.value_type else {
                return Err(fault("over-is-an-object-type"));
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
        system,
        timed,
        workload,
        form,
        scale,
        base_scale,
        subdivisions,
        caps,
        bindings,
        max_states: limits.model.limits.max_states,
        max_transitions: limits.model.limits.max_transitions,
        cancel,
        locals: BTreeMap::new(),
        nodes: Vec::new(),
        index: BTreeMap::new(),
        pending_keys: BTreeSet::new(),
        readings: BTreeMap::new(),
        edges: Vec::new(),
        zero_delay: Vec::new(),
        queue: VecDeque::new(),
        decisions: false,
        time_locked: Vec::new(),
    };
    builder.run()
}

/// At a state whose racing identities all have free delays, the scheduler's
/// choice to let the workload resolve the identity after `delay` units
/// (ADR-028 TA-1, FR-204).
fn resolve_choice(
    representative: Option<&ScheduledIdentity>,
    taken: &Enabled<'_>,
    delay: u64,
) -> ProductAction {
    ProductAction::WorkloadResolve {
        identity: representative.unwrap_or(&taken.identity).clone(),
        delay,
    }
}

fn fault(invariant: &'static str) -> ProductRefusal {
    ProductRefusal::Internal(InternalFault::new("exact-probabilistic-digital", invariant))
}

fn one() -> Rational {
    Rational::from_integer(Integer::one())
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// The operators, occurrences, bounds and reads of [`Form::NestedDeadline`].
struct NestedDeadline<'c> {
    outer_kind: DeadlineKind,
    outer_occurrence: u64,
    outer_lower: Rational,
    outer_upper: Rational,
    inner_kind: DeadlineKind,
    inner_occurrence: u64,
    inner_lower: Rational,
    inner_upper: Rational,
    reads: Vec<(UndefinedRead, &'c CheckedPredicate)>,
}

/// The claim forms the digital route admits (ADR-028 TA-4).
enum Form<'c> {
    /// `eventually[0, D] holds(P)` or `holds(A) until[0, D] holds(B)`.
    Deadline {
        kind: DeadlineKind,
        /// The closed lower end of the time window.
        lower: Rational,
        /// `D` in the model's time unit.
        bound: Rational,
        reads: Vec<(UndefinedRead, &'c CheckedPredicate)>,
    },
    /// One punctual nested occurrence under its dual bounded operator:
    /// `always` under `eventually`, or `eventually` under `always`.
    NestedDeadline(Box<NestedDeadline<'c>>),
    /// Expected elapsed time until a checked state predicate holds.
    MeanTime {
        /// The expected-time comparison threshold in the model's time unit.
        bound: Rational,
        /// The target predicate used to label product states.
        reads: Vec<(UndefinedRead, &'c CheckedPredicate)>,
    },
    /// FR-877: `quantile q of (elapsed from A until B within W) op c`; its
    /// reads are `A` then `B`.
    Elapsed {
        /// The comparison threshold `c` in the model's time unit.
        threshold: Rational,
        /// Whether the transform compares `M < c` (a `>=` claim).
        strict: bool,
        /// The window `W` in the model's time unit.
        window: Rational,
        reads: Vec<(UndefinedRead, &'c CheckedPredicate)>,
    },
    /// An unbounded reachability form: its predicates label the states.
    Labels(Vec<(UndefinedRead, &'c CheckedPredicate)>),
}

impl<'c> Form<'c> {
    #[implements("FR-204-AC-21")]
    fn of(claim: &'c CheckedProbabilisticClaim) -> Result<Self, ProductRefusal> {
        let shape = || ProductRefusal::TimedFormShape {
            span: claim.span,
            claim: claim.occurrence().cloned(),
            member: claim.timed_form_member_path(),
        };
        let shape_at = |chain: &[usize]| ProductRefusal::TimedFormShape {
            span: claim.span,
            claim: claim.occurrence().cloned(),
            member: claim.formula_member_path(chain),
        };
        match &claim.form {
            ProbForm::MeanTime { bound, until } => Ok(Self::MeanTime {
                bound: match bound {
                    fsl_semantics::check::Bound::AtLeast(value)
                    | fsl_semantics::check::Bound::AtMost(value) => value.clone(),
                },
                reads: vec![(UndefinedRead::Predicate(super::PredicateRole::Until), until)],
            }),
            ProbForm::Probability {
                event: Event::Formula { formula, .. },
                ..
            } => {
                let checked = formula.formula();
                let nodes = checked.nodes();
                let root_at = checked.root().index();
                let root = nodes.get(root_at).ok_or_else(|| shape_at(&[root_at]))?;
                let holds = |id: super::FormulaNodeId| {
                    matches!(nodes.get(id.index()), Some(FormulaNode::Holds { .. }))
                        .then(|| formula.condition(id.index()).map(|p| (id, p)))
                        .flatten()
                };
                let (kind, interval, atoms) = match root {
                    FormulaNode::Unary {
                        operator: operator @ (UnaryTemporal::Eventually | UnaryTemporal::Always),
                        interval: Some(CheckedInterval::Timed(interval)),
                        operand,
                    } => {
                        // Only the root operator's dual may nest under it.
                        let (kind, dual, dual_kind) = if *operator == UnaryTemporal::Eventually {
                            (
                                DeadlineKind::Eventually,
                                UnaryTemporal::Always,
                                DeadlineKind::Always,
                            )
                        } else {
                            (
                                DeadlineKind::Always,
                                UnaryTemporal::Eventually,
                                DeadlineKind::Eventually,
                            )
                        };
                        match nodes.get(operand.index()) {
                            Some(FormulaNode::Holds { .. }) => (
                                kind,
                                interval,
                                vec![
                                    holds(*operand)
                                        .ok_or_else(|| shape_at(&[root_at, operand.index()]))?,
                                ],
                            ),
                            Some(FormulaNode::Unary {
                                operator: inner_operator,
                                interval: Some(CheckedInterval::Timed(inner)),
                                operand: atom,
                            }) if *inner_operator == dual => {
                                let Some((id, predicate)) = holds(*atom) else {
                                    return Err(shape_at(&[
                                        root_at,
                                        operand.index(),
                                        atom.index(),
                                    ]));
                                };
                                let Some(inner_upper) = inner.upper() else {
                                    return Err(shape_at(&[root_at, operand.index()]));
                                };
                                if inner.lower() != inner_upper
                                    || !interval.lower_closed()
                                    || !interval.upper_closed()
                                    || !inner.lower_closed()
                                    || !inner.upper_closed()
                                {
                                    return Err(shape_at(&[root_at]));
                                }
                                return Ok(Self::NestedDeadline(Box::new(NestedDeadline {
                                    outer_kind: kind,
                                    outer_occurrence: u64::try_from(root_at)
                                        .map_err(|_| shape_at(&[root_at]))?,
                                    outer_lower: interval.lower().clone(),
                                    outer_upper: interval
                                        .upper()
                                        .ok_or_else(|| shape_at(&[root_at]))?
                                        .clone(),
                                    inner_kind: dual_kind,
                                    inner_occurrence: u64::try_from(operand.index())
                                        .map_err(|_| shape_at(&[root_at, operand.index()]))?,
                                    inner_lower: inner.lower().clone(),
                                    inner_upper: inner_upper.clone(),
                                    reads: vec![(UndefinedRead::Atom(id), predicate)],
                                })));
                            }
                            _ => return Err(shape_at(&[root_at, operand.index()])),
                        }
                    }
                    FormulaNode::Binary {
                        operator: BinaryTemporal::Until,
                        interval: Some(CheckedInterval::Timed(interval)),
                        left,
                        right,
                    } => (
                        DeadlineKind::Until,
                        interval,
                        vec![
                            holds(*left).ok_or_else(|| shape_at(&[root_at, left.index()]))?,
                            holds(*right).ok_or_else(|| shape_at(&[root_at, right.index()]))?,
                        ],
                    ),
                    _ => return Err(shape_at(&[root_at])),
                };
                if !interval.lower_closed() || !interval.upper_closed() {
                    return Err(shape_at(&[root_at]));
                }
                // A deadline needs an upper bound; `[0, *)` checks as no interval.
                let Some(bound) = interval.upper() else {
                    return Err(shape_at(&[root_at]));
                };
                Ok(Self::Deadline {
                    kind,
                    lower: interval.lower().clone(),
                    bound: bound.clone(),
                    reads: atoms
                        .into_iter()
                        .map(|(id, predicate)| (UndefinedRead::Atom(id), predicate))
                        .collect(),
                })
            }
            ProbForm::Quantile {
                measure:
                    BoundedMeasure::Elapsed {
                        from,
                        until,
                        within,
                        ..
                    },
                threshold,
                ..
            } => Ok(Self::Elapsed {
                threshold: match threshold {
                    fsl_semantics::check::Bound::AtLeast(value)
                    | fsl_semantics::check::Bound::AtMost(value) => value.clone(),
                },
                strict: matches!(threshold, fsl_semantics::check::Bound::AtLeast(_)),
                window: within.clone(),
                reads: vec![
                    (UndefinedRead::Predicate(super::PredicateRole::From), from),
                    (UndefinedRead::Predicate(super::PredicateRole::Until), until),
                ],
            }),
            ProbForm::Reach { .. } | ProbForm::ExpectedReward { .. } => {
                let (plan, reads) = super::plan::Plan::of(claim, 1).map_err(|_| shape())?;
                drop(plan);
                Ok(Self::Labels(reads))
            }
            _ => Err(shape()),
        }
    }

    #[implements("FR-204-AC-21")]
    fn reads(&self) -> &[(UndefinedRead, &'c CheckedPredicate)] {
        match self {
            Self::Deadline { reads, .. }
            | Self::MeanTime { reads, .. }
            | Self::Elapsed { reads, .. }
            | Self::Labels(reads) => reads,
            Self::NestedDeadline(deadline) => &deadline.reads,
        }
    }
}

/// Every atomic clock constraint of a convex part, families included.
fn constraints_of<'m>(
    part: &'m ConvexPart,
    found: &mut Vec<&'m fsl_semantics::check::ClockConstraint>,
) {
    found.extend(part.clocks());
    for family in part.families() {
        for inner in family.parts() {
            constraints_of(inner, found);
        }
    }
}

/// The model's clock constraints in guard-then-invariant declaration order.
pub(crate) fn model_constraints<'m>(
    timed: &'m TimedSystem<'_, '_>,
) -> Vec<&'m fsl_semantics::check::ClockConstraint> {
    let mut found = Vec::new();
    for guard in timed.model().guards() {
        for part in guard.parts() {
            constraints_of(part, &mut found);
        }
    }
    for invariant in timed.model().invariants() {
        found.extend(invariant.bounds());
    }
    found
}

/// FR-204 "Admission": the delay distributions and the form shape.
fn admit(
    system: &ModelSystem<'_>,
    package: &fsl_package::CheckedPackage,
    claim: &CheckedProbabilisticClaim,
) -> Result<(), ProductRefusal> {
    let delays = package.graph().probabilistic_model().delays();
    for delay in delays
        .iter()
        .filter(|delay| delay.operation().model() == claim.subject.alias.as_str())
    {
        let admitted = matches!(delay.distribution(), DelayDistribution::Discrete(_));
        if !admitted {
            let operation = delay.operation();
            return Err(ProductRefusal::DelayDistribution {
                operation: system
                    .operation_text(operation.declaring(), operation.name())
                    .unwrap_or_else(|| operation.name())
                    .to_owned(),
            });
        }
    }
    Ok(())
}

/// Where a race outcome's edges go: `planned`, merging an edge into an
/// existing one only from index `scope` on, so that edges of different
/// scheduler choices stay apart.
struct Edges<'p> {
    planned: &'p mut Vec<Planned>,
    scope: usize,
}

/// Two reward lists, added value by value: the rewards of consecutive steps
/// of one chain. A reward is an integer or a quantity of one unit.
fn add_rewards(
    left: &[(Identifier, Value)],
    right: &[(Identifier, Value)],
) -> Step<Vec<(Identifier, Value)>> {
    if left.len() != right.len() {
        return Err(fault("rewards-have-one-name-each").into());
    }
    left.iter()
        .zip(right)
        .map(|((name, first), (other, second))| {
            let sum = match (first, second) {
                (Value::Integer(first), Value::Integer(second)) if name == other => {
                    Value::Integer(first.add(second))
                }
                (Value::Quantity(first), Value::Quantity(second))
                    if name == other && first.unit() == second.unit() =>
                {
                    Value::Quantity(Quantity::new(
                        first.magnitude().add(second.magnitude()),
                        first.unit().clone(),
                    ))
                }
                _ => return Err(fault("a-reward-is-an-integer-or-a-quantity").into()),
            };
            Ok((name.clone(), sum))
        })
        .collect()
}

/// The least common denominator of every time constant of the model and the
/// claim (ADR-028 TA-3), as one positive integer.
#[implements("FR-204-AC-1")]
fn scale_of(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    form: &Form<'_>,
) -> Result<(u64, u64), ProductRefusal> {
    let mut denominators: Vec<BigInt> = Vec::new();
    let mut note = |value: &Rational| denominators.push(value.denominator().as_big().clone());
    for constraint in model_constraints(timed) {
        note(constraint.bound());
    }
    for resets in timed.model().resets() {
        for reset in resets.resets() {
            note(reset.value());
        }
    }
    match form {
        Form::Deadline { lower, bound, .. } => {
            note(lower);
            note(bound);
        }
        Form::NestedDeadline(deadline) => {
            note(&deadline.outer_lower);
            note(&deadline.outer_upper);
            note(&deadline.inner_lower);
            note(&deadline.inner_upper);
        }
        Form::MeanTime { bound, .. } => note(bound),
        Form::Elapsed {
            threshold, window, ..
        } => {
            note(threshold);
            note(window);
        }
        Form::Labels(_) => {}
    }
    for delay in system.input().probabilistic.delays() {
        if let DelayDistribution::Discrete(values) = delay.distribution() {
            for (delay, _) in values {
                note(delay);
            }
        }
    }
    let scale = denominators
        .iter()
        .fold(BigInt::one(), |factor, denominator| factor.lcm(denominator));
    let scale = scale
        .to_u64()
        .filter(|scale| *scale > 0)
        .ok_or_else(|| fault("digital-scale-fits-u64"))?;
    if region_route(timed, claim) {
        // For n model clocks plus the optional deadline clock, the standard
        // region partition has at most n+1 distinct fractional ranks. This
        // refinement gives each rank a representative on the integer grid,
        // while preserving every integral boundary exactly.
        let clocks = timed
            .places()
            .len()
            .checked_add(usize::from(matches!(
                form,
                Form::Deadline { .. } | Form::NestedDeadline(_) | Form::Elapsed { .. }
            )))
            .and_then(|count| count.checked_add(1))
            .and_then(|count| u64::try_from(count).ok())
            .ok_or_else(|| fault("region-clock-rank-count-fits-u64"))?;
        return Ok((scale, clocks));
    }
    Ok((scale, 1))
}

/// Whether FR-922 requires a region graph for this subject and claim.
pub(super) fn region_route(timed: &TimedSystem<'_, '_>, claim: &CheckedProbabilisticClaim) -> bool {
    model_constraints(timed)
        .into_iter()
        .any(|constraint| matches!(constraint.op(), ClockOp::Less | ClockOp::Greater))
        // FR-877: `M < c` is strict, so a `>=` quantile takes the region route.
        || matches!(
            &claim.form,
            ProbForm::Quantile {
                measure: BoundedMeasure::Elapsed { .. },
                threshold: fsl_semantics::check::Bound::AtLeast(_),
                ..
            }
        )
        || matches!(
            &claim.form,
            ProbForm::Probability {
                event: Event::Formula { formula, .. },
                ..
            } if formula.formula().nodes().iter().any(|node| match node {
                FormulaNode::Unary {
                    interval: Some(CheckedInterval::Timed(interval)),
                    ..
                }
                | FormulaNode::Binary {
                    interval: Some(CheckedInterval::Timed(interval)),
                    ..
                } => !interval.upper_closed() || !interval.lower_closed(),
                _ => false,
            })
        )
}

/// The lattice denominator written in an FR-920 product certificate.
pub(super) fn certificate_scale(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
) -> Result<u64, ProductRefusal> {
    let form = Form::of(claim)?;
    let (base, subdivisions) = scale_of(system, timed, claim, &form)?;
    base.checked_mul(subdivisions)
        .ok_or_else(|| fault("region-scale-fits-u64"))
}

/// A rational time constant as a count of digital units.
fn units(value: &Rational, scale: u64) -> Option<u64> {
    let scaled = value.numerator().as_big() * BigInt::from(scale);
    let (quotient, remainder) = scaled.div_rem(value.denominator().as_big());
    remainder.is_zero().then(|| quotient.to_u64()).flatten()
}

/// Each clock place's cap: one plus the largest scaled constant its value is
/// compared with, in its clock field or in one a collection write carries it
/// into (FR-196 "Canonical probability product key").
fn caps_of(timed: &TimedSystem<'_, '_>, scale: &u64) -> Result<Vec<u64>, ProductRefusal> {
    let mut largest = vec![0_u64; timed.model().clocks().len()];
    for constraint in model_constraints(timed) {
        let constant = units(constraint.bound(), *scale).ok_or_else(|| fault("constant-scales"))?;
        let entry = largest
            .get_mut(constraint.clock().index())
            .ok_or_else(|| fault("clock-field-exists"))?;
        *entry = (*entry).max(constant);
    }
    timed.raise_by_carries(&mut largest);
    timed
        .places()
        .iter()
        .map(|place| {
            largest
                .get(place.clock().index())
                .copied()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| fault("cap-fits-u64"))
        })
        .collect()
}

/// TS-5's tail mode (FR-196 canonical key).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Tail {
    Running,
    IdleReady,
    IdleDelayed,
}

/// A digital timed state.
#[derive(Clone)]
struct DState {
    model: ModelSimState,
    clocks: Vec<u64>,
    deadline: Option<u64>,
    tail: Tail,
}

/// What a discrete model state offers, computed once per state.
struct Local {
    actions: Vec<Action>,
    steps: Vec<GuardedStep>,
    invariants: Vec<PlaceConstraint>,
    urgent: bool,
    quiescent: bool,
}

/// A draw of an enabled identity: its enabled post-states.
struct EnabledDraw<'local> {
    transition: &'local filament_semantic_value::model_transition::ModelTransition,
    random: Vec<Value>,
    probability: Rational,
    rewards: Vec<(Identifier, Value)>,
    posts: Vec<(ModelSimState, Vec<u64>)>,
}

/// A scheduled identity enabled at a digital state.
struct Enabled<'local> {
    identity: ScheduledIdentity,
    draws: Vec<EnabledDraw<'local>>,
}

struct Resolution {
    successors: Vec<DState>,
}

struct Node {
    key: ProbKey,
    state: DState,
    truths: Vec<bool>,
    parent: Option<(usize, Option<TraceStep>)>,
    initial: u32,
    resolution: Option<Resolution>,
}

struct Target {
    key: ProbKey,
    state: DState,
    truths: Vec<bool>,
    resolution: Option<Resolution>,
}

#[derive(Clone)]
struct TraceStep {
    action: Option<ScheduledIdentity>,
    draw: Option<Vec<Value>>,
    probability: Rational,
}

struct Planned {
    choice: ProductAction,
    action: Option<ScheduledIdentity>,
    draw: Option<Vec<Value>>,
    target: Target,
    probability: Rational,
    rewards: Vec<(Identifier, Value)>,
    elapsed_ticks: u64,
    step: Option<TraceStep>,
    zero_delay: bool,
}

struct DelayMove<'a> {
    at: usize,
    state: &'a DState,
    choice: ProductAction,
    delayed: (Vec<u64>, Option<u64>),
    tail: Tail,
    elapsed_ticks: u64,
    planned: &'a [Planned],
    omit_unchanged: bool,
}

enum ClockKey {
    Digital(DigitalClockKey),
    Region(RegionClockKey),
}

enum DeadlineKey {
    Digital(DigitalDeadlineKey),
    Region(RegionDeadlineKey),
}

enum Halt {
    Refusal(ProductRefusal),
    Undefined(Box<UndefinedPath>),
}

impl From<ProductRefusal> for Halt {
    fn from(refusal: ProductRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

type Step<T> = Result<T, Halt>;

struct Hit {
    read: UndefinedRead,
    cause: fsl_eval::temporal::UndefinedCause,
    state_key: StateKey,
}

/// The window of one scheduled identity: the delays, in digital units, after
/// which it is enabled, and whether the window continues forever.
struct Window {
    identity: ScheduledIdentity,
    delays: BTreeSet<u64>,
    forever: bool,
    given: Option<Vec<(u64, Rational)>>,
}

impl Window {
    fn contains(&self, delay: u64, reached: u64) -> bool {
        self.delays.contains(&delay.min(reached)) && (delay <= reached || self.forever)
    }
}

/// Every index vector below `radices`, in ascending lexicographic order (the
/// last index fastest): one empty vector for no radix, none when a radix is
/// zero.
struct Odometer {
    radices: Vec<usize>,
    next: Option<Vec<usize>>,
}

impl Odometer {
    fn new(radices: impl IntoIterator<Item = usize>) -> Self {
        let radices: Vec<usize> = radices.into_iter().collect();
        let next = (!radices.contains(&0)).then(|| vec![0; radices.len()]);
        Self { radices, next }
    }
}

impl Iterator for Odometer {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Vec<usize>> {
        let current = self.next.take()?;
        let mut following = current.clone();
        let mut advanced = false;
        for (digit, radix) in following.iter_mut().zip(&self.radices).rev() {
            *digit += 1;
            if *digit < *radix {
                advanced = true;
                break;
            }
            *digit = 0;
        }
        if advanced {
            self.next = Some(following);
        }
        Some(current)
    }
}

/// Visit every ordering of the tied identities (each with its operation's
/// workload weight) with its probability times `probability`: drawing without
/// replacement by the operations' weights, and uniformly within an operation
/// (ADR-026 SD-3). One ordering is held at a time, so a visitor that checks
/// the transition budget bounds the work done before it stops.
fn each_ordering(
    tied: &[(ScheduledIdentity, Rational)],
    probability: Rational,
    prefix: &mut Vec<ScheduledIdentity>,
    visit: &mut dyn FnMut(Rational, &[ScheduledIdentity]) -> Step<()>,
) -> Step<()> {
    if tied.is_empty() {
        return visit(probability, prefix);
    }
    let mut operations: Vec<(&str, &Rational, u64)> = Vec::new();
    for (identity, weight) in tied {
        match operations
            .iter_mut()
            .find(|(name, _, _)| *name == identity.operation)
        {
            Some((_, _, count)) => *count += 1,
            None => operations.push((identity.operation.as_str(), weight, 1)),
        }
    }
    // FR-187: the draw weights are fsl-eval's, shared with `weighted_steps`.
    let effective =
        fsl_eval::simulation::model::draw_weights(operations.iter().map(|(_, weight, _)| *weight));
    let total = effective
        .iter()
        .flatten()
        .fold(zero(), |sum, weight| sum.add(weight));
    for ((name, _, count), weight) in operations.iter().zip(effective) {
        let Some(weight) = weight else {
            continue;
        };
        let each = weight
            .div(&total)
            .and_then(|share| {
                share.div(&Rational::from_integer(Integer::from_big(BigInt::from(
                    *count,
                ))))
            })
            .ok_or_else(|| fault("workload-weights-sum"))?;
        for (index, (identity, _)) in tied.iter().enumerate() {
            if identity.operation != *name {
                continue;
            }
            let rest: Vec<(ScheduledIdentity, Rational)> = tied
                .iter()
                .enumerate()
                .filter(|(position, _)| *position != index)
                .map(|(_, entry)| entry.clone())
                .collect();
            prefix.push(identity.clone());
            each_ordering(&rest, probability.mul(&each), prefix, visit)?;
            prefix.pop();
        }
    }
    Ok(())
}

/// The digital states reached by successive unit delays from a state: index
/// `k` is the state after `k` units, and `forever` holds when the last one
/// repeats under another unit.
struct Trajectory {
    states: Vec<DState>,
    forever: bool,
}

struct Builder<'r, 'a, 'c> {
    system: &'r ModelSystem<'a>,
    timed: TimedSystem<'r, 'a>,
    workload: Option<&'r Workload>,
    form: Form<'c>,
    scale: u64,
    base_scale: u64,
    subdivisions: u64,
    caps: Vec<u64>,
    bindings: Vec<(Option<ModelInstance>, Option<ObjectReference>)>,
    max_states: usize,
    max_transitions: usize,
    cancel: &'r Cancel,
    locals: BTreeMap<Vec<u8>, Rc<Local>>,
    nodes: Vec<Node>,
    index: BTreeMap<ProbKey, usize>,
    pending_keys: BTreeSet<ProbKey>,
    readings: BTreeMap<(usize, Vec<u8>), Vec<bool>>,
    edges: Vec<ProbEdge>,
    zero_delay: Vec<bool>,
    queue: VecDeque<usize>,
    decisions: bool,
    time_locked: Vec<usize>,
}

fn timed_refusal(stop: TimedStop) -> Halt {
    match stop {
        TimedStop::Expansion(stop) => Halt::Refusal(ProductRefusal::Expansion(stop)),
        _ => Halt::Refusal(fault("timed-moves-decide")),
    }
}

impl<'c> Builder<'_, '_, 'c> {
    fn run(mut self) -> Result<ProbBuild, ProductRefusal> {
        match self.explore() {
            Ok(initial) => self.finish(initial).map(ProbBuild::Built),
            Err(Halt::Refusal(refusal)) => Err(refusal),
            Err(Halt::Undefined(path)) => Ok(ProbBuild::Undefined(*path)),
        }
    }

    fn poll(&self) -> Step<()> {
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

    // -- digital arithmetic ------------------------------------------------

    fn valuation(&self, clocks: &[u64]) -> Step<ClockValuation> {
        let denominator = Integer::from_big(BigInt::from(self.scale));
        let values = clocks
            .iter()
            .map(|value| {
                Rational::new(Integer::from_big(BigInt::from(*value)), denominator.clone())
                    .map_err(|_| fault("scale-is-positive").into())
            })
            .collect::<Step<Vec<_>>>()?;
        Ok(ClockValuation::new(values))
    }

    fn digital(&self, valuation: &ClockValuation) -> Step<Vec<u64>> {
        valuation
            .values()
            .iter()
            .zip(&self.caps)
            .map(|(value, cap)| {
                let units = units(value, self.scale).ok_or_else(|| fault("clock-on-grid"))?;
                Ok(units.min(*cap))
            })
            .collect()
    }

    fn bound_units(&self, bound: &Rational) -> Step<u64> {
        units(bound, self.scale).ok_or_else(|| fault("constant-scales").into())
    }

    #[implements("FR-204-AC-21")]
    fn deadline_bound(&self) -> Step<Option<u64>> {
        match &self.form {
            Form::Deadline { bound, .. } => self.bound_units(bound).map(Some),
            Form::NestedDeadline(deadline) => self
                .bound_units(&deadline.outer_upper.add(&deadline.inner_upper))
                .map(Some),
            Form::Elapsed { window, .. } => self.bound_units(window).map(Some),
            Form::MeanTime { .. } | Form::Labels(_) => Ok(None),
        }
    }

    #[implements("FR-204-AC-21")]
    fn deadline_lower(&self) -> Step<Option<u64>> {
        match &self.form {
            Form::Deadline { lower, .. } => self.bound_units(lower).map(Some),
            _ => Ok(None),
        }
    }

    fn local(&mut self, model: &ModelSimState) -> Step<Rc<Local>> {
        let (key, _) = self
            .system
            .state_key(model, fsl_foundation::IdentityLimits::default())
            .map_err(|_| fault("state-key-encodes"))?;
        if let Some(known) = self.locals.get(&key) {
            return Ok(Rc::clone(known));
        }
        super::observe_relation_construction();
        let actions = match self.system.actions(model) {
            Ok(Ok(actions)) => actions,
            Ok(Err(undecided)) => {
                return Err(ProductRefusal::UndecidedSuccessor(undecided).into());
            }
            Err(stop) => return Err(ProductRefusal::Expansion(stop).into()),
        };
        let steps = self.timed.guarded_steps(model).map_err(timed_refusal)?;
        let invariants = self.timed.invariants(model).map_err(timed_refusal)?;
        let urgent = self.timed.is_urgent(model).map_err(timed_refusal)?;
        let local = Rc::new(Local {
            actions,
            steps,
            quiescent: invariants.is_empty() && !urgent,
            invariants,
            urgent,
        });
        self.locals.insert(key, Rc::clone(&local));
        Ok(local)
    }

    /// The state after one more digital unit, or `None` when the delay is
    /// not admissible (urgent, or a time invariant fails after it).
    fn unit_delay(&self, local: &Local, state: &DState) -> Step<Option<(Vec<u64>, Option<u64>)>> {
        if local.urgent {
            return Ok(None);
        }
        let clocks: Vec<u64> = state
            .clocks
            .iter()
            .zip(&self.caps)
            .map(|(value, cap)| if value < cap { value + 1 } else { *value })
            .collect();
        let valuation = self.valuation(&clocks)?;
        if !local.invariants.iter().all(|bound| bound.holds(&valuation)) {
            return Ok(None);
        }
        let deadline = match (state.deadline, self.deadline_bound()?) {
            (Some(t), Some(bound)) => Some((t + 1).min(bound + 1)),
            _ => None,
        };
        Ok(Some((clocks, deadline)))
    }

    /// The scheduled identities enabled at `clocks` with their enabled
    /// post-states (ADR-026 TS-2 with integer clocks).
    fn enabled<'local>(
        &mut self,
        local: &'local Local,
        model: &ModelSimState,
        clocks: &[u64],
    ) -> Step<Vec<Enabled<'local>>> {
        let valuation = self.valuation(clocks)?;
        let mut found = Vec::new();
        for action in &local.actions {
            let mut draws = Vec::with_capacity(action.draws.len());
            for draw in &action.draws {
                let mut posts = Vec::new();
                for (transition, post) in &draw.successors {
                    let Some(step) = local
                        .steps
                        .iter()
                        .find(|step| step.transition() == transition)
                    else {
                        continue;
                    };
                    if !step
                        .parts()
                        .iter()
                        .any(|part| part.iter().all(|constraint| constraint.holds(&valuation)))
                    {
                        continue;
                    }
                    let reset = step.reset(&valuation);
                    let after = self.local(post)?;
                    if !after.invariants.iter().all(|bound| bound.holds(&reset)) {
                        continue;
                    }
                    posts.push((post.clone(), self.digital(&reset)?));
                }
                draws.push(EnabledDraw {
                    transition: &draw.transition,
                    random: draw.random.clone(),
                    probability: draw.probability.clone(),
                    rewards: draw.rewards.clone(),
                    posts,
                });
            }
            if draws.iter().all(|draw| draw.posts.is_empty()) {
                continue;
            }
            if let Some(blocked) = draws.iter().find(|draw| draw.posts.is_empty()) {
                let (_, state) = self
                    .system
                    .state_key(model, fsl_foundation::IdentityLimits::default())
                    .map_err(|_| fault("state-key-encodes"))?;
                return Err(ProductRefusal::NotMarkov(Box::new(NotMarkov {
                    state,
                    transition: action.identity.clone(),
                    drawn_transition: blocked.transition.clone(),
                    random: blocked.random.clone(),
                    post_states: 0,
                    post_state_examples: Vec::new(),
                }))
                .into());
            }
            found.push(Enabled {
                identity: action.identity.clone(),
                draws,
            });
        }
        Ok(found)
    }

    /// Retain model-state evidence from the actual filtered draw. Filtering
    /// preserves the source draw's distinct canonical model-state order.
    fn not_markov_post_keys(&self, posts: &[(ModelSimState, Vec<u64>)]) -> Step<Vec<StateKey>> {
        let mut keys: Vec<StateKey> = Vec::with_capacity(posts.len().min(2));
        for (model, _) in posts.iter().take(2) {
            self.poll()?;
            let (_, key) = self
                .system
                .state_key(model, fsl_foundation::IdentityLimits::default())
                .map_err(|_| fault("state-key-encodes"))?;
            if keys.last().is_some_and(|previous| previous >= &key) {
                return Err(fault("filtered-post-model-keys-are-distinct-and-ordered").into());
            }
            keys.push(key);
        }
        Ok(keys)
    }

    // -- keys and truths ---------------------------------------------------

    #[implements("FR-196-AC-6", "FR-196-AC-10")]
    fn key_of(
        &self,
        state: &DState,
        binding: usize,
        monitor: Option<MonitorState>,
        pending: Option<Pending>,
    ) -> Step<ProbKey> {
        let certificate_state = self.certificate_key(state, binding, monitor, pending.as_ref())?;
        Ok(ProbKey {
            binding: self.bindings[binding].0.clone(),
            model: certificate_state.as_bytes().to_vec(),
            monitor,
            accumulator: None,
            pending,
            certificate_state: Some(certificate_state),
        })
    }

    #[implements("FR-196-AC-6", "FR-196-AC-10")]
    fn certificate_key(
        &self,
        state: &DState,
        binding: usize,
        monitor: Option<MonitorState>,
        pending: Option<&Pending>,
    ) -> Step<ProbabilityProductKey> {
        if pending.is_some() {
            return Err(fault("pending-draw-certificate-key-is-not-admitted").into());
        }
        let (discrete, _) = self
            .system
            .state_key(&state.model, fsl_foundation::IdentityLimits::default())
            .map_err(|_| fault("certificate-discrete-state-key-encodes"))?;
        let discrete: JsonValue = serde_json::from_slice(&discrete)
            .map_err(|_| fault("certificate-discrete-state-key-is-json"))?;
        let binding = self.bindings[binding]
            .1
            .as_ref()
            .map(|reference| {
                let text = fsl_eval::simulation::model::reference_canonical_text(reference)
                    .map_err(|_| fault("certificate-binding-encodes"))?;
                serde_json::from_str(&text).map_err(|_| fault("certificate-binding-is-json"))
            })
            .transpose()?;
        let unit = json!([
            "declared",
            fsl_foundation::node_wire::node_key_to_json(&self.timed.source().unit_node)
                .map_err(|_| fault("certificate-time-unit-encodes"))?
        ]);
        let clocks = self
            .timed
            .places()
            .iter()
            .zip(&state.clocks)
            .zip(&self.caps)
            .map(|((place, value), cap)| {
                let field = self
                    .timed
                    .model()
                    .clock(place.clock())
                    .ok_or_else(|| fault("certificate-clock-field-exists"))?;
                let owner = field.owner();
                let owner = owner.original();
                let declaring = fsl_semantics::model::key::model_node_key(
                    owner.package(),
                    "model",
                    "object_type",
                    owner.node(),
                )
                .map_err(|_| fault("certificate-clock-type-encodes"))?;
                let declaring = fsl_foundation::node_wire::node_key_to_json(&declaring)
                    .map_err(|_| fault("certificate-clock-type-encodes"))?;
                let receiver =
                    fsl_eval::simulation::model::reference_canonical_text(place.object())
                        .map_err(|_| fault("certificate-clock-receiver-encodes"))?;
                let receiver: JsonValue = serde_json::from_str(&receiver)
                    .map_err(|_| fault("certificate-clock-receiver-is-json"))?;
                let identity = json!([declaring, field.name(), receiver]);
                Ok(if self.subdivisions == 1 {
                    ClockKey::Digital(DigitalClockKey {
                        identity,
                        value: *value,
                        cap: *cap,
                    })
                } else {
                    ClockKey::Region(RegionClockKey {
                        identity,
                        value: *value,
                        cap: *cap,
                    })
                })
            })
            .collect::<Result<Vec<_>, ProductRefusal>>()?;
        let measure = match (&self.form, self.deadline_bound()?) {
            (Form::Elapsed { .. }, Some(limit)) => Some((limit, state.deadline)),
            _ => None,
        };
        let deadline = match state.deadline.filter(|_| measure.is_none()) {
            Some(elapsed) => {
                let limit = self
                    .deadline_bound()?
                    .ok_or_else(|| fault("certificate-deadline-bound-exists"))?;
                Some(if self.subdivisions == 1 {
                    DeadlineKey::Digital(DigitalDeadlineKey { limit, elapsed })
                } else {
                    DeadlineKey::Region(RegionDeadlineKey { limit, elapsed })
                })
            }
            None => None,
        };
        let monitor = monitor.map(|monitor| match monitor {
            MonitorState::Deadline { kind, phase } => json!([
                "deadline",
                match kind {
                    DeadlineKind::Eventually => "eventually",
                    DeadlineKind::Always => "always",
                    DeadlineKind::Until => "until",
                },
                match phase {
                    DeadlinePhase::Pending => "pending",
                    DeadlinePhase::Accept => "accept",
                    DeadlinePhase::Reject => "reject",
                }
            ]),
            MonitorState::TimedFormula { root, nested } => {
                let encode = |operator: TimedOperatorState| {
                    json!([
                        operator.occurrence.to_string(),
                        match operator.kind {
                            DeadlineKind::Eventually => "eventually",
                            DeadlineKind::Always => "always",
                            DeadlineKind::Until => "until",
                        },
                        operator.lower.to_string(),
                        operator.upper.to_string(),
                        operator.elapsed.to_string(),
                        match operator.phase {
                            DeadlinePhase::Pending => "pending",
                            DeadlinePhase::Accept => "accept",
                            DeadlinePhase::Reject => "reject",
                        }
                    ])
                };
                json!(["timed-formula", encode(root), nested.map(encode)])
            }
            MonitorState::Measure { phase, .. } => json!([
                "measure",
                null,
                match phase {
                    MeasurePhase::Inactive => "inactive",
                    MeasurePhase::Active => "active",
                    MeasurePhase::Complete => "complete",
                    MeasurePhase::Censored => "censored",
                    MeasurePhase::Unactivated => "unactivated",
                }
            ]),
            _ => JsonValue::Null,
        });
        let tail = match state.tail {
            Tail::Running => DigitalTail::Running,
            Tail::IdleReady => DigitalTail::IdleReady,
            Tail::IdleDelayed => DigitalTail::IdleDelayed,
        };
        let key = if self.subdivisions == 1 {
            digital_probability_product_key(DigitalProductKeyInput {
                binding,
                discrete,
                unit,
                scale: self.base_scale,
                clocks: clocks
                    .into_iter()
                    .map(|clock| match clock {
                        ClockKey::Digital(clock) => Ok(clock),
                        ClockKey::Region(_) => Err(fault("digital-route-clock-key-is-digital")),
                    })
                    .collect::<Result<Vec<_>, ProductRefusal>>()?,
                deadline: deadline
                    .map(|deadline| match deadline {
                        DeadlineKey::Digital(deadline) => Ok(deadline),
                        DeadlineKey::Region(_) => {
                            Err(fault("digital-route-deadline-key-is-digital"))
                        }
                    })
                    .transpose()?,
                tail,
                monitor,
                accumulator: None,
                pending: None,
                elapsed: measure,
            })
        } else {
            let region_clocks = clocks
                .into_iter()
                .map(|clock| match clock {
                    ClockKey::Region(clock) => Some(clock),
                    ClockKey::Digital(_) => None,
                })
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| fault("region-clock-key-is-present"))?;
            let boundary_region = matches!(&self.form, Form::MeanTime { .. });
            let corner = if boundary_region {
                let mut coordinates = region_clocks
                    .iter()
                    .map(|clock| {
                        quire_canonical::to_vec(
                            &clock.identity,
                            quire_canonical::Limits::new(u64::MAX),
                        )
                        .map(|identity| (identity, clock.value.to_string()))
                        .map_err(|_| fault("boundary-region-corner-identity-encodes"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                coordinates.sort_by(|left, right| left.0.cmp(&right.0));
                Some(
                    coordinates
                        .into_iter()
                        .map(|(_, coordinate)| coordinate)
                        .collect(),
                )
            } else {
                None
            };
            region_probability_product_key(RegionProductKeyInput {
                binding,
                discrete,
                unit,
                scale: self.base_scale,
                subdivisions: self.subdivisions,
                clocks: region_clocks,
                deadline: deadline
                    .map(|deadline| match deadline {
                        DeadlineKey::Region(deadline) => Ok(deadline),
                        DeadlineKey::Digital(_) => {
                            Err(fault("region-route-deadline-key-is-region"))
                        }
                    })
                    .transpose()?,
                tail,
                monitor,
                accumulator: None,
                pending: None,
                elapsed: measure,
                kind: if boundary_region {
                    RegionKeyKind::BoundaryRegion
                } else {
                    RegionKeyKind::Region
                },
                corner,
            })
        }
        .map_err(|_| fault("certificate-timed-product-key-encodes"))?;
        Ok(key)
    }

    fn truths(&mut self, state: &ModelSimState, binding: usize) -> Step<Result<Vec<bool>, Hit>> {
        let (model, state_key) = self
            .system
            .state_key(state, fsl_foundation::IdentityLimits::default())
            .map_err(|_| fault("state-key-encodes"))?;
        if let Some(known) = self.readings.get(&(binding, model.clone())) {
            return Ok(Ok(known.clone()));
        }
        let over = self.bindings[binding].1.as_ref();
        let predicates: Vec<(&CheckedPredicate, Option<&ObjectReference>)> = self
            .form
            .reads()
            .iter()
            .map(|(_, predicate)| (*predicate, over))
            .collect();
        let readings = self
            .system
            .state_predicates(state, &predicates)
            .map_err(|stop| Halt::Refusal(ProductRefusal::Expansion(stop)))?;
        let mut truths = Vec::with_capacity(readings.len());
        for ((read, _), reading) in self.form.reads().iter().zip(readings) {
            match reading {
                PredicateReading::True => truths.push(true),
                PredicateReading::False => truths.push(false),
                PredicateReading::Undefined { cause, .. } => {
                    return Ok(Err(Hit {
                        read: *read,
                        cause,
                        state_key,
                    }));
                }
                PredicateReading::Refused(outcome) => {
                    return Err(ProductRefusal::PredicateRefused(outcome).into());
                }
            }
        }
        self.readings.insert((binding, model), truths.clone());
        Ok(Ok(truths))
    }

    fn binding_of(&self, at: usize) -> Step<usize> {
        self.bindings
            .iter()
            .position(|(instance, _)| *instance == self.nodes[at].key.binding)
            .ok_or_else(|| fault("node-binding-is-a-binding").into())
    }

    // -- the deadline event --------------------------------------------------

    /// The event's phase after reading one letter.
    fn read_letter(
        kind: DeadlineKind,
        phase: DeadlinePhase,
        truths: &[bool],
        elapsed: u64,
        lower: u64,
    ) -> DeadlinePhase {
        if phase != DeadlinePhase::Pending {
            return phase;
        }
        match (kind, truths) {
            (DeadlineKind::Eventually, [target]) if elapsed >= lower && *target => {
                DeadlinePhase::Accept
            }
            (DeadlineKind::Always, [target]) if elapsed >= lower && !*target => {
                DeadlinePhase::Reject
            }
            (DeadlineKind::Until, [from, until]) => {
                if elapsed >= lower && *until {
                    DeadlinePhase::Accept
                } else if !*from {
                    DeadlinePhase::Reject
                } else {
                    DeadlinePhase::Pending
                }
            }
            _ => DeadlinePhase::Pending,
        }
    }

    #[implements("FR-204-AC-21")]
    fn deadline_start(&self, truths: &[bool], state: &mut DState) -> Step<Option<MonitorState>> {
        match &self.form {
            Form::Elapsed { .. } => {
                let mut phase = MeasurePhase::Inactive;
                if truths.first().copied().unwrap_or(false) {
                    state.deadline = Some(0);
                    phase = if truths.get(1).copied().unwrap_or(false) {
                        MeasurePhase::Complete
                    } else {
                        MeasurePhase::Active
                    };
                }
                Ok(Some(MonitorState::Measure {
                    remaining: 0,
                    phase,
                }))
            }
            Form::Deadline { kind, .. } => {
                let lower = self.deadline_lower()?.unwrap_or(0);
                Ok(Some(MonitorState::Deadline {
                    kind: *kind,
                    phase: Self::read_letter(
                        *kind,
                        DeadlinePhase::Pending,
                        truths,
                        state.deadline.unwrap_or(0),
                        lower,
                    ),
                }))
            }
            Form::NestedDeadline(deadline) => {
                let mut root = TimedOperatorState {
                    occurrence: deadline.outer_occurrence,
                    kind: deadline.outer_kind,
                    lower: self.bound_units(&deadline.outer_lower)?,
                    upper: self.bound_units(&deadline.outer_upper)?,
                    elapsed: 0,
                    phase: DeadlinePhase::Pending,
                };
                let mut child = TimedOperatorState {
                    occurrence: deadline.inner_occurrence,
                    kind: deadline.inner_kind,
                    lower: self.bound_units(&deadline.inner_lower)?,
                    upper: self.bound_units(&deadline.inner_upper)?,
                    elapsed: 0,
                    phase: DeadlinePhase::Pending,
                };
                let elapsed = state.deadline.unwrap_or(0);
                fsl_eval::probabilistic::advance_nested_timed_deadline(
                    &mut root,
                    &mut child,
                    truths.first().copied().unwrap_or(false),
                    elapsed,
                );
                Ok(Some(MonitorState::TimedFormula {
                    root,
                    nested: Some(child),
                }))
            }
            Form::MeanTime { .. } | Form::Labels(_) => Ok(None),
        }
    }

    /// The letters a pure delay adds to a move that took no discrete step.
    /// A delay is not a predicate position (ADR-026 TS-3), so a single
    /// operator's event reads none and only its deadline clock moves. The
    /// nested monitor reads `truths` once at each instant a delay reaches.
    #[implements("FR-204-AC-21")]
    fn delay_letters(&self, truths: &[bool]) -> Vec<Vec<bool>> {
        match self.form {
            Form::NestedDeadline(_) => vec![truths.to_vec()],
            Form::Deadline { .. }
            | Form::Elapsed { .. }
            | Form::MeanTime { .. }
            | Form::Labels(_) => Vec::new(),
        }
    }

    /// The control after the source's delay and then `letters`: the
    /// predicate positions that follow the delay.
    #[implements("FR-204-AC-21")]
    fn control_after(
        &mut self,
        source: Option<MonitorState>,
        reached: &mut DState,
        letters: &[Vec<bool>],
    ) -> Step<Option<MonitorState>> {
        if matches!(self.form, Form::Elapsed { .. }) {
            let Some(MonitorState::Measure { mut phase, .. }) = source else {
                return Ok(None);
            };
            let bound = self.deadline_bound()?;
            if phase == MeasurePhase::Active {
                if let (Some(t), Some(bound)) = (reached.deadline, bound) {
                    if t > bound {
                        phase = MeasurePhase::Censored;
                    }
                }
            }
            for truths in letters {
                match phase {
                    MeasurePhase::Inactive if truths.first().copied().unwrap_or(false) => {
                        reached.deadline = Some(0);
                        phase = if truths.get(1).copied().unwrap_or(false) {
                            MeasurePhase::Complete
                        } else {
                            MeasurePhase::Active
                        };
                    }
                    MeasurePhase::Active if truths.get(1).copied().unwrap_or(false) => {
                        phase = MeasurePhase::Complete;
                    }
                    _ => {}
                }
            }
            return Ok(Some(MonitorState::Measure {
                remaining: 0,
                phase,
            }));
        }
        if let Some(MonitorState::TimedFormula {
            mut root,
            mut nested,
        }) = source
        {
            let Some(mut child) = nested else {
                return Err(fault("nested-timed-formula-retains-inner-occurrence").into());
            };
            let elapsed = reached.deadline.unwrap_or(root.elapsed);
            for truths in letters {
                fsl_eval::probabilistic::advance_nested_timed_deadline(
                    &mut root,
                    &mut child,
                    truths.first().copied().unwrap_or(false),
                    elapsed,
                );
            }
            nested = Some(child);
            return Ok(Some(MonitorState::TimedFormula { root, nested }));
        }
        let Some(MonitorState::Deadline { kind, phase }) = source else {
            return Ok(None);
        };
        let bound = self.deadline_bound()?;
        let mut phase = phase;
        let lower = self.deadline_lower()?.unwrap_or(0);
        let elapsed = reached.deadline.unwrap_or(0);
        if phase == DeadlinePhase::Pending {
            if let (Some(t), Some(bound)) = (reached.deadline, bound) {
                if t > bound {
                    phase = match kind {
                        DeadlineKind::Always => DeadlinePhase::Accept,
                        DeadlineKind::Eventually | DeadlineKind::Until => DeadlinePhase::Reject,
                    };
                }
            }
        }
        for truths in letters {
            phase = Self::read_letter(kind, phase, truths, elapsed, lower);
        }
        Ok(Some(MonitorState::Deadline { kind, phase }))
    }

    // -- exploration ---------------------------------------------------------

    #[implements("FR-204-AC-1")]
    fn explore(&mut self) -> Step<Vec<(u32, Option<ModelInstance>, ProbStateId)>> {
        self.poll()?;
        let deadline = if matches!(self.form, Form::Elapsed { .. }) {
            None
        } else {
            self.deadline_bound()?.map(|_| 0)
        };
        let mut starts = Vec::new();
        for (position, initial) in self.timed.initial().into_iter().enumerate() {
            let at = u32::try_from(position).map_err(|_| fault("initial-states-count"))?;
            let state = DState {
                model: initial.discrete().clone(),
                clocks: self.digital(initial.clocks())?,
                deadline,
                tail: Tail::Running,
            };
            for binding in 0..self.bindings.len() {
                let truths = match self.truths(&state.model, binding)? {
                    Ok(truths) => truths,
                    Err(hit) => {
                        let monitor = match &self.form {
                            Form::Deadline { kind, .. } => Some(MonitorState::Deadline {
                                kind: *kind,
                                phase: DeadlinePhase::Pending,
                            }),
                            Form::NestedDeadline(_) => {
                                let mut state = state.clone();
                                self.deadline_start(&[], &mut state)?
                            }
                            Form::Elapsed { .. } => Some(MonitorState::Measure {
                                remaining: 0,
                                phase: MeasurePhase::Inactive,
                            }),
                            Form::MeanTime { .. } | Form::Labels(_) => None,
                        };
                        let key = self.key_of(&state, binding, monitor, None)?;
                        return Err(self.undefined(None, None, None, at, key, hit));
                    }
                };
                let mut state = state.clone();
                let monitor = self.deadline_start(&truths, &mut state)?;
                let key = self.key_of(&state, binding, monitor, None)?;
                self.admit_state(&key)?;
                super::observe_target_construction();
                let target = Target {
                    key,
                    state,
                    truths,
                    resolution: None,
                };
                let id = self.intern(target, None, None, at)?;
                starts.push((at, self.bindings[binding].0.clone(), ProbStateId(id)));
            }
        }
        while let Some(at) = self.queue.pop_front() {
            self.poll()?;
            self.expand(at)?;
        }
        Ok(starts)
    }

    fn intern(
        &mut self,
        target: Target,
        parent: Option<usize>,
        step: Option<TraceStep>,
        initial: u32,
    ) -> Step<usize> {
        if let Some(known) = self.index.get(&target.key) {
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
        let decided = target.key.monitor.is_some_and(MonitorState::is_decided);
        self.pending_keys.remove(&target.key);
        self.index.insert(target.key.clone(), id);
        self.nodes.push(Node {
            key: target.key,
            state: target.state,
            truths: target.truths,
            parent: parent.map(|parent| (parent, step)),
            initial,
            resolution: target.resolution,
        });
        if !decided {
            self.queue.push_back(id);
        }
        Ok(id)
    }

    /// Reserve distinct target keys across the whole pending expansion.
    fn admit_state(&mut self, key: &ProbKey) -> Step<()> {
        if self.index.contains_key(key) || self.pending_keys.contains(key) {
            return Ok(());
        }
        if self.nodes.len().saturating_add(self.pending_keys.len()) >= self.max_states {
            return Err(Self::stopped(
                ModelCheckLimit::ExactProbMaxStates,
                self.max_states,
            ));
        }
        self.pending_keys.insert(key.clone());
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
                None => chosen = Some(self.nodes[at].key.clone()),
            }
            cursor = Some(*before);
        }
        steps.reverse();
        let probability = steps
            .iter()
            .fold(one(), |total, step| total.mul(&step.probability));
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

    /// The target reached from node `at` by `reached` after the letters of
    /// the discrete steps listed in `landed` (model states, in order).
    #[implements("FR-204-AC-21")]
    fn arrive(
        &mut self,
        at: usize,
        mut reached: DState,
        landed: &[ModelSimState],
        taken: Option<&TraceStep>,
        planned: &[Planned],
        reuse: impl FnOnce(&ProbKey) -> bool,
    ) -> Step<Target> {
        let binding = self.binding_of(at)?;
        let mut letters = Vec::with_capacity(landed.len());
        for model in landed {
            match self.truths(model, binding)? {
                Ok(truths) => letters.push(truths),
                Err(hit) => {
                    let source = self.nodes[at].key.monitor;
                    let key = self.key_of(&reached, binding, source, None)?;
                    let resolved = taken.is_none().then(|| key.clone());
                    return Err(self.undefined(
                        Some(at),
                        taken,
                        resolved,
                        self.nodes[at].initial,
                        key,
                        hit,
                    ));
                }
            }
        }
        let truths = match letters.last() {
            Some(last) => last.clone(),
            None => match self.truths(&reached.model, binding)? {
                Ok(truths) => truths,
                Err(_) => return Err(fault("a-pure-delay-reads-what-the-parent-read").into()),
            },
        };
        if letters.is_empty() {
            letters = self.delay_letters(&truths);
        }
        let source = self.nodes[at].key.monitor;
        let monitor = self.control_after(source, &mut reached, &letters)?;
        let key = self.key_of(&reached, binding, monitor, None)?;
        if !reuse(&key) {
            self.admit_edge(planned.len())?;
        }
        self.admit_state(&key)?;
        super::observe_target_construction();
        Ok(Target {
            key,
            state: reached,
            truths,
            resolution: None,
        })
    }

    fn expand(&mut self, at: usize) -> Step<()> {
        // Decided states never enter the queue. A queued state needs an edge
        // allowance before querying its successor relation.
        self.admit_edge(0)?;
        self.pending_keys.clear();
        let planned = match self.nodes[at].resolution.take() {
            Some(resolution) => self.resolve(at, &resolution)?,
            None => self.moves(at)?,
        };
        for edge in planned {
            if self.edges.len() >= self.max_transitions {
                return Err(Self::stopped(
                    ModelCheckLimit::ExactProbMaxTransitions,
                    self.max_transitions,
                ));
            }
            let zero_delay = edge.zero_delay;
            let to = self.intern(edge.target, Some(at), edge.step, self.nodes[at].initial)?;
            let mut rewards = edge.rewards;
            if matches!(self.form, Form::MeanTime { .. }) {
                let name = Identifier::new(ELAPSED_REWARD)
                    .map_err(|_| fault("elapsed-reward-identifier-is-valid"))?;
                let magnitude =
                    Rational::new(Integer::from(edge.elapsed_ticks), Integer::from(self.scale))
                        .map_err(|_| fault("elapsed-reward-scale-is-positive"))?;
                let unit = filament_exact::UnitId::declared(self.timed.source().unit_node.clone());
                rewards.push((name, Value::Quantity(Quantity::new(magnitude, unit))));
            }
            self.edges.push(ProbEdge {
                from: ProbStateId(at),
                choice: edge.choice,
                action: edge.action,
                draw: edge.draw,
                to: ProbStateId(to),
                probability: edge.probability,
                elapsed: Rational::new(
                    Integer::from(edge.elapsed_ticks),
                    Integer::from(self.scale),
                )
                .map_err(|_| fault("elapsed-reward-scale-is-positive"))?,
                rewards,
            });
            self.zero_delay.push(zero_delay);
        }
        self.pending_keys.clear();
        Ok(())
    }

    fn resolve(&mut self, at: usize, resolution: &Resolution) -> Step<Vec<Planned>> {
        let mut planned = Vec::new();
        for post in &resolution.successors {
            self.admit_edge(planned.len())?;
            let model = post.model.clone();
            let target = self.arrive(at, post.clone(), &[model], None, &planned, |_| false)?;
            super::observe_edge_construction();
            planned.push(Planned {
                choice: ProductAction::PostState(Box::new(target.key.clone())),
                action: None,
                draw: None,
                target,
                probability: one(),
                rewards: self.system.zero_rewards(),
                elapsed_ticks: 0,
                step: None,
                zero_delay: true,
            });
        }
        Ok(planned)
    }

    fn moves(&mut self, at: usize) -> Step<Vec<Planned>> {
        let state = self.nodes[at].state.clone();
        let local = self.local(&state.model)?;
        match state.tail {
            Tail::Running => match self.workload {
                None => self.scheduler_moves(at, &state, &local),
                Some(workload) => self.workload_moves(at, &state, &local, workload),
            },
            Tail::IdleReady | Tail::IdleDelayed => self.idle_moves(at, &state),
        }
    }

    /// A pure delay move to `clocks`/`deadline` with the tail `tail`.
    #[implements("FR-204-AC-5")]
    fn delay_move(&mut self, movement: DelayMove<'_>) -> Step<Planned> {
        let DelayMove {
            at,
            state,
            choice,
            delayed,
            tail,
            elapsed_ticks,
            planned,
            omit_unchanged,
        } = movement;
        let reached = DState {
            model: state.model.clone(),
            clocks: delayed.0,
            deadline: delayed.1,
            tail,
        };
        let taken = TraceStep {
            action: None,
            draw: None,
            probability: one(),
        };
        let current = self.nodes[at].key.clone();
        let target = self.arrive(at, reached, &[], Some(&taken), planned, |key| {
            omit_unchanged && *key == current
        })?;
        if !omit_unchanged || target.key != current {
            super::observe_edge_construction();
        }
        Ok(Planned {
            choice,
            action: None,
            draw: None,
            target,
            probability: one(),
            rewards: self.system.zero_rewards(),
            elapsed_ticks,
            step: Some(taken),
            zero_delay: false,
        })
    }

    /// The delay and observation moves every running state offers.
    fn running_delays(
        &mut self,
        at: usize,
        state: &DState,
        local: &Local,
        planned: &mut Vec<Planned>,
    ) -> Step<()> {
        if let Some(first_delay) = self.unit_delay(local, state)? {
            if self.subdivisions == 1 {
                let unit = self.delay_move(DelayMove {
                    at,
                    state,
                    choice: ProductAction::UnitDelay,
                    delayed: first_delay.clone(),
                    tail: Tail::Running,
                    elapsed_ticks: 1,
                    planned,
                    omit_unchanged: true,
                })?;
                if unit.target.key != self.nodes[at].key {
                    planned.push(unit);
                }
            } else {
                let current_key = self.nodes[at].key.clone();
                let mut cursor = state.clone();
                for elapsed_ticks in 1..=self.subdivisions {
                    self.poll()?;
                    let Some(delayed) = self.unit_delay(local, &cursor)? else {
                        break;
                    };
                    let unit = self.delay_move(DelayMove {
                        at,
                        state: &cursor,
                        choice: ProductAction::UnitDelay,
                        delayed: delayed.clone(),
                        tail: Tail::Running,
                        elapsed_ticks,
                        planned,
                        omit_unchanged: true,
                    })?;
                    if unit.target.key != current_key {
                        planned.push(unit);
                        break;
                    }
                    if delayed.0 == cursor.clocks && delayed.1 == cursor.deadline {
                        break;
                    }
                    cursor.clocks = delayed.0;
                    cursor.deadline = delayed.1;
                }
            }
            if local.quiescent {
                planned.push(self.delay_move(DelayMove {
                    at,
                    state,
                    choice: ProductAction::EnterIdleDelay,
                    delayed: first_delay,
                    tail: Tail::IdleDelayed,
                    elapsed_ticks: 1,
                    planned,
                    omit_unchanged: false,
                })?);
            }
        }
        Ok(())
    }

    fn idle_moves(&mut self, at: usize, state: &DState) -> Step<Vec<Planned>> {
        let local = self.local(&state.model)?;
        let mut planned = Vec::new();
        if let Some(delayed) = self.unit_delay(&local, state)? {
            let delay = self.delay_move(DelayMove {
                at,
                state,
                choice: ProductAction::IdleDelay,
                delayed,
                tail: Tail::IdleDelayed,
                elapsed_ticks: 1,
                planned: &planned,
                omit_unchanged: self.subdivisions != 1,
            })?;
            if self.subdivisions == 1 || delay.target.key != self.nodes[at].key {
                planned.push(delay);
            }
        }
        if state.tail == Tail::IdleDelayed {
            let reached = DState {
                tail: Tail::IdleReady,
                ..state.clone()
            };
            let taken = TraceStep {
                action: None,
                draw: None,
                probability: one(),
            };
            let landed = [state.model.clone()];
            let target = self.arrive(at, reached, &landed, Some(&taken), &planned, |_| false)?;
            super::observe_edge_construction();
            planned.push(Planned {
                choice: ProductAction::IdleObserve,
                action: None,
                draw: None,
                target,
                probability: one(),
                rewards: self.system.zero_rewards(),
                elapsed_ticks: 0,
                step: Some(taken),
                zero_delay: false,
            });
        }
        self.decisions = true;
        Ok(planned)
    }

    /// Over every scheduler: the delays and each enabled identity's draws.
    fn scheduler_moves(
        &mut self,
        at: usize,
        state: &DState,
        local: &Rc<Local>,
    ) -> Step<Vec<Planned>> {
        let mut planned = Vec::new();
        let enabled = self.enabled(local, &state.model, &state.clocks)?;
        self.every_discrete_moves(at, state, local, &enabled, &mut planned)?;
        if planned.is_empty() {
            self.time_locked.push(at);
        }
        self.decisions = true;
        Ok(planned)
    }

    /// Under every scheduler, given delays are drawn independently and the
    /// scheduler chooses among identities tied at the least sampled delay.
    /// Free-delay identities remain ordinary scheduler choices and are
    /// handled by the running clock moves until a discrete race is admitted.
    #[implements("FR-204-AC-21")]
    fn every_discrete_moves(
        &mut self,
        at: usize,
        state: &DState,
        local: &Rc<Local>,
        enabled: &[Enabled<'_>],
        planned: &mut Vec<Planned>,
    ) -> Step<()> {
        let trajectory = self.trajectory(state)?;
        let windows = self.windows(&trajectory)?;
        let mut racers = Vec::new();
        for window in &windows {
            let Some(support) = self.given(&window.identity.operation) else {
                continue;
            };
            let support = support
                .into_iter()
                .map(|(delay, weight)| Ok((self.bound_units(&delay)?, weight)))
                .collect::<Step<Vec<_>>>()?;
            let inside = support
                .into_iter()
                .filter(|(delay, _)| window.contains(*delay, trajectory.states.len() as u64 - 1))
                .collect::<Vec<_>>();
            let total = inside
                .iter()
                .fold(zero(), |sum, (_, weight)| sum.add(weight));
            if total.is_zero() {
                continue;
            }
            let conditioned = inside
                .into_iter()
                .map(|(delay, weight)| {
                    let probability = weight
                        .div(&total)
                        .ok_or_else(|| fault("conditioned-weights-sum"))?;
                    Ok((delay, probability))
                })
                .collect::<Step<Vec<_>>>()?;
            racers.push((window.identity.clone(), conditioned));
        }
        if racers.is_empty() {
            self.running_delays(at, state, local, planned)?;
            for action in enabled {
                if self.given(&action.identity.operation).is_none() {
                    self.identity_edges(at, state, action, one(), planned)?;
                }
            }
            return Ok(());
        }
        let representative = racers
            .first()
            .map(|(identity, _)| identity.clone())
            .ok_or_else(|| fault("a-race-has-a-racer"))?;
        for draw in Odometer::new(racers.iter().map(|(_, support)| support.len())) {
            self.poll()?;
            let mut probability = one();
            let mut delays = Vec::with_capacity(racers.len());
            for ((identity, support), index) in racers.iter().zip(&draw) {
                let (delay, weight) = support
                    .get(*index)
                    .ok_or_else(|| fault("odometer-index-in-support"))?;
                probability = probability.mul(weight);
                delays.push((identity.clone(), *delay));
            }
            let least = delays
                .iter()
                .map(|(_, delay)| *delay)
                .min()
                .ok_or_else(|| fault("a-race-has-a-racer"))?;
            let mut tied = delays
                .iter()
                .filter(|(_, delay)| *delay == least)
                .map(|(identity, _)| identity.clone())
                .collect::<Vec<_>>();
            let reached = u64::try_from(trajectory.states.len().saturating_sub(1))
                .map_err(|_| fault("delay-count"))?;
            for window in windows
                .iter()
                .filter(|window| window.given.is_none() && window.contains(least, reached))
            {
                if !tied.contains(&window.identity) {
                    tied.push(window.identity.clone());
                }
            }
            let index = usize::try_from(least).map_err(|_| fault("delay-count"))?;
            let last = trajectory.states.len().saturating_sub(1);
            let start = trajectory
                .states
                .get(index.min(last))
                .cloned()
                .ok_or_else(|| fault("race-delay-is-reachable"))?;
            let delayed = TraceStep {
                action: None,
                draw: None,
                probability: probability.clone(),
            };
            // Inspect the monitor before allocating a terminal target. A
            // nonterminal delay is only the start of the tied chain.
            let mut probe = start.clone();
            let letters = self.delay_letters(&self.nodes[at].truths);
            let monitor = self.control_after(self.nodes[at].key.monitor, &mut probe, &letters)?;
            if matches!(
                monitor,
                Some(
                    MonitorState::Deadline {
                        phase: DeadlinePhase::Accept | DeadlinePhase::Reject,
                        ..
                    } | MonitorState::TimedFormula {
                        root: TimedOperatorState {
                            phase: DeadlinePhase::Accept | DeadlinePhase::Reject,
                            ..
                        },
                        ..
                    }
                )
            ) {
                let terminal =
                    self.arrive(at, start.clone(), &[], Some(&delayed), planned, |_| false)?;
                super::observe_edge_construction();
                planned.push(Planned {
                    choice: ProductAction::Identity(representative.clone()),
                    action: None,
                    draw: None,
                    target: terminal,
                    probability,
                    rewards: self.system.zero_rewards(),
                    elapsed_ticks: least,
                    step: Some(delayed),
                    zero_delay: least == 0,
                });
                continue;
            }
            for first in 0..tied.len() {
                let chosen = tied[first].clone();
                let mut order = Vec::with_capacity(tied.len());
                order.push(chosen.clone());
                order.extend(
                    tied.iter()
                        .enumerate()
                        .filter(|(position, _)| *position != first)
                        .map(|(_, identity)| identity.clone()),
                );
                self.chain_edges(
                    at,
                    &start,
                    &order,
                    (probability.clone(), least == 0, least, &chosen),
                    Edges { planned, scope: 0 },
                )?;
            }
        }
        for action in enabled {
            if self.given(&action.identity.operation).is_none() {
                self.identity_edges(at, state, action, one(), planned)?;
            }
        }
        Ok(())
    }

    /// The edges of one enabled identity at `state`, scaled by `share`.
    #[implements("FR-204-AC-3", "FR-204-AC-18", "FR-204-AC-19")]
    fn identity_edges(
        &mut self,
        at: usize,
        state: &DState,
        action: &Enabled<'_>,
        share: Rational,
        planned: &mut Vec<Planned>,
    ) -> Step<()> {
        let first = planned.len();
        for draw in &action.draws {
            let taken = TraceStep {
                action: Some(action.identity.clone()),
                draw: Some(draw.random.clone()),
                probability: share.mul(&draw.probability),
            };
            let probability = share.mul(&draw.probability);
            let post = |model: &ModelSimState, clocks: &[u64]| DState {
                model: model.clone(),
                clocks: clocks.to_vec(),
                deadline: state.deadline,
                tail: Tail::Running,
            };
            if let [(model, clocks)] = draw.posts.as_slice() {
                let reached = post(model, clocks);
                let target = self.arrive(
                    at,
                    reached,
                    std::slice::from_ref(model),
                    Some(&taken),
                    planned,
                    |key| {
                        planned[first..].iter().any(|edge| {
                            edge.target.key == *key && same_rewards(&edge.rewards, &draw.rewards)
                        })
                    },
                )?;
                if let Some(merged) = planned[first..].iter_mut().find(|edge| {
                    edge.target.key == target.key && same_rewards(&edge.rewards, &draw.rewards)
                }) {
                    merged.probability = merged.probability.add(&probability);
                    merged.draw = None;
                    merged.step = Some(TraceStep {
                        action: taken.action,
                        draw: None,
                        probability: merged.probability.clone(),
                    });
                    continue;
                }
                super::observe_edge_construction();
                planned.push(Planned {
                    choice: ProductAction::Identity(action.identity.clone()),
                    action: Some(action.identity.clone()),
                    draw: Some(draw.random.clone()),
                    target,
                    probability,
                    rewards: draw.rewards.clone(),
                    elapsed_ticks: 0,
                    step: Some(taken),
                    zero_delay: true,
                });
                continue;
            }
            // Several post-states: the scheduler resolves them at an
            // intermediate state keyed by the draw (SCH-2).
            let identity_bytes = self
                .system
                .transition_key(
                    &self
                        .local(&state.model)?
                        .actions
                        .iter()
                        .find(|candidate| candidate.identity == action.identity)
                        .and_then(|candidate| candidate.draws.first())
                        .and_then(|draw| draw.successors.first())
                        .map(|(transition, _)| transition.clone())
                        .ok_or_else(|| fault("pending-draw-has-a-transition"))?,
                )
                .map_err(|_| fault("transition-key-encodes"))?;
            let binding = self.binding_of(at)?;
            let monitor = self.nodes[at].key.monitor;
            let key = self.key_of(
                state,
                binding,
                monitor,
                Some(Pending {
                    identity: action.identity.clone(),
                    random: draw.random.clone(),
                    identity_bytes,
                }),
            )?;
            self.admit_edge(planned.len())?;
            self.admit_state(&key)?;
            super::observe_target_construction();
            let target = Target {
                key,
                state: state.clone(),
                truths: self.nodes[at].truths.clone(),
                resolution: Some(Resolution {
                    successors: draw
                        .posts
                        .iter()
                        .map(|(model, clocks)| post(model, clocks))
                        .collect(),
                }),
            };
            super::observe_edge_construction();
            planned.push(Planned {
                choice: ProductAction::Identity(action.identity.clone()),
                action: Some(action.identity.clone()),
                draw: Some(draw.random.clone()),
                target,
                probability,
                rewards: draw.rewards.clone(),
                elapsed_ticks: 0,
                step: Some(taken),
                zero_delay: true,
            });
        }
        Ok(())
    }

    // -- under a workload ----------------------------------------------------

    fn weight_of(&self, workload: &Workload, operation: &str) -> Step<Rational> {
        workload
            .weights()
            .iter()
            .find(|(weighted, _)| {
                self.system
                    .operation_text(weighted.declaring(), weighted.name())
                    == Some(operation)
            })
            .map(|(_, weight)| weight.clone())
            .ok_or_else(|| fault("workload-weighs-an-operation").into())
    }

    /// The delay distribution of `operation`, when it has a `discrete` one.
    fn given(&self, operation: &str) -> Option<Vec<(Rational, Rational)>> {
        self.system
            .input()
            .probabilistic
            .delays()
            .iter()
            .find(|delay| {
                let named = delay.operation();
                self.system.operation_text(named.declaring(), named.name()) == Some(operation)
            })
            .and_then(|delay| match delay.distribution() {
                DelayDistribution::Discrete(values) => Some(values.clone()),
                _ => None,
            })
    }

    fn trajectory(&mut self, state: &DState) -> Step<Trajectory> {
        let mut states = vec![state.clone()];
        let mut forever = false;
        let bound = self.max_states.max(1);
        while states.len() <= bound {
            let last = states
                .last()
                .cloned()
                .ok_or_else(|| fault("trajectory-starts"))?;
            let local = self.local(&last.model)?;
            let Some((clocks, deadline)) = self.unit_delay(&local, &last)? else {
                break;
            };
            if clocks == last.clocks && deadline == last.deadline {
                forever = true;
                break;
            }
            states.push(DState {
                clocks,
                deadline,
                ..last
            });
        }
        Ok(Trajectory { states, forever })
    }

    fn windows(&mut self, trajectory: &Trajectory) -> Step<Vec<Window>> {
        let mut windows: Vec<Window> = Vec::new();
        for (k, state) in trajectory.states.iter().enumerate() {
            let local = self.local(&state.model)?;
            for action in self.enabled(&local, &state.model, &state.clocks)? {
                let delay = u64::try_from(k).map_err(|_| fault("delay-count"))?;
                match windows.iter_mut().find(|w| w.identity == action.identity) {
                    Some(window) => {
                        window.delays.insert(delay);
                    }
                    None => {
                        let given = self.given(&action.identity.operation);
                        let given = match given {
                            Some(values) => Some(
                                values
                                    .into_iter()
                                    .map(|(delay, weight)| Ok((self.bound_units(&delay)?, weight)))
                                    .collect::<Step<Vec<_>>>()?,
                            ),
                            None => None,
                        };
                        windows.push(Window {
                            identity: action.identity.clone(),
                            delays: BTreeSet::from([delay]),
                            forever: false,
                            given,
                        });
                    }
                }
            }
        }
        if trajectory.forever {
            if let Some(last) = trajectory.states.len().checked_sub(1) {
                let last = u64::try_from(last).map_err(|_| fault("delay-count"))?;
                for window in &mut windows {
                    window.forever = window.delays.contains(&last);
                }
            }
        }
        Ok(windows)
    }

    fn workload_moves(
        &mut self,
        at: usize,
        state: &DState,
        local: &Rc<Local>,
        workload: &Workload,
    ) -> Step<Vec<Planned>> {
        let trajectory = self.trajectory(state)?;
        let windows = self.windows(&trajectory)?;
        let reached = u64::try_from(trajectory.states.len().saturating_sub(1))
            .map_err(|_| fault("delay-count"))?;
        let free = windows.iter().any(|window| window.given.is_none());
        // Each given delay is drawn conditioned on its window; an identity
        // with no conditioned mass in its window does not race (ADR-026
        // SD-2, SD-3).
        let mut racers: Vec<(&Window, Vec<(u64, Rational)>)> = Vec::new();
        for window in &windows {
            let Some(support) = &window.given else {
                continue;
            };
            let inside: Vec<(u64, Rational)> = support
                .iter()
                .filter(|(delay, _)| window.contains(*delay, reached))
                .cloned()
                .collect();
            let total = inside
                .iter()
                .fold(zero(), |sum, (_, weight)| sum.add(weight));
            if total.is_zero() {
                continue;
            }
            let conditioned = inside
                .into_iter()
                .map(|(delay, weight)| {
                    let share = weight
                        .div(&total)
                        .ok_or_else(|| Halt::from(fault("conditioned-weights-sum")))?;
                    Ok((delay, share))
                })
                .collect::<Step<Vec<_>>>()?;
            racers.push((window, conditioned));
        }
        if racers.is_empty() && free {
            // Every racing delay is free (FR-204).
            return self.free_moves(at, state, local, workload, (&windows, reached));
        }
        if racers.is_empty() {
            let mut planned = Vec::new();
            self.running_delays(at, state, local, &mut planned)?;
            if planned.is_empty() {
                self.time_locked.push(at);
            }
            self.decisions = true;
            return Ok(planned);
        }
        if free {
            return self.free_delays_moves(at, workload, &trajectory, racers, &windows);
        }
        let mut outcomes: Vec<(u64, Vec<usize>, Rational)> = Vec::new();
        let mut draw = vec![0_usize; racers.len()];
        while !racers.is_empty() {
            let mut probability = one();
            let mut least = u64::MAX;
            for (index, (_, support)) in racers.iter().enumerate() {
                let (delay, share) = &support[draw[index]];
                probability = probability.mul(share);
                least = least.min(*delay);
            }
            let tied: Vec<usize> = racers
                .iter()
                .enumerate()
                .filter(|(index, (_, support))| support[draw[*index]].0 == least)
                .map(|(index, _)| index)
                .collect();
            outcomes.push((least, tied, probability));
            let mut position = racers.len();
            loop {
                if position == 0 {
                    break;
                }
                position -= 1;
                draw[position] += 1;
                if draw[position] < racers[position].1.len() {
                    position = usize::MAX;
                    break;
                }
                draw[position] = 0;
            }
            if position != usize::MAX {
                break;
            }
        }
        let representative = windows
            .iter()
            .find(|window| window.given.is_some())
            .map(|window| window.identity.clone());
        let ids: Vec<ScheduledIdentity> = racers
            .iter()
            .map(|(window, _)| window.identity.clone())
            .collect();
        let mut planned: Vec<Planned> = Vec::new();
        let representative = representative.ok_or_else(|| fault("a-race-has-a-racer"))?;
        for (delay, tied, probability) in &outcomes {
            let identities: Vec<ScheduledIdentity> =
                tied.iter().map(|racer| ids[*racer].clone()).collect();
            self.race_edges(
                at,
                workload,
                &trajectory,
                (*delay, identities, probability.clone()),
                &representative,
                Edges {
                    planned: &mut planned,
                    scope: 0,
                },
            )?;
        }
        Ok(planned)
    }

    /// FR-204: given and free delays race together. The scheduler chooses
    /// every free racing identity's delay after the given delays are drawn
    /// (ADR-026 SD-3), as the `FreeDelays` action. The product builds its row
    /// choices only (FR-196): per drawn given vector `g`, in ascending numeric
    /// order, and per assignment of a delay in its window to every free
    /// identity, in ascending numeric order, the race of `g` with that
    /// assignment, its edges carrying `P(g)` times the outcome's mass.
    fn free_delays_moves(
        &mut self,
        at: usize,
        workload: &Workload,
        trajectory: &Trajectory,
        racers: Vec<(&Window, Vec<(u64, Rational)>)>,
        windows: &[Window],
    ) -> Step<Vec<Planned>> {
        let mut given = racers
            .into_iter()
            .map(|(window, mut support)| {
                support.sort_by_key(|(delay, _)| *delay);
                Ok((
                    self.schedule_key(&window.identity)?,
                    &window.identity,
                    support,
                ))
            })
            .collect::<Step<Vec<_>>>()?;
        given.sort_by(|left, right| left.0.cmp(&right.0));
        let mut free = windows
            .iter()
            .filter(|window| window.given.is_none())
            .map(|window| {
                let delays: Vec<u64> = window.delays.iter().copied().collect();
                Ok((
                    self.schedule_key(&window.identity)?,
                    &window.identity,
                    delays,
                ))
            })
            .collect::<Step<Vec<_>>>()?;
        free.sort_by(|left, right| left.0.cmp(&right.0));
        let representative = free
            .first()
            .map(|(_, identity, _)| (*identity).clone())
            .ok_or_else(|| fault("a-free-delay-races"))?;
        let mut planned: Vec<Planned> = Vec::new();
        for draw in Odometer::new(given.iter().map(|(_, _, support)| support.len())) {
            let mut probability = one();
            let mut drawn = Vec::with_capacity(given.len());
            for ((_, identity, support), index) in given.iter().zip(&draw) {
                let (delay, share) = support
                    .get(*index)
                    .ok_or_else(|| fault("odometer-index-in-support"))?;
                probability = probability.mul(share);
                drawn.push(((*identity).clone(), *delay));
            }
            for assignment in Odometer::new(free.iter().map(|(_, _, delays)| delays.len())) {
                let chosen: Vec<(ScheduledIdentity, u64)> = free
                    .iter()
                    .zip(&assignment)
                    .map(|((_, identity, delays), index)| {
                        let delay = delays
                            .get(*index)
                            .ok_or_else(|| fault("odometer-index-in-window"))?;
                        Ok(((*identity).clone(), *delay))
                    })
                    .collect::<Step<_>>()?;
                let least = drawn
                    .iter()
                    .chain(&chosen)
                    .map(|(_, delay)| *delay)
                    .min()
                    .ok_or_else(|| fault("a-race-has-a-racer"))?;
                let tied: Vec<ScheduledIdentity> = drawn
                    .iter()
                    .chain(&chosen)
                    .filter(|(_, delay)| *delay == least)
                    .map(|(identity, _)| identity.clone())
                    .collect();
                let first = planned.len();
                self.race_edges(
                    at,
                    workload,
                    trajectory,
                    (least, tied, probability.clone()),
                    &representative,
                    Edges {
                        planned: &mut planned,
                        scope: first,
                    },
                )?;
                let choice = ProductAction::FreeDelays(vec![FreeDelayRow {
                    given: drawn.clone(),
                    free: chosen,
                }]);
                for edge in &mut planned[first..] {
                    edge.choice = choice.clone();
                }
            }
        }
        self.decisions = true;
        if planned.is_empty() {
            self.time_locked.push(at);
        }
        Ok(planned)
    }

    /// FR-187's canonical order key of `identity`.
    fn schedule_key(&self, identity: &ScheduledIdentity) -> Step<Vec<u8>> {
        self.system
            .scheduled_identity_key(identity)
            .map_err(|_| fault("schedule-key-encodes").into())
    }

    /// The edges of one race outcome: `(delay, tied identities, probability)`.
    fn race_edges(
        &mut self,
        at: usize,
        workload: &Workload,
        trajectory: &Trajectory,
        outcome: (u64, Vec<ScheduledIdentity>, Rational),
        representative: &ScheduledIdentity,
        out: Edges<'_>,
    ) -> Step<()> {
        let (delay, identities, probability) = outcome;
        let index = usize::try_from(delay).map_err(|_| fault("delay-count"))?;
        let last = trajectory.states.len().saturating_sub(1);
        let start = trajectory
            .states
            .get(index.min(last))
            .ok_or_else(|| fault("race-delay-is-reachable"))?
            .clone();
        let weighted = identities
            .into_iter()
            .map(|identity| {
                let weight = self.weight_of(workload, &identity.operation)?;
                Ok((identity, weight))
            })
            .collect::<Step<Vec<_>>>()?;
        let Edges { planned, scope } = out;
        each_ordering(
            &weighted,
            probability,
            &mut Vec::with_capacity(weighted.len()),
            &mut |order, ordered| {
                self.poll()?;
                self.chain_edges(
                    at,
                    &start,
                    ordered,
                    (order, delay == 0, delay, representative),
                    Edges {
                        planned: &mut *planned,
                        scope,
                    },
                )?;
                Ok(())
            },
        )
    }

    /// The edges of one race outcome: the state after the winning delay, then
    /// each tied identity in order while it is still enabled.
    #[implements("FR-204-AC-20")]
    fn chain_edges(
        &mut self,
        at: usize,
        start: &DState,
        ordered: &[ScheduledIdentity],
        outcome: (Rational, bool, u64, &ScheduledIdentity),
        out: Edges<'_>,
    ) -> Step<()> {
        let Edges { planned, scope } = out;
        let (probability, zero_delay, elapsed_ticks, representative) = outcome;
        // (probability, states reached after each step, first draw, rewards
        // summed over the steps taken)
        type Chain = (
            Rational,
            Vec<DState>,
            Option<Vec<Value>>,
            Vec<(Identifier, Value)>,
        );
        let mut chains: Vec<Chain> = vec![(
            probability,
            vec![start.clone()],
            None,
            self.system.zero_rewards(),
        )];
        for identity in ordered {
            let mut next: Vec<Chain> = Vec::new();
            for (chain_probability, states, first_draw, summed) in chains {
                let current = states
                    .last()
                    .cloned()
                    .ok_or_else(|| fault("chain-has-a-state"))?;
                let local = self.local(&current.model)?;
                let available = self.enabled(&local, &current.model, &current.clocks)?;
                let Some(action) = available
                    .iter()
                    .find(|candidate| candidate.identity == *identity)
                else {
                    next.push((chain_probability, states, first_draw, summed));
                    continue;
                };
                for draw in &action.draws {
                    let [(model, clocks)] = draw.posts.as_slice() else {
                        let (_, state) = self
                            .system
                            .state_key(&current.model, fsl_foundation::IdentityLimits::default())
                            .map_err(|_| fault("state-key-encodes"))?;
                        return Err(ProductRefusal::NotMarkov(Box::new(NotMarkov {
                            state,
                            transition: identity.clone(),
                            drawn_transition: draw.transition.clone(),
                            random: draw.random.clone(),
                            post_states: u64::try_from(draw.posts.len())
                                .expect("supported targets have at most 64-bit usize"),
                            post_state_examples: self.not_markov_post_keys(&draw.posts)?,
                        }))
                        .into());
                    };
                    let mut reached = states.clone();
                    reached.push(DState {
                        model: model.clone(),
                        clocks: clocks.clone(),
                        deadline: current.deadline,
                        tail: Tail::Running,
                    });
                    next.push((
                        chain_probability.mul(&draw.probability),
                        reached,
                        first_draw.clone().or_else(|| Some(draw.random.clone())),
                        add_rewards(&summed, &draw.rewards)?,
                    ));
                }
            }
            chains = next;
        }
        let first = ordered
            .first()
            .cloned()
            .ok_or_else(|| fault("race-takes-an-identity"))?;
        for (chain_probability, states, draw, rewards) in chains {
            let reached = states
                .last()
                .cloned()
                .ok_or_else(|| fault("chain-has-a-state"))?;
            let landed: Vec<ModelSimState> = states
                .iter()
                .skip(1)
                .map(|state| state.model.clone())
                .collect();
            let taken = TraceStep {
                action: Some(first.clone()),
                draw: draw.clone(),
                probability: chain_probability.clone(),
            };
            let target = self.arrive(at, reached, &landed, Some(&taken), planned, |key| {
                planned[scope..].iter().any(|edge| {
                    edge.target.key == *key
                        && edge.action.as_ref() == Some(&first)
                        && same_rewards(&edge.rewards, &rewards)
                })
            })?;
            if let Some(merged) = planned[scope..].iter_mut().find(|edge| {
                edge.target.key == target.key
                    && edge.action.as_ref() == Some(&first)
                    && same_rewards(&edge.rewards, &rewards)
            }) {
                merged.probability = merged.probability.add(&chain_probability);
                merged.draw = None;
                merged.step = Some(TraceStep {
                    action: taken.action,
                    draw: None,
                    probability: merged.probability.clone(),
                });
                continue;
            }
            super::observe_edge_construction();
            planned.push(Planned {
                choice: ProductAction::Identity(representative.clone()),
                action: Some(first.clone()),
                draw,
                target,
                probability: chain_probability,
                rewards,
                elapsed_ticks,
                step: Some(taken),
                zero_delay,
            });
        }
        Ok(())
    }

    /// Under a workload whose racing identities all have free delays: the
    /// scheduler chooses between a unit delay and letting the workload resolve
    /// the identity now (ADR-028 TA-1, RU-6). An identity with a given delay
    /// does not race here, so the workload resolves among the free ones only.
    fn free_moves(
        &mut self,
        at: usize,
        state: &DState,
        local: &Rc<Local>,
        workload: &Workload,
        reading: (&[Window], u64),
    ) -> Step<Vec<Planned>> {
        let mut enabled = self.enabled(local, &state.model, &state.clocks)?;
        enabled.retain(|action| self.given(&action.identity.operation).is_none());
        // FR-204 reads this state as `UnitDelay` and one weighted
        // `WorkloadResolve`; ADR-026 SD-1 and SD-3 have the scheduler pick each
        // free identity's delay inside its window. The readings differ where
        // several free identities are enabled now and where the unit delay
        // disables one, so neither is decided here.
        let (windows, reached) = reading;
        if enabled.len() >= 2 {
            return Err(ProductRefusal::WorkloadFreeDelays(
                fsl_replay::WorkloadFreeDelayRefusal::ManyFreeEnabled,
            )
            .into());
        }
        if reached >= 1 {
            for action in &enabled {
                let closes = windows
                    .iter()
                    .find(|window| window.identity == action.identity)
                    .is_some_and(|window| !window.contains(1, reached));
                if closes {
                    return Err(ProductRefusal::WorkloadFreeDelays(
                        fsl_replay::WorkloadFreeDelayRefusal::ClosesBeforeFiring,
                    )
                    .into());
                }
            }
        }
        let mut planned = Vec::new();
        self.running_delays(at, state, local, &mut planned)?;
        let mut operations: BTreeMap<String, (Rational, u64)> = BTreeMap::new();
        for action in &enabled {
            let operation = action.identity.operation.clone();
            if !operations.contains_key(&operation) {
                let weight = self.weight_of(workload, &operation)?;
                operations.insert(operation.clone(), (weight, 0));
            }
            if let Some(entry) = operations.get_mut(&operation) {
                entry.1 += 1;
            }
        }
        let total = operations
            .values()
            .fold(zero(), |sum, (weight, _)| sum.add(weight));
        let representative = enabled.first().map(|action| action.identity.clone());
        for action in &enabled {
            let (weight, identities) = operations
                .get(action.identity.operation.as_str())
                .ok_or_else(|| fault("operation-is-counted"))?;
            let share = weight
                .div(&total)
                .and_then(|share| {
                    share.div(&Rational::from_integer(Integer::from_big(BigInt::from(
                        *identities,
                    ))))
                })
                .ok_or_else(|| fault("workload-share"))?;
            let first = planned.len();
            self.identity_edges(at, state, action, share, &mut planned)?;
            for edge in &mut planned[first..] {
                edge.choice = resolve_choice(representative.as_ref(), action, 0);
            }
        }
        if planned.is_empty() {
            self.time_locked.push(at);
        }
        self.decisions = true;
        Ok(planned)
    }

    // -- the finished product --------------------------------------------------

    /// FR-924: find a maximal end component whose *whole scheduler action*
    /// takes zero time and whose positive-probability support remains inside
    /// the candidate. A raw edge SCC is insufficient: it mistakes a leaky
    /// cycle for a closed zero-time component.
    fn zero_delay_components(&self) -> Vec<Vec<usize>> {
        let count = self.nodes.len();
        let product = ProbProduct {
            states: self.nodes.iter().map(|node| node.key.clone()).collect(),
            kind: ProductKind::Mdp,
            edges: self.edges.clone(),
            initial: Vec::new(),
            decided: Vec::new(),
            labels: Vec::new(),
            fairness_operations: Vec::new(),
            fairness: Vec::new(),
        };
        let mut choices = Vec::with_capacity(count);
        for state in product.ids() {
            let mut groups: Vec<(ProductAction, Vec<&ProbEdge>, bool)> = Vec::new();
            for (edge_index, edge) in self.edges.iter().enumerate() {
                if edge.from != state || edge.probability.is_zero() {
                    continue;
                }
                let zero = self.zero_delay.get(edge_index).copied().unwrap_or(false);
                if let Some((_, edges, all_zero)) = groups
                    .iter_mut()
                    .find(|(action, _, _)| same_action(action, &edge.choice))
                {
                    edges.push(edge);
                    *all_zero &= zero;
                } else {
                    groups.push((edge.choice.clone(), vec![edge], zero));
                }
            }
            choices.push(
                groups
                    .into_iter()
                    .filter(|(_, _, all_zero)| *all_zero)
                    .map(|(_, edges, _)| Choice { edges })
                    .collect(),
            );
        }
        let graph = Graph {
            product: &product,
            choices,
        };
        maximal_end_components_from(&graph, (0..count).collect()).unwrap_or_default()
    }

    fn finish(
        mut self,
        initial: Vec<(u32, Option<ModelInstance>, ProbStateId)>,
    ) -> Result<ProbProduct, ProductRefusal> {
        for component in self.zero_delay_components() {
            let members: BTreeSet<_> = component.iter().copied().collect();
            let has_exit = component.iter().any(|state| {
                let mut groups: Vec<(&ProductAction, Vec<usize>)> = Vec::new();
                for (edge_index, edge) in self.edges.iter().enumerate() {
                    if edge.from.index() != *state || edge.probability.is_zero() {
                        continue;
                    }
                    if let Some((_, indices)) = groups
                        .iter_mut()
                        .find(|(action, _)| same_action(action, &edge.choice))
                    {
                        indices.push(edge_index);
                    } else {
                        groups.push((&edge.choice, vec![edge_index]));
                    }
                }
                groups.iter().any(|(_, indices)| {
                    !indices.iter().all(|index| {
                        self.zero_delay.get(*index).copied().unwrap_or(false)
                            && members.contains(&self.edges[*index].to.index())
                    })
                })
            });
            if has_exit {
                continue;
            }
            self.time_locked.extend(component);
        }
        if !self.time_locked.is_empty() {
            let locked: BTreeSet<_> = self.time_locked.iter().copied().collect();
            let mut edges = Vec::with_capacity(self.edges.len());
            let mut zero_delay = Vec::with_capacity(self.zero_delay.len());
            for (edge, zero) in self.edges.drain(..).zip(self.zero_delay.drain(..)) {
                if !locked.contains(&edge.from.index()) {
                    edges.push(edge);
                    zero_delay.push(zero);
                }
            }
            self.edges = edges;
            self.zero_delay = zero_delay;
        }
        let measure_limit = match &self.form {
            Form::Elapsed {
                threshold, strict, ..
            } => Some((
                self.bound_units(threshold).map_err(|halt| match halt {
                    Halt::Refusal(refusal) => refusal,
                    Halt::Undefined(_) => fault("elapsed-threshold-scales"),
                })?,
                *strict,
            )),
            _ => None,
        };
        let mut decided: Vec<(ProbStateId, MonitorVerdict)> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| match node.key.monitor {
                Some(MonitorState::Deadline { phase, .. }) => match phase {
                    DeadlinePhase::Accept => Some((ProbStateId(index), MonitorVerdict::Accept)),
                    DeadlinePhase::Reject => Some((ProbStateId(index), MonitorVerdict::Reject)),
                    DeadlinePhase::Pending => None,
                },
                Some(MonitorState::TimedFormula { root, .. }) => match root.phase {
                    DeadlinePhase::Accept => Some((ProbStateId(index), MonitorVerdict::Accept)),
                    DeadlinePhase::Reject => Some((ProbStateId(index), MonitorVerdict::Reject)),
                    DeadlinePhase::Pending => None,
                },
                // FR-877: a completed or censored measure is decided; it is in
                // the transform's event when `M <= c` (`M < c`) holds.
                Some(MonitorState::Measure { phase, .. }) => {
                    let within = match (phase, node.state.deadline, measure_limit) {
                        (MeasurePhase::Complete, Some(elapsed), Some((limit, strict))) => {
                            if strict {
                                elapsed < limit
                            } else {
                                elapsed <= limit
                            }
                        }
                        (MeasurePhase::Complete | MeasurePhase::Censored, _, _) => false,
                        _ => return None,
                    };
                    Some((
                        ProbStateId(index),
                        if within {
                            MonitorVerdict::Accept
                        } else {
                            MonitorVerdict::Reject
                        },
                    ))
                }
                _ => None,
            })
            .collect();
        // A pending event at a state with no move at all (a time-lock) can
        // never be met: ADR-026 TD-2 reports the lock, and the event closes
        // as a rejection (FR-196 "Canonical probability product key").
        for state in &self.time_locked {
            decided.push((ProbStateId(*state), MonitorVerdict::Reject));
        }
        decided.sort_by_key(|(state, _)| state.index());
        decided.dedup_by_key(|(state, _)| state.index());
        let labels = if matches!(self.form, Form::Labels(_) | Form::MeanTime { .. }) {
            self.nodes.iter().map(|node| node.truths.clone()).collect()
        } else {
            Vec::new()
        };
        Ok(ProbProduct {
            states: self.nodes.into_iter().map(|node| node.key).collect(),
            kind: if self.decisions || self.workload.is_none() {
                ProductKind::Mdp
            } else {
                ProductKind::Dtmc
            },
            edges: self.edges,
            initial,
            decided,
            labels,
            fairness_operations: Vec::new(),
            fairness: Vec::new(),
        })
    }
}
