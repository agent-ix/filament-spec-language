// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resolve timed input against actual delayed, guarded and reset successors.

use filament_exact::{Cancel, Charge, ChargePoint, Outcome, Rational, RationalArithmetic};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::ModelRestoreBudget;

use super::{DelayReadError, TimedPosition, TimedState, TimedSystem};
use crate::replay_context::{
    ContextError, ReferenceContextError, StateKeyInput, TemporalInput, TemporalReadbackError,
};

struct TimedPrefix {
    positions: Vec<TimedPosition>,
    steps: Vec<crate::ModelStep<ModelTransition>>,
    initial_model_index: usize,
}

pub(in crate::model_check::replay) fn admit(
    readback: &super::super::model_readback::ModelReadback<'_, '_>,
    reading: &crate::TimedReplay,
    codec: &crate::reference_codec::ReferenceCodec<'_, '_, '_>,
    budget: &mut dyn ModelRestoreBudget,
    cancel: &Cancel,
) -> Result<crate::TemporalCounterexample, TemporalReadbackError> {
    let result = reconstruct(
        readback,
        reading,
        codec,
        budget,
        cancel,
        |transition, supplied, budget| {
            let written = codec
                .scope(
                    budget,
                    cancel,
                    usize::MAX,
                    quire_canonical::Limits::new(u64::MAX),
                )
                .encode_task_transition(transition)?;
            Ok(&written == supplied)
        },
    )?;
    let written = codec
        .scope(
            budget,
            cancel,
            usize::MAX,
            quire_canonical::Limits::new(u64::MAX),
        )
        .temporal_wire(
            &result.payload,
            result.over.as_ref(),
            Some(readback.claim),
            Some(&result.terminal),
        )
        .map_err(ReferenceContextError::from)?;
    if &written != readback.supplied {
        return Err(ContextError::invalid(
            "/payload",
            "native timed reconstruction differs from the complete supplied family wire",
        )
        .into());
    }
    Ok(result.payload)
}

pub(super) struct TimedPayload {
    pub(super) payload: crate::TemporalCounterexample,
    pub(super) over: Option<filament_exact::ObjectReference>,
    pub(super) terminal: fsl_eval::simulation::StateKey,
}

fn arithmetic(
    operation: RationalArithmetic<'_>,
    budget: &mut dyn ModelRestoreBudget,
) -> Result<Rational, TemporalReadbackError> {
    match filament_exact::evaluate_rational_arithmetic(operation, None, budget.meter()) {
        Outcome::Completed(value) => Ok(value),
        Outcome::Incomplete(incomplete) => Err(TemporalReadbackError::Accounting { incomplete }),
        Outcome::Undefined(_) | Outcome::Refused(_) => Err(ContextError::invalid(
            "/payload",
            "native timed arithmetic did not yield a rational",
        )
        .into()),
    }
}

/// Reconstruct native timed data without writing a family wire value. The
/// caller must validate its complete supplied payload with the owning scoped
/// writer before admitting the witness packet or invoking native replay.
pub(super) fn reconstruct(
    readback: &super::super::model_readback::ModelReadback<'_, '_>,
    reading: &crate::TimedReplay,
    codec: &crate::reference_codec::ReferenceCodec<'_, '_, '_>,
    budget: &mut dyn ModelRestoreBudget,
    cancel: &Cancel,
    matches_transition: impl FnMut(
        &ModelTransition,
        &serde_json::Value,
        &mut dyn ModelRestoreBudget,
    ) -> Result<bool, ReferenceContextError>,
) -> Result<TimedPayload, TemporalReadbackError> {
    use super::super::model_readback::ModelMetadata;
    use super::super::protocol_readback::ResolvedPrefix;
    let timed = super::timed_subject(readback.system, &reading.model, reading.max_clocks)?;
    let input = TemporalInput::read(readback.supplied)?;
    let TimedPrefix {
        positions,
        steps,
        initial_model_index,
    } = resolve(&timed, &input, budget, cancel, matches_transition)?;
    let over = readback.restore_over(&input, initial_model_index, codec, budget, cancel)?;
    let package = readback
        .system
        .source_package()
        .ok_or(ReferenceContextError::SourceOwner)?;
    let node = package.graph().temporal_clauses().iter().find(|node| {
        node.identity() == *readback.claim.node() && node.claim() == readback.claim.origin()
    });
    let letters = node
        .map(|node| readback.system.clause_letters(node))
        .transpose()
        .map_err(|_| {
            ContextError::invalid("/subject", "actual timed clause letters could not be built")
        })?;
    let last = positions.last().ok_or_else(|| {
        ContextError::invalid("/payload", "native timed prefix has no final state")
    })?;
    let terminal = last.state_key.clone();
    let mut discrete = Vec::new();
    discrete
        .try_reserve(positions.len())
        .map_err(|_| ContextError::Allocation {
            requested: positions.len(),
        })?;
    for position in &positions {
        let state = position.state.discrete().clone();
        let state_key =
            super::state_key_of(readback.system, &state).map_err(crate::ReplayRefusal::from)?;
        discrete.push(super::Position { state, state_key });
    }
    let resolved = ResolvedPrefix {
        prefix: steps,
        final_state: last.state.discrete().clone(),
        positions: discrete,
    };
    let stamps = positions
        .iter()
        .map(|position| position.stamp.clone())
        .collect::<Vec<_>>();
    let trace = if input.looped.is_empty() {
        fsl_eval::temporal::TimedTrace::new(stamps)
    } else if input.is_terminal_stutter() {
        fsl_eval::temporal::TimedTrace::lasso(
            stamps,
            super::position_of(positions.len().saturating_sub(1)),
            super::stutter_delay(&timed),
        )
    } else {
        let entry = positions
            .get(input.prefix.len())
            .ok_or_else(|| ContextError::invalid("/payload/loop", "native loop entry is absent"))?;
        let duration = arithmetic(
            RationalArithmetic::Subtract(&last.stamp, &entry.stamp),
            budget,
        )?;
        fsl_eval::temporal::TimedTrace::lasso(
            stamps
                .into_iter()
                .take(positions.len().saturating_sub(1))
                .collect(),
            super::position_of(input.prefix.len()),
            duration,
        )
    }
    .map_err(TemporalReadbackError::TemporalRefusal)?;
    let timed_profile = node.is_some_and(|node| {
        node.clause().profile() == fsl_semantics::check::TemporalProfile::Timed
    });
    let mut metadata = readback.metadata(
        &input,
        &resolved,
        ModelMetadata {
            over,
            node,
            letters: letters
                .as_ref()
                .map(|letters| letters as &dyn fsl_eval::model_subject::ModelLetters),
            trace: timed_profile.then_some(&trace),
        },
        budget,
    )?;
    if let crate::CounterexampleKind::UndefinedEvaluation(failure) = &mut metadata.kind {
        let at = usize::try_from(failure.site.position)
            .ok()
            .and_then(|index| positions.get(index))
            .ok_or_else(|| {
                ContextError::invalid("/payload/kind/where", "native undefined position is absent")
            })?;
        let clocks = timed
            .places()
            .iter()
            .zip(at.state.clocks().values())
            .filter(|(place, _)| place.present(at.state.discrete()))
            .map(|(_, value)| value.clone())
            .collect();
        failure.site.time = Some(Box::new(crate::TimedPoint {
            stamp: at.stamp.clone(),
            clocks,
        }));
    }
    let over = metadata.over_binding.as_ref().and_then(|over| {
        letters
            .as_ref()
            .and_then(|letters| letters.instance_reference(over))
    });
    let payload = input.reconstruct(resolved.prefix, &terminal, metadata, budget)?;
    Ok(TimedPayload {
        payload,
        over,
        terminal,
    })
}

fn resolve(
    timed: &TimedSystem<'_, '_>,
    input: &TemporalInput,
    budget: &mut dyn ModelRestoreBudget,
    cancel: &Cancel,
    mut matches_transition: impl FnMut(
        &ModelTransition,
        &serde_json::Value,
        &mut dyn ModelRestoreBudget,
    ) -> Result<bool, ReferenceContextError>,
) -> Result<TimedPrefix, TemporalReadbackError> {
    let initial = timed
        .initial_with_model_indices(budget, cancel)
        .map_err(|source| {
            ReferenceContextError::Codec(crate::reference_codec::ReferenceCodecError::Key {
                source,
                pointer: "/payload/initial_state".to_owned(),
            })
        })?;
    let count = initial.len();
    let (initial_model_index, state) =
        initial
            .into_iter()
            .nth(input.initial_state)
            .ok_or_else(|| {
                super::refused(super::ModelTraceRefusal::InitialState {
                    index: input.initial_state,
                    count,
                })
            })?;
    let looped = if input.is_terminal_stutter() {
        0
    } else {
        input.looped.len()
    };
    let count = input
        .prefix
        .len()
        .checked_add(looped)
        .ok_or_else(|| ContextError::invalid("/payload", "native timed step count overflow"))?;
    let denied = || {
        ReferenceContextError::Codec(crate::reference_codec::ReferenceCodecError::Key {
            source: fsl_eval::simulation::model::ModelKeyError::Denied,
            pointer: "/payload".to_owned(),
        })
    };
    let entries = count.checked_add(1).ok_or_else(denied)?;
    budget
        .reserve(
            entries
                .checked_mul(std::mem::size_of::<TimedPosition>())
                .ok_or_else(denied)?,
        )
        .map_err(|_| denied())?;
    budget
        .reserve(
            count
                .checked_mul(std::mem::size_of::<crate::ModelStep>())
                .ok_or_else(denied)?,
        )
        .map_err(|_| denied())?;
    let mut positions = Vec::new();
    positions
        .try_reserve(entries)
        .map_err(|_| ContextError::Allocation { requested: entries })?;
    let mut steps = Vec::new();
    steps
        .try_reserve(count)
        .map_err(|_| ContextError::Allocation { requested: count })?;
    positions.push(TimedPosition {
        state_key: timed
            .state_key(&state)
            .map_err(ReferenceContextError::from)?,
        state,
        stamp: super::zero(),
        step: None,
    });
    for (index, supplied) in input
        .prefix
        .iter()
        .chain(input.looped.iter().take(looped))
        .enumerate()
    {
        cancel.poll();
        if let Some(cause) = cancel.tripped() {
            return Err(ReferenceContextError::Codec(
                crate::reference_codec::ReferenceCodecError::Key {
                    source: fsl_eval::simulation::model::ModelKeyError::Cancelled(cause),
                    pointer: "/payload".to_owned(),
                },
            )
            .into());
        }
        let current = positions.last().ok_or_else(|| {
            ContextError::invalid("/payload", "native timed prefix has no current state")
        })?;
        let delay = supplied
            .delay(&format!("/payload/steps/{index}/delay"), budget)?
            .ok_or_else(|| {
                super::refused(super::ModelTraceRefusal::MissingDelay { step: Some(index) })
            })?;
        let delayed =
            super::read_delay(timed, &current.state, &delay, Some(index)).map_err(|cause| {
                match cause {
                    DelayReadError::Replay(refusal) => TemporalReadbackError::from(refusal),
                    DelayReadError::Stop(stop) => TemporalReadbackError::TimedStop(stop),
                }
            })?;
        let supplied_state = StateKeyInput::read(
            &supplied.post_state,
            &format!("/payload/steps/{index}/post_state"),
            budget,
        )?;
        let candidates = timed
            .guarded_steps(delayed.discrete())
            .map_err(TemporalReadbackError::TimedStop)?;
        let mut matched = None;
        for candidate in candidates {
            budget
                .meter()
                .charge(Charge::new(ChargePoint::GraphEdge))
                .map_err(|incomplete| TemporalReadbackError::Accounting { incomplete })?;
            if supplied.send_record.is_some()
                || !matches_transition(candidate.transition(), &supplied.transition, budget)?
            {
                continue;
            }
            if !candidate.parts().iter().any(|part| {
                part.iter()
                    .all(|condition| condition.holds(delayed.clocks()))
            }) {
                continue;
            }
            let next = TimedState::new(candidate.post().clone(), candidate.reset(delayed.clocks()));
            let key = timed
                .state_key(&next)
                .map_err(ReferenceContextError::from)?;
            if !supplied_state.matches(&key, budget)? {
                continue;
            }
            if matched.is_some() {
                return Err(TemporalReadbackError::Ambiguous { step: index });
            }
            matched = Some((candidate, next, key));
        }
        let (candidate, next, key) =
            matched.ok_or(TemporalReadbackError::NoMatch { step: index })?;
        if let Some(invariant) = timed
            .broken_invariant(&next)
            .map_err(TemporalReadbackError::TimedStop)?
        {
            return Err(super::refused(super::ModelTraceRefusal::TargetInvariant {
                step: index,
                invariant,
            })
            .into());
        }
        let elapsed = arithmetic(RationalArithmetic::Add(&current.stamp, &delay), budget)?;
        let stamp = arithmetic(
            RationalArithmetic::Add(&elapsed, &timed.tick_of(candidate.operation())),
            budget,
        )?;
        budget.reserve(key.as_bytes().len()).map_err(|_| denied())?;
        budget
            .work(u64::try_from(key.as_bytes().len()).map_err(|_| denied())?)
            .map_err(|_| denied())?;
        steps.push(crate::ModelStep {
            transition: candidate.transition().clone(),
            post_state: key.clone(),
            delay: Some(delay),
        });
        positions.push(TimedPosition {
            state: next,
            stamp,
            state_key: key,
            step: Some(candidate),
        });
    }
    Ok(TimedPrefix {
        positions,
        steps,
        initial_model_index,
    })
}
