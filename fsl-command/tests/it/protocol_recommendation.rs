// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed recommendations through the command entry used by `fsl analyze`.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::cli_contract::{analyze, Case};

const SOURCE: &str = include_str!("../../../examples/protocol-recommendation/workflow.native");
const REQUEST: &str = include_str!("../../../examples/protocol-recommendation/analyze.json");
const SNAPSHOT: &[u8] = include_bytes!("../../../examples/protocol-recommendation/start.json");
const MODEL: &[u8] = include_bytes!("../fixtures/cli-contract/counter.semantic-ir.json");

fn run(source: &str) -> crate::cli_contract::Outcome {
    run_with_snapshot(source, SNAPSHOT)
}

fn run_with_snapshot(source: &str, snapshot: &[u8]) -> crate::cli_contract::Outcome {
    run_with_model(source, snapshot, MODEL)
}

fn run_with_model(source: &str, snapshot: &[u8], model: &[u8]) -> crate::cli_contract::Outcome {
    let case = Case::new();
    case.write("workflow.native", source.as_bytes());
    case.write("start.json", snapshot);
    case.write("counter.json", model);
    let mut request: Value = serde_json::from_str(REQUEST).expect("example request");
    request["request"]["models"][0]["source"]["file"] = json!("counter.json");
    analyze(&case.json("analyze.json", &request))
}

/// Finite argument domains are filtered by the selected precondition and
/// the surviving arguments appear in parameter order in the record.
#[test]
#[trace("FSpec-FR-822-AC-3")]
fn a_recommendation_carries_its_precondition_enabled_arguments() {
    let mut model: Value = serde_json::from_slice(MODEL).expect("counter model");
    let counter = &mut model["types"][1];
    let mut parameter = counter["fields"][0].clone();
    let identity = "ix://test/counter/Counter/inc/allowed";
    parameter["identity"] = json!(identity);
    parameter["name"] = json!("allowed");
    parameter["typeRef"] = json!("ix://filament/native/Boolean");
    parameter["origin"]["generated"]["generatorIdentity"] = json!(identity);
    parameter["origin"]["generated"]["inputIdentities"] = json!([identity]);
    counter["operations"][0]["params"] = json!([parameter]);
    let source = SOURCE.replace("self.value < 2", "self.value < 2 and allowed");
    let outcome = run_with_model(
        &source,
        SNAPSHOT,
        &serde_json::to_vec(&model).expect("model bytes"),
    );
    assert_eq!(outcome.exit, 10, "{}", outcome.document);
    let item = &outcome.document["items"][0];
    let prefix = &item["counterexample"]["prefix"];
    assert_eq!(
        prefix[0]["transition"]["recommendation"]["arguments"],
        json!([{"type": "boolean", "value": true}]),
        "{item}"
    );
    assert_eq!(
        prefix[1]["transition"]["arguments"],
        json!([{"type": "boolean", "value": true}]),
        "{item}"
    );
    assert_eq!(
        item["exploration"]["transitions"], "2",
        "one enabled argument vector at each step: {item}"
    );
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// The selected precondition disables the advisory call at capacity.
#[test]
#[trace("FSpec-FR-822-AC-3")]
fn a_false_precondition_disables_the_recommendation() {
    let mut snapshot: Value = serde_json::from_slice(SNAPSHOT).expect("snapshot");
    snapshot["populations"][0]["objects"][0]["fields"]["value"]["integer"] = json!("2");
    let outcome = run_with_snapshot(
        SOURCE,
        &serde_json::to_vec(&snapshot).expect("snapshot bytes"),
    );
    assert_eq!(outcome.exit, 10, "{}", outcome.document);
    let counterexample = &outcome.document["items"][0]["counterexample"];
    assert_eq!(counterexample["kind"], "deadlock");
    assert_eq!(counterexample["prefix"], json!([]), "{counterexample}");
    assert!(counterexample["blocked"]
        .to_string()
        .contains("Main/Advise"));
}

/// The driver's analysis entry returns a recommendation followed by a model
/// write, and replays that prefix before reporting its deliberate deadlock.
#[test]
#[trace("FSpec-FR-822-AC-3", "FSpec-FR-182-AC-6", "FSpec-FR-182-AC-7")]
fn a_recommendation_reaches_the_analyze_counterexample() {
    let outcome = run(SOURCE);
    assert_eq!(outcome.exit, 10, "{}", outcome.document);
    let item = &outcome.document["items"][0];
    assert_eq!(item["counterexample"]["kind"], "deadlock");
    assert_eq!(item["replay"]["reproduced"], true);
    let prefix = item["counterexample"]["prefix"].as_array().expect("prefix");
    assert_eq!(prefix.len(), 2, "recommendation then attempt: {item}");
    let transition = &prefix[0]["transition"];
    assert_eq!(transition["step"], "recommend");
    let recommendation = &transition["recommendation"];
    assert_eq!(recommendation["type"], "protocol-recommendation");
    assert_eq!(recommendation["node"], "Main/Advise");
    assert_eq!(recommendation["role"], "Auditor");
    assert_eq!(recommendation["instance"], "0");
    assert!(recommendation.get("actor").is_none());
    let operation: Value = serde_json::from_str(
        recommendation["operation"]
            .as_str()
            .expect("operation identity"),
    )
    .expect("canonical declaration identity");
    assert_eq!(
        operation,
        json!({"package": "test/counter", "node": "ix://test/counter/Counter/inc"})
    );
    assert_eq!(recommendation["receiver"]["identity"], "counter");
    assert_eq!(recommendation["arguments"], json!([]));
    for forbidden in ["post_state", "result", "effect"] {
        assert!(recommendation.get(forbidden).is_none(), "{recommendation}");
    }
    assert_eq!(prefix[1]["transition"]["step"], "attempt");
    assert_eq!(
        recommendation["operation"],
        prefix[1]["transition"]["operation"]
    );
    assert_eq!(
        item["exploration"]["transitions"], "2",
        "one recommendation per application, regardless of possible post-states: {item}"
    );
    let semantic = |index: usize| &prefix[index]["post_state"]["state"]["semantic"];
    let field = |index: usize| {
        semantic(index)["entries"][0]["value"]["entries"][0]["value"]["fields"][0]["value"]["value"]
            .clone()
    };
    assert_eq!(field(0), "0", "recommendation leaves the supplied value");
    assert_eq!(field(1), "1", "Seller's attempt writes the model value");
}

/// A selected postcondition that admits no post-state cannot disable advice.
#[test]
#[trace("FSpec-FR-822-AC-3")]
fn a_recommendation_ignores_selected_postconditions() {
    let source = SOURCE
        .replace(
            "protocol Review",
            "post Impossible using v on M::Counter::inc { false }\nprotocol Review",
        )
        .replace(
            "contracts [SpaceAvailable]\n",
            "contracts [SpaceAvailable, Impossible]\n",
        );
    let outcome = run(&source);
    assert_eq!(outcome.exit, 10, "{}", outcome.document);
    let item = &outcome.document["items"][0];
    assert_eq!(
        item["counterexample"]["prefix"][0]["transition"]["step"], "recommend",
        "{item}"
    );
    assert_eq!(
        item["counterexample"]["prefix"].as_array().map(Vec::len),
        Some(2),
        "{item}"
    );
    assert_eq!(item["exploration"]["transitions"], "2", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// The acting object is carried inside the record for a replicated role.
#[test]
#[trace("FSpec-FR-822-AC-3")]
fn a_replicated_auditor_recommendation_names_its_actor() {
    let source = SOURCE.replace(
        "observe role Auditor on M::Counter;",
        "observe role Auditor each M::Counter from counters max 1 lifetime workflow;",
    );
    let outcome = run(&source);
    assert_eq!(outcome.exit, 10, "{}", outcome.document);
    let prefix = &outcome.document["items"][0]["counterexample"]["prefix"];
    let transition = prefix
        .as_array()
        .expect("prefix")
        .iter()
        .map(|step| &step["transition"])
        .find(|transition| transition["step"] == "recommend")
        .expect("replicated recommendation");
    assert_eq!(transition["recommendation"]["actor"]["type"], "reference");
    assert_eq!(transition["recommendation"]["actor"]["identity"], "counter");
    assert_eq!(transition["recommendation"]["actor"], transition["actor"]);
    assert_eq!(transition["recommendation"]["instance"], "0");
}

/// A source observe boundary refuses an attempt before an item can step.
#[test]
#[trace("FSpec-FR-182-AC-7", "FSpec-FR-822-AC-5")]
fn an_observed_attempt_refuses_at_the_command_checker() {
    let outcome = run(&SOURCE.replace("run sequence Main", "run observe sequence Main"));
    assert_eq!(outcome.exit, 20, "{}", outcome.document);
    assert_eq!(outcome.document["last_stage"], "S3");
    assert_eq!(outcome.document["items"], json!([]));
    let diagnostic = &outcome.document["diagnostics"][0];
    assert_eq!(diagnostic["code"], "ill_typed", "{diagnostic}");
    assert!(diagnostic.to_string().contains("attempt"), "{diagnostic}");
    assert!(diagnostic.to_string().contains("Main"), "{diagnostic}");
}
