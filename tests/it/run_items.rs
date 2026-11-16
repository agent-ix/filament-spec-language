// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-651 and FR-652: several selections, population expansions, contexts
//! and value invariants in one `run` request, and FR-650's Reference
//! argument over a call snapshot, through `command`'s `run`.
//!
//! The fixtures are FR-108's ConfigVersion spine files.

use crate::command_harness::{run_file, Output};
use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

fn source(file: &str, document: &str) -> Value {
    json!({
        "file": file,
        "authority": "agent-ix", "identity": format!("test:fr-651:{file}"),
        "document": document, "formal_revision": 1
    })
}

fn population() -> String {
    format!("ix://{}/config_history", spine::PACKAGE_IDENTITY)
}

/// Write the healthy case's spine files into `directory`, plus the
/// violating case's snapshot as `violating-snapshot.json`.
fn write_cases(directory: &Path) {
    spine::write(directory, Case::Healthy).unwrap();
    let violating = directory.join("violating");
    spine::write(&violating, Case::Violating).unwrap();
    std::fs::copy(
        violating.join("spine-snapshot.json"),
        directory.join("violating-snapshot.json"),
    )
    .unwrap();
}

/// The `ParentOrder` clause selection over observation `observation` with
/// `self` `self_value`.
fn parent_order(observation: &str, self_value: Value) -> Value {
    json!({"clause": {"name": "ParentOrder", "observation": observation,
        "self": self_value, "anchor": {"kind": "handler", "name": "validate"}}})
}

fn child() -> Value {
    json!({"population": population(), "key": "child"})
}

/// A native-run/1 request over the spine files carrying `body` as its
/// selection member (`call`, `clause` or `items`).
fn request(directory: &Path, member: &str, body: Value) -> Output {
    let mut inner = json!({
        "models": [{"format":"semantic-ir/2.0.0", "source":
            source("spine-model.semantic-ir.json", "SpineModel")}],
        "program": {"source": source("spine-unit.native", "SpineUnit")},
    });
    inner[member] = body;
    let request = json!({"format": "native-run/1", "request": inner});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    run_file(&directory.join("request.json"))
}

fn observations() -> Value {
    json!({"current": "spine-snapshot.json", "violating": "violating-snapshot.json"})
}

fn items_document(output: &Output) -> Value {
    let document: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "no items document: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(document["format"], "native-run-items/1", "{document}");
    document
}

fn assert_request_refusal(output: &Output, member: &str) {
    assert_eq!(output.status.code(), Some(20), "{member}: {output:?}");
    assert!(output.stdout.is_empty(), "{member}");
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "invalid-request", "{member}: {failure}");
    assert_eq!(failure["stage"], "request", "{member}: {failure}");
    assert!(failure.to_string().contains(member), "{member}: {failure}");
}

#[test]
#[trace("TC-1708", "FR-651-AC-1")]
#[ignore = "QSL-1987: items requests across several rules and snapshots not implemented yet"]
fn two_clause_selections_report_two_items() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", child()),
            parent_order("violating", child()),
        ]}),
    );
    assert_eq!(output.status.code(), Some(10), "{output:?}");
    let document = items_document(&output);
    assert_eq!(
        document["selections"],
        json!([{"index": 0, "expanded": 1}, {"index": 1, "expanded": 1}])
    );
    assert_eq!(document["items"][0]["result"]["category"], "success");
    assert_eq!(document["items"][1]["result"]["category"], "violation");
    assert_eq!(document["category"], "violation");
}

#[test]
#[trace("TC-1709", "FR-651-AC-2")]
#[ignore = "QSL-1987: population selector expansion not implemented yet"]
fn population_selector_expands_one_item_per_object() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", json!({"each": {"population": population()}})),
        ]}),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document = items_document(&output);
    assert_eq!(document["selections"], json!([{"index": 0, "expanded": 2}]));
    assert_eq!(document["items"][0]["object"]["key"], "root");
    assert_eq!(document["items"][1]["object"]["key"], "child");
}

/// Negative: one refused item leaves the others unchanged.
#[test]
#[trace("TC-1710", "FR-651-AC-3")]
#[ignore = "QSL-1987: per-item settlement of an items request not implemented yet"]
fn one_refused_item_leaves_the_others_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", child()),
            parent_order("current", json!({"population": population(), "key": "ghost"})),
            parent_order("violating", child()),
        ]}),
    );
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let document = items_document(&output);
    assert_eq!(document["items"][0]["result"]["category"], "success");
    assert_eq!(document["items"][1]["result"]["category"], "refusal");
    assert_eq!(document["items"][2]["result"]["category"], "violation");
    assert_eq!(document["category"], "refusal");
}

/// Negative: malformed items requests refuse at the request stage.
#[test]
#[trace("TC-1712", "FR-651-AC-5")]
#[ignore = "QSL-1987: items request decoding not implemented yet"]
fn malformed_items_requests_refuse_at_the_request_stage() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let each = json!({"each": {"population": population()}});
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": []}),
    );
    assert_request_refusal(&output, "selections");
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            {"clause": parent_order("current", child())["clause"].clone(),
             "frame": {"operation": "x", "invocation": "current"}}
        ]}),
    );
    assert_request_refusal(&output, "selections");
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": observations(), "selection": parent_order("current", each)}),
    );
    assert_request_refusal(&output, "each");
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("missing", child()),
        ]}),
    );
    assert_request_refusal(&output, "missing");
}

#[test]
#[trace("TC-1713", "FR-651-AC-6")]
#[ignore = "QSL-1987: unheld population selector item not implemented yet"]
fn unheld_population_selector_gives_one_refused_item() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", json!({"each": {"population": "ix://example/config-version/nowhere"}})),
            parent_order("current", child()),
        ]}),
    );
    let document = items_document(&output);
    assert_eq!(document["items"][0]["result"]["category"], "refusal");
    assert!(document["items"][0].get("object").is_none());
    assert_eq!(document["items"][1]["result"]["category"], "success");
}

/// Negative: FR-652 selections with malformed members refuse.
#[test]
#[trace("TC-1715", "FR-652-AC-2")]
#[ignore = "QSL-2193: context and value_invariants request members not implemented yet"]
fn malformed_context_and_value_invariant_selections_refuse() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": observations(), "selection":
            {"context": {"name": "ParentOrder", "invocation": "current", "side": "mid"}}}),
    );
    assert_request_refusal(&output, "side");
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": observations(), "selection":
            {"value_invariants": {"snapshot": "current"}}}),
    );
    assert_request_refusal(&output, "value_invariants");
}

/// FR-652-AC-3: a value_invariants selection is reachable from an items
/// request and lists one item per evaluated invariant and value.
#[test]
#[trace("TC-1716", "FR-652-AC-3")]
#[ignore = "QSL-2193: value_invariants selection not implemented yet"]
fn value_invariants_selection_is_an_items_selection() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            {"value_invariants": {"snapshot": "current"}}
        ]}),
    );
    let document = items_document(&output);
    for item in document["items"].as_array().unwrap() {
        assert!(item.get("invariant").is_some(), "{item}");
        assert!(item.get("path").is_some(), "{item}");
    }
}

/// Negative: FR-650-AC-4's Reference argument with no call snapshot.
#[test]
#[trace("TC-1703", "FR-650-AC-4")]
#[ignore = "QSL-1988: Reference arguments over a call snapshot not implemented yet"]
fn reference_argument_without_a_snapshot_refuses() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    extend_unit(directory.path(), SAME);
    let call = |snapshot: Option<&str>| {
        let mut call = json!({"function": "same", "arguments": [
            {"parameter": "r", "value": {"reference": child()}}]});
        if let Some(snapshot) = snapshot {
            call["snapshot"] = json!(snapshot);
        }
        call
    };
    let output = request(directory.path(), "call", call(Some("spine-snapshot.json")));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(document["outcome"]["value"]["reference"]["object_identity"].is_string());
    let output = request(directory.path(), "call", call(None));
    assert_eq!(output.status.code(), Some(20), "{output:?}");
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "invalid_runtime_input", "{failure}");
    assert_eq!(failure["cause"], "missing-member", "{failure}");
    assert_eq!(failure["stage"], "call", "{failure}");
    assert_eq!(
        failure["details"],
        json!({"position": 0, "path": "/call/snapshot"}),
        "{failure}"
    );
}

/// Append `functions` to the written spine unit.
fn extend_unit(directory: &Path, functions: &str) {
    let unit = std::fs::read_to_string(directory.join("spine-unit.native")).unwrap();
    std::fs::write(
        directory.join("spine-unit.native"),
        format!("{unit}{functions}"),
    )
    .unwrap();
}

const SAME: &str = "function same using v(r: Config::ConfigVersion): \
                    Config::ConfigVersion pure { r }\n";

/// Call `same` with `value` over the healthy snapshot and return the
/// completed value.
fn same_of(directory: &Path, value: Value) -> Value {
    let output = request(
        directory,
        "call",
        json!({"function": "same", "snapshot": "spine-snapshot.json",
            "arguments": [{"parameter": "r", "value": value}]}),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["outcome"]["kind"], "completed", "{document}");
    document["outcome"]["value"].clone()
}

/// FR-650-AC-5: a Reference result is its identity triple and passes back
/// unchanged as an argument.
#[test]
#[trace("TC-1704", "FR-650-AC-5")]
#[ignore = "QSL-2182: structural identity spelling of a Reference result and call.snapshot reference arguments not implemented yet"]
fn reference_result_passes_back_as_an_argument() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    extend_unit(directory.path(), SAME);
    let first = same_of(directory.path(), json!({"reference": child()}));
    let triple = &first["reference"];
    assert!(triple["universe"]["package"].is_string(), "{first}");
    assert!(triple["universe"]["path"].is_array(), "{first}");
    assert!(triple["universe"]["ordinal"].is_u64(), "{first}");
    assert!(triple["type"]["original"]["node"].is_string(), "{first}");
    assert!(triple["object_identity"].is_string(), "{first}");
    assert_eq!(same_of(directory.path(), first.clone()), first);
}

/// FR-650-AC-7: an FR-312 `function` selection admits the run value form.
#[test]
#[trace("TC-1706", "FR-650-AC-7")]
#[ignore = "QSL-1988: run value form arguments in a clause-run function selection not implemented yet"]
fn function_selection_admits_an_identity_reference() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    extend_unit(directory.path(), SAME);
    let identity = same_of(directory.path(), json!({"reference": child()}));
    let output = request(
        directory.path(),
        "clause",
        json!({"observations": observations(), "selection": {"function": {
            "name": "sameIdentity", "snapshot": "current", "arguments": [
                {"parameter": "a", "value": identity},
                {"parameter": "b", "value": {"reference": child()}}]}}}),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["disposition"]["truth"], true, "{document}");
}

/// FR-651-AC-4: the most severe item decides the exit, and an item's budget
/// is its own.
#[test]
#[trace("TC-1711", "FR-651-AC-4")]
#[ignore = "QSL-1987: items severity order and per-item budgets not implemented yet"]
fn items_take_the_most_severe_exit_and_their_own_budgets() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    extend_unit(
        directory.path(),
        "function seven using v(): Int[0, 9] pure { 7 }\n",
    );
    let seven = json!({"call": {"function": "seven", "arguments": []}});
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "accounting": {"work_units": 0},
            "selections": [parent_order("violating", child()), seven.clone()]}),
    );
    assert_eq!(output.status.code(), Some(22), "{output:?}");
    let document = items_document(&output);
    assert_eq!(
        document["items"][1]["result"]["outcome"]["kind"],
        "incomplete"
    );
    assert_eq!(document["category"], "incomplete");
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "accounting": {"work_units": 1000},
            "selections": [seven.clone(), seven]}),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document = items_document(&output);
    for item in document["items"].as_array().unwrap() {
        assert_eq!(item["result"]["outcome"]["kind"], "completed", "{item}");
    }
}

/// FR-652-AC-1: a `context` selection is read and reaches FR-106 admission,
/// which refuses an object invariant selected by `Context` (FR-106-AC-46).
#[test]
#[trace("TC-1714", "FR-652-AC-1")]
#[ignore = "QSL-2193: context selection in a run request not implemented yet"]
fn context_selection_reaches_admission() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::ForbiddenParent).unwrap();
    for side in ["pre", "post"] {
        let output = request(
            directory.path(),
            "clause",
            json!({"observations": {"i": "spine-invocation.json"}, "selection":
                {"context": {"name": "ParentOrder", "invocation": "i", "side": side}}}),
        );
        assert_eq!(output.status.code(), Some(20), "{side}: {output:?}");
        let document: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            panic!(
                "{side}: no clause-run document: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        assert_eq!(document["format"], "native-run-result/2", "{document}");
        assert_eq!(document["disposition"]["stage"], "admit", "{document}");
        assert!(
            document.to_string().contains("wrong-observation"),
            "{document}"
        );
    }
}

/// FR-106-AC-48: `ValueInvariants` admits one current snapshot, reachable
/// through FR-652's `value_invariants` selection.
#[test]
#[trace("TC-1051", "FR-106-AC-48")]
#[ignore = "QSL-2193: ValueInvariants admission from a run request not implemented yet"]
fn value_invariants_admits_a_current_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            {"value_invariants": {"snapshot": "current"}}]}),
    );
    let document = items_document(&output);
    assert_ne!(document["category"], "refusal", "{document}");
}

/// FSpec FR-550-AC-3: items settle independently and the overall category
/// is the most severe.
#[test]
#[trace("FSpec-TC-1202", "FSpec-FR-550-AC-3")]
#[ignore = "QSL-1987: per-item settlement of an items request not implemented yet"]
fn fspec_items_settle_independently() {
    let directory = tempfile::tempdir().unwrap();
    write_cases(directory.path());
    let output = request(
        directory.path(),
        "items",
        json!({"observations": observations(), "selections": [
            parent_order("current", child()),
            parent_order("current", json!({"population": population(), "key": "ghost"})),
            parent_order("current", child()),
        ]}),
    );
    let document = items_document(&output);
    assert_eq!(
        document["items"][0]["result"],
        document["items"][2]["result"]
    );
    assert_eq!(document["items"][1]["result"]["category"], "refusal");
    assert_eq!(document["category"], "refusal");
}
