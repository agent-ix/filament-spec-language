// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1908 step 3 (FR-702-AC-3): a model's type invariants are evaluated by
//! `run` through FR-652's `value_invariants` selection of an `items`
//! request, and a false invariant is a violation naming the invariant and
//! the value's path. The model is TC-235 K2 with `Money`'s invariant
//! `NonNegative` (`self.amount_minor >= 0`) and an `orders` population.

use crate::command_harness::{run_file, Output};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

#[path = "../../fsl-semantics/tests/support/tc235_k2.rs"]
#[allow(dead_code)]
mod tc235_k2;

use tc235_k2::{clause, id, mutate_k2, origin, type_named_mut, BUSINESS, PACKAGE};

fn source(file: &str, document: &str) -> Value {
    json!({
        "file": file,
        "authority": "agent-ix", "identity": format!("test:fr-702:{file}"),
        "document": document, "formal_revision": 1
    })
}

fn model() -> Value {
    mutate_k2(|document| {
        type_named_mut(document, "Money")["clauses"] =
            json!([clause("Money", "NonNegative", "self.amount_minor >= 0")]);
        let order = type_named_mut(document, "Order");
        order["operations"] = json!([{
            "identity": id("Order/requote"), "name": "requote",
            "params": [{
                "identity": id("Order/requote/amount"), "name": "amount",
                "typeRef": id("Money"), "presence": "required", "nullable": false,
                "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": origin("Order/requote/amount"),
            }],
            "pre": [], "post": [],
            "frame": {"modifies": [], "creates": [], "deletes": []},
            "origin": origin("Order/requote"),
        }]);
        document["populations"] = json!([{
            "identity": id("orders"), "displayName": id("orders"),
            "kind": {"module": BUSINESS, "name": "population"},
            "members": [id("Order")], "extent": "closed",
            "origin": origin("orders"),
        }]);
    })
}

fn invocation_selection(directory: &Path, current_amount: i64, select_pre: bool) -> Output {
    std::fs::write(
        directory.join("model.semantic-ir.json"),
        model().to_string(),
    )
    .unwrap();
    std::fs::write(directory.join("unit.native"), UNIT).unwrap();
    let mut current = snapshot(current_amount);
    let mut pre = snapshot(current_amount);
    pre["identity"]["identity"] = json!("ix://test/fr-702/pre");
    pre["observation"] = json!("pre");
    let mut post = pre.clone();
    post["identity"]["identity"] = json!("ix://test/fr-702/post");
    post["observation"] = json!("post");
    current["identity"]["identity"] = json!("ix://test/fr-702/current");
    std::fs::write(directory.join("current.json"), current.to_string()).unwrap();
    std::fs::write(directory.join("pre.json"), pre.to_string()).unwrap();
    std::fs::write(directory.join("post.json"), post.to_string()).unwrap();
    std::fs::write(
        directory.join("invocation.json"),
        json!({
            "format": "filament.state.invocation/v1",
            "identity": {"authority": "agent-ix", "identity": "ix://test/fr-702/i1"},
            "model": {"identity": PACKAGE}, "context": id("Order"),
            "operation": "requote", "self": {"population": id("orders"), "key": "o1"},
            "pre": {"identity": {"authority": "agent-ix", "identity": "ix://test/fr-702/pre"}},
            "post": {"identity": {"authority": "agent-ix", "identity": "ix://test/fr-702/post"}},
            "parameters": {"amount": {"record": {"type": id("Money"), "fields": {
                "amount_minor": {"integer": "-7"},
                "currency": {"kind": "string", "text": "EUR"}
            }}}},
            "result": null, "created": [], "deleted": [],
        })
        .to_string(),
    )
    .unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0",
                    "source": source("model.semantic-ir.json", "K2")}],
        "program": {"source": source("unit.native", "Unit")},
        "items": {"observations": {
            "s1": if select_pre { "pre.json" } else { "current.json" },
            "p1": "pre.json", "q1": "post.json",
            "i1": "invocation.json"
        }, "selections": [
            {"value_invariants": {"snapshot": "s1", "invocation": "i1"}}
        ]},
    }});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    run_file(&directory.join("request.json"))
}

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.model.complete/v1\";\n\
    model M = \"test/orders\";\n\
    invariant Any using v on M::Order at current { true }\n";

/// A current snapshot holding one `Order` whose `total` has `amount_minor`
/// `amount`.
fn snapshot(amount: i64) -> Value {
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "agent-ix", "identity": "ix://test/fr-702/current"},
        "observation": "current",
        "anchor": {"kind": "handler", "name": "validate"},
        "model": {"identity": PACKAGE},
        "populations": [{"population": id("orders"), "complete": true, "objects": [{
            "key": "o1", "type": id("Order"),
            "fields": {
                "order_id": {"kind": "uuid", "text": "00112233-4455-6677-8899-aabbccddee01"},
                "total": {"record": {"type": id("Money"), "fields": {
                    "amount_minor": {"integer": amount.to_string()},
                    "currency": {"kind": "string", "text": "EUR"},
                }}},
            },
        }]}],
    })
}

/// Runs the `items` request selecting `value_invariants` over the snapshot
/// whose `total` is `amount`.
fn run(directory: &Path, amount: i64) -> Output {
    std::fs::write(
        directory.join("model.semantic-ir.json"),
        model().to_string(),
    )
    .unwrap();
    std::fs::write(directory.join("unit.native"), UNIT).unwrap();
    std::fs::write(
        directory.join("snapshot.json"),
        snapshot(amount).to_string(),
    )
    .unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0",
                    "source": source("model.semantic-ir.json", "K2")}],
        "program": {"source": source("unit.native", "Unit")},
        "items": {"observations": {"s1": "snapshot.json"}, "selections": [
            {"value_invariants": {"snapshot": "s1"}}
        ]},
    }});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    run_file(&directory.join("request.json"))
}

fn items_document(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "no items document: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// FR-702-AC-3: a negative `amount_minor` makes the `NonNegative` item a
/// violation naming the invariant and the path, and the document a
/// violation, exit 10; a non-negative one gives success, exit 0.
#[trace("TC-1908", "FR-702-AC-3")]
#[test]
fn a_false_type_invariant_is_a_violation_through_run() {
    let directory = tempfile::tempdir().unwrap();
    let output = run(directory.path(), -1);
    assert_eq!(output.status.code(), Some(10), "{output:?}");
    let document = items_document(&output);
    assert_eq!(document["category"], "violation", "{document}");
    let items = document["items"].as_array().expect("items");
    let violating: Vec<_> = items
        .iter()
        .filter(|item| item["result"]["disposition"]["category"] == "violation")
        .collect();
    assert_eq!(violating.len(), 1, "{document}");
    assert_eq!(
        violating[0]["invariant"]["node"],
        id("Money/NonNegative"),
        "{document}"
    );
    assert_eq!(violating[0]["invariant"]["package"], PACKAGE, "{document}");
    assert!(
        violating[0]["path"]["steps"]
            .as_array()
            .is_some_and(|steps| steps
                .iter()
                .any(|step| step["tag"] == "member" && step["name"] == "total")),
        "{document}"
    );

    let directory = tempfile::tempdir().unwrap();
    let output = run(directory.path(), 7);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(items_document(&output)["category"], "success");
}

/// FR-652-AC-3: invocation parameters are evaluated and located at the
/// invocation binding; an invariant violation settles the items document.
#[trace("TC-1716", "FR-652-AC-3", "FR-106-AC-48")]
#[test]
fn value_invariants_selection_checks_invocation_parameters() {
    let directory = tempfile::tempdir().unwrap();
    let output = invocation_selection(directory.path(), 7, false);
    assert_eq!(output.status.code(), Some(10), "{output:?}");
    let document = items_document(&output);
    assert_eq!(document["category"], "violation", "{document}");
    let parameter = document["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["path"].to_string().contains("amount"))
        .unwrap_or_else(|| panic!("parameter result missing: {document}"));
    assert_eq!(parameter["result"]["category"], "violation", "{parameter}");

    let directory = tempfile::tempdir().unwrap();
    let output = invocation_selection(directory.path(), 7, true);
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let document = items_document(&output);
    assert_eq!(document["items"].as_array().unwrap().len(), 1, "{document}");
    assert_eq!(document["category"], "refusal", "{document}");
    assert_eq!(
        document["items"][0]["result"]["disposition"]["refusal"]["code"], "wrong_snapshot",
        "{document}"
    );
    assert!(
        document["items"][0].get("invariant").is_none(),
        "{document}"
    );
    assert!(document["items"][0].get("path").is_none(), "{document}");
}
