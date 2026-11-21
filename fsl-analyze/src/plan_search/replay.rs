// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-650's native replay: every reported plan is re-applied from the
//! bound initial state through [`ModelSystem`], each step enabled, every
//! state's invariants checked, every policy and the goal re-evaluated and
//! the objective recomputed, before it is reported. A plan that fails replay
//! is a method failure naming the first failing step.

use filament_exact::Rational;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::TransitionSystem;

use super::{
    zero, MonitorStep, ObjectiveKind, PlanLengths, PlanRules, PlanSearchError, PlanStep, PlanView,
    ReportedPlan,
};

/// The operation name `step` invokes, read from the system's operations.
pub fn operation_name<'s>(system: &'s ModelSystem<'_>, step: &ModelTransition) -> Option<&'s str> {
    system
        .operations()
        .into_iter()
        .find(|(_, _, text)| *text == step.operation())
        .map(|(_, name, _)| name)
}

/// Replay `plan` natively from the bound initial state and return its
/// objective value, recomputed from `kind`'s rule.
///
/// # Errors
/// [`PlanSearchError::ReplayFailed`] naming the first step that is not an
/// enabled invocation reaching the recorded state,
/// [`PlanSearchError::ReplayRejected`] when an invariant, a policy or the
/// goal fails, the length is not in `lengths` or the objective is undefined,
/// and [`PlanSearchError::RuleStopped`] when an evaluation is stopped.
pub fn replay<R: PlanRules>(
    system: &ModelSystem<'_>,
    rules: &mut R,
    plan: &ReportedPlan,
    lengths: PlanLengths,
    kind: ObjectiveKind,
) -> Result<Rational, PlanSearchError> {
    let Some(start) = system.initial().into_iter().next() else {
        return Err(PlanSearchError::InitialStates(0));
    };
    let key = |state: &ModelSimState| {
        system
            .state_key(state)
            .map(|(key, _)| key)
            .map_err(|_| PlanSearchError::StateKey)
    };
    if plan.states.first().map(key).transpose()? != Some(key(&start)?)
        || plan.states.len() != plan.steps.len() + 1
    {
        return Err(PlanSearchError::ReplayRejected(
            "the plan does not start at the initial state",
        ));
    }
    for (index, step) in plan.steps.iter().enumerate() {
        let expected = key(&plan.states[index + 1])?;
        let reached = if let Some(operations) = rules.native_operations() {
            selected_outcome(
                system,
                &plan.states[index],
                &operations,
                |candidate| candidate == step.invocation(),
                |ordinal, state| Ok(ordinal == step.outcome_ordinal() && key(state)? == expected),
            )?
            .is_some()
        } else {
            if step.outcome_ordinal().is_some() {
                return Err(PlanSearchError::ReplayRejected(
                    "an outcome ordinal is not admitted by this native integration",
                ));
            }
            let expansion = system
                .successors(&plan.states[index])
                .map_err(PlanSearchError::Expansion)?;
            let mut reached = false;
            for (candidate, state) in &expansion.successors {
                if candidate == step.invocation() && key(state)? == expected {
                    reached = true;
                    break;
                }
            }
            reached
        };
        if !reached {
            return Err(PlanSearchError::ReplayFailed {
                step: index,
                operation: operation_name(system, step)
                    .map(str::to_owned)
                    .unwrap_or_else(|| step.operation().to_owned()),
            });
        }
    }
    for state in &plan.states {
        match super::admission(system, state).map_err(|stop| native_failure(system, stop))? {
            super::Admission::Admitted => {}
            super::Admission::Violated => {
                return Err(PlanSearchError::ReplayRejected(
                    "a state of the plan breaks an invariant",
                ));
            }
            // Not admissible evidence: the run stays `incomplete`.
            super::Admission::Undetermined(invariant) => {
                return Err(PlanSearchError::InvariantUndetermined(invariant));
            }
        }
    }
    check_rules(rules, plan)?;
    if !u64::try_from(plan.steps.len()).is_ok_and(|length| lengths.contains(length)) {
        return Err(PlanSearchError::ReplayRejected(
            "the plan length is not a value of the domain",
        ));
    }
    objective_value(rules, plan, kind)
}

/// FR-755's plan substitution seam: build the plan `operations` names, each
/// step the first enabled invocation of that operation in step order, and
/// replay it. Returns the plan and its replayed objective value.
///
/// # Errors
/// [`PlanSearchError::ReplayFailed`] at the first operation with no enabled
/// invocation, and every error [`replay`] gives.
pub fn substitute<R: PlanRules>(
    system: &ModelSystem<'_>,
    rules: &mut R,
    operations: &[String],
    lengths: PlanLengths,
    kind: ObjectiveKind,
) -> Result<(ReportedPlan, Rational), PlanSearchError> {
    let Some(start) = system.initial().into_iter().next() else {
        return Err(PlanSearchError::InitialStates(0));
    };
    let mut plan = ReportedPlan {
        states: vec![start],
        steps: Vec::new(),
    };
    for (index, name) in operations.iter().enumerate() {
        let Some(current) = plan.states.last() else {
            return Err(PlanSearchError::InitialStates(0));
        };
        let selected = if let Some(operations) = rules.native_operations() {
            selected_outcome(
                system,
                current,
                &operations,
                |candidate| operation_name(system, candidate) == Some(name.as_str()),
                |_, _| Ok(true),
            )?
        } else {
            let expansion = system
                .successors(current)
                .map_err(PlanSearchError::Expansion)?;
            let mut candidates: Vec<_> = expansion
                .successors
                .into_iter()
                .filter(|(step, _)| operation_name(system, step) == Some(name.as_str()))
                .map(|(step, state)| (rules.step_key(&step), step, state))
                .collect();
            candidates.sort_by(|left, right| left.0.cmp(&right.0));
            candidates
                .into_iter()
                .next()
                .map(|(_, step, state)| (PlanStep::from(step), state))
        };
        let Some((step, state)) = selected else {
            return Err(PlanSearchError::ReplayFailed {
                step: index,
                operation: name.clone(),
            });
        };
        plan.steps.push(step);
        plan.states.push(state);
    }
    let value = replay(system, rules, &plan, lengths, kind)?;
    Ok((plan, value))
}

/// Re-derive the selected invocation through the shared ordered native path.
/// The ordinal is checked against the actual number and order of outcomes,
/// including the absence rule for a single outcome.
fn selected_outcome(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    operations: &[(filament_exact::EffectiveId, String)],
    mut invocation: impl FnMut(&ModelTransition) -> bool,
    mut outcome: impl FnMut(Option<u64>, &ModelSimState) -> Result<bool, PlanSearchError>,
) -> Result<Option<(PlanStep, ModelSimState)>, PlanSearchError> {
    use fsl_eval::simulation::model::{
        ClauseOutcome, NativeCandidateControl as Control, NativeCandidateEvent as Event,
    };
    let mut selected = None;
    let mut failure = None;
    system
        .visit_plan_candidates(state, operations, None, &mut |event| match event {
            Event::Begin(step) => {
                if invocation(step) {
                    Control::Continue
                } else {
                    Control::Skip
                }
            }
            Event::Completed {
                transition,
                state,
                outcome: ordinal,
            } => match outcome(ordinal, state) {
                Ok(true) => {
                    selected = Some((PlanStep::new(transition.clone(), ordinal), state.clone()));
                    Control::Stop
                }
                Ok(false) => Control::Continue,
                Err(error) => {
                    failure = Some(error);
                    Control::Stop
                }
            },
            Event::Disabled(_) => Control::Continue,
            Event::Undetermined { outcome, .. } => match outcome {
                ClauseOutcome::KernelUndefined(_) | ClauseOutcome::FamilyUndefined(_) => {
                    Control::Continue
                }
                ClauseOutcome::Incomplete(incomplete) => {
                    failure = Some(PlanSearchError::RuleStopped(super::RuleStop::Accounting {
                        counter: incomplete.limit_kind.as_str(),
                        value: incomplete.limit,
                    }));
                    Control::Stop
                }
                ClauseOutcome::KernelRefused(_)
                | ClauseOutcome::FamilyRefused(_)
                | ClauseOutcome::NotBoolean
                | ClauseOutcome::Stopped(_) => {
                    failure = Some(PlanSearchError::ReplayRejected(
                        "a native invocation refused during replay",
                    ));
                    Control::Stop
                }
            },
        })
        .map_err(|stop| native_failure(system, stop))?;
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(selected)
}

fn native_failure(
    system: &ModelSystem<'_>,
    stop: fsl_eval::simulation::ExpansionStop,
) -> PlanSearchError {
    match stop.limit {
        Some(fsl_eval::simulation::ExpansionLimit::EvaluationMeter(kind)) => {
            PlanSearchError::RuleStopped(super::RuleStop::Accounting {
                counter: kind.as_str(),
                value: super::accounting_limit(system, kind),
            })
        }
        Some(fsl_eval::simulation::ExpansionLimit::MaxCandidates) | None => {
            PlanSearchError::Expansion(stop)
        }
    }
}

fn rejected() -> PlanSearchError {
    PlanSearchError::ReplayRejected("a policy or the goal fails on the plan")
}

/// Re-evaluate every policy and the goal on a replayed plan.
fn check_rules<R: PlanRules>(rules: &mut R, plan: &ReportedPlan) -> Result<(), PlanSearchError> {
    let (Some(first), Some(last)) = (plan.states.first(), plan.states.last()) else {
        return Err(rejected());
    };
    for policy in 0..rules.policy_count() {
        let mut monitor = match rules
            .policy_start(policy, first)
            .map_err(PlanSearchError::RuleStopped)?
        {
            MonitorStep::Continue(monitor) => monitor,
            MonitorStep::Reject => return Err(rejected()),
        };
        for state in &plan.states[1..] {
            monitor = match rules
                .policy_advance(policy, &monitor, state)
                .map_err(PlanSearchError::RuleStopped)?
            {
                MonitorStep::Continue(next) => next,
                MonitorStep::Reject => return Err(rejected()),
            };
        }
        if !rules
            .policy_accepts(policy, &monitor, last)
            .map_err(PlanSearchError::RuleStopped)?
        {
            return Err(rejected());
        }
    }
    let view = PlanView {
        states: &plan.states,
        steps: &plan.steps,
    };
    if !rules.goal(&view).map_err(PlanSearchError::RuleStopped)? {
        return Err(rejected());
    }
    Ok(())
}

/// The plan's objective, recomputed: the sum of its step costs or the
/// objective expression. An undefined value is not admissible (FSpec
/// FR-802).
fn objective_value<R: PlanRules>(
    rules: &mut R,
    plan: &ReportedPlan,
    kind: ObjectiveKind,
) -> Result<Rational, PlanSearchError> {
    let undefined = PlanSearchError::ReplayRejected("the plan's objective is undefined");
    let value = match kind {
        ObjectiveKind::General => rules
            .objective(&plan.view())
            .map_err(PlanSearchError::RuleStopped)?,
        ObjectiveKind::StepSum => {
            let mut sum = Some(super::StepTotal::from_rational(zero()));
            for index in 0..plan.steps.len() {
                let view = PlanView {
                    states: &plan.states[..=index + 1],
                    steps: &plan.steps[..=index],
                };
                let cost = rules
                    .step_cost_typed(&view, index)
                    .map_err(PlanSearchError::RuleStopped)?;
                sum = match (sum, cost) {
                    (Some(total), Some(cost)) => rules
                        .extend_step_sum(&view, &total, &cost)
                        .map_err(PlanSearchError::RuleStopped)?,
                    _ => None,
                };
            }
            match sum {
                Some(total) => rules
                    .step_sum_objective(&plan.view(), &total.value)
                    .map_err(PlanSearchError::RuleStopped)?,
                None => None,
            }
        }
    };
    value.ok_or(undefined)
}
