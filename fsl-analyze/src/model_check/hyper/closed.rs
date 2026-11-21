// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-173 "Event-position form" (ADR-018 TP-2, CX-1; ADR-023 HM-2, HM-3, HV-3):
//! EN-1's decision of an event-position HP-2 or HP-3 clause over tuples of
//! closed finite executions.
//!
//! A trace variable of such a clause ranges over the closed finite executions
//! of its subject: the paths from an initial state that end at a terminal
//! state or cover the body's ADR-014 TR-4 horizon, `h + 1` positions. No
//! trace is read as a lasso, a terminal state is not stutter-extended, and
//! the body's atoms over a component past its last position are false (FR-043).
//! The decision is by enumeration of the tuples, in the engine's canonical
//! order (instance, then each variable's executions in the subject's initial
//! state and successor-key order, the first variable slowest), so it is
//! exhaustive over the clause's whole scope:
//!
//! - HP-2: the clause holds when the body holds on every tuple that `μ_U`
//!   matches at every joint step all its components take, and is refuted by
//!   the first tuple on which the body is false, one closed trace per
//!   variable.
//! - HP-3: the clause is refuted by the first matched universal tuple that has
//!   no existential tuple, matched by `μ_E`, on which the body holds; the
//!   counterexample holds its universal traces alone and `trace_position` is
//!   the joint position at which the last existential candidate is lost.
//!
//! `fsl_replay::replay_hyper_trace_tuple` recomputes either verdict with the
//! same `fsl-eval` code.

use std::collections::BTreeSet;

use filament_exact::LimitKind;
use fsl_eval::hyper::{
    closed_executions, horizon, judge_closed_tuple, search_partner, ClosedExecution, ClosedInputs,
    ClosedStop, ClosedVerdict, ParameterBinding, PartnerSearch, Tuples,
};
use fsl_replay::{
    HyperCounterexample, HyperCounterexampleKind, HyperOutcome, HyperStep, HyperTrace,
    IncompleteCause, ModelCheckLimit, ModelStep, ReachedLimit, StepMark, UndecidedCause,
};

use super::{exhausted, fault, stopped, widen, Ending, HyperRefusal, Run, Totals};
use crate::model_check::ModelCheckRefusal;

/// What reading one tuple or searching one universal tuple ended with.
enum Reading {
    /// Nothing against the clause; the next tuple.
    Continue,
    /// The tuple is a counterexample at this position.
    Refuted(u64),
    /// The run ended with this outcome.
    Ended(HyperOutcome),
}

impl Run<'_, '_> {
    /// Decide the event-position HP-2 or HP-3 clause over `instances`.
    pub(super) fn decide_closed(
        &self,
        instances: &[Vec<ParameterBinding>],
        mut totals: Totals,
    ) -> Result<HyperOutcome, ModelCheckRefusal> {
        let clause = self.request.clause;
        let depth = horizon(clause.body()).ok_or_else(|| fault("a-body-has-a-horizon"))?;
        let limits = self.request.limits;
        let mut lists: Vec<Vec<ClosedExecution>> = Vec::with_capacity(self.systems.len());
        for (index, system) in self.systems.iter().enumerate() {
            let skips: BTreeSet<String> = clause
                .align_skip()
                .iter()
                .filter_map(|key| system.operation_text_of(key))
                .map(str::to_owned)
                .collect();
            match closed_executions(system, depth, limits.limits.max_states, &skips, self.cancel) {
                Ok(list) => {
                    totals.states = totals.states.saturating_add(widen(list.len()));
                    lists.push(list);
                }
                Err(stop) => return self.closed_stop(stop, index, &totals),
            }
        }
        totals.depth = depth;
        let universals = clause.universals();
        let mut matched = false;
        for instance in instances {
            let inputs = ClosedInputs {
                clause,
                instance,
                letters: self.request.letters,
                matcher: self.request.matcher,
                meter: self.request.meter,
                cancel: self.cancel,
            };
            let mut lens = Vec::with_capacity(universals);
            for slot in self.slots.iter().take(universals) {
                lens.push(lists.get(*slot).map_or(0, Vec::len));
            }
            for picked in Tuples::over(lens) {
                if self.cancel.poll() {
                    return self.cancelled(&totals);
                }
                totals.edges = totals.edges.saturating_add(1);
                if totals.edges > widen(limits.limits.max_transitions) {
                    return Ok(exhausted(
                        ModelCheckLimit::MaxTransitions,
                        widen(limits.limits.max_transitions),
                        &totals,
                    ));
                }
                let mut universal: Vec<&ClosedExecution> = Vec::with_capacity(universals);
                for (variable, index) in picked.iter().enumerate() {
                    let slot = self.slots.get(variable).copied();
                    universal.push(
                        slot.and_then(|slot| lists.get(slot))
                            .and_then(|list| list.get(*index))
                            .ok_or_else(|| fault("a-picked-execution-is-listed"))?,
                    );
                }
                let reading = if self.witness {
                    let existentials: Vec<&[ClosedExecution]> = self
                        .slots
                        .iter()
                        .skip(universals)
                        .map(|slot| lists.get(*slot).map_or(&[][..], Vec::as_slice))
                        .collect();
                    let searched =
                        search_partner(&inputs, &universal, &existentials, limits.max_witness_set);
                    self.partner_reading(searched, &mut matched, &totals)?
                } else {
                    let verdict = judge_closed_tuple(&inputs, &universal);
                    self.tuple_reading(verdict, &mut matched, &totals)?
                };
                match reading {
                    Reading::Continue => {}
                    Reading::Ended(outcome) => return Ok(outcome),
                    Reading::Refuted(position) => {
                        let counterexample = HyperCounterexample {
                            kind: HyperCounterexampleKind::Closed,
                            instance: Self::instance_of(instance),
                            traces: self.closed_traces(&universal)?,
                            trace_position: position,
                        };
                        return Ok(HyperOutcome::Violated {
                            counterexample: Box::new(counterexample),
                            stats: totals.stats(),
                        });
                    }
                }
            }
        }
        if !clause.mu_universal().is_empty() && !matched {
            return Ok(HyperOutcome::Undecided {
                cause: UndecidedCause::VacuousMatch,
                stats: totals.stats(),
            });
        }
        Ok(HyperOutcome::HoldsClosed {
            stats: totals.stats(),
        })
    }

    /// The reading of an HP-2 tuple's verdict.
    fn tuple_reading(
        &self,
        verdict: ClosedVerdict,
        matched: &mut bool,
        totals: &Totals,
    ) -> Result<Reading, ModelCheckRefusal> {
        Ok(match verdict {
            ClosedVerdict::UniversalUnmatched | ClosedVerdict::ExistentialUnmatched => {
                Reading::Continue
            }
            ClosedVerdict::Holds => {
                *matched = true;
                Reading::Continue
            }
            ClosedVerdict::Violated { at } => Reading::Refuted(at),
            stopped => self.stopped_reading(stopped, totals)?,
        })
    }

    /// The reading of an HP-3 universal tuple's search.
    fn partner_reading(
        &self,
        searched: PartnerSearch,
        matched: &mut bool,
        totals: &Totals,
    ) -> Result<Reading, ModelCheckRefusal> {
        Ok(match searched {
            PartnerSearch::UniversalUnmatched => Reading::Continue,
            PartnerSearch::Witnessed => {
                *matched = true;
                Reading::Continue
            }
            PartnerSearch::Exhausted { position } => Reading::Refuted(position),
            PartnerSearch::TooMany { limit } => Reading::Ended(stopped(
                IncompleteCause::ResourceExhausted,
                Some(ReachedLimit {
                    limit: ModelCheckLimit::MaxWitnessSet,
                    value: limit,
                }),
                None,
                totals,
            )),
            PartnerSearch::Stopped(verdict) => self.stopped_reading(verdict, totals)?,
            PartnerSearch::Cancelled => Reading::Ended(self.cancelled(totals)?),
        })
    }

    /// The reading of a verdict that settles no tuple.
    fn stopped_reading(
        &self,
        verdict: ClosedVerdict,
        totals: &Totals,
    ) -> Result<Reading, ModelCheckRefusal> {
        match verdict {
            ClosedVerdict::MatchUndetermined => Ok(Reading::Ended(HyperOutcome::Undecided {
                cause: UndecidedCause::MatchUndetermined,
                stats: totals.stats(),
            })),
            ClosedVerdict::Undefined => Err(HyperRefusal::ClosedUndefined.into()),
            ClosedVerdict::Incomplete(counter) => {
                if self.cancel.poll() {
                    return self.cancelled(totals).map(Reading::Ended);
                }
                Ok(Reading::Ended(self.meter_stop(counter, totals)))
            }
            ClosedVerdict::Fault(internal) => Err(ModelCheckRefusal::Fault(internal)),
            _ => {
                if self.cancel.poll() {
                    return self.cancelled(totals).map(Reading::Ended);
                }
                Err(fault("a-stopped-reading-is-not-a-verdict"))
            }
        }
    }

    fn meter_stop(&self, counter: LimitKind, totals: &Totals) -> HyperOutcome {
        exhausted(
            ModelCheckLimit::EvaluationMeter { counter },
            fsl_replay::meter_bound(&self.request.meter, counter),
            totals,
        )
    }

    /// The outcome a stopped enumeration of subject `index`'s executions
    /// settles.
    fn closed_stop(
        &self,
        stop: ClosedStop,
        index: usize,
        totals: &Totals,
    ) -> Result<HyperOutcome, ModelCheckRefusal> {
        match stop {
            ClosedStop::Cancelled => self.cancelled(totals),
            ClosedStop::Encoding(refusal) => Err(ModelCheckRefusal::Encoding(refusal)),
            ClosedStop::Executions { limit } => {
                Ok(exhausted(ModelCheckLimit::MaxStates, widen(limit), totals))
            }
            ClosedStop::Expansion(stop) => {
                let component = self.slots.iter().position(|slot| *slot == index);
                Ok(match self.expansion_ending(component, stop)? {
                    Ending::Stopped(cause, limit, count) => stopped(cause, limit, count, totals),
                    Ending::Complete => return Err(fault("a-stopped-expansion-stops")),
                })
            }
        }
    }

    /// One trace per variable of `executions`, in quantifier order.
    fn closed_traces(
        &self,
        executions: &[&ClosedExecution],
    ) -> Result<Vec<HyperTrace>, ModelCheckRefusal> {
        let mut traces = Vec::with_capacity(executions.len());
        for ((variable, alias), execution) in self.variables().into_iter().zip(executions) {
            traces.push(HyperTrace {
                variable,
                alias,
                initial: u32::try_from(execution.initial)
                    .map_err(|_| fault("an-initial-index-fits-u32"))?,
                steps: execution
                    .steps
                    .iter()
                    .map(|step| {
                        HyperStep::Model(ModelStep {
                            transition: step.transition.clone(),
                            post_state: step.post_state.clone(),
                            delay: None,
                        })
                    })
                    .collect(),
                results: Vec::new(),
                marks: if self.request.clause.align_skip().is_empty() {
                    Vec::new()
                } else {
                    execution
                        .steps
                        .iter()
                        .map(|step| {
                            if step.skipped {
                                StepMark::Skipped
                            } else {
                                StepMark::Visible
                            }
                        })
                        .collect()
                },
            });
        }
        Ok(traces)
    }
}
