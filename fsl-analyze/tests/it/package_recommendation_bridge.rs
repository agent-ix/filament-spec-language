// SPDX-License-Identifier: AGPL-3.0-or-later
//! Package-native recommendation calls and their result-free constraints.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{analyze, package_selection_named, Workspace};

const SOURCE: &str = include_str!("../../../examples/protocol-recommendation/workflow.native");
const REQUEST: &str = include_str!("../../../examples/protocol-recommendation/analyze.json");
const SNAPSHOT: &[u8] = include_bytes!("../../../examples/protocol-recommendation/start.json");
const MODEL: &[u8] =
    include_bytes!("../../../fsl-command/tests/fixtures/cli-contract/counter.semantic-ir.json");

fn request(ws: &Workspace, source: &str) -> Value {
    ws.write("workflow.native", source.as_bytes());
    ws.write("start.json", SNAPSHOT);
    ws.write("counter.json", MODEL);
    let mut request: Value =
        serde_json::from_str(REQUEST).expect("the real recommendation request");
    request["request"]["models"][0]["source"]["file"] = json!("counter.json");
    request["request"]["items"][0]["kind"] = json!("temporal-satisfaction");
    request
}

fn source_and_package(ws: &Workspace, source: &str) -> (Value, Value) {
    let request = request(ws, source);
    let source = analyze(&ws.write("source.json", request.to_string().as_bytes()));
    assert_eq!(source.exit, 10, "Source: {}", source.document());
    let (members, _) = package_selection_named(ws, &request["request"], "recommendation");
    let package_request = json!({"format": "native-analyze/1", "request": members});
    let package = analyze(&ws.write("package.json", package_request.to_string().as_bytes()));
    assert_eq!(package.exit, 10, "Package: {}", package.document());
    (source.item(0), package.item(0))
}

/// The same selector executes admitted bytes, preserving canonical call
/// identity, model states and exact replay through the blocked check.
#[test]
#[trace("FR-940-AC-6", "FSpec-FR-822-AC-3")]
fn package_recommendations_preserve_native_steps_and_replay() {
    let ws = Workspace::new("recommendation-bridge-positive");
    let (source, package) = source_and_package(&ws, SOURCE);
    for item in [&source, &package] {
        assert_eq!(item["replay"]["reproduced"], true, "{item}");
        assert_eq!(item["counterexample"]["kind"], "deadlock", "{item}");
        let prefix = item["counterexample"]["prefix"]
            .as_array()
            .expect("native prefix");
        assert_eq!(prefix.len(), 2, "recommend then model write: {item}");
        assert_eq!(prefix[0]["transition"]["step"], "recommend", "{item}");
        assert_eq!(prefix[1]["transition"]["step"], "attempt", "{item}");
        let recommendation = &prefix[0]["transition"]["recommendation"];
        assert_eq!(recommendation["type"], "protocol-recommendation");
        assert_eq!(recommendation["node"], "Main/Advise");
        assert_eq!(recommendation["role"], "Auditor");
        assert_eq!(recommendation["arguments"], json!([]));
        let field = |position: usize| {
            &prefix[position]["post_state"]["state"]["semantic"]["entries"][0]["value"]["entries"]
                [0]["value"]["fields"][0]["value"]["value"]
        };
        assert_eq!(field(0), "0", "advice leaves the model unchanged: {item}");
        assert_eq!(field(1), "1", "the attempt writes the model: {item}");
        assert!(
            item["counterexample"]["blocked"]
                .to_string()
                .contains("Main/Blocked"),
            "{item}"
        );
    }
    assert_eq!(
        source["counterexample"]["prefix"],
        package["counterexample"]["prefix"]
    );
}

/// A package expression slot decides the recommendation constraint without
/// inventing a result binder or stepping the selected model operation.
#[test]
#[trace("FR-940-AC-6", "FSpec-FR-822-AC-3")]
fn a_false_package_recommendation_constraint_disables_the_call() {
    let ws = Workspace::new("recommendation-bridge-false");
    let source = SOURCE.replace(
        "as (advice: Boolean) { true }",
        "as (advice: Boolean) { false }",
    );
    let (source, package) = source_and_package(&ws, &source);
    for item in [&source, &package] {
        assert_eq!(item["replay"]["reproduced"], true, "{item}");
        assert_eq!(item["counterexample"]["prefix"], json!([]), "{item}");
        assert!(
            item["counterexample"]["blocked"]
                .to_string()
                .contains("Main/Advise"),
            "{item}"
        );
    }
}

/// A readable declaration with an unported control remains an unsupported
/// item, rather than a request-wide pending declaration family.
#[test]
#[trace("FR-940-AC-6")]
fn an_unported_package_control_names_its_typed_item_refusal() {
    let ws = Workspace::new("recommendation-bridge-unported");
    let source = SOURCE.replace(
        "run sequence Main {",
        "run sequence Main { fence Barrier ordering seq_cst;",
    );
    let request = request(&ws, &source);
    let (members, _) = package_selection_named(&ws, &request["request"], "recommendation");
    let request = json!({"format": "native-analyze/1", "request": members});
    let result = analyze(&ws.write("package.json", request.to_string().as_bytes()));
    let item = result.item(0);
    assert_eq!(item["category"], "unsupported", "{}", result.document());
    let diagnostic = &item["record"]["diagnostic"];
    assert_eq!(diagnostic["code"], "unsupported_construct", "{item}");
    assert_eq!(diagnostic["cause"], "not-yet-implemented", "{item}");
    assert_eq!(diagnostic["detail"]["fact"], "Fence", "{item}");
    assert!(item.get("counterexample").is_none(), "{item}");
}
