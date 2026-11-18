// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330 (FR-327, FR-328, FR-329): a temporal clause evaluated over
//! observed snapshots, the one clause evaluator (FR-107) filling the
//! [`TemporalLetters`] seam.
//!
//! Each position of the trace is the object environment its snapshot
//! admitted. A `holds` atom reads its condition through
//! [`CheckedPackageEvaluation::evaluate_holds`] with the clause's `over`
//! object bound, and an `on each` guard through
//! [`CheckedPackageEvaluation::evaluate_guard`] with both binders bound to
//! it; a lasso position past the represented ones reads the environment its
//! wrapped index names.

use filament_eval::{FamilyOutcome, FamilyResult};
use filament_exact::{Meter, ObjectReference, Outcome, Value};
use filament_semantic_value::catalog::InternalFault;
use fsl_package::CheckedPackage;
use fsl_semantics::check::{FormulaNodeId, TemporalClauseNode};
use fsl_semantics::model::object_environment::ObjectEnvironment;

use super::engine::DecisiveSearch;
use super::finite_window::{TimestampedTrace, Valuation, WindowEvaluation, evaluate_finite_window};
use super::tape::{TapeEntry, TapeLetter, TapeReading};
use super::{
    AtomReading, CaptureReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalRefusal, TemporalTrace, UndefinedCause,
};
use crate::value::{CheckedPackageEvaluation, evaluate_capture};
use filament_eval::{CallFailure, Evaluation};

/// Why a `holds` condition stopped the run without a reading, kept whole so
/// the caller reports it as FR-109 reports the same S6a result of a state
/// clause (a refusal with its record; a failed call by its own mapping).
#[derive(Debug)]
pub enum HoldsStop {
    /// The condition's S6a result was a refusal at `at`, the position of
    /// the condition (FR-327 "Refused condition").
    Refused {
        /// The position the refused condition was read at.
        at: TemporalPosition,
        /// The refused S6a evaluation, whole.
        evaluation: Evaluation,
    },
    /// The call failed before or during S6a.
    Failed(CallFailure),
}

/// What [`evaluate_observed`] returns: the evaluator's result and, when a
/// condition stopped the run, why (the result is then the stop's fault).
#[derive(Debug)]
pub struct ObservedRun {
    /// The evaluator's result.
    pub result: Result<TemporalOutcome, TemporalRefusal>,
    /// The condition stop that ended the run, if one did.
    pub stopped: Option<HoldsStop>,
    /// The first undefined reading, with its locus, trace position and cause.
    pub undefined: Option<FirstUndefined>,
    /// The violation's decisive atom, found by [`evaluate_observed_decisive`]
    /// on the run's own evaluator and meter; `None` for every other run.
    pub decisive: Option<DecisiveSearch>,
}

/// The expression an observed run first read as undefined: a `holds` atom,
/// the `on each` activation guard or a capture initializer (FR-337).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservedLocus {
    /// A `holds` atom, by its formula node.
    Atom(FormulaNodeId),
    /// The `when` condition of the clause's guarded `on each` activation.
    Activation,
    /// A capture's initializer, by the capture's name.
    Capture(String),
}

/// The first undefined reading of an observed run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirstUndefined {
    /// The expression read as undefined.
    pub locus: ObservedLocus,
    /// The trace position it was read at.
    pub position: TemporalPosition,
    /// Why it had no value.
    pub cause: UndefinedCause,
}

/// Evaluate `clause`, a temporal clause of `package`, over `environments`,
/// the object environment of each represented position, prefix first, then
/// loop. `trace` is their shape and `over` the object the clause's `over`
/// parameter is bound to.
///
/// A trace whose shape and environments disagree is a fault, not a
/// refusal: the caller builds both from one trace.
pub fn evaluate_observed(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    environments: &[ObjectEnvironment],
    trace: TemporalTrace,
    over: &ObjectReference,
    meter: &mut Meter,
) -> ObservedRun {
    evaluate_observed_goal(
        package,
        clause,
        ObservedPositions::Objects { environments, over },
        trace,
        (super::engine::Goal::Clause, false, None),
        meter,
    )
}

/// [`evaluate_observed`] that also finds the decisive atom of a violation
/// (FR-330 "The decisive atom") on the same evaluator and meter.
/// `silent_from` is the first silent sample of a progressed trace (FR-504):
/// it carries no observation and every `holds` atom there reads false.
pub fn evaluate_observed_decisive(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    environments: &[ObjectEnvironment],
    trace: TemporalTrace,
    over: &ObjectReference,
    silent_from: Option<u64>,
    meter: &mut Meter,
) -> ObservedRun {
    evaluate_observed_goal(
        package,
        clause,
        ObservedPositions::Objects { environments, over },
        trace,
        (super::engine::Goal::Clause, true, silent_from),
        meter,
    )
}

/// Evaluate the temporal formula at one represented position of an
/// observed trace. Unlike [`evaluate_observed`], this does not apply the
/// clause's activation rule: a replay packet names the position whose
/// claimed value is being checked (FR-331).
pub fn evaluate_observed_at(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    environments: &[ObjectEnvironment],
    trace: TemporalTrace,
    over: &ObjectReference,
    position: TemporalPosition,
    meter: &mut Meter,
) -> ObservedRun {
    evaluate_observed_goal(
        package,
        clause,
        ObservedPositions::Objects { environments, over },
        trace,
        (super::engine::Goal::At(position), false, None),
        meter,
    )
}

/// What the represented positions of an observed run hold.
#[derive(Clone, Copy, Debug)]
pub enum ObservedPositions<'a> {
    /// Snapshot positions: the object environment each admitted, and the
    /// object the clause's `over` parameter names.
    Objects {
        /// The environment of each represented position.
        environments: &'a [ObjectEnvironment],
        /// The object bound to `over`.
        over: &'a ObjectReference,
    },
    /// FR-641: record samples of a declared record trace subject, one per
    /// represented position. The sample is the value of the clause's subject
    /// parameter at its position; no object environment is read.
    Samples(&'a [Value]),
    /// FR-820: record samples read once into a letter tape, one entry per
    /// represented position. A letter is read from the tape; a position the
    /// tape holds by value is read as a sample.
    Tape(&'a super::tape::LetterTape),
    /// FR-820: snapshot positions read once into a letter tape. A letter is
    /// read from the tape; a position the tape holds as its object environment
    /// is read there with the `over` parameter bound to `over`. Position `p`
    /// of the run is the tape's entry `from + p`, so a run can read a slice
    /// of the tape.
    ObjectTape {
        /// The tape.
        tape: &'a super::tape::LetterTape,
        /// The object bound to `over`.
        over: &'a ObjectReference,
        /// The tape entry of the run's position 0.
        from: u64,
    },
}

/// [`evaluate_sampled`] over any represented positions: the run FR-330 makes
/// of a clause over record samples, or over a letter tape of them (FR-820).
pub fn evaluate_positions(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    positions: ObservedPositions<'_>,
    trace: TemporalTrace,
    silent_from: Option<u64>,
    meter: &mut Meter,
) -> ObservedRun {
    evaluate_observed_goal(
        package,
        clause,
        positions,
        trace,
        (super::engine::Goal::Clause, false, silent_from),
        meter,
    )
}

/// [`evaluate_sampled_finite_window`] over any represented positions,
/// a letter tape of record samples included (FR-820).
pub fn evaluate_positions_finite_window(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    positions: ObservedPositions<'_>,
    trace: &TimestampedTrace,
    valuations: WindowValuations<'_>,
    meter: &mut Meter,
) -> ObservedWindowRun {
    evaluate_window_over(package, clause, positions, trace, valuations, meter)
}

/// [`evaluate_observed`] over record samples (FR-641): `samples` holds the
/// sample of each represented position, prefix first, then loop. A violation
/// is not searched for a decisive atom.
pub fn evaluate_sampled(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    samples: &[Value],
    trace: TemporalTrace,
    silent_from: Option<u64>,
    meter: &mut Meter,
) -> ObservedRun {
    evaluate_observed_goal(
        package,
        clause,
        ObservedPositions::Samples(samples),
        trace,
        (super::engine::Goal::Clause, false, silent_from),
        meter,
    )
}

fn evaluate_observed_goal(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    positions: ObservedPositions<'_>,
    trace: TemporalTrace,
    (goal, seek, silent_from): (super::engine::Goal, bool, Option<u64>),
    meter: &mut Meter,
) -> ObservedRun {
    let mut letters = ObservedLetters {
        package,
        clause,
        positions,
        empty: ObjectEnvironment::default(),
        trace,
        stopped: None,
        undefined: None,
        absent: &[],
        classes: &[],
        silent_from,
    };
    let (result, decisive) = super::engine::run_seeking(
        super::engine::Subject::of_clause(clause.clause()),
        trace,
        goal,
        seek,
        &mut letters,
        meter,
    );
    // A refusal the result does not stop at was passed over: an activation
    // before it evaluated false, or an atom before it was undefined, and
    // that result is the run's (FR-327 "Refused condition").
    let stopped = match (letters.stopped, &result) {
        (Some(HoldsStop::Refused { .. }), Ok(_)) => None,
        (stopped, _) => stopped,
    };
    ObservedRun {
        result,
        stopped,
        undefined: letters.undefined,
        decisive,
    }
}

/// What a finite-window run reads of each atom beyond the observations.
#[derive(Clone, Copy, Debug)]
pub struct WindowValuations<'a> {
    /// The `(position, atom node)` valuations absent from the admitted
    /// observations.
    pub absent: &'a [(u64, u64)],
    /// Each formula node's atom class: a node index shared by the nodes that
    /// are one atom (the same `holds` condition), so they read one
    /// valuation; empty makes every node its own atom.
    pub classes: &'a [usize],
    /// The first position that is a silent sample (FR-504): it carries no
    /// observation and every `holds` atom there reads false. `None` when
    /// every position is an observation.
    pub silent_from: Option<u64>,
}

/// What [`evaluate_observed_finite_window`] returns: the evaluator's result
/// and, when a condition stopped the run, why (the result is then the stop's
/// fault).
#[derive(Debug)]
pub struct ObservedWindowRun {
    /// The evaluator's result.
    pub result: Result<WindowEvaluation, TemporalRefusal>,
    /// The condition stop that ended the run, if one did.
    pub stopped: Option<HoldsStop>,
    /// The first undefined reading, with its locus, trace position and cause.
    pub undefined: Option<FirstUndefined>,
}

/// Evaluate `clause` under the finite-window profile (FR-501) over the object
/// environment of each position of `trace` in canonical order, reading the
/// possibility sets of its atoms, and return the full result report: verdict,
/// basis, decision support, premises and result identity. `valuations` names
/// the `(position, atom node)` valuations absent from the admitted
/// observations; two observations of one stamp that read differently are a
/// conflict the evaluator finds itself.
pub fn evaluate_observed_finite_window(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    environments: &[ObjectEnvironment],
    trace: &TimestampedTrace,
    over: &ObjectReference,
    valuations: WindowValuations<'_>,
    meter: &mut Meter,
) -> ObservedWindowRun {
    evaluate_window_over(
        package,
        clause,
        ObservedPositions::Objects { environments, over },
        trace,
        valuations,
        meter,
    )
}

/// [`evaluate_observed_finite_window`] over record samples (FR-641):
/// `samples` holds the sample of each position of `trace` in canonical order.
pub fn evaluate_sampled_finite_window(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    samples: &[Value],
    trace: &TimestampedTrace,
    valuations: WindowValuations<'_>,
    meter: &mut Meter,
) -> ObservedWindowRun {
    evaluate_window_over(
        package,
        clause,
        ObservedPositions::Samples(samples),
        trace,
        valuations,
        meter,
    )
}

fn evaluate_window_over(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    positions: ObservedPositions<'_>,
    trace: &TimestampedTrace,
    valuations: WindowValuations<'_>,
    meter: &mut Meter,
) -> ObservedWindowRun {
    let mut letters = ObservedLetters {
        package,
        clause,
        positions,
        empty: ObjectEnvironment::default(),
        trace: TemporalTrace::Finite {
            len: u64::try_from(trace.len()).unwrap_or(u64::MAX),
        },
        stopped: None,
        undefined: None,
        absent: valuations.absent,
        classes: valuations.classes,
        silent_from: valuations.silent_from,
    };
    let facts = match positions {
        ObservedPositions::Tape(_) => {
            crate::temporal::facts::ClauseFacts::of_checked_letters(clause.clause())
        }
        ObservedPositions::Objects { .. }
        | ObservedPositions::ObjectTape { .. }
        | ObservedPositions::Samples(_) => {
            crate::temporal::facts::ClauseFacts::of_checked(clause.clause())
        }
    };
    let result = evaluate_finite_window(&facts, trace, &mut letters, meter);
    // As `evaluate_observed`: a refusal the result passed over is dropped.
    let stopped = match (letters.stopped, &result) {
        (Some(HoldsStop::Refused { .. }), Ok(_)) => None,
        (stopped, _) => stopped,
    };
    ObservedWindowRun {
        result,
        stopped,
        undefined: letters.undefined,
    }
}

/// FR-820: [`evaluate_positions_finite_window`] carried across a trace that
/// grows. The state reads only the positions an open activation can still
/// read ([`WindowState`]), so each step's work is bounded by them, and its
/// report equals the batch report over the prefix.
#[derive(Clone, Debug, Default)]
pub struct CarriedWindow {
    state: Option<super::finite_window::WindowState>,
    /// The first undefined reading of each earlier evaluation: a summarized
    /// activation is not read again, so the reading an `Undefined` verdict
    /// names may come from an earlier step.
    undefined: Option<FirstUndefined>,
}

impl CarriedWindow {
    /// The number of positions the carried evaluation holds (identities and
    /// stamps, no value).
    pub fn held(&self) -> usize {
        self.state
            .as_ref()
            .map_or(0, super::finite_window::WindowState::retained)
    }

    /// [`evaluate_positions_finite_window`] over `trace`, the batch-admitted
    /// trace of the prefix, reading only what the carried state has not
    /// summarized. The observations of `trace` past those admitted earlier
    /// are admitted; its silent samples are read as FR-504 reads them. A run
    /// a condition stopped leaves the state as it was, so the next step reads
    /// the stop again as the batch run does.
    pub fn evaluate(
        &mut self,
        package: &CheckedPackage,
        clause: &TemporalClauseNode,
        positions: ObservedPositions<'_>,
        trace: &TimestampedTrace,
        valuations: WindowValuations<'_>,
        meter: &mut Meter,
    ) -> ObservedWindowRun {
        let facts = match positions {
            ObservedPositions::Tape(_) => {
                crate::temporal::facts::ClauseFacts::of_checked_letters(clause.clause())
            }
            ObservedPositions::Objects { .. }
            | ObservedPositions::ObjectTape { .. }
            | ObservedPositions::Samples(_) => {
                crate::temporal::facts::ClauseFacts::of_checked(clause.clause())
            }
        };
        let mut state = match self.state.clone() {
            Some(state) => state,
            None => match super::finite_window::WindowState::open(
                &facts,
                trace.clause_identity().to_owned(),
                trace.clock().clone(),
            ) {
                Ok(state) => state,
                Err(refusal) => {
                    return ObservedWindowRun {
                        result: Err(refusal),
                        stopped: None,
                        undefined: None,
                    };
                }
            },
        };
        let observed = trace.observed().min(trace.len());
        for position in trace.positions().get(state.len()..observed).unwrap_or(&[]) {
            state.admit(position.clone());
        }
        let admitted = state.clone();
        let silent = trace.positions().get(observed..).unwrap_or(&[]);
        let mut letters = ObservedLetters {
            package,
            clause,
            positions,
            empty: ObjectEnvironment::default(),
            trace: TemporalTrace::Finite {
                len: u64::try_from(trace.len()).unwrap_or(u64::MAX),
            },
            stopped: None,
            undefined: None,
            absent: valuations.absent,
            classes: valuations.classes,
            silent_from: valuations.silent_from,
        };
        let result = state.evaluate(
            &facts,
            silent,
            *trace.progress(),
            *trace.asserted(),
            &mut letters,
            meter,
        );
        let stopped = match (letters.stopped, &result) {
            (Some(HoldsStop::Refused { .. }), Ok(_)) => None,
            (stopped, _) => stopped,
        };
        let earlier = self.undefined.clone();
        let undefined = match (&result, letters.undefined, earlier) {
            (
                Ok(WindowEvaluation {
                    verdict: super::finite_window::WindowVerdict::Undefined(named),
                    ..
                }),
                fresh,
                earlier,
            ) => [fresh, earlier]
                .into_iter()
                .flatten()
                .find(|first| first.position == named.at),
            (_, fresh, _) => fresh,
        };
        if result.is_err() {
            // The state is unchanged but for the positions admitted, which the
            // next step evaluates with the ones it adds.
            self.state = Some(state);
        } else if stopped.is_some() {
            self.state = Some(admitted);
        } else {
            if let Some(first) = &undefined {
                let earliest = self
                    .undefined
                    .as_ref()
                    .is_none_or(|carried| first.position < carried.position);
                if earliest {
                    self.undefined = Some(first.clone());
                }
            }
            self.state = Some(state);
        }
        ObservedWindowRun {
            result,
            stopped,
            undefined,
        }
    }
}

/// The fault codes of a `holds` condition: a failed call and a non-Boolean
/// completion. A refusal is a refused condition, not a fault.
const HOLDS_CODES: [&str; 2] = ["holds-condition-call-failed", "holds-condition-boolean"];

/// The fault codes of an `on each` guard, as [`HOLDS_CODES`].
const ACTIVATION_CODES: [&str; 2] = [
    "activation-condition-call-failed",
    "activation-condition-boolean",
];

pub(super) struct ObservedLetters<'a> {
    package: &'a CheckedPackage,
    clause: &'a TemporalClauseNode,
    positions: ObservedPositions<'a>,
    /// The environment a sample position reads: it holds no object.
    empty: ObjectEnvironment,
    trace: TemporalTrace,
    stopped: Option<HoldsStop>,
    /// The first undefined reading (the evaluator reports its position and
    /// cause, not its locus).
    undefined: Option<FirstUndefined>,
    /// The `(position, atom node)` valuations the finite-window input names
    /// as absent from the admitted observation (FSpec FR-160).
    absent: &'a [(u64, u64)],
    /// Each node's atom class (empty: every node is its own atom).
    classes: &'a [usize],
    /// The first silent sample, where every `holds` atom reads false.
    silent_from: Option<u64>,
}

impl<'a> ObservedLetters<'a> {
    /// Letters over `positions` of `trace`, reading no silent sample, absent
    /// valuation or atom class.
    pub(super) fn over(
        package: &'a CheckedPackage,
        clause: &'a TemporalClauseNode,
        positions: ObservedPositions<'a>,
        trace: TemporalTrace,
    ) -> Self {
        Self {
            package,
            clause,
            positions,
            empty: ObjectEnvironment::default(),
            trace,
            stopped: None,
            undefined: None,
            absent: &[],
            classes: &[],
            silent_from: None,
        }
    }

    /// The letter the tape read at `position`, when the positions are a tape
    /// that holds a letter there.
    fn taped(&self, position: TemporalPosition) -> Option<&'a TapeLetter> {
        let (tape, from) = match self.positions {
            ObservedPositions::Tape(tape) => (tape, 0),
            ObservedPositions::ObjectTape { tape, from, .. } => (tape, from),
            ObservedPositions::Objects { .. } | ObservedPositions::Samples(_) => return None,
        };
        let index = match self.trace {
            TemporalTrace::Finite { .. } => position.index(),
            TemporalTrace::Lasso { prefix, loop_len } => {
                if position.index() < prefix {
                    position.index()
                } else {
                    (position.index() - prefix)
                        .checked_rem(loop_len)
                        .and_then(|offset| prefix.checked_add(offset))?
                }
            }
        };
        match tape.entry(index.checked_add(from)?)? {
            TapeEntry::Read(letter) => Some(letter),
            TapeEntry::Held(_) | TapeEntry::Object(_) => None,
        }
    }
}

impl ObservedLetters<'_> {
    /// Keep the first undefined reading at the earliest position read, the
    /// position the evaluator reports (ADR-018 UE-1).
    fn note(
        &mut self,
        locus: impl FnOnce() -> ObservedLocus,
        position: TemporalPosition,
        cause: &UndefinedCause,
    ) {
        if self
            .undefined
            .as_ref()
            .is_none_or(|first| position.index() < first.position.index())
        {
            self.undefined = Some(FirstUndefined {
                locus: locus(),
                position,
                cause: cause.clone(),
            });
        }
    }

    /// Whether `position` is a silent sample (FR-504).
    fn is_silent(&self, position: TemporalPosition) -> bool {
        self.silent_from
            .is_some_and(|first| position.index() >= first)
    }

    /// The environment position `position` observes and the value its `over`
    /// parameter takes there: a lasso wraps past its last represented
    /// position into the loop.
    fn subject(
        &self,
        position: TemporalPosition,
    ) -> Result<(&ObjectEnvironment, Value), LetterStop> {
        let index = position.index();
        let slot = match self.trace {
            TemporalTrace::Finite { .. } => Some(index),
            TemporalTrace::Lasso { prefix, loop_len } => {
                if index < prefix {
                    Some(index)
                } else {
                    // `loop_len` is at least 1: the evaluator refuses an empty
                    // loop before it reads a letter.
                    (index - prefix)
                        .checked_rem(loop_len)
                        .and_then(|offset| prefix.checked_add(offset))
                }
            }
        };
        let slot = slot.and_then(|slot| usize::try_from(slot).ok());
        let found = match self.positions {
            ObservedPositions::Objects { environments, over } => slot
                .and_then(|slot| environments.get(slot))
                .map(|environment| (environment, Value::Reference(over.clone()))),
            ObservedPositions::Samples(samples) => slot
                .and_then(|slot| samples.get(slot))
                .map(|sample| (&self.empty, sample.clone())),
            ObservedPositions::Tape(tape) => slot
                .and_then(|slot| tape.entries().get(slot))
                .and_then(|entry| match entry {
                    TapeEntry::Held(sample) => Some((&self.empty, sample.clone())),
                    TapeEntry::Read(_) | TapeEntry::Object(_) => None,
                }),
            ObservedPositions::ObjectTape { tape, over, from } => slot
                .and_then(|slot| u64::try_from(slot).ok()?.checked_add(from))
                .and_then(|slot| tape.entry(slot))
                .and_then(|entry| match entry {
                    TapeEntry::Object(environment) => {
                        Some((&**environment, Value::Reference(over.clone())))
                    }
                    TapeEntry::Read(_) | TapeEntry::Held(_) => None,
                }),
        };
        found.ok_or_else(|| {
            LetterStop::Fault(InternalFault::new(
                "temporal.observed",
                "position-has-an-environment",
            ))
        })
    }

    /// The reading of a condition's S6a `evaluation`; the fault codes name
    /// the condition (`HOLDS_CODES`, `ACTIVATION_CODES`). A refusal or a
    /// failed call is kept whole in `stopped` and ends the run.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn reading(
        stopped: &mut Option<HoldsStop>,
        position: TemporalPosition,
        evaluation: Result<Evaluation, CallFailure>,
        [failed, boolean]: [&'static str; 2],
    ) -> Result<AtomReading, LetterStop> {
        let stop =
            |code: &'static str| LetterStop::Fault(InternalFault::new("temporal.observed", code));
        let evaluation = match evaluation {
            Ok(evaluation) => evaluation,
            Err(failure) => {
                *stopped = Some(HoldsStop::Failed(failure));
                return Err(stop(failed));
            }
        };
        let refused = matches!(
            evaluation.outcome,
            FamilyOutcome::Evaluated(Outcome::Refused(_))
                | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))
        );
        if refused {
            *stopped = Some(HoldsStop::Refused {
                at: position,
                evaluation,
            });
            return Err(LetterStop::Refused);
        }
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true))) => {
                Ok(AtomReading::True)
            }
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false))) => {
                Ok(AtomReading::False)
            }
            FamilyOutcome::Evaluated(Outcome::Completed(_)) => Err(stop(boolean)),
            FamilyOutcome::Evaluated(Outcome::Undefined(undefined)) => {
                Ok(AtomReading::Undefined(UndefinedCause::Kernel(undefined)))
            }
            FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
                Err(LetterStop::Incomplete(incomplete))
            }
            FamilyOutcome::Evaluated(Outcome::Refused(_))
            | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_)) => Err(LetterStop::Refused),
            FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => Ok(
                AtomReading::Undefined(UndefinedCause::Family(cause.undefined_record())),
            ),
            #[cfg(seam_probe_eval_downstream)]
            FamilyOutcome::__SeamProbe => unreachable!("never constructed outside the probe build"),
        }
    }
}

impl TemporalLetters for ObservedLetters<'_> {
    #[deny(clippy::wildcard_enum_match_arm)]
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        if self.is_silent(position) {
            return Ok(AtomReading::False);
        }
        if let Some(letter) = self.taped(position) {
            return letter
                .atom(atom.index())
                .map(TapeReading::reading)
                .ok_or_else(|| {
                    LetterStop::Fault(InternalFault::new(
                        "temporal.observed",
                        "tape-letter-reads-every-atom",
                    ))
                });
        }
        let (objects, over) = self.subject(position)?;
        let evaluation = self
            .package
            .evaluate_holds(self.clause, atom, over, objects, meter);
        let reading = Self::reading(&mut self.stopped, position, evaluation, HOLDS_CODES)?;
        if let AtomReading::Undefined(cause) = &reading {
            self.note(|| ObservedLocus::Atom(atom), position, cause);
        }
        Ok(reading)
    }

    fn valuation(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<Valuation, LetterStop> {
        let key = u64::try_from(atom.index())
            .map(|atom| (position.index(), atom))
            .map_err(|_| {
                LetterStop::Fault(InternalFault::new("temporal.observed", "atom-in-u64"))
            })?;
        if self.absent.contains(&key) {
            return Ok(Valuation::Missing);
        }
        self.atom(atom, position, meter).map(Valuation::from)
    }

    fn atom_class(&self, atom: FormulaNodeId) -> usize {
        self.classes
            .get(atom.index())
            .copied()
            .unwrap_or(atom.index())
    }

    fn activation(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        if let Some(letter) = self.taped(position) {
            return match letter.activation() {
                Some(true) => Ok(AtomReading::True),
                Some(false) => Ok(AtomReading::False),
                None => Err(LetterStop::Fault(InternalFault::new(
                    "temporal.observed",
                    "tape-letter-reads-the-guard",
                ))),
            };
        }
        let (objects, over) = self.subject(position)?;
        let evaluation = self
            .package
            .evaluate_guard(self.clause, over, objects, meter);
        let reading = Self::reading(&mut self.stopped, position, evaluation, ACTIVATION_CODES)?;
        if let AtomReading::Undefined(cause) = &reading {
            self.note(|| ObservedLocus::Activation, position, cause);
        }
        Ok(reading)
    }

    fn captures(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<CaptureReading, LetterStop> {
        if let Some(letter) = self.taped(position) {
            return if letter.captures() {
                Ok(CaptureReading::Fixed)
            } else {
                Err(LetterStop::Fault(InternalFault::new(
                    "temporal.observed",
                    "tape-letter-reads-the-captures",
                )))
            };
        }
        self.fix_captures(position, meter)
    }
}

impl ObservedLetters<'_> {
    /// FR-337 "Capture observation": the clause's captures at `position`, in
    /// source order, each through the clause evaluator with the `over`
    /// parameter and the trigger binder both `over` and each earlier capture
    /// bound to its value.
    fn fix_captures(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<CaptureReading, LetterStop> {
        let clause = self.clause.clause();
        if clause.captures().len() == 0 {
            return Ok(CaptureReading::Fixed);
        }
        let (objects, over) = self.subject(position)?;
        let binders = 1 + usize::from(clause.trigger().is_some());
        let mut values: Vec<Value> = Vec::new();
        for (index, (name, _)) in clause.captures().enumerate() {
            let (body, slots) = self.clause.capture(index).ok_or_else(|| {
                LetterStop::Fault(InternalFault::new(
                    "temporal.observed",
                    "clause-has-its-typed-captures",
                ))
            })?;
            let mut arguments = vec![over.clone(); binders];
            arguments.extend(values.iter().cloned());
            let evaluation = evaluate_capture(
                self.package,
                &Capture { body, slots },
                objects,
                arguments,
                None,
                meter,
            )
            .map_err(LetterStop::Fault)?;
            if let FamilyOutcome::Evaluated(Outcome::Completed(value)) = &evaluation.outcome {
                values.push(value.clone());
                continue;
            }
            return match Self::reading(&mut self.stopped, position, Ok(evaluation), CAPTURE_CODES)?
            {
                AtomReading::Undefined(cause) => {
                    self.note(|| ObservedLocus::Capture(name.to_owned()), position, &cause);
                    Ok(CaptureReading::Undefined {
                        capture: name.to_owned(),
                        cause,
                    })
                }
                AtomReading::True | AtomReading::False => Err(LetterStop::Fault(
                    InternalFault::new("temporal.observed", CAPTURE_CODES[1]),
                )),
            };
        }
        Ok(CaptureReading::Fixed)
    }
}

/// The fault codes of a capture's initializer, as [`HOLDS_CODES`].
const CAPTURE_CODES: [&str; 2] = [
    "capture-initializer-call-failed",
    "capture-initializer-completes-a-value",
];

/// One capture's initializer as a predicate of the clause evaluator.
struct Capture<'n> {
    body: fsl_semantics::check::CheckedNode<'n>,
    slots: usize,
}

impl fsl_semantics::check::StatePredicate for Capture<'_> {
    fn body(&self) -> fsl_semantics::check::CheckedNode<'_> {
        self.body
    }

    fn slots(&self) -> usize {
        self.slots
    }
}

/// FR-820: what reading one record sample found besides its letter: the
/// condition stop and the first undefined reading there, at the sample's
/// position in the trace.
#[derive(Debug, Default)]
pub struct SampleStop {
    /// The condition stop the reading met.
    pub stopped: Option<HoldsStop>,
    /// The first undefined reading.
    pub undefined: Option<FirstUndefined>,
}

impl SampleStop {
    /// Whether the reading met no stop and no undefined reading.
    pub fn is_empty(&self) -> bool {
        self.stopped.is_none() && self.undefined.is_none()
    }

    fn of(letters: ObservedLetters<'_>, index: u64) -> Self {
        let at = TemporalPosition::new(index);
        Self {
            stopped: letters.stopped.map(|stop| match stop {
                HoldsStop::Refused { evaluation, .. } => HoldsStop::Refused { at, evaluation },
                failed @ HoldsStop::Failed(_) => failed,
            }),
            undefined: letters.undefined.map(|first| FirstUndefined {
                position: at,
                ..first
            }),
        }
    }
}

/// FR-820: read the record sample `sample` as the next position of `stream`,
/// the streaming form of [`evaluate_sampled`] over a `Finite` trace.
///
/// # Errors
///
/// [`StreamStop`](super::StreamStop) when `meter` runs out; the stream is
/// unchanged.
pub fn push_sample(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    stream: &mut super::ClauseStream,
    sample: &Value,
    meter: &mut Meter,
) -> Result<SampleStop, super::StreamStop> {
    push_position(
        package,
        clause,
        stream,
        ObservedPositions::Samples(std::slice::from_ref(sample)),
        meter,
    )
}

/// FR-820: read the snapshot position whose object environment is
/// `environment` as the next position of `stream`, with the clause's `over`
/// parameter bound to `over`: the streaming form of [`evaluate_observed`] over
/// a `Finite` trace.
///
/// # Errors
///
/// [`StreamStop`](super::StreamStop) when `meter` runs out; the stream is
/// unchanged.
pub fn push_object(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    stream: &mut super::ClauseStream,
    (environment, over): (&ObjectEnvironment, &ObjectReference),
    meter: &mut Meter,
) -> Result<SampleStop, super::StreamStop> {
    push_position(
        package,
        clause,
        stream,
        ObservedPositions::Objects {
            environments: std::slice::from_ref(environment),
            over,
        },
        meter,
    )
}

/// Read the one position `positions` holds as the next position of `stream`.
fn push_position(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    stream: &mut super::ClauseStream,
    positions: ObservedPositions<'_>,
    meter: &mut Meter,
) -> Result<SampleStop, super::StreamStop> {
    let index = stream.len();
    let mut letters =
        ObservedLetters::over(package, clause, positions, TemporalTrace::Finite { len: 1 });
    stream.push(
        clause.clause(),
        &mut letters,
        TemporalPosition::new(0),
        meter,
    )?;
    Ok(SampleStop::of(letters, index))
}

/// FR-820: read the record sample `sample` at position `index` again for
/// the stop it meets, as [`push_sample`] read it.
pub fn read_sample_stop(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    sample: &Value,
    index: u64,
    meter: &mut Meter,
) -> SampleStop {
    read_position_stop(
        package,
        clause,
        ObservedPositions::Samples(std::slice::from_ref(sample)),
        index,
        meter,
    )
}

/// FR-820: read the snapshot position `index`, whose object environment is
/// `environment`, again for the stop it meets, as [`push_object`] read it.
pub fn read_object_stop(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    (environment, over): (&ObjectEnvironment, &ObjectReference),
    index: u64,
    meter: &mut Meter,
) -> SampleStop {
    read_position_stop(
        package,
        clause,
        ObservedPositions::Objects {
            environments: std::slice::from_ref(environment),
            over,
        },
        index,
        meter,
    )
}

fn read_position_stop(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    positions: ObservedPositions<'_>,
    index: u64,
    meter: &mut Meter,
) -> SampleStop {
    let mut letters =
        ObservedLetters::over(package, clause, positions, TemporalTrace::Finite { len: 1 });
    let checked = clause.clause();
    let plan = match super::plan::Plan::new(checked.formula()) {
        Ok(plan) => plan,
        Err(_) => return SampleStop::default(),
    };
    let reader = super::engine::PositionReader {
        formula: checked.formula(),
        atoms: &plan.atoms,
        guarded: matches!(
            checked.activation(),
            fsl_semantics::check::Activation::Each { guarded: true }
        ),
        captured: checked.capture_site(),
    };
    // The reading only recovers what the letters noted; its letter was read
    // when the position was pushed.
    let _ = reader.read(
        &mut letters,
        TemporalPosition::new(0),
        TemporalPosition::new(index),
        meter,
    );
    SampleStop::of(letters, index)
}

/// The snapshot tape a decisive descent reads: the tape, the object bound to
/// the clause's `over` parameter, and the tape entry the descent starts at.
#[derive(Clone, Copy, Debug)]
pub struct DescentSlice<'a> {
    /// The tape.
    pub tape: &'a super::tape::LetterTape,
    /// The object bound to `over`.
    pub over: &'a ObjectReference,
    /// The first tape entry the descent reads.
    pub from: u64,
}

/// FR-330 "The decisive atom", FR-820: the descent to the decisive atom of
/// the violated activation `at` over `slice`, to the end of a trace of `len`
/// positions whose positions from `silent_from` on are silent samples
/// (FR-504). `slice.from` is at most `at` less the formula's past reach
/// ([`ClauseStream::descent_floor`](super::ClauseStream::descent_floor)), so
/// the descent reads every value the batch descent reads. The atom's position
/// is the trace's; the run's other members read the slice. `None` when `at`
/// lies before the slice.
pub fn descend_positions(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    slice: DescentSlice<'_>,
    (len, at, silent_from): (u64, u64, Option<u64>),
    meter: &mut Meter,
) -> Option<ObservedRun> {
    let DescentSlice { tape, over, from } = slice;
    let relative = at.checked_sub(from)?;
    let run = evaluate_observed_goal(
        package,
        clause,
        ObservedPositions::ObjectTape { tape, over, from },
        TemporalTrace::Finite {
            len: len.checked_sub(from)?,
        },
        (
            super::engine::Goal::At(TemporalPosition::new(relative)),
            true,
            silent_from.map(|first| first.saturating_sub(from)),
        ),
        meter,
    );
    let shift = |atom: super::engine::DecisiveAtom| super::engine::DecisiveAtom {
        position: TemporalPosition::new(atom.position.index().saturating_add(from)),
        ..atom
    };
    Some(ObservedRun {
        decisive: run
            .decisive
            .map(|search| search.map(|found| found.map(shift))),
        ..run
    })
}

#[cfg(test)]
mod tests {
    use filament_exact::{CheckedInvariantCause, Refusal};
    use ix_trace_rs::trace;

    use super::*;

    /// FR-327-AC-6: a condition whose S6a evaluation is the kernel
    /// `Refusal::CheckedInvariant` at position 1 is kept whole as a refused
    /// condition at position 1, ending the run with `LetterStop::Refused`
    /// and not as a fault, as every other refusal is; FR-330 reports it as
    /// an internal failure (the report layer's mapping).
    #[trace("TC-1160", "FR-327-AC-6")]
    #[test]
    fn a_checked_invariant_refusal_is_a_refused_condition_at_its_position() {
        let mut stopped = None;
        let evaluation = Evaluation {
            outcome: FamilyOutcome::Evaluated(Outcome::Refused(Refusal::CheckedInvariant {
                cause: CheckedInvariantCause::EqualityOperandSourceNotAdmitted,
            })),
            location: None,
            losses: Vec::new(),
            frame_depth: 0,
        };
        let reading = ObservedLetters::reading(
            &mut stopped,
            TemporalPosition::new(1),
            Ok(evaluation),
            HOLDS_CODES,
        );
        assert_eq!(reading, Err(LetterStop::Refused));
        let Some(HoldsStop::Refused { at, evaluation }) = stopped else {
            panic!("the refusal is kept whole, got {stopped:?}");
        };
        assert_eq!(at, TemporalPosition::new(1));
        assert!(matches!(
            evaluation.outcome,
            FamilyOutcome::Evaluated(Outcome::Refused(Refusal::CheckedInvariant {
                cause: CheckedInvariantCause::EqualityOperandSourceNotAdmitted
            }))
        ));
    }

    /// A family refusal with the catalog code `invalid_runtime_input`/
    /// `absent-key`.
    #[derive(Debug)]
    struct FamilyCause;

    impl filament_semantic_value::catalog::CatalogCoded for FamilyCause {
        fn catalog_code(&self) -> filament_semantic_value::catalog::CatalogCode {
            filament_semantic_value::catalog::CatalogCode::new(
                "invalid_runtime_input",
                "absent-key",
            )
        }

        fn catalog_fields(&self) -> Option<std::collections::BTreeMap<&'static str, String>> {
            None
        }
    }

    /// FR-327 "Refused condition": a family's `FamilyResult::Refused` is a
    /// refused condition as a kernel refusal is, kept whole at its position.
    #[trace("TC-1160", "FR-327-AC-6")]
    #[test]
    fn a_family_refusal_is_a_refused_condition_at_its_position() {
        let mut stopped = None;
        let evaluation = Evaluation {
            outcome: FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(Box::new(FamilyCause))),
            location: None,
            losses: Vec::new(),
            frame_depth: 0,
        };
        let reading = ObservedLetters::reading(
            &mut stopped,
            TemporalPosition::new(2),
            Ok(evaluation),
            HOLDS_CODES,
        );
        assert_eq!(reading, Err(LetterStop::Refused));
        let Some(HoldsStop::Refused { at, evaluation }) = stopped else {
            panic!("the refusal is kept whole, got {stopped:?}");
        };
        assert_eq!(at, TemporalPosition::new(2));
        assert!(matches!(
            evaluation.outcome,
            FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))
        ));
    }
}

/// What [`observed_valuation`] returns: the reading of every `holds` atom of
/// a clause at each represented position of a closed finite trace, and the
/// clause formula's native value at each activation position (FR-757
/// "Cross-evaluation").
#[derive(Debug)]
pub struct ObservedValuation {
    /// The clause's `holds` atoms, in arena order.
    pub atoms: Vec<FormulaNodeId>,
    /// For each represented position, each atom's reading, in the order of
    /// [`Self::atoms`].
    pub readings: Vec<Vec<AtomReading>>,
    /// Each activation position, in order, with the formula's native value
    /// there.
    pub activations: Vec<(TemporalPosition, TemporalOutcome)>,
}

/// Why [`observed_valuation`] has no valuation.
#[derive(Debug)]
pub enum ValuationStop {
    /// The trace is not a closed finite trace.
    NotFinite,
    /// A letter stopped: a condition refused or failed, a meter stopped, or
    /// an internal invariant broke.
    Letter(LetterStop),
    /// The evaluator refused the trace at an activation position.
    Refused(TemporalRefusal),
}

/// The valuation FR-757's cross-evaluation compares: each `holds` atom of
/// `clause` read by the one clause evaluator at every position of the
/// closed finite `trace`, the activation positions (position 0 `on origin`;
/// every position `on each` whose `when` condition, if any, holds) and the
/// formula's value at each, as [`evaluate_observed_at`] reads it.
pub fn observed_valuation(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    positions: ObservedPositions<'_>,
    trace: TemporalTrace,
    meter: &mut Meter,
) -> Result<ObservedValuation, ValuationStop> {
    let TemporalTrace::Finite { len } = trace else {
        return Err(ValuationStop::NotFinite);
    };
    let checked = clause.clause();
    let atoms: Vec<FormulaNodeId> = checked
        .formula()
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node, fsl_semantics::check::FormulaNode::Holds { .. }))
        .map(|(index, _)| FormulaNodeId::from_arena_index(index))
        .collect();
    let mut letters = ObservedLetters {
        package,
        clause,
        positions,
        empty: ObjectEnvironment::default(),
        trace,
        stopped: None,
        undefined: None,
        absent: &[],
        classes: &[],
        silent_from: None,
    };
    let mut readings = Vec::new();
    let mut activations = Vec::new();
    for index in 0..len {
        let position = TemporalPosition::new(index);
        let row = atoms
            .iter()
            .map(|atom| letters.atom(*atom, position, meter))
            .collect::<Result<Vec<_>, _>>()
            .map_err(ValuationStop::Letter)?;
        readings.push(row);
        let active = match checked.activation() {
            fsl_semantics::check::Activation::Origin => index == 0,
            fsl_semantics::check::Activation::Each { guarded: false } => true,
            fsl_semantics::check::Activation::Each { guarded: true } => matches!(
                letters
                    .activation(position, meter)
                    .map_err(ValuationStop::Letter)?,
                AtomReading::True
            ),
        };
        if active {
            let (result, _) = super::engine::run_seeking(
                super::engine::Subject::of_clause(checked),
                trace,
                super::engine::Goal::At(position),
                false,
                &mut letters,
                meter,
            );
            activations.push((position, result.map_err(ValuationStop::Refused)?));
        }
    }
    Ok(ObservedValuation {
        atoms,
        readings,
        activations,
    })
}
