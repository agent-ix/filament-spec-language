// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-896, FR-990, FR-991 and FR-992 (TC-2532, TC-2800 to TC-2804): a
//! `global-conformance` claim over a protocol is decided by a real engine,
//! reports per-obligation and global results beside an independent
//! complete-global-conformance closure record, and its counterexample
//! replays in process. Driven through the in-process command adapter.

use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::{
    Workspace, analyze, label, litmus_request, order_protocol, request_over,
};

/// The order protocol with a second channel `Tell` from `Seller` to `Buyer`,
/// on which `Seller` sends `Told` only after `Ship`.
fn told_protocol() -> String {
    order_protocol(true)
        .replace(
            "run sequence Main {",
            "channel Tell from Seller to Buyer carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\nrun sequence Main {",
        )
        .replace(
            "sequence Ship { }",
            "sequence Ship { send Told via Tell as (told: Ack) { true }; }",
        )
}

/// One `global-conformance` item over `unit`, with `members` merged into the
/// item and `documents` written beside the request.
fn conformance(name: &str, unit: &str, members: Value, documents: &[(&str, Value)]) -> Value {
    let ws = Workspace::new(name);
    let mut item = json!({"claim": "Flow", "kind": "global-conformance"});
    for (key, value) in members.as_object().expect("item members") {
        item[key] = value.clone();
    }
    let files: Vec<Value> = documents
        .iter()
        .map(|(file, document)| {
            ws.write(file, document.to_string().as_bytes());
            json!({"file": file})
        })
        .collect();
    let mut extra = json!({"items": [item]});
    if !files.is_empty() {
        extra["documents"] = Value::Array(files);
    }
    analyze(&litmus_request(&ws, unit, extra)).item(0)
}

/// A closed-or-not recording of the order fixture: `Placed`, `Got`, `Ship`
/// and `finish`, with the closure record `closure`.
fn order_recording(closure: Value) -> Value {
    json!({
        "format": "filament.protocol-recording/1",
        "identity": {"authority": "test", "identity": "recording"},
        "protocol": "Flow",
        "observations": [
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
        ],
        "local_order": {"Buyer": [0], "Seller": [1, 2, 3]},
        "correlations": [{"send": 0, "receive": 1}],
        "closure": closure
    })
}

/// The required premises of instance `w-1` of the order fixture.
fn order_premises() -> Value {
    json!([
        {"kind": "execution", "node": "Flow::Done"},
        {"kind": "branch", "node": "Flow::Main::Decide::no"},
        {"kind": "branch", "node": "Flow::Main::Decide::yes"}
    ])
}

fn recording_member() -> Value {
    json!({"recording": {"authority": "test", "identity": "recording"}})
}

fn obligations(item: &Value) -> &Vec<Value> {
    item["obligations"]
        .as_array()
        .unwrap_or_else(|| panic!("one result per obligation: {item}"))
}

/// QSL-2209 AC-1; FR-896-AC-1, FR-990-AC-1: role implementations that run
/// the protocol settle `proved`, every obligation `satisfied`, closure
/// `closed` by the engine, and complete global success.
#[trace(
    "TC-2532",
    "TC-2800",
    "FR-896-AC-1",
    "FR-990-AC-1",
    "FSpec-FR-059-AC-1"
)]
#[test]
fn matching_implementations_conform_globally() {
    let item = conformance("gc-match", &order_protocol(true), json!({}), &[]);
    assert_eq!(label(&item), "proved", "{item}");
    for entry in obligations(&item) {
        assert_eq!(entry["verdict"], "satisfied", "{entry}");
        let kind = entry["obligation"]["kind"].as_str().expect("a kind");
        let basis = if matches!(kind, "causal-order" | "enabledness" | "channel-premise") {
            "by-construction"
        } else {
            "explored"
        };
        assert_eq!(entry["basis"], basis, "{entry}");
    }
    assert_eq!(item["closure"]["state"], "closed");
    assert_eq!(item["closure"]["producer"], "engine");
    let premises = item["closure"]["premises"].as_array().expect("premises");
    assert_eq!(
        premises.iter().filter(|p| p["kind"] == "execution").count(),
        1
    );
    assert_eq!(premises.iter().filter(|p| p["kind"] == "branch").count(), 2);
    assert_eq!(
        item["global"],
        json!({"label": "proved", "basis": "closed-scope", "complete": true})
    );
    let roles: Vec<&str> = item["projections"]
        .as_array()
        .expect("projections beside the verdict")
        .iter()
        .filter_map(|p| p["role"].as_str())
        .collect();
    assert_eq!(roles, ["Buyer", "Seller"]);
}

/// FR-125-AC-13 and FR-990: global conformance over a bounded subject with no
/// admitted key is inconclusive with `no-instance`; an admitted-key source
/// control still runs the engine to completion.
#[trace("TC-2800", "FR-125-AC-13", "FR-896-AC-1", "FR-990-AC-11")]
#[test]
fn global_conformance_classifies_empty_over_as_no_instance() {
    let ws = Workspace::new("gc-no-instance");
    let mut snapshots: Vec<Vec<u8>> = crate::litmus::Documents::new()
        .snapshots
        .into_values()
        .collect();
    let mut snapshot: Value =
        serde_json::from_slice(snapshots.first().expect("one initial snapshot"))
            .expect("snapshot JSON");
    let populations = snapshot["populations"]
        .as_array_mut()
        .expect("snapshot populations");
    populations[2]["objects"] = json!([]);
    snapshots[0] = serde_json::to_vec(&snapshot).expect("snapshot JSON");
    let request = request_over(
        &ws,
        &crate::litmus::unit_text(&order_protocol(true)),
        &crate::litmus::resulting_document(),
        &snapshots,
        json!({"items": [{"claim": "Flow", "kind": "global-conformance"}]}),
    );
    let item = analyze(&request).item(0);
    assert_eq!(label(&item), "inconclusive", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "no-instance", "{item}");
    for member in [
        "obligations",
        "closure",
        "global",
        "projections",
        "counterexample",
    ] {
        assert!(item[member].is_null(), "NoInstance omits {member}: {item}");
    }

    let admitted = conformance(
        "gc-no-instance-control",
        &order_protocol(true),
        json!({}),
        &[],
    );
    assert_eq!(label(&admitted), "proved", "{admitted}");

    let every_key = order_protocol(true)
        .replace("over (k: M::Core)", "over (k: M::Cell)")
        .replace(
            "finish Done as (zz: Boolean) { true };",
            "finish Done as (zz: Boolean) { k.id = 0 };",
        );
    let later_instance = conformance("gc-every-key", &every_key, json!({}), &[]);
    assert_eq!(
        label(&later_instance),
        "refuted",
        "the first canonical key x satisfies the finish predicate, but y does not: {later_instance}"
    );
    assert_eq!(later_instance["counterexample"]["kind"], "deadlock");
    assert_eq!(later_instance["replay"]["reproduced"], true);
}

/// QSL-2209 AC-2; FR-896-AC-3, FR-991-AC-4: over a recording, a send on
/// `Tell` where the protocol enables none refutes conformance with an
/// `unexpected` enabledness violation; without a `ProtocolSubject`, the item
/// publishes no replay claim for the supplied bytes.
#[trace(
    "TC-2532",
    "TC-2803",
    "FR-896-AC-3",
    "FR-991-AC-4",
    "FSpec-FR-059-AC-2"
)]
#[test]
fn an_out_of_protocol_send_refutes_conformance() {
    let mut recording = order_recording(json!({"state": "closed", "premises": order_premises()}));
    recording["observations"]
        .as_array_mut()
        .expect("observations")
        .insert(
            0,
            json!({"kind": "send", "node": "Flow::Main::Decide::yes::Ship::Told", "role": "Seller",
                   "instance": "w-1", "keys": {"message": "m-0", "delivery": "d-0"},
                   "record": {"ok": true}}),
        );
    recording["local_order"] = json!({"Buyer": [1], "Seller": [0, 2, 3, 4]});
    recording["correlations"] = json!([{"send": 1, "receive": 2}]);
    let item = conformance(
        "gc-unexpected",
        &told_protocol(),
        recording_member(),
        &[("recording.json", recording)],
    );
    assert_eq!(label(&item), "refuted", "{item}");
    assert!(
        obligations(&item).iter().any(|entry| {
            entry["obligation"]["kind"] == "enabledness"
                && entry["verdict"] == "violated"
                && entry["cause"] == "unexpected"
                && entry["witness"] == json!([0])
        }),
        "{item}"
    );
    assert_eq!(item["global"]["complete"], false);
    assert!(item["replay"].is_null(), "{item}");
}

/// QSL-2209 AC-3; FR-990-AC-2, FR-991-AC-2: the refutation without the
/// `receive` is written in the protocol-analysis record form: the closure
/// obligation violated with a witness into the counterexample, closure
/// `open`, and an item `replay` that reproduced.
#[trace("TC-2800", "TC-2802", "FR-990-AC-2", "FR-991-AC-2", "FR-896-AC-2")]
#[test]
fn conformance_writes_the_analysis_record() {
    let item = conformance("gc-record", &order_protocol(false), json!({}), &[]);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["global"]["label"], "refuted");
    assert_eq!(item["global"]["complete"], false);
    let closure = obligations(&item)
        .iter()
        .find(|entry| entry["obligation"]["kind"] == "closure")
        .expect("a closure obligation");
    assert_eq!(closure["verdict"], "violated");
    assert_eq!(closure["cause"], "counterexample");
    assert_eq!(closure["witness"], 0);
    assert_eq!(item["counterexample"]["kind"], "deadlock");
    assert_eq!(item["closure"]["state"], "open");
    assert_eq!(item["closure"]["producer"], "engine");
    assert_eq!(item["replay"]["reproduced"], true);
}

/// FR-990-AC-5: a run stopped by `max_states` leaves its undecided
/// obligations `pending` and the closure `incomplete`, never `proved`.
#[trace("TC-2800", "FR-990-AC-5", "FSpec-FR-059-AC-3")]
#[test]
fn a_limited_run_leaves_obligations_pending_and_closure_incomplete() {
    let item = conformance(
        "gc-limit",
        &order_protocol(true),
        json!({"temporal": {"limits": {"max_states": 1}}}),
        &[],
    );
    let global = &item["global"];
    assert_eq!(global["label"], "incomplete", "{item}");
    assert_eq!(global["cause"], "limit-reached");
    assert_eq!(global["basis"], "unavailable");
    assert!(
        item.to_string().contains("max_states"),
        "names the limit: {item}"
    );
    assert!(
        obligations(&item)
            .iter()
            .any(|entry| entry["verdict"] == "pending")
    );
    assert_eq!(item["closure"]["state"], "incomplete");
    assert_eq!(item["global"]["complete"], false);
}

/// QSL-2209 AC-4; FR-896-AC-4: a role whose model the package does not
/// select is refused naming the role, and nothing is settled.
#[trace("TC-2532", "FR-896-AC-4")]
#[test]
fn a_missing_role_implementation_refuses() {
    let ws = Workspace::new("gc-missing");
    let protocol = order_protocol(true).replace(
        "role Seller on M::Cell;",
        "role Seller on M::Cell;\nrole Courier on N::Core;",
    );
    let request_path = litmus_request(
        &ws,
        &protocol,
        json!({"items": [
            {"claim": "Flow", "kind": "global-conformance"}
        ]}),
    );
    let source = crate::litmus::unit_text(&protocol).replace(
        "model M = \"test/litmus\";",
        "model M = \"test/litmus\";\nmodel N = \"test/second\";",
    );
    ws.write("program.fsl", source.as_bytes());
    let second_model = String::from_utf8(crate::litmus::document())
        .expect("litmus model is UTF-8")
        .replace("test/litmus", "test/second");
    ws.write("second-model.json", second_model.as_bytes());

    let request_bytes = std::fs::read(&request_path).expect("analyze request is readable");
    let mut request: Value = serde_json::from_slice(&request_bytes).expect("request is JSON");
    request["request"]["models"]
        .as_array_mut()
        .expect("model selections")
        .push(json!({
            "format": "semantic-ir/2.0.0",
            "source": {"file": "second-model.json", "authority": "test",
                "identity": "second-model", "document": "Second", "formal_revision": 1}
        }));
    ws.write("analyze.json", request.to_string().as_bytes());
    let run = analyze(&request_path);
    let item = run.item(0);
    let diagnostic = &item["record"]["diagnostic"];
    assert_eq!(
        diagnostic["code"],
        "missing_declaration",
        "item={item}; document={}",
        run.document()
    );
    assert_eq!(diagnostic["cause"], "missing-name");
    assert!(diagnostic.to_string().contains("Courier"), "{item}");
    assert_eq!(item["record"]["result"], "declined", "{item}");
    assert!(item["obligations"].is_null(), "{item}");
    assert!(item["closure"].is_null(), "{item}");
    assert!(item["global"].is_null(), "{item}");
}

/// FR-990-AC-3: over a recording the closure record is the recording's, and
/// only `closed` gives complete global success; `open` and `contradicted`
/// withhold it without changing a decided verdict.
#[trace("TC-2801", "FR-990-AC-3", "FSpec-FR-059-AC-5", "FSpec-FR-247-AC-5")]
#[test]
fn only_a_closed_recording_closure_gives_complete_success() {
    let run = |state: &str| {
        conformance(
            &format!("gc-closure-{state}"),
            &order_protocol(true),
            recording_member(),
            &[(
                "recording.json",
                order_recording(json!({"state": state, "premises": order_premises()})),
            )],
        )
    };
    let closed = run("closed");
    assert_eq!(closed["closure"]["state"], "closed");
    assert_eq!(closed["closure"]["producer"], "recording");
    assert_eq!(closed["global"]["complete"], true);
    let open = run("open");
    assert_eq!(label(&open), "inconclusive");
    assert_eq!(open["global"]["cause"], "open-history");
    assert_eq!(open["global"]["complete"], false);
    let decided = |item: &Value| -> Vec<Value> {
        obligations(item)
            .iter()
            .filter(|entry| entry["verdict"] != "pending")
            .filter(|entry| entry["obligation"]["kind"] != "closure")
            .filter(|entry| entry["obligation"]["kind"] != "channel-premise")
            .cloned()
            .collect()
    };
    assert_eq!(decided(&open), decided(&closed));
    // A channel premise is never satisfied by the absence of a violation: it
    // is `pending` while the closure is open and `satisfied` once closed.
    let premise = |item: &Value| {
        obligations(item)
            .iter()
            .find(|entry| entry["obligation"]["kind"] == "channel-premise")
            .map(|entry| entry["verdict"].clone())
    };
    assert_eq!(premise(&open), Some(json!("pending")));
    assert_eq!(premise(&closed), Some(json!("satisfied")));
    let contradicted = run("contradicted");
    assert_eq!(label(&contradicted), "inconclusive");
    assert_eq!(contradicted["global"]["cause"], "open-history");
    assert_eq!(contradicted["closure"]["state"], "contradicted");
    assert_eq!(contradicted["global"]["complete"], false);
}

/// FR-990-AC-4: an omitted, duplicated or cross-wired premise refuses the
/// recording naming it, and an observed `finish` never upgrades `open`.
#[trace("TC-2801", "FR-990-AC-4", "FSpec-FR-059-AC-5", "FSpec-FR-247-AC-3")]
#[test]
fn a_malformed_closure_premise_set_refuses_the_recording() {
    let premises = order_premises();
    let all = premises.as_array().expect("premises").clone();
    let omitted: Vec<Value> = all
        .iter()
        .filter(|p| p["node"] != "Flow::Main::Decide::yes")
        .cloned()
        .collect();
    let mut duplicated = all.clone();
    duplicated.push(all[0].clone());
    let mut crossed = all.clone();
    crossed[0] = json!({"kind": "execution", "node": "Flow::Main::Decide::yes"});
    for (name, set) in [
        ("omitted", omitted),
        ("duplicated", duplicated),
        ("crossed", crossed),
    ] {
        let item = conformance(
            &format!("gc-premise-{name}"),
            &order_protocol(true),
            recording_member(),
            &[(
                "recording.json",
                order_recording(json!({"state": "closed", "premises": set})),
            )],
        );
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{name}: {item}"
        );
        assert_eq!(
            item["record"]["diagnostic"]["cause"], "invalid-value",
            "{name}: {item}"
        );
        assert!(item["obligations"].is_null(), "{name}: {item}");
    }
    let open = conformance(
        "gc-premise-open",
        &order_protocol(true),
        recording_member(),
        &[(
            "recording.json",
            order_recording(json!({"state": "open", "premises": premises})),
        )],
    );
    assert_eq!(open["closure"]["state"], "open", "{open}");
}

/// The text of a sibling crate's source file.
fn crate_source(path: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    std::fs::read_to_string(root.join(path)).unwrap_or_default()
}

/// FR-991-AC-1, AC-3: the in-process replay is published with its coded
/// refusal. The coder replaces this surface check with a direct call that
/// replays the FR-896-AC-2 counterexample, a tampered step (refused
/// `invalid_runtime_input`/`invalid-value` at its index) and an appended
/// enabled step (`ReplayParity`).
#[trace("TC-2802", "FR-991-AC-1", "FR-991-AC-3")]
#[test]
fn conformance_replay_is_published_with_coded_refusals() {
    let module = crate_source("fsl-analyze/src/protocol_analysis/mod.rs");
    assert!(module.contains("pub mod conformance"), "module not public");
    let source = crate_source("fsl-analyze/src/protocol_analysis/conformance.rs");
    for item in [
        "pub fn replay_counterexample(",
        "pub enum ConformanceCounterexample",
        "pub enum ConformanceReplay",
        "pub struct ConformanceReplayRefusal",
        "pub fn code(&self)",
        "pub fn cause(&self)",
        "pub fn position(&self)",
    ] {
        assert!(source.contains(item), "missing `{item}`");
    }
}

fn keyed_core_snapshots(q_r1: &str) -> Vec<Vec<u8>> {
    crate::litmus::Documents::new()
        .snapshots
        .into_values()
        .map(|bytes| {
            let mut snapshot: Value = serde_json::from_slice(&bytes).expect("snapshot JSON");
            for population in snapshot["populations"]
                .as_array_mut()
                .expect("snapshot populations")
            {
                if population["population"].to_string().contains("cores") {
                    let core = |key: &str, r1: &str| {
                        json!({"key": key, "type": "ix://test/litmus/Core", "fields": {
                            "r1": {"integer": r1}, "r2": {"integer": "0"},
                            "r3": {"integer": "0"}, "r4": {"integer": "0"}}})
                    };
                    population["objects"] = json!([core("p", "0"), core("q", q_r1)]);
                }
            }
            serde_json::to_vec(&snapshot).expect("snapshot JSON")
        })
        .collect()
}

fn bound_protocol_subject(key: &str) -> Value {
    let model = json!({"type": "model-binding", "selection": "test/litmus"});
    json!({"protocol": {"protocol": "Flow",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": key}},
        "roles": {"Buyer": model, "Seller": model}}})
}

/// FR-896-AC-7/8: no binding checks every admitted key; a subject binding
/// checks exactly its selected object, with the expected result computed
/// from p.r1 = 0 and q.r1 = 1 in the supplied initial state.
#[trace("TC-2800", "FR-896-AC-7", "FR-896-AC-8")]
#[test]
fn global_conformance_checks_every_key_and_honors_bound_over() {
    let protocol = order_protocol(true).replace(
        "run sequence Main {",
        "run sequence Main {\ncheck Ready using pp { k.r1 = 0 };",
    );
    let snapshots = keyed_core_snapshots("1");
    let items = json!([{"claim": "Flow", "kind": "global-conformance"}]);
    let ws = Workspace::new("gc-every-key-ac8");
    let request = request_over(
        &ws,
        &crate::litmus::unit_text(&protocol),
        &crate::litmus::resulting_document(),
        &snapshots,
        json!({"items": items.clone()}),
    );
    let every_key = analyze(&request).item(0);
    assert_eq!(
        label(&every_key),
        "refuted",
        "q does not satisfy Ready: {every_key}"
    );
    assert_eq!(every_key["counterexample"]["kind"], "deadlock");

    for (key, expected) in [("p", "proved"), ("q", "refuted")] {
        let ws = Workspace::new(&format!("gc-bound-{key}"));
        let request = request_over(
            &ws,
            &crate::litmus::unit_text(&protocol),
            &crate::litmus::resulting_document(),
            &snapshots,
            json!({"subject": bound_protocol_subject(key), "items": items.clone()}),
        );
        let item = analyze(&request).item(0);
        assert_eq!(label(&item), expected, "subject binds {key}: {item}");
    }
}

/// FR-896-AC-9: an item's subject.protocol cannot bind a different protocol.
#[trace("TC-2800", "FR-896-AC-9")]
#[test]
fn global_conformance_declines_a_binding_to_another_protocol() {
    let flow = order_protocol(true);
    let (_, other) = flow.split_once("protocol Flow").expect("Flow declaration");
    let unit = format!("{flow}\nprotocol Other{other}");
    let subject = bound_protocol_subject("p");
    let mut subject = subject;
    subject["protocol"]["protocol"] = json!("Other");
    let ws = Workspace::new("gc-other-protocol");
    let request = request_over(
        &ws,
        &crate::litmus::unit_text(&unit),
        &crate::litmus::resulting_document(),
        &keyed_core_snapshots("1"),
        json!({"subject": subject, "items": [{"claim": "Flow", "kind": "global-conformance"}]}),
    );
    let item = analyze(&request).item(0);
    assert_eq!(item["record"]["result"], "declined", "{item}");
    assert_eq!(item["record"]["cause"]["kind"], "invalid-input", "{item}");
}

/// FR-896-AC-10: protocol deadlock analysis requires an explicit protocol subject.
#[trace("TC-2532", "FR-896-AC-10")]
#[test]
fn protocol_deadlock_item_declines_without_subject_protocol() {
    let ws = Workspace::new("gc-deadlock-requires-protocol-subject");
    let request = request_over(
        &ws,
        &crate::litmus::unit_text(&order_protocol(true)),
        &crate::litmus::resulting_document(),
        &[],
        json!({"items": [{"claim": "Flow",
            "temporal": {"item_kind": "deadlock-freedom"}}]}),
    );
    let item = analyze(&request).item(0);
    assert_eq!(item["record"]["result"], "declined", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_runtime_input",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "invalid-value",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["member"], "temporal.subject.protocol",
        "{item}"
    );
    for member in [
        "obligations",
        "closure",
        "global",
        "projections",
        "counterexample",
    ] {
        assert!(
            item[member].is_null(),
            "declined item omits {member}: {item}"
        );
    }
}

/// FR-992-AC-1 to AC-3: a compiled handoff publishes source bytes by label,
/// and analyze admits that provision but refuses one without its unit.
#[trace("TC-2804", "FR-992-AC-1", "FR-992-AC-2", "FR-992-AC-3")]
#[test]
fn a_compiled_handoff_publishes_its_source_provision() {
    use fsl_analyze::analyze::document::SubjectSelection;
    use fsl_analyze::analyze::{
        AnalyzeLimits, AnalyzeRefusal, AnalyzeRequest, Engines, SourceProvision, analyze,
    };
    use fsl_command::checked_handoff::{CheckedHandoffRequest, Corpus, compile, write};
    use fsl_command::emit_checked_v2::Request;
    use fsl_foundation::source::provenance::RawSourceRef;

    let workspace = Workspace::new("gc-source-provision");
    let library_bytes = b"language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"filament.value.complete/v1\";\n\
        function helper using v(): Boolean pure { true }\n";
    let source = crate::litmus::unit_text(&order_protocol(true)).replace(
        "model M = \"test/litmus\";",
        "import \"test/protocol-helper\" as g;\nmodel M = \"test/litmus\";",
    );
    let source_path = workspace.write("program.fsl", source.as_bytes());
    let model_path = workspace.write("model.json", &crate::litmus::document());
    let library_path = workspace.write("library.fsl", library_bytes);
    let handoff_path = source_path
        .parent()
        .expect("workspace source has a parent")
        .join("compiled-handoff");
    let request = CheckedHandoffRequest {
        emit: Request {
            source: source_path.clone(),
            package_name: "qsl-2209-source-provision".to_owned(),
            models: vec![model_path],
            libraries: vec![("test/protocol-helper".to_owned(), library_path.clone())],
            execution_points: None,
            temporal_selections: Vec::new(),
            out: None,
        },
        corpus: Corpus::None,
        native_join: fsl_package::NativeJoinLimits::new(1_000_000, 1_000_000),
    };
    write(&request, &handoff_path).expect("the checked handoff is written");
    let handoff = compile(&handoff_path).expect("the handoff compiles in process");

    let unit_source = handoff.checked().graph().source();
    let library_source = RawSourceRef::new("local", format!("library:{}", library_path.display()))
        .expect("the library source label is valid");
    assert_eq!(handoff.sources().get(unit_source), Some(source.as_bytes()));
    assert_eq!(
        handoff.sources().get(&library_source),
        Some(library_bytes.as_slice())
    );
    let entries: Vec<_> = handoff.sources().entries().collect();
    assert_eq!(entries.len(), 2, "both unit and library bytes are present");
    assert!(entries.iter().any(|(authority, identity, bytes)| {
        *authority == unit_source.authority()
            && *identity == unit_source.identity()
            && *bytes == source.as_bytes()
    }));
    assert!(entries.iter().any(|(authority, identity, bytes)| {
        *authority == library_source.authority()
            && *identity == library_source.identity()
            && *bytes == library_bytes.as_slice()
    }));

    let world = crate::litmus::Documents::new();
    let selection = SubjectSelection::default_over(&world.snapshots);
    let binding = selection.binding(handoff.packages(), &world.snapshots);
    let limits = AnalyzeLimits::unbounded();
    let engines = Engines::fsl().expect("FSL engines are registered");
    let cancel = filament_exact::Cancel::new();
    let complete = AnalyzeRequest {
        binding: binding.clone(),
        items: Vec::new(),
        provision: handoff.sources().clone(),
    };
    assert!(
        analyze(handoff.checked(), &complete, &engines, &limits, &cancel).is_ok(),
        "the complete handoff provision admits"
    );

    let mut without_unit = SourceProvision::default();
    for (authority, identity, bytes) in handoff.sources().entries() {
        if (authority, identity) != (unit_source.authority(), unit_source.identity()) {
            let source = RawSourceRef::new(authority, identity).expect("valid source label");
            without_unit.insert(&source, bytes.to_vec());
        }
    }
    let incomplete = AnalyzeRequest {
        binding,
        items: Vec::new(),
        provision: without_unit,
    };
    let refusal = analyze(handoff.checked(), &incomplete, &engines, &limits, &cancel)
        .expect_err("analyze refuses a provision without the unit source")
        .into_refused()
        .expect("missing source is a refusal");
    let AnalyzeRefusal::Provision(refusal) = refusal else {
        panic!("expected a missing source refusal, got {refusal:?}");
    };
    assert_eq!(refusal.authority, unit_source.authority());
    assert_eq!(refusal.identity, unit_source.identity());
}

/// FR-990-AC-6 (TC-2831): each closure premise is `{kind, instance, node}`
/// read from the checked protocol, and a recording premise differing in any
/// one component is refused before any verdict.
#[trace("TC-2831", "FR-990-AC-6", "FSpec-FR-247-AC-3")]
#[test]
fn closure_premises_carry_their_componentwise_identity() {
    let engine = conformance("gc-premise-identity", &order_protocol(true), json!({}), &[]);
    assert_eq!(engine["closure"]["premises"], order_premises(), "{engine}");
    let admitted = conformance(
        "gc-premise-identity-recorded",
        &order_protocol(true),
        recording_member(),
        &[(
            "recording.json",
            order_recording(json!({"state": "closed", "premises": order_premises()})),
        )],
    );
    assert_eq!(admitted["closure"]["state"], "closed", "{admitted}");
    let all = order_premises().as_array().expect("premises").clone();
    let mut instanced = all.clone();
    instanced[0]["instance"] = json!("w-1");
    let mut wrong_node = all.clone();
    wrong_node[0]["node"] = json!("Flow::Main::Decide::yes");
    let mut wrong_kind = all.clone();
    wrong_kind[1]["kind"] = json!("workflow");
    wrong_kind[2]["kind"] = json!("workflow");
    for (name, set) in [
        ("instanced", instanced),
        ("wrong-node", wrong_node),
        ("wrong-kind", wrong_kind),
    ] {
        let item = conformance(
            &format!("gc-premise-identity-{name}"),
            &order_protocol(true),
            recording_member(),
            &[(
                "recording.json",
                order_recording(json!({"state": "closed", "premises": set})),
            )],
        );
        assert_eq!(
            item["record"]["diagnostic"]["code"], "invalid_runtime_input",
            "{name}: {item}"
        );
        assert!(item["obligations"].is_null(), "{name}: {item}");
    }
}
