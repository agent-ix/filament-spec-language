// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-636 (TC-1596): a supplied finite protocol trace replays through
//! `fsl analyze`, and channel capacity under a parallel join is analyzed.

use base64::Engine as _;
use fsl_eval::model_subject::RoleBinding;
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::simulation::protocol::{
    send_record_json, transition_json, ProtocolSystem, Sc, StepKind,
};
use fsl_eval::simulation::{SubjectSystem, TransitionSystem};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, litmus_request, order_protocol, Workspace};

/// The steps of the order protocol's run that follow `kinds` from the
/// initial state, spelled as FSpec FR-433 spells a counterexample's steps:
/// each step's transition identity, the key of its post-state and, for a
/// `send`, its send record.
fn order_steps(kinds: &[StepKind]) -> Vec<Value> {
    let documents = crate::litmus::Documents::new();
    let package = crate::litmus::package_called("analyze-unit", &order_protocol(true));
    let mut subject = documents.subject(&package, "Flow", &[]);
    subject.roles = ["Buyer", "Seller"]
        .map(|role| RoleBinding {
            role: role.to_owned(),
            model: crate::litmus::PACKAGE.to_owned(),
        })
        .to_vec();
    let model = ModelSystem::new(subject.model.simulation.clone()).expect("the subject admits");
    let system = ProtocolSystem::new(model, &subject, Sc).expect("the protocol system builds");
    let mut state = system.initial().remove(0);
    let mut steps = Vec::new();
    for kind in kinds {
        let (transition, post) = system
            .successors(&state)
            .expect("the state expands")
            .successors
            .into_iter()
            .find(|(transition, _)| transition.kind() == *kind)
            .expect("the step is enabled");
        let key = system.canonical_state_key(&post).expect("the key encodes");
        let mut step = json!({
            "transition": transition_json(&transition).expect("the identity is spelled"),
            "post_state": {"key": base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(key.as_bytes())},
        });
        if let Some(record) = transition.send_record() {
            step["send_record"] = send_record_json(record).expect("the record is spelled");
        }
        steps.push(step);
        state = post;
    }
    steps
}

fn trace_document(identity: &str, steps: &[Value]) -> Value {
    json!({"format": "filament.protocol-trace/v1",
        "identity": {"authority": "test", "identity": identity},
        "steps": steps})
}

/// The analyze outcome of one `finite-replay` item over the order protocol
/// and the trace document `trace`.
fn replay_item(identity: &str, trace: &Value) -> crate::analyze_cli_support::Run {
    let ws = Workspace::new(&format!("protocol-trace-{identity}"));
    let path = litmus_request(
        &ws,
        &order_protocol(true),
        json!({"items": [{"claim": "Flow", "kind": "finite-replay",
            "trace": {"authority": "test", "identity": identity}}]}),
    );
    ws.write("trace.json", trace.to_string().as_bytes());
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("json");
    document["request"]["documents"]
        .as_array_mut()
        .expect("documents")
        .push(json!({"file": "trace.json"}));
    ws.write("analyze.json", document.to_string().as_bytes());
    analyze(&path)
}

/// FR-636-AC-3: a supplied trace of `Placed`, `Got` and `Ship` settles
/// `proved`; a supplied trace beginning with `Got` settles `declined`
/// `invalid-input` naming step 0. The steps are spelled as FSpec FR-433
/// spells them (FR-636 "Decision").
#[trace("TC-1596", "FR-636-AC-3")]
#[test]
fn a_supplied_trace_replays_or_is_declined_at_its_step() {
    let steps = order_steps(&[StepKind::Send, StepKind::Receive, StepKind::Finish]);
    let good = replay_item("good", &trace_document("good", &steps));
    let item = good.item(0);
    assert_eq!(label(&item), "proved", "{item}");

    let stale = replay_item("stale", &trace_document("stale", &steps[1..2]));
    assert_eq!(stale.exit, 20);
    let item = stale.item(0);
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "invalid-input", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_runtime_input",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "invalid-value",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["position"], 0,
        "{item}"
    );
}

/// FR-636-AC-4: a parallel join over a capacity-1 channel overflows, and a
/// message budget stops the run incomplete.
#[trace("TC-1596", "FR-636-AC-4")]
#[test]
#[ignore = "QSL-2224: protocol analysis items are not reachable through fsl analyze yet"]
fn parallel_join_overflow_and_message_budget() {
    let body = "record Ack { ok: Boolean; }\n\
        protocol Burst using pp over (k: M::Core) on origin {\n\
        role Buyer on M::Cell;\nrole Seller on M::Cell;\n\
        channel Order from Buyer to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 1 overflow reject;\n\
        run parallel Both { branch left sequence L { send A via Order as (a: Ack) { true }; } \
        branch right sequence R { send B via Order as (b: Ack) { true }; } } join all [left, right];\n\
        finish Done as (zz: Boolean) { true };\n}\n";
    let ws = Workspace::new("protocol-overflow");
    let run = analyze(&litmus_request(
        &ws,
        body,
        json!({"items": [
            {"claim": "Burst", "kind": "global-conformance"},
            {"claim": "Burst", "kind": "global-conformance",
             "temporal": {"limits": {"max_messages": 1}}},
        ]}),
    ));
    assert_eq!(label(&run.item(0)), "refuted");
    assert_eq!(label(&run.item(1)), "incomplete");
    assert_eq!(run.item(1)["record"]["cause"]["limit"], "max_messages");
}

/// FR-636 "Supplied trace refusals": a step whose recorded post-state is not
/// the one the replay reaches declines `invalid-value` at that step. A `send`
/// step without its `send_record` declines `missing-member`, a non-`send` step
/// with one `unknown-member`, before any step replays (FSpec FR-433-AC-8), so
/// a stale step ahead of the defect is not what is reported. A document with
/// an extra member declines `invalid-input`, and a document naming no
/// provisioned trace refuses the request.
#[trace("TC-1596", "FR-636-AC-3")]
#[test]
fn a_stale_or_malformed_supplied_trace_is_declined_and_an_unprovisioned_one_refused() {
    let steps = order_steps(&[StepKind::Send, StepKind::Receive, StepKind::Finish]);

    let mut moved = steps.clone();
    moved[1]["post_state"]["key"] = json!("AAAA");
    let run = replay_item("moved", &trace_document("moved", &moved));
    let item = run.item(0);
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["position"], 1,
        "{item}"
    );

    let mut bare = steps.clone();
    bare[0]
        .as_object_mut()
        .expect("a step")
        .remove("send_record");
    let run = replay_item("bare", &trace_document("bare", &bare));
    let item = run.item(0);
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "invalid-input", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "missing-member",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["position"], 0,
        "{item}"
    );

    // A stale step 0 ahead of a bare `send` at step 1: the wire defect is
    // found before replay reaches step 0.
    let mut late = vec![steps[1].clone(), steps[0].clone()];
    late[1]
        .as_object_mut()
        .expect("a step")
        .remove("send_record");
    let run = replay_item("late", &trace_document("late", &late));
    let item = run.item(0);
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "missing-member",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["position"], 1,
        "{item}"
    );

    let mut stray = steps.clone();
    stray[2]["send_record"] = steps[0]["send_record"].clone();
    let run = replay_item("stray", &trace_document("stray", &stray));
    let item = run.item(0);
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "unknown-member",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["position"], 2,
        "{item}"
    );

    let mut extra = trace_document("extra", &steps);
    extra["unread"] = json!(true);
    let run = replay_item("extra", &extra);
    let item = run.item(0);
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "invalid-input", "{item}");
    assert!(
        item["record"]["diagnostic"]
            .get("detail")
            .is_none_or(|detail| detail.get("position").is_none()),
        "{item}"
    );

    let ws = Workspace::new("protocol-trace-unprovisioned");
    let path = litmus_request(
        &ws,
        &order_protocol(true),
        json!({"items": [{"claim": "Flow", "kind": "finite-replay",
            "trace": {"authority": "test", "identity": "nowhere"}}]}),
    );
    let run = analyze(&path);
    assert_eq!(run.exit, 20);
    assert_eq!(run.document()["diagnostics"][0]["code"], "invalid-request");
    let message = run.document()["diagnostics"][0]["message"].to_string();
    assert!(message.contains("nowhere"), "{message}");
}
