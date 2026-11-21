// SPDX-License-Identifier: AGPL-3.0-or-later
//! The lock example's hidden choice through native Source acquisition.

use ix_trace_rs::trace;

use crate::analyze_cli_support::{analyze, label, Workspace};

/// Holder knows the received decision; Observer cannot distinguish receiving
/// the accepted branch's notice from ending in the empty denied branch.
#[trace("TC-1411", "FR-571-AC-2", "FR-573-AC-2", "FR-574-AC-1", "FR-286-AC-36")]
#[test]
fn hidden_choice_source_refutes_with_a_counterexample() {
    let ws = Workspace::new("hidden-choice-source");
    for (path, bytes) in [
        (
            "model/lock.semantic-ir.json",
            &include_bytes!("../fixtures/hidden-choice-source/model/lock.semantic-ir.json")[..],
        ),
        (
            "data/lock-free.json",
            &include_bytes!("../fixtures/hidden-choice-source/data/lock-free.json")[..],
        ),
        (
            "rules/protocols/handover-hidden-choice.fsl",
            &include_bytes!(
                "../fixtures/hidden-choice-source/rules/protocols/handover-hidden-choice.fsl"
            )[..],
        ),
        (
            "requests/protocols/protocol-unrealizable.json",
            &include_bytes!(
                "../fixtures/hidden-choice-source/requests/protocols/protocol-unrealizable.json"
            )[..],
        ),
    ] {
        ws.write(path, bytes);
    }
    let run = analyze(&ws.path("requests/protocols/protocol-unrealizable.json"));
    let document = run.document();
    eprintln!("hidden-choice Source exit={} outcome={document}", run.exit);
    assert_eq!(
        run.exit, 10,
        "valid hidden-choice Source must refute: {document}"
    );
    let items = document["items"].as_array().expect("analyze outcome items");
    assert_eq!(
        items.len(),
        1,
        "one requested realizability item: {document}"
    );
    let item = &items[0];
    assert_eq!(label(item), "refuted", "{document}");
    assert!(
        item["counterexample"].is_object(),
        "a refutation carries its counterexample: {document}"
    );
    assert_eq!(item["request_index"], 0);
    let body = &item["counterexample"];
    assert_eq!(
        body["type"], "protocol-analysis-counterexample",
        "{document}"
    );
    assert_eq!(body["kind"], "indistinguishable-histories", "{document}");
    assert_eq!(body["key"]["kind"], "realizability", "{document}");
    for member in [
        "role",
        "history_a",
        "history_b",
        "action_a",
        "action_b",
        "obligation",
        "separating_witness",
    ] {
        assert!(body[member].is_object(), "missing {member}: {document}");
    }
    assert_ne!(body["history_a"], body["history_b"], "{document}");
    assert_ne!(body["action_a"], body["action_b"], "{document}");
}

/// Complete bodies have identical semantics across actual Source and Package
/// intake, validate against published contracts, and reject structural edits.
#[trace(
    "FR-286-AC-33",
    "FR-286-AC-34",
    "FR-286-AC-35",
    "FR-286-AC-37",
    "FR-621",
    "FR-850"
)]
#[test]
fn protocol_counterexample_intake_parity_and_schema() {
    use crate::analyze_cli_support::package_selection;
    use serde_json::{json, Value};

    let original =
        include_str!("../fixtures/hidden-choice-source/rules/protocols/handover-hidden-choice.fsl");
    let replicated = "language \"ix:native\" edition \"1-draft\"; \
        profile pp = \"filament.protocol.finite-global/v1\"; model L = \"example/lock\"; \
        record Grant { granted: Boolean; } \
        protocol Handover using pp over (lock: L::Lock) on origin { \
        role Asker on L::Lock; role Holder on L::Lock; role Observer on L::Lock; \
        role Pool each L::Lock from locks max 18446744073709551615 lifetime workflow; \
        channel Granted from Pool to Observer carries Grant ordering unordered \
        delivery exactly-once-premise capacity 4 overflow reject; \
        run sequence Main { send GrantedNotice via Granted as (grant: Grant) { true }; \
        receive ObservedGrant via Granted of GrantedNotice as (seen: Grant) { true }; } \
        finish Done as (done: Boolean) { true }; }";
    let outcome_schema: Value =
        serde_json::from_str(fsl_replay::outcome::OUTCOME_SCHEMA_1).unwrap();
    let outcome_schema = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&outcome_schema)
        .unwrap();
    let provider = crate::published_fspec::json("proposals/backend-provider-v1/schema.json");
    let body_schema = json!({
        "$schema": provider["$schema"], "$defs": provider["$defs"],
        "$ref": "#/$defs/ProtocolAnalysisCounterexample",
    });
    let body_schema = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_document(
            "urn:filament:checked-package:v2".to_owned(),
            crate::published_fspec::json("proposals/checked-package-v2/schema.json"),
        )
        .compile(&body_schema)
        .unwrap();
    for (name, source, kind, role, expected_body, expected_decision, stage) in [
        (
            "hidden",
            original.to_owned(),
            "realizability",
            None,
            "indistinguishable-histories",
            None,
            "S8",
        ),
        (
            "loss",
            original.replace("exactly-once-premise", "at-most-once"),
            "realizability",
            None,
            "environment-defeat",
            None,
            "S8",
        ),
        (
            "choice",
            original.to_owned(),
            "local-projection",
            Some("Observer"),
            "projection",
            Some("choice"),
            "S8",
        ),
        (
            "replicated",
            replicated.to_owned(),
            "local-projection",
            Some("Pool"),
            "projection",
            Some("acting-instance"),
            "S7",
        ),
    ] {
        let ws = Workspace::new(&format!("protocol-counterexample-parity-{name}"));
        ws.write(
            "model.json",
            include_bytes!("../fixtures/hidden-choice-source/model/lock.semantic-ir.json"),
        );
        ws.write(
            "data.json",
            include_bytes!("../fixtures/hidden-choice-source/data/lock-free.json"),
        );
        ws.write("program.fsl", source.as_bytes());
        let mut request: Value = serde_json::from_slice(include_bytes!(
            "../fixtures/hidden-choice-source/requests/protocols/protocol-unrealizable.json"
        ))
        .unwrap();
        request["request"]["models"][0]["source"]["file"] = json!("model.json");
        request["request"]["program"]["source"]["file"] = json!("program.fsl");
        request["request"]["documents"][0]["file"] = json!("data.json");
        let item = &mut request["request"]["items"][0];
        item["kind"] = json!(kind);
        if let Some(role) = role {
            item["options"] = json!({"role": role});
            item.as_object_mut().unwrap().remove("premises");
            item["temporal"]
                .as_object_mut()
                .unwrap()
                .remove("max_history");
        }
        if name == "replicated" {
            item["temporal"]["subject"]["protocol"]["roles"]["Pool"] =
                json!({"type": "model-binding", "selection": "example/lock"});
        }
        let source_path = ws.write("source.json", request.to_string().as_bytes());
        let source_run = analyze(&source_path);
        let source_outcome = source_run.document();
        assert_eq!(source_run.exit, 10, "{name}: {source_outcome}");
        assert_eq!(source_outcome["format"], "filament-outcome/1");
        assert_eq!(source_outcome["operation"], "analyze");
        assert_eq!(source_outcome["category"], "violation");
        assert_eq!(
            source_outcome["last_stage"], stage,
            "{name}: {source_outcome}"
        );
        assert_eq!(source_outcome["items"].as_array().unwrap().len(), 1);
        let source_item = &source_outcome["items"][0];
        assert_eq!(label(source_item), "refuted");
        assert_eq!(source_item["request_index"], 0);
        assert_eq!(source_item["claim_name"], "Handover");
        assert_eq!(source_item["request"]["kind"], kind);
        assert!(
            source_item.get("replay").is_none(),
            "no invented replay target"
        );
        let body = &source_item["counterexample"];
        assert_eq!(body["kind"], expected_body);
        if let Some(decision) = expected_decision {
            assert_eq!(body["decision"]["kind"], decision);
        }
        assert_schema_accepts(&outcome_schema, &source_outcome);
        assert_schema_accepts(&body_schema, body);
        assert_protocol_body_adverses(&body_schema, body);

        let (selected, _) = package_selection(&ws, &request["request"]);
        let package_request = json!({"format": "native-analyze/1", "request": selected});
        let package_path = ws.write("package.json", package_request.to_string().as_bytes());
        std::fs::remove_file(ws.path("program.fsl")).unwrap();
        let package_run = analyze(&package_path);
        let package_outcome = package_run.document();
        assert_eq!(package_run.exit, 10, "{name}: {package_outcome}");
        assert_eq!(package_outcome["category"], "violation");
        assert_eq!(package_outcome["last_stage"], stage);
        let package_item = &package_outcome["items"][0];
        for member in [
            "request_index",
            "claim_name",
            "request",
            "record",
            "category",
        ] {
            assert_eq!(
                package_item[member], source_item[member],
                "{name}.{member}: {package_outcome}"
            );
        }
        assert_eq!(
            fsl_foundation::canonical_json::encode(&package_item["counterexample"]).unwrap(),
            fsl_foundation::canonical_json::encode(body).unwrap()
        );
        assert_schema_accepts(&outcome_schema, &package_outcome);
        assert_schema_accepts(&body_schema, &package_item["counterexample"]);
    }
}

fn assert_schema_accepts(schema: &jsonschema::JSONSchema, value: &serde_json::Value) {
    if let Err(errors) = schema.validate(value) {
        let errors: Vec<_> = errors
            .map(|error| format!("{}: {error}", error.instance_path))
            .collect();
        panic!("schema rejected emitted data: {errors:?}\n{value}");
    }
}

fn assert_protocol_body_adverses(schema: &jsonschema::JSONSchema, body: &serde_json::Value) {
    use serde_json::json;
    for member in body.as_object().unwrap().keys() {
        let mut missing = body.clone();
        missing.as_object_mut().unwrap().remove(member);
        assert!(!schema.is_valid(&missing), "required {member}");
    }
    let mut unknown = body.clone();
    unknown["unexpected"] = json!(true);
    assert!(!schema.is_valid(&unknown));
    unknown = body.clone();
    unknown["kind"] = json!("unknown-kind");
    assert!(!schema.is_valid(&unknown));
    unknown = body.clone();
    unknown["key"]["bounds"]["max_depth"] = json!(32);
    assert!(
        !schema.is_valid(&unknown),
        "body key bounds use decimal strings"
    );
    for bad in ["00", "+1", "18446744073709551616"] {
        unknown = body.clone();
        unknown["key"]["bounds"]["max_depth"] = json!(bad);
        assert!(!schema.is_valid(&unknown));
    }
    let play_pointer = match body["kind"].as_str().unwrap() {
        "indistinguishable-histories" => Some("/history_a"),
        "environment-defeat" => Some("/defeats_per_branch/0/play"),
        "projection" => None,
        _ => unreachable!(),
    };
    if let Some(play_pointer) = play_pointer {
        let steps = body
            .pointer(&format!("{play_pointer}/steps"))
            .unwrap()
            .as_array()
            .unwrap();
        let send = steps
            .iter()
            .position(|step| step.get("send_record").is_some())
            .expect("real send");
        let non_send = steps
            .iter()
            .position(|step| step.get("send_record").is_none())
            .expect("real non-send");
        let send_pointer = format!("{play_pointer}/steps/{send}");
        let non_send_pointer = format!("{play_pointer}/steps/{non_send}");
        let mut missing = body.clone();
        missing
            .pointer_mut(&send_pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("send_record");
        assert!(!schema.is_valid(&missing), "send record required");
        let mut null_record = body.clone();
        null_record.pointer_mut(&send_pointer).unwrap()["send_record"] = serde_json::Value::Null;
        assert!(!schema.is_valid(&null_record));
        let mut misplaced = body.clone();
        misplaced.pointer_mut(&non_send_pointer).unwrap()["send_record"] =
            steps[send]["send_record"].clone();
        assert!(!schema.is_valid(&misplaced), "non-send record forbidden");
        let mut mixed = body.clone();
        mixed["decision"] = json!({"kind": "environment"});
        assert!(!schema.is_valid(&mixed));
    } else {
        let mut mixed = body.clone();
        mixed["defeats_per_branch"] = json!([]);
        assert!(!schema.is_valid(&mixed));
        let mut unknown_decision = body.clone();
        unknown_decision["decision"]["kind"] = json!("unknown");
        assert!(!schema.is_valid(&unknown_decision));
    }
}
