// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-921's independent timed-product certificate admission.

use super::*;
use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{One, ToPrimitive};
use std::collections::{BTreeMap, VecDeque};

use filament_exact::{Integer, Rational};
use fsl_eval::simulation::model::{ClockValuation, ModelSimState, TimedState, TimedSystem};
use num_traits::Zero;
use serde_json::{json, Value as JsonValue};

pub(super) fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let document = read_document_profile(bytes, limits, cancel, &mut meter, false, None, &mut 0)?;
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cancel)?;
    let Some(objective) = document.timed.as_ref() else {
        return Err(ProbabilityDocumentError::MalformedMember("objective"));
    };
    let timed = fsl_eval::simulation::model::TimedSystem::new(system, &claim.subject.alias)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
    let route = if region_route(&timed, claim) {
        TimedProbabilityRoute::Region
    } else {
        TimedProbabilityRoute::Digital
    };
    let scale = certificate_scale(system, &timed, claim, route)?;
    if objective.route != route
        || objective.unit != timed.source().unit_node
        || objective.scale != scale
        || objective.form != TimedProbabilityForm::Deadline
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    if route != TimedProbabilityRoute::Region {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let Some((bound, predicate)) = deadline_event(claim) else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let optimization = match (&claim.scheduler, &bound) {
        (fsl_semantics::check::ProbScheduler::Every, Bound::AtLeast(_)) => {
            ProbabilityObjective::Minimum
        }
        (fsl_semantics::check::ProbScheduler::Every, Bound::AtMost(_)) => {
            ProbabilityObjective::Maximum
        }
        (fsl_semantics::check::ProbScheduler::Workload(_), _) => ProbabilityObjective::Workload,
    };
    if document.optimization != optimization || !document.ranking.is_empty() {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let binding = claim_binding(system, claim)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?
        .map(|binding| binding.reference);
    let graph = enumerate_region_deadline(
        system,
        &timed,
        claim,
        binding.as_ref(),
        predicate,
        objective.scale,
        scale,
        limits,
        cancel,
    )?;
    if target_keys(&graph) != document.targets {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let certificate = ExactProbabilityCertificate {
        values: document.values,
    };
    let mut checked = super::super::check_exact_reachability(
        &graph,
        &certificate,
        optimization,
        &bound,
        cancel,
    )
    .map_err(ProbabilityDocumentError::Graph)?;
    if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut checked {
        accepted.document = Some(bytes.into());
    }
    Ok(checked)
}

fn deadline_event(
    claim: &CheckedProbabilisticClaim,
) -> Option<(Bound, &fsl_semantics::check::CheckedPredicate)> {
    let fsl_semantics::check::ProbForm::Probability { bound, event } = &claim.form else {
        return None;
    };
    let fsl_semantics::check::Event::Formula {
        formula,
        timed_horizon: Some(_),
        ..
    } = event
    else {
        return None;
    };
    let root = formula.formula().root();
    let fsl_semantics::check::FormulaNode::Unary {
        operator: fsl_semantics::check::UnaryTemporal::Eventually,
        interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
        operand,
    } = formula.formula().nodes().get(root.index())?
    else {
        return None;
    };
    if !interval.lower().is_zero() || !interval.lower_closed() || !interval.upper_closed() {
        return None;
    }
    let fsl_semantics::check::FormulaNode::Holds { .. } =
        formula.formula().nodes().get(operand.index())?
    else {
        return None;
    };
    Some((bound.clone(), formula.condition(operand.index())?))
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Phase {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Clone)]
struct RegionState {
    timed: TimedState,
    elapsed: u64,
    phase: Phase,
}

fn enumerate_region_deadline(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    binding: Option<&filament_exact::ObjectReference>,
    predicate: &fsl_semantics::check::CheckedPredicate,
    key_scale: u64,
    scale: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<CheckerGraph, ProbabilityDocumentError> {
    let (base_scale, subdivisions) = region_scale_parts(timed, claim, scale)?;
    let deadline = time_bound(claim, key_scale)?;
    let mut caps = Vec::with_capacity(timed.places().len());
    for place in timed.places() {
        let mut largest = 0_u64;
        for constraint in clock_constraints(timed)
            .into_iter()
            .filter(|constraint| constraint.clock() == place.clock())
        {
            largest = largest.max(
                time_units(constraint.bound(), key_scale)
                    .ok_or(ProbabilityDocumentError::Unsupported)?,
            );
        }
        caps.push(
            largest
                .checked_add(1)
                .ok_or(ProbabilityDocumentError::Unsupported)?,
        );
    }
    let mut initial = timed.initial();
    if initial.len() != 1 {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let state = initial.pop().ok_or(ProbabilityDocumentError::Unsupported)?;
    let phase = phase_at(
        system,
        predicate,
        binding,
        state.discrete(),
        0,
        deadline,
        cancel,
    )?;
    let first = RegionState {
        timed: state,
        elapsed: 0,
        phase,
    };
    let first_key = region_key(
        system,
        timed,
        binding,
        &first,
        &caps,
        base_scale,
        subdivisions,
        key_scale,
        deadline,
    )?;
    let mut pending = VecDeque::from([first_key.clone()]);
    let mut reached = BTreeMap::from([(first_key.clone(), first)]);
    let mut states = BTreeMap::new();
    let mut edges = 0_u64;
    while let Some(key) = pending.pop_front() {
        if let Some(cause) = cancel.tripped() {
            return Err(ProbabilityDocumentError::Cancelled(cause));
        }
        let current = reached
            .get(&key)
            .cloned()
            .ok_or(ProbabilityDocumentError::Malformed)?;
        match current.phase {
            Phase::Accepted => {
                states.insert(
                    key,
                    CheckerState {
                        terminal: Some(true),
                        choices: Vec::new(),
                    },
                );
                continue;
            }
            Phase::Rejected => {
                states.insert(
                    key,
                    CheckerState {
                        terminal: Some(false),
                        choices: Vec::new(),
                    },
                );
                continue;
            }
            Phase::Pending => {}
        }
        if timed
            .is_quiescent(current.timed.discrete())
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
        {
            // Idle observation adds a distinct scheduler choice. This slice
            // refuses it until the tail has a checker-owned product state.
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let mut choices = Vec::new();
        if let Some((delayed_key, delayed)) = next_region_delay(
            system,
            timed,
            predicate,
            binding,
            &current,
            &caps,
            base_scale,
            subdivisions,
            key_scale,
            deadline,
            cancel,
        )? {
            admit_region_state(
                delayed_key.clone(),
                delayed,
                &mut reached,
                &mut pending,
                limits,
            )?;
            choices.push(vec![ProbabilityEdge {
                target: delayed_key,
                probability: one(),
            }]);
            edges = edges.saturating_add(1);
        }
        let clocked_steps = timed
            .steps(&current.timed)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let actions = system
            .actions_with_cancel(current.timed.discrete(), cancel)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        for action in actions {
            let mut choice = Vec::new();
            let mut enabled = true;
            for draw in action.draws {
                if draw.successors.len() != 1 {
                    return Err(ProbabilityDocumentError::Unsupported);
                }
                let (transition, discrete_post) = &draw.successors[0];
                let Some((_, timed_post)) = clocked_steps.iter().find(|(step, post)| {
                    step.transition() == transition
                        && system
                            .state_key(post.discrete())
                            .ok()
                            .zip(system.state_key(discrete_post).ok())
                            .is_some_and(|(left, right)| left.0 == right.0)
                }) else {
                    enabled = false;
                    break;
                };
                let clocks = quantized_clocks(timed_post, &caps, key_scale)?;
                let post =
                    TimedState::new(timed_post.discrete().clone(), ClockValuation::new(clocks));
                let phase = phase_at(
                    system,
                    predicate,
                    binding,
                    post.discrete(),
                    current.elapsed,
                    deadline,
                    cancel,
                )?;
                let reached_state = RegionState {
                    timed: post,
                    elapsed: current.elapsed,
                    phase,
                };
                let target = region_key(
                    system,
                    timed,
                    binding,
                    &reached_state,
                    &caps,
                    base_scale,
                    subdivisions,
                    key_scale,
                    deadline,
                )?;
                admit_region_state(
                    target.clone(),
                    reached_state,
                    &mut reached,
                    &mut pending,
                    limits,
                )?;
                if let Some(edge) = choice.iter_mut().find(|edge| edge.target == target) {
                    edge.probability = edge.probability.add(&draw.probability);
                } else {
                    choice.push(ProbabilityEdge {
                        target,
                        probability: draw.probability.clone(),
                    });
                }
                edges = edges.saturating_add(1);
            }
            if enabled && !choice.is_empty() {
                choices.push(choice);
            }
        }
        if edges > u64::try_from(limits.edges).unwrap_or(u64::MAX) {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Edges,
                bound: u64::try_from(limits.edges).unwrap_or(u64::MAX),
                actual: edges,
            });
        }
        states.insert(
            key,
            CheckerState {
                terminal: choices.is_empty().then_some(false),
                choices,
            },
        );
    }
    Ok(CheckerGraph {
        initial: first_key,
        states,
    })
}

fn region_scale_parts(
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    full_scale: u64,
) -> Result<(u64, u64), ProbabilityDocumentError> {
    let deadline = matches!(
        &claim.form,
        fsl_semantics::check::ProbForm::Probability {
            event: fsl_semantics::check::Event::Formula {
                timed_horizon: Some(_),
                ..
            },
            ..
        }
    );
    let subdivisions = u64::try_from(timed.places().len())
        .ok()
        .and_then(|clocks| clocks.checked_add(if deadline { 1 } else { 0 }))
        .and_then(|clocks| clocks.checked_add(1))
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    let base = full_scale
        .checked_div(subdivisions)
        .filter(|base| *base > 0)
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    if base.checked_mul(subdivisions) != Some(full_scale) {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    Ok((base, subdivisions))
}

fn time_bound(
    claim: &CheckedProbabilisticClaim,
    scale: u64,
) -> Result<u64, ProbabilityDocumentError> {
    let fsl_semantics::check::ProbForm::Probability {
        event:
            fsl_semantics::check::Event::Formula {
                formula,
                timed_horizon: Some(_),
                ..
            },
        ..
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let root = formula.formula().root();
    let Some(fsl_semantics::check::FormulaNode::Unary {
        interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
        ..
    }) = formula.formula().nodes().get(root.index())
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    let upper = interval
        .upper()
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    time_units(upper, scale).ok_or(ProbabilityDocumentError::Unsupported)
}

fn time_units(value: &Rational, scale: u64) -> Option<u64> {
    let scaled = value.numerator().as_big() * BigInt::from(scale);
    let (quotient, remainder) = scaled.div_rem(value.denominator().as_big());
    remainder.is_zero().then(|| quotient.to_u64()).flatten()
}

fn rational_units(value: u64, scale: u64) -> Result<Rational, ProbabilityDocumentError> {
    Rational::new(Integer::from(value), Integer::from(scale))
        .map_err(|_| ProbabilityDocumentError::Unsupported)
}

fn quantized_clocks(
    state: &TimedState,
    caps: &[u64],
    scale: u64,
) -> Result<Vec<Rational>, ProbabilityDocumentError> {
    state
        .clocks()
        .values()
        .iter()
        .zip(caps)
        .map(|(value, cap)| {
            let value = time_units(value, scale)
                .ok_or(ProbabilityDocumentError::Unsupported)?
                .min(*cap);
            rational_units(value, scale)
        })
        .collect()
}

fn phase_at(
    system: &ModelSystem<'_>,
    predicate: &fsl_semantics::check::CheckedPredicate,
    binding: Option<&filament_exact::ObjectReference>,
    state: &ModelSimState,
    elapsed: u64,
    deadline: u64,
    cancel: &Cancel,
) -> Result<Phase, ProbabilityDocumentError> {
    if elapsed > deadline {
        return Ok(Phase::Rejected);
    }
    let [reading] = system
        .state_predicates_with_cancel(state, &[(predicate, binding)], cancel)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?
        .as_slice()
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    match reading {
        fsl_eval::simulation::model::PredicateReading::True => Ok(Phase::Accepted),
        fsl_eval::simulation::model::PredicateReading::False => Ok(Phase::Pending),
        fsl_eval::simulation::model::PredicateReading::Undefined { .. }
        | fsl_eval::simulation::model::PredicateReading::Refused(_) => {
            Err(ProbabilityDocumentError::Unsupported)
        }
    }
}

fn region_key(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    binding: Option<&filament_exact::ObjectReference>,
    state: &RegionState,
    caps: &[u64],
    base_scale: u64,
    subdivisions: u64,
    full_scale: u64,
    deadline: u64,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    let (discrete_bytes, _) = system
        .state_key(state.timed.discrete())
        .map_err(ProbabilityDocumentError::KeyEncoding)?;
    let discrete: JsonValue =
        serde_json::from_slice(&discrete_bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let binding = binding
        .map(|reference| {
            let text = fsl_eval::simulation::model::reference_canonical_text(reference)
                .map_err(|_| ProbabilityDocumentError::Unsupported)?;
            serde_json::from_str(&text).map_err(|_| ProbabilityDocumentError::Malformed)
        })
        .transpose()?;
    let unit = json!([
        "declared",
        fsl_foundation::node_wire::node_key_to_json(&timed.source().unit_node)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
    ]);
    let mut clocks = Vec::with_capacity(timed.places().len());
    for ((place, value), cap) in timed
        .places()
        .iter()
        .zip(state.timed.clocks().values())
        .zip(caps)
    {
        let field = timed
            .model()
            .clock(place.clock())
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        let owner = field.owner().original();
        let declaring = fsl_semantics::model::key::model_node_key(
            owner.package(),
            "model",
            "object_type",
            owner.node(),
        )
        .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let declaring = fsl_foundation::node_wire::node_key_to_json(&declaring)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let receiver = fsl_eval::simulation::model::reference_canonical_text(place.object())
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let receiver: JsonValue =
            serde_json::from_str(&receiver).map_err(|_| ProbabilityDocumentError::Malformed)?;
        let identity = json!([declaring, field.name(), receiver]);
        let value = time_units(value, full_scale).ok_or(ProbabilityDocumentError::Unsupported)?;
        clocks.push(RegionClockKey {
            identity,
            value,
            cap: *cap,
        });
    }
    let monitor = match state.phase {
        Phase::Pending => "pending",
        Phase::Accepted => "accept",
        Phase::Rejected => "reject",
    };
    region_probability_product_key(RegionProductKeyInput {
        binding,
        discrete,
        unit,
        scale: base_scale,
        subdivisions,
        clocks,
        deadline: Some(RegionDeadlineKey {
            limit: deadline,
            elapsed: state.elapsed,
        }),
        tail: DigitalTail::Running,
        monitor: Some(json!(["deadline", "eventually", monitor])),
        accumulator: None,
        pending: None,
        kind: RegionKeyKind::Region,
        corner: None,
    })
    .map_err(ProbabilityDocumentError::Graph)
}

fn next_region_delay(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    predicate: &fsl_semantics::check::CheckedPredicate,
    binding: Option<&filament_exact::ObjectReference>,
    current: &RegionState,
    caps: &[u64],
    base_scale: u64,
    subdivisions: u64,
    scale: u64,
    deadline: u64,
    cancel: &Cancel,
) -> Result<Option<(ProbabilityProductKey, RegionState)>, ProbabilityDocumentError> {
    let current_key = region_key(
        system,
        timed,
        binding,
        current,
        caps,
        base_scale,
        subdivisions,
        scale,
        deadline,
    )?;
    let delta = rational_units(1, scale)?;
    let mut state = current.clone();
    while state.elapsed <= deadline {
        if let Some(cause) = cancel.tripped() {
            return Err(ProbabilityDocumentError::Cancelled(cause));
        }
        let Some(delayed) = timed
            .delay(&state.timed, &delta)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?
        else {
            return Ok(None);
        };
        let elapsed = state
            .elapsed
            .saturating_add(1)
            .min(deadline.saturating_add(1));
        let clocks = quantized_clocks(&delayed, caps, scale)?;
        let delayed = TimedState::new(delayed.discrete().clone(), ClockValuation::new(clocks));
        let phase = phase_at(
            system,
            predicate,
            binding,
            delayed.discrete(),
            elapsed,
            deadline,
            cancel,
        )?;
        let reached = RegionState {
            timed: delayed,
            elapsed,
            phase,
        };
        let key = region_key(
            system,
            timed,
            binding,
            &reached,
            caps,
            base_scale,
            subdivisions,
            scale,
            deadline,
        )?;
        if key != current_key {
            return Ok(Some((key, reached)));
        }
        state = reached;
    }
    Ok(None)
}

fn admit_region_state(
    key: ProbabilityProductKey,
    state: RegionState,
    reached: &mut BTreeMap<ProbabilityProductKey, RegionState>,
    pending: &mut VecDeque<ProbabilityProductKey>,
    limits: ProbabilityDocumentLimits,
) -> Result<(), ProbabilityDocumentError> {
    if reached.contains_key(&key) {
        return Ok(());
    }
    let required = u64::try_from(reached.len())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    if required > u64::try_from(limits.states).unwrap_or(u64::MAX) {
        return Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::States,
            bound: u64::try_from(limits.states).unwrap_or(u64::MAX),
            actual: required,
        });
    }
    reached.insert(key.clone(), state);
    pending.push_back(key);
    Ok(())
}

fn one() -> Rational {
    Rational::from_integer(Integer::from(1))
}

/// The checker derives the route from the checked clock constraints and
/// temporal intervals; the objective's route string is never authoritative.
fn region_route(
    timed: &fsl_eval::simulation::model::TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
) -> bool {
    clock_constraints(timed).into_iter().any(|constraint| {
        matches!(
            constraint.op(),
            fsl_semantics::check::ClockOp::Less | fsl_semantics::check::ClockOp::Greater
        )
    }) || matches!(
        &claim.form,
        fsl_semantics::check::ProbForm::Probability {
            event: fsl_semantics::check::Event::Formula { formula, .. },
            ..
        } if formula.formula().nodes().iter().any(|node| match node {
            fsl_semantics::check::FormulaNode::Unary {
                interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
                ..
            }
            | fsl_semantics::check::FormulaNode::Binary {
                interval: Some(fsl_semantics::check::CheckedInterval::Timed(interval)),
                ..
            } => !interval.upper_closed() || !interval.lower_closed(),
            _ => false,
        })
    )
}

fn clock_constraints<'a>(
    timed: &'a fsl_eval::simulation::model::TimedSystem<'_, '_>,
) -> Vec<&'a fsl_semantics::check::ClockConstraint> {
    fn part_constraints<'a>(
        part: &'a fsl_semantics::check::ConvexPart,
        found: &mut Vec<&'a fsl_semantics::check::ClockConstraint>,
    ) {
        found.extend(part.clocks());
        for family in part.families() {
            for inner in family.parts() {
                part_constraints(inner, found);
            }
        }
    }

    let mut found = Vec::new();
    for guard in timed.model().guards() {
        for part in guard.parts() {
            part_constraints(part, &mut found);
        }
    }
    for invariant in timed.model().invariants() {
        found.extend(invariant.bounds());
    }
    found
}

/// Recompute the canonical lattice and region refinement from the checked
/// model. A document cannot select a coarser grid that merges strict-boundary
/// states.
fn certificate_scale(
    system: &ModelSystem<'_>,
    timed: &fsl_eval::simulation::model::TimedSystem<'_, '_>,
    claim: &CheckedProbabilisticClaim,
    route: TimedProbabilityRoute,
) -> Result<u64, ProbabilityDocumentError> {
    let mut denominators = Vec::new();
    for constraint in clock_constraints(timed) {
        denominators.push(constraint.bound().denominator().as_big().clone());
    }
    for reset in timed.model().resets() {
        for reset in reset.resets() {
            denominators.push(reset.value().denominator().as_big().clone());
        }
    }
    if let fsl_semantics::check::ProbForm::Probability {
        event:
            fsl_semantics::check::Event::Formula {
                timed_horizon: Some(horizon),
                ..
            },
        ..
    } = &claim.form
    {
        denominators.push(horizon.denominator().as_big().clone());
    }
    for delay in system.input().probabilistic.delays() {
        if let fsl_semantics::check::DelayDistribution::Discrete(values) = delay.distribution() {
            denominators.extend(
                values
                    .iter()
                    .map(|(value, _)| value.denominator().as_big().clone()),
            );
        }
    }
    let base = denominators
        .iter()
        .fold(BigInt::one(), |scale, denominator| scale.lcm(denominator))
        .to_u64()
        .filter(|scale| *scale > 0)
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    let subdivisions = if route == TimedProbabilityRoute::Region {
        let has_deadline = matches!(
            &claim.form,
            fsl_semantics::check::ProbForm::Probability {
                event: fsl_semantics::check::Event::Formula {
                    timed_horizon: Some(_),
                    ..
                },
                ..
            }
        );
        u64::try_from(timed.places().len())
            .ok()
            .and_then(|clocks| clocks.checked_add(if has_deadline { 1 } else { 0 }))
            .and_then(|clocks| clocks.checked_add(1))
            .ok_or(ProbabilityDocumentError::Unsupported)?
    } else {
        1
    };
    base.checked_mul(subdivisions)
        .ok_or(ProbabilityDocumentError::Unsupported)
}
