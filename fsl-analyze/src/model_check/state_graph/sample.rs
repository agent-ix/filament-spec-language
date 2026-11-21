// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-166 (ADR-022 GE-2): phase 0, seeded witness walks for `possible`
//! items before exploration.
//!
//! Walk `r` from initial state `i` is FR-101's sampled trace with the run's
//! seed and trace index `i × witness_samples + r`, over the view of the
//! subject whose only initial state is `i`, taking at most the lesser of
//! `max_walk_steps` and the horizon. A walk is a function of the subject,
//! the seed and its trace index alone, so one walk serves every instance
//! that still has no witness from `i`: the target of each is read at every
//! state the walk visits, position 0 included, and the walk ends at the
//! first state by which every one of them has found its target, or an
//! undefined or refused reading of it. Each instance keeps the witness of
//! the lowest walk index that found one and reads no later walk.

use std::collections::BTreeMap;

use filament_exact::{
    length_amount, Cancel, Charge, ChargePoint, Meter, ObjectReference, ScalarLimits,
};
use fsl_eval::simulation::model::{
    sample_model_walk_keyed, ModelSystem, ModelWalkSettings, PredicateReading, WalkMemo,
};
use fsl_eval::simulation::{
    NotSimulated, SampleProvenance, StateKey, StopReason, TransitionSystem,
};
use fsl_foundation::selection::DefinitionRef;
use fsl_replay::{
    GraphWitness, ModelCheckLimit, ModelCheckLimits, ModelPath, ModelStep, WitnessSource,
};
use fsl_semantics::check::{CheckedStateGraphClaim, PredicateRole};

use super::{
    budget_is_ceiling, cancelled, expansion_stop, fault, sampler, step_ceiling, stopped,
    trace_index, trace_indices_fit, Budgets, Halt, InstanceView,
};
use crate::model_check::check::BOUND_POSITIONS;
use crate::model_check::ModelCheckRefusal;

/// An instance's identity within the request: item and instance index.
type InstanceKey = (usize, usize);

/// One instance's target as a walk reads it.
type Target<'a> = (
    &'a CheckedStateGraphClaim,
    PredicateRole,
    Option<&'a ObjectReference>,
);

/// Stable truth readings only. Target indices identify the fixed claim,
/// role and binding in this phase; positions and witness paths are never
/// retained. Storage admission precedes each retained key allocation, and
/// a full memo simply leaves subsequent readings to the evaluator.
struct TruthMemo {
    values: BTreeMap<StateKey, BTreeMap<usize, bool>>,
    storage: Meter,
}

impl TruthMemo {
    fn new(limits: ScalarLimits, cancel: &Cancel) -> Self {
        Self {
            values: BTreeMap::new(),
            storage: Meter::new(limits).with_cancel(cancel.clone()),
        }
    }

    fn get(
        &self,
        key: &StateKey,
        target: usize,
    ) -> Result<Option<bool>, filament_exact::Incomplete> {
        let truth = self
            .values
            .get(key)
            .and_then(|values| values.get(&target))
            .copied();
        if truth.is_some() {
            let mut meter = Meter::new(*self.storage.limits());
            if let Some(cancel) = self.storage.cancel() {
                meter = meter.with_cancel(cancel.clone());
            }
            meter.charge(Charge::new(ChargePoint::GraphEdge))?;
        }
        Ok(truth)
    }

    fn retain(&mut self, key: &StateKey, target: usize, reading: &PredicateReading) {
        let truth = match reading {
            PredicateReading::True => true,
            PredicateReading::False => false,
            PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => return,
        };
        let retained = length_amount(key.as_bytes().len())
            .saturating_add(length_amount(std::mem::size_of::<(StateKey, usize, bool)>()))
            .saturating_add(128);
        if self
            .storage
            .charge(Charge::new(ChargePoint::GraphResultRetain).results(retained))
            .is_ok()
        {
            if let Some(values) = self.values.get_mut(key) {
                values.insert(target, truth);
            } else {
                self.values
                    .insert(key.clone(), BTreeMap::from([(target, truth)]));
            }
        }
    }
}

/// What phase 0 found: per `possible` instance, each initial state's sampled
/// witness, and the count of its walks `max_walk_steps` stopped.
#[derive(Default)]
pub(super) struct Sampled {
    witnesses: BTreeMap<InstanceKey, Vec<Option<GraphWitness>>>,
    stopped: BTreeMap<InstanceKey, u64>,
}

impl Sampled {
    /// `view`'s sampled witness per initial state; empty for an instance
    /// phase 0 drew no walk for.
    pub(super) fn witnesses(&self, view: &InstanceView<'_>) -> &[Option<GraphWitness>] {
        self.witnesses
            .get(&(view.item, view.instance))
            .map_or(&[], Vec::as_slice)
    }

    /// The count of `view`'s walks `max_walk_steps` stopped.
    pub(super) fn stopped(&self, view: &InstanceView<'_>) -> u64 {
        self.stopped
            .get(&(view.item, view.instance))
            .copied()
            .unwrap_or(0)
    }
}

/// What one walk found for one instance: the position of the first target
/// state, or a position where the target had no truth value.
#[derive(Clone, Copy)]
enum Found {
    Target(usize),
    Unreadable,
}

/// One walk: what it found for each instance it read, its steps, whether
/// `max_walk_steps` ended it, and its provenance.
struct Walk {
    found: Vec<Option<Found>>,
    steps: Vec<ModelStep>,
    budget_stopped: bool,
    provenance: SampleProvenance,
}

/// Phase 0 of one run.
pub(super) struct Phase<'r, 'a> {
    pub(super) system: &'r ModelSystem<'a>,
    pub(super) limits: &'r ModelCheckLimits,
    pub(super) horizon: Option<usize>,
    pub(super) seed: u64,
    pub(super) cancel: &'r Cancel,
    pub(super) budgets: Budgets,
    /// The expansions phase 0's walks share: each state is expanded once.
    pub(super) walks: WalkMemo,
}

impl Phase<'_, '_> {
    /// Run phase 0 for `instances`, the `possible` item instances of the
    /// request: `Ok(Err(halt))` when a cancellation, a meter or an
    /// overflowing trace index stopped it, before the first walk in the last
    /// case.
    pub(super) fn run(
        &self,
        instances: &[&InstanceView<'_>],
    ) -> Result<Result<Sampled, Halt>, ModelCheckRefusal> {
        let samples = self.limits.witness_samples;
        let mut sampled = Sampled::default();
        if samples == 0 || instances.is_empty() {
            return Ok(Ok(sampled));
        }
        let count = self.system.initial().len();
        // The greatest trace index, with checked arithmetic, before any walk.
        if !trace_indices_fit(count, samples) {
            return Ok(Err(stopped(ModelCheckLimit::WitnessSamples, samples)));
        }
        let sampler = sampler()?;
        let mut truths = TruthMemo::new(self.budgets.meter, self.cancel);
        let mut targets: Vec<Target<'_>> = Vec::with_capacity(instances.len());
        for view in instances {
            let Some((_, Some(target))) = view
                .predicates
                .iter()
                .find(|(role, _)| *role == PredicateRole::Target)
            else {
                return Err(fault("possible-has-a-target"));
            };
            targets.push((*target, PredicateRole::Target, view.over.as_ref()));
            sampled
                .witnesses
                .insert((view.item, view.instance), vec![None; count]);
            sampled.stopped.insert((view.item, view.instance), 0);
        }
        for initial in 0..count {
            let mut pending: Vec<usize> = (0..instances.len()).collect();
            let mut walk_index = 0;
            while walk_index < samples && !pending.is_empty() {
                if let Some(halt) = cancelled(self.cancel) {
                    return Ok(Err(halt));
                }
                let asked: Vec<Target<'_>> = pending.iter().map(|index| targets[*index]).collect();
                let walk = match self.walk(
                    &sampler,
                    initial,
                    trace_index(initial, samples, walk_index)?,
                    &asked,
                    &pending,
                    &mut truths,
                )? {
                    Ok(walk) => walk,
                    Err(halt) => return Ok(Err(halt)),
                };
                let mut still = Vec::with_capacity(pending.len());
                for (index, found) in pending.into_iter().zip(&walk.found) {
                    let view = instances[index];
                    let key = (view.item, view.instance);
                    match found {
                        Some(Found::Target(position)) => {
                            let path = ModelPath {
                                initial: u32::try_from(initial)
                                    .map_err(|_| fault("initial-index-fits-u32"))?,
                                steps: walk.steps.iter().take(*position).cloned().collect(),
                            };
                            if let Some(slot) = sampled
                                .witnesses
                                .get_mut(&key)
                                .and_then(|slots| slots.get_mut(initial))
                            {
                                // The walk this instance would draw alone
                                // ends at its target: `Reached`, whatever
                                // ended the shared walk.
                                *slot = Some(GraphWitness {
                                    path,
                                    source: WitnessSource::Sampled(SampleProvenance {
                                        stopped: StopReason::Reached,
                                        ..walk.provenance.clone()
                                    }),
                                });
                            }
                        }
                        Some(Found::Unreadable) => still.push(index),
                        None => {
                            if walk.budget_stopped {
                                if let Some(count) = sampled.stopped.get_mut(&key) {
                                    *count = count.saturating_add(1);
                                }
                            }
                            still.push(index);
                        }
                    }
                }
                pending = still;
                walk_index += 1;
            }
        }
        Ok(Ok(sampled))
    }

    /// Walk `trace` from `initial`, reading each of `targets` at every state
    /// it visits until each has found its target or an unreadable value.
    fn walk(
        &self,
        sampler: &DefinitionRef,
        initial: usize,
        trace: u64,
        targets: &[Target<'_>],
        target_indices: &[usize],
        truths: &mut TruthMemo,
    ) -> Result<Result<Walk, Halt>, ModelCheckRefusal> {
        let mut found: Vec<Option<Found>> = vec![None; targets.len()];
        let mut halt: Option<Result<Halt, ModelCheckRefusal>> = None;
        let mut position = 0;
        let walked = sample_model_walk_keyed(
            self.system,
            initial,
            ModelWalkSettings {
                position_limit: BOUND_POSITIONS,
                sampler,
                seed: self.seed,
                trace,
                max_steps: step_ceiling(self.limits, self.horizon),
                cancel: Some(self.cancel),
            },
            &self.walks,
            |state, key| {
                let here = position;
                position += 1;
                if let Some(cancel) = cancelled(self.cancel) {
                    halt = Some(Ok(cancel));
                    return true;
                }
                let open: Vec<usize> = (0..targets.len())
                    .filter(|index| found[*index].is_none())
                    .collect();
                let mut known = Vec::with_capacity(open.len());
                let mut missing = Vec::new();
                for index in &open {
                    let reading = match truths.get(key, target_indices[*index]) {
                        Ok(reading) => reading,
                        Err(incomplete) => {
                            halt = Some(Ok(cancelled(self.cancel).unwrap_or_else(|| {
                                stopped(
                                    ModelCheckLimit::EvaluationMeter {
                                        counter: incomplete.limit_kind,
                                    },
                                    fsl_replay::meter_bound(
                                        &self.budgets.meter,
                                        incomplete.limit_kind,
                                    ),
                                )
                            })));
                            return true;
                        }
                    };
                    if reading.is_none() {
                        missing.push(*index);
                    }
                    known.push(reading);
                }
                let asked: Vec<Target<'_>> = missing.iter().map(|index| targets[*index]).collect();
                let evaluated = if asked.is_empty() {
                    Ok(Vec::new())
                } else {
                    self.system.state_graph_predicates(state, &asked)
                };
                let mut readings = match evaluated {
                    Ok(readings) => readings,
                    Err(stop) => {
                        halt = Some(expansion_stop(&stop, self.budgets));
                        return true;
                    }
                }
                .into_iter();
                for (index, truth) in open.into_iter().zip(known) {
                    let reading = if let Some(truth) = truth {
                        if truth {
                            PredicateReading::True
                        } else {
                            PredicateReading::False
                        }
                    } else {
                        let Some(reading) = readings.next() else {
                            halt = Some(Err(fault("one-reading-per-target")));
                            return true;
                        };
                        truths.retain(key, target_indices[index], &reading);
                        reading
                    };
                    found[index] = match reading {
                        PredicateReading::True => Some(Found::Target(here)),
                        PredicateReading::False => None,
                        PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => {
                            Some(Found::Unreadable)
                        }
                    };
                }
                found.iter().all(Option::is_some)
            },
        );
        if let Some(halt) = halt {
            return halt.map(Err);
        }
        let trace = match walked {
            Ok(trace) => trace,
            Err(NotSimulated::KeyEncoding(refusal)) => {
                return Err(ModelCheckRefusal::Encoding(refusal))
            }
            Err(_) => return Err(fault("phase-zero-walk-samples")),
        };
        let provenance = trace
            .provenance
            .ok_or_else(|| fault("sampled-trace-has-provenance"))?;
        let budget_stopped = budget_is_ceiling(self.limits, self.horizon)
            && provenance.stopped == StopReason::StepLimit;
        let steps = trace
            .steps
            .into_iter()
            .map(|step| ModelStep {
                transition: step.transition,
                post_state: step.key,
                delay: None,
            })
            .collect();
        Ok(Ok(Walk {
            found,
            steps,
            budget_stopped,
            provenance,
        }))
    }
}

#[cfg(test)]
mod tests {
    use filament_exact::{CancelCause, LimitKind};
    use fsl_eval::simulation::model::ClauseOutcome;
    use ix_trace_rs::trace;

    use super::*;

    fn limits() -> ScalarLimits {
        ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        }
    }

    /// A cached truth belongs only to its state and target instance.
    #[trace("FR-166-AC-1")]
    #[test]
    fn truth_memo_separates_states_and_target_instances() {
        let mut memo = TruthMemo::new(limits(), &Cancel::new());
        let zero = StateKey::from_canonical_bytes(b"0".to_vec());
        let one = StateKey::from_canonical_bytes(b"1".to_vec());
        memo.retain(&zero, 0, &PredicateReading::False);
        memo.retain(&zero, 1, &PredicateReading::True);
        memo.retain(&one, 0, &PredicateReading::True);
        assert_eq!(memo.get(&zero, 0), Ok(Some(false)));
        assert_eq!(memo.get(&zero, 1), Ok(Some(true)));
        assert_eq!(memo.get(&one, 0), Ok(Some(true)));
        assert_eq!(memo.get(&one, 1), Ok(None));
    }

    /// An unreadable result is evaluated again, never reused as truth.
    #[trace("FR-166-AC-1", "FR-166-AC-5")]
    #[test]
    fn truth_memo_does_not_retain_unreadable_results() {
        let mut memo = TruthMemo::new(limits(), &Cancel::new());
        let key = StateKey::from_canonical_bytes(b"0".to_vec());
        memo.retain(
            &key,
            0,
            &PredicateReading::Refused(ClauseOutcome::NotBoolean),
        );
        memo.retain(
            &key,
            1,
            &PredicateReading::Undefined {
                cause: fsl_eval::temporal::UndefinedCause::Family(
                    filament_semantic_value::catalog::UndefinedRecord {
                        reason: filament_semantic_value::catalog::UndefinedReason::AbsentKey,
                        fields: BTreeMap::new(),
                    },
                ),
                location: None,
            },
        );
        assert_eq!(memo.get(&key, 0), Ok(None));
        assert_eq!(memo.get(&key, 1), Ok(None));
        assert!(memo.values.is_empty());
        assert_eq!(memo.storage.consumed(LimitKind::ResultUnits), 0);
    }

    /// Retention denial leaves no allocated cache entry and permits fallback.
    #[trace("FR-166-AC-1", "FR-166-AC-4")]
    #[test]
    fn truth_memo_admits_storage_before_retaining_keys() {
        let mut memo = TruthMemo::new(
            ScalarLimits {
                result_units: 0,
                ..limits()
            },
            &Cancel::new(),
        );
        let key = StateKey::from_canonical_bytes(b"0".to_vec());
        memo.retain(&key, 0, &PredicateReading::False);
        assert_eq!(memo.get(&key, 0), Ok(None));
        assert!(memo.values.is_empty());
        assert_eq!(memo.storage.consumed(LimitKind::ResultUnits), 0);
    }

    /// Cache hits continue to poll cancellation and admit work.
    #[trace("FR-166-AC-4")]
    #[test]
    fn truth_memo_hits_admit_work_and_observe_cancellation() {
        let cancel = Cancel::new();
        let mut memo = TruthMemo::new(limits(), &cancel);
        let key = StateKey::from_canonical_bytes(b"0".to_vec());
        memo.retain(&key, 0, &PredicateReading::False);
        assert_eq!(memo.get(&key, 0), Ok(Some(false)));
        cancel.cancel(CancelCause::Requested);
        assert_eq!(
            memo.get(&key, 0).expect_err("cancelled hit").limit_kind,
            LimitKind::WorkUnits
        );

        let mut blocked = TruthMemo::new(
            ScalarLimits {
                work_units: 0,
                ..limits()
            },
            &Cancel::new(),
        );
        blocked
            .values
            .insert(key.clone(), BTreeMap::from([(0, false)]));
        assert_eq!(
            blocked
                .get(&key, 0)
                .expect_err("hit work admission")
                .limit_kind,
            LimitKind::WorkUnits
        );
    }
}
