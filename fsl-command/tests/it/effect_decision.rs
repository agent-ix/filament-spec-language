// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1046 receipts through the public native-analyze command intake.

use filament_exact::Cancel;
use fsl_command::command::RunLimits;
use fsl_foundation::canonical_json;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::cli_contract::{analyze, analyze_under, Case, Outcome};

const SOURCE: &str = include_str!("../../../examples/protocol-recommendation/workflow.native");
const MODEL: &[u8] = include_bytes!("../fixtures/cli-contract/counter.semantic-ir.json");
const SNAPSHOT: &[u8] = include_bytes!("../../../examples/protocol-recommendation/start.json");

struct Fixture {
    case: Case,
    request: Value,
    baseline: Outcome,
    receipt: Value,
}

impl Fixture {
    fn new() -> Self {
        Self::with_input(SOURCE, MODEL)
    }

    fn with_input(source: &str, model: &[u8]) -> Self {
        let case = Case::new();
        case.write("workflow.native", source.as_bytes());
        case.write("counter.json", model);
        case.write("start.json", SNAPSHOT);
        let mut request: Value = serde_json::from_str(include_str!(
            "../../../examples/protocol-recommendation/analyze.json"
        ))
        .unwrap();
        request["request"]["models"][0]["source"]["file"] = json!("counter.json");
        let baseline = analyze(&case.json("analyze.json", &request));
        assert_eq!(baseline.exit, 10, "genuine baseline: {}", baseline.document);
        let item = &baseline.document["items"][0];
        assert_eq!(item["replay"]["reproduced"], true, "{item}");
        let decision = item["counterexample"]["prefix"]
            .as_array()
            .unwrap()
            .iter()
            .map(|step| &step["transition"]["recommendation"])
            .find(|record| record["type"] == "protocol-recommendation")
            .unwrap()
            .clone();
        assert_eq!(decision["type"], "protocol-recommendation", "{item}");
        let receipt = json!({"contract_version": "filament.effect-decision/v1", "decision": decision,
            "authority_grant": {}, "idempotency_key": "caller-key", "effect_request": {},
            "transport_receipt": {}, "lineage": {}});
        Self {
            case,
            request,
            baseline,
            receipt,
        }
    }

    fn select(&mut self, identity: &str, receipt: &Value) {
        self.select_bytes(identity, &canonical_json::encode(receipt).unwrap());
    }

    fn select_bytes(&mut self, identity: &str, bytes: &[u8]) {
        let file = format!("{identity}.json");
        self.case.write(&file, bytes);
        let reference = json!({"authority": "receipt-authority", "identity": identity});
        self.request["request"]["documents"]
            .as_array_mut()
            .unwrap()
            .push(json!({"file": file, "identity": reference}));
        let item = &mut self.request["request"]["items"][0];
        if item.get("effect_decisions").is_none() {
            item["effect_decisions"] = json!([]);
        }
        item["effect_decisions"]
            .as_array_mut()
            .unwrap()
            .push(reference);
    }

    fn run(&self) -> Outcome {
        analyze(&self.case.json("selected.json", &self.request))
    }

    fn valid(&mut self) {
        self.select("first", &self.receipt.clone());
        let bound = self.run();
        assert_eq!(bound.exit, 10, "{}", bound.document);
        assert_eq!(
            bound.document, self.baseline.document,
            "binding must not change formal output"
        );
        let schema: Value = serde_json::from_str(fsl_replay::outcome::OUTCOME_SCHEMA_1).unwrap();
        assert!(
            jsonschema::JSONSchema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .compile(&schema)
                .unwrap()
                .is_valid(&bound.document),
            "{}",
            bound.document
        );
    }
}

fn refused(outcome: &Outcome, cause: &str, reason: &str, pointer: &str) {
    assert_eq!(outcome.exit, 20, "{}", outcome.document);
    let diagnostic = &outcome.document["diagnostics"][0];
    assert_eq!(
        diagnostic["code"], "invalid_runtime_input",
        "{}",
        outcome.document
    );
    assert_eq!(diagnostic["cause"], cause, "{diagnostic}");
    let message = diagnostic["message"].as_str().unwrap();
    assert!(
        message.starts_with(&format!("effect-decision {reason}")),
        "{diagnostic}"
    );
    assert!(
        message.contains("/items/0/effect_decisions/"),
        "{diagnostic}"
    );
    assert!(message.contains("receipt-authority"), "{diagnostic}");
    assert!(message.contains(pointer), "{diagnostic}");
    assert_eq!(diagnostic["details"], Value::Null);
    assert_eq!(diagnostic["locus"], Value::Null);
    let schema: Value = serde_json::from_str(fsl_replay::outcome::OUTCOME_SCHEMA_1).unwrap();
    assert!(
        jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .compile(&schema)
            .unwrap()
            .is_valid(&outcome.document),
        "refusal must keep the published schema: {}",
        outcome.document
    );
}

#[test]
#[trace("FR-1046-AC-1", "FR-1046-AC-8", "FR-1046-AC-12")]
fn receipt_binding_preserves_actual_source_outcome_and_model_truth() {
    let mut fixture = Fixture::new();
    fixture.receipt["effect_observation"] = json!({"claimed": "external success"});
    fixture.receipt["authority_grant"] = json!({"caller": "descriptive only"});
    fixture.valid();
}

#[test]
#[trace("FR-1046-AC-3")]
fn every_canonical_decision_component_is_matched() {
    let mut fixture = Fixture::new();
    fixture.valid();
    let mut declaration: Value =
        serde_json::from_str(fixture.receipt["decision"]["operation"].as_str().unwrap()).unwrap();
    declaration["package"] = json!("different-domain");
    let same_name_other_identity =
        String::from_utf8(canonical_json::encode(&declaration).unwrap()).unwrap();
    for (member, replacement) in [
        ("type", json!("other")),
        ("node", json!("other")),
        ("role", json!("other")),
        ("instance", json!("1")),
        ("actor", json!({"type": "reference", "identity": "other"})),
        ("operation", json!("other")),
        ("operation", json!(same_name_other_identity)),
        ("receiver", json!({"type": "boolean", "value": false})),
        ("arguments", json!([{"type": "boolean", "value": true}])),
    ] {
        let mut receipt = fixture.receipt.clone();
        receipt["decision"][member] = replacement;
        fixture
            .case
            .write("first.json", &canonical_json::encode(&receipt).unwrap());
        let outcome = fixture.run();
        refused(&outcome, "invalid-value", "wrong-decision", "/decision");
        assert!(outcome.document["items"][0].get("counterexample").is_none());
        assert!(outcome.document["items"][0].get("certificate").is_none());
        assert_eq!(
            outcome.document["items"][0]["record"]["diagnostic"]["code"],
            "invalid_runtime_input"
        );
    }
}

#[test]
#[trace("FR-1046-AC-5", "FR-1046-AC-6", "FR-1046-AC-12")]
fn the_full_canonical_grant_and_caller_key_distinguish_requests() {
    let mut fixture = Fixture::new();
    fixture.valid();
    let mut different_grant = fixture.receipt.clone();
    different_grant["authority_grant"] = json!({"descriptive": {"scope": ["other"]}});
    different_grant["effect_request"] = json!({"different": "request"});
    fixture.select("different-grant", &different_grant);
    let mut different_key = fixture.receipt.clone();
    different_key["idempotency_key"] = json!("another-caller-key");
    different_key["effect_request"] = json!({"different": "request"});
    fixture.select("different-key", &different_key);
    assert_eq!(fixture.run().document, fixture.baseline.document);
}

#[test]
#[trace("FR-1046-AC-5", "FR-1046-AC-6", "FR-1046-AC-10")]
fn retries_reconcile_but_changed_effect_request_refuses_later_selector() {
    let mut fixture = Fixture::new();
    fixture.valid();
    fixture.select("same", &fixture.receipt.clone());
    let mut retry = fixture.receipt.clone();
    retry["transport_receipt"] = json!({"attempt": "second"});
    retry["lineage"] = json!({"previous": "caller-history"});
    fixture.select("retry", &retry);
    assert_eq!(fixture.run().document, fixture.baseline.document);
    retry["effect_request"] = json!({"different": true});
    fixture.select("conflict", &retry);
    let outcome = fixture.run();
    refused(
        &outcome,
        "invalid-value",
        "request-key-contradiction",
        "/effect_request",
    );
    let message = outcome.document["diagnostics"][0]["message"]
        .as_str()
        .unwrap();
    assert!(
        message.contains("/3") && message.contains("first") && message.contains("conflict"),
        "{message}"
    );
    // The first selector's match failure must precede the later contradiction.
    let mut wrong = fixture.receipt.clone();
    wrong["decision"]["node"] = json!("not-emitted");
    fixture
        .case
        .write("first.json", &canonical_json::encode(&wrong).unwrap());
    refused(
        &fixture.run(),
        "invalid-value",
        "wrong-decision",
        "/decision",
    );
}

#[test]
#[trace("FR-1046-AC-7", "FR-1046-AC-10")]
fn malformed_receipt_causes_and_pointers_are_preserved_before_matching() {
    let mut fixture = Fixture::new();
    fixture.valid();
    let mut cases = vec![(b"{".to_vec(), "malformed-json", "\"\"")];
    let mut missing = fixture.receipt.clone();
    missing.as_object_mut().unwrap().remove("lineage");
    cases.push((
        canonical_json::encode(&missing).unwrap(),
        "missing-member",
        "/lineage",
    ));
    let mut unknown = fixture.receipt.clone();
    unknown["identity"] = json!({});
    cases.push((
        canonical_json::encode(&unknown).unwrap(),
        "unknown-member",
        "/identity",
    ));
    let mut wrong = fixture.receipt.clone();
    wrong["effect_request"] = json!(false);
    cases.push((
        canonical_json::encode(&wrong).unwrap(),
        "wrong-value-kind",
        "/effect_request",
    ));
    let mut version = fixture.receipt.clone();
    version["contract_version"] = json!("filament.effect-decision/v2");
    version["decision"]["node"] = json!("not-emitted");
    cases.push((
        canonical_json::encode(&version).unwrap(),
        "invalid-value",
        "/contract_version",
    ));
    let bytes = canonical_json::encode(&fixture.receipt).unwrap();
    let mut noncanonical = bytes.clone();
    noncanonical.push(b'\n');
    cases.push((noncanonical, "invalid-value", "\"\""));
    let text = String::from_utf8(bytes)
        .unwrap()
        .replace("\"lineage\":{}", "\"lineage\":{},\"lineage\":{}");
    cases.push((text.into_bytes(), "duplicate-member", "/lineage"));
    let mut domain = fixture.receipt.clone();
    domain["authority_grant"]["unsafe_integer"] = json!(9_007_199_254_740_992_u64);
    cases.push((
        serde_json::to_vec(&domain).unwrap(),
        "invalid-value",
        "/authority_grant/unsafe_integer",
    ));
    let overflow = serde_json::to_string(&domain)
        .unwrap()
        .replace("9007199254740992", "1e400");
    cases.push((
        overflow.into_bytes(),
        "invalid-value",
        "/authority_grant/unsafe_integer",
    ));
    let duplicate = serde_json::to_string(&fixture.receipt).unwrap().replace(
        "\"authority_grant\":{}",
        "\"authority_grant\":{\"scope\":{},\"scope\":{}}",
    );
    cases.push((
        duplicate.into_bytes(),
        "duplicate-member",
        "/authority_grant/scope",
    ));
    for (bytes, cause, pointer) in cases {
        fixture.case.write("first.json", &bytes);
        let outcome = fixture.run();
        refused(&outcome, cause, "malformed-receipt", pointer);
        assert_eq!(
            outcome.document["items"],
            json!([]),
            "canonical admission is whole-request refusal"
        );
    }
}

#[test]
#[trace("FR-1046-AC-8")]
fn selector_shapes_and_unresolved_documents_refuse_at_request_pointer() {
    let mut fixture = Fixture::new();
    fixture.request["request"]["items"][0]["effect_decisions"] = json!([]);
    assert_eq!(fixture.run().document, fixture.baseline.document);
    for value in [
        Value::Null,
        json!({}),
        json!([{"authority": "x"}]),
        json!([{"authority": "x", "identity": "y", "extra": true}]),
        json!([{"authority": "x", "identity": "y"}]),
    ] {
        fixture.request["request"]["items"][0]["effect_decisions"] = value;
        let outcome = fixture.run();
        assert_eq!(outcome.exit, 20, "{}", outcome.document);
        assert!(
            outcome
                .bytes
                .windows(b"effect_decisions".len())
                .any(|window| window == b"effect_decisions"),
            "{}",
            outcome.document
        );
    }
    fixture.request["request"]["items"][0]
        .as_object_mut()
        .unwrap()
        .remove("effect_decisions");
    fixture.request["request"]["effect_decisions"] = json!([]);
    assert_eq!(fixture.run().exit, 20);
}

#[test]
#[trace("FR-1046-AC-9")]
fn receipt_bytes_use_the_exact_aggregate_dependent_allowance() {
    let mut fixture = Fixture::new();
    fixture.valid();
    let path = fixture.case.json("selected.json", &fixture.request);
    let bytes = SOURCE.len()
        + MODEL.len()
        + SNAPSHOT.len()
        + canonical_json::encode(&fixture.receipt).unwrap().len();
    let limits = RunLimits {
        dependent_bytes: bytes,
        ..RunLimits::default()
    };
    let run = |limits| analyze_under(&path, limits, &Default::default(), &Cancel::new());
    assert_eq!(run(limits).document, fixture.baseline.document);
    let denied = run(RunLimits {
        dependent_bytes: bytes - 1,
        ..limits
    });
    assert_eq!(denied.exit, 22, "{}", denied.document);
    assert!(
        denied.document.to_string().contains("dependent_bytes"),
        "{}",
        denied.document
    );
}

#[test]
#[trace("FR-1046-AC-4", "FR-1046-AC-10")]
fn equal_actual_recommendations_in_other_items_do_not_change_local_matching() {
    let mut fixture = Fixture::new();
    fixture.valid();
    let second = fixture.request["request"]["items"][0].clone();
    fixture.request["request"]["items"][0]
        .as_object_mut()
        .unwrap()
        .remove("effect_decisions");
    fixture.request["request"]["items"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let both = fixture.run();
    assert_eq!(both.exit, 10, "{}", both.document);
    assert_eq!(both.document["items"].as_array().unwrap().len(), 2);
    assert_eq!(
        both.document["items"][0]["counterexample"],
        both.document["items"][1]["counterexample"]
    );
    let mut wrong = fixture.receipt.clone();
    wrong["decision"]["node"] = json!("not-in-item");
    fixture
        .case
        .write("first.json", &canonical_json::encode(&wrong).unwrap());
    let refused = fixture.run();
    assert_eq!(refused.exit, 20, "{}", refused.document);
    assert_eq!(
        refused.document["items"][0],
        fixture.baseline.document["items"][0]
    );
    assert!(refused.document["items"][1].get("counterexample").is_none());
}

#[test]
#[trace("FR-1046-AC-3")]
fn parameter_order_and_replicated_actor_retain_their_typed_identity() {
    let mut model: Value = serde_json::from_slice(MODEL).unwrap();
    let mut params = Vec::new();
    for name in ["allowA", "allowB"] {
        let mut parameter = model["types"][1]["fields"][0].clone();
        let identity = format!("ix://test/counter/Counter/inc/{name}");
        parameter["identity"] = json!(identity);
        parameter["name"] = json!(name);
        parameter["typeRef"] = json!("ix://filament/native/Boolean");
        parameter["origin"]["generated"]["generatorIdentity"] = json!(identity);
        parameter["origin"]["generated"]["inputIdentities"] = json!([identity]);
        params.push(parameter);
    }
    model["types"][1]["operations"][0]["params"] = json!(params);
    let source = SOURCE.replace("self.value < 2", "self.value < 2 and allowA and not allowB");
    let mut fixture = Fixture::with_input(&source, &serde_json::to_vec(&model).unwrap());
    fixture.valid();
    let mut reordered = fixture.receipt.clone();
    let arguments = reordered["decision"]["arguments"].as_array_mut().unwrap();
    assert_eq!(arguments.len(), 2);
    assert_ne!(arguments[0], arguments[1]);
    arguments.swap(0, 1);
    fixture
        .case
        .write("first.json", &canonical_json::encode(&reordered).unwrap());
    refused(
        &fixture.run(),
        "invalid-value",
        "wrong-decision",
        "/decision",
    );

    let source = SOURCE.replace(
        "observe role Auditor on M::Counter;",
        "observe role Auditor each M::Counter from counters max 1 lifetime workflow;",
    );
    let mut fixture = Fixture::with_input(&source, MODEL);
    fixture.valid();
    let mut actor = fixture.receipt.clone();
    assert_eq!(actor["decision"]["actor"]["identity"], "counter");
    actor["decision"]["actor"]["identity"] = json!("different-actor");
    fixture
        .case
        .write("first.json", &canonical_json::encode(&actor).unwrap());
    refused(
        &fixture.run(),
        "invalid-value",
        "wrong-decision",
        "/decision",
    );
}

#[test]
#[trace("FR-1046-AC-4")]
fn repeated_indistinguishable_actual_occurrences_refuse_ambiguity() {
    let source = SOURCE.replace(
        "recommend Advise by Auditor on M::Counter::inc contracts [SpaceAvailable]\n        as (advice: Boolean) { true };",
        "repeat Again by Seller visible () max 2 while { true } sequence Body {\n\
         recommend Advise by Auditor on M::Counter::inc contracts [SpaceAvailable]\n\
         as (advice: Boolean) { true }; } exhausted sequence End { }");
    assert_ne!(source, SOURCE);
    let mut fixture = Fixture::with_input(&source, MODEL);
    let actual: Vec<_> = fixture.baseline.document["items"][0]["counterexample"]["prefix"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|step| step["transition"].get("recommendation"))
        .collect();
    assert_eq!(
        actual.len(),
        2,
        "actual repeated events: {}",
        fixture.baseline.document
    );
    assert_eq!(actual[0], actual[1]);
    fixture.select("loop", &fixture.receipt.clone());
    refused(
        &fixture.run(),
        "invalid-value",
        "ambiguous-decision",
        "/decision",
    );
}

#[test]
#[trace("FR-1046-AC-4")]
fn another_items_recommendation_does_not_supply_a_missing_local_match() {
    let second = SOURCE.split_once("protocol Review").unwrap().1;
    let source = format!(
        "{SOURCE}\nprotocol Other{}",
        second.replace("Advise", "DifferentAdvice")
    );
    let mut fixture = Fixture::with_input(&source, MODEL);
    let mut item = fixture.request["request"]["items"][0].clone();
    item["claim"] = json!("Other");
    let mut subject = fixture.request["request"]["subject"].clone();
    subject["protocol"]["protocol"] = json!("Other");
    item["temporal"]["subject"] = subject;
    fixture.request["request"]["items"]
        .as_array_mut()
        .unwrap()
        .push(item);
    let baseline = fixture.run();
    assert_eq!(baseline.exit, 10, "{}", baseline.document);
    assert_eq!(baseline.document["items"][1]["replay"]["reproduced"], true);
    fixture.select("other-item", &fixture.receipt.clone());
    let selectors = fixture.request["request"]["items"][0]
        .as_object_mut()
        .unwrap()
        .remove("effect_decisions")
        .unwrap();
    fixture.request["request"]["items"][1]["effect_decisions"] = selectors;
    let refused = fixture.run();
    assert_eq!(refused.exit, 20, "{}", refused.document);
    assert_eq!(refused.document["items"][0], baseline.document["items"][0]);
    assert!(refused.document["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .starts_with("effect-decision wrong-decision"));
}

#[test]
#[trace("FR-1046-AC-11")]
fn a_bounded_item_without_trace_keeps_its_engine_outcome() {
    let mut fixture = Fixture::new();
    fixture.request["request"]["items"][0]["temporal"]["limits"] = json!({"max_states": 1});
    let baseline = fixture.run();
    assert_ne!(
        baseline.exit, 10,
        "bounded fixture must not produce a complete counterexample: {}",
        baseline.document
    );
    assert!(baseline.document["items"][0]
        .get("counterexample")
        .is_none());
    let mut wrong = fixture.receipt.clone();
    wrong["decision"]["node"] = json!("not-explored");
    fixture.select("unexplored", &wrong);
    assert_eq!(fixture.run().document, baseline.document);
}

#[test]
#[trace("FR-1046-AC-2")]
fn checked_package_receipts_match_the_same_actual_recommendation() {
    let mut fixture = Fixture::new();
    fixture.request["request"]["items"][0]["kind"] = json!("temporal-satisfaction");
    let source = fixture.run();
    assert_eq!(
        source.exit, 10,
        "same supported Source selector: {}",
        source.document
    );
    assert_eq!(source.document["items"][0]["replay"]["reproduced"], true);
    let compile = json!({"format": "native-compile/1", "request": {
        "program": fixture.request["request"]["program"], "models": fixture.request["request"]["models"],
        "handoff": {"directory": "handoff", "package_name": "recommendation", "native_join": {
            "work_units": 1_u64 << 40, "storage": 1_u64 << 24}}}});
    let emitted = fsl_command::command::compile(&fixture.case.json("compile.json", &compile))
        .expect("public checked handoff");
    let package: Value = serde_json::from_slice(&emitted).unwrap();
    let protocol = package["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["semantic_form"] == "protocol_clause")
        .unwrap();
    assert_eq!(
        protocol["node_id"]["package"], "recommendation",
        "public Source namespace"
    );
    let source_map: Vec<_> = package["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["node_id"] == protocol["node_id"])
        .collect();
    eprintln!(
        "Package baseline selector: {}; retained protocol: {}; source map: {}",
        fixture.request["request"]["items"][0],
        protocol,
        json!(source_map)
    );
    fixture.request["request"]
        .as_object_mut()
        .unwrap()
        .remove("program");
    fixture.request["request"]["package"] = json!({"directory": "handoff"});
    let baseline = fixture.run();
    assert_eq!(
        baseline.exit, 10,
        "independent Package baseline without receipts: {}",
        baseline.document
    );
    fixture.select("package", &fixture.receipt.clone());
    let bound = fixture.run();
    assert_eq!(bound.document, baseline.document, "{}", bound.document);
}
