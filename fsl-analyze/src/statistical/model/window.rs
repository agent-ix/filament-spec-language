// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-4's reading of one timed run (FR-254): the run is `fsl-eval`'s sampled
//! run, and each formula and measure is read from its positions by the
//! reference evaluator (ADR-038 RL-1).

use filament_exact::{Integer, Meter, Rational};
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::simulation::model::{
    sample_timed_from, PredicateReading, TimedRaceStop, TimedSample, TimedSampleEnd,
    TimedSampleStop, TimedState, TimedStop,
};
use fsl_eval::simulation::{ChoicePoint, SampledStep};
use fsl_eval::temporal::{
    evaluate_timed_formula_at, idle_tail_fraction, idle_tail_stutters, timed_accumulate,
    timed_delay_fraction, timed_elapsed, timed_fraction, timed_steps, timed_weighted_fraction,
    AtomReading, LetterStop, MeasureReading, TemporalLetters, TemporalOutcome, TemporalPosition,
    TimedTrace, UndefinedCause,
};
use fsl_replay::statistical::{IdleTailReason, IdleTailUndecided, MeasureValue};
use fsl_replay::{ClaimPredicate, UndefinedEvaluation, UndefinedLocus, UndefinedSite};
use fsl_semantics::check::{CheckedPredicate, CheckedTemporalFormula, FormulaNodeId, Weighting};
use ix_trace_rs::implements;

use super::{broken, compares, judged, magnitude, undecided, Ended, Reading, Shared, Truths};
use crate::statistical::{Drawn, Sample, SampleHalt, TestSubject, UnitFraction};

/// The halt of a refused timed sample, or `None` where a tick subject's step
/// has an undecided successor, which ends a sample as a sample.
fn sample_halt(
    stop: TimedSampleStop,
    max_sample_steps: u64,
) -> Result<Option<TimedSample>, SampleHalt> {
    Err(match stop {
        TimedSampleStop::Race(TimedRaceStop::NotStochastic { state, identity }) => {
            SampleHalt::NotStochastic { state, identity }
        }
        TimedSampleStop::Race(TimedRaceStop::Timed(TimedStop::Expansion(stop))) => {
            SampleHalt::Expansion(stop)
        }
        TimedSampleStop::MaxSampleSteps => SampleHalt::MaxSampleSteps {
            value: max_sample_steps,
        },
        TimedSampleStop::Step(stop) => match Ended::from(*stop) {
            Ended::Halt(halt) => halt,
            Ended::Undecided => return Ok(None),
        },
        // Only an unbounded walk refuses a time-lock.
        TimedSampleStop::TimeLockedSample { .. } | TimedSampleStop::Race(_) => broken(),
    })
}

/// `sample` read as `reading`, with the steps the run took.
fn drawn(run: TimedSample, reading: Sample) -> Drawn {
    Drawn {
        sample: reading,
        steps: run.steps,
    }
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// The atoms of a claim formula, each read by `read`.
struct FormulaLetters<F>(F);

impl<F: FnMut(FormulaNodeId, usize) -> AtomReading> TemporalLetters for FormulaLetters<F> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        // An index past `usize` names no position of the run.
        let position = usize::try_from(position.index()).unwrap_or(usize::MAX);
        Ok((self.0)(atom, position))
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        // A claim's formula carries no activation of its own.
        Err(LetterStop::Fault(InternalFault::new(
            "statistical",
            "claim-formula-has-no-activation",
        )))
    }
}

impl Shared<'_, '_, '_> {
    /// Sample `test`'s timed run at `trace` through the window of length
    /// `within`, which starts at the first position where `starts` holds,
    /// or at the first position where there is no `starts`. `None` where a
    /// tick subject's step has an undecided successor.
    #[implements("FR-254-AC-3")]
    #[implements("FR-254-AC-9")]
    fn timed_run(
        &self,
        test: &TestSubject,
        binding: usize,
        seed: u64,
        trace: u64,
        within: &Rational,
        starts: Option<&CheckedPredicate>,
    ) -> Result<Option<TimedSample>, SampleHalt> {
        let timed = self.timed_system.as_ref().ok_or_else(broken)?;
        let initial = timed
            .initial()
            .into_iter()
            .nth(usize::try_from(test.initial).unwrap_or(usize::MAX))
            .ok_or_else(broken)?;
        if let Some(cause) = self.cancel.cause() {
            return Err(SampleHalt::Cancelled(cause));
        }
        let over = self
            .bindings
            .get(binding)
            .and_then(|(_, over)| over.as_ref());
        let budget = self.timed_sampling.max_sample_steps;
        let mut stopped = None;
        // A position where the predicate has no truth value starts the
        // window: the reading stops there.
        let mut starts_here = |state: &TimedState| {
            let Some(predicate) = starts else {
                return true;
            };
            match self
                .system
                .state_predicates(state.discrete(), &[(predicate, over)])
            {
                Ok(readings) => !matches!(readings.first(), Some(PredicateReading::False)),
                Err(stop) => {
                    stopped = Some(stop);
                    true
                }
            }
        };
        let run = sample_timed_from(
            timed,
            &initial,
            self.workload,
            ChoicePoint {
                seed,
                trace,
                step: 0,
                choice: 0,
            },
            self.timed_sampling.q,
            within,
            &mut starts_here,
            budget,
        );
        if let Some(stop) = stopped {
            return Err(SampleHalt::Expansion(stop));
        }
        match run {
            Ok(run) => Ok(Some(run)),
            Err(stop) => sample_halt(stop, budget),
        }
    }

    /// The meter of a reading: the run's length is bounded by its
    /// `max_sample_steps`, so the work of one reading is not.
    fn reading_meter(&self) -> Meter {
        let mut limits = self.system.meter_limits();
        limits.work_units = u64::MAX;
        limits.result_units = u64::MAX;
        Meter::new(limits)
    }

    /// The site of `run`'s position `position`, stamped `stamp`.
    fn site(
        &self,
        run: &TimedSample,
        position: usize,
        stamp: &Rational,
        locus: UndefinedLocus,
    ) -> Result<UndefinedSite, SampleHalt> {
        // A dense idle tail's stutter positions repeat the last state.
        let state = run.states.get(position).unwrap_or(&run.state);
        Ok(UndefinedSite {
            position: u64::try_from(position).map_err(|_| broken())?,
            state_key: self
                .system
                .state_key(state.discrete(), fsl_foundation::IdentityLimits::default())
                .map_err(|_| broken())?
                .1,
            locus,
            transition: None,
            time: Some(Box::new(fsl_replay::TimedPoint {
                stamp: stamp.clone(),
                clocks: state.clocks().values().to_vec(),
            })),
        })
    }

    /// The predicates `reads` names at `run`'s position `position`.
    fn truths_at(
        &self,
        run: &TimedSample,
        position: usize,
        binding: usize,
        reads: &[(UndefinedLocus, &CheckedPredicate)],
    ) -> Result<Truths, SampleHalt> {
        let state = run.states.get(position).ok_or_else(broken)?;
        let stamp = run.stamps.get(position).ok_or_else(broken)?;
        let over = self
            .bindings
            .get(binding)
            .and_then(|(_, over)| over.as_ref());
        let predicates: Vec<_> = reads.iter().map(|(_, read)| (*read, over)).collect();
        let readings = self
            .system
            .state_predicates(state.discrete(), &predicates)
            .map_err(SampleHalt::Expansion)?;
        let mut truths = Vec::with_capacity(readings.len());
        for ((locus, _), reading) in reads.iter().zip(readings) {
            match reading {
                PredicateReading::True => truths.push(true),
                PredicateReading::False => truths.push(false),
                PredicateReading::Undefined { cause, .. } => {
                    return Ok(Err(Box::new(UndefinedEvaluation {
                        site: self.site(run, position, stamp, locus.clone())?,
                        cause,
                    })));
                }
                PredicateReading::Refused(outcome) => {
                    return Err(SampleHalt::PredicateRefused(outcome));
                }
            }
        }
        Ok(Ok(truths))
    }

    /// An `accumulate`, `steps` or `elapsed` measure within a time horizon
    /// (FSpec FR-407 "Time windows" and "Elapsed"). A stutter position
    /// changes no predicate and has reward 0, so every completion of a dense
    /// idle tail gives the same reading, and a measure a time-lock leaves
    /// uncompleted is censored.
    #[implements("FR-254-AC-3")]
    #[implements("FR-254-AC-4")]
    #[implements("FR-254-AC-7")]
    #[implements("FR-254-AC-9")]
    pub(super) fn timed_measure(
        &self,
        test: &TestSubject,
        binding: usize,
        seed: u64,
        trace: u64,
        horizon: &Rational,
    ) -> Result<Drawn, SampleHalt> {
        let Reading::Measure { measure, judge } = &self.reading else {
            return Err(broken());
        };
        let starts = measure.elapsed.then_some(measure.from);
        let Some(run) = self.timed_run(test, binding, seed, trace, horizon, starts)? else {
            return Ok(undecided());
        };
        let reads = [
            (
                UndefinedLocus::ClaimPredicate(ClaimPredicate::From),
                measure.from,
            ),
            (
                UndefinedLocus::ClaimPredicate(ClaimPredicate::Until),
                measure.until,
            ),
        ];
        let (mut from, mut until) = (Vec::new(), Vec::new());
        let mut undefined = None;
        for position in 0..run.states.len() {
            let truths = match self.truths_at(&run, position, binding, &reads)? {
                Ok(truths) => truths,
                Err(evaluation) => {
                    undefined = Some(*evaluation);
                    break;
                }
            };
            let [a, b] = truths[..] else {
                return Err(broken());
            };
            from.push(a);
            until.push(b);
        }
        // An undefined predicate leaves the positions before it, a prefix
        // that may still decide the measure.
        let timed = match &undefined {
            Some(_) if from.is_empty() => None,
            Some(_) => Some(TimedTrace::new(
                run.stamps.get(..from.len()).ok_or_else(broken)?.to_vec(),
            )),
            None => Some(run.timed_trace()),
        };
        let origin = TemporalPosition::new(0);
        let mut meter = self.reading_meter();
        let reading = match timed {
            None => MeasureReading::Pending,
            Some(timed) => {
                let timed = timed.map_err(|_| broken())?;
                let read = match (measure.elapsed, measure.reward) {
                    (true, _) => timed_elapsed(&timed, origin, &from, &until, horizon, &mut meter),
                    (false, None) => {
                        timed_steps(&timed, origin, &from, &until, horizon, &mut meter)
                    }
                    (false, Some(name)) => {
                        // The reward of the step into each position; none
                        // leads into the first or into a stutter position.
                        let mut rewards = vec![zero()];
                        for step in &run.steps {
                            rewards.push(magnitude(step, name)?);
                        }
                        rewards.resize(run.states.len(), zero());
                        timed_accumulate(
                            &timed, origin, &from, &until, &rewards, horizon, &mut meter,
                        )
                    }
                };
                read.map_err(|_| broken())?
            }
        };
        let sample = match (reading, undefined) {
            (MeasureReading::Value(value), _) => {
                judged(judge, Some(MeasureValue::Value(value)), true)
            }
            (_, Some(evaluation)) => Sample::Undefined(evaluation),
            (MeasureReading::Censored, None) => judged(judge, None, true),
            (MeasureReading::NotActivated, None) => judged(judge, None, false),
            // A finished run in which `A` never holds starts no `elapsed`
            // clock and gains no further position.
            (MeasureReading::Pending, None) if !from.contains(&true) => judged(judge, None, false),
            (
                MeasureReading::Pending | MeasureReading::NoWeight | MeasureReading::Incomplete(_),
                None,
            ) => return Err(broken()),
        };
        Ok(drawn(run, sample))
    }

    /// A fraction over the positions of a time window (FSpec FR-407
    /// "Fraction", "Time windows"). How a fraction reads a run that ends in a
    /// time-lock is not decided, so such a sample is unsupported (FSpec
    /// FR-420 "Sampling timed runs").
    #[implements("FR-254-AC-4")]
    #[implements("FR-254-AC-5")]
    #[implements("FR-254-AC-7")]
    #[implements("FR-254-AC-8")]
    pub(super) fn timed_fraction(
        &self,
        test: &TestSubject,
        binding: usize,
        seed: u64,
        trace: u64,
        horizon: &Rational,
    ) -> Result<Drawn, SampleHalt> {
        let Reading::Fraction { fraction, event } = &self.reading else {
            return Err(broken());
        };
        let Some(run) = self.timed_run(test, binding, seed, trace, horizon, None)? else {
            return Ok(undecided());
        };
        if let TimedSampleEnd::TimeLock { state } = &run.end {
            return Err(SampleHalt::TimeLockedSample {
                state: state.clone(),
            });
        }
        let reads = [(
            UndefinedLocus::ClaimPredicate(ClaimPredicate::Target),
            &fraction.predicate,
        )];
        let mut holds = Vec::with_capacity(run.states.len());
        for position in 0..run.states.len() {
            match self.truths_at(&run, position, binding, &reads)? {
                Ok(truths) => holds.push(truths.first().copied().ok_or_else(broken)?),
                Err(undefined) => return Ok(drawn(run, Sample::Undefined(*undefined))),
            }
        }
        let timed = run.timed_trace().map_err(|_| broken())?;
        let origin = TemporalPosition::new(0);
        let mut meter = self.reading_meter();
        let reading = match &fraction.weight {
            None if run.end == TimedSampleEnd::DenseIdleTail => {
                let held = holds.iter().filter(|held| **held).count();
                let held = u64::try_from(held).map_err(|_| broken())?;
                let positions = u64::try_from(holds.len()).map_err(|_| broken())?;
                let tail = holds.last().copied().ok_or_else(broken)?;
                match idle_tail_fraction(held, positions, tail, event.as_ref()) {
                    Some(value) => Ok(MeasureReading::Value(value)),
                    None => {
                        return Err(SampleHalt::NotStochasticIdleTail {
                            state: self.tail_state(&run)?,
                        });
                    }
                }
            }
            None => timed_fraction(&timed, origin, &holds, horizon, &mut meter),
            // The step that leaves a position weighs it, even when it lands
            // after the window; a stutter weighs 0, so every completion of a
            // dense idle tail gives one value.
            Some(Weighting::Reward(name)) => {
                let crossing = match &run.end {
                    TimedSampleEnd::Crossing(step) => Some(step.as_ref()),
                    TimedSampleEnd::DenseIdleTail
                    | TimedSampleEnd::Complete
                    | TimedSampleEnd::TimeLock { .. } => None,
                };
                let leaving: Vec<&SampledStep> = run.steps.iter().chain(crossing).collect();
                let mut weights = Vec::with_capacity(run.states.len());
                for step in leaving {
                    weights.push(magnitude(step, name)?);
                }
                weights.resize(run.states.len(), zero());
                timed_weighted_fraction(&timed, origin, &holds, &weights, horizon, &mut meter)
            }
            Some(Weighting::Delay) => {
                timed_delay_fraction(&timed, origin, &holds, horizon, &mut meter)
            }
        };
        let sample = match reading.map_err(|_| broken())? {
            MeasureReading::Value(value) => match event {
                Some(bound) => Sample::Event(compares(bound, &MeasureValue::Value(value))),
                None => Sample::Fraction(UnitFraction::try_from(value).map_err(|_| broken())?),
            },
            MeasureReading::NoWeight => {
                let position = run.states.len().checked_sub(1).ok_or_else(broken)?;
                Sample::Undefined(UndefinedEvaluation {
                    site: self.site(
                        &run,
                        position,
                        &run.stamp,
                        UndefinedLocus::ClaimPredicate(ClaimPredicate::Target),
                    )?,
                    cause: UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero),
                })
            }
            MeasureReading::NotActivated
            | MeasureReading::Censored
            | MeasureReading::Pending
            | MeasureReading::Incomplete(_) => return Err(broken()),
        };
        Ok(drawn(run, sample))
    }

    /// The timed state at the first position of `run`'s idle tail.
    fn tail_state(&self, run: &TimedSample) -> Result<fsl_eval::simulation::StateKey, SampleHalt> {
        let timed = self.timed_system.as_ref().ok_or_else(broken)?;
        timed.state_key(&run.state).map_err(|_| broken())
    }

    /// A timed formula over the positions of its window. A delay is not a
    /// position (FSpec FR-421, FR-416), so only a stutter position can follow
    /// the sampled ones; on a dense idle tail the sample is read over every
    /// completion of the tail (FSpec FR-420 "Windows on an idle tail"). A
    /// run that ends time-locked is read over the positions it has.
    #[implements("FR-254-AC-4")]
    #[implements("FR-254-AC-7")]
    #[implements("FR-254-AC-8")]
    pub(super) fn timed_formula(
        &self,
        test: &TestSubject,
        binding: usize,
        seed: u64,
        trace: u64,
        formula: &CheckedTemporalFormula,
        horizon: &Rational,
    ) -> Result<Drawn, SampleHalt> {
        let Some(run) = self.timed_run(test, binding, seed, trace, horizon, None)? else {
            return Ok(undecided());
        };
        if run.end != TimedSampleEnd::DenseIdleTail {
            let sample = self.completion(&run, binding, formula, &[])?;
            return Ok(drawn(run, sample));
        }
        // With no temporal operator inside another, a completion reads as the
        // choice of which interval pieces hold a stutter position.
        let end = run.window_end.as_ref().ok_or_else(broken)?;
        let Some(pieces) = idle_tail_stutters(formula.formula(), &run.stamp, end) else {
            return Err(SampleHalt::IdleTailUndecided(IdleTailUndecided {
                state: self.tail_state(&run)?,
                reason: IdleTailReason::TimedFormula,
            }));
        };
        let mut agreed = self.completion(&run, binding, formula, &[])?;
        for choice in 1u32..(1u32 << pieces.len()) {
            if let Some(cause) = self.cancel.cause() {
                return Err(SampleHalt::Cancelled(cause));
            }
            let stutters: Vec<&Rational> = pieces
                .iter()
                .enumerate()
                .filter(|(piece, _)| choice & (1 << piece) != 0)
                .map(|(_, stamp)| stamp)
                .collect();
            let read = self.completion(&run, binding, formula, &stutters)?;
            match (&agreed, read) {
                (Sample::Event(first), Sample::Event(other)) if *first == other => {}
                (Sample::Undefined(_), Sample::Undefined(evaluation)) => {
                    agreed = Sample::Undefined(evaluation);
                }
                _ => {
                    return Err(SampleHalt::NotStochasticIdleTail {
                        state: self.tail_state(&run)?,
                    });
                }
            }
        }
        Ok(drawn(run, agreed))
    }

    /// `formula` over `run`'s positions followed by stutter positions at
    /// `stutters`, in ascending order, read by the reference evaluator as a
    /// trace complete through the window's end.
    fn completion(
        &self,
        run: &TimedSample,
        binding: usize,
        formula: &CheckedTemporalFormula,
        stutters: &[&Rational],
    ) -> Result<Sample, SampleHalt> {
        let end = run.window_end.as_ref().ok_or_else(broken)?;
        let mut stamps = run.stamps.clone();
        stamps.extend(stutters.iter().map(|stamp| (*stamp).clone()));
        let timed = TimedTrace::through(stamps.clone(), end.clone()).map_err(|_| broken())?;
        let over = self
            .bindings
            .get(binding)
            .and_then(|(_, over)| over.as_ref());
        let mut failure = None;
        let mut first_undefined = None;
        // A reading that fails stops the sample below, whatever it returns
        // here.
        let unread = || {
            AtomReading::Undefined(UndefinedCause::Kernel(
                filament_exact::Undefined::DivisionByZero,
            ))
        };
        let mut letters = FormulaLetters(|atom: FormulaNodeId, position: usize| {
            let state = run.states.get(position).unwrap_or(&run.state);
            let Some(predicate) = formula.condition(atom.index()) else {
                failure = Some(broken());
                return unread();
            };
            let readings = match self
                .system
                .state_predicates(state.discrete(), &[(predicate, over)])
            {
                Ok(readings) => readings,
                Err(stop) => {
                    failure = Some(SampleHalt::Expansion(stop));
                    return unread();
                }
            };
            match readings.first() {
                Some(PredicateReading::True) => AtomReading::True,
                Some(PredicateReading::False) => AtomReading::False,
                Some(PredicateReading::Undefined { cause, .. }) => {
                    first_undefined.get_or_insert((position, atom));
                    AtomReading::Undefined(cause.clone())
                }
                Some(PredicateReading::Refused(outcome)) => {
                    failure = Some(SampleHalt::PredicateRefused(outcome.clone()));
                    unread()
                }
                None => {
                    failure = Some(broken());
                    unread()
                }
            }
        });
        let outcome = evaluate_timed_formula_at(
            formula.formula(),
            &timed,
            TemporalPosition::new(0),
            &mut letters,
            &mut self.reading_meter(),
        );
        if let Some(failure) = failure {
            return Err(failure);
        }
        Ok(match outcome.map_err(|_| broken())? {
            TemporalOutcome::Holds => Sample::Event(true),
            TemporalOutcome::Violated { .. } => Sample::Event(false),
            TemporalOutcome::Undefined(undefined) => {
                let (position, atom) = first_undefined.ok_or_else(broken)?;
                let stamp = stamps.get(position).ok_or_else(broken)?;
                Sample::Undefined(UndefinedEvaluation {
                    site: self.site(run, position, stamp, UndefinedLocus::Atom(atom))?,
                    cause: undefined.cause,
                })
            }
            TemporalOutcome::Pending
            | TemporalOutcome::InsufficientData(_)
            | TemporalOutcome::MissingFairnessPremise { .. }
            | TemporalOutcome::Incomplete(_)
            | TemporalOutcome::WindowLimitReached(_) => return Err(broken()),
        })
    }
}
