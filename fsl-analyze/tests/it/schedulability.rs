// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-246 to FR-248 and FSpec FR-419: task sets, EN-7's closed-form
//! analyses, and the checker that recomputes their evidence before a
//! verdict settles. The fixtures are ADR-026 §12's `Ctl` and the sets the
//! acceptance criteria name.

use filament_exact::{Cancel, CancelCause, Integer, Rational};
use filament_semantic_value::catalog::Category;
use fsl_analyze::schedulability::{analyze, ScheduleLimits};
use fsl_replay::{
    recompute, settle, AmcEvidence, Arrival, Certification, ClosedFormAnalysis, ClosedFormCheck,
    ClosedFormDisagreement, ClosedFormEvidence, ClosedFormOutcome, Code, Criticality,
    DisagreementCause, Identifier, InconclusiveCause, JobIterates, MissEvidence, ProofBasis,
    QualifiedName, ScheduleClaim, ScheduleSubject, SchedulingPolicy, Task, TaskFixpoint, TaskSet,
    TaskSetRefusal, TerminalValue, TimeMember, Verdict, Wcet,
};
use fsl_replay::{IncompleteCause, ModelCheckLimit};
use ix_trace_rs::trace;

fn r(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("a nonzero divisor")
}

fn n(value: i64) -> Rational {
    r(value, 1)
}

fn id(name: &str) -> Identifier {
    Identifier::new(name).expect("an identifier")
}

fn set_name(name: &str) -> QualifiedName {
    QualifiedName::new(vec![id(name)]).expect("one segment")
}

/// A `LO` task with `D = T`.
fn task(name: &str, wcet: i64, period: i64, priority: u64) -> Task {
    Task {
        name: id(name),
        arrival: Arrival::Periodic { period: n(period) },
        deadline: n(period),
        wcet: Wcet::Single(n(wcet)),
        priority: Some(priority),
        criticality: Criticality::Lo,
        jitter: n(0),
        blocking: n(0),
    }
}

fn hi_task(name: &str, lo: i64, hi: i64, period: i64, priority: u64) -> Task {
    Task {
        criticality: Criticality::Hi,
        wcet: Wcet::Dual {
            lo: n(lo),
            hi: n(hi),
        },
        ..task(name, lo, period, priority)
    }
}

/// ADR-026 §12's `Ctl`: `(C, T = D)` of `(1, 4)`, `(2, 6)` and `(c, 12)`,
/// priorities by period.
fn ctl(third: i64) -> TaskSet {
    TaskSet::new(
        set_name("Ctl"),
        vec![
            task("a", 1, 4, 1),
            task("b", 2, 6, 2),
            task("c", third, 12, 3),
        ],
    )
    .expect("an admitted task set")
}

fn b_set(third: i64, periods: [i64; 3]) -> TaskSet {
    TaskSet::new(
        set_name("B"),
        vec![
            task("t1", 1, periods[0], 1),
            task("t2", 2, periods[1], 2),
            task("t3", third, periods[2], 3),
        ],
    )
    .expect("an admitted set")
}

fn b_response_time_results(
    periods: [i64; 3],
    third_fixpoint: i64,
    three: &[i64],
    four: &[i64],
    loads: [Rational; 2],
) {
    let fixed = schedulable(b_set(3, periods), SchedulingPolicy::FixedPriority);
    let outcome = run(&fixed);
    let points = fixed_points(&outcome);
    assert_eq!(fixpoints(points), vec![n(1), n(3), n(third_fixpoint)]);
    assert_eq!(points[2].jobs, vec![JobIterates(iterates(three))]);
    assert_eq!(
        settled(&fixed, &outcome),
        proved(ClosedFormAnalysis::FixedPriorityRta)
    );
    let missed = schedulable(b_set(4, periods), SchedulingPolicy::FixedPriority);
    let outcome = run(&missed);
    assert_eq!(
        outcome,
        ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
            task: id("t3"),
            job: 0,
            iterates: iterates(four)
        })
    );
    assert_eq!(settled(&missed, &outcome), TerminalValue::Refuted);
    for (third, load) in [3, 4].into_iter().zip(loads) {
        let edf = schedulable(b_set(third, periods), SchedulingPolicy::Edf);
        let utilization = edf.task_set().tasks().iter().fold(n(0), |sum, task| {
            sum.add(
                &task
                    .wcet
                    .hi()
                    .div(task.arrival.interval())
                    .expect("positive period"),
            )
        });
        assert_eq!(utilization, load);
        let outcome = run(&edf);
        assert!(
            matches!(
                outcome,
                ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand { .. })
            ),
            "{outcome:?}"
        );
        assert_eq!(settled(&edf, &outcome), proved(ClosedFormAnalysis::EdfQpa));
    }
}

/// The original B results stay exact independently of the EN-6 quotient.
#[trace("TC-3008", "FR-249-AC-8")]
#[test]
fn the_original_task_set_b_retains_its_response_time_results() {
    b_response_time_results(
        [4, 6, 10],
        10,
        &[3, 6, 7, 9, 10, 10],
        &[4, 7, 10, 11],
        [r(53, 60), r(59, 60)],
    );
}

/// Retiming to (6,6,8) preserves all four policy/work outcomes.
#[trace("TC-3008", "FR-249-AC-8")]
#[test]
fn b_reduced_has_the_approved_response_time_results() {
    b_response_time_results([6, 6, 8], 6, &[3, 6, 6], &[4, 7, 10], [r(7, 8), n(1)]);
}

fn subject(set: TaskSet, claim: ScheduleClaim) -> ScheduleSubject {
    ScheduleSubject::new(set, claim).expect("an admitted claim")
}

fn schedulable(set: TaskSet, policy: SchedulingPolicy) -> ScheduleSubject {
    subject(set, ScheduleClaim::Schedulable(policy))
}

fn run(subject: &ScheduleSubject) -> ClosedFormOutcome {
    analyze(subject, &ScheduleLimits::default(), &Cancel::new())
}

fn check(subject: &ScheduleSubject, outcome: &ClosedFormOutcome) -> ClosedFormCheck {
    recompute(subject, outcome, &Cancel::new())
}

fn settled(subject: &ScheduleSubject, outcome: &ClosedFormOutcome) -> TerminalValue {
    settle(subject, outcome, &Cancel::new())
}

/// Each task's job-0 fixpoint, in the evidence's order.
fn fixpoints(evidence: &[TaskFixpoint]) -> Vec<Rational> {
    evidence
        .iter()
        .map(|entry| {
            entry
                .jobs
                .first()
                .and_then(JobIterates::last)
                .expect("a stated fixpoint")
                .clone()
        })
        .collect()
}

fn iterates(values: &[i64]) -> Vec<Rational> {
    values.iter().map(|value| n(*value)).collect()
}

fn fixed_points(outcome: &ClosedFormOutcome) -> &[TaskFixpoint] {
    let ClosedFormOutcome::Schedulable(ClosedFormEvidence::FixedPoints(listed)) = outcome else {
        panic!("expected fixed-priority fixpoints, got {outcome:?}");
    };
    listed
}

fn amc_evidence(outcome: &ClosedFormOutcome) -> &AmcEvidence {
    let (ClosedFormOutcome::Schedulable(ClosedFormEvidence::Amc(evidence))
    | ClosedFormOutcome::SufficientTestFailed(ClosedFormEvidence::Amc(evidence))) = outcome
    else {
        panic!("expected AMC evidence, got {outcome:?}");
    };
    evidence
}

/// The jitter-and-blocking set of FR-247-AC-2.
fn jitter_and_blocking() -> TaskSet {
    let b = Task {
        jitter: n(1),
        blocking: n(3),
        ..task("b", 2, 6, 2)
    };
    TaskSet::new(set_name("Jb"), vec![task("a", 1, 4, 1), b]).expect("an admitted task set")
}

fn mc(high_budget: i64, low_budget: i64) -> TaskSet {
    TaskSet::new(
        set_name("Mc"),
        vec![
            task("a", 1, 4, 1),
            hi_task("b", low_budget, high_budget, 6, 2),
        ],
    )
    .expect("an admitted task set")
}

/// Premises are part of the subject; routing and the obligation identity are
/// not implemented, so no criterion is bound.
#[test]
fn ctl_is_admitted_under_both_policies_and_its_premises_are_its_identity() {
    let fixed = schedulable(ctl(5), SchedulingPolicy::FixedPriority);
    let edf = schedulable(ctl(5), SchedulingPolicy::Edf);
    assert_eq!(fixed.task_set(), edf.task_set());
    assert_ne!(
        fixed,
        schedulable(ctl(6), SchedulingPolicy::FixedPriority),
        "the third WCET is a premise of the subject"
    );
    assert_eq!(fixed.analysis(), ClosedFormAnalysis::FixedPriorityRta);
    assert_eq!(edf.analysis(), ClosedFormAnalysis::EdfQpa);
}

/// Retained closed-form admission; S3's task-set tests cover source spans,
/// BCET and dimensional refusals, which this rational-only API does not hold.
#[trace("TC-701", "FR-246-AC-2", "FSpec-TC-372", "FSpec-FR-419-AC-5")]
#[test]
fn a_task_set_or_claim_that_breaks_a_rule_is_refused_with_its_code() {
    let name = || set_name("S");
    let refusal = |tasks: Vec<Task>| TaskSet::new(name(), tasks).expect_err("a refusal");

    let empty = refusal(vec![]);
    assert_eq!(empty, TaskSetRefusal::Empty { task_set: name() });
    assert_eq!(empty.code(), Code::InvalidTimedModel);
    assert_eq!(empty.cause(), "invalid-task-set");
    assert_eq!(
        empty.to_string(),
        "invalid_timed_model/invalid-task-set: task set S declares no task"
    );

    let non_positive = |member| TaskSetRefusal::NotPositive {
        task: id("a"),
        member,
    };
    for (invalid, expected) in [
        (task("a", 1, 0, 1), non_positive(TimeMember::Period)),
        (
            Task {
                arrival: Arrival::Sporadic {
                    min_interarrival: n(0),
                },
                ..task("a", 1, 4, 1)
            },
            non_positive(TimeMember::Period),
        ),
        (
            Task {
                deadline: n(0),
                ..task("a", 1, 4, 1)
            },
            non_positive(TimeMember::Deadline),
        ),
        (
            Task {
                deadline: n(-1),
                ..task("a", 1, 4, 1)
            },
            non_positive(TimeMember::Deadline),
        ),
        (task("a", 0, 4, 1), non_positive(TimeMember::Wcet)),
        (hi_task("a", 0, 2, 4, 1), non_positive(TimeMember::Wcet)),
        (hi_task("a", 1, 0, 4, 1), non_positive(TimeMember::Wcet)),
        (
            Task {
                jitter: n(-1),
                ..task("a", 1, 4, 1)
            },
            TaskSetRefusal::Negative {
                task: id("a"),
                member: TimeMember::Jitter,
            },
        ),
        (
            Task {
                blocking: n(-1),
                ..task("a", 1, 4, 1)
            },
            TaskSetRefusal::Negative {
                task: id("a"),
                member: TimeMember::Blocking,
            },
        ),
        (
            hi_task("a", 3, 2, 10, 1),
            TaskSetRefusal::BudgetOrder { task: id("a") },
        ),
        (
            Task {
                wcet: Wcet::Dual { lo: n(1), hi: n(2) },
                ..task("a", 1, 4, 1)
            },
            TaskSetRefusal::LoDualBudgets { task: id("a") },
        ),
    ] {
        let refused = refusal(vec![invalid]);
        assert_eq!(refused, expected);
        assert_eq!(refused.code(), Code::InvalidTimedModel);
        assert_eq!(refused.cause(), "invalid-task-set");
        assert!(refused.to_string().contains("task a"));
    }

    let duplicate = refusal(vec![task("a", 1, 4, 1), task("a", 1, 5, 2)]);
    assert_eq!(duplicate, TaskSetRefusal::DuplicateTask { task: id("a") });
    assert_eq!(duplicate.code(), Code::ConflictingDeclaration);
    assert_eq!(duplicate.cause(), "duplicate-declaration");

    for (tasks, expected, message) in [
        (
            vec![Task {
                priority: None,
                ..task("a", 1, 4, 1)
            }],
            TaskSetRefusal::MissingPriority { task: id("a") },
            "task a states no priority",
        ),
        (
            vec![task("a", 1, 4, 1), task("b", 1, 5, 1)],
            TaskSetRefusal::PriorityClash {
                first: id("a"),
                second: id("b"),
                priority: 1,
            },
            "tasks a and b share priority 1",
        ),
    ] {
        let set = TaskSet::new(name(), tasks).expect("priorities are the claim's");
        for claim in [
            ScheduleClaim::Schedulable(SchedulingPolicy::FixedPriority),
            ScheduleClaim::Schedulable(SchedulingPolicy::Amc),
            ScheduleClaim::Response {
                task: id("a"),
                bound: n(4),
            },
        ] {
            let refused = ScheduleSubject::new(set.clone(), claim).expect_err("invalid priorities");
            assert_eq!(refused, expected);
            assert_eq!(refused.code(), Code::InvalidTimedModel);
            assert_eq!(refused.cause(), "invalid-task-set");
            assert_eq!(
                refused.to_string(),
                format!("invalid_timed_model/invalid-task-set: {message}")
            );
        }
        let edf = schedulable(set, SchedulingPolicy::Edf);
        assert!(matches!(run(&edf), ClosedFormOutcome::Schedulable(_)));
    }

    let beyond = TaskSet::new(
        name(),
        vec![Task {
            deadline: n(5),
            ..task("a", 1, 4, 1)
        }],
    )
    .expect("deadline above period is policy-dependent");
    let refused = ScheduleSubject::new(beyond, ScheduleClaim::Schedulable(SchedulingPolicy::Amc))
        .expect_err("AMC requires constrained deadlines");
    assert_eq!(
        refused,
        TaskSetRefusal::AmcDeadlineBeyondPeriod { task: id("a") }
    );
    assert_eq!(refused.code(), Code::InvalidTimedModel);
    assert_eq!(refused.cause(), "invalid-task-set");
    assert!(refused.to_string().contains("task a"));

    let unknown = ScheduleSubject::new(
        ctl(5),
        ScheduleClaim::Response {
            task: id("zz"),
            bound: n(5),
        },
    )
    .expect_err("a task the set lacks");
    assert_eq!(unknown.code(), Code::MissingDeclaration);
    assert_eq!(unknown.cause(), "missing-name");
}

#[trace("TC-702", "FR-247-AC-1", "FSpec-FR-419-AC-1")]
#[test]
fn ctl_is_schedulable_under_fixed_priority_and_edf() {
    let fixed = run(&schedulable(ctl(5), SchedulingPolicy::FixedPriority));
    let listed = fixed_points(&fixed);
    assert_eq!(fixpoints(listed), vec![n(1), n(3), n(12)]);
    assert_eq!(listed[2].jobs, vec![JobIterates(iterates(&[5, 9, 12, 12]))]);

    let edf = run(&schedulable(ctl(5), SchedulingPolicy::Edf));
    assert_eq!(
        edf,
        ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand {
            busy_period: n(12),
            qpa_sequence: vec![n(8)],
        })
    );
}

#[trace("TC-702", "FR-247-AC-1", "FSpec-TC-372", "FSpec-FR-419-AC-9")]
#[test]
fn a_response_claim_holds_at_twelve_and_misses_at_eleven() {
    let claim = |bound| ScheduleClaim::Response {
        task: id("c"),
        bound: n(bound),
    };
    let holds_subject = subject(ctl(5), claim(12));
    let holds = run(&holds_subject);
    assert_eq!(fixpoints(fixed_points(&holds)), vec![n(12)]);
    assert_eq!(
        settled(&holds_subject, &holds),
        proved(ClosedFormAnalysis::FixedPriorityRta)
    );
    let misses = subject(ctl(5), claim(11));
    let outcome = run(&misses);
    assert_eq!(
        outcome,
        ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
            task: id("c"),
            job: 0,
            iterates: iterates(&[5, 9, 12]),
        })
    );
    assert_eq!(settled(&misses, &outcome), TerminalValue::Refuted);
}

#[trace("TC-702", "FR-247-AC-2", "FSpec-TC-372", "FSpec-FR-419-AC-2")]
#[test]
fn a_third_wcet_of_six_misses_under_fixed_priority_and_edf() {
    assert_eq!(
        run(&schedulable(ctl(6), SchedulingPolicy::FixedPriority)),
        ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
            task: id("c"),
            job: 0,
            iterates: iterates(&[6, 10, 13]),
        })
    );
    assert_eq!(
        run(&schedulable(ctl(6), SchedulingPolicy::Edf)),
        ClosedFormOutcome::Miss(MissEvidence::Utilization(r(13, 12)))
    );
}

#[trace("TC-702", "FR-247-AC-2", "FSpec-TC-372", "FSpec-FR-419-AC-7")]
#[test]
fn jitter_and_blocking_move_the_miss_to_b_and_its_evidence_rechecks() {
    let subject = schedulable(jitter_and_blocking(), SchedulingPolicy::FixedPriority);
    let outcome = run(&subject);
    assert_eq!(
        outcome,
        ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
            task: id("b"),
            job: 0,
            iterates: iterates(&[5, 7]),
        })
    );
    assert_eq!(check(&subject, &outcome), ClosedFormCheck::Agrees);
    assert_eq!(settled(&subject, &outcome), TerminalValue::Refuted);
}

#[trace("TC-702", "FR-247-AC-3", "FSpec-FR-419-AC-3")]
#[test]
fn amc_decides_schedulable_sufficient_test_failed_and_a_low_mode_miss() {
    let base = run(&schedulable(mc(3, 1), SchedulingPolicy::Amc));
    let evidence = amc_evidence(&base);
    assert_eq!(fixpoints(&evidence.low), vec![n(1), n(2)]);
    assert_eq!(fixpoints(&evidence.high), vec![n(4)]);
    assert!(matches!(base, ClosedFormOutcome::Schedulable(_)));

    let high_fails = run(&schedulable(mc(6, 1), SchedulingPolicy::Amc));
    assert!(matches!(
        high_fails,
        ClosedFormOutcome::SufficientTestFailed(_)
    ));

    assert_eq!(
        run(&schedulable(mc(5, 5), SchedulingPolicy::Amc)),
        ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
            task: id("b"),
            job: 0,
            iterates: iterates(&[5, 7]),
        })
    );
}

#[trace("TC-702", "FR-247-AC-4")]
#[test]
fn qpa_stops_at_the_demand_point_limit_naming_it() {
    // Utilization 19/20 < 1; QPA checks 16, 15, 14, 9, 7.
    let set = TaskSet::new(
        set_name("Many"),
        vec![
            Task {
                deadline: n(5),
                ..task("a", 3, 10, 1)
            },
            task("b", 2, 8, 2),
            task("c", 2, 5, 3),
        ],
    )
    .expect("an admitted task set");
    let subject = schedulable(set, SchedulingPolicy::Edf);
    let ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand { qpa_sequence, .. }) =
        run(&subject)
    else {
        panic!("the set is schedulable under the default limit");
    };
    assert_eq!(qpa_sequence, iterates(&[16, 15, 14, 9, 7]));

    let limits = ScheduleLimits {
        max_demand_points: 3,
    };
    assert_eq!(
        analyze(&subject, &limits, &Cancel::new()),
        ClosedFormOutcome::Stopped(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ScheduleMaxDemandPoints,
            value: 3,
            required: None,
        })
    );
}

/// FR-247-AC-5: window iterations of the fixed-priority recurrence and of the
/// EDF busy period charge the same count as QPA demand points, so no
/// recurrence runs past `max_demand_points`.
#[trace("TC-702", "FR-247-AC-5")]
#[test]
fn recurrence_iterations_stop_at_the_demand_point_limit() {
    let limits = ScheduleLimits {
        max_demand_points: 1,
    };
    for policy in [SchedulingPolicy::FixedPriority, SchedulingPolicy::Edf] {
        let subject = schedulable(ctl(5), policy);
        assert!(
            matches!(run(&subject), ClosedFormOutcome::Schedulable(_)),
            "{policy:?} decides under the default limit"
        );
        assert_eq!(
            analyze(&subject, &limits, &Cancel::new()),
            ClosedFormOutcome::Stopped(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::ScheduleMaxDemandPoints,
                value: 1,
                required: None,
            }),
            "{policy:?}"
        );
    }
}

#[trace("TC-702", "FR-247-AC-4")]
#[test]
fn a_cancelled_handle_stops_the_analysis() {
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let outcome = analyze(
        &schedulable(ctl(5), SchedulingPolicy::FixedPriority),
        &ScheduleLimits::default(),
        &cancel,
    );
    assert!(matches!(
        outcome,
        ClosedFormOutcome::Stopped(IncompleteCause::Cancelled { .. })
    ));
}

fn proved(analysis: ClosedFormAnalysis) -> TerminalValue {
    TerminalValue::Proved {
        basis: ProofBasis::ClosedForm { analysis },
        certification: Certification::Certified,
    }
}

/// FSpec FR-419-AC-1: `Ctl` at utilization exactly 1 settles `Proved` under
/// fixed priority, with the fixpoints, and under EDF. FR-248-AC-1's evidence
/// half is not asserted, so it is not bound.
#[trace("TC-703", "FSpec-FR-419-AC-1", "FSpec-FR-419-AC-2")]
#[test]
fn ctl_settles_proved_with_its_fixpoints_and_a_third_wcet_of_six_settles_refuted() {
    let fixed = schedulable(ctl(5), SchedulingPolicy::FixedPriority);
    let outcome = run(&fixed);
    assert_eq!(fixpoints(fixed_points(&outcome)), vec![n(1), n(3), n(12)]);
    assert_eq!(
        settled(&fixed, &outcome),
        proved(ClosedFormAnalysis::FixedPriorityRta)
    );
    let edf = schedulable(ctl(5), SchedulingPolicy::Edf);
    let utilization =
        edf.task_set()
            .tasks()
            .iter()
            .fold(n(0), |sum, task| match (&task.wcet, &task.arrival) {
                (Wcet::Single(c), Arrival::Periodic { period }) => {
                    sum.add(&c.div(period).expect("a positive period"))
                }
                _ => panic!("Ctl is periodic with single budgets"),
            });
    assert_eq!(utilization, n(1));
    assert_eq!(
        settled(&edf, &run(&edf)),
        proved(ClosedFormAnalysis::EdfQpa)
    );

    for policy in [SchedulingPolicy::FixedPriority, SchedulingPolicy::Edf] {
        let missing = schedulable(ctl(6), policy);
        let outcome = run(&missing);
        assert!(matches!(outcome, ClosedFormOutcome::Miss(_)));
        assert_eq!(settled(&missing, &outcome), TerminalValue::Refuted);
        assert_eq!(settled(&missing, &outcome).category(), Category::Violation);
    }
}

fn parity() -> TerminalValue {
    TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
        DisagreementCause::NoValue {
            proved: Verdict::from_category(Category::Success),
            replayed: Verdict::from_category(Category::Inconclusive),
        },
    ))
}

#[trace("TC-703", "FR-248-AC-2", "FSpec-FR-419-AC-4")]
#[test]
fn altered_evidence_settles_inconclusive_replay_parity() {
    let fixed = schedulable(ctl(5), SchedulingPolicy::FixedPriority);
    let mut tampered = run(&fixed);
    let ClosedFormOutcome::Schedulable(ClosedFormEvidence::FixedPoints(listed)) = &mut tampered
    else {
        panic!("Ctl is schedulable");
    };
    listed[2].jobs[0] = JobIterates(iterates(&[5, 9, 11, 11]));
    assert!(matches!(
        check(&fixed, &tampered),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::Iterate { index: 2, .. })
    ));
    assert_eq!(settled(&fixed, &tampered), parity());

    let missing = schedulable(ctl(6), SchedulingPolicy::FixedPriority);
    let mut altered = run(&missing);
    let ClosedFormOutcome::Miss(MissEvidence::FixedPriority { iterates, .. }) = &mut altered else {
        panic!("the third WCET of six misses");
    };
    iterates[2] = n(12);
    assert!(matches!(
        check(&missing, &altered),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::Iterate { .. })
    ));
    let TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(cause)) =
        settled(&missing, &altered)
    else {
        panic!("a disagreement settles replay-parity");
    };
    assert_eq!(
        cause.proved().map(|verdict| verdict.category()),
        Some(Category::Violation)
    );
}

/// `a` (`C = 1`, `T = D = 2`, priority 1) and `b` (`C = 1`, `T = 3`,
/// `D = 1`, blocking 1, priority 2).
fn busy_period_set() -> ScheduleSubject {
    let b = Task {
        deadline: n(1),
        blocking: n(1),
        ..task("b", 1, 3, 2)
    };
    schedulable(
        TaskSet::new(set_name("Bp"), vec![task("a", 1, 2, 1), b]).expect("an admitted task set"),
        SchedulingPolicy::FixedPriority,
    )
}

fn miss_of(job: u64, values: &[i64]) -> ClosedFormOutcome {
    ClosedFormOutcome::Miss(MissEvidence::FixedPriority {
        task: id("b"),
        job,
        iterates: iterates(values),
    })
}

#[trace("TC-703", "FR-248-AC-2", "FSpec-TC-372", "FSpec-FR-419-AC-8")]
#[test]
fn a_miss_for_a_job_past_the_busy_period_names_the_job_that_closes_it() {
    let subject = busy_period_set();
    let outcome = run(&subject);
    assert_eq!(outcome, miss_of(0, &[2]));
    assert_eq!(settled(&subject, &outcome), TerminalValue::Refuted);
    assert_eq!(
        check(&subject, &miss_of(2, &[4, 6, 7, 8])),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::BusyPeriodClosed {
            task: id("b"),
            job: 1,
        })
    );
    assert_eq!(
        settled(&subject, &miss_of(2, &[4, 6, 7, 8])),
        parity_violation()
    );
    assert_eq!(settled(&subject, &miss_of(0, &[2])), TerminalValue::Refuted);
}

fn parity_violation() -> TerminalValue {
    TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
        DisagreementCause::NoValue {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Inconclusive),
        },
    ))
}

#[trace("TC-703", "FR-248-AC-3", "FSpec-FR-419-AC-3")]
#[test]
fn amc_settles_proved_and_a_failed_sufficient_test_settles_inconclusive() {
    let base = schedulable(mc(3, 1), SchedulingPolicy::Amc);
    assert_eq!(
        settled(&base, &run(&base)),
        proved(ClosedFormAnalysis::AmcRtb)
    );
    let high_fails = schedulable(mc(6, 1), SchedulingPolicy::Amc);
    assert_eq!(
        settled(&high_fails, &run(&high_fails)),
        TerminalValue::Inconclusive(InconclusiveCause::SufficientTestFailed)
    );
    let low_miss = schedulable(mc(5, 5), SchedulingPolicy::Amc);
    assert_eq!(settled(&low_miss, &run(&low_miss)), TerminalValue::Refuted);
}

#[test]
fn a_stopped_analysis_settles_incomplete() {
    let subject = schedulable(ctl(5), SchedulingPolicy::Edf);
    let cause = IncompleteCause::LimitReached {
        limit: ModelCheckLimit::ScheduleMaxDemandPoints,
        value: 3,
        required: None,
    };
    let outcome = ClosedFormOutcome::Stopped(cause.clone());
    assert_eq!(
        settled(&subject, &outcome),
        TerminalValue::Incomplete(cause)
    );
}

#[trace("TC-703", "FR-248-AC-2")]
#[test]
fn evidence_of_another_analysis_or_for_another_subject_disagrees() {
    let fixed = schedulable(ctl(5), SchedulingPolicy::FixedPriority);
    let edf = schedulable(ctl(5), SchedulingPolicy::Edf);
    assert_eq!(
        check(&fixed, &run(&edf)),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::Shape)
    );
    // The same evidence checked against a task set with a raised WCET.
    let raised = schedulable(ctl(6), SchedulingPolicy::FixedPriority);
    assert!(matches!(
        check(&raised, &run(&fixed)),
        ClosedFormCheck::Disagrees(_)
    ));
    // A shortened QPA sequence and a wrong busy-period length disagree.
    let ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand { busy_period, .. }) = run(&edf)
    else {
        panic!("Ctl is schedulable under edf");
    };
    let shortened = ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand {
        busy_period: busy_period.clone(),
        qpa_sequence: vec![],
    });
    assert!(matches!(
        check(&edf, &shortened),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::QpaSequence { .. })
    ));
    let wrong = ClosedFormOutcome::Schedulable(ClosedFormEvidence::Demand {
        busy_period: n(11),
        qpa_sequence: vec![n(8)],
    });
    assert_eq!(
        check(&edf, &wrong),
        ClosedFormCheck::Disagrees(ClosedFormDisagreement::BusyPeriodLength)
    );
}

/// An `edf` claim refuses jitter and blocking, which the demand test ignores
/// and fixed priority accounts for; the same sets analyse under fixed
/// priority and an EDF set without them is unchanged.
#[trace("TC-701", "FR-246-AC-2", "FSpec-TC-372", "FSpec-FR-419-AC-10")]
#[test]
fn an_edf_claim_refuses_jitter_and_blocking() {
    let with = |jitter: i64, blocking: i64| {
        let t = Task {
            deadline: n(1),
            jitter: n(jitter),
            blocking: n(blocking),
            ..task("t", 1, 2, 1)
        };
        TaskSet::new(set_name("E"), vec![t]).expect("an admitted task set")
    };
    for (jitter, blocking, member) in [(1, 0, TimeMember::Jitter), (0, 1, TimeMember::Blocking)] {
        let refused = ScheduleSubject::new(
            with(jitter, blocking),
            ScheduleClaim::Schedulable(SchedulingPolicy::Edf),
        )
        .expect_err("an edf claim over jitter or blocking");
        assert_eq!(
            refused,
            TaskSetRefusal::EdfIgnoresTerm {
                task: id("t"),
                member,
            }
        );
        assert_eq!(refused.code(), Code::InvalidTimedModel);
        assert_eq!(refused.cause(), "invalid-task-set");
        assert!(refused.to_string().contains("task t"));
        let fixed = schedulable(with(jitter, blocking), SchedulingPolicy::FixedPriority);
        assert!(matches!(run(&fixed), ClosedFormOutcome::Miss(_)));
    }
    let plain = schedulable(ctl(5), SchedulingPolicy::Edf);
    assert!(matches!(run(&plain), ClosedFormOutcome::Schedulable(_)));
}
