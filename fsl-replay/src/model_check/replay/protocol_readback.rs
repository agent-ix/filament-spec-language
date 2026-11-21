// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resolve canonical protocol step input against genuine native successors.
//! Canonicalization is supplied by the owning admitted-reference converter.

use filament_exact::{Charge, ChargePoint};
use fsl_eval::simulation::model::ModelRestoreBudget;
use fsl_eval::simulation::protocol::ProtocolTransition;
use fsl_eval::simulation::{ExpansionStop, SubjectSystem};
use serde_json::Value;

use super::{expand, state_key_of, ModelTraceRefusal};
use crate::{ModelStep, ReplayRefusal};

/// One original protocol payload step, before native transition readback.
pub(crate) struct NativeStepInput<'a> {
    pub(crate) transition: &'a Value,
    pub(crate) post_state: &'a Value,
    pub(crate) send_record: Option<&'a Value>,
    pub(crate) has_delay: bool,
}

pub(super) struct ClauseInput<'a> {
    pub(super) over: Option<&'a fsl_eval::model_subject::ModelInstance>,
    pub(super) node: Option<&'a fsl_semantics::check::TemporalClauseNode>,
    pub(super) letters: Option<&'a dyn fsl_eval::model_subject::ModelLetters>,
    pub(super) trace: Option<&'a fsl_eval::temporal::TimedTrace>,
}

/// A typed temporal readback failure, never a guessed native transition.
#[derive(Debug, thiserror::Error)]
pub enum TemporalReadbackError {
    /// The actual timed subject stopped delay, guard or invariant evaluation.
    #[error("native timed reconstruction stopped: {0:?}")]
    TimedStop(fsl_eval::simulation::model::TimedStop),
    /// The actual protocol owner refused its resolved fairness set.
    #[error("native protocol fairness reconstruction refused: {0:?}")]
    Fairness(fsl_eval::simulation::protocol::ProtocolFairnessError),
    /// Native letter evaluation refused before it could reconstruct a failure.
    #[error("native failure reconstruction refused: {0:?}")]
    TemporalRefusal(fsl_eval::temporal::TemporalRefusal),
    /// A native evaluation stopped or disagreed with the supplied undefined kind.
    #[error("native failure reconstruction returned {0:?}")]
    TemporalOutcome(Box<fsl_eval::temporal::TemporalOutcome>),
    /// No complete supplied transition and post-state matches a native successor.
    #[error("no canonical native successor matches step {step}")]
    NoMatch {
        /// Original zero-based supplied step position.
        step: usize,
    },
    /// Multiple native successors match the complete supplied transition and state.
    #[error("canonical native successor is ambiguous at step {step}")]
    Ambiguous {
        /// Original zero-based supplied step position.
        step: usize,
    },
    /// An untimed family received timed step data.
    #[error("native temporal step {step} contains an inapplicable delay")]
    Delay {
        /// Original zero-based supplied step position.
        step: usize,
    },
    /// The same consumer meter stopped before the requested native operation.
    #[error("native temporal reconstruction accounting stopped: {incomplete:?}")]
    Accounting {
        /// The original accounting denial, including its counter and charge.
        incomplete: filament_exact::Incomplete,
    },
    /// The admitted native subject stopped successor expansion.
    #[error("native temporal expansion stopped: {stop:?}")]
    Expansion {
        /// The original native expansion stop.
        stop: ExpansionStop,
    },
    /// Existing native replay refused reconstruction or execution.
    #[error(transparent)]
    Replay(Box<ReplayRefusal>),
    /// The FR621-owned transport or subject validation refused the input.
    #[error(transparent)]
    Context(#[from] crate::replay_context::ContextError),
    /// The owning shared reference converter refused reconstruction.
    #[error(transparent)]
    Reference(#[from] crate::replay_context::ReferenceContextError),
}

/// Reconstruct failure metadata from the actual admitted prefix. Wire reason
/// strings are never used to manufacture native loci, values or causes.
pub(crate) fn metadata<M: fsl_eval::simulation::protocol::MemoryModel>(
    system: &fsl_eval::simulation::protocol::ProtocolSystem<'_, M>,
    input: &crate::replay_context::TemporalInput,
    resolved: &ResolvedPrefix<ProtocolTransition, fsl_eval::simulation::protocol::ProtocolState<M>>,
    subject: &crate::ProtocolTraceSubject,
    node: Option<&fsl_semantics::check::TemporalClauseNode>,
    letters: Option<&fsl_eval::simulation::model::ClauseLetters<'_, '_>>,
    budget: &mut dyn ModelRestoreBudget,
) -> Result<crate::replay_context::TemporalMetadata<ProtocolTransition>, TemporalReadbackError> {
    use crate::replay_context::{ContextError, TemporalMetadata};
    let fairness = match node {
        Some(node) => {
            let clause = node.clause();
            let targets = clause
                .protocol_fairness()
                .map(|(target, _)| target.clone())
                .collect::<Vec<_>>();
            system
                .fairness_set_with_targets(clause.fairness(), &targets)
                .map_err(TemporalReadbackError::Fairness)?
        }
        None => Vec::new(),
    };
    let over_binding = (!input.over_binding.is_null()).then(|| subject.over.clone());
    let kind = match input.kind.as_str() {
        Some("formula") => crate::CounterexampleKind::Formula,
        Some("deadlock") => crate::CounterexampleKind::Deadlock,
        Some("time-lock") => crate::CounterexampleKind::TimeLock,
        Some("race") => {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct RaceInput {
                location: Value,
                earlier: String,
                later: String,
            }
            let fields: RaceInput =
                serde_json::from_value(input.race.clone().ok_or_else(|| {
                    ContextError::invalid("/payload/race", "missing native race claim")
                })?)
                .map_err(ContextError::from)?;
            let actual = resolved
                .prefix
                .last()
                .and_then(|step| step.transition.race())
                .ok_or_else(|| {
                    ContextError::invalid(
                        "/payload/race",
                        "actual final transition has no data race",
                    )
                })?;
            let _location_claim = fields.location;
            crate::CounterexampleKind::Race(crate::Race {
                location: actual.location.clone(),
                earlier: crate::replay_context::unsigned(&fields.earlier, "/payload/race/earlier")?,
                later: crate::replay_context::unsigned(&fields.later, "/payload/race/later")?,
            })
        }
        Some("loop-proof") => {
            let proof = loop_proof(input)?;
            crate::CounterexampleKind::LoopProof(proof)
        }
        None if input.kind.get("type").and_then(Value::as_str) == Some("undefined-evaluation") => {
            crate::CounterexampleKind::UndefinedEvaluation(undefined_metadata(
                system,
                input,
                resolved,
                ClauseInput {
                    over: over_binding.as_ref(),
                    node,
                    letters: letters
                        .map(|letters| letters as &dyn fsl_eval::model_subject::ModelLetters),
                    trace: None,
                },
                budget,
                |_| Ok(None),
            )?)
        }
        _ => {
            return Err(
                ContextError::invalid("/payload/kind", "unknown native temporal kind").into(),
            )
        }
    };
    let blocked = if matches!(kind, crate::CounterexampleKind::Deadlock) {
        let prefix = resolved
            .prefix
            .iter()
            .map(|step| &step.transition)
            .collect::<Vec<_>>();
        system
            .blocked(&resolved.final_state, &prefix)
            .map_err(|stop| TemporalReadbackError::Expansion { stop })?
    } else {
        Vec::new()
    };
    Ok(TemporalMetadata {
        over_binding,
        fairness,
        kind,
        blocked,
        refinement: None,
    })
}

pub(crate) fn loop_proof(
    input: &crate::replay_context::TemporalInput,
) -> Result<crate::LoopProofCounterexample, TemporalReadbackError> {
    use crate::replay_context::ContextError;
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Proof {
        repeat: usize,
        obligation: String,
        failure: String,
    }
    let proof: Proof = serde_json::from_value(input.loop_proof.clone().ok_or_else(|| {
        ContextError::invalid("/payload/loop_proof", "missing native loop-proof claim")
    })?)
    .map_err(ContextError::from)?;
    let obligation = match proof.obligation.as_str() {
        "loop-invariant" => crate::LoopProofObligation::Invariant,
        "loop-termination" => crate::LoopProofObligation::Termination,
        _ => {
            return Err(ContextError::invalid(
                "/payload/loop_proof/obligation",
                "unknown native obligation",
            )
            .into())
        }
    };
    let failure = match proof.failure.as_str() {
        "invariant-false" => crate::LoopProofFailure::InvariantFalse,
        "undefined" => crate::LoopProofFailure::Undefined,
        "variant-did-not-decrease" => crate::LoopProofFailure::VariantDidNotDecrease,
        "variant-below-floor" => crate::LoopProofFailure::VariantBelowFloor,
        _ => {
            return Err(ContextError::invalid(
                "/payload/loop_proof/failure",
                "unknown native failure",
            )
            .into())
        }
    };
    Ok(crate::LoopProofCounterexample {
        repeat: proof.repeat,
        obligation,
        failure,
    })
}

pub(super) fn undefined_metadata<S: SubjectSystem>(
    system: &S,
    input: &crate::replay_context::TemporalInput,
    resolved: &ResolvedPrefix<S::TransitionId, S::State>,
    clause: ClauseInput<'_>,
    budget: &mut dyn ModelRestoreBudget,
    mut constraint: impl FnMut(
        &S::State,
    ) -> Result<
        Option<(crate::UndefinedLocus, fsl_eval::temporal::UndefinedCause)>,
        TemporalReadbackError,
    >,
) -> Result<crate::UndefinedEvaluation, TemporalReadbackError> {
    use crate::replay_context::ContextError;
    let ClauseInput {
        over,
        node,
        letters,
        trace: timed_trace,
    } = clause;
    let mut authored = None;
    let at = |position: usize,
              locus,
              transition,
              cause|
     -> Result<crate::UndefinedEvaluation, TemporalReadbackError> {
        let state = resolved.positions.get(position).ok_or_else(|| {
            ContextError::invalid(
                "/payload/kind/where",
                "native undefined position is absent from the admitted prefix",
            )
        })?;
        Ok(crate::UndefinedEvaluation {
            site: crate::UndefinedSite {
                position: super::position_of(position),
                state_key: state.state_key.clone(),
                locus,
                transition,
                time: None,
            },
            cause,
        })
    };
    if let (Some(node), Some(letters)) = (node, letters) {
        let instance = over.ok_or_else(|| {
            ContextError::invalid(
                "/subject/over",
                "native authored evaluation has no original selected instance",
            )
        })?;
        let looped = !input.looped.is_empty();
        let stutter = input.is_terminal_stutter();
        let mut replayed = super::ReplayedLetters {
            system,
            letters,
            instance,
            positions: &resolved.positions,
            prefix: &resolved.prefix,
            loop_edge_counts: !looped
                || stutter
                || resolved
                    .prefix
                    .last()
                    .is_none_or(|step| system.counts_step(&step.transition)),
            undefined: None,
        };
        let trace = input.temporal_trace(resolved.positions.len())?;
        let open = !looped
            && node.clause().profile().is_bounded()
            && !super::ends_terminal(system, &resolved.positions)?;
        let outcome = if let Some(trace) = timed_trace {
            fsl_eval::temporal::evaluate_timed_at(
                node.clause(),
                trace,
                fsl_eval::temporal::TemporalPosition::new(0),
                &mut replayed,
                budget.meter(),
            )
        } else if open {
            fsl_eval::temporal::evaluate_open_prefix(
                node.clause(),
                super::position_of(resolved.positions.len()),
                &mut replayed,
                budget.meter(),
            )
        } else {
            fsl_eval::temporal::evaluate_with_enabledness(
                node.clause(),
                trace,
                &mut replayed,
                budget.meter(),
            )
        }
        .map_err(TemporalReadbackError::TemporalRefusal)?;
        match outcome {
            fsl_eval::temporal::TemporalOutcome::Undefined(undefined) => {
                let (position, locus) = replayed.undefined.take().ok_or_else(|| {
                    ContextError::invalid(
                        "/payload/kind/where",
                        "native undefined evaluation did not retain its locus",
                    )
                })?;
                if position != undefined.at.index() {
                    return Err(ContextError::invalid(
                        "/payload/kind/where",
                        "native first undefined position disagrees with its evaluator",
                    )
                    .into());
                }
                let position = usize::try_from(position).map_err(|_| {
                    ContextError::invalid(
                        "/payload/kind/where",
                        "native undefined position cannot be represented",
                    )
                })?;
                authored = Some((position, locus, undefined.cause));
            }
            outcome @ (fsl_eval::temporal::TemporalOutcome::Incomplete(_)
            | fsl_eval::temporal::TemporalOutcome::WindowLimitReached(_)) => {
                return Err(TemporalReadbackError::TemporalOutcome(Box::new(outcome)));
            }
            _ => {}
        }
    }
    for (position, state) in resolved.positions.iter().enumerate() {
        if let Some((found, locus, cause)) = &authored {
            if *found == position {
                return at(position, locus.clone(), None, cause.clone());
            }
        }
        if let Some((locus, cause)) = constraint(&state.state)? {
            return at(position, locus, None, cause);
        }
        let (expansion, cut) = super::expand_bounded(system, &state.state)?
            .map_err(|stop| TemporalReadbackError::Expansion { stop })?;
        if let Some((locus, transition, cause)) = super::undefined_finding(&expansion.findings) {
            return at(position, locus, transition, cause);
        }
        if node.is_none() {
            match system
                .terminal_class(&state.state, &expansion, cut)
                .map_err(|stop| TemporalReadbackError::Expansion { stop })?
            {
                fsl_eval::simulation::model::TerminalClass::Undefined { model, cause } => {
                    return at(
                        position,
                        crate::UndefinedLocus::Terminal(model),
                        None,
                        cause,
                    )
                }
                _ => {}
            }
        }
    }
    Err(ContextError::invalid(
        "/payload/kind",
        "actual admitted prefix produces no native undefined failure",
    )
    .into())
}

impl From<ReplayRefusal> for TemporalReadbackError {
    fn from(refusal: ReplayRefusal) -> Self {
        Self::Replay(Box::new(refusal))
    }
}

pub(crate) struct ResolvedPrefix<T, S> {
    pub(crate) prefix: Vec<ModelStep<T>>,
    pub(crate) final_state: S,
    pub(super) positions: Vec<super::Position<S>>,
}

pub(super) fn native_steps<'a>(
    input: &'a crate::replay_context::TemporalInput,
    budget: &mut dyn ModelRestoreBudget,
) -> Result<Vec<NativeStepInput<'a>>, TemporalReadbackError> {
    let looped = if input.is_terminal_stutter() {
        0
    } else {
        input.looped.len()
    };
    let count = input.prefix.len().checked_add(looped).ok_or_else(|| {
        crate::replay_context::ContextError::invalid("/payload", "native step count overflow")
    })?;
    let denied = || {
        crate::replay_context::ReferenceContextError::Codec(
            crate::reference_codec::ReferenceCodecError::Key {
                source: fsl_eval::simulation::model::ModelKeyError::Denied,
                pointer: "/payload".to_owned(),
            },
        )
    };
    budget
        .reserve(
            count
                .checked_mul(std::mem::size_of::<NativeStepInput<'_>>())
                .ok_or_else(denied)?,
        )
        .map_err(|_| denied())?;
    budget
        .work(u64::try_from(count).map_err(|_| denied())?)
        .map_err(|_| denied())?;
    let mut steps = Vec::new();
    steps
        .try_reserve(count)
        .map_err(|_| crate::replay_context::ContextError::Allocation { requested: count })?;
    steps.extend(
        input
            .prefix
            .iter()
            .chain(input.looped.iter().take(looped))
            .map(|step| NativeStepInput {
                transition: &step.transition,
                post_state: &step.post_state,
                send_record: step.send_record.as_ref(),
                has_delay: step.has_delay(),
            }),
    );
    Ok(steps)
}

/// Read complete canonical supplied transitions uniquely against actual
/// admitted successors. Matching charges native graph work before encoding
/// each candidate; a limit, unknown identity or ambiguous wire never wins.
pub(crate) fn resolve<S: SubjectSystem>(
    system: &S,
    initial_index: usize,
    supplied: &[NativeStepInput<'_>],
    budget: &mut dyn ModelRestoreBudget,
    mut canonical_transition: impl FnMut(
        &S::TransitionId,
        &S::State,
        &S::State,
        &mut dyn ModelRestoreBudget,
    ) -> Result<
        (Value, Option<Value>),
        crate::replay_context::ReferenceContextError,
    >,
) -> Result<ResolvedPrefix<S::TransitionId, S::State>, TemporalReadbackError> {
    let initial = system.initial();
    let count = initial.len();
    let mut current = initial.into_iter().nth(initial_index).ok_or_else(|| {
        super::refused(ModelTraceRefusal::InitialState {
            index: initial_index,
            count,
        })
    })?;
    let mut prefix = Vec::new();
    let mut positions = Vec::new();
    positions
        .try_reserve(supplied.len().saturating_add(1))
        .map_err(|_| crate::replay_context::ContextError::Allocation {
            requested: supplied.len().saturating_add(1),
        })?;
    positions.push(super::Position {
        state_key: state_key_of(system, &current).map_err(ReplayRefusal::from)?,
        state: current.clone(),
    });
    prefix.try_reserve(supplied.len()).map_err(|_| {
        crate::replay_context::ContextError::Allocation {
            requested: supplied.len(),
        }
    })?;
    for (step, input) in supplied.iter().enumerate() {
        if input.has_delay {
            return Err(TemporalReadbackError::Delay { step });
        }
        let supplied_state = crate::replay_context::StateKeyInput::read(
            input.post_state,
            &format!("/payload/steps/{step}/post_state"),
            budget,
        )?;
        let expansion =
            expand(system, &current)?.map_err(|stop| TemporalReadbackError::Expansion { stop })?;
        let mut matched = None;
        for (transition, next) in expansion.successors {
            budget
                .meter()
                .charge(Charge::new(ChargePoint::GraphEdge))
                .map_err(|incomplete| TemporalReadbackError::Accounting { incomplete })?;
            let (identity, send_record) =
                canonical_transition(&transition, &current, &next, budget)?;
            if &identity != input.transition || send_record.as_ref() != input.send_record {
                continue;
            }
            let state_key = state_key_of(system, &next).map_err(ReplayRefusal::from)?;
            if !supplied_state.matches(&state_key, budget)? {
                continue;
            }
            if matched.is_some() {
                return Err(TemporalReadbackError::Ambiguous { step });
            }
            matched = Some((transition, next, state_key));
        }
        let (transition, next, post_state) =
            matched.ok_or(TemporalReadbackError::NoMatch { step })?;
        let retained_key = post_state.clone();
        prefix.push(ModelStep {
            transition,
            post_state,
            delay: None,
        });
        positions.push(super::Position {
            state_key: retained_key,
            state: next.clone(),
        });
        current = next;
    }
    Ok(ResolvedPrefix {
        prefix,
        final_state: current,
        positions,
    })
}

/// Restore protocol steps only through actual native successors and validate
/// every supplied family member against the complete owning-family writer.
pub(crate) fn admit<M: fsl_eval::simulation::protocol::MemoryModel>(
    system: &fsl_eval::simulation::protocol::ProtocolSystem<'_, M>,
    supplied: &Value,
    subject: &crate::ProtocolWire,
    claim: &fsl_foundation::source::provenance::OccurrenceKey,
    codec: &crate::reference_codec::ReferenceCodec<'_, '_, '_>,
    budget: &mut dyn ModelRestoreBudget,
    cancel: &filament_exact::Cancel,
    restore_metadata: impl FnOnce(
        &crate::replay_context::TemporalInput,
        &ResolvedPrefix<ProtocolTransition, fsl_eval::simulation::protocol::ProtocolState<M>>,
        &mut dyn ModelRestoreBudget,
    ) -> Result<
        crate::replay_context::TemporalMetadata<ProtocolTransition>,
        TemporalReadbackError,
    >,
) -> Result<crate::TemporalCounterexample<ProtocolTransition>, TemporalReadbackError> {
    use crate::replay_context::{ContextError, ReferenceContextError, TemporalInput};
    let input = TemporalInput::read(supplied)?;
    let steps = native_steps(&input, budget)?;
    let resolved = resolve(
        system,
        input.initial_state,
        &steps,
        budget,
        |transition, _, _, budget| {
            let mut scope = codec.scope(
                budget,
                cancel,
                usize::MAX,
                quire_canonical::Limits::new(u64::MAX),
            );
            let identity = transition.write_borrowed(&mut scope)?;
            let send = transition
                .send_record()
                .map(|record| record.write_borrowed(&mut scope))
                .transpose()?;
            Ok((identity, send))
        },
    )?;
    let metadata = restore_metadata(&input, &resolved, budget)?;
    let terminal = system
        .canonical_state_key(&resolved.final_state)
        .map_err(ReferenceContextError::from)?;
    let payload = input.reconstruct(resolved.prefix, &terminal, metadata, budget)?;
    let written = codec
        .scope(
            budget,
            cancel,
            usize::MAX,
            quire_canonical::Limits::new(u64::MAX),
        )
        .protocol_temporal_wire(&payload, subject, Some(claim), Some(&terminal))
        .map_err(ReferenceContextError::from)?;
    if &written != supplied {
        return Err(ContextError::invalid(
            "/payload",
            "native protocol reconstruction differs from the complete supplied family wire",
        )
        .into());
    }
    Ok(payload)
}
