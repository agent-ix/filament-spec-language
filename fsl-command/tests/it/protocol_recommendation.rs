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
    let case = Case::new();
    case.write("workflow.native", source.as_bytes());
    case.write("start.json", SNAPSHOT);
    case.write("counter.json", MODEL);
    let mut request: Value = serde_json::from_str(REQUEST).expect("example request");
    request["request"]["models"][0]["source"]["file"] = json!("counter.json");
    analyze(&case.json("analyze.json", &request))
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
    assert_eq!(recommendation["receiver"]["identity"], "counter");
    assert_eq!(recommendation["arguments"], json!([]));
    for forbidden in ["post_state", "result", "effect"] {
        assert!(recommendation.get(forbidden).is_none(), "{recommendation}");
    }
    assert_eq!(prefix[1]["transition"]["step"], "attempt");
    let semantic = |index: usize| &prefix[index]["post_state"]["state"]["semantic"];
    let field = |index: usize| {
        semantic(index)["entries"][0]["value"]["entries"][0]["value"]["fields"][0]["value"]["value"]
            .clone()
    };
    assert_eq!(field(0), "0", "recommendation leaves the supplied value");
    assert_eq!(field(1), "1", "Seller's attempt writes the model value");
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
