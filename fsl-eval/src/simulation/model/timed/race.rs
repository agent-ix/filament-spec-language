// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-253/FR-254: one exact timed race and the ordered tie steps it takes.

use filament_exact::{Identifier, Integer, Rational, Value};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_semantics::check::{DelayDistribution, Workload};
use ix_trace_rs::implements;

use super::{GuardedStep, TimedState, TimedStop, TimedSystem};
use crate::simulation::choice::{weighted_choice_counted, ChoicePoint};
use crate::simulation::model::TransitionId;
use crate::simulation::sampled::{draw_timed_delay, take_enabled_step, TimedDelayRefusal};
use crate::simulation::sampled::{SampleStop, SampledBehaviour, SampledStep, StepAt};
use crate::simulation::StateKey;
use crate::temporal::{TemporalRefusal, TimedTrace};

/// One transition taken by a timed race. The first transition has the
/// minimum sampled delay; later transitions in the same tie have delay 0.
#[derive(Clone, Debug)]
pub struct TimedRaceStep {
    /// The operation transition.
    pub transition: ModelTransition,
    /// The exact delay before this transition.
    pub delay: Rational,
    /// The reached timed state.
    pub state: TimedState,
    /// This ordered race outcome's probability factor.
    pub probability: Rational,
    /// The choice indices used to select its delay and tie position.
    pub choices: Vec<u32>,
}

/// One stochastic race result at a timed state.
#[derive(Clone, Debug)]
pub struct TimedRace {
    /// The ordered operation steps taken at the least sampled delay.
    pub steps: Vec<TimedRaceStep>,
    /// The timed state after the tie sequence.
    pub state: TimedState,
    /// The joint probability of the delay draws and ordered tie outcome.
    pub probability: Rational,
    /// Whether the state had no racing identity and therefore takes the idle
    /// tail.
    pub idle: bool,
    /// The race contains scheduler-owned free delays, so it is a Markov
    /// decision outcome rather than a sampled successor. The recorded draws
    /// show stochastic delay choices made before that scheduler decision.
    pub scheduler_choice: Option<TimedSchedulerChoice>,
}

/// A Markov decision race at a state with one or more free-delay identities.
#[derive(Clone, Debug)]
pub struct TimedSchedulerChoice {
    /// The timed state where the scheduler chooses the free delay.
    pub state: StateKey,
    /// Stochastic identities' exact draws at this state.
    pub draws: Vec<TimedRaceDraw>,
    /// Enabled free-delay identities and their admissible windows.
    pub free_delays: Vec<super::ScheduledWindow>,
    /// The joint probability of the draws in `draws`.
    pub probability: Rational,
}

/// One stochastic delay draw made before a scheduler-owned free delay.
#[derive(Clone, Debug)]
pub struct TimedRaceDraw {
    /// The operation whose delay was sampled.
    pub transition: ModelTransition,
    /// The exact sampled delay.
    pub delay: Rational,
    /// The probability of that delay draw.
    pub probability: Rational,
    /// The choice index used to make the draw, if it consumed one.
    pub choices: Vec<u32>,
}

/// Why a timed race has no sampled successor.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum TimedRaceStop {
    /// A free delay races, or a delay law has no defined distribution in its
    /// window. The state key and operation identify the refusal.
    #[error("no stochastic delay is defined for {identity:?} at {state:?}")]
    NotStochastic {
        /// The full timed-state key.
        state: StateKey,
        /// The declaration identity of the scheduled operation.
        identity: ModelTransition,
    },
    /// The timed model stopped while its windows or transitions were read.
    #[error("timed expansion stopped: {0:?}")]
    Timed(TimedStop),
    /// The workload does not assign positive weight to a tied operation.
    #[error("a workload weight is missing for a tied operation")]
    MissingWeight,
    /// The choice total exceeds the sampler's 256-bit range.
    #[error("a race choice exceeds the sampler's draw range")]
    DrawRange,
    /// The timed state could not be assigned its canonical state key.
    #[error("the timed state has no canonical state key")]
    StateKey,
    /// A sampled tie step is no longer enabled at its turn.
    #[error("a tied transition stopped being enabled")]
    TieDisabled,
}

/// A sampled timed run: its positions up to the end of its window, with the
/// delay carried on each transition step.
#[derive(Clone, Debug)]
pub struct TimedSample {
    /// The seed-derived trace index.
    pub trace: u64,
    /// The canonical key of the initial timed state.
    pub start: StateKey,
    /// The sampled steps, including each exact delay.
    pub steps: Vec<SampledStep>,
    /// Timed positions, initial position first, aligned with `stamps`: the
    /// post-state of each step, then a tick source's stutter positions
    /// stamped within the window. A delay is not a position.
    pub states: Vec<TimedState>,
    /// Exact absolute time stamp of each state.
    pub stamps: Vec<Rational>,
    /// The last position's timed state.
    pub state: TimedState,
    /// The last position's time stamp.
    pub stamp: Rational,
    /// The last time stamp the window admits, or `None` where no position
    /// started it.
    pub window_end: Option<Rational>,
    /// How the run ends after its last position.
    pub end: TimedSampleEnd,
}

/// How a sampled timed run ends after its last position (FSpec FR-420
/// "Sampling timed runs").
#[derive(Clone, Debug, PartialEq)]
pub enum TimedSampleEnd {
    /// The step that leaves the last position and lands after the window. It
    /// is drawn for its rewards and adds no position.
    Crossing(Box<SampledStep>),
    /// A dense idle tail enters the window before its end: any number
    /// `m >= 0` of stutter positions of the last position follow it.
    DenseIdleTail,
    /// No further position lies in the window, which is read in full.
    Complete,
    /// The run reached a time-lock before the window's end. It ends there
    /// with the positions it has (FSpec FR-416 "How claims read a
    /// time-lock").
    TimeLock {
        /// The key of the time-locked timed state.
        state: StateKey,
    },
}

/// A timed sampling request refused before a sample can be completed.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum TimedSampleStop {
    /// A race could not be sampled.
    #[error(transparent)]
    Race(#[from] TimedRaceStop),
    /// The sampled prefix exceeded its request budget before its horizon.
    #[error("the timed sample exceeded its maximum number of steps")]
    MaxSampleSteps,
    /// A tick subject's untimed step (FSpec FR-420 "Tick subjects") stopped.
    #[error("the tick subject's step stopped: {0:?}")]
    Step(Box<SampleStop>),
    /// An unbounded walk reached a time-lock, which no long-run fraction
    /// reads.
    #[error("the timed walk reached a time-lock at {state:?}")]
    TimeLockedSample {
        /// The key of the time-locked state.
        state: StateKey,
    },
}

/// A sampled timed witness disagrees with deterministic recomputation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedSampleDisagreement {
    /// The recorded step differs from the seed-derived step.
    Step {
        /// The first differing step index.
        step: u64,
    },
    /// All recorded steps agree but the final time or end does not.
    EndPosition,
}

/// Replay result for a timed sample witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedSampleReplay {
    /// The sample, including every exact delay, recomputes from its seed.
    Reproduced,
    /// The recomputed sample differs from the witness.
    Disagrees(TimedSampleDisagreement),
}

/// Sample one timed run whose window `[0, horizon]` starts at its first
/// position. See [`sample_timed_from`].
///
/// # Errors
///
/// As [`sample_timed_from`].
pub fn sample_timed(
    system: &TimedSystem<'_, '_>,
    initial: &TimedState,
    workload: &Workload,
    at: ChoicePoint,
    q: u32,
    horizon: &Rational,
    max_sample_steps: u64,
) -> Result<TimedSample, TimedSampleStop> {
    sample_timed_from(
        system,
        initial,
        workload,
        at,
        q,
        horizon,
        &mut |_| true,
        max_sample_steps,
    )
}

/// Sample one timed run, repeating FR-253's race, through the window of
/// length `within` that starts at the first position for whose state `starts`
/// answers true. `starts` is asked at each position until then, and no
/// horizon ends the run before it.
///
/// The first position is at time zero. The run ends when the next position
/// would pass the window's end, keeping that step as the crossing step; at an
/// idle tail; or at a time-lock, where delay is bounded and no identity
/// races. Tied transitions are retained in their race order; only the first
/// has a positive delay and every later tied transition has delay zero. A
/// tick subject runs no race: each step is FR-408's untimed step, stamps move
/// by the tick period and a terminal state's stutter positions are read up
/// to the window's end.
///
/// # Errors
///
/// [`TimedSampleStop`] when a race has no sampled successor, the run needs
/// more than `max_sample_steps` positions, or a tick step stops.
#[implements("FR-254-AC-3")]
#[implements("FR-254-AC-4")]
#[implements("FR-254-AC-9")]
#[implements("FSpec-FR-420-AC-4")]
#[allow(clippy::too_many_arguments)]
pub fn sample_timed_from(
    system: &TimedSystem<'_, '_>,
    initial: &TimedState,
    workload: &Workload,
    at: ChoicePoint,
    q: u32,
    within: &Rational,
    starts: &mut dyn FnMut(&TimedState) -> bool,
    max_sample_steps: u64,
) -> Result<TimedSample, TimedSampleStop> {
    let start = system
        .state_key(initial)
        .map_err(|_| TimedRaceStop::StateKey)?;
    let mut state = initial.clone();
    let mut stamp = zero();
    let mut steps = Vec::new();
    let mut states = vec![state.clone()];
    let mut stamps = vec![stamp.clone()];
    let mut window_end = starts(&state).then(|| within.clone());
    let ticking = system.tick_period();
    let positions = |steps: &Vec<SampledStep>| u64::try_from(steps.len()).unwrap_or(u64::MAX);
    let end = 'run: loop {
        if ticking.is_some() {
            let step_at = StepAt {
                seed: at.seed,
                trace: at.trace,
                step: positions(&steps),
            };
            // A tick subject admits no delay of its own: its terminal states
            // stutter (FSpec FR-420 "Tick subjects") and are no time-lock.
            let Some((step, next, advance)) = tick_step(system, &state, workload, step_at)? else {
                if let (Some(period), Some(end)) = (ticking, &window_end) {
                    let mut stutter = stamp.add(period);
                    while stutter <= *end {
                        if u64::try_from(states.len()).unwrap_or(u64::MAX) > max_sample_steps {
                            return Err(TimedSampleStop::MaxSampleSteps);
                        }
                        states.push(state.clone());
                        stamps.push(stutter.clone());
                        stamp = stutter.clone();
                        stutter = stutter.add(period);
                    }
                }
                break TimedSampleEnd::Complete;
            };
            let next_stamp = stamp.add(&advance);
            if window_end.as_ref().is_some_and(|end| next_stamp > *end) {
                break TimedSampleEnd::Crossing(Box::new(step));
            }
            if positions(&steps) >= max_sample_steps {
                return Err(TimedSampleStop::MaxSampleSteps);
            }
            steps.push(step);
            state = next;
            stamp = next_stamp;
            states.push(state.clone());
            stamps.push(stamp.clone());
            if window_end.is_none() && starts(&state) {
                window_end = Some(stamp.add(within));
            }
            continue;
        }
        let point = ChoicePoint {
            step: positions(&steps),
            choice: 0,
            ..at
        };
        let race = sample_timed_race(system, &state, workload, point, q)?;
        if let Some(choice) = &race.scheduler_choice {
            return Err(free_delay(choice)?.into());
        }
        let mut advanced = false;
        for race_step in race.steps {
            let next_stamp = stamp.add(&race_step.delay);
            let rewards = step_rewards(system, &state, &race_step.transition)?;
            let passes = window_end.as_ref().is_some_and(|end| next_stamp > *end);
            if !passes && positions(&steps) >= max_sample_steps {
                return Err(TimedSampleStop::MaxSampleSteps);
            }
            let post_state = system
                .state_key(&race_step.state)
                .map_err(|_| TimedRaceStop::StateKey)?;
            let step = SampledStep {
                transition: TransitionId::Operation(race_step.transition),
                delay: Some(race_step.delay.clone()),
                post_state,
                choices: race_step.choices,
                random: Vec::new(),
                probability: race_step.probability,
                rewards,
            };
            if passes {
                break 'run TimedSampleEnd::Crossing(Box::new(step));
            }
            steps.push(step);
            state = race_step.state;
            stamp = next_stamp;
            states.push(state.clone());
            stamps.push(stamp.clone());
            if window_end.is_none() && starts(&state) {
                window_end = Some(stamp.add(within));
            }
            advanced = true;
        }
        if !advanced {
            let remaining = window_end.as_ref().map(|end| end.sub(&stamp));
            if let Some(locked) = time_locked(system, &state, remaining.as_ref())? {
                break TimedSampleEnd::TimeLock {
                    state: system
                        .state_key(&locked)
                        .map_err(|_| TimedRaceStop::StateKey)?,
                };
            }
            let open = window_end.as_ref().is_some_and(|end| stamp < *end);
            if open
                && system
                    .is_quiescent(state.discrete())
                    .map_err(TimedRaceStop::Timed)?
            {
                break TimedSampleEnd::DenseIdleTail;
            }
            break TimedSampleEnd::Complete;
        }
    };
    Ok(TimedSample {
        trace: at.trace,
        start,
        steps,
        states,
        stamps,
        state,
        stamp,
        window_end,
        end,
    })
}

/// The refusal of a race with a scheduler-owned free delay.
fn free_delay(choice: &TimedSchedulerChoice) -> Result<TimedRaceStop, TimedRaceStop> {
    let identity = choice
        .free_delays
        .first()
        .ok_or(TimedRaceStop::StateKey)?
        .transition
        .transition()
        .clone();
    Ok(TimedRaceStop::NotStochastic {
        state: choice.state.clone(),
        identity,
    })
}

/// The time-locked state a run reaches from `state`, where no identity races:
/// delay is bounded, whether every window is empty or a non-empty one holds
/// no mass (FSpec FR-420 "The race"). The state is the one after the greatest
/// delay, or `state` itself where the bound is excluded. `None` where delay is
/// unbounded, or the bound is not before the window's end, `remaining` time
/// from here (`None`: the walk has no window).
#[implements("FSpec-FR-420-AC-4")]
#[implements("FSpec-FR-420-AC-7")]
fn time_locked(
    system: &TimedSystem<'_, '_>,
    state: &TimedState,
    remaining: Option<&Rational>,
) -> Result<Option<TimedState>, TimedRaceStop> {
    let delays = system
        .admissible_delays(state)
        .map_err(TimedRaceStop::Timed)?;
    let Some((upper, closed)) = delays.upper() else {
        return Ok(None);
    };
    if remaining.is_some_and(|remaining| upper > remaining || (closed && upper == remaining)) {
        return Ok(None);
    }
    if !closed {
        return Ok(Some(state.clone()));
    }
    system
        .delay(state, upper)
        .map_err(TimedRaceStop::Timed)?
        .ok_or(TimedRaceStop::StateKey)
        .map(Some)
}

/// What one more race of an unbounded timed walk yields (FR-420 "Delay-weighted
/// cycles").
#[derive(Clone, Debug)]
pub enum TimedAdvance {
    /// No identity races and delay is unbounded: the idle tail of a terminal
    /// state, whose weight is unbounded.
    Terminal,
    /// The race's ordered steps, taken at the least sampled delay.
    Race(Vec<TimedRaceStep>),
}

/// Race once at `state` with no horizon: the next step of a walk that runs
/// until a terminal state, a time-lock or the caller stops it. The race and
/// its free-delay refusals are those of [`sample_timed_from`].
///
/// # Errors
///
/// [`TimedSampleStop::TimeLockedSample`] where no identity races and delay is
/// bounded, and a race with no sampled successor.
#[implements("FSpec-FR-420-AC-11")]
pub fn advance_timed(
    system: &TimedSystem<'_, '_>,
    state: &TimedState,
    workload: &Workload,
    at: ChoicePoint,
    q: u32,
) -> Result<TimedAdvance, TimedSampleStop> {
    let race = sample_timed_race(system, state, workload, at, q)?;
    if let Some(choice) = &race.scheduler_choice {
        return Err(free_delay(choice)?.into());
    }
    if !race.steps.is_empty() {
        return Ok(TimedAdvance::Race(race.steps));
    }
    match time_locked(system, state, None)? {
        Some(locked) => Err(TimedSampleStop::TimeLockedSample {
            state: system
                .state_key(&locked)
                .map_err(|_| TimedRaceStop::StateKey)?,
        }),
        None => Ok(TimedAdvance::Terminal),
    }
}

/// Recompute a recorded timed sample from its seed and compare every draw,
/// delay, successor and end position (FR-254 witness replay).
#[allow(clippy::too_many_arguments)] // each value is an explicit replay input
pub fn replay_timed_sample(
    system: &TimedSystem<'_, '_>,
    initial: &TimedState,
    workload: &Workload,
    at: ChoicePoint,
    q: u32,
    horizon: &Rational,
    max_sample_steps: u64,
    record: &TimedSample,
) -> Result<TimedSampleReplay, TimedSampleStop> {
    let replayed = sample_timed(system, initial, workload, at, q, horizon, max_sample_steps)?;
    if let Some((step, _)) = replayed
        .steps
        .iter()
        .zip(&record.steps)
        .enumerate()
        .find(|(_, (actual, expected))| actual != expected)
    {
        return Ok(TimedSampleReplay::Disagrees(
            TimedSampleDisagreement::Step {
                step: u64::try_from(step).unwrap_or(u64::MAX),
            },
        ));
    }
    let same_state = system.state_key(&replayed.state).ok() == system.state_key(&record.state).ok();
    let same_start = replayed.start == record.start;
    if replayed.steps.len() != record.steps.len()
        || replayed.trace != record.trace
        || !same_start
        || replayed.stamp != record.stamp
        || replayed.window_end != record.window_end
        || replayed.end != record.end
        || !same_state
    {
        return Ok(TimedSampleReplay::Disagrees(
            TimedSampleDisagreement::EndPosition,
        ));
    }
    Ok(TimedSampleReplay::Reproduced)
}

impl TimedSample {
    /// The run's positions as the reference evaluator reads them: complete
    /// through the window's end, since no further position can lie in the
    /// window. A dense idle tail's stutter positions are not among them.
    ///
    /// # Errors
    ///
    /// As [`TimedTrace::through`], for a run whose stamps are not a trace.
    pub fn timed_trace(&self) -> Result<TimedTrace, TemporalRefusal> {
        match &self.window_end {
            Some(end) => TimedTrace::through(self.stamps.clone(), end.clone()),
            None => TimedTrace::new(self.stamps.clone()),
        }
    }

    /// The portion usable by FR-193 witness storage.
    pub fn behaviour(&self) -> SampledBehaviour {
        SampledBehaviour {
            trace: self.trace,
            start: self.start.clone(),
            steps: self.steps.clone(),
        }
    }
}

/// Every reward of the model for the step `transition` takes from `state`
/// (FR-187): empty for a model that declares none.
#[implements("FR-254-AC-5")]
fn step_rewards(
    system: &TimedSystem<'_, '_>,
    state: &TimedState,
    transition: &ModelTransition,
) -> Result<Vec<(Identifier, Value)>, TimedSampleStop> {
    let model = system.model_system();
    if model.zero_rewards().is_empty() {
        return Ok(Vec::new());
    }
    let actions = model
        .enabled_actions(state.discrete())
        .map_err(|stop| TimedRaceStop::Timed(TimedStop::Expansion(stop)))?
        .map_err(|_| TimedSampleStop::Step(Box::new(SampleStop::UndecidedSuccessor)))?;
    actions
        .iter()
        .flat_map(|action| &action.draws)
        .find(|draw| draw.successors.iter().any(|(taken, _)| taken == transition))
        .map(|draw| draw.rewards.clone())
        .ok_or_else(|| TimedRaceStop::StateKey.into())
}

/// One step of a tick subject from `state`: FR-408's scheduler draw and
/// random parameters over the identities the clocks enable, the reached
/// timed state and the time it advances. `None` at an idle tail.
#[implements("FR-254-AC-6")]
fn tick_step(
    system: &TimedSystem<'_, '_>,
    state: &TimedState,
    workload: &Workload,
    at: StepAt,
) -> Result<Option<(SampledStep, TimedState, Rational)>, TimedSampleStop> {
    let successors = system.steps(state).map_err(TimedRaceStop::Timed)?;
    let model = system.model_system();
    let mut actions = model
        .enabled_actions(state.discrete())
        .map_err(|stop| TimedRaceStop::Timed(TimedStop::Expansion(stop)))?
        .map_err(|_| TimedSampleStop::Step(Box::new(SampleStop::UndecidedSuccessor)))?;
    for action in &mut actions {
        for draw in &mut action.draws {
            draw.successors.retain(|(transition, _)| {
                successors
                    .iter()
                    .any(|(step, _)| step.transition() == transition)
            });
        }
    }
    actions.retain(|action| action.draws.iter().any(|draw| !draw.successors.is_empty()));
    if actions.is_empty() {
        return Ok(None);
    }
    let key = system
        .state_key(state)
        .map_err(|_| TimedRaceStop::StateKey)?;
    let taken = take_enabled_step(model, workload, &actions, &key, at)
        .map_err(|stop| TimedSampleStop::Step(Box::new(stop)))?;
    let TransitionId::Operation(transition) = &taken.transition else {
        return Err(TimedRaceStop::StateKey.into());
    };
    let (step, next) = successors
        .into_iter()
        .find(|(step, _)| step.transition() == transition)
        .ok_or(TimedRaceStop::TieDisabled)?;
    let advance = system.tick_of(step.operation());
    let post_state = system
        .state_key(&next)
        .map_err(|_| TimedRaceStop::StateKey)?;
    let sampled = SampledStep {
        transition: taken.transition,
        delay: Some(zero()),
        post_state,
        choices: taken.choices,
        random: taken.random,
        probability: taken.probability,
        rewards: taken.rewards,
    };
    Ok(Some((sampled, next, advance)))
}

struct Candidate {
    step: GuardedStep,
    delay: Rational,
    delay_probability: Rational,
    choices: Vec<u32>,
}

/// Sample and take the race at `state` (FR-253 SD-3, FR-254 SS-2).
///
/// Delays draw independently at consecutive choice indices. The workload
/// orders ties by weighted operation, then uniformly among that operation's
/// identities, without replacement.
pub fn sample_timed_race(
    system: &TimedSystem<'_, '_>,
    state: &TimedState,
    workload: &Workload,
    at: ChoicePoint,
    q: u32,
) -> Result<TimedRace, TimedRaceStop> {
    let windows = system
        .scheduled_windows(state)
        .map_err(TimedRaceStop::Timed)?;
    if windows.is_empty() {
        return Ok(TimedRace {
            steps: Vec::new(),
            state: state.clone(),
            probability: one(),
            idle: true,
            scheduler_choice: None,
        });
    }
    let key = system
        .state_key(state)
        .map_err(|_| TimedRaceStop::StateKey)?;
    let mut choice = at.choice;
    let mut candidates = Vec::new();
    let mut stochastic_draws = Vec::new();
    let mut free_delays = Vec::new();
    for window in windows {
        let Some(distribution) = &window.distribution else {
            free_delays.push(window);
            continue;
        };
        if matches!(distribution, DelayDistribution::Uniform)
            && window
                .intervals
                .iter()
                .any(|interval| interval.upper().is_none())
        {
            return Err(TimedRaceStop::NotStochastic {
                state: key,
                identity: window.transition.transition().clone(),
            });
        }
        let point = ChoicePoint { choice, ..at };
        let delay = match draw_timed_delay(distribution, &window.intervals, point, q) {
            Ok(delay) => delay,
            Err(TimedDelayRefusal::NoMass) => continue,
            Err(
                TimedDelayRefusal::UnboundedUniform
                | TimedDelayRefusal::GridExponent { .. }
                | TimedDelayRefusal::UnsampledWindow,
            ) => {
                return Err(TimedRaceStop::NotStochastic {
                    state: key,
                    identity: window.transition.transition().clone(),
                });
            }
        };
        let probability = delay_probability(distribution, &window.intervals, &delay, q);
        let consumes = delay_choice(distribution, &window.intervals, q);
        let choices = if consumes {
            let selected = choice;
            choice = choice.checked_add(1).ok_or(TimedRaceStop::DrawRange)?;
            vec![selected]
        } else {
            Vec::new()
        };
        let candidate = Candidate {
            step: window.transition,
            delay: delay.clone(),
            delay_probability: probability.clone(),
            choices: choices.clone(),
        };
        stochastic_draws.push(TimedRaceDraw {
            transition: candidate.step.transition().clone(),
            delay,
            probability,
            choices,
        });
        candidates.push(candidate);
    }
    if !free_delays.is_empty() {
        let probability = stochastic_draws
            .iter()
            .fold(one(), |product, draw| product.mul(&draw.probability));
        return Ok(TimedRace {
            steps: Vec::new(),
            state: state.clone(),
            probability: probability.clone(),
            idle: false,
            scheduler_choice: Some(TimedSchedulerChoice {
                state: key,
                draws: stochastic_draws,
                free_delays,
                probability,
            }),
        });
    }
    if candidates.is_empty() {
        return Ok(TimedRace {
            steps: Vec::new(),
            state: state.clone(),
            probability: one(),
            idle: true,
            scheduler_choice: None,
        });
    }
    let delay_mass = candidates_probability(&candidates);
    let least = candidates
        .iter()
        .map(|candidate| &candidate.delay)
        .min()
        .expect("a non-empty candidate list")
        .clone();
    let mut tied: Vec<Candidate> = candidates
        .into_iter()
        .filter(|candidate| candidate.delay == least)
        .collect();
    let delay = least;
    let mut ordered = Vec::new();
    let mut tie_factors = Vec::new();
    while !tied.is_empty() {
        let mut operations: Vec<(String, Rational, Vec<usize>)> = Vec::new();
        for (index, candidate) in tied.iter().enumerate() {
            let operation = candidate.step.operation().to_owned();
            if let Some((_, _, identities)) = operations
                .iter_mut()
                .find(|(text, _, _)| text == &operation)
            {
                identities.push(index);
                continue;
            }
            let weight = workload_weight(system, workload, &operation)?;
            operations.push((operation, weight, vec![index]));
        }
        let weights: Vec<_> = operations
            .iter()
            .map(|(_, weight, _)| weight.clone())
            .collect();
        let point = ChoicePoint { choice, ..at };
        let selected = choose(&weights, point)?;
        let (operation, operation_weight, identities) = &operations[selected.index];
        let operation_total = weights.iter().fold(zero(), |sum, weight| sum.add(weight));
        let op_probability = quotient(operation_weight, &operation_total);
        let identity_index = if identities.len() == 1 {
            0
        } else {
            let identity_weights = vec![one(); identities.len()];
            let point = ChoicePoint {
                choice: choice
                    .checked_add(u32::from(selected.consumed))
                    .ok_or(TimedRaceStop::DrawRange)?,
                ..at
            };
            choose(&identity_weights, point)?.index
        };
        let index = *identities
            .get(identity_index)
            .ok_or(TimedRaceStop::DrawRange)?;
        tie_factors.push(
            op_probability
                .div(&Rational::from_integer(Integer::from(
                    identities.len() as u64
                )))
                .expect("positive tie count"),
        );
        let mut candidate = tied.remove(index);
        if selected.consumed {
            candidate.choices.push(choice);
            choice = choice.checked_add(1).ok_or(TimedRaceStop::DrawRange)?;
        }
        if identities.len() > 1 {
            candidate.choices.push(choice);
            choice = choice.checked_add(1).ok_or(TimedRaceStop::DrawRange)?;
        }
        let _ = operation;
        ordered.push(candidate);
    }
    let mut current = state.clone();
    let mut steps = Vec::new();
    let mut remaining_probability = delay_mass;
    for (index, candidate) in ordered.into_iter().enumerate() {
        let after_delay = if index == 0 {
            system
                .delay(&current, &delay)
                .map_err(TimedRaceStop::Timed)?
        } else {
            Some(current.clone())
        }
        .ok_or(TimedRaceStop::TieDisabled)?;
        let reached = system
            .steps(&after_delay)
            .map_err(TimedRaceStop::Timed)?
            .into_iter()
            .find(|(step, _)| step.transition() == candidate.step.transition())
            .map(|(_, next)| next);
        let Some(reached) = reached else {
            continue;
        };
        let factor = if index == 0 {
            remaining_probability.mul(&tie_factors[index])
        } else {
            tie_factors[index].clone()
        };
        if index == 0 {
            remaining_probability = one();
        }
        steps.push(TimedRaceStep {
            transition: candidate.step.transition().clone(),
            delay: if index == 0 { delay.clone() } else { zero() },
            state: reached.clone(),
            probability: factor,
            choices: candidate.choices,
        });
        current = reached;
    }
    let probability = steps.iter().fold(one(), |product, step: &TimedRaceStep| {
        product.mul(&step.probability)
    });
    Ok(TimedRace {
        steps,
        state: current,
        probability,
        idle: false,
        scheduler_choice: None,
    })
}

struct Chosen {
    index: usize,
    consumed: bool,
}

fn choose(weights: &[Rational], at: ChoicePoint) -> Result<Chosen, TimedRaceStop> {
    if weights.len() == 1 {
        return Ok(Chosen {
            index: 0,
            consumed: false,
        });
    }
    let (index, _) = weighted_choice_counted(at, weights);
    Ok(Chosen {
        index: index.map_err(|_| TimedRaceStop::DrawRange)?,
        consumed: true,
    })
}

fn workload_weight(
    system: &TimedSystem<'_, '_>,
    workload: &Workload,
    operation: &str,
) -> Result<Rational, TimedRaceStop> {
    workload
        .weights()
        .iter()
        .find(|(candidate, _)| {
            system
                .system()
                .operation_text(candidate.declaring(), candidate.name())
                == Some(operation)
        })
        .map(|(_, weight)| weight.clone())
        .ok_or(TimedRaceStop::MissingWeight)
}

fn delay_choice(
    distribution: &DelayDistribution,
    intervals: &[super::DelayInterval],
    q: u32,
) -> bool {
    match distribution {
        DelayDistribution::Discrete(values) => {
            values
                .iter()
                .filter(|(delay, _)| intervals.iter().any(|window| window.contains(delay)))
                .count()
                > 1
        }
        DelayDistribution::Uniform | DelayDistribution::UniformRange { .. } => {
            q > 0
                && intervals.iter().any(|window| {
                    window
                        .upper()
                        .is_none_or(|(upper, _)| upper > window.lower().0)
                })
        }
        DelayDistribution::Exponential { .. } => q > 0,
    }
}

fn delay_probability(
    distribution: &DelayDistribution,
    intervals: &[super::DelayInterval],
    delay: &Rational,
    q: u32,
) -> Rational {
    match distribution {
        DelayDistribution::Discrete(values) => {
            let selected = values
                .iter()
                .find(|(value, _)| value == delay)
                .map(|(_, weight)| weight);
            let total = values
                .iter()
                .filter(|(value, _)| intervals.iter().any(|window| window.contains(value)))
                .fold(zero(), |sum, (_, weight)| sum.add(weight));
            selected.map_or_else(zero, |weight| quotient(weight, &total))
        }
        _ if !delay_choice(distribution, intervals, q) => one(),
        _ => Rational::from_integer(Integer::one()).divided_by_power_of_two(u64::from(q)),
    }
}

fn candidates_probability(candidates: &[Candidate]) -> Rational {
    candidates.iter().fold(one(), |product, candidate| {
        product.mul(&candidate.delay_probability)
    })
}

fn quotient(numerator: &Rational, denominator: &Rational) -> Rational {
    numerator.div(denominator).expect("positive workload total")
}

fn one() -> Rational {
    Rational::from_integer(Integer::one())
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}
