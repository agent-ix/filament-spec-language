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
        "prefix": steps})
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

/// FR-941-AC-1: a bare protocol claim is proved over a supplied trace when
/// every step replays, naming the trace and the steps replayed; a prefix
/// that stops before `Seller`'s choice is proved too.
#[trace("TC-2621", "FR-941-AC-1")]
#[test]
fn a_bare_claim_is_proved_when_every_step_replays() {
    let whole = order_steps(&[StepKind::Send, StepKind::Receive, StepKind::Finish]);
    for (identity, steps) in [("whole", &whole[..]), ("prefix", &whole[..2])] {
        let run = replay_item(identity, &trace_document(identity, steps));
        let item = run.item(0);
        assert_eq!(label(&item), "proved", "{identity}: {item}");
        assert_eq!(
            item["record"]["steps"],
            steps.len(),
            "{identity}: the replayed step count"
        );
        assert_eq!(
            item["record"]["trace"],
            json!({"authority": "test", "identity": identity}),
            "{identity}: {item}"
        );
    }
}

/// FR-941-AC-2: the trace document is read before replay. A `send` step
/// without its `send_record` declines `missing-member` at
/// `/prefix/0/send_record`; a `receive` step with one declines
/// `unknown-member` at that step; a trace whose step 0 is stale and whose
/// step 2 lacks a required member reports the step-2 shape defect.
#[trace("TC-2621", "FR-941-AC-2")]
#[test]
fn a_trace_is_read_before_it_is_replayed() {
    let steps = order_steps(&[StepKind::Send, StepKind::Receive, StepKind::Finish]);
    let declined = |identity: &str, steps: &[Value]| {
        let item = replay_item(identity, &trace_document(identity, steps)).item(0);
        assert_eq!(label(&item), "declined", "{identity}: {item}");
        assert_eq!(item["record"]["cause"]["kind"], "invalid-input", "{item}");
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{item}"
        );
        item
    };

    let mut bare = steps.clone();
    bare[0]
        .as_object_mut()
        .expect("a step")
        .remove("send_record");
    let item = declined("bare", &bare);
    let diagnostic = &item["record"]["diagnostic"];
    assert_eq!(diagnostic["cause"], "missing-member", "{item}");
    assert_eq!(
        diagnostic["detail"]["path"], "/prefix/0/send_record",
        "{item}"
    );
    assert_eq!(diagnostic["detail"]["position"], 0, "{item}");

    let mut stray = steps.clone();
    stray[1]["send_record"] = steps[0]["send_record"].clone();
    let item = declined("stray", &stray);
    let diagnostic = &item["record"]["diagnostic"];
    assert_eq!(diagnostic["cause"], "unknown-member", "{item}");
    assert_eq!(diagnostic["detail"]["position"], 1, "{item}");
    assert_eq!(
        diagnostic["detail"]["path"], "/prefix/1/send_record",
        "{item}"
    );

    let mut late = vec![steps[1].clone(), steps[0].clone(), steps[2].clone()];
    late[2]
        .as_object_mut()
        .expect("a step")
        .remove("post_state");
    let item = declined("late", &late);
    let diagnostic = &item["record"]["diagnostic"];
    assert_eq!(diagnostic["cause"], "missing-member", "{item}");
    assert_eq!(diagnostic["detail"]["position"], 2, "{item}");
    assert_eq!(
        diagnostic["detail"]["path"], "/prefix/2/post_state",
        "{item}"
    );
}

/// FR-941-AC-3: a well-formed trace beginning with `Got` declines
/// `invalid-value` at position 0.
#[trace("TC-2621", "FR-941-AC-3")]
#[test]
fn a_stale_first_step_is_declined_at_position_zero() {
    let steps = order_steps(&[StepKind::Send, StepKind::Receive]);
    let item = replay_item("got-first", &trace_document("got-first", &steps[1..])).item(0);
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "invalid-input", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "invalid-value",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["position"], 0,
        "{item}"
    );
    assert!(
        item["record"]["diagnostic"]["detail"].get("path").is_none(),
        "{item}"
    );
}

/// FSpec FR-785 over a supplied trace: an analyze request's subject carries
/// no capacity binding, so a protocol with a symbolic capacity is an
/// unbounded root of the replay. The item takes `analyze`'s requires-bound
/// row (`unbounded-extent`), never the broken-invariant fault it settled
/// before.
#[trace("FSpec-TC-1911")]
#[test]
fn a_supplied_trace_over_a_symbolic_capacity_requires_a_bound() {
    let ws = Workspace::new("protocol-trace-symbolic");
    let path = litmus_request(
        &ws,
        &order_protocol(true).replace("capacity 4", "capacity symbolic(Bound)"),
        json!({"items": [{"claim": "Flow", "kind": "finite-replay",
            "trace": {"authority": "test", "identity": "symbolic"}}]}),
    );
    ws.write(
        "trace.json",
        trace_document("symbolic", &[]).to_string().as_bytes(),
    );
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("json");
    document["request"]["documents"]
        .as_array_mut()
        .expect("documents")
        .push(json!({"file": "trace.json"}));
    ws.write("analyze.json", document.to_string().as_bytes());
    let item = analyze(&path).item(0);
    let diagnostic = &item["record"]["diagnostic"];
    assert_eq!(
        (diagnostic["code"].as_str(), diagnostic["cause"].as_str()),
        (Some("unsupported_projection"), Some("unbounded-extent")),
        "{item}"
    );
}

/// [`replay_item`] over the protocol unit body `body` in place of the order
/// fixture.
fn replay_item_over(identity: &str, body: &str, trace: &Value) -> crate::analyze_cli_support::Run {
    let ws = Workspace::new(&format!("protocol-trace-over-{identity}"));
    let path = litmus_request(
        &ws,
        body,
        json!({"items": [{"claim": "Flow", "kind": "finite-replay",
            "trace": {"authority": "test", "identity": identity}}]}),
    );
    // The bounded profile `tb` a clause deciding `eventually` selects (FR-327).
    let unit = std::fs::read_to_string(path.with_file_name("program.fsl")).expect("unit");
    ws.write(
        "program.fsl",
        unit.replacen(
            "model M =",
            "profile tb = \"filament.temporal.event-position.false-extension/v1\";\nmodel M =",
            1,
        )
        .as_bytes(),
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

/// The order fixture requiring the temporal clauses `clauses`, each a
/// `(name, formula)` over `k: M::Core` under the infinite-trace profile,
/// in `requires` order.
fn order_requiring(clauses: &[(&str, &str)]) -> String {
    // A closed trace decides `eventually` (a bounded profile, FR-327); the
    // infinite-trace profile reads the behaviour as a prefix (FR-328).
    let mut declarations = String::new();
    let mut requirements = String::new();
    for (name, formula) in clauses {
        declarations.push_str(&format!(
            "temporal {name} using {} over (k: M::Core) clock \"model-steps\" on origin {{ {formula} }}\n",
            if formula.starts_with("eventually") { "tb" } else { "t0" }
        ));
        requirements.push_str(&format!("requires temporal {name};\n"));
    }
    let protocol = order_protocol(true).replacen(
        "run sequence Main {",
        &format!("{requirements}run sequence Main {{"),
        1,
    );
    format!("{declarations}{protocol}")
}

/// FR-941-AC-4: a protocol clause that requires temporal clauses is decided
/// over the replayed behaviour by the one temporal evaluator, reporting the
/// deciding clause's FSpec FR-352 disposition: `holds` proves, `violated`
/// and `undefined` refute with a replayed counterexample, `pending` is
/// inconclusive with exit 0, and the first refuting requirement decides.
#[trace("TC-2965", "FR-941-AC-4")]
#[test]
fn a_claim_with_a_formula_is_decided_over_the_replayed_behaviour() {
    let steps = order_steps(&[StepKind::Send, StepKind::Receive, StepKind::Finish]);
    let run_with = |identity: &str, clauses: &[(&str, &str)]| {
        replay_item_over(
            identity,
            &order_requiring(clauses),
            &trace_document(identity, &steps),
        )
    };

    let holds = run_with("formula-holds", &[("R", "eventually[0,3] holds(k.r1 = k.r1)")]);
    let item = holds.item(0);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["disposition"]["temporal"]["kind"], "holds", "{item}");
    assert_eq!(item["record"]["steps"], 3, "{item}");

    let violated = run_with("formula-violated", &[("R", "always holds(k.r1 != k.r1)")]);
    let item = violated.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["disposition"]["temporal"]["kind"], "violated", "{item}");
    assert_eq!(item["disposition"]["temporal"]["position"], 0, "{item}");
    assert_eq!(item["counterexample"]["kind"], "formula", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");

    let pending = run_with("formula-pending", &[("R", "always holds(k.r1 = k.r1)")]);
    let item = pending.item(0);
    assert_eq!(label(&item), "inconclusive", "{item}");
    assert_eq!(item["disposition"]["temporal"]["kind"], "pending", "{item}");
    assert_eq!(item["disposition"]["category"], "pending", "{item}");
    assert_eq!(item["disposition"]["basis"], "unsettled", "{item}");
    assert_eq!(pending.exit, 0);

    let first_refuting = run_with(
        "formula-order",
        &[
            ("P", "always holds(k.r1 = k.r1)"),
            ("R", "always holds(k.r1 != k.r1)"),
        ],
    );
    let item = first_refuting.item(0);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["disposition"]["temporal"]["kind"], "violated", "{item}");
}

/// FR-941-AC-5: a protocol expression undefined at a replayed state refutes
/// the item there, cause `undefined-evaluation` with locus `protocol`, with
/// a replayed counterexample, before a later stale step; a stale first step
/// still declines at position 0.
#[trace("TC-2965", "FR-941-AC-5")]
#[test]
#[ignore = "QSL-2272: spec gap - fsl analyze checks packages in Linked mode, which refuses every protocol expression that can be undefined (unproved-nonzero, expression-form), so FR-941-AC-5 has no authorable fixture through the CLI"]
fn an_undefined_protocol_expression_refutes_at_its_state() {
    let steps = order_steps(&[StepKind::Send, StepKind::Receive, StepKind::Finish]);
    let undefined_got = order_protocol(true).replacen(
        "receive Got via Order of Placed as (got: Ack) { true }",
        "receive Got via Order of Placed as (got: Ack) { 6 mod (2 - k.r1) >= 0 }",
        1,
    );

    let refuted_at_one = |item: &Value| {
        assert_eq!(label(item), "refuted", "{item}");
        let cause = &item["record"]["cause"];
        assert_eq!(cause["kind"], "undefined-evaluation", "{item}");
        assert_eq!(cause["where"]["position"], 1, "{item}");
        assert_eq!(cause["where"]["locus"], "protocol", "{item}");
        assert_eq!(cause["cause"], "division-by-zero", "{item}");
    };

    let run = replay_item_over(
        "undefined-step",
        &undefined_got,
        &trace_document("undefined-step", &steps),
    );
    let item = run.item(0);
    refuted_at_one(&item);
    assert_eq!(
        item["counterexample"]["prefix"].as_array().map(Vec::len),
        Some(1),
        "{item}"
    );
    assert_eq!(item["replay"]["reproduced"], true, "{item}");

    let mut moved = steps.clone();
    moved[2]["post_state"]["key"] = json!("AAAA");
    let run = replay_item_over(
        "undefined-before-stale",
        &undefined_got,
        &trace_document("undefined-before-stale", &moved),
    );
    refuted_at_one(&run.item(0));

    let run = replay_item_over(
        "undefined-stale-first",
        &undefined_got,
        &trace_document("undefined-stale-first", &steps[1..]),
    );
    let item = run.item(0);
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(item["record"]["diagnostic"]["cause"], "invalid-value", "{item}");
    assert_eq!(item["record"]["diagnostic"]["detail"]["position"], 0, "{item}");
}
