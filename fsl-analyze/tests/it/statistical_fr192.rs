// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-192-AC-9: fixed-N settlement, partial statistical counts, and checked
//! claim-bound validation at the serialized result boundary.

use filament_exact::{Cancel, Integer, Rational};
use fsl_analyze::statistical::{
    check_sampled, BehaviourSampler, Bound, Comparison, Drawn, RegenerationWalk, Sample,
    SampleHalt, SampledRequest, StatisticalClaim, TestSubject, WalkStep, Weight,
};
use fsl_eval::simulation::StateKey;
use fsl_foundation::{selection::DefinitionRef, BackendId, RequestIndex};
use fsl_replay::statistical::{
    verify_statistical_result_bound, BoundComparison, ClaimForm, Confidence, Decision,
    StatisticalLimit, StatisticalLimits, StatisticalMethod, StatisticalOutcome, StatisticalStop,
    UndecidedCause, VerdictBound,
};
use fsl_replay::{UndefinedEvaluation, UndefinedLocus, UndefinedSite};
use ix_trace_rs::trace;
use serde_json::json;

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("nonzero")
}

fn confidence() -> Confidence {
    Confidence::new(ratio(1, 100), ratio(1, 100), ratio(1, 10)).expect("valid confidence")
}

fn claim() -> StatisticalClaim {
    StatisticalClaim::new(
        ClaimForm::Probability,
        Bound {
            comparison: Comparison::AtLeast,
            threshold: ratio(1, 2),
        },
        confidence(),
    )
    .expect("valid probability claim")
}

fn request<'a>(
    claim: &'a StatisticalClaim,
    sampler: &'a DefinitionRef,
    method: StatisticalMethod,
    limits: StatisticalLimits,
) -> SampledRequest<'a> {
    static TESTS: [TestSubject; 1] = [TestSubject {
        initial: 0,
        binding: None,
        reference: None,
    }];
    SampledRequest {
        claim,
        tests: &TESTS,
        method,
        seed: 7,
        workers: 1,
        sampler,
        limits,
        max_witnesses: 0,
        timed_sampling: fsl_replay::statistical::TimedSampling::default(),
    }
}

struct UnusedWalk;

impl RegenerationWalk for UnusedWalk {
    fn step(&mut self) -> Result<WalkStep, SampleHalt> {
        panic!("the sampled-method case does not request a regeneration walk")
    }
}

struct FixedSample {
    sample: Sample,
    calls: u64,
}

impl BehaviourSampler for FixedSample {
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
        self.calls += 1;
        Ok(Drawn {
            sample: self.sample.clone(),
            steps: Vec::new(),
        })
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

#[trace("TC-627", "FR-192-AC-9")]
#[test]
fn fr192_ac9_fixed_n_preflight_settles_required_count_without_partial_basis() {
    for (method, error_probability, expected_count) in [
        (StatisticalMethod::Okamoto, ratio(1, 100), 9_210_341_u64),
        (
            StatisticalMethod::ClopperPearson,
            ratio(1, 2000),
            15_201_805_u64,
        ),
    ] {
        let claim = StatisticalClaim::new(
            ClaimForm::Probability,
            Bound {
                comparison: Comparison::AtLeast,
                threshold: ratio(999, 1000),
            },
            Confidence::new(error_probability.clone(), error_probability, ratio(1, 2000))
                .expect("valid fixed-N confidence"),
        )
        .expect("valid fixed-N claim");
        let sampler =
            DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1").expect("sampler");
        let mut probe = FixedSample {
            sample: Sample::Event(true),
            calls: 0,
        };
        let outcome = check_sampled(
            request(
                &claim,
                &sampler,
                method,
                StatisticalLimits {
                    max_samples: 1_000_000,
                    ..StatisticalLimits::default()
                },
            ),
            &mut probe,
            &Cancel::new(),
        )
        .expect("fixed-N preflight is a stopped outcome");
        let meter = filament_exact::ScalarLimits {
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
        };
        let record = fsl_replay::statistical::settle_statistical(
            &outcome,
            fsl_replay::statistical::SubjectLimits {
                expansion: fsl_eval::simulation::model::ExpansionLimits { max_candidates: 2 },
                meter: &meter,
            },
        );
        let StatisticalOutcome::Stopped {
            stop:
                StatisticalStop::PreflightLimit {
                    limit: StatisticalLimit::MaxSamples,
                    value: 1_000_000,
                    required,
                },
            partial,
            ..
        } = outcome
        else {
            panic!("fixed-N preflight stops exactly: {outcome:?}");
        };
        assert_eq!(required, Integer::from(expected_count));
        assert_eq!((partial.samples, partial.draws), (0, 0));
        assert_eq!(probe.calls, 0);
        assert_eq!(
            StatisticalLimit::MaxSamples.setting(),
            "statistical.max_samples"
        );
        let terminal = record
            .clone()
            .into_terminal(RequestIndex::new(0), BackendId::new("en-4"));
        let wire = serde_json::to_value(fsl_replay::OutcomeItem::from_terminal(&terminal))
            .expect("terminal result serializes");
        assert_eq!(wire["result"], "incomplete");
        assert_eq!(wire["cause"]["kind"], "limit-reached");
        assert_eq!(wire["cause"]["limit"], "max_samples");
        assert_eq!(wire["cause"]["value"], 1_000_000);
        assert_eq!(wire["cause"]["setting"], "statistical.max_samples");
        assert_eq!(wire["cause"]["required"], expected_count.to_string());
        let fsl_replay::TerminalValue::Incomplete(fsl_replay::IncompleteCause::LimitReached {
            limit: fsl_replay::ModelCheckLimit::Statistical(StatisticalLimit::MaxSamples),
            value: 1_000_000,
            required: Some(settled_required),
        }) = record.value
        else {
            panic!("preflight settlement keeps limit, value and required count");
        };
        assert_eq!(settled_required, Integer::from(expected_count));
    }
}

struct CycleWalk;

impl RegenerationWalk for CycleWalk {
    fn step(&mut self) -> Result<WalkStep, SampleHalt> {
        Ok(WalkStep::Position {
            holds: true,
            weight: Weight::unit(),
            returned: true,
        })
    }
}

struct CycleSampler;

impl BehaviourSampler for CycleSampler {
    type Walk = CycleWalk;

    fn sample_batch_width(&self) -> usize {
        1
    }

    fn sample(
        &mut self,
        _test: &TestSubject,
        _seed: u64,
        _trace: u64,
    ) -> Result<Drawn, SampleHalt> {
        panic!("regenerative methods use a walk")
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
        CycleWalk
    }
}

#[trace("TC-627", "FR-192-AC-9")]
#[test]
fn fr192_ac9_regenerative_limit_keeps_closed_cycle_and_draw_counts() {
    let claim = StatisticalClaim::new(
        ClaimForm::LongRunFraction,
        Bound {
            comparison: Comparison::AtLeast,
            threshold: ratio(1, 2),
        },
        confidence(),
    )
    .expect("valid long-run claim");
    let sampler =
        DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1").expect("sampler");
    let outcome = check_sampled(
        request(
            &claim,
            &sampler,
            StatisticalMethod::Regenerative { min_cycles: 10 },
            StatisticalLimits {
                max_samples: 1,
                ..StatisticalLimits::default()
            },
        ),
        &mut CycleSampler,
        &Cancel::new(),
    )
    .expect("regenerative run stops at max_samples");
    let StatisticalOutcome::Stopped {
        stop:
            StatisticalStop::Limit {
                limit: StatisticalLimit::MaxSamples,
                value: 1,
            },
        partial,
        ..
    } = outcome
    else {
        panic!("regenerative run stops at its sample limit: {outcome:?}");
    };
    assert_eq!((partial.samples, partial.draws), (1, 1));
}

#[trace("TC-627", "FR-192-AC-9")]
#[test]
fn fr192_ac9_undefined_sample_increments_draws_without_counting_sample() {
    let claim = claim();
    let sampler =
        DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1").expect("sampler");
    let undefined = UndefinedEvaluation {
        site: UndefinedSite {
            position: 0,
            state_key: StateKey::default(),
            locus: UndefinedLocus::FractionValue,
            transition: None,
            time: None,
        },
        cause: fsl_eval::temporal::UndefinedCause::Kernel(
            filament_exact::Undefined::DivisionByZero,
        ),
    };
    let mut probe = FixedSample {
        sample: Sample::Undefined(undefined),
        calls: 0,
    };
    let outcome = check_sampled(
        request(
            &claim,
            &sampler,
            StatisticalMethod::Sprt,
            StatisticalLimits::default(),
        ),
        &mut probe,
        &Cancel::new(),
    )
    .expect("undefined is a measured rejection");
    let StatisticalOutcome::Completed(verdict) = outcome else {
        panic!("undefined is a completed measured result: {outcome:?}");
    };
    assert_eq!(verdict.decision, Decision::Rejected);
    assert_eq!(
        (verdict.basis.draws, verdict.basis.samples, probe.calls),
        (1, 0, 1)
    );
    assert_eq!((verdict.tests[0].draws, verdict.tests[0].samples), (1, 0));
    assert_eq!(
        verdict.tests[0]
            .undefined
            .as_ref()
            .map(|sample| sample.trace),
        Some(0)
    );
}

#[trace("TC-627", "FR-192-AC-9")]
#[test]
fn fr192_ac9_sprt_limit_keeps_partial_counts_and_every_verdict_keeps_claim_bound() {
    let claim = claim();
    let sampler =
        DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1").expect("sampler");
    let mut probe = FixedSample {
        sample: Sample::Event(true),
        calls: 0,
    };
    let outcome = check_sampled(
        request(
            &claim,
            &sampler,
            StatisticalMethod::Sprt,
            StatisticalLimits {
                max_samples: 1,
                ..StatisticalLimits::default()
            },
        ),
        &mut probe,
        &Cancel::new(),
    )
    .expect("SPRT stops at its configured sample limit");
    let StatisticalOutcome::Stopped {
        stop:
            StatisticalStop::Limit {
                limit: StatisticalLimit::MaxSamples,
                value: 1,
            },
        partial,
        ..
    } = outcome
    else {
        panic!("SPRT stops at one sample: {outcome:?}");
    };
    assert_eq!((partial.samples, partial.draws, probe.calls), (1, 1, 1));

    let expected = VerdictBound {
        form: ClaimForm::Probability,
        comparison: BoundComparison::GreaterOrEqual,
        threshold: ratio(1, 2),
        level: None,
    };
    for (sample, decision) in [
        (Sample::Event(true), Decision::Accepted),
        (Sample::Event(false), Decision::Rejected),
        (
            Sample::UndecidedSuccessor,
            Decision::Undecided(UndecidedCause::UndecidedSuccessor),
        ),
    ] {
        let mut probe = FixedSample { sample, calls: 0 };
        let sampler =
            DefinitionRef::new("agent-ix", "filament.simulation.sampler/v1").expect("sampler");
        let outcome = check_sampled(
            request(
                &claim,
                &sampler,
                StatisticalMethod::Sprt,
                StatisticalLimits::default(),
            ),
            &mut probe,
            &Cancel::new(),
        )
        .expect("the sample yields a completed measured verdict");
        let StatisticalOutcome::Completed(verdict) = outcome else {
            panic!("sample completes a verdict: {outcome:?}");
        };
        assert_eq!(verdict.decision, decision);
        assert_eq!(&verdict.bound, &expected);
    }
}

#[trace("TC-627", "FR-192-AC-9")]
#[test]
fn fr192_ac9_result_reader_refuses_missing_malformed_and_mismatched_claim_bounds() {
    let expected = VerdictBound {
        form: ClaimForm::Probability,
        comparison: BoundComparison::GreaterOrEqual,
        threshold: ratio(1, 2),
        level: None,
    };
    let matching = json!({"decision": "accepted", "bound": {
        "type": "probability",
        "comparison": "greater-or-equal",
        "threshold": {"numerator": "1", "denominator": "2"}
    }});
    assert_eq!(
        verify_statistical_result_bound(&matching, &expected),
        Ok(())
    );
    assert_eq!(
        verify_statistical_result_bound(&json!({"decision": "accepted"}), &expected),
        Err(fsl_replay::statistical::StatisticalBoundReadError::Missing)
    );
    assert_eq!(
        verify_statistical_result_bound(
            &json!({"decision": "accepted", "bound": {
                "type": "probability", "comparison": "greater-or-equal",
                "threshold": {"numerator": "one", "denominator": "2"}
            }}),
            &expected,
        ),
        Err(fsl_replay::statistical::StatisticalBoundReadError::Malformed)
    );
    assert_eq!(
        verify_statistical_result_bound(
            &json!({"decision": "accepted", "bound": {
                "type": "probability", "comparison": "less-or-equal",
                "threshold": {"numerator": "1", "denominator": "2"}
            }}),
            &expected,
        ),
        Err(fsl_replay::statistical::StatisticalBoundReadError::Mismatch)
    );
}
