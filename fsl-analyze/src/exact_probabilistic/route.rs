// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-038 RL-7 "Declared data": EN-5's own reading of the conditions the
//! route admission table tests, and its own time-lock search. The table is
//! [`fsl_eval::simulation::model::en5_admission`]; the certificate checker
//! establishes the same conditions with its own code.

use std::collections::BTreeSet;

use filament_exact::{Cancel, CancelCause, ValueType};
use fsl_eval::simulation::model::{
    en5_admission, Admission, ClaimForm, DelayClass, ModelSystem, PredicateReading, ReadFrom,
    RegionCause, RouteConditions, TimedSystem,
};
use fsl_replay::ModelCheckLimit;
use fsl_route::deadlock::{ItemObligation, ObligationKind};
use fsl_route::reductions::{ObligationReductions, Reductions};
use fsl_semantics::check::{
    BinaryTemporal, Bound, BoundedMeasure, CheckedInterval, CheckedPredicate,
    CheckedProbabilisticClaim, CheckedTemporalFormula, ClaimActivation, DelayDistribution, Event,
    FormulaNode, FractionMeasure, ProbForm, ReachTarget, UnaryTemporal, Weighting,
};
use ix_trace_rs::implements;
use num_traits::Zero;

use super::{digital, ExactProbLimits, ProductStop};
use crate::zone_check::{
    check_timed_model, ZoneCheckItem, ZoneCheckLimit, ZoneCheckLimits, ZoneCheckOutcome,
    ZoneCheckRequest, ZoneStop, ZoneUndecided,
};

/// How EN-5 reads one claim over a timed subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TimedRoute {
    /// The timed product builds it: digital clocks where the table admits
    /// the claim, and for a strict constraint, interval or `M < c` the
    /// refined grid walk of FR-922-AC-4.
    Product,
    /// The claim needs a region process that is not built.
    RegionNotBuilt(RegionCause),
    /// A budget or the cancellation stopped the time-lock search.
    Stopped(ProductStop),
}

/// The route of `claim` over `timed`, or `None` where the table does not
/// apply: a continuous delay, which the product refuses as
/// `DelayDistribution`.
#[implements("FSpec-FR-421-AC-8", "FR-921-AC-5")]
pub(crate) fn timed_route(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Option<TimedRoute> {
    let conditions = conditions(system, timed, claim, cancel)?;
    Some(match en5_admission(conditions) {
        Admission::Digital | Admission::Region(RegionCause::Strict) => TimedRoute::Product,
        Admission::Region(cause) => TimedRoute::RegionNotBuilt(cause),
        Admission::DigitalIfNoTimeLock => match time_lock(timed, limits, cancel) {
            TimeLockFact::None => TimedRoute::Product,
            TimeLockFact::ReachableOrUnknown => TimedRoute::RegionNotBuilt(RegionCause::TimeLock),
            TimeLockFact::Stopped(stop) => TimedRoute::Stopped(stop),
        },
    })
}

/// Each condition a row of the table tests, as EN-5 reads it.
fn conditions(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    cancel: &Cancel,
) -> Option<RouteConditions> {
    let delays = delay_class(system, claim)?;
    let (form, takes_event_maximum) = claim_form(claim);
    let read_from = match &claim.activation {
        ClaimActivation::Origin => match elapsed_from(claim) {
            Some(from) if !holds_at_every_initial(system, timed, claim, from, cancel) => {
                ReadFrom::Later
            }
            _ => ReadFrom::Origin,
        },
        _ => ReadFrom::Later,
    };
    Some(RouteConditions {
        // FR-204 "Strict constraints" and FR-877 (`M < c`): the one reading
        // the product's own scale and certificate use.
        closed: !digital::region_route(timed, claim),
        delays,
        fairness: !claim.fairness.is_empty(),
        form,
        read_from,
        takes_event_maximum,
    })
}

/// `given` when every operation of the subject has a `discrete` delay,
/// `free` when none has a delay, `mixed` otherwise; `None` for a continuous
/// delay.
fn delay_class(system: &ModelSystem<'_>, claim: &CheckedProbabilisticClaim) -> Option<DelayClass> {
    let mut given = BTreeSet::new();
    for delay in system.input().probabilistic.delays() {
        let operation = delay.operation();
        if operation.model() != claim.subject.alias.as_str() {
            continue;
        }
        if !matches!(delay.distribution(), DelayDistribution::Discrete(_)) {
            return None;
        }
        if let Some(text) = system.operation_text(operation.declaring(), operation.name()) {
            given.insert(text);
        }
    }
    if given.is_empty() {
        return Some(DelayClass::Free);
    }
    Some(
        if system
            .operations()
            .iter()
            .all(|(_, _, text)| given.contains(text))
        {
            DelayClass::Given
        } else {
            DelayClass::Mixed
        },
    )
}

/// The form of `claim` and whether its value is the maximum of a reach,
/// until or deadline event (the minimum of the matching `always`).
fn claim_form(claim: &CheckedProbabilisticClaim) -> (ClaimForm, bool) {
    let at_most = |bound: &Bound| matches!(bound, Bound::AtMost(_));
    match &claim.form {
        ProbForm::Reach { bound, target } => {
            let always = matches!(target, ReachTarget::Always(_));
            (ClaimForm::Reachability, at_most(bound) != always)
        }
        ProbForm::ExpectedReward { .. } => (ClaimForm::ExpectedAccumulate, false),
        ProbForm::MeanTime { .. } => (ClaimForm::ExpectedElapsed, false),
        ProbForm::LongRunFraction { weight, .. } => (
            match weight {
                Some(Weighting::Delay) => ClaimForm::DelayWeightedFraction,
                Some(Weighting::Reward(_)) | None => ClaimForm::DiscreteStepMeasure,
            },
            false,
        ),
        ProbForm::MeanOfFraction { fraction, .. } => (fraction_form(fraction), false),
        ProbForm::Quantile {
            measure, threshold, ..
        } => (measure_form(measure, threshold), false),
        ProbForm::Probability { bound, event } => match event {
            Event::Compare {
                measure,
                bound: compared,
            } => (measure_form(measure, compared), false),
            Event::Formula { formula, .. } => match event_shape(formula) {
                Some((form, always)) => (form, at_most(bound) != always),
                None => (untimed_or_other(formula), false),
            },
        },
    }
}

fn measure_form(measure: &BoundedMeasure, compared: &Bound) -> ClaimForm {
    let horizon = |timed: bool| {
        if timed {
            ClaimForm::TimeHorizonMeasure
        } else {
            ClaimForm::DiscreteStepMeasure
        }
    };
    match measure {
        BoundedMeasure::Elapsed { .. } => match compared {
            Bound::AtMost(_) => ClaimForm::ElapsedAtMost,
            Bound::AtLeast(_) => ClaimForm::OtherElapsed,
        },
        BoundedMeasure::Accumulate { timed_within, .. }
        | BoundedMeasure::Steps { timed_within, .. } => horizon(timed_within.is_some()),
        BoundedMeasure::Fraction(fraction) => fraction_form(fraction),
    }
}

fn fraction_form(fraction: &FractionMeasure) -> ClaimForm {
    match (&fraction.weight, fraction.timed_window.is_some()) {
        (Some(Weighting::Delay), _) => ClaimForm::DelayWeightedFraction,
        (Some(Weighting::Reward(_)), true) => ClaimForm::TimeHorizonMeasure,
        (None, true) => ClaimForm::UnweightedTimeFraction,
        (_, false) => ClaimForm::DiscreteStepMeasure,
    }
}

/// A reachability or until event over state predicates, or a deadline event
/// `[0, D]`, and whether it is an `always`; `None` for any other formula.
fn event_shape(formula: &CheckedTemporalFormula) -> Option<(ClaimForm, bool)> {
    let formula = formula.formula();
    let nodes = formula.nodes();
    let holds = |id: fsl_semantics::check::FormulaNodeId| {
        matches!(nodes.get(id.index()), Some(FormulaNode::Holds { .. }))
    };
    let shape = |interval: &Option<CheckedInterval>| match interval {
        None => Some(ClaimForm::Reachability),
        Some(CheckedInterval::Timed(interval))
            if interval.lower().numerator().as_big().is_zero() && interval.upper().is_some() =>
        {
            Some(ClaimForm::DeadlineEvent)
        }
        Some(_) => None,
    };
    match nodes.get(formula.root().index())? {
        FormulaNode::Unary {
            operator,
            interval,
            operand,
        } if holds(*operand) => match operator {
            UnaryTemporal::Eventually => Some((shape(interval)?, false)),
            UnaryTemporal::Always => Some((shape(interval)?, true)),
            UnaryTemporal::Once | UnaryTemporal::Historically => None,
        },
        FormulaNode::Binary {
            operator: BinaryTemporal::Until,
            interval,
            left,
            right,
        } if holds(*left) && holds(*right) => Some((shape(interval)?, false)),
        _ => None,
    }
}

/// A formula that is no single event: a step-bounded one reads only the
/// discrete steps; any other, a timed window included, does not.
fn untimed_or_other(formula: &CheckedTemporalFormula) -> ClaimForm {
    let nodes = formula.formula().nodes();
    let interval = |node: &FormulaNode| match node {
        FormulaNode::Unary { interval, .. } | FormulaNode::Binary { interval, .. } => {
            interval.clone()
        }
        _ => None,
    };
    let step_bounded = nodes
        .iter()
        .filter_map(interval)
        .all(|interval| matches!(interval, CheckedInterval::Steps(_)))
        && nodes.iter().filter_map(interval).next().is_some();
    if step_bounded {
        ClaimForm::DiscreteStepMeasure
    } else {
        ClaimForm::OtherFormula
    }
}

/// `A` of an elapsed measure, whose first position the measure starts at.
fn elapsed_from(claim: &CheckedProbabilisticClaim) -> Option<&CheckedPredicate> {
    match &claim.form {
        ProbForm::Quantile {
            measure: BoundedMeasure::Elapsed { from, .. },
            ..
        }
        | ProbForm::Probability {
            event:
                Event::Compare {
                    measure: BoundedMeasure::Elapsed { from, .. },
                    ..
                },
            ..
        } => Some(from),
        _ => None,
    }
}

/// Whether `from` is not false at any initial state, for any `over` binding,
/// so the measure starts at position 0 on every run. An initial state where
/// it has no value ends the run there, which the product reports; any
/// failure to read it counts as a later start.
fn holds_at_every_initial(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    from: &CheckedPredicate,
    cancel: &Cancel,
) -> bool {
    let bindings = match &claim.over {
        None => vec![None],
        Some(over) => {
            let ValueType::Reference(object) = &over.value_type else {
                return false;
            };
            let Ok(instances) = system.instances(&over.value_type) else {
                return false;
            };
            let mut bindings = Vec::with_capacity(instances.len());
            for instance in instances {
                let Ok(reference) = system.written_reference(object.clone(), &instance) else {
                    return false;
                };
                bindings.push(Some(reference));
            }
            bindings
        }
    };
    timed.initial().iter().all(|initial| {
        bindings.iter().all(|binding| {
            system
                .state_predicates_with_cancel(
                    initial.discrete(),
                    &[(from, binding.as_ref())],
                    cancel,
                )
                .is_ok_and(|readings| !readings.contains(&PredicateReading::False))
        })
    })
}

enum TimeLockFact {
    /// No time-lock is reachable over the dense runs.
    None,
    /// One is, or the search did not establish that none is.
    ReachableOrUnknown,
    Stopped(ProductStop),
}

/// Whether a time-lock is reachable over the subject's dense runs, by
/// EN-6's time-lock-freedom search on zones (never by the digital product).
fn time_lock(
    timed: &TimedSystem<'_, '_>,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> TimeLockFact {
    let max_states = u64::try_from(limits.model.limits.max_states).unwrap_or(u64::MAX);
    let max_transitions = u64::try_from(limits.model.limits.max_transitions).unwrap_or(u64::MAX);
    let outcome = check_timed_model(
        ZoneCheckRequest {
            subject: timed,
            item: ZoneCheckItem::TimeLockFreedom,
            obligation: ItemObligation {
                subject: (),
                kind: ObligationKind::TimeLockFreedom,
                reductions: ObligationReductions::default(),
            },
            limits: ZoneCheckLimits {
                max_symbolic_states: max_states,
                max_symbolic_transitions: max_transitions,
                ..ZoneCheckLimits::default()
            },
            reductions: &Reductions::default(),
        },
        cancel,
    );
    let stopped = |limit, value| TimeLockFact::Stopped(ProductStop::Limit { limit, value });
    match outcome {
        Ok(
            ZoneCheckOutcome::Holds { .. }
            | ZoneCheckOutcome::Undecided {
                cause: ZoneUndecided::NoInitialState,
                ..
            },
        ) => TimeLockFact::None,
        Ok(ZoneCheckOutcome::Stopped { cause, .. }) => match cause {
            ZoneStop::LimitReached(ZoneCheckLimit::MaxSymbolicStates(value)) => {
                stopped(ModelCheckLimit::ExactProbMaxStates, value)
            }
            ZoneStop::LimitReached(ZoneCheckLimit::MaxSymbolicTransitions(value)) => {
                stopped(ModelCheckLimit::ExactProbMaxTransitions, value)
            }
            ZoneStop::LimitReached(_) => TimeLockFact::ReachableOrUnknown,
            ZoneStop::Cancelled => TimeLockFact::Stopped(ProductStop::Cancelled(
                cancel.tripped().unwrap_or(CancelCause::Requested),
            )),
        },
        _ => TimeLockFact::ReachableOrUnknown,
    }
}
