// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-895: replay-mode reading and obligation assessment of protocol recordings.

use fsl_analyze::protocol_analysis::recording::{
    RecordingFact, RecordingVerdict, assess_recording,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::order_protocol;

fn protocol() -> fsl_package::CheckedPackage {
    crate::litmus::package_called("analysis-unit", &order_protocol(true))
}

fn recording(observations: Value, local_order: Value, closure_state: &str) -> Value {
    json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": observations,
        "local_order": local_order,
        "correlations": [{"send": 0, "receive": 1}],
        "closure": {"state": closure_state, "premises": []}
    })
}

fn paid_protocol() -> fsl_package::CheckedPackage {
    crate::fill::package(&crate::protocol_compensation::pay("true"))
}

fn paid_protocol_window() -> fsl_package::CheckedPackage {
    let protocol =
        crate::protocol_compensation::pay("true").replace("within [0, 10]", "within [0, 120]");
    crate::fill::package(&protocol)
}

fn delivery_protocol(policy: &str) -> fsl_package::CheckedPackage {
    let protocol = crate::analyze_cli_support::order_protocol(true).replace(
        "delivery exactly-once-premise",
        &format!("delivery {policy}"),
    );
    crate::litmus::package_called("analysis-unit", &protocol)
}

fn told_protocol() -> fsl_package::CheckedPackage {
    let source = crate::analyze_cli_support::order_protocol(true)
        .replace(
            "run sequence Main {",
            "channel Tell from Seller to Buyer carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\nrun sequence Main {",
        )
        .replace(
            "sequence Ship { }",
            "sequence Ship { send Told via Tell as (told: Ack) { true }; }",
        );
    crate::litmus::package_called("analysis-unit", &source)
}

fn paid_recording(observations: Value) -> Value {
    let local_order: Vec<usize> =
        (0..observations.as_array().expect("observations").len()).collect();
    json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Pay",
        "observations": observations,
        "local_order": {"w": local_order},
        "correlations": [],
        "closure": {"state": "open", "premises": []}
    })
}

fn duplicate_delivery_recording() -> Value {
    json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": [
            {"kind": "send", "node": "Flow::Main::Placed", "role": "Buyer",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-0"},
             "record": {"ok": true}},
            {"kind": "receive", "node": "Flow::Main::Got", "role": "Seller",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-0"},
             "record": {"ok": true}},
            {"kind": "receive", "node": "Flow::Main::Got", "role": "Seller",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
             "record": {"ok": true}}
        ],
        "local_order": {"Buyer": [0], "Seller": [1, 2]},
        "correlations": [{"send": 0, "receive": 1}, {"send": 0, "receive": 2}],
        "closure": {"state": "open", "premises": []}
    })
}

fn assess_duplicate_delivery(
    policy: &str,
) -> fsl_analyze::protocol_analysis::recording::RecordingAssessment {
    let package = delivery_protocol(policy);
    let run = duplicate_delivery_recording();
    assess_recording(&package, run.to_string().as_bytes()).expect("readable delivery recording")
}

fn compensation_attempt(identity: &str, outcome: &str, retry_of: Option<&str>) -> Value {
    let mut keys = json!({
        "compensation_attempt": identity,
        "registration": "registration-1"
    });
    if let Some(predecessor) = retry_of {
        keys["retry_of"] = json!(predecessor);
    }
    json!({
        "kind": "compensation-attempt",
        "node": "Pay::Main::Paid",
        "role": "w",
        "instance": "instance-1",
        "keys": keys,
        "outcome": outcome
    })
}

fn recovery(outcome: &str) -> Value {
    json!({
        "kind": "recovery",
        "node": "Pay::Main::Paid",
        "role": "w",
        "instance": "instance-1",
        "keys": {"registration": "registration-1"},
        "outcome": outcome
    })
}

fn paid_effect(identity: &str, attempt: &str) -> Value {
    paid_effect_at(identity, attempt, 0)
}

fn paid_effect_at(identity: &str, attempt: &str, ordinal: u64) -> Value {
    json!({
        "kind": "effect",
        "node": "Pay::Main::Paid",
        "role": "w",
        "instance": "instance-1",
        "keys": {
            "business_effect": identity,
            "attempt": attempt,
            "registration": "registration-1"
        },
        "occurrence": {"path": [], "ordinal": ordinal}
    })
}

fn assess_paid(
    observations: Value,
) -> fsl_analyze::protocol_analysis::recording::RecordingAssessment {
    let package = paid_protocol();
    let run = paid_recording(observations);
    assess_recording(&package, run.to_string().as_bytes()).expect("readable paid recording")
}

fn complete_observations() -> Value {
    json!([
        {"kind": "send", "node": "Flow::Main::Placed", "role": "Buyer",
         "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
         "record": {"ok": true}},
        {"kind": "receive", "node": "Flow::Main::Got", "role": "Seller",
         "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
         "record": {"ok": true}},
        {"kind": "case", "node": "Flow::Main::Decide::yes", "role": "Seller",
         "instance": "w-1", "keys": {}},
        {"kind": "finish", "node": "Flow::Done", "role": "Seller",
         "instance": "w-1", "keys": {}, "record": {"zz": true}}
    ])
}

/// Trace: FR-895-AC-1, FR-895-AC-3.
#[trace("FR-895-AC-1", "FR-895-AC-3")]
#[test]
fn a_closed_recording_assesses_observed_and_unobserved_obligations() {
    let package = protocol();
    let closed = recording(
        complete_observations(),
        json!({"Buyer": [0], "Seller": [1, 2, 3]}),
        "closed",
    );
    let assessed = assess_recording(&package, closed.to_string().as_bytes()).expect("readable run");
    assert_eq!(
        assessed.global.label, "proved",
        "{:#?}",
        assessed.obligations
    );
    assert!(
        assessed
            .obligations
            .iter()
            .all(|entry| entry.verdict == RecordingVerdict::Satisfied)
    );

    let mut cut = complete_observations();
    cut.as_array_mut().expect("array").truncate(2);
    let open = recording(cut, json!({"Buyer": [0], "Seller": [1]}), "open");
    let assessed = assess_recording(&package, open.to_string().as_bytes()).expect("readable run");
    assert_eq!(assessed.global.label, "inconclusive");
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "control" && entry.verdict == RecordingVerdict::Pending
    }));
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "closure" && entry.verdict == RecordingVerdict::Pending
    }));
}

/// Trace: FR-895-AC-4.
#[trace("FR-895-AC-4")]
#[test]
fn an_undeclared_role_is_a_correspondence_refusal() {
    let package = protocol();
    let run = recording(
        json!([{
            "kind": "send", "node": "Flow::Main::Placed", "role": "Courier",
            "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"}
        }]),
        json!({"Courier": [0]}),
        "open",
    );
    let refusal =
        assess_recording(&package, run.to_string().as_bytes()).expect_err("role is absent");
    assert_eq!(refusal.code().cause(), "missing-name");
    assert_eq!(refusal.position(), Some(0));
}

/// Trace: FR-895-AC-5.
#[trace("FR-895-AC-5")]
#[test]
fn a_declared_but_wrongly_owned_role_is_an_unexpected_observation() {
    let package = protocol();
    let run = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": [{
            "kind": "send", "node": "Flow::Main::Placed", "role": "Seller",
            "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
            "record": {"ok": true}
        }],
        "local_order": {"Seller": [0]},
        "correlations": [],
        "closure": {"state": "open", "premises": []}
    });
    let assessed =
        assess_recording(&package, run.to_string().as_bytes()).expect("known role and node");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "enabledness"
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause
                    == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Unexpected)
                && entry.witness == [0]
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895-AC-5.
#[trace("FR-895-AC-5")]
#[test]
fn a_send_in_an_unselected_choice_branch_is_unexpected() {
    let package = told_protocol();
    let run = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": [{
            "kind": "send", "node": "Flow::Main::Decide::yes::Ship::Told", "role": "Seller",
            "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
            "record": {"ok": true}
        }],
        "local_order": {"Seller": [0]},
        "correlations": [],
        "closure": {"state": "open", "premises": []}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("known send node");
    assert_eq!(assessed.global.label, "refuted");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "enabledness"
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause
                    == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Unexpected)
                && entry.witness == [0]
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: TC-2530.
#[trace("TC-2530")]
#[test]
fn a_false_bound_event_constraint_is_a_violated_obligation() {
    let source = crate::analyze_cli_support::order_protocol(true).replace(
        "send Placed via Order as (placed: Ack) { true };",
        "send Placed via Order as (placed: Ack) { placed.ok };",
    );
    let package = crate::litmus::package_called("analysis-unit", &source);
    let run = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": [{
            "kind": "send", "node": "Flow::Main::Placed", "role": "Buyer",
            "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
            "record": {"ok": false}
        }],
        "local_order": {"Buyer": [0]},
        "correlations": [],
        "closure": {"state": "open", "premises": []}
    });
    let assessed =
        assess_recording(&package, run.to_string().as_bytes()).expect("known event node");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "constraint"
                && entry.obligation.node.as_deref() == Some("Flow::Main::Placed")
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause
                    == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Unexpected)
                && entry.witness == [0]
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895-AC-2.
#[trace("FR-895-AC-2")]
#[test]
fn receive_before_send_is_a_causal_order_violation() {
    let source = crate::analyze_cli_support::order_protocol(true)
        .replace(
            "channel Order from Buyer to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;",
            "channel Order from Buyer to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\nchannel Return from Seller to Buyer carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;",
        )
        .replace(
            "receive Got via Order of Placed as (got: Ack) { true };",
            "receive Got via Order of Placed as (got: Ack) { true };\nsend Confirm via Return as (confirm: Ack) { true };\nreceive Confirmed via Return of Confirm as (confirmed: Ack) { true };",
        );
    let package = crate::litmus::package_called("analysis-unit", &source);
    let run = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": [
            {"kind": "receive", "node": "Flow::Main::Got", "role": "Seller",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
             "record": {"ok": true}},
            {"kind": "send", "node": "Flow::Main::Confirm", "role": "Seller",
             "instance": "w-1", "keys": {"message": "m-2", "delivery": "d-2"},
             "record": {"ok": true}},
            {"kind": "receive", "node": "Flow::Main::Confirmed", "role": "Buyer",
             "instance": "w-1", "keys": {"message": "m-2", "delivery": "d-2"},
             "record": {"ok": true}},
            {"kind": "send", "node": "Flow::Main::Placed", "role": "Buyer",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
             "record": {"ok": true}}
        ],
        "local_order": {"Seller": [0, 1], "Buyer": [2, 3]},
        "correlations": [{"send": 1, "receive": 2}, {"send": 3, "receive": 0}],
        "closure": {"state": "closed", "premises": []}
    });
    let assessed =
        assess_recording(&package, run.to_string().as_bytes()).expect("well-formed recording");
    assert_eq!(assessed.global.label, "refuted");
    assert_eq!(
        assessed.trace_position,
        Some(0),
        "{:#?}",
        assessed.obligations
    );
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "causal-order"
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause
                    == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Misordered)
                && entry.witness == [0, 3]
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895-AC-6.
#[trace("FR-895-AC-6")]
#[test]
fn compensation_windows_use_declared_clock_and_remain_pending_without_a_reading() {
    let package = paid_protocol_window();
    let assess = |clocks: Value, closure: &str| {
        let mut attempt = compensation_attempt("comp-1", "successful", None);
        attempt["clocks"] = clocks;
        let observations = json!([attempt]);
        let mut run = paid_recording(observations.clone());
        run["closure"]["state"] = json!(closure);
        assess_recording(&package, run.to_string().as_bytes()).expect("readable recording")
    };

    let out_of_window = assess(json!({"c": 130}), "open");
    assert!(
        out_of_window.obligations.iter().any(|entry| {
            entry.obligation.kind == "window"
                && entry.verdict == RecordingVerdict::Violated
                && entry.witness == [0]
        }),
        "{:#?}",
        out_of_window.obligations
    );

    let missing_reading = assess(json!({}), "open");
    assert!(
        missing_reading.obligations.iter().any(|entry| {
            entry.obligation.kind == "window" && entry.verdict == RecordingVerdict::Pending
        }),
        "{:#?}",
        missing_reading.obligations
    );

    let assert_batch_incremental_parity = |clocks: Value| {
        let mut observation = compensation_attempt("comp-1", "successful", None);
        observation["clocks"] = clocks;
        let recording = paid_recording(json!([observation]));
        let batch =
            assess_recording(&package, recording.to_string().as_bytes()).expect("batch assessment");
        let mut incremental = None;
        for observations in recording["observations"].as_array().expect("observations") {
            let prefix = paid_recording(json!([observations]));
            incremental = Some(
                assess_recording(&package, prefix.to_string().as_bytes())
                    .expect("incremental assessment"),
            );
        }
        assert_eq!(incremental.expect("one recorded attempt"), batch);
    };
    assert_batch_incremental_parity(json!({"c": 130}));
    assert_batch_incremental_parity(json!({}));
}

/// Trace: FR-894-AC-2, TC-2528.
#[trace("TC-2528", "FR-894-AC-2")]
#[test]
fn two_effect_keys_for_one_paid_occurrence_report_a_duplicate_fact() {
    let assessed = assess_paid(json!([
        paid_effect("effect-1", "operation-1"),
        paid_effect("effect-2", "operation-1"),
        compensation_attempt("comp-1", "successful", None),
        recovery("successful")
    ]));

    let duplicates: Vec<_> = assessed
        .facts
        .iter()
        .filter_map(|fact| match fact {
            RecordingFact::DuplicateBusinessEffect {
                registration,
                occurrence,
                business_effect_keys,
                value,
            } => Some((registration, occurrence, business_effect_keys, value)),
            _ => None,
        })
        .collect();
    assert_eq!(duplicates.len(), 1, "{:#?}", assessed.facts);
    assert_eq!(duplicates[0].0, &Some(json!("registration-1")));
    assert_eq!(duplicates[0].1.instance, "instance-1");
    assert_eq!(duplicates[0].1.node, "Pay::Main::Paid");
    assert_eq!(duplicates[0].1.path, json!([]));
    assert_eq!(duplicates[0].1.ordinal, 0);
    assert_eq!(duplicates[0].2, &vec![json!("effect-1"), json!("effect-2")]);
    assert_eq!(duplicates[0].3, "duplicate");
    let facts = serde_json::to_value(&assessed.facts).expect("facts serialize");
    assert_eq!(facts[2]["kind"], "duplicate-business-effect");
    assert!(assessed.facts.iter().any(|fact| matches!(fact,
        RecordingFact::AttemptOutcome { outcome, .. } if outcome == "successful")));
    assert!(assessed.facts.iter().any(|fact| matches!(fact,
        RecordingFact::Recovery { outcome, .. } if outcome == "successful")));
}

/// Trace: FR-894-AC-3, TC-2528.
#[trace("TC-2528", "FR-894-AC-3")]
#[test]
fn failed_attempt_then_successful_retry_has_one_effect_and_no_duplicate() {
    let assessed = assess_paid(json!([
        paid_effect("effect-2", "operation-2"),
        compensation_attempt("comp-1", "failed", None),
        compensation_attempt("comp-2", "successful", Some("comp-1")),
        recovery("successful")
    ]));

    assert!(
        !assessed
            .facts
            .iter()
            .any(|fact| matches!(fact, RecordingFact::DuplicateBusinessEffect { .. }))
    );
    let attempts: Vec<_> = assessed
        .facts
        .iter()
        .filter_map(|fact| match fact {
            RecordingFact::AttemptOutcome {
                attempt,
                retry_of,
                outcome,
                ..
            } => Some((attempt, retry_of, outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(attempts.len(), 2, "{:#?}", assessed.facts);
    assert_eq!(attempts[0].2, "failed");
    assert_eq!(attempts[1].2, "successful");
    assert_eq!(attempts[1].1, &Some(json!("comp-1")));
    assert!(assessed.facts.iter().any(|fact| matches!(fact,
        RecordingFact::Recovery { outcome, .. } if outcome == "successful")));
}

/// Trace: FR-894-AC-3, TC-2528.
#[trace("TC-2528", "FR-894-AC-3")]
#[test]
fn two_successful_attempt_effects_keep_outcomes_beside_the_duplicate_fact() {
    let assessed = assess_paid(json!([
        paid_effect("effect-1", "operation-1"),
        paid_effect("effect-2", "operation-2"),
        compensation_attempt("comp-1", "successful", None),
        compensation_attempt("comp-2", "successful", Some("comp-1")),
        recovery("successful")
    ]));

    assert_eq!(
        assessed
            .facts
            .iter()
            .filter(|fact| matches!(fact, RecordingFact::DuplicateBusinessEffect { .. }))
            .count(),
        1,
        "{:#?}",
        assessed.facts
    );
    assert_eq!(
        assessed
            .facts
            .iter()
            .filter(|fact| matches!(fact, RecordingFact::AttemptOutcome { outcome, .. } if outcome == "successful"))
            .count(),
        2,
        "{:#?}",
        assessed.facts
    );
    assert_eq!(
        assessed
            .facts
            .iter()
            .filter(|fact| matches!(fact, RecordingFact::Recovery { outcome, .. } if outcome == "successful"))
            .count(),
        1,
        "{:#?}",
        assessed.facts
    );
}

/// Trace: FR-894-AC-4, TC-2528.
#[trace("TC-2528", "FR-894-AC-4")]
#[test]
fn distinct_effect_occurrences_and_repeated_effect_keys_are_not_duplicates() {
    let distinct = assess_paid(json!([
        paid_effect_at("effect-1", "operation-1", 0),
        paid_effect_at("effect-2", "operation-2", 1)
    ]));
    assert!(
        !distinct
            .facts
            .iter()
            .any(|fact| matches!(fact, RecordingFact::DuplicateBusinessEffect { .. }))
    );

    let repeated_effect = paid_effect("effect-replayed", "operation-replayed");
    let replayed = assess_paid(json!([repeated_effect.clone(), repeated_effect]));
    assert!(
        !replayed
            .facts
            .iter()
            .any(|fact| matches!(fact, RecordingFact::DuplicateBusinessEffect { .. }))
    );
    assert_eq!(
        replayed
            .facts
            .iter()
            .filter(|fact| matches!(fact, RecordingFact::ReplayedObservation { .. }))
            .count(),
        1,
        "{:#?}",
        replayed.facts
    );
}

/// Trace: FR-894-AC-5, TC-2529.
#[trace("TC-2529", "FR-894-AC-5")]
#[test]
fn duplicate_delivery_is_classified_against_the_channel_premise() {
    let admitted = assess_duplicate_delivery("at-least-once");
    assert!(admitted.facts.iter().any(|fact| matches!(
        fact,
        RecordingFact::DuplicateDelivery {
            channel,
            delivery_keys,
            value,
            ..
        } if channel == "Order" && delivery_keys == &vec![json!("d-0"), json!("d-1")] && value == "duplicate"
    )));
    assert!(admitted.obligations.iter().any(|entry| {
        entry.obligation.kind == "channel-premise"
            && entry.obligation.node.as_deref() == Some("Order")
            && entry.verdict == RecordingVerdict::Satisfied
    }));

    let refused = assess_duplicate_delivery("exactly-once-premise");
    assert!(
        refused
            .facts
            .iter()
            .any(|fact| matches!(fact, RecordingFact::DuplicateDelivery { .. }))
    );
    assert!(refused.obligations.iter().any(|entry| {
        entry.obligation.kind == "channel-premise"
            && entry.obligation.node.as_deref() == Some("Order")
            && entry.verdict == RecordingVerdict::Violated
            && entry.cause
                == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Duplicated)
    }));
}
