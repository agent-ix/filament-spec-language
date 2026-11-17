// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-893-AC-1 to AC-5: compensation attempt, aggregate and instance outcomes.

use std::collections::BTreeSet;

use fsl_eval::simulation::SubjectSystem;
use fsl_eval::simulation::protocol::{ProtocolSystem, Sc, StepKind};
use ix_trace_rs::trace;
use serde_json::Value as Json;

use crate::fill::{contracts, json_of, protocol};
use crate::protocol_compensation::{REFUND, entries, explore, key_json, pay, pay_run, refund};
use crate::protocol_system::Case;

fn attempt_outcomes(reached: &[crate::fill::Reached]) -> BTreeSet<String> {
    reached
        .iter()
        .flat_map(|state| state.steps.iter())
        .filter_map(|(step, _)| step.compensation_outcome().map(str::to_owned))
        .collect()
}

fn summaries(
    system: &ProtocolSystem<'_, Sc>,
    reached: &[crate::fill::Reached],
) -> BTreeSet<String> {
    reached
        .iter()
        .map(|state| key_json(system, state))
        .filter_map(|key| {
            entries(&key, "control")
                .into_iter()
                .find(|(identity, _)| identity["type"] == "protocol-instance")
                .and_then(|(_, value)| value["recovery_summary"].as_str().map(str::to_owned))
        })
        .collect()
}

fn registrations(key: &Json) -> Vec<Json> {
    entries(key, "control")
        .into_iter()
        .filter(|(identity, _)| identity["type"] == "registration")
        .map(|(_, value)| value)
        .collect()
}

#[trace("TC-2526", "FR-893-AC-1")]
#[test]
fn a_failed_compensation_is_reported() {
    explore(&pay("false"), |_, explored| {
        assert!(explored.reached.iter().any(|state| {
            state.steps.iter().any(|(step, _)| {
                step.kind() == StepKind::CFail && step.compensation_outcome() == Some("failed")
            })
        }));
    });
}

#[trace("TC-2526", "FR-893-AC-1", "FR-893-AC-2")]
#[test]
fn a_timed_out_compensation_is_reported() {
    explore(&pay("false"), |system, explored| {
        assert!(explored.reached.iter().any(|state| {
            state.steps.iter().any(|(step, _)| {
                step.kind() == StepKind::CTimeout
                    && step.compensation_outcome() == Some("timed-out")
            })
        }));
        assert!(explored.reached.iter().any(|state| {
            state
                .steps
                .iter()
                .any(|(step, next)| {
                    step.kind() == StepKind::CExpire
                        && registrations(&key_json(system, &explored.reached[*next]))
                            .iter()
                            .any(|value| value["recovery"] == "timed-out")
                })
        }));
    });
}

#[trace("TC-2526", "FR-893-AC-1", "FR-893-AC-4")]
#[test]
fn a_retried_compensation_is_reported() {
    let body = pay("false")
        .replace("attempts 1 of M::Attempt;", "attempts 2 of M::Attempt;")
        .replace(
            "retry (e: M::Attempt, l: M::Attempt) { false }",
            "retry (e: M::Attempt, l: M::Attempt) { true }",
        );
    explore(&body, |system, explored| {
        let has_failed_then_retry = explored.reached.iter().any(|state| {
            registrations(&key_json(system, state))
                .iter()
                .any(|registration| {
                    let attempts = registration["attempts"].as_array().unwrap();
                    attempts.iter().any(|attempt| {
                        attempt["outcome"] == "failed"
                            && attempts.iter().any(|later| {
                                later["outcome"] == "successful" && later["retry_of"] == "1"
                            })
                    })
                })
        });
        assert!(
            has_failed_then_retry,
            "the first attempt stays failed after a successful retry"
        );
    });
}

#[trace("TC-2527", "FR-893-AC-3", "FR-893-AC-5")]
#[test]
fn an_untriggered_compensation_is_reported() {
    explore(&pay("true"), |system, explored| {
        assert!(summaries(system, &explored.reached).contains("untriggered"));
        assert!(explored.reached.iter().any(|state| {
            state
                .steps
                .iter()
                .any(|(step, _)| step.kind() == StepKind::Finish)
                && registrations(&key_json(system, state))
                    .iter()
                    .all(|registration| {
                        registration["attempts"].as_array().unwrap().is_empty()
                            && registration.get("recovery").is_none()
                    })
        }));
    });
}

#[trace("TC-2527", "FR-893-AC-3")]
#[test]
fn a_partially_recovered_saga_is_reported() {
    let first = refund("Main::Paid", "dec", 1, "false", "never", "true")
        .replace("compensate Refund", "compensate RefundA");
    let second = refund("Main::Paid", "touch", 1, "false", "never", "false")
        .replace("compensate Refund", "compensate RefundB")
        .replace("(fwd:", "(fwdB:")
        .replace("(f: Boolean)", "(fB: Boolean)")
        .replace("when { f }", "when { fB }")
        .replace("(e: M::Attempt", "(eB: M::Attempt")
        .replace(", l: M::Attempt", ", lB: M::Attempt")
        .replace("(r: M::Attempt", "(rB: M::Attempt");
    let template = format!("{first}\n{second}");
    let body = contracts("self.v = 0") + REFUND + &protocol("Pay", &template, &pay_run(""), "d");
    explore(&body, |system, explored| {
        assert!(summaries(system, &explored.reached).contains("partially-recovered"));
        assert!(explored.reached.iter().any(|state| {
            state.steps.iter().any(|(step, _)| {
                step.kind() == StepKind::Finish
                    && json_of(&system.transition_key(step).unwrap()).get("recovery_summary")
                        == Some(&Json::String("partially-recovered".to_owned()))
            })
        }));
    });
}

#[trace("TC-2527", "FR-893-AC-4")]
#[test]
fn the_five_outcomes_are_distinct() {
    explore(&pay("false"), |system, explored| {
        let outcomes = attempt_outcomes(&explored.reached);
        assert_eq!(
            outcomes,
            BTreeSet::from([
                "failed".to_owned(),
                "successful".to_owned(),
                "timed-out".to_owned()
            ])
        );
        assert!(summaries(system, &explored.reached).contains("unrecovered"));
        assert!(explored.reached.iter().any(|state| {
            state
                .steps
                .iter()
                .any(|(step, _)| step.kind() == StepKind::CExpire)
        }));
    });
}

#[trace("TC-2527", "FR-893-AC-5")]
#[test]
fn a_clean_saga_reports_no_outcome() {
    let body = pay("false");
    let case = Case::new(&body, &[0]);
    let subject = case.subject("Pay");
    let system = crate::fill::sc_system(&subject);
    let reached = crate::fill::reachable(&system);
    let no_trigger_finish = reached.iter().any(|state| {
        state
            .steps
            .iter()
            .any(|(step, _)| step.kind() == StepKind::Finish)
            && registrations(&key_json(&system, state))
                .iter()
                .all(|registration| {
                    registration["attempts"].as_array().unwrap().is_empty()
                        && registration.get("recovery").is_none()
                })
    });
    assert!(
        no_trigger_finish,
        "the false trigger path has no attempt or aggregate fact"
    );
}
