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

use std::collections::{BTreeMap, BTreeSet};

use filament_exact::{
    Cancel, Charge, ChargePoint, Meter, ObjectReference, ScalarLimits, length_amount,
};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{
    ModelSystem, ModelWalkSettings, PredicateReading, WalkMemo, sample_model_walk_observed_keyed,
};
use fsl_eval::simulation::{
    NotSimulated, ObservedSample, ObserverDecision, SampleExpansion, SampleObservation,
    SampleProvenance, StateKey, StopReason, TransitionSystem,
};
use fsl_foundation::selection::DefinitionRef;
use fsl_replay::{
    GraphWitness, ModelCheckLimit, ModelCheckLimits, ModelPath, ModelStep, WitnessSource,
};
use fsl_semantics::check::{CheckedStateGraphClaim, PredicateRole};

use super::{
    Budgets, Halt, InstanceView, budget_is_ceiling, cancelled, expansion_stop, fault, sampler,
    step_ceiling, stopped, trace_index, trace_indices_fit,
};
use crate::model_check::ModelCheckRefusal;
use crate::model_check::check::BOUND_POSITIONS;

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
    provenance: Option<SampleProvenance>,
}

/// Incrementally proves that a sampled walk's observed subgraph is closed.
/// Edge identity is the source key, transition identity and destination key.
struct ObservedClosure<T> {
    visited: BTreeSet<StateKey>,
    outgoing: BTreeMap<StateKey, Vec<(T, StateKey)>>,
    taken: BTreeMap<StateKey, Vec<(T, StateKey)>>,
    previous: Option<StateKey>,
}

impl<T> Default for ObservedClosure<T> {
    fn default() -> Self {
        Self {
            visited: BTreeSet::new(),
            outgoing: BTreeMap::new(),
            taken: BTreeMap::new(),
            previous: None,
        }
    }
}

impl<T: Clone + Eq> ObservedClosure<T> {
    fn observe<S, F>(&mut self, observation: &SampleObservation<'_, S, T, F>) -> bool {
        if let (Some(source), Some(incoming)) = (&self.previous, observation.incoming) {
            let edge = (incoming.transition.clone(), incoming.key.clone());
            let edges = self.taken.entry(source.clone()).or_default();
            if !edges.contains(&edge) {
                edges.push(edge);
            }
        }
        let repeated = !self.visited.insert(observation.key.clone());
        let SampleExpansion::Complete { successors, .. } = &observation.expansion else {
            self.previous = Some(observation.key.clone());
            return false;
        };
        let edges = successors
            .iter()
            .map(|successor| (successor.transition.clone(), successor.key.clone()))
            .collect();
        self.outgoing
            .entry(observation.key.clone())
            .or_insert(edges);
        self.previous = Some(observation.key.clone());
        repeated
            && self.outgoing.len() == self.visited.len()
            && self.outgoing.iter().all(|(source, edges)| {
                edges.iter().all(|(transition, destination)| {
                    self.visited.contains(destination)
                        && self.taken.get(source).is_some_and(|taken| {
                            taken.contains(&(transition.clone(), destination.clone()))
                        })
                })
            })
    }
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
                                        ..walk
                                            .provenance
                                            .clone()
                                            .ok_or_else(|| fault("target-walk-has-provenance"))?
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
        let mut closure = ObservedClosure::<ModelTransition>::default();
        let walked = sample_model_walk_observed_keyed(
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
            |observation| {
                let state = observation.state;
                let key = observation.key;
                let here = position;
                position += 1;
                if let Some(cancel) = cancelled(self.cancel) {
                    halt = Some(Ok(cancel));
                    return ObserverDecision::Stop;
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
                            return ObserverDecision::Stop;
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
                        return ObserverDecision::Stop;
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
                            return ObserverDecision::Stop;
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
                if found.iter().all(Option::is_some) {
                    return ObserverDecision::Stop;
                }
                if closure.observe(observation) {
                    return ObserverDecision::Stop;
                }
                ObserverDecision::Continue
            },
        );
        if let Some(halt) = halt {
            return halt.map(Err);
        }
        let (trace_steps, provenance) = match walked {
            Ok(ObservedSample::Completed(trace)) => {
                let provenance = trace
                    .provenance
                    .ok_or_else(|| fault("sampled-trace-has-provenance"))?;
                (trace.steps, Some(provenance))
            }
            Ok(ObservedSample::ObserverStopped(prefix)) => {
                let provenance = found
                    .iter()
                    .any(|found| matches!(found, Some(Found::Target(_))))
                    .then(|| SampleProvenance {
                        seed: self.seed,
                        trace,
                        sampler: sampler.clone(),
                        stopped: StopReason::Reached,
                    });
                (prefix.steps, provenance)
            }
            Err(NotSimulated::KeyEncoding(refusal)) => {
                return Err(ModelCheckRefusal::Encoding(refusal));
            }
            Err(_) => return Err(fault("phase-zero-walk-samples")),
        };
        let budget_stopped = provenance.as_ref().is_some_and(|provenance| {
            budget_is_ceiling(self.limits, self.horizon)
                && provenance.stopped == StopReason::StepLimit
        });
        let steps: Vec<ModelStep> = trace_steps
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
    use filament_exact::{NodeKey, ValueType};
    use filament_semantic_value::catalog::CatalogCode;
    use filament_semantic_value::declaration::TypeEnvironment;
    use fsl_eval::simulation::model::ClauseOutcome;
    use fsl_eval::simulation::{
        Expansion, ExpansionStop, SampleRequest, sample_request, sample_request_observed,
    };
    use fsl_replay::{DEFAULT_WITNESS_SAMPLES, DEFAULT_WITNESS_SEED};
    use ix_trace_rs::trace;
    use serde::Serialize;

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

    #[derive(Serialize)]
    struct Graph(Vec<Vec<(usize, usize)>>);

    impl TransitionSystem for Graph {
        type State = usize;
        type TransitionId = usize;
        type Key = usize;
        type Finding = ();

        fn initial(&self) -> Vec<Self::State> {
            vec![0]
        }

        fn key(&self, state: &Self::State) -> Self::Key {
            *state
        }

        fn successors(
            &self,
            state: &Self::State,
        ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop>
        {
            Ok(Expansion {
                successors: self.0[*state].to_vec(),
                findings: Vec::new(),
            })
        }
    }

    struct StopsAtOne;

    impl TransitionSystem for StopsAtOne {
        type State = usize;
        type TransitionId = usize;
        type Key = usize;
        type Finding = &'static str;

        fn initial(&self) -> Vec<Self::State> {
            vec![0]
        }

        fn key(&self, state: &Self::State) -> Self::Key {
            *state
        }

        fn successors(
            &self,
            state: &Self::State,
        ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop>
        {
            if *state == 1 {
                return Err(ExpansionStop {
                    cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
                    limit: None,
                });
            }
            Ok(Expansion {
                successors: vec![(17, 1)],
                findings: vec!["expanded-zero"],
            })
        }

        fn state_findings(
            &self,
            state: &Self::State,
        ) -> Option<Result<Vec<Self::Finding>, ExpansionStop>> {
            Some(Ok(if *state == 1 {
                vec!["stopped-one"]
            } else {
                vec!["state-zero"]
            }))
        }
    }

    fn closed_walk_steps(graph: &Graph, trace: u64, ceiling: usize) -> usize {
        let sampler =
            DefinitionRef::new("filament", "filament.simulation.sampler/v1").expect("sampler");
        let domains: &[(NodeKey, &ValueType)] = &[];
        let types = TypeEnvironment::default();
        let mut closure = ObservedClosure::<usize>::default();
        let result = sample_request_observed(
            graph,
            SampleRequest {
                domains,
                types: &types,
                position_limit: BOUND_POSITIONS,
                sampler: &sampler,
                seed: DEFAULT_WITNESS_SEED,
                trace,
                max_steps: ceiling,
                identity: fsl_foundation::IdentityLimits::default(),
            },
            |observation| {
                if closure.observe(observation) {
                    ObserverDecision::Stop
                } else {
                    ObserverDecision::Continue
                }
            },
        )
        .expect("sample request is bounded");
        let ObservedSample::ObserverStopped(prefix) = result else {
            panic!("walk did not prove closure before its ceiling");
        };
        prefix.steps.len()
    }

    /// FR-101-AC-23: an observer that continues preserves the ordinary trace,
    /// including its findings and sampler provenance.
    #[trace("FR-101-AC-23")]
    #[test]
    fn an_observer_that_continues_preserves_the_sampled_trace() {
        let graph = Graph(vec![
            vec![(1, 1), (2, 2)],
            vec![(0, 0), (2, 2), (3, 3)],
            vec![(0, 0), (1, 1), (3, 3)],
            vec![(0, 0), (2, 2)],
        ]);
        let sampler =
            DefinitionRef::new("filament", "filament.simulation.sampler/v1").expect("sampler");
        let domains: &[(NodeKey, &ValueType)] = &[];
        let types = TypeEnvironment::default();
        let request = || SampleRequest {
            domains,
            types: &types,
            position_limit: BOUND_POSITIONS,
            sampler: &sampler,
            seed: DEFAULT_WITNESS_SEED,
            trace: 19,
            max_steps: 32,
            identity: fsl_foundation::IdentityLimits::default(),
        };
        let ordinary = sample_request(
            &graph,
            domains,
            &types,
            BOUND_POSITIONS,
            &sampler,
            DEFAULT_WITNESS_SEED,
            19,
            32,
            fsl_foundation::IdentityLimits::default(),
        )
        .expect("ordinary trace");
        let observed = sample_request_observed(&graph, request(), |_| ObserverDecision::Continue)
            .expect("observed trace");
        let ObservedSample::Completed(observed) = observed else {
            panic!("always-continue observer must complete");
        };
        assert_eq!(observed, ordinary);
    }

    /// FR-101-AC-23: observers receive a stopped state's cause and can stop
    /// with a prefix that does not become a completed trace.
    #[trace("FR-101-AC-23")]
    #[test]
    fn observer_sees_stopped_expansions_before_the_sample_ends() {
        let sampler =
            DefinitionRef::new("filament", "filament.simulation.sampler/v1").expect("sampler");
        let domains: &[(NodeKey, &ValueType)] = &[];
        let types = TypeEnvironment::default();
        let mut saw_stop = false;
        let mut observations = 0;
        let observed = sample_request_observed(
            &StopsAtOne,
            SampleRequest {
                domains,
                types: &types,
                position_limit: BOUND_POSITIONS,
                sampler: &sampler,
                seed: DEFAULT_WITNESS_SEED,
                trace: 0,
                max_steps: 4,
                identity: fsl_foundation::IdentityLimits::default(),
            },
            |observation| {
                observations += 1;
                match &observation.expansion {
                    SampleExpansion::Stopped(stop) => {
                        saw_stop = true;
                        assert_eq!(observation.step, 1);
                        assert_eq!(*observation.state, 1);
                        let incoming = observation.incoming.expect("one edge reaches state one");
                        assert_eq!(incoming.transition, 17);
                        assert_eq!(incoming.ordinal, 0);
                        assert_eq!(incoming.key.as_bytes(), b"1");
                        assert_eq!(
                            stop.cause,
                            CatalogCode::new("resource_exhausted", "insufficient-next-charge")
                        );
                        ObserverDecision::Stop
                    }
                    SampleExpansion::Complete { .. } => ObserverDecision::Continue,
                }
            },
        )
        .expect("sample request is bounded");
        let ObservedSample::ObserverStopped(prefix) = observed else {
            panic!("observer stop must return a prefix");
        };
        assert!(saw_stop);
        let initial = StateKey::from_canonical_bytes(b"0".to_vec());
        let stopped = StateKey::from_canonical_bytes(b"1".to_vec());
        assert_eq!(observations, 2, "the observer stopped before another draw");
        assert_eq!(prefix.initial, initial);
        assert_eq!(
            prefix.steps,
            vec![fsl_eval::simulation::Step {
                transition: 17,
                ordinal: 0,
                key: stopped.clone(),
            }]
        );
        assert_eq!(
            prefix.findings,
            vec![
                fsl_eval::simulation::StateFindings {
                    state: initial,
                    depth: 0,
                    findings: vec!["expanded-zero"],
                },
                fsl_eval::simulation::StateFindings {
                    state: stopped,
                    depth: 1,
                    findings: vec!["stopped-one"],
                },
            ]
        );
    }

    /// FR-166-AC-2: each seeded walk closes the complete 9-state/18-edge
    /// worked graph within 512 transitions; record and bound the total work.
    #[trace("FR-166-AC-2", "FR-166-AC-7")]
    #[test]
    fn observed_closure_bounds_the_nine_state_worked_graph() {
        let mut edges = vec![Vec::new(); 9];
        for version_a in 0..3 {
            for version_b in 0..3 {
                let state = version_a * 3 + version_b;
                edges[state] = vec![
                    (0, ((version_a + 1) % 3) * 3 + version_b),
                    (1, version_a * 3 + (version_b + 1) % 3),
                ];
            }
        }
        let graph = Graph(edges);
        assert_eq!(graph.0.len(), 9);
        assert_eq!(graph.0.iter().map(Vec::len).sum::<usize>(), 18);
        let steps: Vec<usize> = (0..DEFAULT_WITNESS_SAMPLES)
            .map(|trace| closed_walk_steps(&graph, trace, 512))
            .collect();
        assert_eq!(steps.len(), DEFAULT_WITNESS_SAMPLES as usize);
        assert!(steps.iter().all(|steps| *steps <= 512), "{steps:?}");
        let total: usize = steps.iter().sum();
        assert!(total <= 64 * 512, "total sampled steps: {total}");
        assert_eq!(total, 3450, "total sampled steps: {steps:?}");
    }

    /// FR-166-AC-8: a seeded walk closes only after all outgoing edges of the
    /// visited subgraph have been taken, within the published 256-step cap.
    #[trace("FR-166-AC-8")]
    #[test]
    fn observed_closure_bounds_the_four_state_ten_edge_graph() {
        let graph = Graph(vec![
            vec![(1, 1), (2, 2)],
            vec![(0, 0), (2, 2), (3, 3)],
            vec![(0, 0), (1, 1), (3, 3)],
            vec![(0, 0), (2, 2)],
        ]);
        assert_eq!(graph.0.len(), 4);
        assert_eq!(graph.0.iter().map(Vec::len).sum::<usize>(), 10);
        let steps: Vec<usize> = (0..DEFAULT_WITNESS_SAMPLES)
            .map(|trace| closed_walk_steps(&graph, trace, 256))
            .collect();
        assert_eq!(steps.len(), DEFAULT_WITNESS_SAMPLES as usize);
        assert!(steps.iter().all(|steps| *steps <= 256), "{steps:?}");
        let total: usize = steps.iter().sum();
        assert!(total <= 64 * 256, "total sampled steps: {total}");
        assert_eq!(total, 1892, "total sampled steps: {steps:?}");
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
