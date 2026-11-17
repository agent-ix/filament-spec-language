// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-896, FR-990, FR-991 and FR-992 (TC-2532, TC-2800 to TC-2804): a
//! `global-conformance` claim over a protocol is decided by a real engine,
//! reports per-obligation and global results beside an independent
//! complete-global-conformance closure record, and its counterexample
//! replays in process. Driven through the in-process command adapter.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, label, litmus_request, order_protocol, Workspace};

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
#[ignore = "QSL-2209: global-conformance settles unsupported; no per-obligation, closure or global result is reported"]
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

/// QSL-2209 AC-2; FR-896-AC-3, FR-991-AC-4: over a recording, a send on
/// `Tell` where the protocol enables none refutes conformance with an
/// `unexpected` enabledness violation, and the counterexample replays.
#[trace(
    "TC-2532",
    "TC-2803",
    "FR-896-AC-3",
    "FR-991-AC-4",
    "FSpec-FR-059-AC-2"
)]
#[test]
#[ignore = "QSL-2209: no replay-mode global-conformance engine reports or replays an unexpected send"]
fn an_out_of_protocol_send_refutes_conformance() {
    let mut recording = order_recording(json!({"state": "closed", "premises": order_premises()}));
    recording["observations"]
        .as_array_mut()
        .expect("observations")
        .insert(
            0,
            json!({"kind": "send", "node": "Flow::Main::Ship::Told", "role": "Seller",
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
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// QSL-2209 AC-3; FR-990-AC-2, FR-991-AC-2: the refutation without the
/// `receive` is written in the protocol-analysis record form: the closure
/// obligation violated with a witness into the counterexample, closure
/// `open`, and an item `replay` that reproduced.
#[trace("TC-2800", "TC-2802", "FR-990-AC-2", "FR-991-AC-2", "FR-896-AC-2")]
#[test]
#[ignore = "QSL-2209: global-conformance writes no protocol-analysis record with obligations, closure and global members"]
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
#[ignore = "QSL-2209: a limited global-conformance run reports no pending obligations or incomplete closure"]
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
    assert!(obligations(&item)
        .iter()
        .any(|entry| entry["verdict"] == "pending"));
    assert_eq!(item["closure"]["state"], "incomplete");
    assert_eq!(item["global"]["complete"], false);
}

/// QSL-2209 AC-4; FR-896-AC-4: a role whose model the package does not
/// select is refused naming the role, and nothing is settled.
#[trace("TC-2532", "FR-896-AC-4")]
#[test]
#[ignore = "QSL-2209: a missing role implementation is not refused before analysis"]
fn a_missing_role_implementation_refuses() {
    let unit = order_protocol(true).replace(
        "role Seller on M::Cell;",
        "role Seller on M::Cell;\nrole Courier on N::Truck;",
    );
    let item = conformance("gc-missing", &unit, json!({}), &[]);
    let diagnostic = &item["diagnostics"][0];
    assert_eq!(diagnostic["code"], "missing_declaration", "{item}");
    assert_eq!(diagnostic["cause"], "missing-name");
    assert!(diagnostic.to_string().contains("Courier"), "{item}");
    assert!(item["record"]["result"].is_null(), "{item}");
}

/// FR-990-AC-3: over a recording the closure record is the recording's, and
/// only `closed` gives complete global success; `open` and `contradicted`
/// withhold it without changing a decided verdict.
#[trace("TC-2801", "FR-990-AC-3", "FSpec-FR-059-AC-5", "FSpec-FR-247-AC-5")]
#[test]
#[ignore = "QSL-2209: no global result or closure record is reported over a recording"]
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
            .cloned()
            .collect()
    };
    assert_eq!(decided(&open), decided(&closed));
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
#[ignore = "QSL-2209: the recording's closure premise set is not checked by a strict reader"]
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
    crossed[0] = json!({"kind": "execution", "instance": "w-2", "node": "Flow::Done"});
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
            item["diagnostics"][0]["code"], "invalid_runtime_input",
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
#[ignore = "QSL-2209: fsl_analyze::protocol_analysis::conformance::replay_counterexample is not published"]
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

/// FR-992-AC-1 to AC-3: a compiled handoff publishes its sources by label.
/// The coder replaces this surface check with a test over a handoff of the
/// order fixture and one library: exactly two entries with the files' bytes,
/// `get` by the unit's provenance label, and `entries()` accepted as a
/// replay request's byte provision (refused without the unit entry).
#[trace("TC-2804", "FR-992-AC-1", "FR-992-AC-2", "FR-992-AC-3")]
#[test]
#[ignore = "QSL-2209: CompiledHandoff::sources() and SourceProvision::entries() are not published"]
fn a_compiled_handoff_publishes_its_source_provision() {
    let handoff = crate_source("fsl-command/src/checked_handoff.rs");
    assert!(
        handoff.contains("pub fn sources(&self) -> &fsl_analyze::analyze::SourceProvision"),
        "CompiledHandoff::sources() not published"
    );
    let request = crate_source("fsl-analyze/src/analyze/request.rs");
    assert!(
        request.contains("pub fn entries(&self)"),
        "SourceProvision::entries() not published"
    );
}

/// FR-990-AC-6 (TC-2831): each closure premise is `{kind, instance, node}`
/// read from the checked protocol, and a recording premise differing in any
/// one component is refused before any verdict.
#[trace("TC-2831", "FR-990-AC-6", "FSpec-FR-247-AC-3")]
#[test]
#[ignore = "QSL-2210: closure premises do not carry the componentwise {kind, instance, node} identity yet"]
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
            item["diagnostics"][0]["code"], "invalid_runtime_input",
            "{name}: {item}"
        );
        assert!(item["obligations"].is_null(), "{name}: {item}");
    }
}
