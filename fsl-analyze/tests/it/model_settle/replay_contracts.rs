// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real deadlock, terminal, undefined and limited runs through settlement/replay.

use super::*;
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::{StateKey, TransitionSystem};

fn record(case: &Case) -> fsl_replay::ModelCheckRecord {
    settle_model_check(
        &case.outcome,
        &ModelCheckLimits::default(),
        &case.request(),
        &Cancel::new(),
        |witness| replay(case, witness.clone()),
    )
}

#[trace("FR-124-AC-3", "FR-127-AC-1", "FR-128-AC-2")]
#[test]
fn terminal_predicates_do_not_turn_enabled_states_into_early_terminal_states() {
    for (terminal, intended) in [
        ("", false),
        (
            "terminal Done using v on M when { self.value = 2 };\n",
            false,
        ),
        (
            "terminal Done using v on M when { self.value = 3 };\n",
            true,
        ),
        ("terminal Done using v on M any;\n", true),
    ] {
        let case = case(&with_terminal(terminal), &[2]);
        let settled = record(&case);
        if intended {
            assert_eq!(
                settled.value,
                TerminalValue::Proved {
                    basis: fsl_replay::ProofBasis::Exhaustive,
                    certification: fsl_replay::Certification::Certified,
                }
            );
            assert_eq!(settled.counterexample, None);
        } else {
            assert_eq!(settled.value, TerminalValue::Refuted);
            assert_eq!(settled.basis, SettlementBasis::DecisiveCounterexample);
            assert_eq!(settled.truth, Truth::Violated);
            let witness = counterexample(&case);
            assert_eq!(witness.kind, CounterexampleKind::Deadlock);
            assert_eq!(witness.prefix.len(), 1);
            let system =
                ModelSystem::new(case.request().subject.simulation).expect("the model admits");
            let expansion = system
                .successors(&system.initial()[0])
                .expect("the enabled state expands");
            let [(transition, state)] = expansion.successors.as_slice() else {
                panic!("one actual inc successor")
            };
            assert_eq!(crate::counter::value(state, "c"), 3);
            assert_eq!(&witness.prefix[0].transition, transition);
            assert_eq!(
                witness.prefix[0].post_state,
                StateKey::from_canonical_bytes(system.state_key(state).expect("a key").0)
            );
            assert_eq!(settled.counterexample.as_ref(), Some(&witness));
            assert!(matches!(
                replay(&case, witness)
                    .expect("the returned witness admits")
                    .settlement,
                ModelTraceSettlement::Reproduced {
                    value: ModelTraceValue::Truth(false),
                    ..
                }
            ));
        }
    }
}

#[trace("FR-128-AC-3")]
#[test]
fn a_returned_counterexample_rejects_an_out_of_range_initial_state() {
    let case = case(CONTRACT, &[2]);
    let returned = counterexample(&case);
    let mut wrong_initial = returned.clone();
    wrong_initial.initial_state = 1;
    let error = replay(&case, wrong_initial).expect_err("the initial index is outside the subject");
    assert_eq!(error.code(), Code::InvalidRuntimeInput);
    assert_eq!(error.cause(), Some("invalid-value"));
    let ReplayRefusal::ModelTrace(reason) = error else {
        panic!("typed trace refusal")
    };
    assert!(matches!(
        *reason,
        ModelTraceRefusal::InitialState { index: 1, count: 1 }
    ));
}

#[trace("FR-128-AC-3")]
#[test]
#[ignore = "QSL-2702: enabled temporal replay edges with a wrong target must retain PostState classification"]
fn a_returned_counterexample_rejects_a_different_real_post_state_as_content_mismatch() {
    let case = case(CONTRACT, &[2]);
    let returned = counterexample(&case);
    let system = ModelSystem::new(case.request().subject.simulation).expect("the model admits");
    let initial = system.initial();
    let mut wrong_target = returned;
    wrong_target.prefix[0].post_state =
        StateKey::from_canonical_bytes(system.state_key(&initial[0]).expect("initial key").0);
    let error = replay(&case, wrong_target).expect_err("inc cannot remain at value2");
    assert_eq!(error.code(), Code::StaleDependency);
    assert_eq!(error.cause(), Some("content-mismatch"));
    let ReplayRefusal::ModelTrace(reason) = error else {
        panic!("typed trace refusal")
    };
    assert!(matches!(
        *reason,
        ModelTraceRefusal::PostState { step: 0, .. }
    ));
}

#[trace("FR-127-AC-1", "FR-127-AC-4", "FR-128-AC-6")]
#[test]
fn returned_undefined_and_actual_state_limit_runs_settle_without_invented_outcomes() {
    let undefined = kernel_case(&with_terminal(TERMINAL_WHEN), DIVIDE, &[2]);
    let settled = record(&undefined);
    assert_eq!(settled.value, TerminalValue::Refuted);
    assert_eq!(settled.basis, SettlementBasis::DecisiveCounterexample);
    let witness = settled
        .counterexample
        .expect("refutation has its replayed counterexample");
    let CounterexampleKind::UndefinedEvaluation(evaluation) = &witness.kind else {
        panic!("the reached terminal predicate divides by zero")
    };
    assert_eq!(evaluation.site.position, 1);
    assert_eq!(
        evaluation.cause,
        UndefinedCause::Kernel(filament_exact::Undefined::DivisionByZero)
    );
    assert!(matches!(
        replay(&undefined, witness)
            .expect("the recorded mode is preserved")
            .settlement,
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Undefined(_),
            trace_position: Some(1)
        }
    ));

    let plain = case(CONTRACT, &[0]);
    let mut item = plain.model_item();
    item.request.limits.limits.max_states = 1;
    let limits = item.request.limits;
    let outcome = check_model(item.request, &Cancel::new()).expect("the request admits");
    let settled = settle_model_check(&outcome, &limits, &plain.request(), &Cancel::new(), |_| {
        panic!("a limit carries no counterexample to replay")
    });
    assert_eq!(
        settled.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::MaxStates,
            value: 1
        })
    );
    assert_eq!(settled.label, ResultLabel::Failed);
    assert_eq!(settled.execution, Execution::ResourceIncomplete);
    assert_eq!(settled.truth, Truth::Unavailable);
    assert_eq!(settled.counterexample, None);
}
