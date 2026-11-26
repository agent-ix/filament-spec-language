// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-189 to FR-193 (TC-624, TC-625, TC-627, TC-628, TC-640): EN-4's
//! `check_statistical` over ADR-024 §7's models (see [`crate::prob_world`]):
//! FR-188's behaviours drawn from the subject, each read by the layer-5
//! evaluator.

use std::sync::{Arc, Barrier, Condvar, Mutex};

use filament_checked::key::DeclarationKey;
use filament_exact::{Cancel, Integer, Rational, Value};
use filament_semantic_value::catalog::UndefinedReason;
use fsl_analyze::statistical::{
    check_sampled, check_statistical, witness_reader, witness_reader_with_timed_sampling,
    BehaviourSampler, Drawn, QuantileComparison, RegenerationWalk, Sample, SampleHalt,
    SampledRequest, StatisticalClaim, StatisticalRequest, TestSubject, WalkStep,
    DEFAULT_MAX_WITNESSES,
};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::TransitionId;
use fsl_eval::simulation::model::{ExpansionLimits, TimedSystem};
use fsl_eval::temporal::UndefinedCause;
use fsl_foundation::diagnostic::Code;
use fsl_foundation::selection::DefinitionRef;
use fsl_replay::statistical::{
    replay_sampled_witness, Confidence, Decision, MeasureValue, StatisticalLimit,
    StatisticalLimits, StatisticalMethod, StatisticalOutcome, StatisticalRefusal, StatisticalStop,
    StatisticalUnsupported, StatisticalVerdict, UndecidedCause, WitnessReplay, WitnessValue,
};
use fsl_replay::statistical::{settle_statistical, settle_statistical_refusal, SubjectLimits};
use fsl_replay::SymmetryRefusal;
use fsl_replay::{
    settle_exact_prob_build_issue, ExactProbBuildIssue, IncompleteCause, ModelCheckLimit,
};
use fsl_replay::{ClaimPredicate, TerminalValue, UndefinedLocus};
use fsl_route::reductions::SymmetryDeclaration;
use fsl_semantics::check::{Event, ProbForm, ProbScheduler};
use ix_trace_rs::trace;
use serde_json::json;

use crate::analyze_cli_support::{analyze as analyze_source, request_over, Workspace};
use crate::prob_world::{self, World};

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn sampler() -> DefinitionRef {
    DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1").expect("a valid DefinitionRef")
}

fn run(
    world: &World,
    claim: &str,
    method: StatisticalMethod,
    limits: StatisticalLimits,
    max_witnesses: u32,
) -> Result<StatisticalOutcome, StatisticalRefusal> {
    run_workers(world, claim, method, limits, max_witnesses, 1)
}

fn run_workers(
    world: &World,
    claim: &str,
    method: StatisticalMethod,
    limits: StatisticalLimits,
    max_witnesses: u32,
    workers: usize,
) -> Result<StatisticalOutcome, StatisticalRefusal> {
    check_statistical(
        StatisticalRequest {
            workers,
            subject: world.subject(),
            claim: world.claim(claim),
            method,
            seed: 7,
            sampler: sampler(),
            limits,
            max_witnesses,
            symmetry: Vec::new(),
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        },
        &Cancel::new(),
    )
}

/// An unused walk for the synchronous `check_sampled` test seam.
struct UnusedWalk;

impl RegenerationWalk for UnusedWalk {
    fn step(&mut self) -> Result<WalkStep, SampleHalt> {
        panic!("the SPRT tests do not request a regenerative walk")
    }
}

struct CandidateStopSampler;

impl BehaviourSampler for CandidateStopSampler {
    type Walk = UnusedWalk;

    fn sample_batch_width(&self) -> usize {
        1
    }

    fn sample(
        &mut self,
        _test: &TestSubject,
        _seed: u64,
        _trace: u64,
    ) -> Result<Drawn, SampleHalt> {
        Err(SampleHalt::Expansion(fsl_eval::simulation::ExpansionStop {
            cause: filament_semantic_value::catalog::CatalogCode::new(
                "resource_exhausted",
                "insufficient-next-charge",
            ),
            limit: Some(fsl_eval::simulation::ExpansionLimit::MaxCandidates),
        }))
    }

    fn sample_batch(
        &mut self,
        test: &TestSubject,
        seed: u64,
        traces: &[u64],
    ) -> Vec<(u64, Result<Drawn, SampleHalt>)> {
        traces
            .first()
            .copied()
            .map(|trace| (trace, self.sample(test, seed, trace)))
            .into_iter()
            .collect()
    }

    fn walk(&mut self, _test: &TestSubject, _seed: u64, _trace: u64) -> Self::Walk {
        UnusedWalk
    }
}

#[derive(Clone, Copy)]
enum TracePattern {
    /// Alternate the result for the first hundred trace indices, then hold.
    PrefixAroundBoundary,
    /// Every sample holds.
    AlwaysHolds,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecordedSample {
    seed: u64,
    initial: u32,
    binding: Option<String>,
    trace: u64,
    holds: bool,
}

/// Deterministic seam test double. The first hundred samples remain inside
/// the SPRT indifference region, and the all-holding tail makes the default
/// run terminate shortly after the stopped prefix.
struct TraceSampler {
    pattern: TracePattern,
    draws: Vec<RecordedSample>,
    completed: Vec<u64>,
    cancel_on_trace: Option<(u64, Cancel)>,
}

impl TraceSampler {
    fn new(pattern: TracePattern) -> Self {
        Self {
            pattern,
            draws: Vec::new(),
            completed: Vec::new(),
            cancel_on_trace: None,
        }
    }

    fn draw(pattern: TracePattern, seed: u64, trace: u64) -> Drawn {
        let holds = match pattern {
            TracePattern::AlwaysHolds => true,
            TracePattern::PrefixAroundBoundary if trace >= 100 => true,
            TracePattern::PrefixAroundBoundary => trace.is_multiple_of(2) == (seed == 7),
        };
        Drawn {
            sample: Sample::Measured {
                holds,
                value: MeasureValue::Value(if holds { ratio(4, 1) } else { ratio(6, 1) }),
            },
            steps: Vec::new(),
        }
    }

    fn cancelling(mut self, trace: u64, cancel: Cancel) -> Self {
        self.cancel_on_trace = Some((trace, cancel));
        self
    }
}

impl BehaviourSampler for TraceSampler {
    type Walk = UnusedWalk;

    fn sample_batch_width(&self) -> usize {
        64
    }

    fn sample(&mut self, test: &TestSubject, seed: u64, trace: u64) -> Result<Drawn, SampleHalt> {
        let drawn = Self::draw(self.pattern, seed, trace);
        let Sample::Measured { holds, .. } = &drawn.sample else {
            unreachable!("the trace sampler always returns a measurement");
        };
        self.draws.push(RecordedSample {
            seed,
            initial: test.initial,
            binding: test.binding.as_ref().map(|instance| instance.key.clone()),
            trace,
            holds: *holds,
        });
        self.completed.push(trace);
        if let Some((cancel_at, cancel)) = &self.cancel_on_trace {
            if trace == *cancel_at {
                cancel.cancel(filament_exact::CancelCause::Requested);
            }
        }
        Ok(drawn)
    }

    fn sample_batch(
        &mut self,
        test: &TestSubject,
        seed: u64,
        traces: &[u64],
    ) -> Vec<(u64, Result<Drawn, SampleHalt>)> {
        if traces.len() == 1 {
            let trace = traces[0];
            return vec![(trace, self.sample(test, seed, trace))];
        }
        let barrier = Arc::new(Barrier::new(traces.len()));
        let released = Arc::new((Mutex::new(false), Condvar::new()));
        let completed = Arc::new(Mutex::new(Vec::with_capacity(traces.len())));
        let pattern = self.pattern;
        let initial = test.initial;
        let binding = test.binding.as_ref().map(|instance| instance.key.clone());
        let cancel_on_trace = self.cancel_on_trace.clone();
        let max_trace = *traces.iter().max().expect("a worker batch is nonempty");
        let mut joined = std::thread::scope(|scope| {
            traces
                .iter()
                .copied()
                .map(|trace| {
                    let barrier = Arc::clone(&barrier);
                    let released = Arc::clone(&released);
                    let completed = Arc::clone(&completed);
                    let binding = binding.clone();
                    let cancel_on_trace = cancel_on_trace.clone();
                    scope.spawn(move || {
                        barrier.wait();
                        let is_first = trace == max_trace;
                        if !is_first {
                            let (lock, wake) = &*released;
                            let mut first_finished = lock.lock().expect("completion lock");
                            while !*first_finished {
                                first_finished =
                                    wake.wait(first_finished).expect("completion wait");
                            }
                        }
                        let drawn = Self::draw(pattern, seed, trace);
                        let Sample::Measured { holds, .. } = &drawn.sample else {
                            unreachable!("the trace sampler always returns a measurement");
                        };
                        completed.lock().expect("completion record").push(trace);
                        if let Some((cancel_at, cancel)) = &cancel_on_trace {
                            if trace == *cancel_at {
                                cancel.cancel(filament_exact::CancelCause::Requested);
                            }
                        }
                        if is_first {
                            let (lock, wake) = &*released;
                            *lock.lock().expect("completion lock") = true;
                            wake.notify_all();
                        }
                        (
                            trace,
                            RecordedSample {
                                seed,
                                initial,
                                binding,
                                trace,
                                holds: *holds,
                            },
                            Ok(drawn),
                        )
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().expect("sample worker completes"))
                .collect::<Vec<_>>()
        });
        self.completed
            .extend(completed.lock().expect("completion record").iter().copied());
        self.draws
            .extend(joined.iter().map(|(_, record, _)| record.clone()));
        joined
            .drain(..)
            .map(|(trace, _, result)| (trace, result))
            .collect()
    }

    fn walk(&mut self, _test: &TestSubject, _seed: u64, _trace: u64) -> Self::Walk {
        UnusedWalk
    }
}

fn sampled_p95() -> StatisticalClaim {
    StatisticalClaim::quantile(
        ratio(1, 2),
        QuantileComparison::AtMost,
        ratio(5, 1),
        Confidence::new(ratio(1, 10), ratio(1, 10), ratio(1, 10)).expect("confidence"),
    )
    .expect("a valid quantile claim")
}

fn sampled_request<'a>(
    claim: &'a StatisticalClaim,
    tests: &'a [TestSubject],
    sampler_identity: &'a DefinitionRef,
    method: StatisticalMethod,
    seed: u64,
    limits: StatisticalLimits,
) -> SampledRequest<'a> {
    SampledRequest {
        workers: 1,
        claim,
        tests,
        method,
        seed,
        sampler: sampler_identity,
        limits,
        max_witnesses: 0,
        timed_sampling: fsl_replay::statistical::TimedSampling::default(),
    }
}

/// Two real binding tests execute serially with the same per-test
/// confidence. The minimum allowance that completes either alone cannot be
/// replenished when the second test starts.
#[trace("FR-120-AC-35", "FR-192-AC-8")]
#[test]
fn serial_binding_tests_share_the_probability_work_allowance() {
    let claims = "probabilistic One using ev on M over (n: M::Node) under Steady on origin with alpha 0.2, beta 0.2, indifference 0.4 { probability >= 0.5 [ always[0,0] holds(n.healthy) ] }\nprobabilistic Two using ev on M over (n: M::Node) under Steady on origin with alpha 0.4, beta 0.4, indifference 0.4 { probability >= 0.5 [ always[0,0] holds(n.healthy) ] }";
    let model = prob_world::health(
        "self.healthy = pre(self.healthy)",
        "true: 1, false: 1",
        false,
        claims,
    );
    let one_world = World::new(&model);
    let two_world = World::with_objects(&model, &["a", "b"]);
    let run_with = |bound, two, workers| {
        let world = if two { &two_world } else { &one_world };
        let mut subject = world.subject();
        subject.simulation.meter.work_units = bound;
        check_statistical(
            StatisticalRequest {
                workers,
                subject,
                claim: world.claim(if two { "Two" } else { "One" }),
                method: StatisticalMethod::Okamoto,
                seed: 7,
                sampler: sampler(),
                limits: StatisticalLimits {
                    max_samples: 64,
                    max_draws: 64,
                    ..StatisticalLimits::default()
                },
                max_witnesses: 0,
                symmetry: Vec::new(),
                timed_sampling: fsl_replay::statistical::TimedSampling::default(),
            },
            &Cancel::new(),
        )
    };
    // Find an exact completion boundary through the real engine; no cost
    // formula or duplicate evaluator is used as a test oracle.
    let (mut low, mut high) = (0, 1_000_000);
    assert!(matches!(
        run_with(high, false, 1),
        Ok(StatisticalOutcome::Completed(_))
    ));
    while low < high {
        let middle = low + (high - low) / 2;
        if matches!(
            run_with(middle, false, 1),
            Ok(StatisticalOutcome::Completed(_))
        ) {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    let Ok(StatisticalOutcome::Completed(one)) = run_with(low, false, 1) else {
        panic!("one test fits its boundary")
    };
    assert!(one.basis.samples > 0);
    let two = run_with(low, true, 1);
    let Ok(StatisticalOutcome::Stopped {
        stop,
        partial,
        provenance,
    }) = two
    else {
        panic!("the later binding test must spend the same allowance: {two:?}")
    };
    assert!(
        matches!(
            stop.clone(),
            StatisticalStop::Expansion(fsl_eval::simulation::ExpansionStop {
                limit: Some(fsl_eval::simulation::ExpansionLimit::EvaluationMeter(
                    filament_exact::LimitKind::WorkUnits
                )),
                ..
            })
        ),
        "{stop:?}"
    );
    assert_eq!(partial.tests, 2);
    assert_eq!(partial.samples, one.basis.samples);
    assert_eq!(provenance.tests.len(), 2);
    assert_eq!(provenance.tests[0].initial, 0);
    assert_eq!(provenance.tests[1].initial, 0);
    assert_ne!(provenance.tests[0].binding, provenance.tests[1].binding);
    assert_eq!(provenance.tests[0].traces.count, one.basis.draws);
    assert_eq!(
        provenance.tests[1].traces.count, 1,
        "the existing serial sampler reports its attempted unpaid draw"
    );

    let mut meter = prob_world::UNLIMITED;
    meter.work_units = low;
    let limits = SubjectLimits {
        expansion: ExpansionLimits {
            max_candidates: 1_000_000,
        },
        meter: &meter,
    };
    let outcome = StatisticalOutcome::Stopped {
        stop: stop.clone(),
        partial: partial.clone(),
        provenance: provenance.clone(),
    };
    for workers in [2, 4, 64] {
        assert_eq!(
            run_with(low, true, workers),
            Ok(outcome.clone()),
            "model work retains ascending logical admission with workers={workers}"
        );
    }
    let statistical = settle_statistical(&outcome, limits);
    let StatisticalStop::Expansion(expansion) = &stop else {
        panic!("expected an expansion stop, got {stop:?}");
    };
    let exact = settle_exact_prob_build_issue(&ExactProbBuildIssue::Expansion(*expansion), limits);
    assert_eq!(statistical.value, exact.value().clone());
    assert_eq!(statistical.measured(), None);
    assert!(statistical.diagnostic.is_none());
    assert_eq!(
        statistical.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::EvaluationMeter {
                counter: filament_exact::LimitKind::WorkUnits,
            },
            value: low,
            required: None,
        })
    );
    let TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, .. }) = &statistical.value
    else {
        panic!(
            "the work-unit stop remains incomplete: {:?}",
            statistical.value
        );
    };
    assert_eq!(limit.setting(), "accounting.work_units");

    let local_candidate_bound = 17;
    let local_claim = sampled_p95();
    let local_tests = [TestSubject {
        initial: 0,
        binding: None,
        reference: None,
    }];
    let sampler_identity = sampler();
    let local_candidate = check_sampled(
        sampled_request(
            &local_claim,
            &local_tests,
            &sampler_identity,
            StatisticalMethod::Sprt,
            7,
            StatisticalLimits::default(),
        ),
        &mut CandidateStopSampler,
        &Cancel::new(),
    )
    .expect("the sampler's candidate stop is returned");
    assert!(matches!(
        &local_candidate,
        StatisticalOutcome::Stopped {
            stop: StatisticalStop::Expansion(fsl_eval::simulation::ExpansionStop {
                limit: Some(fsl_eval::simulation::ExpansionLimit::MaxCandidates),
                ..
            }),
            ..
        }
    ));
    let local_limits = SubjectLimits {
        expansion: ExpansionLimits {
            max_candidates: local_candidate_bound,
        },
        meter: &meter,
    };
    let local_record = settle_statistical(&local_candidate, local_limits);
    assert_eq!(
        local_record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxCandidates,
            value: local_candidate_bound,
            required: None,
        })
    );
    let TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, .. }) =
        &local_record.value
    else {
        panic!(
            "the candidate stop remains incomplete: {:?}",
            local_record.value
        );
    };
    assert_eq!(limit.setting(), "expansion.max_candidates");
    assert_eq!(local_record.measured(), None);
    assert!(local_record.diagnostic.is_none());

    let cancel = Cancel::new();
    cancel.cancel(filament_exact::CancelCause::Requested);
    let cancelled = check_sampled(
        sampled_request(
            &local_claim,
            &local_tests,
            &sampler_identity,
            StatisticalMethod::Sprt,
            7,
            StatisticalLimits::default(),
        ),
        &mut TraceSampler::new(TracePattern::AlwaysHolds),
        &cancel,
    )
    .expect("the cancelled request retains its cause");
    assert!(matches!(
        &cancelled,
        StatisticalOutcome::Stopped {
            stop: StatisticalStop::Cancelled(filament_exact::CancelCause::Requested),
            ..
        }
    ));
    let cancelled_record = settle_statistical(&cancelled, limits);
    assert_eq!(
        cancelled_record.value,
        TerminalValue::Incomplete(IncompleteCause::Cancelled {
            source: filament_exact::CancelCause::Requested,
        })
    );
    assert_eq!(cancelled_record.measured(), None);
    assert!(cancelled_record.diagnostic.is_none());
}

/// FR-191-AC-3's sample-reproduction clauses through the public
/// `BehaviourSampler` seam: the stopped trace prefix and its default-limit
/// rerun have identical sample values; changing only the seed changes a
/// value while retaining the same checked statistical claim.
#[trace("TC-626", "FR-191-AC-3")]
#[test]
fn sampled_seven_reproduces_a_stopped_prefix_and_seed_eight_changes_it() {
    let claim = sampled_p95();
    let tests = [TestSubject {
        initial: 0,
        binding: None,
        reference: None,
    }];
    let sampler_identity = sampler();
    let default = sampled_request(
        &claim,
        &tests,
        &sampler_identity,
        StatisticalMethod::Sprt,
        7,
        StatisticalLimits::default(),
    );
    let mut first_sampler = TraceSampler::new(TracePattern::PrefixAroundBoundary);
    let first = check_sampled(default, &mut first_sampler, &Cancel::new())
        .expect("the deterministic sampled run completes");
    let mut repeated_sampler = TraceSampler::new(TracePattern::PrefixAroundBoundary);
    let repeated = check_sampled(default, &mut repeated_sampler, &Cancel::new())
        .expect("the repeated sampled run completes");
    assert_eq!(first, repeated);
    assert_eq!(
        format!("{first:?}").as_bytes(),
        format!("{repeated:?}").as_bytes()
    );
    assert_eq!(first_sampler.draws, repeated_sampler.draws);

    let limited = StatisticalLimits {
        max_samples: 100,
        ..Default::default()
    };
    let stopped_request = sampled_request(
        &claim,
        &tests,
        &sampler_identity,
        StatisticalMethod::Sprt,
        7,
        limited,
    );
    let mut stopped_sampler = TraceSampler::new(TracePattern::PrefixAroundBoundary);
    let stopped = check_sampled(stopped_request, &mut stopped_sampler, &Cancel::new())
        .expect("the requested sample limit is an outcome");
    let StatisticalOutcome::Stopped {
        stop:
            StatisticalStop::Limit {
                limit: StatisticalLimit::MaxSamples,
                value: 100,
            },
        partial,
        provenance,
    } = stopped
    else {
        panic!("stopped at max_samples 100: {stopped:?}");
    };
    assert_eq!((partial.samples, partial.draws), (100, 100));
    assert_eq!(provenance.seed, 7);
    assert_eq!(provenance.tests[0].traces.first, 0);
    assert_eq!(provenance.tests[0].traces.step, 1);
    assert_eq!(provenance.tests[0].traces.count, 100);
    assert_eq!(
        stopped_sampler
            .draws
            .iter()
            .map(|draw| draw.trace)
            .collect::<Vec<_>>(),
        (0..100).collect::<Vec<_>>()
    );

    let StatisticalOutcome::Completed(full) = first else {
        panic!("the default-limit rerun completes: {first:?}");
    };
    assert_eq!(full.provenance.seed, 7);
    assert_eq!(
        stopped_sampler.draws,
        repeated_sampler
            .draws
            .iter()
            .take(100)
            .cloned()
            .collect::<Vec<_>>()
    );

    let reseeded_request = sampled_request(
        &claim,
        &tests,
        &sampler_identity,
        StatisticalMethod::Sprt,
        8,
        StatisticalLimits::default(),
    );
    let mut reseeded_sampler = TraceSampler::new(TracePattern::PrefixAroundBoundary);
    let reseeded = check_sampled(reseeded_request, &mut reseeded_sampler, &Cancel::new())
        .expect("seed 8 is the same claim under another seed");
    assert_eq!(claim.verdict_bound(), sampled_p95().verdict_bound());
    assert!(reseeded_sampler.draws.iter().any(|draw| {
        repeated_sampler
            .draws
            .iter()
            .find(|prior| prior.trace == draw.trace)
            .is_some_and(|prior| prior.holds != draw.holds)
    }));
    match reseeded {
        StatisticalOutcome::Completed(verdict) => assert_eq!(verdict.provenance.seed, 8),
        StatisticalOutcome::Stopped { provenance, .. }
        | StatisticalOutcome::Unsupported { provenance, .. } => {
            assert_eq!(provenance.seed, 8)
        }
    }
    // StatisticalOutcome exposes request provenance and the claim-bound, but
    // no obligation-identity field exists at this seam to compare by seed.
}

/// FR-191-AC-6: native model requests preserve the serial model-work order
/// for every requested worker count, while a real concurrent sampler
/// completes high trace indices first and is merged in trace order.
#[trace("TC-1417", "FR-191-AC-6")]
#[test]
fn worker_counts_preserve_model_results_and_merge_concurrent_samples_by_trace() {
    let service = service();
    let limited = StatisticalLimits {
        max_samples: 100,
        ..StatisticalLimits::default()
    };
    let one_worker = run_workers(&service, "P95", StatisticalMethod::Sprt, limited, 0, 1)
        .expect("the one-worker P95 run");
    for workers in [2, 4, 64] {
        let parallel = run_workers(
            &service,
            "P95",
            StatisticalMethod::Sprt,
            limited,
            0,
            workers,
        )
        .expect("the requested worker count keeps the same model route");
        assert_eq!(parallel, one_worker, "workers={workers}");
        assert_eq!(
            format!("{parallel:?}").as_bytes(),
            format!("{one_worker:?}").as_bytes(),
            "workers={workers}"
        );
    }
    let no_fault = World::new(&prob_world::no_fault(
        "probabilistic NoFault using ev on M over (n: M::Node) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.0005 \
         { probability >= 0.999 [ always[0,1000] holds(n.healthy) ] }",
    ));
    let preflight = StatisticalLimits {
        max_samples: 1_000,
        ..StatisticalLimits::default()
    };
    let baseline = run_workers(
        &no_fault,
        "NoFault",
        StatisticalMethod::Okamoto,
        preflight,
        0,
        1,
    )
    .expect("the one-worker fixed-sample preflight");
    assert!(matches!(
        &baseline,
        StatisticalOutcome::Stopped {
            stop: StatisticalStop::PreflightLimit {
                limit: StatisticalLimit::MaxSamples,
                value: 1_000,
                required,
            },
            partial,
            ..
        } if partial.samples == 0 && partial.draws == 0 && required == &filament_exact::Integer::from(9_210_341_u64)
    ));
    for workers in [2, 4, 64] {
        let parallel = run_workers(
            &no_fault,
            "NoFault",
            StatisticalMethod::Okamoto,
            preflight,
            0,
            workers,
        )
        .expect("the requested worker count keeps the same preflight");
        assert_eq!(parallel, baseline, "workers={workers}");
        assert_eq!(
            format!("{parallel:?}").as_bytes(),
            format!("{baseline:?}").as_bytes(),
            "workers={workers}"
        );
    }

    let claim = sampled_p95();
    let tests = [TestSubject {
        initial: 0,
        binding: None,
        reference: None,
    }];
    let sampler_identity = sampler();
    let mut concurrent_request = sampled_request(
        &claim,
        &tests,
        &sampler_identity,
        StatisticalMethod::Sprt,
        7,
        limited,
    );
    concurrent_request.workers = 4;
    let mut concurrent_sampler = TraceSampler::new(TracePattern::PrefixAroundBoundary);
    let concurrent = check_sampled(concurrent_request, &mut concurrent_sampler, &Cancel::new())
        .expect("the concurrent deterministic sampler completes");
    let mut serial_sampler = TraceSampler::new(TracePattern::PrefixAroundBoundary);
    let serial = check_sampled(
        SampledRequest {
            workers: 1,
            ..concurrent_request
        },
        &mut serial_sampler,
        &Cancel::new(),
    )
    .expect("the serial deterministic sampler completes");
    assert_eq!(concurrent, serial);
    assert_eq!(
        concurrent_sampler.draws, serial_sampler.draws,
        "only the merged trace prefix contributes samples"
    );
    let StatisticalOutcome::Stopped {
        stop:
            StatisticalStop::Limit {
                limit: StatisticalLimit::MaxSamples,
                value: 100,
            },
        partial,
        provenance,
    } = &concurrent
    else {
        panic!("the parallel run stops at max_samples 100: {concurrent:?}");
    };
    assert_eq!((partial.samples, partial.draws), (100, 100));
    assert_eq!(concurrent_sampler.draws.len(), 100);
    assert_eq!(
        concurrent_sampler
            .draws
            .iter()
            .map(|sample| sample.trace)
            .collect::<Vec<_>>(),
        (0..100).collect::<Vec<_>>(),
        "the merged and recorded trace prefix is exactly 0 through 99"
    );
    assert_eq!(provenance.tests[0].traces.first, 0);
    assert_eq!(provenance.tests[0].traces.count, 100);
    assert!(concurrent_sampler
        .completed
        .chunks(4)
        .all(|batch| batch[0] == *batch.iter().max().expect("nonempty batch")));
}

/// FR-191-AC-7: a zero-worker request is refused before the sampler is called.
#[trace("TC-1417", "FR-191-AC-7")]
#[test]
fn zero_workers_refuses_without_drawing_a_sample() {
    let claim = sampled_p95();
    let tests = [TestSubject {
        initial: 0,
        binding: None,
        reference: None,
    }];
    let sampler_identity = sampler();
    let mut request = sampled_request(
        &claim,
        &tests,
        &sampler_identity,
        StatisticalMethod::Sprt,
        7,
        StatisticalLimits::default(),
    );
    request.workers = 0;
    let mut sampler = TraceSampler::new(TracePattern::AlwaysHolds);
    assert_eq!(
        check_sampled(request, &mut sampler, &Cancel::new()),
        Err(StatisticalRefusal::Workers)
    );
    assert!(sampler.draws.is_empty());
    let world = service();
    assert_eq!(
        run_workers(
            &world,
            "P95",
            StatisticalMethod::Sprt,
            StatisticalLimits::default(),
            0,
            0,
        ),
        Err(StatisticalRefusal::Workers)
    );
}

/// FR-191-AC-7: cancellation after a shared prefix retains exactly the
/// one-worker merged prefix and exposes no speculative higher trace.
#[trace("TC-1417", "FR-191-AC-7")]
#[test]
fn concurrent_cancellation_discards_unmerged_higher_traces() {
    let claim = sampled_p95();
    let tests = [TestSubject {
        initial: 0,
        binding: None,
        reference: None,
    }];
    let sampler_identity = sampler();
    let limits = StatisticalLimits {
        max_samples: 100,
        ..StatisticalLimits::default()
    };
    let mut concurrent_request = sampled_request(
        &claim,
        &tests,
        &sampler_identity,
        StatisticalMethod::Sprt,
        7,
        limits,
    );
    concurrent_request.workers = 4;
    let concurrent_cancel = Cancel::new();
    let mut concurrent_sampler = TraceSampler::new(TracePattern::PrefixAroundBoundary)
        .cancelling(4, concurrent_cancel.clone());
    let concurrent = check_sampled(
        concurrent_request,
        &mut concurrent_sampler,
        &concurrent_cancel,
    )
    .expect("the cancellation is a stopped outcome");

    let serial_cancel = Cancel::new();
    let mut serial_sampler =
        TraceSampler::new(TracePattern::PrefixAroundBoundary).cancelling(4, serial_cancel.clone());
    let serial = check_sampled(
        SampledRequest {
            workers: 1,
            ..concurrent_request
        },
        &mut serial_sampler,
        &serial_cancel,
    )
    .expect("the serial cancellation is a stopped outcome");
    assert_eq!(concurrent, serial);
    let StatisticalOutcome::Stopped {
        stop: StatisticalStop::Cancelled(filament_exact::CancelCause::Requested),
        partial,
        provenance,
    } = concurrent
    else {
        panic!("cancellation retains the merged prefix: {concurrent:?}");
    };
    assert_eq!((partial.samples, partial.draws, partial.tests), (4, 4, 1));
    assert_eq!(provenance.tests[0].traces.first, 0);
    assert_eq!(provenance.tests[0].traces.count, 4);
}

/// FR-191-AC-3: the source route writes one obligation identity for the same
/// checked P95 claim regardless of its seed, while retaining the selected
/// seed in each run's provenance.
#[trace("TC-626", "FR-191-AC-3")]
#[test]
fn source_seed_changes_leave_the_p95_obligation_identity_unchanged() {
    let first = source_p95_item("fr191-ac3-seed-seven", &["s"], "sprt", 7);
    let repeated = source_p95_item("fr191-ac3-repeat-seven", &["s"], "sprt", 7);
    let reseeded = source_p95_item("fr191-ac3-seed-eight", &["s"], "sprt", 8);

    assert_eq!(first["record"]["result"], "measured", "{first}");
    assert_eq!(repeated["record"]["result"], "measured", "{repeated}");
    assert_eq!(reseeded["record"]["result"], "measured", "{reseeded}");
    assert!(first["identity"].is_object(), "{first}");
    assert_eq!(first["identity"], repeated["identity"]);
    assert_eq!(first["identity"], reseeded["identity"]);
    assert_eq!(first["verdict"]["provenance"]["seed"], "7");
    assert_eq!(reseeded["verdict"]["provenance"]["seed"], "8");
}

/// FR-191-AC-4's provenance fields through `check_sampled`: each logical test
/// retains its initial-state index, concrete binding and interleaved trace
/// range, and the selected method is recorded in the result basis.
#[trace("TC-626", "FR-191-AC-4")]
#[test]
fn sampled_two_test_provenance_keeps_bindings_and_method() {
    let claim = sampled_p95();
    let population = DeclarationKey {
        package: "test/statistical".to_owned(),
        node: "ix://test/statistical/servers".to_owned(),
    };
    let tests = [
        TestSubject {
            initial: 3,
            binding: Some(fsl_eval::model_subject::ModelInstance {
                population: population.clone(),
                key: "a".to_owned(),
            }),
            reference: None,
        },
        TestSubject {
            initial: 4,
            binding: Some(fsl_eval::model_subject::ModelInstance {
                population: population.clone(),
                key: "b".to_owned(),
            }),
            reference: None,
        },
    ];
    let sampler_identity = sampler();
    let request = |method| {
        sampled_request(
            &claim,
            &tests,
            &sampler_identity,
            method,
            7,
            StatisticalLimits::default(),
        )
    };
    let mut okamoto_sampler = TraceSampler::new(TracePattern::AlwaysHolds);
    let StatisticalOutcome::Completed(okamoto) = check_sampled(
        request(StatisticalMethod::Okamoto),
        &mut okamoto_sampler,
        &Cancel::new(),
    )
    .expect("Okamoto completes") else {
        panic!("Okamoto completion expected");
    };
    assert_eq!(okamoto.provenance.tests.len(), 2);
    for (entry, initial, key, trace) in [
        (&okamoto.provenance.tests[0], 3, "a", 0),
        (&okamoto.provenance.tests[1], 4, "b", 1),
    ] {
        assert_eq!(entry.initial, initial);
        assert_eq!(
            entry.binding.as_ref().map(|binding| binding.key.as_str()),
            Some(key)
        );
        assert_eq!((entry.traces.first, entry.traces.step), (trace, 2));
        assert_eq!(
            entry.traces.count,
            okamoto.tests[entry.index as usize].draws
        );
    }
    assert_eq!(okamoto.basis.method, StatisticalMethod::Okamoto);

    let mut sprt_sampler = TraceSampler::new(TracePattern::AlwaysHolds);
    let StatisticalOutcome::Completed(sprt) = check_sampled(
        request(StatisticalMethod::Sprt),
        &mut sprt_sampler,
        &Cancel::new(),
    )
    .expect("SPRT completes") else {
        panic!("SPRT completion expected");
    };
    assert_eq!(sprt.basis.method, StatisticalMethod::Sprt);
    assert_eq!(
        okamoto.provenance.tests, sprt.provenance.tests,
        "the logical tests and trace assignment do not depend on method"
    );
    // Obligation identity is not returned by SampledRequest or
    // StatisticalOutcome, so the method/identity clause has no comparison
    // surface at this API boundary.
}

/// FR-191-AC-4: the source route records both bindings and their interleaved
/// trace ranges; changing the method changes the recorded method, not the
/// claim's certificate identity.
#[trace("TC-626", "FR-191-AC-4")]
#[test]
fn source_two_test_p95_provenance_keeps_identity_across_methods() {
    let okamoto = source_p95_item("fr191-ac4-okamoto", &["a", "b"], "okamoto", 7);
    let sprt = source_p95_item("fr191-ac4-sprt", &["a", "b"], "sprt", 7);

    assert_eq!(okamoto["record"]["result"], "measured", "{okamoto}");
    assert_eq!(sprt["record"]["result"], "measured", "{sprt}");
    assert!(okamoto["identity"].is_object(), "{okamoto}");
    assert_eq!(okamoto["identity"], sprt["identity"]);
    assert_eq!(okamoto["verdict"]["basis"]["method"]["type"], "okamoto");
    assert_eq!(sprt["verdict"]["basis"]["method"]["type"], "sprt");

    let tests = okamoto["verdict"]["provenance"]["tests"]
        .as_array()
        .expect("the source verdict records test provenance");
    assert_eq!(tests.len(), 2, "{okamoto}");
    for (index, key) in ["a", "b"].into_iter().enumerate() {
        let test = &tests[index];
        assert_eq!(
            test["index"],
            u64::try_from(index).expect("two tests"),
            "{test}"
        );
        assert_eq!(test["initial_state"], 0, "{test}");
        assert_eq!(test["binding"]["reference"]["key"], key, "{test}");
        assert_eq!(test["first_trace_index"], index.to_string(), "{test}");
        let first = test["first_trace_index"]
            .as_str()
            .expect("the decimal trace start")
            .parse::<u64>()
            .expect("a trace index");
        let last = test["last_trace_index"]
            .as_str()
            .expect("the decimal trace end")
            .parse::<u64>()
            .expect("a trace index");
        let draws = okamoto["verdict"]["tests"][index]["draws"]
            .as_u64()
            .expect("the test's draw count");
        assert!(draws > 0, "{test}");
        assert_eq!(last - first, 2 * (draws - 1), "{test}");
    }
}

fn run_timed_method(
    world: &crate::timed_world::World,
    claim_name: &str,
    method: StatisticalMethod,
    seed: u64,
    timed_sampling: fsl_replay::statistical::TimedSampling,
    max_witnesses: u32,
) -> Result<StatisticalOutcome, StatisticalRefusal> {
    run_timed_method_with_limits(
        world,
        claim_name,
        method,
        seed,
        timed_sampling,
        StatisticalLimits::default(),
        max_witnesses,
    )
}

fn run_timed_method_with_limits(
    world: &crate::timed_world::World,
    claim_name: &str,
    method: StatisticalMethod,
    seed: u64,
    timed_sampling: fsl_replay::statistical::TimedSampling,
    limits: StatisticalLimits,
    max_witnesses: u32,
) -> Result<StatisticalOutcome, StatisticalRefusal> {
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim(claim_name)
        .expect("the timed claim checks");
    check_statistical(
        StatisticalRequest {
            workers: 1,
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim,
            method,
            seed,
            sampler: sampler(),
            limits,
            max_witnesses,
            symmetry: Vec::new(),
            timed_sampling,
        },
        &Cancel::new(),
    )
}

fn timed_late_document() -> Vec<u8> {
    let mut document: serde_json::Value = serde_json::from_slice(&crate::timed_world::document(
        &["x"],
        &[("finish", &["phase", "late"])],
    ))
    .expect("the timed fixture document is JSON");
    let operation = &mut document["types"][1]["operations"][0];
    let parameter = crate::timed_world::ident("Call/finish/isLate");
    operation["params"] = json!([{
        "identity": parameter,
        "name": "isLate",
        "typeRef": "ix://filament/native/Boolean",
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"source": {"sourceIdentity": crate::timed_world::ident("spec"),
            "path": "spec.fspec", "startLine": 1, "startColumn": 1}},
    }]);
    serde_json::to_vec(&document).expect("the parameterized document serializes")
}

fn timed_late_world(
    delay_declaration: &str,
    probability_bound: &str,
    horizon_ms: u32,
) -> crate::timed_world::World {
    let body = format!(
        "{}\
         pre Waiting using v on M::Call::finish {{ self.phase = 0 }}\n\
         post Finished using v on M::Call::finish {{ self.phase = 1 and self.late = isLate }}\n\
         random M::Call::finish(isLate) ~ {{ false: 19, true: 1 }};\n\
         {delay_declaration}\n\
         workload Even on M {{ weight Call::finish = 1; }}\n\
         probabilistic RareLate using ev on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             {{ {probability_bound} [ eventually[0 ms, {horizon_ms} ms] holds(c.late) ] }}\n",
        crate::timed_world::DENSE,
    );
    crate::timed_world::world(
        &timed_late_document(),
        "profile ev = \"filament.temporal.timed/v1\";\n",
        &body,
    )
}

fn timed_world_with(
    operations: &[(&str, &[&str])],
    declarations: &str,
) -> crate::timed_world::World {
    let body = format!("{}\n{declarations}", crate::timed_world::DENSE);
    crate::timed_world::world(
        &crate::timed_world::document(&["x"], operations),
        "profile ev = \"filament.temporal.timed/v1\";\n",
        &body,
    )
}

fn completed(outcome: Result<StatisticalOutcome, StatisticalRefusal>) -> StatisticalVerdict {
    match outcome {
        Ok(StatisticalOutcome::Completed(verdict)) => *verdict,
        other => panic!("expected a completed run, got {other:?}"),
    }
}

fn decided(world: &World, claim: &str, method: StatisticalMethod) -> StatisticalVerdict {
    completed(run(
        world,
        claim,
        method,
        StatisticalLimits::default(),
        DEFAULT_MAX_WITNESSES,
    ))
}

fn integer_of(value: &Value) -> i64 {
    match value {
        Value::Integer(integer) => integer.to_string().parse().expect("small"),
        other => panic!("not an integer: {other:?}"),
    }
}

/// ADR-024 §7.2's `P95` at `threshold`: the `duration` from `Busy` to
/// `Done` within 10 steps.
fn p95(name: &str, threshold: u32) -> String {
    format!(
        "probabilistic {name} using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         {{ quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
         until holds(s.phase = 2) within 10) <= {threshold} }}\n"
    )
}

fn service() -> World {
    World::new(&prob_world::service(&format!(
        "{}{}{}",
        p95("P95", 5),
        p95("Two", 2),
        p95("Zero", 0)
    )))
}

/// Runs the actual EN-4 source route over the service model with one binding
/// test for each live server key, returning the emitted P95 item.
fn source_p95_item(name: &str, keys: &[&str], method: &str, seed: u64) -> serde_json::Value {
    let model = prob_world::service(&p95("P95", 5));
    let (unit, package) = model.unit_and_document();
    let snapshot = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": name},
        "observation": "current",
        "model": {"identity": model.package},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{
            "population": model.ident(model.population),
            "complete": true,
            "objects": keys.iter().map(|key| json!({
                "key": key,
                "type": model.ident(model.object),
                "fields": model.initial.clone(),
            })).collect::<Vec<_>>(),
        }],
    });
    let workspace = Workspace::new(name);
    let request = request_over(
        &workspace,
        &unit,
        &package,
        &[snapshot.to_string().into_bytes()],
        json!({"items": [{"claim": "P95", "probabilistic": {
            "evidence": "statistical",
            "method": {"type": method},
            "seed": seed.to_string(),
        }}]}),
    );
    let document = analyze_source(&request).document();
    let [item] = document["items"]
        .as_array()
        .expect("the source analysis writes its item")
        .as_slice()
    else {
        panic!("one P95 item is returned: {document}");
    };
    item.clone()
}

/// Checks FR-189-AC-1 (TC-624 step 1): `P95` with Okamoto runs one test of
/// `N = 23,026` samples, each of at most 11 positions, and is Accepted with
/// an interval of half-width `1/100` whose lower end is at least `19/20`; at
/// `2 ms` it is Rejected with an interval whose upper end is below `19/20`.
/// SPRT decides both the same way with fewer samples.
#[trace("TC-624", "FR-189-AC-1")]
#[test]
fn p95_over_service_is_decided_from_the_models_own_behaviours() {
    let world = service();
    let accepted = decided(&world, "P95", StatisticalMethod::Okamoto);
    assert_eq!(accepted.decision, Decision::Accepted);
    assert_eq!(accepted.tests.len(), 1);
    assert_eq!(accepted.tests[0].samples, 23_026);
    let (low, high) = accepted.interval.clone().expect("an Okamoto interval");
    assert_eq!(high.sub(&low), ratio(2, 100));
    assert!(low >= ratio(19, 20), "lower end {low:?}");
    // `Zero` reads the same behaviours (seed, trace indices, window), and
    // every one of them violates it, so keeping every witness shows every
    // sample's steps.
    let every = completed(run(
        &world,
        "Zero",
        StatisticalMethod::Okamoto,
        StatisticalLimits::default(),
        u32::MAX,
    ));
    assert_eq!(every.witnesses.len(), 23_026);
    for witness in &every.witnesses {
        assert!(witness.steps.len() <= 10, "at most 11 positions");
    }
    let rejected = decided(&world, "Two", StatisticalMethod::Okamoto);
    assert_eq!(rejected.decision, Decision::Rejected);
    let (_, high) = rejected.interval.clone().expect("an Okamoto interval");
    assert!(high < ratio(19, 20), "upper end {high:?}");

    for (claim, decision) in [("P95", Decision::Accepted), ("Two", Decision::Rejected)] {
        let sequential = decided(&world, claim, StatisticalMethod::Sprt);
        assert_eq!(sequential.decision, decision, "{claim}");
        assert!(sequential.tests[0].samples < 23_026, "{claim}");
        assert_eq!(sequential.interval, None);
    }
}

/// FR-254-AC-3 (TC-709): an elapsed claim reads the exact difference between
/// the first waiting state and the state reached after the sampled operation.
#[trace("TC-709", "FR-254-AC-3")]
#[test]
fn elapsed_measure_reads_the_sampled_timestamp_difference() {
    let body = format!(
        "{}\
         pre Waiting using v on M::Call::send {{ self.phase = 0 }}\n\
         post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
         delay M::Call::send ~ discrete {{ 1 ms: 1 }};\n\
         workload Even on M {{ weight Call::send = 1; }}\n\
         probabilistic Latency using ev on M over (c: M::Call) under Even on origin\n\
             with alpha 0.1, beta 0.1, indifference 0.1\n\
             {{ quantile 0.5 of (elapsed from holds(c.phase = 0)\n\
             until holds(c.phase = 1) within 3 ms) <= 0 ms }}\n",
        crate::timed_world::DENSE
    );
    let world = crate::timed_world::world(
        &crate::timed_world::document(&["x"], &[("send", &["phase"])]),
        "profile ev = \"filament.temporal.timed/v1\";\n",
        &body,
    );
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("Latency")
        .expect("the latency claim checks");
    let outcome = check_statistical(
        StatisticalRequest {
            workers: 1,
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim,
            method: StatisticalMethod::Okamoto,
            seed: 7,
            sampler: sampler(),
            limits: StatisticalLimits::default(),
            max_witnesses: 1,
            symmetry: Vec::new(),
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        },
        &Cancel::new(),
    )
    .expect("the timed measure samples");
    let StatisticalOutcome::Completed(verdict) = outcome else {
        panic!("the deterministic elapsed sample decides: {outcome:?}");
    };
    assert_eq!(verdict.decision, Decision::Rejected);
    assert_eq!(verdict.basis.delay_grid_bits, Some(64));
    assert_eq!(verdict.witnesses.len(), 1);
    let witness = &verdict.witnesses[0];
    assert_eq!(
        witness.value,
        WitnessValue::Measure(MeasureValue::Value(ratio(1, 1)))
    );
    assert_eq!(witness.steps.len(), 1);
    assert_eq!(witness.steps[0].delay.as_ref(), Some(&ratio(1, 1)));
    let workload_name = match &claim.scheduler {
        ProbScheduler::Workload(name) => name,
        ProbScheduler::Every => panic!("the claim names its workload"),
    };
    let workload = world
        .package
        .graph()
        .probabilistic_model()
        .workload(workload_name)
        .expect("the workload checks");
    let cancel = Cancel::new();
    let mut reader = witness_reader(&system, &world.package, claim, &sampler(), &cancel)
        .expect("the timed claim is readable");
    let replay = WitnessReplay {
        system: &system,
        timed: Some(&timed),
        workload,
        provenance: &verdict.provenance,
        cancel: &cancel,
    };
    assert_eq!(
        replay_sampled_witness(&replay, witness, &mut reader),
        Ok(fsl_replay::statistical::SampledWitnessReplay::Reproduced)
    );
    let mut altered = witness.clone();
    altered.steps[0].delay = Some(ratio(2, 1));
    assert!(matches!(
        replay_sampled_witness(&replay, &altered, &mut reader),
        Ok(fsl_replay::statistical::SampledWitnessReplay::Inconclusive(
            _
        ))
    ));
}

/// FR-254-AC-1/2 (TC-709): the exact 1/20 late probability is rejected by
/// the 1% bound, while uniform delay is accepted; runs replay with the
/// recorded grid and detect an altered delay.
#[trace("TC-709", "FR-254-AC-1", "FR-254-AC-2")]
#[test]
fn timed_event_sampling_is_reproducible_and_replays_exact_grid_delays() {
    let discrete = timed_late_world(
        "delay M::Call::finish ~ discrete { 1 ms: 1 };",
        "probability <= 0.01",
        1,
    );
    let run_a = completed(run_timed_method(
        &discrete,
        "RareLate",
        StatisticalMethod::Sprt,
        7,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    ));
    let run_b = completed(run_timed_method(
        &discrete,
        "RareLate",
        StatisticalMethod::Sprt,
        7,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    ));
    assert_eq!(run_a.decision, Decision::Rejected);
    assert_eq!(run_a.basis.delay_grid_bits, Some(64));
    assert_eq!(run_a.decision, run_b.decision);
    assert_eq!(run_a.tests, run_b.tests);

    let uniform = timed_late_world(
        "delay M::Call::finish ~ uniform[1 ms, 3 ms];",
        "probability <= 0.01",
        1,
    );
    let accepted = completed(run_timed_method(
        &uniform,
        "RareLate",
        StatisticalMethod::Sprt,
        7,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    ));
    assert_eq!(accepted.decision, Decision::Accepted);
    assert_eq!(accepted.basis.delay_grid_bits, Some(64));

    let quantized_world = timed_late_world(
        "delay M::Call::finish ~ uniform[1 ms, 3 ms];",
        "probability >= 0.5",
        3,
    );
    let quantized = completed(run_timed_method(
        &quantized_world,
        "RareLate",
        StatisticalMethod::Sprt,
        7,
        fsl_replay::statistical::TimedSampling {
            q: 8,
            max_sample_steps: 100,
        },
        1,
    ));
    assert_eq!(quantized.basis.delay_grid_bits, Some(8));
    let witness = quantized.witnesses.first().expect("a sampled witness");
    assert!(witness.steps.iter().all(|step| {
        step.delay
            .as_ref()
            .is_some_and(|delay| delay.mul(&ratio(256, 1)).denominator() == &Integer::one())
    }));

    let system = quantized_world.system();
    let timed = TimedSystem::new(&system, "M").expect("a timed subject");
    let claim = quantized_world
        .package
        .graph()
        .probabilistic_model()
        .claim("RareLate")
        .expect("the timed claim checks");
    let workload_name = match &claim.scheduler {
        ProbScheduler::Workload(name) => name,
        ProbScheduler::Every => panic!("the claim names its workload"),
    };
    let workload = quantized_world
        .package
        .graph()
        .probabilistic_model()
        .workload(workload_name)
        .expect("the workload checks");
    let cancel = Cancel::new();
    let mut reader = witness_reader_with_timed_sampling(
        &system,
        &quantized_world.package,
        claim,
        &sampler(),
        fsl_replay::statistical::TimedSampling {
            q: 8,
            max_sample_steps: 100,
        },
        &cancel,
    )
    .expect("the timed claim is readable");
    let replay = WitnessReplay {
        system: &system,
        timed: Some(&timed),
        workload,
        provenance: &quantized.provenance,
        cancel: &cancel,
    };
    assert_eq!(
        replay_sampled_witness(&replay, witness, &mut reader),
        Ok(fsl_replay::statistical::SampledWitnessReplay::Reproduced)
    );
    let mut altered = witness.clone();
    altered.steps[0].delay = Some(ratio(999, 1));
    assert!(matches!(
        replay_sampled_witness(&replay, &altered, &mut reader),
        Ok(fsl_replay::statistical::SampledWitnessReplay::Inconclusive(
            _
        ))
    ));
}

/// FR-254-AC-3 (TC-709): a Boolean branch uses the longest timed path as its
/// horizon. A free-delay operation enabled only after that horizon must not
/// change the bounded event's result.
#[trace("TC-709", "FR-254-AC-3")]
#[test]
fn branching_timed_formula_stops_at_the_longest_path_horizon() {
    let world = timed_world_with(
        &[("go", &["phase"]), ("free", &["phase"])],
        "pre Start using v on M::Call::go { self.phase = 0 }\n\
         post Done using v on M::Call::go { self.phase = 2 }\n\
         pre Free using v on M::Call::free { self.phase = 2 }\n\
         post FreeAgain using v on M::Call::free { self.phase = 2 }\n\
         delay M::Call::go ~ discrete { 4 ms: 1 };\n\
         workload Even on M { weight Call::go = 1; weight Call::free = 1; }\n\
         probabilistic BranchBound using ev on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             { probability <= 0.01 [ (eventually[0 ms, 1 ms] holds(c.phase = 2)) or\n\
                 (eventually[0 ms, 3 ms] holds(c.phase = 2)) ] }\n",
    );
    let claim = world
        .package
        .graph()
        .probabilistic_model()
        .claim("BranchBound")
        .expect("the bounded timed claim checks");
    let ProbForm::Probability {
        event: Event::Formula {
            timed_horizon: Some(horizon),
            ..
        },
        ..
    } = &claim.form
    else {
        panic!("the claim has a timed formula horizon: {claim:?}");
    };
    assert_eq!(horizon, &ratio(3, 1));

    let outcome = run_timed_method(
        &world,
        "BranchBound",
        StatisticalMethod::Sprt,
        7,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    );
    let verdict = match outcome {
        Ok(StatisticalOutcome::Completed(verdict)) => verdict,
        Ok(other) => panic!("the bounded event is decided: {other:?}"),
        Err(refusal) => panic!("post-horizon free delay is not sampled: {refusal:?}"),
    };
    assert_eq!(verdict.decision, Decision::Accepted);
}

/// FR-254-AC-3 (TC-709): a timed regenerative run that never returns to its
/// initial state stops at the configured `max_cycle_steps` limit.
#[trace("TC-709", "FR-254-AC-3")]
#[test]
fn timed_regeneration_names_the_max_cycle_steps_limit() {
    let world = timed_world_with(
        &[("go", &["phase"]), ("tick", &["phase"])],
        "pre Start using v on M::Call::go { self.phase = 0 }\n\
         post Entered using v on M::Call::go { self.phase = 1 }\n\
         pre Running using v on M::Call::tick { self.phase = 1 }\n\
         post Ticked using v on M::Call::tick { self.phase = 1 }\n\
         delay M::Call::go ~ discrete { 0 ms: 1 };\n\
         delay M::Call::tick ~ discrete { 1 ms: 1 };\n\
         workload Even on M { weight Call::go = 1; weight Call::tick = 1; }\n\
         probabilistic NoReturn using ev on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             { long_run_fraction holds(c.phase = 1) >= 0.5 }\n",
    );
    let outcome = run_timed_method_with_limits(
        &world,
        "NoReturn",
        StatisticalMethod::Regenerative { min_cycles: 1 },
        7,
        fsl_replay::statistical::TimedSampling::default(),
        StatisticalLimits {
            max_cycle_steps: 2,
            ..StatisticalLimits::default()
        },
        1,
    )
    .expect("the timed regenerative walk reaches its cycle budget");
    assert!(matches!(
        outcome,
        StatisticalOutcome::Stopped {
            stop: StatisticalStop::Limit {
                limit: StatisticalLimit::MaxCycleSteps,
                value: 2,
            },
            ..
        }
    ));
}

/// FR-254-AC-3 (TC-709): an enabled transition without a delay law refuses
/// the first race that includes it, preserving the state and operation.
#[trace("TC-709", "FR-254-AC-3")]
#[test]
fn timed_sampling_refuses_the_first_free_delay_race() {
    let world = timed_world_with(
        &[
            ("send", &["phase"]),
            ("reply", &["phase"]),
            ("timeout", &["phase", "late"]),
        ],
        "pre Waiting using v on M::Call::send { self.phase = 0 }\n\
         post Sent using v on M::Call::send { self.phase = 1 }\n\
         pre Reply using v on M::Call::reply { self.phase = 1 }\n\
         post Replied using v on M::Call::reply { self.phase = 2 }\n\
         pre Timeout using v on M::Call::timeout { self.phase = 1 }\n\
         post TimedOut using v on M::Call::timeout { self.phase = 3 and self.late }\n\
         delay M::Call::send ~ discrete { 0 ms: 1 };\n\
         delay M::Call::reply ~ discrete { 1 ms: 1 };\n\
         workload Even on M { weight Call::send = 1; weight Call::reply = 1; \
             weight Call::timeout = 1; }\n\
         probabilistic RareLate using ev on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.005\n\
             { probability <= 0.01 [ eventually[0 ms, 3 ms] holds(c.late) ] }\n",
    );
    let outcome = run_timed_method(
        &world,
        "RareLate",
        StatisticalMethod::Sprt,
        7,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    )
    .expect("a free delay is an unsupported sampling outcome");
    let StatisticalOutcome::Unsupported {
        cause: StatisticalUnsupported::NotStochastic { state: _, identity },
        ..
    } = outcome
    else {
        panic!("the first free-delay race is unsupported: {outcome:?}");
    };
    assert!(format!("{identity:?}").contains("timeout"));
}

/// FR-254-AC-3 (TC-709): the caller's per-sample step budget reaches the
/// statistical limit settlement with its configured value.
#[trace("TC-709", "FR-254-AC-3")]
#[test]
fn timed_sampling_settles_the_configured_step_limit() {
    let world = timed_world_with(
        &[("send", &["phase"]), ("reply", &["phase"])],
        "pre Waiting using v on M::Call::send { self.phase = 0 }\n\
         post Sent using v on M::Call::send { self.phase = 1 }\n\
         pre Reply using v on M::Call::reply { self.phase = 1 }\n\
         post Replied using v on M::Call::reply { self.phase = 2 }\n\
         delay M::Call::send ~ discrete { 0 ms: 1 };\n\
         delay M::Call::reply ~ discrete { 1 ms: 1 };\n\
         workload Even on M { weight Call::send = 1; weight Call::reply = 1; }\n\
         probabilistic Replied using ev on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.1\n\
             { probability >= 0.5 [ eventually[0 ms, 3 ms] holds(c.phase = 2) ] }\n",
    );
    let outcome = run_timed_method(
        &world,
        "Replied",
        StatisticalMethod::Sprt,
        7,
        fsl_replay::statistical::TimedSampling {
            q: 64,
            max_sample_steps: 1,
        },
        1,
    )
    .expect("a bounded sample stops the request");
    assert!(matches!(
        outcome,
        StatisticalOutcome::Stopped {
            stop: StatisticalStop::Limit {
                limit: StatisticalLimit::MaxSampleSteps { value: 1 },
                value: 1,
            },
            ..
        }
    ));
}

/// FR-254-AC-4 (TC-709): reaching a local time-lock before the horizon makes
/// the run unsupported, with no witness retained.
#[trace("TC-709", "FR-254-AC-4")]
#[test]
fn timed_sampling_settles_a_local_time_lock_without_a_witness() {
    let world = timed_world_with(
        &[("go", &["phase"])],
        "pre Waiting using v on M::Call::go { self.phase = 0 }\n\
         post Locked using v on M::Call::go { self.phase = 1 }\n\
         time invariant Lock when { self.phase = 1 } { self.x <= 1 ms }\n\
         delay M::Call::go ~ discrete { rational(1, 2) ms: 1 };\n\
         workload Even on M { weight Call::go = 1; }\n\
         probabilistic Goes using ev on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.1\n\
             { probability >= 0.5 [ eventually[0 ms, 3 ms] holds(c.phase = 1) ] }\n",
    );
    let outcome = run_timed_method(
        &world,
        "Goes",
        StatisticalMethod::Sprt,
        7,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    )
    .expect("a local time-lock is an unsupported sampling outcome");
    assert!(matches!(
        outcome,
        StatisticalOutcome::Unsupported {
            cause: StatisticalUnsupported::TimeLockedSample { .. },
            ..
        }
    ));
}

/// Checks FR-193-AC-1 (TC-628 step 1): the `2 ms` claim with seed 7 and
/// `max_witnesses` 3 keeps three witnesses in increasing trace-index order,
/// each with latency above `2 ms` and the product of its steps' FR-187
/// probabilities; in one whose attempt drew `d = 3`, `ok`, the `request`
/// step has probability 1 and the `attempt` step `343/5000`.
#[trace("TC-628", "FR-193-AC-1")]
#[test]
fn the_two_ms_claim_keeps_its_first_violating_behaviours() {
    let world = service();
    let verdict = completed(run(
        &world,
        "Two",
        StatisticalMethod::Okamoto,
        StatisticalLimits::default(),
        3,
    ));
    assert_eq!(verdict.witnesses.len(), 3);
    let traces: Vec<u64> = verdict
        .witnesses
        .iter()
        .map(|witness| witness.trace)
        .collect();
    assert!(
        traces.windows(2).all(|pair| pair[0] < pair[1]),
        "{traces:?}"
    );
    let mut three_ok = 0;
    for witness in &verdict.witnesses {
        match &witness.value {
            WitnessValue::Measure(MeasureValue::Value(latency)) => {
                assert!(*latency > ratio(2, 1), "{latency:?}");
            }
            WitnessValue::Measure(MeasureValue::Censored) => {}
            other => panic!("a measured witness: {other:?}"),
        }
        let product = witness
            .steps
            .iter()
            .fold(ratio(1, 1), |product, step| product.mul(&step.probability));
        assert_eq!(witness.path_probability, product);
        let [request, attempt, ..] = &witness.steps[..] else {
            panic!("a request and an attempt: {:?}", witness.steps);
        };
        assert_eq!(request.probability, ratio(1, 1));
        if integer_of(&attempt.random[0]) == 3 && matches!(attempt.random[1], Value::Boolean(true))
        {
            assert_eq!(attempt.probability, ratio(343, 5000));
            three_ok += 1;
        }
    }
    assert!(three_ok > 0, "a witness whose attempt drew 3 ms and ok");
}

/// FR-192 "estimate": a quantile claim reports, beside `Pr(M <= c | activated)`,
/// the empirical nearest-rank quantile of the counted measures. Over
/// `Service` the exact distribution of the latency is `Pr(M <= 2) = 0.898`
/// and `Pr(M <= 3) = 0.967`, so the `0.95`-quantile is 3 ms; a claim of
/// another form reports none.
#[trace("TC-624")]
#[test]
fn a_quantile_claim_reports_its_empirical_quantile() {
    let world = service();
    let verdict = decided(&world, "P95", StatisticalMethod::Okamoto);
    let three = Some(MeasureValue::Value(ratio(3, 1)));
    assert_eq!(verdict.tests[0].quantile, three);
    assert_eq!(verdict.quantile, three);
    // FR-410: the verdict's bound is the claim as written, with its level.
    assert_eq!(
        serde_json::to_value(fsl_replay::shared_wire::ProbabilisticBound::from(
            &verdict.bound
        ))
        .expect("writes"),
        json!({"type": "quantile", "comparison": "less-or-equal",
            "threshold": {"numerator": "5", "denominator": "1"},
            "level": {"numerator": "19", "denominator": "20"}})
    );

    let mean = "probabilistic Mean using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.1, beta 0.1, indifference 0.1 \
         { mean of (fraction holds(s.phase = 0) over [0, 10]) >= 0.2 }";
    let world = World::new(&prob_world::service(mean));
    let verdict = decided(&world, "Mean", StatisticalMethod::Okamoto);
    assert_eq!(verdict.quantile, None);
}

/// FR-189-AC-15: quantity-valued random parameters cross the model transition
/// identity and reward paths with their unit, and measured values retain the
/// exact magnitude in the checked reward unit.
#[trace("TC-624", "FR-189-AC-15")]
#[test]
fn quantity_parameters_are_encoded_and_measured_as_exact_unit_magnitudes() {
    let claim = "probabilistic Q using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         { quantile 0.75 of (accumulate duration from holds(s.phase = 1) \
         until holds(s.phase = 2) within 10) <= 0.005 s }";
    let decimal = World::new(&prob_world::quantity_service("1 ms: 1, 1.50 ms: 1", claim));
    let rational = World::new(&prob_world::quantity_service(
        "1 ms: 1, rational(3, 2) ms: 1",
        claim,
    ));

    let check = |world: &World| {
        completed(run(
            world,
            "Q",
            StatisticalMethod::Sprt,
            StatisticalLimits::default(),
            DEFAULT_MAX_WITNESSES,
        ))
    };
    let decimal = check(&decimal);
    let rational = check(&rational);
    let expected = Some(MeasureValue::Value(ratio(3, 2)));
    assert_eq!(decimal.quantile, expected, "{decimal:?}");
    assert_eq!(rational.quantile, expected, "{rational:?}");
    assert_eq!(decimal.tests[0].samples, rational.tests[0].samples);
}

/// FR-189-AC-15 / FR-185-AC-8: a support written in another unit is rejected
/// by S3 before a checked subject can reach the sampler.
#[trace("TC-624", "FR-189-AC-15")]
#[test]
fn a_quantity_support_in_another_unit_is_refused_before_sampling() {
    let claim = "probabilistic Q using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         { quantile 0.75 of (accumulate duration from holds(s.phase = 1) \
         until holds(s.phase = 2) within 10) <= 5 ms }";
    let refusals = match World::try_new(&prob_world::quantity_service("1 s: 1", claim)) {
        Ok(_) => panic!("a support in seconds does not have the parameter's ms type"),
        Err(refusals) => refusals,
    };
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].cause.code(), Code::InvalidProbabilisticModel);
    assert_eq!(
        refusals[0].cause.catalog_code().map(|code| code.cause()),
        Some("value-outside-type")
    );
}

/// Checks FR-189-AC-4 (TC-624 step 4), the activation clause: over a
/// `Service` whose `request` starts the server with probability `1/2`, an
/// Okamoto `P95` counts only activated samples, `23,026` of them, and draws
/// more.
#[trace("TC-624")]
#[test]
fn a_quantile_counts_only_activated_samples() {
    let world = World::new(&prob_world::activated_service(&p95("P95", 5)));
    let verdict = decided(&world, "P95", StatisticalMethod::Okamoto);
    assert_eq!(verdict.tests[0].samples, 23_026);
    assert!(verdict.tests[0].draws > verdict.tests[0].samples);
}

/// FR-189-AC-12's stop (TC-624 step 9), with an activation predicate that
/// never holds in place of an activation of probability 0, which a weight
/// cannot state: with `max_draws` 30,000 (above the 23,026 samples Okamoto
/// needs), stops `MaxDraws` with no sample counted.
#[trace("TC-624")]
#[test]
fn a_quantile_with_no_activation_stops_on_max_draws() {
    let never = "probabilistic Never using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         { quantile 0.95 of (accumulate duration from holds(s.phase = 1 and s.phase = 2) \
         until holds(s.phase = 2) within 10) <= 5 }";
    let world = World::new(&prob_world::activated_service(never));
    let outcome = run(
        &world,
        "Never",
        StatisticalMethod::Okamoto,
        StatisticalLimits {
            max_draws: 30_000,
            ..StatisticalLimits::default()
        },
        DEFAULT_MAX_WITNESSES,
    );
    let Ok(StatisticalOutcome::Stopped { stop, partial, .. }) = outcome else {
        panic!("a stopped run: {outcome:?}");
    };
    assert_eq!(
        stop,
        StatisticalStop::Limit {
            limit: StatisticalLimit::MaxDraws,
            value: 30_000
        }
    );
    assert_eq!(partial.samples, 0);
    assert_eq!(partial.draws, 30_000);
}

/// Checks FR-189-AC-5 (TC-624 step 5), the `NotMarkov` clause: a `Health`
/// whose `tick` leaves `spare` unconstrained has two post-states per draw,
/// and the run returns `Unsupported` naming the state, the scheduled
/// identity, the draw and the two post-states.
#[trace("TC-624")]
#[test]
fn a_non_unique_post_state_is_unsupported_with_its_step() {
    let claim = "probabilistic Steadily using ev on M over (n: M::Node) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,3] holds(n.healthy) ] }";
    let world = World::new(&prob_world::health(
        "self.healthy = (pre(self.healthy) and not fault)",
        "true: 1, false: 9",
        true,
        claim,
    ));
    let outcome = run(
        &world,
        "Steadily",
        StatisticalMethod::Okamoto,
        StatisticalLimits::default(),
        DEFAULT_MAX_WITNESSES,
    );
    let Ok(StatisticalOutcome::Unsupported {
        cause: StatisticalUnsupported::NotMarkov(cause),
        provenance,
    }) = outcome
    else {
        panic!("an unsupported run: {outcome:?}");
    };
    assert_eq!(cause.post_states, 2);
    assert!(
        cause.transition.operation.contains("Node/tick"),
        "{cause:?}"
    );
    assert_eq!(cause.random.len(), 1);
    assert_eq!(provenance.tests.len(), 1);
}

/// Checks FR-189-AC-6 (TC-640 step 1) and FR-193-AC-4's keeping clause:
/// over `Coin`, `always[0,3] holds(c.v = 0)` is Rejected at the first sample
/// whose path drops the coin with `b = 0`, no later sample is drawn, and the
/// test's `undefined` names test 0, that sample's trace index and the first
/// position after the drop, `absent-key`; the sample is kept as an undefined
/// witness.
#[trace("TC-640", "TC-627", "FR-189-AC-6")]
#[test]
fn a_dropped_coin_rejects_its_test_at_the_undefined_position() {
    let claim = "probabilistic Kept using ev on M over (c: M::Coin) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,3] holds(c.v = 0) ] }";
    let world = World::new(&prob_world::coin(claim));
    let verdict = decided(&world, "Kept", StatisticalMethod::Okamoto);
    assert_eq!(verdict.decision, Decision::Rejected);
    // FR-410: a rejection by `undefined-evaluation` still carries its bound.
    assert_eq!(
        serde_json::to_value(fsl_replay::shared_wire::ProbabilisticBound::from(
            &verdict.bound
        ))
        .expect("writes"),
        json!({"type": "probability", "comparison": "greater-or-equal",
            "threshold": {"numerator": "1", "denominator": "2"}})
    );
    let [test] = &verdict.tests[..] else {
        panic!("one test: {:?}", verdict.tests);
    };
    assert_eq!(test.decision, Decision::Rejected);
    assert_eq!(
        (test.draws, test.samples),
        (test.samples + 1, test.samples),
        "the undefined sample is a draw but not a counted sample"
    );
    let undefined = test.undefined.as_ref().expect("an undefined sample");
    assert_eq!(undefined.test, 0);
    // The run stops at this sample: it is the test's last draw.
    assert_eq!(undefined.trace, test.draws - 1);
    let witness = verdict.witnesses.first().expect("the undefined witness");
    assert_eq!(witness.trace, undefined.trace);
    assert_eq!(
        witness.value,
        WitnessValue::Undefined(undefined.evaluation.clone())
    );
    let drop = witness.steps.last().expect("the drop");
    assert!(matches!(drop.transition, TransitionId::Operation(_)));
    assert_eq!(integer_of(&drop.random[0]), 0);
    for kept in &witness.steps[..witness.steps.len() - 1] {
        assert_eq!(
            integer_of(&kept.random[0]),
            1,
            "an earlier drop keeps the coin"
        );
    }
    let site = &undefined.evaluation.site;
    assert_eq!(
        site.position,
        u64::try_from(witness.steps.len()).expect("small")
    );
    assert!(matches!(site.locus, UndefinedLocus::Atom(_)));
    let UndefinedCause::Family(record) = &undefined.evaluation.cause else {
        panic!("absent-key: {:?}", undefined.evaluation.cause);
    };
    assert_eq!(record.reason, UndefinedReason::AbsentKey);
}

/// Checks FR-189-AC-5 (TC-624 step 5), the undecided-successor clause: over
/// a cell whose `inc` postcondition divides by zero at a candidate
/// post-state, the contract conjunction is undecided at the first state, so
/// the test is Undecided, `UndecidedSuccessor`, with no interval and no
/// counted sample, and the claim is Undecided.
#[trace("TC-624", "FR-189-AC-5")]
#[test]
fn an_undecided_contract_conjunction_leaves_its_test_undecided() {
    let claim = "probabilistic Safe using ev on M over (c: M::Cell) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,3] holds(c.v >= 0) ] }";
    let world = World::new_with_mode(
        &prob_world::undetermined(claim),
        filament_semantic_value::checking::CheckMode::Kernel,
    );
    let verdict = decided(&world, "Safe", StatisticalMethod::Okamoto);
    let undecided = Decision::Undecided(UndecidedCause::UndecidedSuccessor);
    assert_eq!(verdict.decision, undecided);
    let [test] = &verdict.tests[..] else {
        panic!("one test: {:?}", verdict.tests);
    };
    assert_eq!(test.decision, undecided);
    assert_eq!((test.samples, test.interval.clone()), (0, None));
    assert!(verdict.witnesses.is_empty());
}

/// Checks FR-189-AC-7 (TC-624 step 7): over `Wide`, whose `spin` draws from
/// weights totalling `2^256 + 1`, the first sample stops the run
/// `Unsupported` with `TotalAboveDrawRange{total}`, `total` `2^256 + 1`, and
/// settles `unsupported`, `total-above-draw-range`.
#[trace("TC-624", "FR-189-AC-7")]
#[test]
fn a_draw_above_the_range_is_unsupported() {
    let claim = "probabilistic Spun using ev on M over (w: M::Wide) under Unit on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,3] holds(w.v or not w.v) ] }";
    let world = World::new(&prob_world::wide(claim));
    let outcome = run(
        &world,
        "Spun",
        StatisticalMethod::Okamoto,
        StatisticalLimits::default(),
        DEFAULT_MAX_WITNESSES,
    );
    let Ok(unsupported) = outcome else {
        panic!("an outcome: {outcome:?}");
    };
    let record = settle_statistical(
        &unsupported,
        SubjectLimits {
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: &prob_world::UNLIMITED,
        },
    );
    let TerminalValue::Unsupported(cause) = &record.value else {
        panic!("settles unsupported: {record:?}");
    };
    assert_eq!(cause.as_str(), "total-above-draw-range");
    let StatisticalOutcome::Unsupported {
        cause: StatisticalUnsupported::TotalAboveDrawRange { total },
        provenance,
    } = unsupported
    else {
        panic!("an unsupported run: {unsupported:?}");
    };
    assert_eq!(
        total.to_string(),
        "115792089237316195423570985008687907853269984665640564039457584007913129639937"
    );
    assert_eq!(provenance.tests[0].traces.count, 1);
}

/// FR-192-AC-4's gate clauses (TC-627 step 4): through `check_statistical`,
/// a claim under every scheduler, one with no confidence parameters and an
/// expected-reward form are each refused with its cause before the subject
/// is read.
#[trace("TC-627", "FR-192-AC-4")]
#[test]
fn exact_only_claims_are_refused_before_any_sample() {
    let claims = "probabilistic Every using ev on M over (s: M::Server) under every scheduler \
         on origin with alpha 0.01, beta 0.01, indifference 0.01 \
         { probability >= 0.5 [ eventually[0,3] holds(s.phase = 2) ] }\n\
         probabilistic Bare using ev on M over (s: M::Server) under Steady on origin \
         { probability >= 0.5 [ eventually[0,3] holds(s.phase = 2) ] }\n\
         probabilistic Expected using inf on M over (s: M::Server) under Steady on origin \
         { expected accumulate duration until holds(s.phase = 2) <= 5 }";
    let world = World::new(&prob_world::service(claims));
    for (claim, refusal_expected) in [
        ("Every", StatisticalRefusal::EveryScheduler),
        ("Bare", StatisticalRefusal::MissingConfidence),
        ("Expected", StatisticalRefusal::ExactOnlyForm),
    ] {
        let refusal = run(
            &world,
            claim,
            StatisticalMethod::Okamoto,
            StatisticalLimits::default(),
            DEFAULT_MAX_WITNESSES,
        )
        .expect_err("the gate refuses before a sample");
        assert_eq!(refusal, refusal_expected);
        let record =
            settle_statistical_refusal(&refusal).expect("a gate refusal has a terminal settlement");
        let TerminalValue::Unsupported(cause) = record.value else {
            panic!("an exact-only gate refusal settles unsupported: {record:?}");
        };
        assert_eq!(
            cause.as_str(),
            match claim {
                "Every" => "every-scheduler",
                "Bare" => "missing-confidence",
                "Expected" => "exact-only-form",
                _ => unreachable!("listed claim"),
            }
        );
    }
}

/// FR-189-AC-11 and FR-192-AC-1: a subject with no initial-state/binding
/// pair is refused before sampling and settles as inconclusive
/// `no-initial-state`, never as a measured acceptance.
#[trace("TC-627", "FR-189-AC-11", "FR-192-AC-1")]
#[test]
fn a_subject_without_an_initial_binding_pair_is_inconclusive() {
    let model = prob_world::service(&p95("P95", 5));
    let world = World::with_initials(&model, &[]);
    let refusal = run(
        &world,
        "P95",
        StatisticalMethod::Okamoto,
        StatisticalLimits::default(),
        DEFAULT_MAX_WITNESSES,
    )
    .expect_err("there is no test to sample");
    assert_eq!(refusal, StatisticalRefusal::NoTest);
    let record = settle_statistical_refusal(&refusal).expect("NoTest settles");
    assert_eq!(
        record.value,
        TerminalValue::Inconclusive(fsl_replay::InconclusiveCause::NoInitialState)
    );
    assert_eq!(record.measured(), None);
}

/// FR-189 "Methods" and FR-190: over `Service` a mean of a fraction is
/// decided by Okamoto over the windowed fraction of `Idle` positions; over a
/// `Health` whose `tick` sets `healthy` to a fair coin, a long-run fraction
/// is decided by the regenerative method, which keeps no witness
/// (FR-193-AC-3's `LongRun` clause).
#[trace("TC-624", "TC-625", "TC-627", "FR-192-AC-6")]
#[test]
fn fractions_are_decided_over_the_models_behaviours() {
    let mean = "probabilistic Mean using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.1, beta 0.1, indifference 0.1 \
         { mean of (fraction holds(s.phase = 0) over [0, 10]) >= 0.2 }";
    let world = World::new(&prob_world::service(mean));
    let mean = decided(&world, "Mean", StatisticalMethod::Okamoto);
    assert_eq!(mean.decision, Decision::Accepted);
    // `N = ⌈ln 10 / (2 · 1/100)⌉`.
    assert_eq!(mean.tests[0].samples, 116);
    // FR-192-AC-6: a mean of a fraction carries no `quantile`.
    assert_eq!(mean.quantile, None);
    let healthy = "probabilistic Up using ev on M over (n: M::Node) under Steady on origin \
         with alpha 0.1, beta 0.1, indifference 0.1 \
         { long_run_fraction holds(n.healthy) >= 0.3 }";
    let world = World::new(&prob_world::health(
        "self.healthy = (not fault)",
        "true: 1, false: 1",
        false,
        healthy,
    ));
    let long_run = decided(
        &world,
        "Up",
        StatisticalMethod::Regenerative { min_cycles: 10 },
    );
    assert_eq!(long_run.decision, Decision::Accepted);
    let estimate = long_run.estimate.clone().expect("a ratio estimate");
    let (low, high) = long_run.interval.clone().expect("a regenerative interval");
    assert!(
        low >= ratio(3, 10) && low < estimate && estimate < high,
        "{low:?} {estimate:?} {high:?}"
    );
    assert!(long_run.tests[0].samples >= 10, "min_cycles cycles closed");
    assert!(long_run.witnesses.is_empty());
    // FR-192-AC-6: a long-run fraction carries no `quantile`.
    assert_eq!(long_run.quantile, None);
}

/// A declaration that the servers `a` and `b` of `Service` are interchangeable.
fn interchangeable() -> Vec<SymmetryDeclaration> {
    vec![SymmetryDeclaration {
        population: DeclarationKey {
            package: "test/service".to_owned(),
            node: "ix://test/service/servers".to_owned(),
        },
        classes: vec![vec!["a".to_owned(), "b".to_owned()]],
    }]
}

fn busy_claim() -> String {
    "probabilistic Busy using ev on M over (s: M::Server) under Steady on origin \
     with alpha 0.1, beta 0.1, indifference 0.1 \
     { probability >= 0.5 [ eventually[0,10] holds(s.phase = 1) ] }"
        .to_owned()
}

/// Checks FR-189-AC-3 (TC-624 step 3), the symmetry clause: two servers
/// that a declaration makes interchangeable, from one initial state, run
/// one test, with `α' = α` where two pairs would run two; without the
/// declaration, or over a population not declared `symmetric`, they run two,
/// and a declaration the population does not admit is refused.
#[trace("TC-624", "FR-189-AC-3")]
#[test]
fn two_symmetric_servers_run_one_test() {
    let claim = busy_claim();
    let symmetric = World::symmetric_objects(&prob_world::service(&claim), &["a", "b"]);
    fn request(world: &World, symmetry: Vec<SymmetryDeclaration>) -> StatisticalRequest<'_> {
        StatisticalRequest {
            workers: 1,
            subject: world.subject(),
            claim: world.claim("Busy"),
            method: StatisticalMethod::Okamoto,
            seed: 7,
            sampler: sampler(),
            limits: StatisticalLimits::default(),
            max_witnesses: DEFAULT_MAX_WITNESSES,
            symmetry,
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        }
    }
    let verdict = completed(check_statistical(
        request(&symmetric, interchangeable()),
        &Cancel::new(),
    ));
    assert_eq!(verdict.basis.tests, 1);
    assert_eq!(verdict.tests.len(), 1);
    assert_eq!(verdict.basis.alpha_per_test, ratio(1, 10));
    // Okamoto at `m = 1`: `N = ⌈ln 10 / (2 · 1/100)⌉`.
    assert_eq!(verdict.tests[0].samples, 116);

    let plain = completed(check_statistical(
        request(&symmetric, Vec::new()),
        &Cancel::new(),
    ));
    assert_eq!(plain.basis.tests, 2);
    assert_eq!(plain.basis.alpha_per_test, ratio(1, 20));

    let ordinary = World::with_objects(&prob_world::service(&claim), &["a", "b"]);
    let refusal = check_statistical(request(&ordinary, interchangeable()), &Cancel::new());
    let Err(refusal) = refusal else {
        panic!("a population not declared symmetric is refused: {refusal:?}");
    };
    assert!(matches!(
        refusal,
        StatisticalRefusal::Symmetry(SymmetryRefusal::NotSymmetric { declaration: 0, .. })
    ));
    assert!(settle_statistical_refusal(&refusal).is_err());
}

/// Checks FR-189-AC-3's symmetry clause under each workload: workload
/// weights are per operation, so two interchangeable servers run `m = 1`
/// both under a workload that weights each operation equally and under one
/// whose operation weights differ from one another.
#[trace("TC-624", "FR-189-AC-3")]
#[test]
fn two_symmetric_servers_run_one_test_under_any_operation_weights() {
    let claims = |workload: &str| {
        format!(
            "workload Skew on M {{ weight Server::request = 3; weight Server::attempt = 1; \
                 weight Server::reset = 2; }}\n\
             probabilistic Busy using ev on M over (s: M::Server) under {workload} on origin \
             with alpha 0.1, beta 0.1, indifference 0.1 \
             {{ probability >= 0.5 [ eventually[0,10] holds(s.phase = 1) ] }}"
        )
    };
    for workload in ["Steady", "Skew"] {
        let world = World::symmetric_objects(&prob_world::service(&claims(workload)), &["a", "b"]);
        let verdict = completed(check_statistical(
            StatisticalRequest {
                workers: 1,
                subject: world.subject(),
                claim: world.claim("Busy"),
                method: StatisticalMethod::Okamoto,
                seed: 7,
                sampler: sampler(),
                limits: StatisticalLimits::default(),
                max_witnesses: DEFAULT_MAX_WITNESSES,
                symmetry: interchangeable(),
                timed_sampling: fsl_replay::statistical::TimedSampling::default(),
            },
            &Cancel::new(),
        ));
        assert_eq!(verdict.basis.tests, 1, "{workload}");
        assert_eq!(verdict.tests.len(), 1, "{workload}");
    }
}

/// Checks FR-189-AC-3's symmetry clause across initial states: two initial
/// states that are images of each other under the class (`a` busy and `b`
/// idle, then the reverse), with an `over` binding, are four pairs in two
/// orbits, `(s1, a) ~ (s2, b)` and `(s1, b) ~ (s2, a)`. EN-4 runs `m = 2`
/// tests, each on the least pair of its orbit, so both start at the same
/// initial state and differ by binding; without the declaration it runs four.
#[trace("TC-624", "FR-189-AC-3")]
#[test]
fn mirrored_initial_states_merge_into_two_orbits() {
    let state =
        |phase: i64| json!({"phase": {"integer": phase.to_string()}, "attempts": {"integer": "0"}});
    let mirrored = [
        json!({"objects": {"a": state(1), "b": state(0)}}),
        json!({"objects": {"a": state(0), "b": state(1)}}),
    ];
    let world =
        World::symmetric_with_initials(&prob_world::service(&busy_claim()), &["a", "b"], &mirrored);
    let run = |symmetry| {
        completed(check_statistical(
            StatisticalRequest {
                workers: 1,
                subject: world.subject(),
                claim: world.claim("Busy"),
                method: StatisticalMethod::Okamoto,
                seed: 7,
                sampler: sampler(),
                limits: StatisticalLimits::default(),
                max_witnesses: DEFAULT_MAX_WITNESSES,
                symmetry,
                timed_sampling: fsl_replay::statistical::TimedSampling::default(),
            },
            &Cancel::new(),
        ))
    };
    let merged = run(interchangeable());
    assert_eq!(merged.basis.tests, 2);
    assert_eq!(merged.basis.alpha_per_test, ratio(1, 20));
    let [first, second] = &merged.tests[..] else {
        panic!("two tests: {:?}", merged.tests);
    };
    assert_eq!(first.initial, second.initial);
    let keys: Vec<&str> = [first, second]
        .iter()
        .map(|test| test.binding.as_ref().expect("a binding").key.as_str())
        .collect();
    assert_eq!(keys, ["a", "b"]);
    let plain = run(Vec::new());
    assert_eq!(plain.basis.tests, 4);
}

/// FR-189 "Inputs": the sampler is checked by identity before any draw.
#[trace("TC-624")]
#[test]
fn another_sampler_is_refused() {
    let world = service();
    let other = DefinitionRef::new("agent-ix", "filament.simulation.other/v1").expect("valid");
    let refusal = check_statistical(
        StatisticalRequest {
            workers: 1,
            subject: world.subject(),
            claim: world.claim("P95"),
            method: StatisticalMethod::Okamoto,
            seed: 7,
            sampler: other.clone(),
            limits: StatisticalLimits::default(),
            max_witnesses: DEFAULT_MAX_WITNESSES,
            symmetry: Vec::new(),
            timed_sampling: fsl_replay::statistical::TimedSampling::default(),
        },
        &Cancel::new(),
    );
    assert_eq!(
        refusal,
        Err(StatisticalRefusal::GeneratorMismatch { supplied: other })
    );
}

/// FR-189 "Per sample" and FR-126's closure at a terminal state (ADR-018
/// SM-4): over `Process`, terminal once `done`, `always[0,3]` of a
/// tautology fails on every sample that reaches `done` before the window's
/// last position, as EN-5's product closes it with the false extension, so its
/// probability is `(3/4)^2 = 9/16`. A sample that reaches the terminal state
/// stops there with no stutter step.
#[trace("TC-624")]
#[test]
fn a_bounded_event_closes_at_a_terminal_state() {
    let claim = "probabilistic Live using ev on M over (p: M::Proc) under Even on origin \
         with alpha 0.1, beta 0.1, indifference 0.1 \
         { probability >= 0.7 [ always[0,3] holds(p.done or not p.done) ] }";
    let world = World::new(&prob_world::process(claim));
    let verdict = completed(run(
        &world,
        "Live",
        StatisticalMethod::Okamoto,
        StatisticalLimits::default(),
        u32::MAX,
    ));
    assert_eq!(verdict.decision, Decision::Rejected);
    let estimate = verdict.estimate.clone().expect("an estimate");
    let exact = ratio(9, 16);
    assert!(
        estimate.sub(&exact) <= ratio(1, 10) && exact.sub(&estimate) <= ratio(1, 10),
        "{estimate:?}"
    );
    assert!(!verdict.witnesses.is_empty());
    for witness in &verdict.witnesses {
        assert_eq!(witness.value, WitnessValue::Event(false));
        assert!(witness.steps.len() <= 2, "{:?}", witness.steps);
        let last = witness.steps.last().expect("a step to the terminal state");
        assert!(matches!(last.transition, TransitionId::Operation(_)));
        assert!(
            matches!(last.random[..], [Value::Boolean(true)]),
            "the flip ends it"
        );
    }
}

/// A probability bound over an `accumulate` measure that never activates:
/// PF-3's probability is over every behaviour, so each sample is counted, on
/// the side that violates the bound: `>= 1/2` is Rejected with estimate 0 and
/// `<= 1/2` Rejected with estimate 1.
#[trace("TC-624")]
#[test]
fn an_unactivated_measure_under_a_probability_bound_counts_against_it() {
    let never = |name: &str, op: &str| {
        format!(
            "probabilistic {name} using ev on M over (s: M::Server) under Steady on origin \
             with alpha 0.1, beta 0.1, indifference 0.1 \
             {{ probability {op} 0.5 [ accumulate duration from holds(s.phase = 1 and s.phase = 2) \
             until holds(s.phase = 2) within 10 <= 5 ] }}\n"
        )
    };
    let world = World::new(&prob_world::service(&format!(
        "{}{}",
        never("Least", ">="),
        never("Most", "<=")
    )));
    for (claim, estimate) in [("Least", 0), ("Most", 1)] {
        let verdict = decided(&world, claim, StatisticalMethod::Okamoto);
        assert_eq!(verdict.decision, Decision::Rejected, "{claim}");
        assert_eq!(verdict.tests[0].samples, 116, "{claim}");
        assert_eq!(verdict.estimate, Some(ratio(estimate, 1)), "{claim}");
    }
}

/// A weighted fraction whose window has total weight 0 divides 0 by 0: the
/// sample is undefined at the window's last position (ADR-018 UE-1), which
/// rejects its test and is kept as the undefined witness (ADR-024 SV-11).
#[trace("TC-624")]
#[test]
fn a_weightless_fraction_is_an_undefined_sample() {
    let claim = "probabilistic Weightless using ev on M over (s: M::Server) under Steady \
         on origin with alpha 0.1, beta 0.1, indifference 0.1 \
         { mean of (fraction holds(s.phase = 0) over [0, 0] weighted by duration) >= 0.5 }";
    let world = World::new(&prob_world::service(claim));
    let verdict = decided(&world, "Weightless", StatisticalMethod::Okamoto);
    assert_eq!(verdict.decision, Decision::Rejected);
    let undefined = verdict.tests[0].undefined.as_ref().expect("undefined");
    assert_eq!(
        (undefined.trace, undefined.evaluation.site.position),
        (0, 0)
    );
    assert_eq!(
        undefined.evaluation.cause,
        UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero)
    );
    assert!(matches!(
        undefined.evaluation.site.locus,
        UndefinedLocus::ClaimPredicate(ClaimPredicate::Target)
    ));
    assert_eq!(
        verdict.witnesses[0].value,
        WitnessValue::Undefined(undefined.evaluation.clone())
    );
}

/// FR-189 "Per sample": a weighted fraction draws `h + 1` steps, since the
/// step that leaves a position weighs it. Over `[0, 1]` of `Service` the
/// walk is `request` (weight 0) then `attempt` (weight `d`, the duration):
/// position 1 (`Busy`) is weighed only by the second step, the one leaving
/// the window's last position. Drawing `h` steps would leave total weight 0
/// and every sample undefined; drawing `h + 1` makes every sample exactly
/// `d / d = 1`.
#[trace("TC-624", "FR-189-AC-1")]
#[test]
fn a_weighted_fraction_draws_the_step_leaving_its_last_position() {
    let claim = "probabilistic Leaving using ev on M over (s: M::Server) under Steady \
         on origin with alpha 0.1, beta 0.1, indifference 0.1 \
         { mean of (fraction holds(s.phase = 1) over [0, 1] weighted by duration) >= 0.5 }";
    let world = World::new(&prob_world::service(claim));
    let verdict = decided(&world, "Leaving", StatisticalMethod::Okamoto);
    assert_eq!(verdict.decision, Decision::Accepted);
    assert_eq!(verdict.tests[0].undefined, None);
    assert_eq!(verdict.estimate, Some(ratio(1, 1)));
}

/// Residence intervals are [0, 1], [1, 4], whether the next transition
/// crosses 4 or the position at 1 begins an idle tail.
#[trace("FR-254-AC-3", "FSpec-FR-407-AC-19", "FSpec-FR-407-AC-20")]
#[test]
fn delay_fractions_clip_crossing_steps_and_idle_tails() {
    for (name, delay, predicate, expected) in [
        ("Crossing", Some(5), "c.phase = 1", ratio(3, 4)),
        ("AtEnd", Some(3), "c.phase = 1", ratio(3, 4)),
        ("Idle", None, "c.phase = 0", ratio(1, 4)),
    ] {
        let reply = delay.map_or_else(String::new, |delay| {
            format!(
                "pre Reply using v on M::Call::reply {{ self.phase = 1 }}\n\
             post Replied using v on M::Call::reply {{ self.phase = 2 }}\n\
             delay M::Call::reply ~ discrete {{ {delay} ms: 1 }};\n"
            )
        });
        let operations: &[(&str, &[&str])] = if delay.is_some() {
            &[("send", &["phase"]), ("reply", &["phase"])]
        } else {
            &[("send", &["phase"])]
        };
        let reply_weight = if delay.is_some() {
            "weight Call::reply = 1;"
        } else {
            ""
        };
        let declarations = format!(
            "pre Send using v on M::Call::send {{ self.phase = 0 }}\n\
             post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
             delay M::Call::send ~ discrete {{ 1 ms: 1 }};\n\
             {reply}\n\
             workload Even on M {{ weight Call::send = 1; {reply_weight} }}\n\
             probabilistic {name} using ev on M over (c: M::Call) under Even on origin\n\
                 with alpha 0.4, beta 0.4, indifference 0.05\n\
                 {{ mean of (fraction holds({predicate}) over [0, 4 ms] weighted by delay) >= 0.5 }}\n"
        );
        let world = timed_world_with(operations, &declarations);
        let run = || {
            completed(run_timed_method(
                &world,
                name,
                StatisticalMethod::Okamoto,
                7,
                fsl_replay::statistical::TimedSampling::default(),
                1,
            ))
        };
        let verdict = run();
        assert_eq!(verdict.estimate, Some(expected), "{name}");
        assert_eq!(verdict, run(), "{name}: seeded repeat");
        let stopped = run_timed_method(
            &world,
            name,
            StatisticalMethod::Okamoto,
            7,
            fsl_replay::statistical::TimedSampling {
                q: 64,
                max_sample_steps: 0,
            },
            1,
        )
        .expect("the delay fraction honors the step budget");
        assert!(
            matches!(
                stopped,
                StatisticalOutcome::Stopped {
                    stop: StatisticalStop::Limit {
                        limit: StatisticalLimit::MaxSampleSteps { value: 0 },
                        value: 0,
                    },
                    ..
                }
            ),
            "{name}: {stopped:?}"
        );
    }
}

/// Checks FR-189-AC-3's first clause (TC-624 step 3): a `Service` with two
/// initial states runs `m = 2` tests, each with `α' = β' = 1/200`, sample
/// `i` of test `j` at trace index `2i + j`.
#[trace("TC-624")]
#[test]
fn two_initial_states_run_two_interleaved_tests() {
    let model = prob_world::service(&p95("Two", 2));
    let busy = serde_json::json!({"phase": {"integer": "1"}, "attempts": {"integer": "0"}});
    let world = World::with_initials(&model, &[model.initial.clone(), busy]);
    let verdict = completed(run(
        &world,
        "Two",
        StatisticalMethod::Okamoto,
        StatisticalLimits::default(),
        u32::MAX,
    ));
    assert_eq!(verdict.basis.tests, 2);
    assert_eq!(verdict.basis.alpha_per_test, ratio(1, 200));
    assert_eq!(verdict.basis.beta_per_test, ratio(1, 200));
    for (j, test) in (0_u64..).zip(&verdict.provenance.tests) {
        assert_eq!(u64::from(test.initial), j);
        assert_eq!((test.traces.first, test.traces.step), (j, 2));
    }
    assert!(verdict.witnesses.iter().any(|witness| witness.test == 1));
    for witness in &verdict.witnesses {
        assert_eq!(witness.trace % 2, u64::from(witness.test));
    }
}

/// FR-410, FR-413: a `quantile q of M >= c` claim written from source keeps
/// its own `greater-or-equal`, its threshold `c` and its level `q`, not the
/// strict probability bound it reduces to.
#[trace("TC-627", "FR-192-AC-2")]
#[test]
fn an_at_least_quantile_claim_writes_its_own_bound_with_its_level() {
    let claim = "probabilistic AtLeast using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         { quantile 0.5 of (accumulate duration from holds(s.phase = 1) \
         until holds(s.phase = 2) within 10) >= 2 }";
    let world = World::new(&prob_world::service(claim));
    let verdict = decided(&world, "AtLeast", StatisticalMethod::Okamoto);
    assert_eq!(
        serde_json::to_value(fsl_replay::shared_wire::ProbabilisticBound::from(
            &verdict.bound
        ))
        .expect("writes"),
        json!({"type": "quantile", "comparison": "greater-or-equal",
            "threshold": {"numerator": "2", "denominator": "1"},
            "level": {"numerator": "1", "denominator": "2"}})
    );
}

/// A two-phase model over `Call` (`phase` 0 is `Up`, 1 is `Down`) whose
/// steps each take `up` and `down` milliseconds, with `reset` the effect on
/// the clock `x` that each step writes.
fn up_down_declarations(up: u32, down: u32, claims: &str) -> String {
    format!(
        "pre Fall using v on M::Call::fall {{ self.phase = 0 }}\n\
         post Fell using v on M::Call::fall {{ self.phase = 1 and self.x = 0 ms }}\n\
         delay M::Call::fall ~ discrete {{ {up} ms: 1 }};\n\
         pre Rise using v on M::Call::rise {{ self.phase = 1 }}\n\
         post Rose using v on M::Call::rise {{ self.phase = 0 and self.x = 0 ms }}\n\
         delay M::Call::rise ~ discrete {{ {down} ms: 1 }};\n\
         workload Even on M {{ weight Call::fall = 1; weight Call::rise = 1; }}\n\
         {claims}"
    )
}

fn up_down_world(up: u32, down: u32, claims: &str) -> crate::timed_world::World {
    timed_world_with(
        &[("fall", &["phase", "x"]), ("rise", &["phase", "x"])],
        &up_down_declarations(up, down, claims),
    )
}

const UP_DOWN_CONFIDENCE: &str = "with alpha 0.1, beta 0.1, indifference 0.05";

/// FSpec FR-420-AC-9, FR-408-AC-20: `Up` lasts 1 ms and `Down` 3 ms with the
/// one clock reset by each step, so every regeneration cycle has `Y = 1` and
/// `L = 4`. The delay-weighted fraction is `1/4`, the unweighted one `1/2`;
/// `>= 1/5` is accepted and `>= 1/3` rejected, with coverage `asymptotic`.
#[trace(
    "FSpec-TC-374",
    "FSpec-FR-420-AC-9",
    "FSpec-FR-408-AC-20",
    "FSpec-FR-420-AC-10"
)]
#[test]
fn a_delay_weighted_long_run_fraction_weighs_each_cycle_by_its_delays() {
    let claims = format!(
        "probabilistic Low using ev on M over (c: M::Call) under Even on origin \
         {UP_DOWN_CONFIDENCE} {{ long_run_fraction holds(c.phase = 0) weighted by delay >= rational(1, 5) }}\n\
         probabilistic High using ev on M over (c: M::Call) under Even on origin \
         {UP_DOWN_CONFIDENCE} {{ long_run_fraction holds(c.phase = 0) weighted by delay >= rational(1, 3) }}\n\
         probabilistic Plain using ev on M over (c: M::Call) under Even on origin \
         {UP_DOWN_CONFIDENCE} {{ long_run_fraction holds(c.phase = 0) >= rational(2, 5) }}\n"
    );
    let world = up_down_world(1, 3, &claims);
    let method = StatisticalMethod::Regenerative { min_cycles: 5 };
    let mut estimates = Vec::new();
    for (name, decision) in [
        ("Low", Decision::Accepted),
        ("High", Decision::Rejected),
        ("Plain", Decision::Accepted),
    ] {
        let verdict = completed(run_timed_method(
            &world,
            name,
            method,
            7,
            fsl_replay::statistical::TimedSampling::default(),
            1,
        ));
        assert_eq!(verdict.decision, decision, "{name}");
        assert_eq!(
            verdict.basis.coverage,
            fsl_replay::statistical::Coverage::Asymptotic,
            "{name}"
        );
        let (low, high) = verdict.interval.clone().expect("an interval");
        let estimate = verdict.estimate.clone().expect("an estimate");
        assert!(
            low < estimate && estimate < high,
            "{name}: positive half-width"
        );
        estimates.push(estimate);
    }
    assert_eq!(estimates[0], ratio(1, 4));
    assert_eq!(estimates[1], ratio(1, 4));
    assert_eq!(estimates[2], ratio(1, 2));
    // Each run recomputes from its seed with its delays: a repeat of the same
    // request gives the same verdict, cycle count and step count.
    let again = || {
        completed(run_timed_method(
            &world,
            "Low",
            method,
            7,
            fsl_replay::statistical::TimedSampling::default(),
            1,
        ))
    };
    assert_eq!(again(), again());
}

/// FSpec FR-420-AC-10: the return to the initial discrete state after the
/// first of two tied steps, with the second still pending, is not a
/// regeneration point. `p` resets the clock without changing the discrete
/// state and ties with `q`; when `p` goes first its post state equals the
/// initial key with `q` pending, and a cut there would give one-step
/// cycles. Every cycle is `q, r` or `p, q, r`: at least two steps.
#[trace("FSpec-TC-374", "FSpec-FR-420-AC-10")]
#[test]
fn a_return_with_a_tied_step_pending_cuts_no_cycle() {
    let claims = format!(
        "pre P using v on M::Call::p {{ not self.retried }}\n\
         post Pd using v on M::Call::p {{ self.x = 0 ms }}\n\
         delay M::Call::p ~ discrete {{ 1 ms: 1 }};\n\
         pre Q using v on M::Call::q {{ not self.retried }}\n\
         post Qd using v on M::Call::q {{ self.retried = true }}\n\
         delay M::Call::q ~ discrete {{ 1 ms: 1 }};\n\
         pre R using v on M::Call::r {{ self.retried }}\n\
         post Rd using v on M::Call::r {{ self.retried = false and self.x = 0 ms }}\n\
         delay M::Call::r ~ discrete {{ 1 ms: 1 }};\n\
         workload Even on M {{ weight Call::p = 1; weight Call::q = 1; weight Call::r = 1; }}\n\
         probabilistic Tie using ev on M over (c: M::Call) under Even on origin \
         {UP_DOWN_CONFIDENCE} {{ long_run_fraction holds(c.retried) weighted by delay >= rational(1, 10) }}\n"
    );
    let world = timed_world_with(
        &[("p", &["x"]), ("q", &["retried"]), ("r", &["retried", "x"])],
        &claims,
    );
    let verdict = completed(run_timed_method(
        &world,
        "Tie",
        StatisticalMethod::Regenerative { min_cycles: 200 },
        3,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    ));
    assert_eq!(verdict.decision, Decision::Accepted);
    assert!(
        verdict.basis.draws >= 2 * verdict.basis.samples,
        "{} steps over {} cycles: a cycle was cut with a tied step pending",
        verdict.basis.draws,
        verdict.basis.samples
    );
}

/// FSpec FR-420-AC-11: a time-locked state reached by a delay-weighted
/// long-run behaviour settles `unsupported`, `time-locked-sample{state}`.
#[trace("FSpec-TC-374", "FSpec-FR-420-AC-11", "FSpec-FR-420-AC-7")]
#[test]
fn a_time_locked_long_run_behaviour_is_unsupported() {
    let claims = format!(
        "pre Go using v on M::Call::go {{ self.phase = 0 }}\n\
         post Went using v on M::Call::go {{ self.phase = 1 }}\n\
         delay M::Call::go ~ discrete {{ 1 ms: 1 }};\n\
         time invariant Deadline when {{ self.phase = 1 }} {{ self.x <= 2 ms }}\n\
         workload Even on M {{ weight Call::go = 1; }}\n\
         probabilistic Locked using ev on M over (c: M::Call) under Even on origin \
         {UP_DOWN_CONFIDENCE} {{ long_run_fraction holds(c.phase = 1) weighted by delay >= rational(1, 10) }}\n"
    );
    let world = timed_world_with(&[("go", &["phase"])], &claims);
    let outcome = run_timed_method(
        &world,
        "Locked",
        StatisticalMethod::Regenerative { min_cycles: 5 },
        3,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    )
    .expect("the run ends with an outcome");
    assert!(
        matches!(
            outcome,
            StatisticalOutcome::Unsupported {
                cause: StatisticalUnsupported::TimeLockedSample { .. },
                ..
            }
        ),
        "{outcome:?}"
    );
}

/// FSpec FR-420-AC-10: a return to the initial discrete state with a clock
/// above 0 is not a regeneration point. Only the step `Z` to `A` resets the
/// clock, so every cycle contains the full `A`, `B`, `Z` run: at least three
/// positions. A model whose steps never reset the clock never returns to the
/// initial key and stops at `max_cycle_steps`.
#[trace("FSpec-TC-374", "FSpec-FR-420-AC-10", "FSpec-FR-408-AC-20")]
#[test]
fn only_a_return_with_every_clock_at_its_initial_value_cuts_a_cycle() {
    let loop_model = |reset: &str| {
        format!(
            "pre Ab using v on M::Call::ab {{ self.phase = 0 }}\n\
             post Abd using v on M::Call::ab {{ self.phase = 1 }}\n\
             delay M::Call::ab ~ discrete {{ 1 ms: 1 }};\n\
             pre Ba using v on M::Call::ba {{ self.phase = 1 }}\n\
             post Bad using v on M::Call::ba {{ self.phase = 0 }}\n\
             delay M::Call::ba ~ discrete {{ 1 ms: 1 }};\n\
             pre Bz using v on M::Call::bz {{ self.phase = 1 }}\n\
             post Bzd using v on M::Call::bz {{ self.phase = 2 }}\n\
             delay M::Call::bz ~ discrete {{ 1 ms: 1 }};\n\
             pre Za using v on M::Call::za {{ self.phase = 2 }}\n\
             post Zad using v on M::Call::za {{ self.phase = 0{reset} }}\n\
             delay M::Call::za ~ discrete {{ 1 ms: 1 }};\n\
             workload Even on M {{ weight Call::ab = 1; weight Call::ba = 1; \
             weight Call::bz = 1; weight Call::za = 1; }}\n\
             probabilistic Loop using ev on M over (c: M::Call) under Even on origin \
             {UP_DOWN_CONFIDENCE} {{ long_run_fraction holds(c.phase = 2) weighted by delay >= rational(1, 10) }}\n"
        )
    };
    let operations: &[(&str, &[&str])] = &[
        ("ab", &["phase", "x"]),
        ("ba", &["phase", "x"]),
        ("bz", &["phase", "x"]),
        ("za", &["phase", "x"]),
    ];
    let resetting = timed_world_with(operations, &loop_model(" and self.x = 0 ms"));
    let verdict = completed(run_timed_method(
        &resetting,
        "Loop",
        StatisticalMethod::Regenerative { min_cycles: 200 },
        3,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    ));
    assert_eq!(verdict.decision, Decision::Accepted);
    assert!(
        verdict.basis.draws >= 3 * verdict.basis.samples,
        "{} steps over {} cycles: a cycle was cut before its clock reset",
        verdict.basis.draws,
        verdict.basis.samples
    );
    let never = timed_world_with(operations, &loop_model(""));
    let limits = StatisticalLimits {
        max_cycle_steps: 50,
        ..StatisticalLimits::default()
    };
    let stopped = run_timed_method_with_limits(
        &never,
        "Loop",
        StatisticalMethod::Regenerative { min_cycles: 5 },
        3,
        fsl_replay::statistical::TimedSampling::default(),
        limits,
        1,
    )
    .expect("the run ends with a stop");
    assert!(
        matches!(
            stopped,
            StatisticalOutcome::Stopped {
                stop: StatisticalStop::Limit {
                    limit: StatisticalLimit::MaxCycleSteps,
                    value: 50,
                },
                ..
            }
        ),
        "{stopped:?}"
    );
}

/// FSpec FR-420-AC-11: the initial state moves with weight 1 to a terminal
/// state where `P` holds and with weight 1 to one where it does not. A
/// behaviour that terminates has long-run fraction 1 or 0, the estimate is the
/// mean of those limits over Okamoto's `N` behaviours, `>= 1/4` is accepted
/// and `>= 3/4` rejected.
#[trace("FSpec-TC-374", "FSpec-FR-420-AC-11")]
#[test]
fn a_terminating_delay_weighted_behaviour_takes_the_limit_of_its_idle_tail() {
    let claims = format!(
        "pre Yes using v on M::Call::yes {{ self.phase = 0 }}\n\
         post Yesd using v on M::Call::yes {{ self.phase = 1 }}\n\
         delay M::Call::yes ~ discrete {{ 1 ms: 1 }};\n\
         pre No using v on M::Call::no {{ self.phase = 0 }}\n\
         post Nod using v on M::Call::no {{ self.phase = 2 }}\n\
         delay M::Call::no ~ discrete {{ 1 ms: 1 }};\n\
         workload Even on M {{ weight Call::yes = 1; weight Call::no = 1; }}\n\
         probabilistic Low using ev on M over (c: M::Call) under Even on origin \
         {UP_DOWN_CONFIDENCE} {{ long_run_fraction holds(c.phase = 1) weighted by delay >= rational(1, 4) }}\n\
         probabilistic High using ev on M over (c: M::Call) under Even on origin \
         {UP_DOWN_CONFIDENCE} {{ long_run_fraction holds(c.phase = 1) weighted by delay >= rational(3, 4) }}\n"
    );
    let world = timed_world_with(&[("yes", &["phase"]), ("no", &["phase"])], &claims);
    let method = StatisticalMethod::Regenerative { min_cycles: 5 };
    for (name, decision) in [("Low", Decision::Accepted), ("High", Decision::Rejected)] {
        let verdict = completed(run_timed_method(
            &world,
            name,
            method,
            11,
            fsl_replay::statistical::TimedSampling::default(),
            1,
        ));
        assert_eq!(verdict.decision, decision, "{name}");
        let estimate = verdict.estimate.clone().expect("an estimate");
        assert!(
            estimate > ratio(2, 5) && estimate < ratio(3, 5),
            "{name}: the mean of the limits 1 and 0 is near 1/2, not {estimate:?}"
        );
        let (low, high) = verdict.interval.clone().expect("an interval");
        let half = high.sub(&low).div(&ratio(2, 1)).expect("two");
        assert_eq!(
            half,
            ratio(1, 20),
            "{name}: the fixed-sample half-width is iota"
        );
        assert!(verdict.basis.samples > 1, "{name}: N behaviours were drawn");
    }
}

/// FSpec FR-420-AC-11: a cycle of total delay 0 adds nothing to the
/// denominator, and a run whose every cycle has total delay 0 takes the
/// unweighted fraction over positions.
#[trace("FSpec-TC-374", "FSpec-FR-420-AC-11")]
#[test]
fn a_run_of_zero_delay_cycles_takes_the_unweighted_fraction() {
    let claims = format!(
        "probabilistic Zero using ev on M over (c: M::Call) under Even on origin \
         {UP_DOWN_CONFIDENCE} {{ long_run_fraction holds(c.phase = 0) weighted by delay >= rational(2, 5) }}\n"
    );
    let world = timed_world_with(
        &[("fall", &["phase", "x"]), ("rise", &["phase", "x"])],
        &up_down_declarations(0, 0, &claims),
    );
    let verdict = completed(run_timed_method(
        &world,
        "Zero",
        StatisticalMethod::Regenerative { min_cycles: 5 },
        5,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    ));
    assert_eq!(verdict.decision, Decision::Accepted);
    assert_eq!(verdict.estimate, Some(ratio(1, 2)));
}

/// FSpec FR-407 "Time windows": a delay adds no position. The sampled and the
/// exact route read the same hand-computed value on each one-behaviour model.
/// `send` is the only step. At 5 ms and 6 ms it lands after the window; at
/// 4 ms it is the position at 4 ms; under a time invariant the state after
/// `send` at 2 ms is not quiescent, so it has no stutter position either.
#[test]
fn a_timed_formula_reads_no_position_inside_a_delay_on_either_route() {
    const HELD: &str = "time invariant Held when { self.phase = 1 } { self.x <= 10 ms }\n";
    let mut sampled = Vec::new();
    let mut exact = Vec::new();
    let mut expected = Vec::new();
    for (name, send_ms, invariant, formula, holds) in [
        (
            "Eventually",
            5,
            "",
            "eventually[4 ms, 4 ms] holds(c.phase = 0)",
            false,
        ),
        (
            "Always",
            5,
            "",
            "always[4 ms, 4 ms] holds(c.phase = 1)",
            true,
        ),
        (
            "Until",
            5,
            "",
            "holds(c.phase = 0) until[4 ms, 4 ms] holds(c.phase = 0)",
            false,
        ),
        (
            "Window",
            6,
            "",
            "eventually[3 ms, 5 ms] holds(c.phase = 0)",
            false,
        ),
        (
            "AtHorizon",
            4,
            "",
            "eventually[4 ms, 4 ms] holds(c.phase = 1)",
            true,
        ),
        (
            "Invariant",
            2,
            HELD,
            "eventually[4 ms, 4 ms] holds(c.phase = 1)",
            false,
        ),
    ] {
        let declarations = format!(
            "pre Send using v on M::Call::send {{ self.phase = 0 }}\n\
             post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
             delay M::Call::send ~ discrete {{ {send_ms} ms: 1 }};\n\
             {invariant}\
             workload Even on M {{ weight Call::send = 1; }}\n\
             probabilistic {name} using ev on M over (c: M::Call) under Even on origin\n\
                 with alpha 0.4, beta 0.4, indifference 0.05\n\
                 {{ probability >= 0.5 [ {formula} ] }}\n"
        );
        let world = timed_world_with(&[("send", &["phase"])], &declarations);
        let verdict = completed(run_timed_method(
            &world,
            name,
            StatisticalMethod::Okamoto,
            7,
            fsl_replay::statistical::TimedSampling::default(),
            1,
        ));
        sampled.push((
            name,
            verdict.estimate,
            verdict.decision == Decision::Accepted,
        ));
        exact.push((name, exact_outcome(&world, name)));
        expected.push((name, Some(ratio(i64::from(holds), 1)), holds));
    }
    assert_eq!(sampled, expected, "sampled route");
    let exact_expected: Vec<_> = expected
        .iter()
        .map(|(name, _, holds)| (*name, if *holds { "proved" } else { "refuted" }.to_owned()))
        .collect();
    assert_eq!(exact, exact_expected, "exact route");
}

/// The exact route's outcome word for the claim `name`.
fn exact_outcome(world: &crate::timed_world::World, name: &str) -> String {
    let item = fsl_analyze::analyze::exact_probabilistic_item(
        fsl_foundation::RequestIndex::new(0),
        fsl_analyze::exact_probabilistic::ProductRequest {
            subject: ModelSubject {
                simulation: world.request(),
                proof_bounds: Vec::new(),
            },
            claim: world
                .package
                .graph()
                .probabilistic_model()
                .claim(name)
                .expect("the claim checks"),
            limits: fsl_analyze::exact_probabilistic::ExactProbLimits::default(),
        },
    );
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the linked engines are admitted");
    let record =
        fsl_analyze::analyze::analyze_items(&engines, vec![item], &Cancel::new()).remove(0);
    let terminal = record.terminal().expect("a terminal record");
    let outcome = serde_json::to_value(fsl_replay::OutcomeItem::from_terminal(terminal))
        .expect("the outcome serializes");
    outcome["result"]
        .as_str()
        .expect("a result word")
        .to_owned()
}

/// TC-709 step 5's fixtures: `send` moves `phase` 0 to 1 and, where `reply`
/// is given, moves it 1 to 2; `rewards` states `reward cost` declarations.
fn window_world(
    send_ms: u32,
    reply_ms: Option<u32>,
    rewards: &str,
    claim: &str,
) -> crate::timed_world::World {
    let mut operations: Vec<(&str, &[&str])> = vec![("send", &["phase"])];
    let mut reply = String::new();
    let mut reply_weight = "";
    if let Some(delay) = reply_ms {
        operations.push(("reply", &["phase"]));
        reply = format!(
            "pre Reply using v on M::Call::reply {{ self.phase = 1 }}\n\
             post Replied using v on M::Call::reply {{ self.phase = 2 }}\n\
             delay M::Call::reply ~ discrete {{ {delay} ms: 1 }};\n"
        );
        reply_weight = "weight Call::reply = 1;";
    }
    timed_world_with(
        &operations,
        &format!(
            "pre Send using v on M::Call::send {{ self.phase = 0 }}\n\
             post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
             delay M::Call::send ~ discrete {{ {send_ms} ms: 1 }};\n\
             {reply}{rewards}\
             workload Even on M {{ weight Call::send = 1; {reply_weight} }}\n\
             {}",
            window_claim(claim)
        ),
    )
}

/// TC-709's `Tick` fixture: `time tick` with period 2 ms on `tick`, which is
/// enabled at `phase = 2` only.
fn tick_world(claim: &str) -> crate::timed_world::World {
    let body = format!(
        "dimension Time;\nunit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         time M tick M::Call::tick period 2 ms unit ms;\n\
         pre Send using v on M::Call::send {{ self.phase = 0 }}\n\
         post Sent using v on M::Call::send {{ self.phase = 1 }}\n\
         pre Tick using v on M::Call::tick {{ self.phase = 2 }}\n\
         post Ticked using v on M::Call::tick {{ self.phase = 2 }}\n\
         workload Even on M {{ weight Call::send = 1; weight Call::tick = 1; }}\n\
         {}",
        window_claim(claim)
    );
    crate::timed_world::world(
        &crate::timed_world::document(&[], &[("send", &["phase"]), ("tick", &["phase"])]),
        "profile ev = \"filament.temporal.timed/v1\";\n",
        &body,
    )
}

fn window_claim(claim: &str) -> String {
    format!(
        "probabilistic Probe using ev on M over (c: M::Call) under Even on origin\n\
             with alpha 0.4, beta 0.4, indifference 0.05\n\
             {{ {claim} }}\n"
    )
}

fn probe(world: &crate::timed_world::World) -> StatisticalOutcome {
    run_timed_method(
        world,
        "Probe",
        StatisticalMethod::Okamoto,
        7,
        fsl_replay::statistical::TimedSampling::default(),
        1,
    )
    .expect("the claim runs")
}

/// The estimate, as a fraction, and whether the claim is accepted.
fn measured(world: &crate::timed_world::World) -> (Option<Rational>, Decision) {
    let verdict = completed(Ok(probe(world)));
    (verdict.estimate, verdict.decision)
}

const COSTS: &str = "reward cost on M::Call::send = 1;\nreward cost on M::Call::reply = 3;\n";
const SEND_COST: &str = "reward cost on M::Call::send = 1;\n";

/// FR-254-AC-5: positions stamped 0 and 2 ms lie in `[0, 5 ms]`; the `reply`
/// step that leaves the second lands at 12 ms and weighs it.
#[trace("TC-709", "FR-254-AC-5")]
#[test]
fn a_time_window_fraction_reads_every_position_and_the_leaving_rewards() {
    let claim = "mean of (fraction holds(c.phase = 1) over [0, 5 ms]) <= 0.25";
    assert_eq!(
        measured(&window_world(2, Some(10), "", claim)),
        (Some(ratio(1, 2)), Decision::Rejected)
    );
    let weighted = "mean of (fraction holds(c.phase = 1) over [0, 5 ms] weighted by cost) <= 0.25";
    assert_eq!(
        measured(&window_world(2, Some(10), COSTS, weighted)),
        (Some(ratio(3, 4)), Decision::Rejected)
    );
}

/// Hand values over `Send` (stamps 0, 2, 12; `cost` 1 then 3): `accumulate`
/// and `steps` read the positions of a time window.
#[test]
fn accumulate_and_steps_within_a_time_horizon_read_the_window_positions() {
    for (claim, holds) in [
        (
            "probability >= 0.5 [steps from holds(c.phase = 0) until holds(c.phase = 2) within 12 ms >= 2]",
            true,
        ),
        (
            "probability >= 0.5 [accumulate cost from holds(c.phase = 0) until holds(c.phase = 2) within 12 ms >= 4]",
            true,
        ),
        (
            "probability >= 0.5 [accumulate cost from holds(c.phase = 0) until holds(c.phase = 1) within 5 ms >= 2]",
            false,
        ),
        (
            "probability >= 0.5 [steps from holds(c.phase = 0) until holds(c.phase = 2) within 5 ms <= 3]",
            false,
        ),
    ] {
        let decision = if holds {
            Decision::Accepted
        } else {
            Decision::Rejected
        };
        assert_eq!(
            measured(&window_world(2, Some(10), COSTS, claim)),
            (Some(ratio(i64::from(holds), 1)), decision),
            "{claim}"
        );
    }
}

/// FR-254-AC-6: the positions are stamped 0, 0, 2 and 4 ms.
#[trace("TC-709", "FR-254-AC-6")]
#[test]
fn a_tick_subject_is_sampled_without_a_race_and_reads_its_stutter_positions() {
    for (claim, estimate, decision) in [
        (
            "mean of (fraction holds(c.phase = 1) over [0, 5 ms]) <= 0.25",
            ratio(3, 4),
            Decision::Rejected,
        ),
        (
            "probability >= 0.5 [eventually[3 ms, 5 ms] holds(c.phase = 1)]",
            ratio(1, 1),
            Decision::Accepted,
        ),
        (
            "probability >= 0.5 [eventually[3 ms, 3 ms] holds(c.phase = 1)]",
            ratio(0, 1),
            Decision::Rejected,
        ),
    ] {
        assert_eq!(
            measured(&tick_world(claim)),
            (Some(estimate), decision),
            "{claim}"
        );
    }
}

/// FR-254-AC-7: on `Idle` every completion of the tail gives one reading.
#[trace("TC-709", "FR-254-AC-7")]
#[test]
fn an_idle_tail_reading_every_completion_agrees_on_is_decided() {
    for (rewards, claim, holds, decision) in [
        (
            "",
            "probability >= 0.5 [fraction holds(c.phase = 1) over [0, 5 ms] >= rational(2, 5)]",
            1,
            Decision::Accepted,
        ),
        (
            "",
            "probability >= 0.5 [fraction holds(c.phase = 1) over [0, 5 ms] <= rational(2, 5)]",
            0,
            Decision::Rejected,
        ),
        (
            "",
            "mean of (fraction holds(c.phase = 0 or c.phase = 1) over [0, 5 ms]) <= 0.25",
            1,
            Decision::Rejected,
        ),
        (
            "",
            "mean of (fraction holds(c.phase = 2) over [0, 5 ms]) <= 0.25",
            0,
            Decision::Accepted,
        ),
        (
            SEND_COST,
            "mean of (fraction holds(c.phase = 1) over [0, 5 ms] weighted by cost) <= 0.25",
            0,
            Decision::Accepted,
        ),
        (
            "",
            "probability >= 0.5 [steps from holds(c.phase = 0) until holds(c.phase = 1) within 5 ms >= 2]",
            0,
            Decision::Rejected,
        ),
        (
            "",
            "probability >= 0.5 [steps from holds(c.phase = 0) until holds(c.phase = 2) within 5 ms <= 3]",
            0,
            Decision::Rejected,
        ),
        (
            "",
            "probability >= 0.5 [eventually[3 ms, 5 ms] holds(c.phase = 0)]",
            0,
            Decision::Rejected,
        ),
        (
            "",
            "probability >= 0.5 [always[3 ms, 5 ms] holds(c.phase = 1)]",
            1,
            Decision::Accepted,
        ),
    ] {
        assert_eq!(
            measured(&window_world(2, None, rewards, claim)),
            (Some(ratio(holds, 1)), decision),
            "{claim}"
        );
    }
}

/// The timed state just after `send` at 2 ms.
fn state_after_send(world: &crate::timed_world::World) -> fsl_eval::simulation::StateKey {
    let system = world.system();
    let timed = TimedSystem::new(&system, "M").expect("the subject is timed");
    let initial = timed.initial().remove(0);
    let delayed = timed
        .delay(&initial, &ratio(2, 1))
        .expect("the delay reads")
        .expect("the delay is admissible");
    let (_, after) = timed.steps(&delayed).expect("the steps read").remove(0);
    timed.state_key(&after).expect("the state encodes")
}

/// FR-254-AC-8: two completions of the tail give different readings.
#[trace("TC-709", "FR-254-AC-8")]
#[test]
fn an_idle_tail_reading_that_depends_on_the_stutters_is_not_stochastic() {
    for claim in [
        "mean of (fraction holds(c.phase = 1) over [0, 5 ms]) <= 0.25",
        "probability >= 0.5 [fraction holds(c.phase = 1) over [0, 5 ms] <= rational(3, 5)]",
        "probability >= 0.5 [eventually[3 ms, 5 ms] holds(c.phase = 1)]",
    ] {
        let world = window_world(2, None, "", claim);
        let expected = state_after_send(&world);
        match probe(&world) {
            StatisticalOutcome::Unsupported {
                cause: StatisticalUnsupported::NotStochasticIdleTail { state },
                ..
            } => assert_eq!(state, expected, "{claim}"),
            other => panic!("{claim}: {other:?}"),
        }
    }
    assert_eq!(
        measured(&window_world(
            5,
            None,
            "",
            "mean of (fraction holds(c.phase = 1) over [0, 5 ms]) <= 0.25"
        )),
        (Some(ratio(1, 2)), Decision::Rejected)
    );
}

/// A nested timed formula on a dense idle tail is not decided yet: it stops
/// typed incomplete and gives no reading (FR-254 "Undecided completions").
#[test]
fn a_nested_timed_formula_on_a_dense_idle_tail_stops_incomplete() {
    let world = window_world(
        2,
        None,
        "",
        "probability >= 0.5 [eventually[0 ms, 5 ms] always[0 ms, 1 ms] holds(c.phase = 1)]",
    );
    let expected = state_after_send(&world);
    match probe(&world) {
        StatisticalOutcome::Stopped {
            stop: StatisticalStop::IdleTailUndecided(stopped),
            ..
        } => assert_eq!(
            (stopped.state, stopped.reason),
            (
                expected,
                fsl_replay::statistical::IdleTailReason::TimedFormula
            )
        ),
        other => panic!("{other:?}"),
    }
}

/// FSpec FR-420 "Windows on an idle tail": `accumulate` and `elapsed` read
/// the same on every completion. `send` at 2 ms with cost 1, then idle:
/// the run accumulates 1 and takes 2 ms to `phase = 1`, and never reaches
/// `phase = 2`.
#[test]
fn accumulate_and_elapsed_on_a_dense_idle_tail_are_decided() {
    for (claim, holds) in [
        (
            "accumulate cost from holds(c.phase = 0) until holds(c.phase = 1) within 5 ms >= 1",
            true,
        ),
        (
            "accumulate cost from holds(c.phase = 0) until holds(c.phase = 1) within 5 ms >= 2",
            false,
        ),
        (
            "accumulate cost from holds(c.phase = 0) until holds(c.phase = 2) within 5 ms <= 3",
            false,
        ),
        (
            "elapsed from holds(c.phase = 0) until holds(c.phase = 1) within 5 ms <= 2 ms",
            true,
        ),
        (
            "elapsed from holds(c.phase = 0) until holds(c.phase = 1) within 5 ms <= 1 ms",
            false,
        ),
        (
            "elapsed from holds(c.phase = 0) until holds(c.phase = 2) within 5 ms <= 5 ms",
            false,
        ),
    ] {
        let decision = if holds {
            Decision::Accepted
        } else {
            Decision::Rejected
        };
        assert_eq!(
            measured(&window_world(
                2,
                None,
                SEND_COST,
                &format!("probability >= 0.5 [{claim}]")
            )),
            (Some(ratio(i64::from(holds), 1)), decision),
            "{claim}"
        );
    }
}

/// Whether `claim` on the `Idle` fixture stops `idle-tail-undecided` as a
/// timed formula, at the state after `send`.
fn stops_undecided(claim: &str) {
    let world = window_world(2, None, "", claim);
    let expected = state_after_send(&world);
    match probe(&world) {
        StatisticalOutcome::Stopped {
            stop: StatisticalStop::IdleTailUndecided(stopped),
            ..
        } => assert_eq!(
            (stopped.state, stopped.reason),
            (
                expected,
                fsl_replay::statistical::IdleTailReason::TimedFormula
            ),
            "{claim}"
        ),
        other => panic!("{claim}: {other:?}"),
    }
}

/// A past operator on a dense idle tail is not decided.
#[test]
fn a_past_operator_on_a_dense_idle_tail_stops_incomplete() {
    for claim in [
        "probability >= 0.5 [once[0 ms, 5 ms] holds(c.phase = 0)]",
        "probability >= 0.5 [historically[0 ms, 5 ms] holds(c.phase = 0)]",
        "probability >= 0.5 [holds(c.phase = 0) since[0 ms, 5 ms] holds(c.phase = 0)]",
        "probability >= 0.5 [holds(c.phase = 0) triggered[0 ms, 5 ms] holds(c.phase = 0)]",
    ] {
        stops_undecided(claim);
    }
}

/// A window aggregate atom is not admitted in a probabilistic event, so it
/// never reaches the idle-tail reading.
///
/// The refusal today is an `InternalFault` where a user diagnostic is due
/// (QSL-3524); that fix tightens these assertions to the diagnostic.
#[test]
fn a_window_aggregate_event_does_not_check() {
    let checked = std::panic::catch_unwind(|| {
        window_world(
            2,
            None,
            "",
            "probability >= 0.5 [count where holds(c.phase = 1) over past[0 ms, 5 ms] >= 1]",
        )
    });
    let refusal = checked.err().expect("the unit does not check");
    let refusal = refusal.downcast_ref::<String>().expect("a message");
    assert!(refusal.contains("CheckRefusal"), "{refusal}");
    assert!(refusal.contains("aggregate-term-typed"), "{refusal}");
}

/// Five distinct interval bounds inside the idle tail are decided and six are
/// not. No position holds `phase = 0` after 0 ms, so the decided event is
/// false: the bounds inside `(2, 8)` are 3, 4, 5, 6 and 7; `[3 ms, 9 ms]`
/// moves the window's end to 9 and puts 8 inside too.
#[test]
fn six_interval_bounds_inside_a_dense_idle_tail_stop_incomplete() {
    let five = "eventually[3 ms, 4 ms] holds(c.phase = 0) \
                or eventually[5 ms, 6 ms] holds(c.phase = 0) \
                or eventually[7 ms, 8 ms] holds(c.phase = 0)";
    assert_eq!(
        measured(&window_world(
            2,
            None,
            "",
            &format!("probability >= 0.5 [{five}]")
        )),
        (Some(ratio(0, 1)), Decision::Rejected)
    );
    stops_undecided(&format!(
        "probability >= 0.5 [{five} or eventually[3 ms, 9 ms] holds(c.phase = 0)]"
    ));
}

/// A falling fraction: `phase = 0` holds at 0 ms and not in the idle state, so
/// with `m` stutters the value is `1 / (2 + m)`. It is at most `3/5` for every
/// `m`; it is at least `2/5` at `m = 0` and not for large `m`.
#[test]
fn a_falling_fraction_on_a_dense_idle_tail_is_read_over_every_completion() {
    let below = "probability >= 0.5 [fraction holds(c.phase = 0) over [0, 5 ms] <= rational(3, 5)]";
    assert_eq!(
        measured(&window_world(2, None, "", below)),
        (Some(ratio(1, 1)), Decision::Accepted)
    );
    for claim in [
        "probability >= 0.5 [fraction holds(c.phase = 0) over [0, 5 ms] >= rational(2, 5)]",
        "mean of (fraction holds(c.phase = 0) over [0, 5 ms]) <= 0.25",
    ] {
        assert!(
            matches!(
                probe(&window_world(2, None, "", claim)),
                StatisticalOutcome::Unsupported {
                    cause: StatisticalUnsupported::NotStochasticIdleTail { .. },
                    ..
                }
            ),
            "{claim}"
        );
    }
}
