// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite-horizon measure proofs, independently reconstructed from checked input.

use super::*;
use filament_exact::OrderingOperator as Order;
use fsl_eval::probabilistic::{
    Accumulator, MeasureComparison, MeasurePayoff, MeasurePlan, MonitorState,
};
use fsl_eval::simulation::model::{ModelKeyBudget, ModelKeyError, PredicateReading};
use fsl_semantics::check::{BoundedMeasure, CheckedPredicate, Event, Weighting};
use ix_trace_rs::implements;
use quire_canonical::{Encode, Error, Node, Sink, Writer};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) const PROFILE: &str = "filament.probability-objective/finite-horizon-measure/v1";
pub(in crate::probability_certificate) mod bounds;

/// Terminal-payoff role of a finite-horizon measure proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeasurePayoffKind {
    /// A probability comparison event.
    Event,
    /// A quantile's satisfied and activated numerator.
    Satisfied,
    /// A quantile's activation denominator.
    Activated,
    /// An every-scheduler quantile's signed transform.
    Shifted,
    /// The mean of a completed fraction.
    Fraction,
}
impl MeasurePayoffKind {
    fn text(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Satisfied => "satisfied",
            Self::Activated => "activated",
            Self::Shifted => "shifted",
            Self::Fraction => "fraction",
        }
    }
    fn read(text: &str) -> Result<Self, ProbabilityDocumentError> {
        match text {
            "event" => Ok(Self::Event),
            "satisfied" => Ok(Self::Satisfied),
            "activated" => Ok(Self::Activated),
            "shifted" => Ok(Self::Shifted),
            "fraction" => Ok(Self::Fraction),
            _ => Err(ProbabilityDocumentError::ObjectiveMismatch),
        }
    }
}

/// Unchecked finite-horizon measure evidence. Construction supplies no acceptance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasureProbabilityDocument {
    /// Exact checked obligation identity.
    pub identity: ObligationIdentity,
    /// Terminal-payoff selection, independently checked against the claim.
    pub payoff: MeasurePayoffKind,
    /// Complete full-key values in canonical order.
    pub values: Vec<ExactProbabilityValue>,
    /// Remaining-horizon ranks in canonical order.
    pub ranking: Vec<(ProbabilityProductKey, u64)>,
    objective: Objective,
    conditional: Option<Box<Conditional>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
enum Conditional {
    Ratio(
        Box<MeasureProbabilityDocument>,
        Box<MeasureProbabilityDocument>,
    ),
    Vacuous,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Objective {
    kind: &'static str,
    window: u64,
    reward: Option<ProbabilityRewardObjective>,
    comparison: Option<MeasureComparison>,
    threshold: Option<Rational>,
    level: Option<Rational>,
    optimization: ProbabilityObjective,
    fairness: Vec<fairness::Record>,
}

struct Plan<'a> {
    control: MeasurePlan<'a>,
    predicates: Vec<&'a CheckedPredicate>,
    objective: Objective,
    payoff: MeasurePayoff,
    bound: Bound,
    strict: bool,
}

impl Plan<'_> {
    /// The bound a claim-side document's initial value meets (FR-1042
    /// "Pairing"): the claim's own, or, for the `shifted` payoff, at least 0
    /// for a `<= c` quantile and at most 0 for a `>= c` one. Its direction is
    /// the kind such a document has.
    fn certified_bound(&self, payoff: MeasurePayoffKind) -> Bound {
        match (payoff, self.strict) {
            (MeasurePayoffKind::Shifted, true) => Bound::AtMost(super::rational(0)),
            (MeasurePayoffKind::Shifted, false) => Bound::AtLeast(super::rational(0)),
            _ => self.bound.clone(),
        }
    }
}

/// Which product states one enumeration expands.
#[derive(Clone, Copy)]
enum Scope<'a> {
    /// Every reachable state.
    Whole,
    /// A mean-refutation `lower`: each named state, restored from its key, and
    /// its successors. All of them count against the state limit.
    Restored(&'a BTreeSet<ProbabilityProductKey>),
    /// A partial `lower` proof: the named states the initial states reach
    /// through named states, and their successors. A successor left unnamed
    /// does not count against the state limit.
    Reached(&'a BTreeSet<ProbabilityProductKey>),
}

impl<'a> Scope<'a> {
    /// The states a partial enumeration expands.
    fn named(self) -> Option<&'a BTreeSet<ProbabilityProductKey>> {
        match self {
            Self::Whole => None,
            Self::Restored(named) | Self::Reached(named) => Some(named),
        }
    }

    /// Whether a successor that reaches `states` states exceeds `limit`.
    fn refuses(self, states: usize, limit: usize) -> bool {
        !matches!(self, Self::Reached(_)) && states > limit
    }
}

pub(super) struct CheckOptions<'a> {
    pub(super) partial_lower: bool,
    pub(super) bound_side: Option<super::SubsystemBoundKind>,
    pub(super) role: super::CertificateRole,
    pub(super) limits: ProbabilityDocumentLimits,
    pub(super) cancel: &'a Cancel,
}

pub(in crate::probability_certificate) fn ordered(
    operator: filament_exact::OrderingOperator,
    left: &Rational,
    right: &Rational,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    poll(cancel)?;
    match filament_exact::order_numbers(
        operator,
        filament_exact::OrderedOperands::Rationals(left, right),
        meter,
    ) {
        filament_exact::Outcome::Completed(value) => Ok(value),
        filament_exact::Outcome::Incomplete(error) => match cancel.tripped() {
            Some(cause) => Err(ProbabilityDocumentError::Cancelled(cause)),
            None => Err(ProbabilityDocumentError::Accounting(Box::new(error))),
        },
        filament_exact::Outcome::Undefined(_) | filament_exact::Outcome::Refused(_) => {
            Err(ProbabilityDocumentError::Malformed)
        }
    }
}

fn plan<'a>(
    system: &ModelSystem<'_>,
    claim: &'a CheckedProbabilisticClaim,
    payoff: MeasurePayoffKind,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<Plan<'a>, ProbabilityDocumentError> {
    use fsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
    if fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        != Some(fsl_semantics::check::TemporalProfile::EventPositionFalseExtension)
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let (measure, fraction, bound, q, comparison) = match (&claim.form, payoff) {
        (
            ProbForm::Probability {
                event:
                    Event::Compare {
                        measure,
                        bound: comparison,
                    },
                bound,
            },
            MeasurePayoffKind::Event,
        ) => (Some(measure), None, bound.clone(), None, Some(comparison)),
        (
            ProbForm::Quantile {
                measure,
                threshold,
                q,
            },
            MeasurePayoffKind::Satisfied
            | MeasurePayoffKind::Activated
            | MeasurePayoffKind::Shifted,
        ) => (
            Some(measure),
            None,
            Bound::AtLeast(q.clone()),
            Some(q),
            Some(threshold),
        ),
        (ProbForm::MeanOfFraction { fraction, bound }, MeasurePayoffKind::Fraction) => {
            (None, Some(fraction), bound.clone(), None, None)
        }
        _ => return Err(ProbabilityDocumentError::ObjectiveMismatch),
    };
    let strict = matches!(
        &claim.form,
        ProbForm::Quantile {
            threshold: Bound::AtLeast(_),
            ..
        }
    );
    let comparison_kind = comparison.map(|bound| match bound {
        Bound::AtLeast(_) if strict => MeasureComparison::Lt,
        Bound::AtLeast(_) => MeasureComparison::Ge,
        Bound::AtMost(_) => MeasureComparison::Le,
    });
    let threshold = comparison.map(|bound| match bound {
        Bound::AtLeast(value) | Bound::AtMost(value) => value.clone(),
    });
    let weight = |weight: &'a Option<Weighting>| match weight {
        None => Ok(None),
        Some(Weighting::Reward(name)) => Ok(Some(name)),
        Some(Weighting::Delay) => Err(ProbabilityDocumentError::Unsupported),
    };
    let (control, predicates, kind, window, reward_name) = match (measure, fraction) {
        (
            Some(BoundedMeasure::Steps {
                from,
                until,
                within,
                ..
            }),
            _,
        ) => (
            MeasurePlan::Measure {
                reward: None,
                threshold: threshold
                    .clone()
                    .ok_or(ProbabilityDocumentError::Malformed)?,
                within: *within,
            },
            vec![from.as_ref(), until.as_ref()],
            "steps",
            *within,
            None,
        ),
        (
            Some(BoundedMeasure::Accumulate {
                reward,
                from,
                until,
                within,
                ..
            }),
            _,
        ) => (
            MeasurePlan::Measure {
                reward: Some(reward),
                threshold: threshold
                    .clone()
                    .ok_or(ProbabilityDocumentError::Malformed)?,
                within: *within,
            },
            vec![from.as_ref(), until.as_ref()],
            "accumulate",
            *within,
            Some(reward),
        ),
        (Some(BoundedMeasure::Fraction(fraction)), _) | (None, Some(fraction)) => {
            let weight = weight(&fraction.weight)?;
            (
                MeasurePlan::Fraction {
                    window: fraction.window,
                    weight,
                },
                vec![&fraction.predicate],
                if weight.is_some() {
                    "weighted-fraction"
                } else {
                    "fraction"
                },
                fraction.window,
                weight,
            )
        }
        _ => return Err(ProbabilityDocumentError::Unsupported),
    };
    let reward = reward_name
        .map(|name| super::super::reward::objective(system, claim, name, limits, cancel, storage))
        .transpose()?;
    let optimization = match claim.scheduler {
        ProbScheduler::Workload(_) => ProbabilityObjective::Workload,
        ProbScheduler::Every => match (&claim.form, &bound) {
            (ProbForm::Quantile { .. }, _) if strict => ProbabilityObjective::Maximum,
            (_, Bound::AtLeast(_)) => ProbabilityObjective::Minimum,
            (_, Bound::AtMost(_)) => ProbabilityObjective::Maximum,
        },
    };
    let fairness = if claim.fairness.is_empty() {
        Vec::new()
    } else {
        fairness::checked(system, claim, limits, cancel, meter, storage)?
    };
    let comparison = if matches!(
        payoff,
        MeasurePayoffKind::Activated | MeasurePayoffKind::Fraction
    ) {
        None
    } else {
        comparison_kind
    };
    let objective = Objective {
        kind,
        window,
        reward,
        comparison,
        threshold: comparison.and(threshold.clone()),
        level: if payoff == MeasurePayoffKind::Shifted {
            q.cloned()
        } else {
            None
        },
        optimization,
        fairness,
    };
    let compared = || -> Result<_, ProbabilityDocumentError> {
        Ok((
            comparison_kind.ok_or(ProbabilityDocumentError::Malformed)?,
            threshold
                .clone()
                .ok_or(ProbabilityDocumentError::Malformed)?,
        ))
    };
    let terminal = match payoff {
        MeasurePayoffKind::Event => {
            let (op, value) = compared()?;
            MeasurePayoff::Event(op, value)
        }
        MeasurePayoffKind::Satisfied => {
            let (op, value) = compared()?;
            MeasurePayoff::Satisfied(op, value)
        }
        MeasurePayoffKind::Activated => MeasurePayoff::Activated,
        MeasurePayoffKind::Shifted => {
            let (op, value) = compared()?;
            MeasurePayoff::Shifted(
                op,
                value,
                q.cloned().ok_or(ProbabilityDocumentError::Malformed)?,
            )
        }
        MeasurePayoffKind::Fraction => MeasurePayoff::Fraction,
    };
    Ok(Plan {
        control,
        predicates,
        objective,
        payoff: terminal,
        bound,
        strict,
    })
}

/// Construct the complete shared-owner measure key under the actual caller.
/// No model, monitor, amount or binding component is projected away.
#[allow(clippy::too_many_arguments)]
pub fn measure_probability_product_key(
    binding: Option<&ObjectReference>,
    raw: &[u8],
    control: MonitorState,
    accumulator: &Accumulator,
    pending: Option<&[u8]>,
    reward: Option<&ProbabilityRewardObjective>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    poll(cancel)?;
    let mut admission = ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
    let result = {
        let scope = RefCell::new(&mut admission as &mut dyn ModelKeyBudget);
        struct Reserve<'a, 'b>(&'a RefCell<&'b mut dyn ModelKeyBudget>);
        impl quire_canonical::Reservation for Reserve<'_, '_> {
            fn reserve(&mut self, bytes: usize) -> Result<(), quire_canonical::ReservationDenied> {
                self.0
                    .borrow_mut()
                    .reserve(bytes)
                    .map_err(|_| quire_canonical::ReservationDenied)
            }
        }
        let identity = fsl_foundation::IdentityLimits::default();
        (|| {
            scope
                .borrow_mut()
                .work(u64::try_from(raw.len()).unwrap_or(u64::MAX))
                .map_err(|_| ModelKeyError::Denied)?;
            let model = quire_canonical::read_with_reservation(
                raw,
                identity.input_bytes,
                &mut Reserve(&scope),
            )
            .map_err(|error| key_parse_error(error, identity))?;
            if !key::model_form(model.root()) {
                return Err(ModelKeyError::Unsupported("canonical model body"));
            }
            let original =
                writer::encode_admitted(&model, identity.input_bytes, &mut Reserve(&scope))
                    .map_err(|_| ModelKeyError::Denied)?;
            if original != raw {
                return Err(ModelKeyError::Unsupported("canonical model body"));
            }
            let pending = pending
                .map(|bytes| {
                    quire_canonical::read_with_reservation(
                        bytes,
                        identity.input_bytes,
                        &mut Reserve(&scope),
                    )
                    .map_err(|error| key_parse_error(error, identity))
                })
                .transpose()?;
            let key = fsl_eval::probabilistic::key::MeasureProductKey {
                binding,
                model: model.root(),
                control,
                accumulator,
                pending: pending.as_ref().map(|pending| pending.root()),
                reward: reward.map(|reward| fsl_eval::probabilistic::key::RewardKey {
                    model: &reward.model,
                    name: &reward.name,
                    unit: reward.unit.as_ref(),
                }),
                budget: &scope,
                max_bytes: identity.input_bytes,
            };
            key.validate().map_err(ModelKeyError::Unsupported)?;
            writer::encode_admitted(&key, identity.input_bytes, &mut Reserve(&scope))
                .map(ProbabilityProductKey::from_canonical_bytes)
                .map_err(|error| {
                    ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::of(
                        &error, identity,
                    ))
                })
        })()
    };
    result.map_err(|error| admission.refusal(error))
}

type Resolution = (
    Vec<ModelSimState>,
    Vec<(filament_exact::Identifier, filament_exact::Value)>,
);

enum Successor {
    Position(
        Rational,
        ModelSimState,
        Vec<(filament_exact::Identifier, filament_exact::Value)>,
    ),
    Draw(
        Rational,
        fsl_eval::simulation::model::ScheduledIdentity,
        Vec<filament_exact::Value>,
        Vec<ModelSimState>,
        Vec<(filament_exact::Identifier, filament_exact::Value)>,
    ),
}
/// One model state's expansion: its distributions and action labels, and the
/// canonical key of each successor state once a product edge has needed it.
struct Expansion {
    distributions: Vec<Vec<Successor>>,
    labels: Vec<Option<crate::ProductAction>>,
    post_raws: RefCell<Vec<Vec<Option<Vec<u8>>>>>,
}
struct State {
    terminal: Option<Rational>,
    activation_terminal: Option<Rational>,
    rank: u64,
    choices: Vec<Vec<ProbabilityEdge>>,
    actions: Vec<Option<crate::ProductAction>>,
    activated: bool,
}
struct Graph {
    initial: ProbabilityProductKey,
    initials: Vec<(u32, ProbabilityProductKey)>,
    states: BTreeMap<ProbabilityProductKey, State>,
}

fn restore_error(
    error: fsl_eval::simulation::model::ModelRestoreError,
    admission: &mut ProbabilityKeyAdmission<'_>,
    cancel: &Cancel,
) -> ProbabilityDocumentError {
    use fsl_eval::simulation::model::ModelRestoreError;
    if let Some(cause) = cancel.tripped() {
        return ProbabilityDocumentError::Cancelled(cause);
    }
    match error {
        ModelRestoreError::Malformed(_)
        | ModelRestoreError::Key(ModelKeyError::Unsupported(_))
        | ModelRestoreError::Key(ModelKeyError::Encoding(
            fsl_eval::simulation::EncodingRefusal::NoEncoding(_),
        )) => ProbabilityDocumentError::MalformedMember("values"),
        ModelRestoreError::Key(error) => admission.refusal(error),
        ModelRestoreError::Accounting(error) => ProbabilityDocumentError::Accounting(error),
    }
}

#[allow(clippy::too_many_arguments)]
fn restore_named(
    system: &ModelSystem<'_>,
    plan: &Plan<'_>,
    binding: Option<&ObjectReference>,
    key: &ProbabilityProductKey,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<(ModelSimState, MonitorState, Accumulator, Option<Resolution>), ProbabilityDocumentError>
{
    let malformed = || ProbabilityDocumentError::MalformedMember("values");
    poll(cancel)?;
    let mut admission = ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
    let parsed =
        quire_canonical::read_with_reservation(key.as_bytes(), limits.bytes, &mut admission)
            .map_err(|error| match error {
                quire_canonical::ReadError::ReservationDenied { .. } => {
                    admission.refusal(ModelKeyError::Denied)
                }
                quire_canonical::ReadError::Limit(limit) => {
                    exceeded(ProbabilityDocumentLimit::Bytes, limit.bound, limit.required)
                }
                _ => malformed(),
            })?;
    let Node::Array(parts) = parsed.root().node() else {
        return Err(malformed());
    };
    if parts.len() != 8 {
        return Err(malformed());
    }
    let parts: [_; 8] = parts
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| malformed())?;
    let Node::Array(model) = parts[4].node() else {
        return Err(malformed());
    };
    let [_, model]: [_; 2] = model
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| malformed())?;
    let identity = fsl_foundation::IdentityLimits::default();
    let raw =
        writer::encode_admitted(&model, identity.input_bytes, &mut admission).map_err(|error| {
            match error {
                Error::ReservationDenied { .. } => admission.refusal(ModelKeyError::Denied),
                _ => malformed(),
            }
        })?;
    let model = system
        .restore_state_key_with_budget(&raw, identity, &mut admission, cancel)
        .map_err(|error| restore_error(error, &mut admission, cancel))?;
    let (control, accumulator) = plan
        .control
        .restore_fraction_control(parts[5], parts[6], &mut admission)
        .map_err(|error| restore_error(error, &mut admission, cancel))?;
    let pending = if matches!(parts[7].node(), Node::Null) {
        None
    } else {
        Some(
            writer::encode_admitted(&parts[7], identity.input_bytes, &mut admission).map_err(
                |error| match error {
                    Error::ReservationDenied { .. } => admission.refusal(ModelKeyError::Denied),
                    _ => malformed(),
                },
            )?,
        )
    };
    drop(admission);
    let rebuilt = measure_probability_product_key(
        binding,
        &raw,
        control,
        &accumulator,
        pending.as_deref(),
        plan.objective.reward.as_ref(),
        limits,
        cancel,
        meter,
        storage,
    )?;
    if &rebuilt != key {
        return Err(malformed());
    }
    let resolution = if let Some(pending) = pending {
        if control.is_decided() || plan.objective.optimization == ProbabilityObjective::Workload {
            return Err(malformed());
        }
        let mut admission =
            ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
        let actions = system
            .actions_with_budget(&model, identity, &mut admission, cancel)
            .map_err(|error| admission.expansion_refusal(error))?
            .map_err(|_| {
                ProbabilityDocumentError::Build(Box::new(
                    crate::ExactProbBuildIssue::UndecidedSuccessor,
                ))
            })?;
        let mut selected = None;
        for action in actions {
            for draw in action.draws {
                poll(cancel)?;
                if draw.probability.is_zero() || draw.successors.len() < 2 {
                    continue;
                }
                let actual = system
                    .pending_key_bytes_with_budget(
                        &action.identity,
                        &draw.random,
                        identity,
                        &mut admission,
                        cancel,
                    )
                    .map_err(|error| admission.refusal(error))?;
                if actual == pending {
                    if selected.is_some() {
                        return Err(malformed());
                    }
                    selected = Some((
                        draw.successors.into_iter().map(|(_, post)| post).collect(),
                        draw.rewards,
                    ));
                }
            }
        }
        Some(selected.ok_or_else(malformed)?)
    } else {
        None
    };
    Ok((model, control, accumulator, resolution))
}

/// The whole graph, or, where `max_states` stops it and `lower` gives the
/// payoff and rows of a `lower` proof, the part those rows show: the named
/// states the initial states reach through named states, and their successors
/// (FSpec FR-411 "Evidence on a partial product"). The stop stands wherever
/// that part cannot show the proof: a payoff other than `shifted` or a
/// comparison event, a fair claim, more states named than `max_states`
/// admits, or a named state the part does not reach.
#[implements("FSpec-FR-412-AC-5")]
#[allow(clippy::too_many_arguments)]
fn whole_or_lower_part(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    plan: &Plan<'_>,
    lower: Option<(MeasurePayoffKind, &[ExactProbabilityValue])>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<Graph, ProbabilityDocumentError> {
    let exhausted = match enumerate(
        system,
        claim,
        plan,
        limits,
        cancel,
        meter,
        storage,
        Scope::Whole,
    ) {
        Err(
            error @ ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::States,
                ..
            },
        ) => error,
        whole => return whole,
    };
    let Some((MeasurePayoffKind::Shifted | MeasurePayoffKind::Event, rows)) = lower else {
        return Err(exhausted);
    };
    if !claim.fairness.is_empty() || rows.len() > limits.states {
        return Err(exhausted);
    }
    let mut named = BTreeSet::new();
    for row in rows {
        poll(cancel)?;
        charge(
            storage,
            limits.storage_bytes,
            u64::try_from(
                row.state
                    .as_bytes()
                    .len()
                    .saturating_add(std::mem::size_of::<ProbabilityProductKey>()),
            )
            .unwrap_or(u64::MAX),
        )?;
        named.insert(row.state.clone());
    }
    let part = enumerate(
        system,
        claim,
        plan,
        limits,
        cancel,
        meter,
        storage,
        Scope::Reached(&named),
    )?;
    if named.iter().any(|key| !part.states.contains_key(key)) {
        return Err(exhausted);
    }
    Ok(part)
}

#[allow(clippy::too_many_arguments)]
fn enumerate(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    plan: &Plan<'_>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    scope: Scope<'_>,
) -> Result<Graph, ProbabilityDocumentError> {
    let binding = claim_binding(system, claim)?;
    let binding = binding.as_ref().map(|binding| &binding.reference);
    let workload = claim_workload(system, claim)?;
    if limits.states == 0 {
        return Err(exceeded(ProbabilityDocumentLimit::States, 0, 1));
    }
    let initial = system.initial();
    if initial.is_empty() {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let read = |state: &ModelSimState| -> Result<Vec<bool>, ProbabilityDocumentError> {
        let predicates = plan
            .predicates
            .iter()
            .map(|predicate| (*predicate, binding))
            .collect::<Vec<_>>();
        system
            .state_predicates_with_cancel(state, &predicates, cancel)
            .map_err(|error| match cancel.tripped() {
                Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                None => ProbabilityDocumentError::Build(Box::new(
                    crate::ExactProbBuildIssue::Expansion(error),
                )),
            })?
            .into_iter()
            .map(|reading| match reading {
                PredicateReading::True => Ok(true),
                PredicateReading::False => Ok(false),
                PredicateReading::Undefined { cause, .. } => {
                    Err(ProbabilityDocumentError::UndefinedPredicate(cause))
                }
                PredicateReading::Refused(outcome) => Err(ProbabilityDocumentError::Build(
                    Box::new(crate::ExactProbBuildIssue::Predicate(outcome)),
                )),
            })
            .collect()
    };
    let raw_of = |state: &ModelSimState,
                  meter: &mut filament_exact::Meter,
                  storage: &mut u64|
     -> Result<Vec<u8>, ProbabilityDocumentError> {
        let mut admission =
            ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
        system
            .state_key_bytes_with_budget(
                state,
                fsl_foundation::IdentityLimits::default(),
                &mut admission,
                cancel,
            )
            .map_err(|error| admission.refusal(error))
    };
    let product_key = |raw: &[u8],
                       control: MonitorState,
                       accumulator: &Accumulator,
                       meter: &mut filament_exact::Meter,
                       storage: &mut u64| {
        measure_probability_product_key(
            binding,
            raw,
            control,
            accumulator,
            None,
            plan.objective.reward.as_ref(),
            limits,
            cancel,
            meter,
            storage,
        )
    };
    let mut queue = VecDeque::new();
    let mut seen = BTreeSet::new();
    // The canonical key of each queued model state, kept so a model state
    // reached under many product keys is encoded once.
    let mut raws: BTreeMap<ProbabilityProductKey, Vec<u8>> = BTreeMap::new();
    // A model state's expansion, shared by every product state over it.
    let mut expansions: BTreeMap<(Vec<u8>, String), Rc<Expansion>> = BTreeMap::new();
    let mut initials = Vec::new();
    for (index, initial) in initial.iter().enumerate() {
        poll(cancel)?;
        let truths = read(initial)?;
        let (control, accumulator) = plan
            .control
            .start(&truths, meter)
            .map_err(ProbabilityDocumentError::from)?;
        let control = control.ok_or(ProbabilityDocumentError::Malformed)?;
        let accumulator = accumulator.ok_or(ProbabilityDocumentError::Malformed)?;
        let initial_raw = raw_of(initial, meter, storage)?;
        let root = product_key(&initial_raw, control, &accumulator, meter, storage)?;
        charge(
            storage,
            limits.storage_bytes,
            u64::try_from(
                root.as_bytes()
                    .len()
                    .saturating_add(std::mem::size_of::<(u32, ProbabilityProductKey)>()),
            )
            .unwrap_or(u64::MAX),
        )?;
        initials.push((
            u32::try_from(index).map_err(|_| ProbabilityDocumentError::Malformed)?,
            root.clone(),
        ));
        if seen.insert(root.clone()) {
            raws.insert(root.clone(), initial_raw);
            if seen.len() > limits.states {
                return Err(exceeded(
                    ProbabilityDocumentLimit::States,
                    u64::try_from(limits.states).unwrap_or(u64::MAX),
                    u64::try_from(seen.len()).unwrap_or(u64::MAX),
                ));
            }
            queue.push_back((
                root,
                initial.clone(),
                control,
                accumulator,
                truths,
                None::<Resolution>,
            ));
        }
    }
    let root = seen
        .first()
        .cloned()
        .ok_or(ProbabilityDocumentError::Malformed)?;
    if let Scope::Restored(named) = scope {
        for key in named {
            if seen.contains(key) {
                continue;
            }
            let (model, control, accumulator, resolution) =
                restore_named(system, plan, binding, key, limits, cancel, meter, storage)?;
            let truths = if control.is_decided() {
                Vec::new()
            } else {
                read(&model)?
            };
            if seen.len() >= limits.states {
                return Err(exceeded(
                    ProbabilityDocumentLimit::States,
                    u64::try_from(limits.states).unwrap_or(u64::MAX),
                    u64::try_from(seen.len().saturating_add(1)).unwrap_or(u64::MAX),
                ));
            }
            charge(
                storage,
                limits.storage_bytes,
                u64::try_from(key.as_bytes().len().saturating_mul(3)).unwrap_or(u64::MAX),
            )?;
            seen.insert(key.clone());
            queue.push_back((key.clone(), model, control, accumulator, truths, resolution));
        }
    }
    let mut states = BTreeMap::new();
    let mut edges = 0usize;
    while let Some((key, model, control, accumulator, truths, resolution)) = queue.pop_front() {
        poll(cancel)?;
        charge(
            storage,
            limits.storage_bytes,
            u64::try_from(
                key.as_bytes()
                    .len()
                    .saturating_mul(3)
                    .saturating_add(std::mem::size_of::<State>()),
            )
            .unwrap_or(u64::MAX),
        )?;
        if scope.named().is_some_and(|named| !named.contains(&key)) {
            // A lower certificate assigns zero to this frontier key. Its
            // descendants and terminal payoff are outside the named proof.
            states.insert(
                key,
                State {
                    terminal: None,
                    activation_terminal: None,
                    rank: 0,
                    choices: Vec::new(),
                    actions: Vec::new(),
                    activated: false,
                },
            );
            continue;
        }
        let terminal = plan
            .payoff
            .terminal(control, &accumulator, meter)
            .map_err(ProbabilityDocumentError::from)?;
        let activation_terminal = if terminal.is_some()
            && matches!(
                plan.payoff,
                MeasurePayoff::Satisfied(_, _) | MeasurePayoff::Activated
            ) {
            let value = MeasurePayoff::Activated
                .terminal(control, &accumulator, meter)
                .map_err(ProbabilityDocumentError::from)?;
            charge(
                storage,
                limits.storage_bytes,
                u64::try_from(std::mem::size_of::<Rational>() + 2).unwrap_or(u64::MAX),
            )?;
            value
        } else {
            None
        };
        let rank = if control.is_decided() {
            0
        } else {
            match control {
                MonitorState::Measure { remaining, .. } | MonitorState::Fraction { remaining } => {
                    remaining
                }
                _ => return Err(ProbabilityDocumentError::Malformed),
            }
        };
        let activated = matches!(
            accumulator,
            Accumulator::Steps {
                activated: true,
                ..
            } | Accumulator::Reward {
                activated: true,
                ..
            }
        );
        let mut choices = Vec::new();
        let mut action_labels = Vec::new();
        let resolving = resolution.is_some();
        if terminal.is_none() {
            let model_raw = if resolving {
                None
            } else {
                Some(match raws.remove(&key) {
                    Some(raw) => raw,
                    None => raw_of(&model, meter, storage)?,
                })
            };
            // Expansion reads the state's provenance (a restored state has no
            // anchor), so the anchor is part of the cache key.
            let cache_key = model_raw
                .as_ref()
                .map(|raw| (raw.clone(), format!("{:?}", model.anchor())));
            let cached = cache_key
                .as_ref()
                .and_then(|at| expansions.get(at))
                .cloned();
            let expansion = if let Some(expansion) = cached {
                expansion
            } else {
                let mut action_labels = Vec::new();
                let mut admission =
                    ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
                let distributions = if let Some((posts, rewards)) = resolution {
                    posts
                        .into_iter()
                        .map(|post| {
                            action_labels.push(None);
                            vec![Successor::Position(rational(1), post, rewards.clone())]
                        })
                        .collect()
                } else if let Some(workload) = workload {
                    let steps = system
                        .weighted_steps_with_budget(
                            &model,
                            workload,
                            fsl_foundation::IdentityLimits::default(),
                            &mut admission,
                            cancel,
                        )
                        .map_err(|error| distribution_error(error, &mut admission))?
                        .map_err(|stop| {
                            ProbabilityDocumentError::Build(Box::new(match stop {
                                fsl_eval::simulation::model::Unsteppable::NotMarkov(cause) => {
                                    crate::ExactProbBuildIssue::NotMarkov(cause)
                                }
                                fsl_eval::simulation::model::Unsteppable::UndecidedSuccessor(_) => {
                                    crate::ExactProbBuildIssue::UndecidedSuccessor
                                }
                            }))
                        })?;
                    action_labels.push(None);
                    vec![
                        steps
                            .into_iter()
                            .map(|step| {
                                Successor::Position(step.probability, step.post, step.rewards)
                            })
                            .collect::<Vec<_>>(),
                    ]
                } else {
                    let actions = system
                        .actions_with_budget(
                            &model,
                            fsl_foundation::IdentityLimits::default(),
                            &mut admission,
                            cancel,
                        )
                        .map_err(|error| distribution_error(error, &mut admission))?
                        .map_err(|_| {
                            ProbabilityDocumentError::Build(Box::new(
                                crate::ExactProbBuildIssue::UndecidedSuccessor,
                            ))
                        })?;
                    if actions.is_empty() {
                        let rewards = system.zero_rewards();
                        action_labels.push(Some(crate::ProductAction::TerminalStutter));
                        vec![vec![Successor::Position(
                            rational(1),
                            model.clone(),
                            rewards,
                        )]]
                    } else {
                        let mut distributions = Vec::new();
                        for action in actions {
                            action_labels.push(Some(crate::ProductAction::Identity(
                                crate::Schedule::of(system, &action.identity)
                                    .map_err(|_| ProbabilityDocumentError::Malformed)?,
                            )));
                            let mut distribution = Vec::new();
                            for draw in action.draws {
                                if draw.successors.is_empty() {
                                    let state = system
                                        .state_key_bytes_with_budget(
                                            &model,
                                            fsl_foundation::IdentityLimits::default(),
                                            &mut admission,
                                            cancel,
                                        )
                                        .map_err(|error| admission.refusal(error))?;
                                    return Err(ProbabilityDocumentError::Build(Box::new(
                                        crate::ExactProbBuildIssue::NotMarkov(Box::new(
                                            fsl_eval::simulation::model::NotMarkov {
                                                state:
                                                    fsl_eval::simulation::StateKey::from_canonical_bytes(
                                                        state,
                                                    ),
                                                transition: action.identity,
                                                drawn_transition: draw.transition,
                                                random: draw.random,
                                                post_states: 0,
                                                post_state_examples: Vec::new(),
                                            },
                                        )),
                                    )));
                                }
                                if draw.successors.len() == 1 {
                                    let (_, post) = draw
                                        .successors
                                        .into_iter()
                                        .next()
                                        .ok_or(ProbabilityDocumentError::Malformed)?;
                                    distribution.push(Successor::Position(
                                        draw.probability,
                                        post,
                                        draw.rewards,
                                    ));
                                } else {
                                    distribution.push(Successor::Draw(
                                        draw.probability,
                                        action.identity.clone(),
                                        draw.random,
                                        draw.successors.into_iter().map(|(_, post)| post).collect(),
                                        draw.rewards,
                                    ));
                                }
                            }
                            distributions.push(distribution);
                        }
                        distributions
                    }
                };
                drop(admission);
                let post_raws = distributions
                    .iter()
                    .map(|distribution| vec![None; distribution.len()])
                    .collect();
                let expansion = Rc::new(Expansion {
                    distributions,
                    labels: action_labels,
                    post_raws: RefCell::new(post_raws),
                });
                if let Some(at) = cache_key {
                    // The entry keeps every successor state for the run: charge
                    // the model key, and one state's encoded size per successor.
                    let retained = at.0.len().saturating_mul(1usize.saturating_add(
                        expansion.distributions.iter().map(Vec::len).sum::<usize>(),
                    ));
                    charge(
                        storage,
                        limits.storage_bytes,
                        u64::try_from(retained.saturating_add(at.1.len()).saturating_add(
                            std::mem::size_of::<Successor>().saturating_mul(
                                expansion.distributions.iter().map(Vec::len).sum::<usize>(),
                            ),
                        ))
                        .unwrap_or(u64::MAX),
                    )?;
                    expansions.insert(at, Rc::clone(&expansion));
                }
                expansion
            };
            action_labels = expansion.labels.clone();
            for (choice_index, distribution) in expansion.distributions.iter().enumerate() {
                let mut choice = Vec::new();
                for (successor_index, successor) in distribution.iter().enumerate() {
                    if let Successor::Draw(probability, scheduled, random, posts, rewards) =
                        successor
                    {
                        if probability.is_zero() {
                            continue;
                        }
                        edges = edges
                            .checked_add(1)
                            .ok_or(ProbabilityDocumentError::KeySizeOverflow)?;
                        if edges > limits.edges {
                            return Err(exceeded(
                                ProbabilityDocumentLimit::Edges,
                                u64::try_from(limits.edges).unwrap_or(u64::MAX),
                                u64::try_from(edges).unwrap_or(u64::MAX),
                            ));
                        }
                        let (raw, pending) = {
                            let mut admission = ProbabilityKeyAdmission::new(
                                meter,
                                storage,
                                limits.storage_bytes,
                                cancel,
                            );
                            let identity = fsl_foundation::IdentityLimits::default();
                            let raw = system
                                .state_key_bytes_with_budget(
                                    &model,
                                    identity,
                                    &mut admission,
                                    cancel,
                                )
                                .map_err(|error| admission.refusal(error))?;
                            let pending = system
                                .pending_key_bytes_with_budget(
                                    scheduled,
                                    random,
                                    identity,
                                    &mut admission,
                                    cancel,
                                )
                                .map_err(|error| admission.refusal(error))?;
                            (raw, pending)
                        };
                        let target = measure_probability_product_key(
                            binding,
                            &raw,
                            control,
                            &accumulator,
                            Some(&pending),
                            plan.objective.reward.as_ref(),
                            limits,
                            cancel,
                            meter,
                            storage,
                        )?;
                        charge(
                            storage,
                            limits.storage_bytes,
                            u64::try_from(
                                target
                                    .as_bytes()
                                    .len()
                                    .saturating_mul(3)
                                    .saturating_add(std::mem::size_of::<ProbabilityEdge>()),
                            )
                            .unwrap_or(u64::MAX),
                        )?;
                        if seen.insert(target.clone()) {
                            if scope.refuses(seen.len(), limits.states) {
                                return Err(exceeded(
                                    ProbabilityDocumentLimit::States,
                                    u64::try_from(limits.states).unwrap_or(u64::MAX),
                                    u64::try_from(seen.len()).unwrap_or(u64::MAX),
                                ));
                            }
                            queue.push_back((
                                target.clone(),
                                model.clone(),
                                control,
                                accumulator.clone(),
                                truths.clone(),
                                Some((posts.clone(), rewards.clone())),
                            ));
                        }
                        choice.push(ProbabilityEdge {
                            target,
                            probability: probability.clone(),
                        });
                        continue;
                    }
                    let Successor::Position(probability, post, rewards) = successor else {
                        return Err(ProbabilityDocumentError::Malformed);
                    };
                    poll(cancel)?;
                    if probability.is_zero() {
                        continue;
                    }
                    edges = edges
                        .checked_add(1)
                        .ok_or(ProbabilityDocumentError::KeySizeOverflow)?;
                    if edges > limits.edges {
                        return Err(exceeded(
                            ProbabilityDocumentLimit::Edges,
                            u64::try_from(limits.edges).unwrap_or(u64::MAX),
                            u64::try_from(edges).unwrap_or(u64::MAX),
                        ));
                    }
                    let post_truths = if plan.control.reads_post(Some(control)) {
                        read(post)?
                    } else {
                        Vec::new()
                    };
                    let (next, sums) = plan
                        .control
                        .step(
                            Some(control),
                            Some(&accumulator),
                            &truths,
                            &post_truths,
                            rewards,
                            meter,
                        )
                        .map_err(ProbabilityDocumentError::from)?;
                    let next = next.ok_or(ProbabilityDocumentError::Malformed)?;
                    let sums = sums.ok_or(ProbabilityDocumentError::Malformed)?;
                    let cached_raw =
                        expansion.post_raws.borrow()[choice_index][successor_index].clone();
                    let post_raw = match cached_raw {
                        Some(raw) => raw,
                        None => {
                            let raw = raw_of(post, meter, storage)?;
                            charge(
                                storage,
                                limits.storage_bytes,
                                u64::try_from(raw.len()).unwrap_or(u64::MAX),
                            )?;
                            expansion.post_raws.borrow_mut()[choice_index][successor_index] =
                                Some(raw.clone());
                            raw
                        }
                    };
                    let target = product_key(&post_raw, next, &sums, meter, storage)?;
                    charge(
                        storage,
                        limits.storage_bytes,
                        u64::try_from(
                            target
                                .as_bytes()
                                .len()
                                .saturating_mul(3)
                                .saturating_add(std::mem::size_of::<ProbabilityEdge>()),
                        )
                        .unwrap_or(u64::MAX),
                    )?;
                    if seen.insert(target.clone()) {
                        if scope.refuses(seen.len(), limits.states) {
                            return Err(exceeded(
                                ProbabilityDocumentLimit::States,
                                u64::try_from(limits.states).unwrap_or(u64::MAX),
                                u64::try_from(seen.len()).unwrap_or(u64::MAX),
                            ));
                        }
                        raws.insert(target.clone(), post_raw);
                        queue.push_back((
                            target.clone(),
                            post.clone(),
                            next,
                            sums,
                            post_truths,
                            None,
                        ));
                    }
                    choice.push(ProbabilityEdge {
                        target,
                        probability: probability.clone(),
                    });
                }
                if resolving {
                    let target = choice
                        .first()
                        .ok_or(ProbabilityDocumentError::Malformed)?
                        .target
                        .clone();
                    *action_labels
                        .get_mut(choices.len())
                        .ok_or(ProbabilityDocumentError::Malformed)? =
                        Some(crate::ProductAction::PostState(target));
                }
                choices.push(choice);
            }
        }
        states.insert(
            key,
            State {
                terminal,
                activation_terminal,
                rank,
                choices,
                actions: action_labels,
                activated,
            },
        );
    }
    Ok(Graph {
        initial: root,
        initials,
        states,
    })
}

fn distribution_error(
    error: fsl_eval::simulation::model::ModelKeyExpansionError,
    admission: &mut ProbabilityKeyAdmission<'_>,
) -> ProbabilityDocumentError {
    match error {
        fsl_eval::simulation::model::ModelKeyExpansionError::Key(error) => admission.refusal(error),
        fsl_eval::simulation::model::ModelKeyExpansionError::Expansion(error) => {
            ProbabilityDocumentError::Build(Box::new(crate::ExactProbBuildIssue::Expansion(error)))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireMeasure {
    kind: String,
    window: u64,
    reward: Option<(Box<serde_json::value::RawValue>, String)>,
    unit: Box<serde_json::value::RawValue>,
    comparison: Option<String>,
    threshold: Option<WireRational>,
    level: Option<WireRational>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireObjective {
    profile: String,
    payoff: String,
    measure: WireMeasure,
    optimization: String,
    fairness: Vec<Box<serde_json::value::RawValue>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireDocument {
    identity: WireIdentity,
    objective: WireObjective,
    kind: String,
    values: Vec<WireValue>,
    ranking: Vec<WireRank>,
    conditional: Option<WireConditional>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireConditional {
    mode: String,
    numerator: Option<Box<WireDocument>>,
    denominator: Option<Box<WireDocument>>,
}

fn decode_state(
    text: &str,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    poll(cancel)?;
    charge(
        storage,
        limits.storage_bytes,
        u64::try_from(text.len()).unwrap_or(u64::MAX),
    )?;
    let bytes = URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| ProbabilityDocumentError::MalformedMember("values"))?;
    if URL_SAFE_NO_PAD.encode(&bytes) != text {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    let admission = RefCell::new(writer::Admission::with_used(
        limits.storage_bytes,
        cancel,
        *storage,
    ));
    let parsed = quire_canonical::read_with_reservation(
        &bytes,
        limits.bytes,
        &mut writer::Reserve(&admission),
    );
    *storage = admission.borrow().used();
    if let Some(error) = admission.borrow_mut().take_stop() {
        return Err(error);
    }
    let parsed = parsed.map_err(|_| ProbabilityDocumentError::MalformedMember("values"))?;
    let canonical = writer::encode_reserved(&parsed, limits.bytes, &admission)?;
    *storage = admission.borrow().used();
    if canonical != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    let Node::Array(parts) = parsed.root().node() else {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    };
    let parts = parts.collect::<Vec<_>>();
    if parts.len() != 8
        || !matches!(
            parts[0].node(),
            Node::String("filament.probability-product-key/v1")
        )
        || !matches!(parts[2].node(), Node::Null)
        || !matches!(parts[3].node(), Node::Null)
    {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    }
    if !matches!(parts[5].node(), Node::Array(monitor) if monitor.len() == 3)
        || !matches!(parts[6].node(), Node::Array(_))
    {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    }
    Ok(ProbabilityProductKey::from_canonical_bytes(bytes))
}

fn read_wire(
    wire: WireDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    depth: u8,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    poll(cancel)?;
    if depth > 1 || wire.objective.profile != PROFILE {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let payoff = MeasurePayoffKind::read(&wire.objective.payoff)?;
    let identity = ObligationIdentity::new(
        fsl_foundation::node_wire::node_key_from_text(wire.identity.node_id.get())
            .map_err(|_| ProbabilityDocumentError::MalformedMember("identity"))?,
        wire.identity.model.map(crate::ModelIdentity::new),
        wire.identity.ordinal,
    );
    let objective = wire.objective;
    let measure = objective.measure;
    let reward = measure
        .reward
        .map(|(model, name)| {
            Ok::<_, ProbabilityDocumentError>(ProbabilityRewardObjective {
                model: fsl_foundation::node_wire::node_key_from_text(model.get())
                    .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
                name: filament_exact::Identifier::new(&name)
                    .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
                unit: unit::read(measure.unit.get().as_bytes(), limits, cancel)?,
            })
        })
        .transpose()?;
    if reward.is_none() && measure.unit.get() != "null" {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let parse_rational = |wire, meter: &mut filament_exact::Meter| {
        ratio_checked(wire, limits.scalar_bits, meter, cancel, false)
    };
    let comparison = measure
        .comparison
        .as_deref()
        .map(|text| match text {
            "le" => Ok(MeasureComparison::Le),
            "lt" => Ok(MeasureComparison::Lt),
            "ge" => Ok(MeasureComparison::Ge),
            _ => Err(ProbabilityDocumentError::ObjectiveMismatch),
        })
        .transpose()?;
    let kind = match measure.kind.as_str() {
        "steps" => "steps",
        "accumulate" => "accumulate",
        "fraction" => "fraction",
        "weighted-fraction" => "weighted-fraction",
        _ => return Err(ProbabilityDocumentError::ObjectiveMismatch),
    };
    let objective = Objective {
        kind,
        window: measure.window,
        reward,
        comparison,
        threshold: measure
            .threshold
            .map(|wire| parse_rational(wire, meter))
            .transpose()?,
        level: measure
            .level
            .map(|wire| parse_rational(wire, meter))
            .transpose()?,
        optimization: match objective.optimization.as_str() {
            "minimum" => ProbabilityObjective::Minimum,
            "maximum" => ProbabilityObjective::Maximum,
            "workload" => ProbabilityObjective::Workload,
            _ => return Err(ProbabilityDocumentError::ObjectiveMismatch),
        },
        fairness: fairness::read(&objective.fairness, limits, cancel, meter, storage)?,
    };
    if wire.values.len() > limits.states || wire.ranking.len() > limits.states {
        return Err(exceeded(
            ProbabilityDocumentLimit::States,
            u64::try_from(limits.states).unwrap_or(u64::MAX),
            u64::try_from(wire.values.len().max(wire.ranking.len())).unwrap_or(u64::MAX),
        ));
    }
    let mut values = Vec::new();
    for row in wire.values {
        let state = decode_state(&row.state, limits, cancel, storage)?;
        let value = parse_rational(row.value, meter)?;
        if values
            .last()
            .is_some_and(|previous: &ExactProbabilityValue| previous.state >= state)
        {
            return Err(ProbabilityDocumentError::MalformedMember("values"));
        }
        charge(
            storage,
            limits.storage_bytes,
            u64::try_from(
                std::mem::size_of::<ExactProbabilityValue>().saturating_add(state.as_bytes().len()),
            )
            .unwrap_or(u64::MAX),
        )?;
        values.push(ExactProbabilityValue { state, value });
    }
    let mut ranking = Vec::new();
    for row in wire.ranking {
        let state = decode_state(&row.state, limits, cancel, storage)?;
        if row.rank > MAX_COUNT
            || ranking
                .last()
                .is_some_and(|previous: &(ProbabilityProductKey, u64)| previous.0 >= state)
        {
            return Err(ProbabilityDocumentError::MalformedMember("ranking"));
        }
        ranking.push((state, row.rank));
    }
    let conditional = match (wire.kind.as_str(), wire.conditional) {
        ("exact", None) => None,
        ("conditional", Some(conditional))
            if depth == 0 && values.is_empty() && ranking.is_empty() =>
        {
            Some(Box::new(match conditional.mode.as_str() {
                "ratio" if objective.optimization == ProbabilityObjective::Workload => {
                    Conditional::Ratio(
                        Box::new(read_wire(
                            *conditional
                                .numerator
                                .ok_or(ProbabilityDocumentError::MalformedMember("numerator"))?,
                            limits,
                            cancel,
                            meter,
                            storage,
                            depth + 1,
                        )?),
                        Box::new(read_wire(
                            *conditional
                                .denominator
                                .ok_or(ProbabilityDocumentError::MalformedMember("denominator"))?,
                            limits,
                            cancel,
                            meter,
                            storage,
                            depth + 1,
                        )?),
                    )
                }
                "vacuous"
                    if conditional.numerator.is_none() && conditional.denominator.is_none() =>
                {
                    Conditional::Vacuous
                }
                _ => return Err(ProbabilityDocumentError::MalformedMember("mode")),
            }))
        }
        ("conditional", None) => {
            return Err(ProbabilityDocumentError::MalformedMember("conditional"));
        }
        _ => return Err(ProbabilityDocumentError::MalformedMember("kind")),
    };
    Ok(MeasureProbabilityDocument {
        identity,
        payoff,
        values,
        ranking,
        objective,
        conditional,
    })
}

/// Decode strict canonical finite-horizon measure evidence, without accepting it.
pub(super) fn decode_measure_probability_certificate(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    charge(
        storage,
        limits.storage_bytes,
        u64::try_from(bytes.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(4),
    )?;
    let wire: WireDocument =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let document = read_wire(wire, limits, cancel, meter, storage, 0)?;
    let canonical = encode_with_storage(&document, limits, cancel, storage)?;
    if canonical != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    Ok(document)
}

/// Decode the strict closed profile and validate its terminal-payoff value range.
/// Contextual checking compares the checked objective before this range test.
pub fn read_measure_probability_certificate(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let document =
        decode_measure_probability_certificate(bytes, limits, cancel, &mut meter, &mut 0)?;
    validate_ranges(&document, cancel, &mut meter)?;
    Ok(document)
}
pub(super) fn identity(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ObligationIdentity, ProbabilityDocumentError> {
    Ok(decode_measure_probability_certificate(
        bytes,
        limits,
        cancel,
        &mut filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone()),
        &mut 0,
    )?
    .identity)
}
fn validate_ranges(
    document: &MeasureProbabilityDocument,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<(), ProbabilityDocumentError> {
    let (lower, upper) = if document.payoff == MeasurePayoffKind::Shifted {
        let level = document
            .objective
            .level
            .as_ref()
            .ok_or(ProbabilityDocumentError::ObjectiveMismatch)?;
        (
            arithmetic(
                filament_exact::RationalArithmetic::Negate(level),
                meter,
                cancel,
            )?,
            arithmetic(
                filament_exact::RationalArithmetic::Subtract(&super::rational(1), level),
                meter,
                cancel,
            )?,
        )
    } else {
        (super::rational(0), super::rational(1))
    };
    for row in &document.values {
        poll(cancel)?;
        if ordered(Order::Less, &row.value, &lower, meter, cancel)?
            || ordered(Order::Greater, &row.value, &upper, meter, cancel)?
        {
            return Err(ProbabilityDocumentError::MalformedMember("values"));
        }
    }
    if let Some(conditional) = &document.conditional {
        if let Conditional::Ratio(numerator, denominator) = conditional.as_ref() {
            validate_ranges(numerator, cancel, meter)?;
            validate_ranges(denominator, cancel, meter)?;
        }
    }
    Ok(())
}

struct Events<'a> {
    document: &'a MeasureProbabilityDocument,
    text: writer::Text<'a>,
}
impl Events<'_> {
    fn rational<S: Sink + ?Sized>(
        &self,
        value: &Rational,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("numerator")?;
        self.text.integer(value.numerator(), writer)?;
        writer.name("denominator")?;
        self.text.integer(value.denominator(), writer)?;
        writer.end_object()
    }
}
impl Encode for Events<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.text.poll()?;
        let document = self.document;
        writer.begin_object()?;
        writer.name("identity")?;
        writer.begin_object()?;
        writer.name("node_id")?;
        fsl_foundation::node_wire::encode_node_key(document.identity.node(), writer)?;
        writer.name("model")?;
        match document.identity.model() {
            Some(model) => writer.string(model.as_str())?,
            None => writer.null()?,
        }
        writer.name("ordinal")?;
        writer.integer(i128::from(document.identity.ordinal()))?;
        writer.end_object()?;
        writer.name("objective")?;
        writer.begin_object()?;
        writer.name("profile")?;
        writer.string(PROFILE)?;
        writer.name("payoff")?;
        writer.string(document.payoff.text())?;
        writer.name("measure")?;
        writer.begin_object()?;
        writer.name("kind")?;
        writer.string(document.objective.kind)?;
        writer.name("window")?;
        writer.integer(i128::from(document.objective.window))?;
        writer.name("reward")?;
        match &document.objective.reward {
            None => writer.null()?,
            Some(reward) => {
                writer.begin_array()?;
                fsl_foundation::node_wire::encode_node_key(&reward.model, writer)?;
                writer.string(reward.name.as_str())?;
                writer.end_array()?;
            }
        }
        writer.name("unit")?;
        self.text.unit(
            document
                .objective
                .reward
                .as_ref()
                .and_then(|reward| reward.unit.as_ref()),
            writer,
        )?;
        writer.name("comparison")?;
        match document.objective.comparison {
            None => writer.null()?,
            Some(comparison) => writer.string(match comparison {
                MeasureComparison::Le => "le",
                MeasureComparison::Lt => "lt",
                MeasureComparison::Ge => "ge",
            })?,
        }
        writer.name("threshold")?;
        match &document.objective.threshold {
            None => writer.null()?,
            Some(value) => self.rational(value, writer)?,
        }
        writer.name("level")?;
        match &document.objective.level {
            None => writer.null()?,
            Some(value) => self.rational(value, writer)?,
        }
        writer.end_object()?;
        writer.name("optimization")?;
        writer.string(match document.objective.optimization {
            ProbabilityObjective::Minimum => "minimum",
            ProbabilityObjective::Maximum => "maximum",
            ProbabilityObjective::Workload => "workload",
            ProbabilityObjective::WorkloadMinimum => "workload-minimum",
            ProbabilityObjective::WorkloadMaximum => "workload-maximum",
        })?;
        writer.name("fairness")?;
        writer.begin_array()?;
        for constraint in &document.objective.fairness {
            self.text.poll()?;
            constraint.encode_into(writer)?;
        }
        writer.end_array()?;
        writer.end_object()?;
        writer.name("kind")?;
        writer.string(if document.conditional.is_some() {
            "conditional"
        } else {
            "exact"
        })?;
        writer.name("values")?;
        writer.begin_array()?;
        for (index, row) in document.values.iter().enumerate() {
            writer.begin_object()?;
            writer.name("state")?;
            self.text.key(&row.state, writer, index != 0)?;
            writer.name("value")?;
            self.rational(&row.value, writer)?;
            writer.end_object()?;
        }
        writer.end_array()?;
        writer.name("ranking")?;
        writer.begin_array()?;
        for (index, (state, rank)) in document.ranking.iter().enumerate() {
            writer.begin_object()?;
            writer.name("state")?;
            self.text.key(state, writer, index != 0)?;
            writer.name("rank")?;
            writer.integer(i128::from(*rank))?;
            writer.end_object()?;
        }
        writer.end_array()?;
        if let Some(conditional) = &document.conditional {
            writer.name("conditional")?;
            writer.begin_object()?;
            writer.name("mode")?;
            match conditional.as_ref() {
                Conditional::Vacuous => writer.string("vacuous")?,
                Conditional::Ratio(numerator, denominator) => {
                    writer.string("ratio")?;
                    writer.name("numerator")?;
                    Events {
                        document: numerator,
                        text: writer::Text::new(self.text.limits(), self.text.cancel()),
                    }
                    .encode_into(writer)?;
                    writer.name("denominator")?;
                    Events {
                        document: denominator,
                        text: writer::Text::new(self.text.limits(), self.text.cancel()),
                    }
                    .encode_into(writer)?;
                }
            }
            writer.end_object()?;
        }
        writer.end_object()
    }
}
pub(super) fn encode(
    document: &MeasureProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    encode_with_storage(document, limits, cancel, &mut 0)
}
fn encode_with_storage(
    document: &MeasureProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let admission = RefCell::new(writer::Admission::with_used(
        limits.storage_bytes,
        cancel,
        *storage,
    ));
    let events = Events {
        document,
        text: writer::Text::new(limits, cancel),
    };
    let result = writer::encode_reserved(&events, limits.bytes, &admission);
    *storage = admission.borrow().used();
    if let Some(error) = events
        .text
        .take_stop()
        .or_else(|| admission.borrow_mut().take_stop())
    {
        return Err(error);
    }
    result
}
/// Write canonical unchecked finite-horizon measure proof bytes.
pub fn write_measure_probability_certificate(
    document: &MeasureProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let bytes = encode(document, limits, cancel)?;
    read_measure_probability_certificate(&bytes, limits, cancel)?;
    Ok(bytes)
}

/// Build an unchecked exact measure document from the producer's values.
/// Ranks, objective and reachable coverage come from independent reconstruction;
/// the offered values are copied without solving any equation.
#[allow(clippy::too_many_arguments)]
pub fn measure_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    payoff: MeasurePayoffKind,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    producer_document(
        system,
        claims,
        item,
        certificate,
        payoff,
        false,
        limits,
        cancel,
    )
}

/// [`measure_probability_certificate_document`], or, where `max_states` stops
/// the whole graph, the document of a `lower` proof ranking the states
/// `certificate` names, over the states it names and their successors
/// (FR-1042 "Document"). Only a `shifted` payoff and a comparison event have
/// that part. This does not grant acceptance.
pub fn partial_lower_measure_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    payoff: MeasurePayoffKind,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    producer_document(
        system,
        claims,
        item,
        certificate,
        payoff,
        true,
        limits,
        cancel,
    )
}

#[allow(clippy::too_many_arguments)]
fn producer_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    payoff: MeasurePayoffKind,
    partial_lower: bool,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    let claim = bind_claim(claims, item, cancel)?;
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let plan = plan(
        system,
        claim,
        payoff,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    let graph = whole_or_lower_part(
        system,
        claim,
        &plan,
        partial_lower.then_some((payoff, certificate.values.as_slice())),
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    Ok(MeasureProbabilityDocument {
        identity: item.clone(),
        payoff,
        values: certificate.values.clone(),
        ranking: graph
            .states
            .into_iter()
            .filter_map(|(key, state)| {
                certificate
                    .values
                    .iter()
                    .any(|value| value.state == key)
                    .then_some((key, state.rank))
            })
            .collect(),
        objective: plan.objective,
        conditional: None,
    })
}

/// Build unchecked value-free quantile evidence without computing discarded
/// numeric values or ranks. Only the independent checker proves vacuity.
pub fn vacuous_measure_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    let claim = bind_claim(claims, item, cancel)?;
    if !matches!(claim.form, ProbForm::Quantile { .. }) {
        return Err(ProbabilityDocumentError::MalformedMember("mode"));
    }
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let selected = plan(
        system,
        claim,
        MeasurePayoffKind::Satisfied,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    Ok(MeasureProbabilityDocument {
        identity: item.clone(),
        payoff: MeasurePayoffKind::Satisfied,
        values: Vec::new(),
        ranking: Vec::new(),
        objective: selected.objective,
        conditional: Some(Box::new(Conditional::Vacuous)),
    })
}

/// Combine exact numerator and denominator documents into a workload ratio.
/// Both parts remain independently checked before any ratio is accepted.
pub fn conditional_measure_probability_certificate(
    numerator: MeasureProbabilityDocument,
    denominator: MeasureProbabilityDocument,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    if numerator.identity != denominator.identity
        || numerator.payoff != MeasurePayoffKind::Satisfied
        || denominator.payoff != MeasurePayoffKind::Activated
        || numerator.conditional.is_some()
        || denominator.conditional.is_some()
        || numerator.objective.optimization != ProbabilityObjective::Workload
        || denominator.objective.optimization != ProbabilityObjective::Workload
    {
        return Err(ProbabilityDocumentError::MalformedMember("mode"));
    }
    Ok(MeasureProbabilityDocument {
        identity: numerator.identity.clone(),
        payoff: numerator.payoff,
        values: Vec::new(),
        ranking: Vec::new(),
        objective: numerator.objective.clone(),
        conditional: Some(Box::new(Conditional::Ratio(
            Box::new(numerator),
            Box::new(denominator),
        ))),
    })
}

fn checked_values(
    document: &MeasureProbabilityDocument,
    graph: &Graph,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    partial_floor: Option<&Rational>,
) -> Result<
    Result<BTreeMap<ProbabilityProductKey, Rational>, ExactProbabilityCertificateCheck>,
    ProbabilityDocumentError,
> {
    let shape = |member| {
        ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::Malformed,
            at: ProbabilityCertificateLocus::Member(member),
        })
    };
    let keys = document
        .values
        .iter()
        .map(|row| &row.state)
        .collect::<Vec<_>>();
    if (partial_floor.is_none() && document.values.len() != graph.states.len())
        || keys.iter().any(|key| !graph.states.contains_key(*key))
        || keys.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Ok(Err(shape("values")));
    }
    if document.ranking.len() != document.values.len()
        || document
            .ranking
            .iter()
            .map(|row| &row.0)
            .ne(keys.iter().copied())
    {
        return Ok(Err(shape("ranking")));
    }
    let values = document
        .values
        .iter()
        .map(|row| (row.state.clone(), row.value.clone()))
        .collect::<BTreeMap<_, _>>();
    for row in &document.values {
        let key = &row.state;
        let Some(state) = graph.states.get(key) else {
            return Ok(Err(shape("values")));
        };
        poll(cancel)?;
        if document
            .ranking
            .binary_search_by(|row| row.0.cmp(key))
            .ok()
            .map(|index| document.ranking[index].1)
            != Some(state.rank)
        {
            return Ok(Err(rejected(
                ProbabilityCertificateRule::RankBroken,
                key.clone(),
            )));
        }
        let expected = match &state.terminal {
            Some(value) => value.clone(),
            None => {
                let mut optimum: Option<Rational> = None;
                for choice in &state.choices {
                    let sum = match partial_floor {
                        Some(floor) => {
                            partial_lower_action_sum(choice, &values, floor, meter, cancel)?
                        }
                        None => action_sum(choice, &values, meter, cancel)?,
                    };
                    optimum = Some(match optimum {
                        None => sum,
                        Some(old) => match document.objective.optimization {
                            ProbabilityObjective::Minimum => {
                                if ordered(Order::Less, &old, &sum, meter, cancel)? {
                                    old
                                } else {
                                    sum
                                }
                            }
                            ProbabilityObjective::Maximum => {
                                if ordered(Order::Greater, &old, &sum, meter, cancel)? {
                                    old
                                } else {
                                    sum
                                }
                            }
                            ProbabilityObjective::Workload
                            | ProbabilityObjective::WorkloadMinimum
                            | ProbabilityObjective::WorkloadMaximum => {
                                return Err(ProbabilityDocumentError::Malformed);
                            }
                        },
                    });
                }
                optimum.ok_or(ProbabilityDocumentError::Malformed)?
            }
        };
        check_value_bits(&expected, limits.scalar_bits)?;
        let holds = match partial_floor {
            Some(_) => ordered(
                Order::LessOrEqual,
                values.get(key).ok_or(ProbabilityDocumentError::Malformed)?,
                &expected,
                meter,
                cancel,
            )?,
            None => values.get(key) == Some(&expected),
        };
        if !holds {
            return Ok(Err(rejected(
                ProbabilityCertificateRule::InequalityFails,
                key.clone(),
            )));
        }
    }
    Ok(Ok(values))
}
fn check_value_bits(value: &Rational, bits: u64) -> Result<(), ProbabilityDocumentError> {
    let actual = value
        .numerator()
        .magnitude_bits()
        .max(value.denominator().magnitude_bits());
    if actual > bits {
        return Err(exceeded(
            ProbabilityDocumentLimit::RationalBits,
            bits,
            actual,
        ));
    }
    Ok(())
}

fn action_sum(
    choice: &[ProbabilityEdge],
    values: &BTreeMap<ProbabilityProductKey, Rational>,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<Rational, ProbabilityDocumentError> {
    let mut sum = super::rational(0);
    let mut mass = super::rational(0);
    for edge in choice {
        poll(cancel)?;
        if edge.probability < super::rational(0) {
            return Err(ProbabilityDocumentError::Malformed);
        }
        let value = values
            .get(&edge.target)
            .ok_or(ProbabilityDocumentError::MalformedMember("values"))?;
        let term = arithmetic(
            filament_exact::RationalArithmetic::Multiply(&edge.probability, value),
            meter,
            cancel,
        )?;
        sum = arithmetic(
            filament_exact::RationalArithmetic::Add(&sum, &term),
            meter,
            cancel,
        )?;
        mass = arithmetic(
            filament_exact::RationalArithmetic::Add(&mass, &edge.probability),
            meter,
            cancel,
        )?;
    }
    if mass != super::rational(1) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(sum)
}

fn partial_lower_action_sum(
    choice: &[ProbabilityEdge],
    values: &BTreeMap<ProbabilityProductKey, Rational>,
    floor: &Rational,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<Rational, ProbabilityDocumentError> {
    let mut sum = super::rational(0);
    let mut mass = super::rational(0);
    for edge in choice {
        poll(cancel)?;
        if edge.probability < super::rational(0) {
            return Err(ProbabilityDocumentError::Malformed);
        }
        let value = values.get(&edge.target).unwrap_or(floor);
        let term = arithmetic(
            filament_exact::RationalArithmetic::Multiply(&edge.probability, value),
            meter,
            cancel,
        )?;
        sum = arithmetic(
            filament_exact::RationalArithmetic::Add(&sum, &term),
            meter,
            cancel,
        )?;
        mass = arithmetic(
            filament_exact::RationalArithmetic::Add(&mass, &edge.probability),
            meter,
            cancel,
        )?;
    }
    if mass != super::rational(1) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(sum)
}
/// The first activated state the tight actions reach from the initial state,
/// breadth-first with each layer in canonical order (FR-201's locus).
fn tight_activation(
    graph: &Graph,
    values: &BTreeMap<ProbabilityProductKey, Rational>,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<Option<ProbabilityProductKey>, ProbabilityDocumentError> {
    let mut seen = BTreeSet::from([graph.initial.clone()]);
    let mut layer = BTreeSet::from([graph.initial.clone()]);
    while !layer.is_empty() {
        let mut next = BTreeSet::new();
        for key in &layer {
            poll(cancel)?;
            let state = graph
                .states
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            if state.activated {
                return Ok(Some(key.clone()));
            }
            for choice in &state.choices {
                if values.get(key) != Some(&action_sum(choice, values, meter, cancel)?) {
                    continue;
                }
                for edge in choice {
                    if !edge.probability.is_zero() && seen.insert(edge.target.clone()) {
                        next.insert(edge.target.clone());
                    }
                }
            }
        }
        layer = next;
    }
    Ok(None)
}
/// For a `>= c` quantile whose initial shifted maximum is exactly 0, the first
/// activated state its tight actions reach: some scheduler may then meet the
/// level while activating (FR-1042 "Pairing"). Tight actions are those that
/// attain the document's own values. Each route names its own rejection.
#[implements("FR-1042-AC-28")]
fn tight_zero_activation(
    plan: &Plan<'_>,
    graph: &Graph,
    values: &BTreeMap<ProbabilityProductKey, Rational>,
    initial: &Rational,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<Option<ProbabilityProductKey>, ProbabilityDocumentError> {
    if !(plan.strict && initial.is_zero()) {
        return Ok(None);
    }
    tight_activation(graph, values, meter, cancel)
}
fn acceptance(
    document: &MeasureProbabilityDocument,
    value: Rational,
    side: ProbabilityBoundSide,
    bytes: &[u8],
) -> ExactProbabilityCertificateCheck {
    ExactProbabilityCertificateCheck::Accepted(ExactProbabilityAcceptance {
        initial_values: None,
        bound: None,
        policy: Vec::new(),
        value,
        side,
        certificate: ExactProbabilityCertificate {
            values: document.values.clone(),
        },
        document: Some(bytes.to_vec().into_boxed_slice()),
    })
}

#[implements("FR-1042-AC-27", "FR-1042-AC-29", "FR-1042-AC-30", "FR-1043-AC-10")]
pub(super) fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    options: CheckOptions<'_>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let CheckOptions {
        partial_lower,
        bound_side,
        role,
        limits,
        cancel,
    } = options;
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let document = match decode_measure_probability_certificate(
        bytes,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    ) {
        Ok(document) => document,
        Err(error) => return document_admission_error(error),
    };
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    if !std::ptr::eq(system.input().probabilistic.claims(), claims) {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cancel)?;
    let plan = plan(
        system,
        claim,
        document.payoff,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    if plan.objective != document.objective {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    if document.conditional.is_none() && bound_side.is_none() {
        if let Err(error) = validate_ranges(&document, cancel, &mut meter) {
            return document_admission_error(error);
        }
    }
    if let Some(conditional) = &document.conditional {
        if matches!(conditional.as_ref(), Conditional::Vacuous) {
            if !matches!(claim.form, ProbForm::Quantile { .. })
                || document.payoff != MeasurePayoffKind::Satisfied
            {
                return Err(ProbabilityDocumentError::MalformedMember("mode"));
            }
            let graph = enumerate(
                system,
                claim,
                &plan,
                limits,
                cancel,
                &mut meter,
                &mut storage,
                Scope::Whole,
            )?;
            if graph.states.values().any(|state| state.activated) {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    graph.initial,
                ));
            }
            poll(cancel)?;
            return Ok(ExactProbabilityCertificateCheck::AcceptedVacuous(
                ProbabilityVacuityAcceptance {
                    document: bytes.to_vec().into_boxed_slice(),
                    side: if plan.strict {
                        ProbabilityBoundSide::AtLeast
                    } else {
                        ProbabilityBoundSide::AtMost
                    },
                },
            ));
        }
        let Conditional::Ratio(numerator, denominator) = conditional.as_ref() else {
            return Err(ProbabilityDocumentError::MalformedMember("mode"));
        };
        if !matches!(claim.scheduler, ProbScheduler::Workload(_))
            || !matches!(claim.form, ProbForm::Quantile { .. })
            || numerator.payoff != MeasurePayoffKind::Satisfied
            || denominator.payoff != MeasurePayoffKind::Activated
            || numerator.identity != document.identity
            || denominator.identity != document.identity
        {
            return Err(ProbabilityDocumentError::MalformedMember("conditional"));
        }
        let mut graph = None;
        let mut checked_parts = Vec::new();
        for (part, part_name) in [
            (numerator, ProbabilityCertificatePart::Numerator),
            (denominator, ProbabilityCertificatePart::Denominator),
        ] {
            let part_plan = plan_for_part(
                system,
                claim,
                part,
                limits,
                cancel,
                &mut meter,
                &mut storage,
            )?;
            if let Err(error) = validate_ranges(part, cancel, &mut meter) {
                return document_admission_error(error);
            }
            if graph.is_none() {
                graph = Some(enumerate(
                    system,
                    claim,
                    &part_plan,
                    limits,
                    cancel,
                    &mut meter,
                    &mut storage,
                    Scope::Whole,
                )?);
            } else {
                // Both independently checked objectives name the same measure and
                // therefore the same stopped control, transitions and complete keys.
                // The denominator uses the shared evaluator's retained activation payoff.
                let existing = graph.as_mut().ok_or(ProbabilityDocumentError::Malformed)?;
                for state in existing.states.values_mut() {
                    poll(cancel)?;
                    meter
                        .charge(
                            filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                                .work(Integer::one()),
                        )
                        .map_err(|error| ProbabilityDocumentError::Accounting(Box::new(error)))?;
                    state.terminal = state.activation_terminal.take();
                }
            }
            let graph = graph.as_ref().ok_or(ProbabilityDocumentError::Malformed)?;
            let values = match checked_values(part, graph, limits, cancel, &mut meter, None)? {
                Ok(values) => values,
                Err(mut rejection) => {
                    if let ExactProbabilityCertificateCheck::Rejected(failure) = &mut rejection {
                        if let ProbabilityCertificateLocus::ProductState(key) = &failure.at {
                            failure.at = ProbabilityCertificateLocus::ProductPart {
                                key: key.clone(),
                                part: part_name,
                            };
                        }
                    }
                    return Ok(rejection);
                }
            };
            checked_parts.push(values);
        }
        let [numerator_values, denominator_values] = checked_parts.as_slice() else {
            return Err(ProbabilityDocumentError::Malformed);
        };
        let denominator_graph = graph.as_ref().ok_or(ProbabilityDocumentError::Malformed)?;
        let threshold = match &plan.bound {
            Bound::AtLeast(value) | Bound::AtMost(value) => value,
        };
        let mut coverage = Vec::new();
        let mut least: Option<Rational> = None;
        for (index, key) in &denominator_graph.initials {
            charge(
                &mut storage,
                limits.storage_bytes,
                u64::try_from(std::mem::size_of::<(u32, Option<Rational>)>()).unwrap_or(u64::MAX),
            )?;
            let numerator = numerator_values
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let denominator = denominator_values
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            if denominator.is_zero() {
                // checked_values proves the full nonnegative activation Bellman
                // system, hence zero at this initial proves no positive activation.
                if !numerator.is_zero() {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                coverage.push((*index, None));
                continue;
            }
            let value = arithmetic(
                filament_exact::RationalArithmetic::Divide(numerator, denominator),
                &mut meter,
                cancel,
            )?;
            check_value_bits(&value, limits.scalar_bits)?;
            charge(
                &mut storage,
                limits.storage_bytes,
                value
                    .numerator()
                    .magnitude_bits()
                    .saturating_add(value.denominator().magnitude_bits())
                    .div_ceil(8),
            )?;
            if !ordered(
                if plan.strict {
                    Order::Less
                } else {
                    Order::GreaterOrEqual
                },
                &value,
                threshold,
                &mut meter,
                cancel,
            )? {
                return Ok(rejected(
                    ProbabilityCertificateRule::ThresholdMissed,
                    key.clone(),
                ));
            }
            let replace = match &least {
                None => true,
                Some(old) => ordered(
                    if plan.strict {
                        Order::Greater
                    } else {
                        Order::Less
                    },
                    &value,
                    old,
                    &mut meter,
                    cancel,
                )?,
            };
            if replace {
                least = Some(value.clone());
            }
            coverage.push((*index, Some(value)));
        }
        let Some(value) = least else {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                denominator_graph.initial.clone(),
            ));
        };
        let mut accepted = acceptance(
            &document,
            value,
            if plan.strict {
                ProbabilityBoundSide::AtLeast
            } else {
                ProbabilityBoundSide::AtMost
            },
            bytes,
        );
        if let ExactProbabilityCertificateCheck::Accepted(token) = &mut accepted {
            let binding = claim_binding(system, claim)?.map(|binding| crate::ExactBinding {
                population: binding.instance.population.node,
                key: binding.instance.key,
            });
            token.initial_values = Some(CheckedInitialValues {
                bindings: vec![binding; coverage.len()],
                instances: vec![None; coverage.len()],
                values: coverage,
            });
        }
        return Ok(accepted);
    }
    if matches!(claim.form, ProbForm::Quantile { .. })
        && document.payoff != MeasurePayoffKind::Shifted
    {
        return Err(ProbabilityDocumentError::MalformedMember("conditional"));
    }
    if super::ValueRelation::of(bound_side, role).misplaced(&plan.certified_bound(document.payoff))
    {
        return Ok(super::misplaced_kind());
    }
    // Only a claim-side `lower` is read over the part of the graph it names.
    let claim_lower = bound_side == Some(super::SubsystemBoundKind::Lower)
        && role == super::CertificateRole::Claim;
    let graph = whole_or_lower_part(
        system,
        claim,
        &plan,
        claim_lower.then_some((document.payoff, document.values.as_slice())),
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    let partial_lower =
        partial_lower || (claim_lower && document.values.len() < graph.states.len());
    // These rules are the claim's, so they hold for an exact document and for
    // either bound alike: a threshold is read at one initial key, and no
    // numeric document stands where activation is unreachable.
    if graph.initials.len() != 1 {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    if matches!(claim.form, ProbForm::Quantile { .. })
        && !graph.states.values().any(|state| state.activated)
    {
        return Ok(rejected(
            ProbabilityCertificateRule::InequalityFails,
            graph.initial,
        ));
    }
    if let Some(side) = bound_side.filter(|_| !partial_lower) {
        return check_value_bounds(
            &document, &graph, &plan, side, role, bytes, limits, cancel, &mut meter,
        );
    }
    let partial_floor = if partial_lower {
        if document.payoff == MeasurePayoffKind::Shifted {
            Some(
                plan.objective
                    .level
                    .as_ref()
                    .ok_or(ProbabilityDocumentError::Malformed)?
                    .neg(),
            )
        } else {
            Some(super::rational(0))
        }
    } else {
        None
    };
    let values = match checked_values(
        &document,
        &graph,
        limits,
        cancel,
        &mut meter,
        partial_floor.as_ref(),
    )? {
        Ok(values) => values,
        Err(rejection) => return Ok(rejection),
    };
    let value = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::Malformed)?
        .clone();
    let (holds, side) = if document.payoff == MeasurePayoffKind::Shifted {
        if let Some(key) =
            tight_zero_activation(&plan, &graph, &values, &value, &mut meter, cancel)?
        {
            return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
        }
        (
            if plan.strict {
                ordered(
                    Order::LessOrEqual,
                    &value,
                    &super::rational(0),
                    &mut meter,
                    cancel,
                )?
            } else {
                ordered(
                    Order::GreaterOrEqual,
                    &value,
                    &super::rational(0),
                    &mut meter,
                    cancel,
                )?
            },
            if plan.strict {
                ProbabilityBoundSide::AtLeast
            } else {
                ProbabilityBoundSide::AtMost
            },
        )
    } else {
        match &plan.bound {
            Bound::AtLeast(threshold) => (
                ordered(Order::GreaterOrEqual, &value, threshold, &mut meter, cancel)?,
                ProbabilityBoundSide::AtLeast,
            ),
            Bound::AtMost(threshold) => (
                ordered(Order::LessOrEqual, &value, threshold, &mut meter, cancel)?,
                ProbabilityBoundSide::AtMost,
            ),
        }
    };
    if !holds {
        return Ok(rejected(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial,
        ));
    }
    Ok(acceptance(&document, value, side, bytes))
}
#[implements("FR-1042-AC-28")]
#[allow(clippy::too_many_arguments)]
fn check_value_bounds(
    document: &MeasureProbabilityDocument,
    graph: &Graph,
    plan: &Plan<'_>,
    side: super::SubsystemBoundKind,
    role: super::CertificateRole,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let malformed = |member| {
        ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::Malformed,
            at: ProbabilityCertificateLocus::Member(member),
        })
    };
    let fail = |key| {
        ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::InequalityFails,
            at: ProbabilityCertificateLocus::ProductState(key),
        })
    };
    if document.conditional.is_some() || document.values.len() != graph.states.len() {
        return Ok(malformed("values"));
    }
    let value_keys = document
        .values
        .iter()
        .map(|row| &row.state)
        .collect::<Vec<_>>();
    if value_keys.windows(2).any(|pair| pair[0] >= pair[1])
        || value_keys
            .iter()
            .any(|key| !graph.states.contains_key(*key))
    {
        return Ok(malformed("values"));
    }
    match side {
        super::SubsystemBoundKind::Lower => {
            if document.ranking.len() != document.values.len()
                || document
                    .ranking
                    .iter()
                    .map(|row| &row.0)
                    .ne(value_keys.iter().copied())
            {
                return Ok(malformed("ranking"));
            }
        }
        super::SubsystemBoundKind::Upper if !document.ranking.is_empty() => {
            return Ok(malformed("ranking"));
        }
        super::SubsystemBoundKind::Upper => {}
    }
    let values = document
        .values
        .iter()
        .map(|row| (row.state.clone(), row.value.clone()))
        .collect::<BTreeMap<_, _>>();
    for row in &document.values {
        check_value_bits(&row.value, limits.scalar_bits)?;
        let state = graph
            .states
            .get(&row.state)
            .ok_or(ProbabilityDocumentError::MalformedMember("values"))?;
        if side == super::SubsystemBoundKind::Lower {
            let rank = document
                .ranking
                .binary_search_by(|entry| entry.0.cmp(&row.state))
                .ok()
                .and_then(|index| document.ranking.get(index))
                .map(|entry| entry.1);
            if rank != Some(state.rank) {
                return Ok(ExactProbabilityCertificateCheck::Rejected(
                    ProbabilityCertificateRejection {
                        rule: ProbabilityCertificateRule::RankBroken,
                        at: ProbabilityCertificateLocus::ProductState(row.state.clone()),
                    },
                ));
            }
        }
        let Some(terminal) = &state.terminal else {
            if state.choices.is_empty() {
                return Err(ProbabilityDocumentError::Malformed);
            }
            let mut action_sums = Vec::with_capacity(state.choices.len());
            for choice in &state.choices {
                action_sums.push(action_sum(choice, &values, meter, cancel)?);
            }
            let all = matches!(
                (side, document.objective.optimization),
                (
                    super::SubsystemBoundKind::Lower,
                    ProbabilityObjective::Minimum
                ) | (
                    super::SubsystemBoundKind::Upper,
                    ProbabilityObjective::Maximum
                ) | (
                    super::SubsystemBoundKind::Lower,
                    ProbabilityObjective::Workload
                ) | (
                    super::SubsystemBoundKind::Upper,
                    ProbabilityObjective::Workload
                )
            );
            let holds = action_sums
                .iter()
                .map(|sum| match side {
                    super::SubsystemBoundKind::Lower => {
                        ordered(Order::LessOrEqual, &row.value, sum, meter, cancel)
                    }
                    super::SubsystemBoundKind::Upper => {
                        ordered(Order::LessOrEqual, sum, &row.value, meter, cancel)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            if if all {
                holds.iter().any(|holds| !holds)
            } else {
                !holds.iter().any(|holds| *holds)
            } {
                return Ok(fail(row.state.clone()));
            }
            continue;
        };
        let contains_terminal = match side {
            super::SubsystemBoundKind::Lower => {
                ordered(Order::LessOrEqual, &row.value, terminal, meter, cancel)?
            }
            super::SubsystemBoundKind::Upper => {
                ordered(Order::GreaterOrEqual, &row.value, terminal, meter, cancel)?
            }
        };
        if !contains_terminal {
            return Ok(fail(row.state.clone()));
        }
    }
    let initial = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::MalformedMember("values"))?;
    let certified = plan.certified_bound(document.payoff);
    if role == super::CertificateRole::Claim {
        let meets = match &certified {
            Bound::AtLeast(threshold) => {
                ordered(Order::GreaterOrEqual, initial, threshold, meter, cancel)?
            }
            Bound::AtMost(threshold) => {
                ordered(Order::LessOrEqual, initial, threshold, meter, cancel)?
            }
        };
        if !meets {
            return Ok(ExactProbabilityCertificateCheck::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::ThresholdMissed,
                    at: ProbabilityCertificateLocus::ProductState(graph.initial.clone()),
                },
            ));
        }
        // A bound is not exact: an activation on its tight actions misses the
        // threshold, which more precision may meet.
        if tight_zero_activation(plan, graph, &values, initial, meter, cancel)?.is_some() {
            return Ok(rejected(
                ProbabilityCertificateRule::ThresholdMissed,
                graph.initial.clone(),
            ));
        }
    }
    let accepted_side = match certified {
        Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
        Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
    };
    Ok(acceptance(document, initial.clone(), accepted_side, bytes))
}

fn plan_for_part<'a>(
    system: &ModelSystem<'_>,
    claim: &'a CheckedProbabilisticClaim,
    document: &MeasureProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<Plan<'a>, ProbabilityDocumentError> {
    let plan = plan(
        system,
        claim,
        document.payoff,
        limits,
        cancel,
        meter,
        storage,
    )?;
    if plan.objective != document.objective || document.conditional.is_some() {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    Ok(plan)
}

fn charge(storage: &mut u64, bound: u64, bytes: u64) -> Result<(), ProbabilityDocumentError> {
    let actual = storage
        .checked_add(bytes)
        .ok_or(ProbabilityDocumentError::KeySizeOverflow)?;
    if actual > bound {
        return Err(exceeded(
            ProbabilityDocumentLimit::StorageBytes,
            bound,
            actual,
        ));
    }
    *storage = actual;
    Ok(())
}

/// Resolve the shared terminal payoff, actual reward descriptor and operator
/// from the checked claim. Producers use this same selection as the checker;
/// only the graph construction and backward induction belong to the engine.
#[allow(clippy::too_many_arguments)]
pub fn measure_probability_objective(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    payoff: MeasurePayoffKind,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<
    (
        MeasurePayoff,
        Option<ProbabilityRewardObjective>,
        ProbabilityObjective,
    ),
    ProbabilityDocumentError,
> {
    let selected = plan(system, claim, payoff, limits, cancel, meter, storage)?;
    Ok((
        selected.payoff,
        selected.objective.reward,
        selected.objective.optimization,
    ))
}

pub(in crate::probability_certificate) struct ReplayControl<'a> {
    pub(in crate::probability_certificate) control: MeasurePlan<'a>,
    pub(in crate::probability_certificate) predicates: Vec<&'a CheckedPredicate>,
    pub(in crate::probability_certificate) payoff: MeasurePayoff,
    pub(in crate::probability_certificate) reward: Option<ProbabilityRewardObjective>,
}
#[allow(clippy::too_many_arguments)]
pub(in crate::probability_certificate) fn replay_control<'a>(
    system: &ModelSystem<'_>,
    claim: &'a CheckedProbabilisticClaim,
    kind: MeasurePayoffKind,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<ReplayControl<'a>, ProbabilityDocumentError> {
    let plan = plan(system, claim, kind, limits, cancel, meter, storage)?;
    Ok(ReplayControl {
        control: plan.control,
        predicates: plan.predicates,
        payoff: plan.payoff,
        reward: plan.objective.reward,
    })
}

impl MeasurePayoffKind {
    /// Initialize the shared measure control against the admitted subject.
    /// `unread` preserves position-zero control before any predicate is read.
    ///
    /// # Errors
    /// Unsupported shape, undefined/refused predicates, or actual caller stops.
    #[allow(clippy::too_many_arguments)]
    pub fn initial_control(
        self,
        system: &ModelSystem<'_>,
        claim: &CheckedProbabilisticClaim,
        unread: bool,
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
        meter: &mut filament_exact::Meter,
        storage: &mut u64,
    ) -> Result<(Option<MonitorState>, Option<Accumulator>), ProbabilityDocumentError> {
        let selected = replay_control(system, claim, self, limits, cancel, meter, storage)?;
        if unread {
            return selected
                .control
                .initial_control(meter)
                .map_err(ProbabilityDocumentError::from);
        }
        let binding = claim_binding(system, claim)?;
        let reference = binding.as_ref().map(|binding| &binding.reference);
        let initial = system.initial();
        let [initial] = initial.as_slice() else {
            return Err(ProbabilityDocumentError::Unsupported);
        };
        let inputs = selected
            .predicates
            .iter()
            .map(|predicate| (*predicate, reference))
            .collect::<Vec<_>>();
        let truths = system
            .state_predicates_with_cancel(initial, &inputs, cancel)
            .map_err(|error| {
                ProbabilityDocumentError::Build(Box::new(crate::ExactProbBuildIssue::Expansion(
                    error,
                )))
            })?
            .into_iter()
            .map(|reading| match reading {
                PredicateReading::True => Ok(true),
                PredicateReading::False => Ok(false),
                PredicateReading::Refused(outcome) => Err(ProbabilityDocumentError::Build(
                    Box::new(crate::ExactProbBuildIssue::Predicate(outcome)),
                )),
                PredicateReading::Undefined { cause, .. } => {
                    Err(ProbabilityDocumentError::UndefinedPredicate(cause))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        selected
            .control
            .start(&truths, meter)
            .map_err(ProbabilityDocumentError::from)
    }
}

fn key_parse_error(
    error: quire_canonical::ReadError,
    identity: fsl_foundation::IdentityLimits,
) -> ModelKeyError {
    match error {
        quire_canonical::ReadError::ReservationDenied { .. } => ModelKeyError::Denied,
        quire_canonical::ReadError::Allocation { requested } => {
            ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::Allocation { requested })
        }
        quire_canonical::ReadError::Limit(limit) => {
            ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::Limit(
                fsl_foundation::diagnostic::LimitExceeded::new(
                    fsl_foundation::Setting::IdentityInputBytes,
                    identity.input_bytes,
                    u128::from(limit.required),
                ),
            ))
        }
        other => ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::NoEncoding(
            other.to_string(),
        )),
    }
}

/// Internal role result; it cannot be used as a proof acceptance token.
pub(in crate::probability_certificate) enum MeanRefutationCheck {
    Accepted,
    AcceptedBounds(Vec<(ProbabilityProductKey, crate::ProductAction)>),
    Rejected(ProbabilityCertificateRejection),
    PolicyParity,
}
#[allow(clippy::too_many_arguments)]
pub(in crate::probability_certificate) fn check_mean_refutation(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    item: &ObligationIdentity,
    bytes: &[u8],
    scheduler: &[(ProbabilityProductKey, crate::ProductAction)],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeanRefutationCheck, ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    if bounds::is_bound(bytes)? {
        return bounds::check(system, claim, item, bytes, scheduler, limits, cancel);
    }
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let document =
        decode_measure_probability_certificate(bytes, limits, cancel, &mut meter, &mut storage)?;
    if document.identity != *item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    if document.conditional.is_some() || document.payoff != MeasurePayoffKind::Fraction {
        return Err(ProbabilityDocumentError::MalformedMember("kind"));
    }
    let selected = plan(
        system,
        claim,
        MeasurePayoffKind::Fraction,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    if selected.objective != document.objective {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    validate_ranges(&document, cancel, &mut meter)?;
    let graph = enumerate(
        system,
        claim,
        &selected,
        limits,
        cancel,
        &mut meter,
        &mut storage,
        Scope::Whole,
    )?;
    let values = match checked_values(&document, &graph, limits, cancel, &mut meter, None)? {
        Ok(values) => values,
        Err(ExactProbabilityCertificateCheck::Rejected(rejection)) => {
            return Ok(MeanRefutationCheck::Rejected(rejection));
        }
        Err(_) => return Err(ProbabilityDocumentError::Malformed),
    };
    if selected.objective.optimization == ProbabilityObjective::Workload {
        if !scheduler.is_empty() {
            return Ok(MeanRefutationCheck::PolicyParity);
        }
    } else {
        let policy = scheduler
            .iter()
            .map(|(key, action)| (key, action))
            .collect::<BTreeMap<_, _>>();
        for (key, action) in scheduler {
            poll(cancel)?;
            let Some(state) = graph.states.get(key) else {
                return Ok(MeanRefutationCheck::PolicyParity);
            };
            let Some(index) = state
                .actions
                .iter()
                .position(|candidate| candidate.as_ref() == Some(action))
            else {
                return Ok(MeanRefutationCheck::PolicyParity);
            };
            if state.terminal.is_some()
                || action_sum(
                    state
                        .choices
                        .get(index)
                        .ok_or(ProbabilityDocumentError::Malformed)?,
                    &values,
                    &mut meter,
                    cancel,
                )? != *values.get(key).ok_or(ProbabilityDocumentError::Malformed)?
            {
                return Ok(MeanRefutationCheck::PolicyParity);
            }
        }
        let mut queue = VecDeque::from([graph.initial.clone()]);
        let mut seen = BTreeSet::from([graph.initial.clone()]);
        while let Some(key) = queue.pop_front() {
            poll(cancel)?;
            let state = graph
                .states
                .get(&key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            if state.terminal.is_some() {
                continue;
            }
            let index = if let Some(action) = policy.get(&key) {
                state
                    .actions
                    .iter()
                    .position(|candidate| candidate.as_ref() == Some(*action))
                    .ok_or(ProbabilityDocumentError::Malformed)?
            } else if state.choices.len() == 1 {
                0
            } else {
                return Ok(MeanRefutationCheck::PolicyParity);
            };
            for edge in state
                .choices
                .get(index)
                .ok_or(ProbabilityDocumentError::Malformed)?
            {
                if !edge.probability.is_zero() && seen.insert(edge.target.clone()) {
                    charge(
                        &mut storage,
                        limits.storage_bytes,
                        u64::try_from(edge.target.as_bytes().len().saturating_mul(2))
                            .unwrap_or(u64::MAX),
                    )?;
                    queue.push_back(edge.target.clone());
                }
            }
        }
    }
    let value = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    let refutes = match &selected.bound {
        Bound::AtLeast(threshold) => ordered(Order::Less, value, threshold, &mut meter, cancel)?,
        Bound::AtMost(threshold) => ordered(Order::Greater, value, threshold, &mut meter, cancel)?,
    };
    if !refutes {
        return Ok(MeanRefutationCheck::Rejected(
            ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::ThresholdMissed,
                at: ProbabilityCertificateLocus::ProductState(graph.initial),
            },
        ));
    }
    poll(cancel)?;
    Ok(MeanRefutationCheck::Accepted)
}

#[cfg(test)]
mod partial_lower_tests {
    use super::*;

    fn limits() -> ProbabilityDocumentLimits {
        ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 8_000_000,
            states: 100,
            edges: 100,
            scalar_bits: 128,
            accounting: crate::AccountingLimits::default().0,
        }
    }

    #[test]
    fn unnamed_shifted_successor_uses_negative_level() {
        let root = ProbabilityProductKey::from_canonical_bytes(vec![1]);
        let retained = ProbabilityProductKey::from_canonical_bytes(vec![2]);
        let omitted = ProbabilityProductKey::from_canonical_bytes(vec![3]);
        let half = Rational::new(Integer::one(), Integer::from(2_i64)).unwrap();
        let mut states = BTreeMap::new();
        states.insert(
            root.clone(),
            State {
                terminal: None,
                activation_terminal: None,
                rank: 1,
                choices: vec![vec![
                    ProbabilityEdge {
                        target: retained.clone(),
                        probability: half.clone(),
                    },
                    ProbabilityEdge {
                        target: omitted.clone(),
                        probability: half,
                    },
                ]],
                actions: vec![None],
                activated: true,
            },
        );
        states.insert(
            retained.clone(),
            State {
                terminal: Some(super::super::rational(0)),
                activation_terminal: None,
                rank: 0,
                choices: Vec::new(),
                actions: Vec::new(),
                activated: true,
            },
        );
        states.insert(
            omitted,
            State {
                terminal: Some(super::super::rational(0)),
                activation_terminal: None,
                rank: 0,
                choices: Vec::new(),
                actions: Vec::new(),
                activated: true,
            },
        );
        let graph = Graph {
            initial: root.clone(),
            initials: vec![(0, root.clone())],
            states,
        };
        let mut document = MeasureProbabilityDocument {
            identity: ObligationIdentity::new(
                filament_exact::NodeKey::new(
                    "partial",
                    vec![filament_exact::PathSegment::Text("lower".into())],
                    0,
                )
                .unwrap(),
                None,
                0,
            ),
            payoff: MeasurePayoffKind::Shifted,
            // `retained` is named at its decided value 0; `omitted` is not
            // named, so it counts as `-level` (FR-1042 "Document").
            values: vec![
                ExactProbabilityValue {
                    state: root.clone(),
                    value: Rational::new(Integer::from(-1_i64), Integer::from(5_i64)).unwrap(),
                },
                ExactProbabilityValue {
                    state: retained.clone(),
                    value: Rational::from_integer(Integer::zero()),
                },
            ],
            ranking: vec![(root.clone(), 1), (retained.clone(), 0)],
            objective: Objective {
                kind: "steps",
                window: 1,
                reward: None,
                comparison: Some(MeasureComparison::Lt),
                threshold: None,
                level: Some(Rational::new(Integer::from(2_i64), Integer::from(5_i64)).unwrap()),
                optimization: ProbabilityObjective::Minimum,
                fairness: Vec::new(),
            },
            conditional: None,
        };
        let floor = document.objective.level.as_ref().unwrap().neg();
        let mut meter = filament_exact::Meter::new(limits().accounting);
        let result = checked_values(
            &document,
            &graph,
            limits(),
            &Cancel::new(),
            &mut meter,
            Some(&floor),
        )
        .unwrap();
        assert!(
            matches!(result, Ok(values) if values.get(&root) == Some(&document.values[0].value))
        );
        document.values[0].value = Rational::from_integer(Integer::zero());
        let mut meter = filament_exact::Meter::new(limits().accounting);
        let result = checked_values(
            &document,
            &graph,
            limits(),
            &Cancel::new(),
            &mut meter,
            Some(&floor),
        )
        .unwrap();
        assert!(matches!(
            result,
            Err(ExactProbabilityCertificateCheck::Rejected(_))
        ));
        // With `retained` unnamed too, both successors count as `-level`, so
        // `-1/5` is no longer below the sum.
        document.values.truncate(1);
        document.ranking.truncate(1);
        document.values[0].value =
            Rational::new(Integer::from(-1_i64), Integer::from(5_i64)).unwrap();
        let mut meter = filament_exact::Meter::new(limits().accounting);
        let result = checked_values(
            &document,
            &graph,
            limits(),
            &Cancel::new(),
            &mut meter,
            Some(&floor),
        )
        .unwrap();
        assert!(matches!(
            result,
            Err(ExactProbabilityCertificateCheck::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::InequalityFails,
                    ..
                }
            ))
        ));
    }
}
