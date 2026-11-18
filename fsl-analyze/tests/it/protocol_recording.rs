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
    let premises = if closure_state == "closed" {
        json!([
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Decide::no"},
            {"kind":"branch", "node":"Flow::Main::Decide::yes"}
        ])
    } else {
        json!([])
    };
    json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": observations,
        "local_order": local_order,
        "correlations": [{"send": 0, "receive": 1}],
        "closure": {"state": closure_state, "premises": premises}
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
    assert_eq!(
        assessed.global.label, "inconclusive",
        "{:#?}",
        assessed.obligations
    );
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "control" && entry.verdict == RecordingVerdict::Pending
    }));
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "closure" && entry.verdict == RecordingVerdict::Pending
    }));
}

/// Trace: FR-895 control obligations.
#[trace("FR-895-AC-1")]
#[test]
fn a_false_control_guard_on_a_second_instance_refutes() {
    let package = protocol();
    let run = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": [
            {"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}},
            {"kind":"receive", "node":"Flow::Main::Got", "role":"Seller", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}},
            {"kind":"case", "node":"Flow::Main::Decide::yes", "role":"Seller", "instance":"w-1", "keys":{}},
            {"kind":"finish", "node":"Flow::Done", "role":"Seller", "instance":"w-1", "keys":{}, "record":{"zz":true}},
            {"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer", "instance":"w-2", "keys":{"message":"m-2", "delivery":"d-2"}, "record":{"ok":false}},
            {"kind":"receive", "node":"Flow::Main::Got", "role":"Seller", "instance":"w-2", "keys":{"message":"m-2", "delivery":"d-2"}, "record":{"ok":false}},
            {"kind":"case", "node":"Flow::Main::Decide::yes", "role":"Seller", "instance":"w-2", "keys":{}},
            {"kind":"finish", "node":"Flow::Done", "role":"Seller", "instance":"w-2", "keys":{}, "record":{"zz":true}}
        ],
        "local_order":{"Buyer":[0,4], "Seller":[1,2,3,5,6,7]},
        "correlations":[{"send":0,"receive":1}, {"send":4,"receive":5}],
        "closure":{"state":"closed", "premises":[
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Decide::no"},
            {"kind":"branch", "node":"Flow::Main::Decide::yes"}
        ]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert_eq!(
        assessed.global.label, "refuted",
        "{:#?}",
        assessed.obligations
    );
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "control"
            && entry.obligation.instance.as_deref() == Some("w-2")
            && entry.obligation.node.as_deref() == Some("Flow::Main::Decide")
            && entry.verdict == RecordingVerdict::Violated
    }));
}

/// Trace: FR-895 closed-history scope.
#[trace("FR-895-AC-3")]
#[test]
fn an_empty_closed_recording_does_not_prove_vacuously() {
    let package = protocol();
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Flow",
        "observations":[], "local_order":{}, "correlations":[],
        "closure":{"state":"closed", "premises":[
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Decide::no"},
            {"kind":"branch", "node":"Flow::Main::Decide::yes"}
        ]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert_ne!(
        assessed.global.label, "proved",
        "{:#?}",
        assessed.obligations
    );
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "closure" && entry.verdict == RecordingVerdict::Violated
    }));
}

/// FR-990: `incomplete` is a valid closure state, but it cannot settle a
/// missing closure obligation or produce complete global success.
#[trace("TC-2801", "FR-990-AC-3")]
#[test]
fn an_incomplete_closure_is_admitted_and_keeps_closure_pending() {
    let package = protocol();
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Flow",
        "observations":[], "local_order":{}, "correlations":[],
        "closure":{"state":"incomplete", "premises":[]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes())
        .expect("incomplete is an admitted closure state");
    assert_eq!(assessed.closure.state, "incomplete");
    assert_eq!(
        assessed.global.label, "inconclusive",
        "{:#?}",
        assessed.obligations
    );
    assert!(!assessed.global.complete);
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "closure" && entry.verdict == RecordingVerdict::Pending
    }));
}

/// FR-990 closure identity includes each activated member of a fan-out
/// branch, grounded by that member's FR-737 object reference.
#[trace("TC-2831", "FR-990-AC-6")]
#[test]
fn fanout_branch_closure_premises_include_each_role_instance() {
    let run_body = "sequence Main { parallel Both { branch left sequence Left { event LeftWork by r as (left_worked: Boolean) { true }; } branch right sequence Right { event RightWork by r as (right_worked: Boolean) { true }; } } join all [left, right]; }";
    let source = crate::protocol_roles::unit(
        crate::protocol_roles::ORIGIN,
        &crate::protocol_roles::role(2, "workflow"),
        "",
        run_body,
    );
    let package = crate::fill::package(&source);
    let member = |identity: &str| json!({"authority":"test", "identity":identity});
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"R",
        "observations":[
            {"kind":"event", "node":"R::Main::Both::left::Left::LeftWork", "role":"r", "role_instance":member("cell-a"), "keys":{}, "record":true},
            {"kind":"event", "node":"R::Main::Both::left::Left::LeftWork", "role":"r", "role_instance":member("cell-b"), "keys":{}, "record":true},
            {"kind":"event", "node":"R::Main::Both::right::Right::RightWork", "role":"r", "role_instance":member("cell-a"), "keys":{}, "record":true},
            {"kind":"event", "node":"R::Main::Both::right::Right::RightWork", "role":"r", "role_instance":member("cell-b"), "keys":{}, "record":true}
        ],
        "local_order":{"r":[0,1,2,3]}, "correlations":[],
        "closure":{"state":"closed", "premises":[
            {"kind":"execution", "node":"R::Done"},
            {"kind":"branch", "node":"R::Main::Both::left", "role_instance":member("cell-a")},
            {"kind":"branch", "node":"R::Main::Both::left", "role_instance":member("cell-b")},
            {"kind":"branch", "node":"R::Main::Both::right", "role_instance":member("cell-a")},
            {"kind":"branch", "node":"R::Main::Both::right", "role_instance":member("cell-b")}
        ]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes())
        .expect("complete per-member fan-out premise set");
    let role_instance =
        |identity: &str| fsl_analyze::protocol_analysis::recording::RecordingRoleInstance {
            authority: "test".to_owned(),
            identity: identity.to_owned(),
        };
    let branch =
        |node: &str, identity: &str| fsl_analyze::protocol_analysis::recording::RecordingPremise {
            kind: fsl_analyze::protocol_analysis::recording::RecordingPremiseKind::Branch,
            instance: None,
            node: node.to_owned(),
            role_instance: Some(role_instance(identity)),
        };
    assert_eq!(
        assessed.closure.premises,
        vec![
            fsl_analyze::protocol_analysis::recording::RecordingPremise {
                kind: fsl_analyze::protocol_analysis::recording::RecordingPremiseKind::Execution,
                instance: None,
                node: "R::Done".to_owned(),
                role_instance: None,
            },
            branch("R::Main::Both::left", "cell-a"),
            branch("R::Main::Both::left", "cell-b"),
            branch("R::Main::Both::right", "cell-a"),
            branch("R::Main::Both::right", "cell-b"),
        ]
    );
    assert_eq!(
        assessed.global.label, "inconclusive",
        "{:#?}",
        assessed.obligations
    );
    assert!(!assessed.global.complete);
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "closure" && entry.verdict == RecordingVerdict::Pending
    }));
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "join"
            && entry.obligation.node.as_deref() == Some("R::Main::Both")
            && entry.verdict == RecordingVerdict::Satisfied
    }));

    let mut omitted = run.clone();
    omitted["closure"]["premises"]
        .as_array_mut()
        .expect("closure premises")
        .remove(2);
    let refusal = assess_recording(&package, omitted.to_string().as_bytes())
        .expect_err("an omitted observed fan-out member is refused");
    assert!(
        format!("{refusal}").contains("closure premise set"),
        "{refusal:?}"
    );

    let mut crossed = run;
    crossed["closure"]["premises"][1]["role_instance"] = member("cell-z");
    let refusal = assess_recording(&package, crossed.to_string().as_bytes())
        .expect_err("a cross-wired fan-out member is refused");
    assert!(
        format!("{refusal}").contains("closure premise set"),
        "{refusal:?}"
    );
}

/// Fan-out branch closure is member-scoped for every checked join policy.
#[trace("TC-2831", "FR-990-AC-6")]
#[test]
fn fanout_join_policies_do_not_reuse_a_member_closure_for_another_member() {
    let policies = [
        ("join all [left, right];", false),
        ("join any [left, right] outstanding continue;", false),
        ("join quorum(1) [left, right] outstanding continue;", false),
        (
            "join predicate(both) [left, right] outstanding continue;",
            true,
        ),
    ];
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"R",
        "observations":[
            {"kind":"event", "node":"R::Main::Both::left::Left::LeftWork", "role":"r", "role_instance":{"authority":"test","identity":"cell-a"}, "keys":{}, "record":true},
            {"kind":"event", "node":"R::Main::Both::left::Left::LeftWork", "role":"r", "role_instance":{"authority":"test","identity":"cell-b"}, "keys":{}, "record":true},
            {"kind":"event", "node":"R::Main::Both::right::Right::RightWork", "role":"r", "role_instance":{"authority":"test","identity":"cell-a"}, "keys":{}, "record":true},
            {"kind":"event", "node":"R::Main::Both::right::Right::RightWork", "role":"r", "role_instance":{"authority":"test","identity":"cell-b"}, "keys":{}, "record":true}
        ],
        "local_order":{"r":[0,1,2,3]}, "correlations":[],
        "closure":{"state":"open", "premises":[
            {"kind":"branch", "node":"R::Main::Both::left", "role_instance":{"authority":"test","identity":"cell-a"}},
            {"kind":"branch", "node":"R::Main::Both::right", "role_instance":{"authority":"test","identity":"cell-a"}}
        ]}
    });
    for (policy, needs_predicate) in policies {
        let run_body = format!(
            "sequence Main {{ parallel Both {{ branch left sequence Left {{ event LeftWork by r as (left_worked: Boolean) {{ true }}; }} branch right sequence Right {{ event RightWork by r as (right_worked: Boolean) {{ true }}; }} }} {policy} }}"
        );
        let mut source = crate::protocol_roles::unit(
            crate::protocol_roles::ORIGIN,
            &crate::protocol_roles::role(2, "workflow"),
            "",
            &run_body,
        );
        if needs_predicate {
            source = format!(
                "function both using v(left: Boolean, right: Boolean): Boolean pure {{ left and right }}\n{source}"
            );
        }
        let package = crate::fill::package(&source);
        let assessed = assess_recording(&package, run.to_string().as_bytes())
            .expect("readable partial fan-out closure");
        assert!(
            assessed.obligations.iter().any(|entry| {
                entry.obligation.kind == "join"
                    && entry.obligation.node.as_deref() == Some("R::Main::Both")
                    && entry.verdict == RecordingVerdict::Pending
            }),
            "policy {policy}: {:#?}",
            assessed.obligations
        );
    }
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

/// Trace: FR-895 enabledness.
#[trace("FR-895-AC-1")]
#[test]
fn a_node_repeated_outside_a_repeat_is_unexpected() {
    let package = protocol();
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Flow",
        "observations":[
            {"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}},
            {"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer", "instance":"w-1", "keys":{"message":"m-2", "delivery":"d-2"}}
        ],
        "local_order":{"Buyer":[0,1]}, "correlations":[],
        "closure":{"state":"open", "premises":[]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "enabledness"
            && entry.obligation.node.as_deref() == Some("Flow::Main::Placed")
            && entry.verdict == RecordingVerdict::Violated
            && entry.cause
                == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Unexpected)
    }));
}

/// Trace: FR-895 choice exclusivity.
#[trace("FR-895-AC-1")]
#[test]
fn two_cases_of_one_choice_are_not_a_single_decision() {
    let package = protocol();
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Flow",
        "observations":[
            {"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}},
            {"kind":"receive", "node":"Flow::Main::Got", "role":"Seller", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}},
            {"kind":"case", "node":"Flow::Main::Decide::yes", "role":"Seller", "instance":"w-1", "keys":{}},
            {"kind":"case", "node":"Flow::Main::Decide::no", "role":"Seller", "instance":"w-1", "keys":{}}
        ],
        "local_order":{"Buyer":[0], "Seller":[1,2,3]},
        "correlations":[{"send":0,"receive":1}],
        "closure":{"state":"open", "premises":[]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert_eq!(
        assessed.global.label, "refuted",
        "{:#?}",
        assessed.obligations
    );
    assert!(assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "enabledness"
            && entry.obligation.node.as_deref() == Some("Flow::Main::Decide::no")
            && entry.verdict == RecordingVerdict::Violated
    }));
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

/// Trace: FR-895 closed-path reachability.
#[trace("FR-895-AC-1")]
#[test]
fn a_closed_path_does_not_require_actions_in_an_unselected_case() {
    let package = told_protocol();
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Flow",
        "observations":[
            {"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":false}},
            {"kind":"receive", "node":"Flow::Main::Got", "role":"Seller", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":false}},
            {"kind":"case", "node":"Flow::Main::Decide::no", "role":"Seller", "instance":"w-1", "keys":{}},
            {"kind":"finish", "node":"Flow::Done", "role":"Seller", "instance":"w-1", "keys":{}, "record":{"zz":true}}
        ],
        "local_order":{"Buyer":[0], "Seller":[1,2,3]},
        "correlations":[{"send":0,"receive":1}],
        "closure":{"state":"closed", "premises":[
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Decide::no"},
            {"kind":"branch", "node":"Flow::Main::Decide::yes"}
        ]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert_ne!(
        assessed.global.label, "refuted",
        "{:#?}",
        assessed.obligations
    );
    assert!(!assessed.obligations.iter().any(|entry| {
        entry.obligation.kind == "causal-edge"
            && entry
                .obligation
                .node
                .as_deref()
                .is_some_and(|node| node.contains("Told"))
            && entry.verdict == RecordingVerdict::Violated
    }));
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
        "closure": {"state": "closed", "premises": [
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Decide::no"},
            {"kind":"branch", "node":"Flow::Main::Decide::yes"}
        ]}
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

/// Trace: FR-895-AC-1.
#[trace("FR-895-AC-1")]
#[test]
fn a_closed_empty_recording_reports_static_causal_edges_as_missing() {
    let package = protocol();
    let run = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": [],
        "local_order": {},
        "correlations": [],
        "closure": {"state": "closed", "premises": [
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Decide::no"},
            {"kind":"branch", "node":"Flow::Main::Decide::yes"}
        ]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes())
        .expect("empty recording remains assessable");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "causal-edge"
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause
                    == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Missing)
                && entry.witness.is_empty()
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895-AC-6.
#[trace("FR-895-AC-6")]
#[test]
fn compensation_windows_are_assessed_for_each_instance() {
    let package = paid_protocol_window();
    let mut first = compensation_attempt("comp-1", "successful", None);
    first["instance"] = json!("instance-1");
    first["clocks"] = json!({"c": 10});
    let mut second = compensation_attempt("comp-2", "successful", None);
    second["instance"] = json!("instance-2");
    second["clocks"] = json!({"c": 130});
    let run = paid_recording(json!([first, second]));
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    let windows = assessed
        .obligations
        .iter()
        .filter(|entry| entry.obligation.kind == "window")
        .collect::<Vec<_>>();
    assert!(
        windows.iter().any(|entry| {
            entry.obligation.instance.as_deref() == Some("instance-1")
                && entry.verdict == RecordingVerdict::Satisfied
        }),
        "{windows:#?}"
    );
    assert!(
        windows.iter().any(|entry| {
            entry.obligation.instance.as_deref() == Some("instance-2")
                && entry.verdict == RecordingVerdict::Violated
        }),
        "{windows:#?}"
    );
}

/// Trace: FR-895 compensation obligations.
#[trace("FR-895-AC-1")]
#[test]
fn a_registered_compensation_with_no_attempts_is_still_an_obligation() {
    let package = paid_protocol();
    let run = paid_recording(json!([paid_effect("effect-1", "operation-1")]));
    let mut closed = run;
    closed["closure"]["state"] = json!("closed");
    closed["closure"]["premises"] = json!([
        {"kind":"execution", "node":"Pay::Done"},
        {"kind":"workflow", "node":"Pay::Main::Paid"}
    ]);
    let assessed =
        assess_recording(&package, closed.to_string().as_bytes()).expect("readable paid recording");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "compensation"
                && entry.obligation.instance.as_deref() == Some("instance-1")
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause
                    == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Missing)
        }),
        "{:#?}",
        assessed.obligations
    );
    assert_ne!(
        assessed.global.label, "proved",
        "a closed recording cannot prove compensation completeness without an exact scope"
    );
}

/// Trace: FR-895 closed-path reachability.
#[trace("FR-895-AC-1")]
#[test]
fn an_unregistered_compensation_is_not_required_when_its_effect_was_not_observed() {
    let package = paid_protocol();
    let run = paid_recording(json!([]));
    let mut closed = run;
    closed["closure"]["state"] = json!("closed");
    closed["closure"]["premises"] = json!([{"kind":"execution", "node":"Pay::Done"}]);
    let assessed =
        assess_recording(&package, closed.to_string().as_bytes()).expect("readable paid recording");
    assert!(
        !assessed
            .obligations
            .iter()
            .any(|entry| entry.obligation.kind == "compensation")
    );
}

/// Trace: FR-895-AC-2.
#[trace("FR-895-AC-2")]
#[test]
fn compensation_order_uses_local_order_when_observation_array_is_reversed() {
    let package = paid_protocol();
    let run = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Pay",
        "observations": [
            recovery("successful"),
            compensation_attempt("comp-1", "successful", None),
            paid_effect("effect-1", "attempt-1")
        ],
        "local_order": {"w": [2, 1, 0]},
        "correlations": [],
        "closure": {"state": "open", "premises": []}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "compensation"
                && entry.verdict == RecordingVerdict::Satisfied
                && entry.witness == [1, 0]
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895-AC-1.
#[trace("FR-895-AC-1")]
#[test]
fn channel_delivery_minimum_and_capacity_are_assessed() {
    let base = crate::analyze_cli_support::order_protocol(true);
    let package = crate::litmus::package_called("analysis-unit", &base);
    let send_only = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": [{"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer",
            "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}}],
        "local_order": {"Buyer":[0]}, "correlations": [],
        "closure": {"state":"closed", "premises":[
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Decide::no"},
            {"kind":"branch", "node":"Flow::Main::Decide::yes"}
        ]}
    });
    let assessed =
        assess_recording(&package, send_only.to_string().as_bytes()).expect("readable run");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "channel-premise"
                && entry.obligation.node.as_deref() == Some("Order")
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause
                    == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Missing)
        }),
        "{:#?}",
        assessed.obligations
    );

    let zero_capacity = base.replace("capacity 4", "capacity 0");
    let package = crate::litmus::package_called("analysis-unit", &zero_capacity);
    let open = send_only.clone().to_string().replace("closed", "open");
    let assessed = assess_recording(&package, open.as_bytes()).expect("readable run");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "channel-premise"
                && entry.obligation.node.as_deref() == Some("Order")
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause
                    == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Unexpected)
        }),
        "{:#?}",
        assessed.obligations
    );

    let loss_source = base
        .replace("capacity 4", "capacity 1")
        .replace("overflow reject", "overflow loss new");
    let package = crate::litmus::package_called("analysis-unit", &loss_source);
    let lossy = json!({
        "format":"filament.protocol-recording/1", "protocol":"Flow",
        "observations":[
            {"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}},
            {"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer", "instance":"w-1", "keys":{"message":"m-2", "delivery":"d-2"}, "record":{"ok":true}},
            {"kind":"receive", "node":"Flow::Main::Got", "role":"Seller", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}}
        ],
        "local_order":{"Buyer":[0,1], "Seller":[2]},
        "correlations":[{"send":0,"receive":2}],
        "closure":{"state":"closed", "premises":[
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Decide::no"},
            {"kind":"branch", "node":"Flow::Main::Decide::yes"}
        ]}
    });
    let assessed = assess_recording(&package, lossy.to_string().as_bytes()).expect("readable run");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "channel-premise"
                && entry.obligation.node.as_deref() == Some("Order")
                && entry.verdict == RecordingVerdict::Satisfied
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895 control obligations.
#[trace("FR-895-AC-1")]
#[test]
fn repeat_control_reports_a_run_over_its_checked_maximum() {
    let package = crate::fill::package(&crate::protocol_compensation::twice());
    let effect = |identity: &str, ordinal: u64| {
        json!({
            "kind":"effect", "node":"Pay::Main::Loop::Body::Paid", "role":"w",
            "instance":"w-1", "keys":{"business_effect":identity, "attempt":identity, "registration":"reg-1"},
            "occurrence":{"path":[], "ordinal":ordinal}
        })
    };
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Pay",
        "observations":[effect("effect-0",0), effect("effect-1",1), effect("effect-2",2)],
        "local_order":{"w":[0,1,2]}, "correlations":[],
        "closure":{"state":"closed", "premises":[
            {"kind":"execution", "node":"Pay::Done"},
            {"kind":"workflow", "node":"Pay::Main::Loop::Body::Paid"}
        ]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "control"
                && entry.obligation.node.as_deref() == Some("Pay::Main::Loop")
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause
                    == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Duplicated)
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895 repeat bounds.
#[trace("FR-895-AC-1")]
#[test]
fn repeated_nodes_within_the_checked_repeat_bound_remain_enabled() {
    let package = crate::fill::package(&crate::protocol_compensation::twice());
    let effect = |identity: &str, ordinal: u64| {
        json!({
            "kind":"effect", "node":"Pay::Main::Loop::Body::Paid", "role":"w",
            "instance":"w-1", "keys":{"business_effect":identity, "attempt":identity, "registration":"reg-1"},
            "occurrence":{"path":[], "ordinal":ordinal}
        })
    };
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Pay",
        "observations":[
            effect("effect-0", 0),
            effect("effect-1", 1)
        ],
        "local_order":{"w":[0,1]}, "correlations":[],
        "closure":{"state":"open", "premises":[]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert!(
        !assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "enabledness"
                && entry.obligation.node.as_deref() == Some("Pay::Main::Loop::Body::Paid")
                && entry.verdict == RecordingVerdict::Violated
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895 repeated choice ambiguity.
#[trace("FR-895-AC-1")]
#[test]
fn mixed_cases_across_repeat_iterations_remain_pending() {
    let source = crate::protocol_compensation::twice().replace(
        "sequence Body { effect Paid of Main::C as (pd: M::Cell) { true }; event Fail by w as (failed: Boolean) { true }; }",
        "sequence Body { effect Paid of Main::C as (pd: M::Cell) { true }; event Fail by w as (failed: Boolean) { true }; choice Decide by w visible (failed) { case no when { not failed } sequence No { } case yes when { failed } sequence Yes { } } }",
    );
    let package = crate::fill::package(&source);
    let run = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Pay",
        "observations": [
            {"kind":"case", "node":"Pay::Main::Loop::Body::Decide::no", "role":"w", "instance":"w-1", "keys":{}},
            {"kind":"case", "node":"Pay::Main::Loop::Body::Decide::yes", "role":"w", "instance":"w-1", "keys":{}}
        ],
        "local_order":{"w":[0,1]}, "correlations":[],
        "closure":{"state":"open", "premises":[]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert_ne!(
        assessed.global.label, "refuted",
        "{:#?}",
        assessed.obligations
    );
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "control"
                && entry.obligation.node.as_deref() == Some("Pay::Main::Loop::Body::Decide")
                && entry.obligation.instance.as_deref() == Some("w-1")
                && entry.verdict == RecordingVerdict::Pending
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// A replicated member's choice guard reads that same member's binder record.
#[trace("FR-895-AC-1")]
#[test]
fn fanout_choice_guard_uses_the_binder_for_its_role_instance() {
    let run_body = "sequence Main { event Ready by r as (ready: Boolean) { true }; choice Decide by r visible (ready) { case no when { not ready } sequence No { } case yes when { ready } sequence Yes { } } }";
    let source = crate::protocol_roles::unit(
        crate::protocol_roles::ORIGIN,
        &crate::protocol_roles::role(2, "workflow"),
        "",
        run_body,
    );
    let package = crate::fill::package(&source);
    let member = |identity: &str| json!({"authority":"test", "identity":identity});
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"R",
        "observations":[
            {"kind":"event", "node":"R::Main::Ready", "role":"r", "role_instance":member("cell-b"), "keys":{}, "record":false},
            {"kind":"event", "node":"R::Main::Ready", "role":"r", "role_instance":member("cell-a"), "keys":{}, "record":true},
            {"kind":"case", "node":"R::Main::Decide::yes", "role":"r", "role_instance":member("cell-b"), "keys":{}},
            {"kind":"case", "node":"R::Main::Decide::yes", "role":"r", "role_instance":member("cell-a"), "keys":{}}
        ],
        "local_order":{"r":[0,1,2,3]}, "correlations":[],
        "closure":{"state":"open", "premises":[]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes())
        .expect("readable replicated-role choice observations");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "control"
                && entry.obligation.node.as_deref() == Some("R::Main::Decide")
                && entry.verdict == RecordingVerdict::Violated
                && entry.witness == [2, 3]
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Repeated observations of the same case are assessed independently; a
/// later false guard must refute the choice even when its first guard held.
#[trace("FR-895-AC-1")]
#[test]
fn repeated_same_case_checks_each_guard_observation() {
    let mut source = crate::analyze_cli_support::order_protocol(true).replace(
        "run sequence Main {",
        "run sequence Main { repeat Loop by Seller visible () max 2 while { true } sequence Body {",
    );
    let finish = source.find("finish Done").expect("finish terminal");
    let body_close = source[..finish]
        .rfind(" }")
        .expect("the main sequence closes before finish");
    source.replace_range(
        body_close..body_close + 2,
        " } exhausted sequence Out { } }",
    );
    let package = crate::litmus::package_called("analysis-unit", &source);
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Flow",
        "observations":[
            {"kind":"send", "node":"Flow::Main::Loop::Body::Placed", "role":"Buyer", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}},
            {"kind":"receive", "node":"Flow::Main::Loop::Body::Got", "role":"Seller", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}},
            {"kind":"case", "node":"Flow::Main::Loop::Body::Decide::yes", "role":"Seller", "keys":{}},
            {"kind":"send", "node":"Flow::Main::Loop::Body::Placed", "role":"Buyer", "keys":{"message":"m-2", "delivery":"d-2"}, "record":{"ok":false}},
            {"kind":"receive", "node":"Flow::Main::Loop::Body::Got", "role":"Seller", "keys":{"message":"m-2", "delivery":"d-2"}, "record":{"ok":false}},
            {"kind":"case", "node":"Flow::Main::Loop::Body::Decide::yes", "role":"Seller", "keys":{}}
        ],
        "local_order":{"Buyer":[0,3], "Seller":[1,2,4,5]},
        "correlations":[{"send":0,"receive":1}, {"send":3,"receive":4}],
        "closure":{"state":"open", "premises":[]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes())
        .expect("readable repeated-choice recording");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "control"
                && entry.obligation.node.as_deref() == Some("Flow::Main::Loop::Body::Decide")
                && entry.verdict == RecordingVerdict::Violated
                && entry.witness.contains(&2)
                && entry.witness.contains(&5)
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895 control obligations.
#[trace("FR-895-AC-1")]
#[test]
fn parallel_join_obligation_reads_branch_closure_premises() {
    let source = crate::analyze_cli_support::order_protocol(false).replace(
        "run sequence Main {",
        "run sequence Main { parallel Both memory sc { branch left sequence Left { event L by Buyer as (l: Ack) { true }; } branch right sequence Right { event R by Seller as (r: Ack) { true }; } } join all [left, right];",
    );
    let package = crate::litmus::package_called("analysis-unit", &source);
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Flow",
        "observations":[
            {"kind":"event", "node":"Flow::Main::Both::left::Left::L", "role":"Buyer", "instance":"w-1", "keys":{}, "record":{"ok":true}},
            {"kind":"event", "node":"Flow::Main::Both::right::Right::R", "role":"Seller", "instance":"w-1", "keys":{}, "record":{"ok":true}}
        ],
        "local_order":{"Buyer":[0], "Seller":[1]}, "correlations":[],
        "closure":{"state":"closed", "premises":[
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Both::left"},
            {"kind":"branch", "node":"Flow::Main::Both::right"}
        ]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "join"
                && entry.obligation.node.as_deref() == Some("Flow::Main::Both")
                && entry.verdict == RecordingVerdict::Satisfied
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895 check control.
#[trace("FR-895-AC-1")]
#[test]
fn check_control_evaluates_its_checked_body_against_prior_recorded_state() {
    for (condition, expected) in [
        ("true", RecordingVerdict::Satisfied),
        ("false", RecordingVerdict::Violated),
    ] {
        let source = crate::protocol_compensation::pay("true").replace(
            "event Fail by w as (failed: Boolean) { true };",
            &format!("event Fail by w as (failed: Boolean) {{ true }}; check Gate using v {{ {condition} }};"),
        );
        let package = crate::fill::package(&source);
        let run = json!({
            "format":"filament.protocol-recording/1", "protocol":"Pay",
            "observations":[{"kind":"event", "node":"Pay::Main::Fail", "role":"w", "instance":"w-1", "keys":{}, "record":true}],
            "local_order":{"w":[0]}, "correlations":[],
            "closure":{"state":"open", "premises":[]}
        });
        let assessed =
            assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
        assert!(
            assessed.obligations.iter().any(|entry| {
                entry.obligation.kind == "control"
                    && entry.obligation.node.as_deref() == Some("Pay::Main::Gate")
                    && entry.verdict == expected
            }),
            "{condition}: {:#?}",
            assessed.obligations
        );
    }
}

/// Trace: FR-895 join control.
#[trace("FR-895-AC-1")]
#[test]
fn predicate_join_evaluates_the_checked_function_over_branch_premises() {
    let source = crate::analyze_cli_support::order_protocol(false)
        .replace(
            "run sequence Main {",
            "run sequence Main { parallel Both memory sc { branch left sequence Left { event L by Buyer as (l: Ack) { true }; } branch right sequence Right { event R by Seller as (r: Ack) { true }; } } join predicate(both) [left, right] outstanding cancel;",
        );
    let source = format!(
        "function both using v(left: Boolean, right: Boolean): Boolean pure {{ left and right }}\n{source}"
    );
    let package = crate::litmus::package_called("analysis-unit", &source);
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Flow",
        "observations":[
            {"kind":"event", "node":"Flow::Main::Both::left::Left::L", "role":"Buyer", "instance":"w-1", "keys":{}, "record":{"ok":true}},
            {"kind":"event", "node":"Flow::Main::Both::right::Right::R", "role":"Seller", "instance":"w-1", "keys":{}, "record":{"ok":true}}
        ],
        "local_order":{"Buyer":[0], "Seller":[1]}, "correlations":[],
        "closure":{"state":"closed", "premises":[
            {"kind":"execution", "node":"Flow::Done"},
            {"kind":"branch", "node":"Flow::Main::Both::left"},
            {"kind":"branch", "node":"Flow::Main::Both::right"}
        ]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "join"
                && entry.obligation.node.as_deref() == Some("Flow::Main::Both")
                && entry.verdict == RecordingVerdict::Satisfied
        }),
        "{:#?}",
        assessed.obligations
    );

    let mut incomplete = run;
    incomplete["closure"]["premises"] = json!([
        {"kind":"execution", "node":"Flow::Done"},
        {"kind":"branch", "node":"Flow::Main::Both::left"}
    ]);
    let refusal = assess_recording(&package, incomplete.to_string().as_bytes())
        .expect_err("omitted closure premise is refused");
    assert!(
        format!("{refusal}").contains("closure premise set"),
        "{refusal:?}"
    );
}

/// Trace: FR-895 channel premises.
#[trace("FR-895-AC-1")]
#[test]
fn fifo_channel_premise_detects_reversed_delivery_order() {
    let source = crate::analyze_cli_support::order_protocol(true)
        .replace("ordering unordered", "ordering fifo by (m: Ack) { m.ok }");
    let package = crate::litmus::package_called("analysis-unit", &source);
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Flow",
        "observations":[
            {"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}},
            {"kind":"send", "node":"Flow::Main::Placed", "role":"Buyer", "instance":"w-1", "keys":{"message":"m-2", "delivery":"d-2"}, "record":{"ok":true}},
            {"kind":"receive", "node":"Flow::Main::Got", "role":"Seller", "instance":"w-1", "keys":{"message":"m-2", "delivery":"d-2"}, "record":{"ok":true}},
            {"kind":"receive", "node":"Flow::Main::Got", "role":"Seller", "instance":"w-1", "keys":{"message":"m-1", "delivery":"d-1"}, "record":{"ok":true}}
        ],
        "local_order":{"Buyer":[0,1], "Seller":[2,3]},
        "correlations":[{"send":0,"receive":3}, {"send":1,"receive":2}],
        "closure":{"state":"open", "premises":[]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert!(
        assessed.obligations.iter().any(|entry| {
            entry.obligation.kind == "channel-premise"
                && entry.obligation.node.as_deref() == Some("Order")
                && entry.verdict == RecordingVerdict::Violated
                && entry.cause
                    == Some(fsl_analyze::protocol_analysis::recording::RecordingCause::Misordered)
        }),
        "{:#?}",
        assessed.obligations
    );
}

/// Trace: FR-895 closure.
#[trace("FR-895-AC-3")]
#[test]
fn closure_workflow_premise_is_retained_as_recorded_fact() {
    let package = paid_protocol();
    let run = json!({
        "format":"filament.protocol-recording/1", "protocol":"Pay",
        "observations":[], "local_order":{}, "correlations":[],
        "closure":{"state":"open", "premises":[{"kind":"workflow", "node":"Pay::Main::Spend"}]}
    });
    let assessed = assess_recording(&package, run.to_string().as_bytes()).expect("readable run");
    assert_eq!(assessed.closure.premises.len(), 1);
    assert_eq!(
        assessed.closure.premises[0].kind,
        fsl_analyze::protocol_analysis::recording::RecordingPremiseKind::Workflow
    );
    assert_eq!(assessed.closure.premises[0].node, "Pay::Main::Spend");
}
