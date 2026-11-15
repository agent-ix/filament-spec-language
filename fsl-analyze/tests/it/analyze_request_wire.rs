// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-620, FR-621, FR-622, FR-281-AC-23 (TC-1580, TC-1581, TC-1582): the
//! `native-analyze/1` request document is read, its items resolve by name and
//! round-trip through the echoed FSpec FR-331 request item, and the outcome
//! carries the FR-621 item members.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, analyze_as, below_three, bounded, counter_request, counter_request_with, label,
    request_members, Workspace,
};

/// FR-620-AC-1, AC-2: the admitted item is echoed as its FR-331 request item,
/// and that echo, written back as a document item, reads to an equal item.
#[trace("TC-1580", "FR-620-AC-1")]
#[trace("TC-1580", "FR-620-AC-2")]
#[trace("TC-1080", "FSpec-FR-520-AC-1")]
#[test]
#[ignore = "QSL-2121: the native-analyze/1 reader and the echoed request item are not implemented yet"]
fn an_admitted_item_echoes_its_request_and_round_trips() {
    let ws = Workspace::new("wire-echo");
    let path = counter_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let first = analyze(&path);
    let request = first.item(0)["request"].clone();
    assert_eq!(request["request_index"], 0);
    assert_eq!(request["kind"], "temporal-satisfaction");
    assert_eq!(request["temporal"]["item_kind"], "formula");
    assert!(request["claim"]["node"].is_object(), "{request}");
    assert!(
        request["temporal"]["limits"]["max_states"].is_string()
            || request["temporal"]["limits"]["max_states"].is_number()
    );

    let mut item = request.clone();
    let object = item.as_object_mut().expect("an item object");
    object.remove("request_index");
    object.remove("extent");
    object.remove("candidates");
    object.insert("claim".to_owned(), json!("Bounded"));
    if let Some(temporal) = object.get_mut("temporal").and_then(Value::as_object_mut) {
        temporal.remove("fairness");
    }
    let ws2 = Workspace::new("wire-echo-again");
    let again = analyze(&counter_request(&ws2, &bounded(), json!([item])));
    assert_eq!(again.item(0)["request"], request);
}

/// FR-620-AC-3: computed members and unknown members refuse the whole request.
#[trace("TC-1580", "FR-620-AC-3")]
#[trace("TC-1080", "FSpec-FR-520-AC-2")]
#[test]
#[ignore = "QSL-2121: the closed native-analyze/1 reader is not implemented yet"]
fn computed_and_unknown_members_refuse_the_request() {
    let forged = [
        json!({"claim": "Bounded", "request_index": 0}),
        json!({"claim": "Bounded", "extent": {"classification": "bounded", "bounds": []}}),
        json!({"claim": "Bounded", "candidates": {"type": "set", "backends": ["en-1"]}}),
        json!({"claim": "Bounded", "temporal": {"fairness": []}}),
        json!({"claim": "Bounded", "colour": "blue"}),
    ];
    for (n, item) in forged.into_iter().enumerate() {
        let ws = Workspace::new(&format!("wire-forged-{n}"));
        let run = analyze(&counter_request(&ws, &bounded(), json!([item])));
        assert_eq!(run.exit, 20, "{item}");
        let document = run.document();
        assert_eq!(document["category"], "refusal");
        assert_eq!(document["items"], json!([]));
        assert_eq!(document["diagnostics"][0]["code"], "invalid-request");
    }
    let ws = Workspace::new("wire-unknown-request-member");
    let mut request = request_members(Some(json!([{"claim": "Bounded"}])));
    request["shade"] = json!(1);
    let run = analyze(&counter_request_with(&ws, &bounded(), request));
    assert_eq!(run.exit, 20);
    assert_eq!(run.document()["diagnostics"][0]["code"], "invalid-request");
}

/// FR-620-AC-4: an unknown claim settles only its own item.
#[trace("TC-1580", "FR-620-AC-4")]
#[test]
#[ignore = "QSL-2121: per-item claim resolution by name is not implemented yet"]
fn an_unknown_claim_settles_only_its_item() {
    let ws = Workspace::new("wire-unknown-claim");
    let run = analyze(&counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded"}, {"claim": "Missing"}]),
    ));
    assert_eq!(run.exit, 20);
    assert_eq!(label(&run.item(0)), "proved");
    let missing = run.item(1);
    assert_eq!(missing["category"], "refusal");
    assert_eq!(
        missing["record"]["diagnostic"]["code"],
        "invalid_capability"
    );
    assert_eq!(missing["record"]["diagnostic"]["cause"], "unknown-claim");
}

/// FR-620-AC-5: with no `items`, every claim then the derived items.
#[trace("TC-1580", "FR-620-AC-5")]
#[test]
#[ignore = "QSL-2121: default items are not implemented yet"]
fn absent_items_analyze_every_claim_then_the_derived_items() {
    let ws = Workspace::new("wire-default-items");
    let body = format!("{}{}", bounded(), below_three());
    let run = analyze(&counter_request_with(&ws, &body, request_members(None)));
    let names: Vec<Value> = run.document()["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| item["claim_name"].clone())
        .collect();
    assert_eq!(names[..2], [json!("Bounded"), json!("BelowThree")]);
    assert_eq!(
        run.item(2)["request"]["temporal"]["item_kind"],
        "deadlock-freedom"
    );
}

/// FR-620-AC-6, AC-7: deadline and per-item limits.
#[trace("TC-1580", "FR-620-AC-6")]
#[trace("TC-1580", "FR-620-AC-7")]
#[test]
#[ignore = "QSL-2121: request deadlines and per-item limits are not implemented yet"]
fn the_deadline_and_item_limits_settle_incomplete() {
    let ws = Workspace::new("wire-deadline");
    let mut request = request_members(Some(json!([{"claim": "Bounded"}])));
    request["deadline_ms"] = json!(0);
    let run = analyze(&counter_request_with(&ws, &bounded(), request));
    assert_eq!(run.exit, 22);
    assert_eq!(label(&run.item(0)), "incomplete");

    let ws = Workspace::new("wire-limits");
    let run = analyze(&counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded"},
            {"claim": "Bounded", "temporal": {"limits": {"max_states": 1}}}]),
    ));
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(label(&run.item(1)), "incomplete");
    assert_eq!(run.item(1)["record"]["cause"]["limit"], "max_states");
}

/// FR-620-AC8, FR-281-AC-23: EN-7 is in the registry.
#[trace("TC-1580", "FR-620-AC-8")]
#[trace("TC-1580", "FR-281-AC-23")]
#[test]
#[ignore = "QSL-2131: EN-7 is not registered in Engines::fsl() yet"]
fn the_fsl_registry_holds_en7() {
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the layer-A engines register");
    assert!(
        engines
            .registrations()
            .any(|descriptor| descriptor.id().as_str() == "en-7"),
        "Engines::fsl() registers en-7"
    );
}

/// FR-620-AC-9: each reader refuses the other's format.
#[trace("TC-1580", "FR-620-AC-9")]
#[test]
#[ignore = "QSL-2121: the analyze reader's format check is not implemented yet"]
fn a_run_request_handed_to_analyze_refuses() {
    let ws = Workspace::new("wire-format");
    let path = counter_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("the request")).expect("json");
    document["format"] = json!("native-run/1");
    let wrong = ws.write("wrong.json", document.to_string().as_bytes());
    let run = analyze(&wrong);
    assert_eq!(run.exit, 20);
    assert_eq!(run.document()["diagnostics"][0]["code"], "invalid-request");
}

/// FR-621-AC-1 to AC-3: proved, refuted and incomplete item members.
#[trace("TC-1581", "FR-621-AC-1")]
#[trace("TC-1581", "FR-621-AC-2")]
#[trace("TC-1581", "FR-621-AC-3")]
#[trace("TC-1080", "FSpec-FR-520-AC-3")]
#[test]
#[ignore = "QSL-2121: the analyze outcome item members are not implemented yet"]
fn outcome_items_carry_verdict_certificate_counterexample_and_budget_use() {
    let ws = Workspace::new("result-members");
    let body = format!("{}{}", bounded(), below_three());
    let run = analyze(&counter_request(
        &ws,
        &body,
        json!([{"claim": "Bounded"}, {"claim": "BelowThree"},
            {"claim": "Bounded", "temporal": {"limits": {"max_states": 2}}}]),
    ));
    let proved = run.item(0);
    assert_eq!(label(&proved), "proved");
    assert_eq!(proved["record"]["certification"], "certified");
    assert_eq!(proved["certificate"]["accepted"], true);
    assert_eq!(proved["exploration"]["states"], "4");
    assert_eq!(proved["exploration"]["transitions"], "3");
    let refuted = run.item(1);
    assert_eq!(label(&refuted), "refuted");
    assert!(refuted["counterexample"].is_object());
    assert_eq!(refuted["replay"]["reproduced"], true);
    let stopped = run.item(2);
    assert_eq!(label(&stopped), "incomplete");
    assert_eq!(stopped["exploration"]["states"], "2");
    assert!(stopped.get("counterexample").is_none());
    assert_eq!(run.document()["aggregate"]["status"], "incomplete");
    assert_eq!(run.exit, 10);
}

/// FR-621-AC-6: text mode prints one line per item.
#[trace("TC-1581", "FR-621-AC-6")]
#[test]
#[ignore = "QSL-2121: the analyze text rendering is not implemented yet"]
fn text_mode_prints_one_line_per_item() {
    let ws = Workspace::new("result-text");
    let run = analyze_as(
        &counter_request(&ws, &below_three(), json!([{"claim": "BelowThree"}])),
        "text",
    );
    assert_eq!(run.exit, 10);
    let text = String::from_utf8_lossy(&run.stdout);
    assert!(
        text.lines()
            .next()
            .is_some_and(|line| line.starts_with("0 BelowThree refuted")),
        "{text}"
    );
}

/// FR-622-AC-3: analyze items carry the mapped axes.
#[trace("TC-1582", "FR-622-AC-3")]
#[trace("TC-1081", "FSpec-FR-521-AC-2")]
#[test]
#[ignore = "QSL-2121: outcome axes on analyze items are not implemented yet"]
fn analyze_items_carry_the_mapped_axes() {
    let ws = Workspace::new("axes");
    let body = format!("{}{}", bounded(), below_three());
    let run = analyze(&counter_request(
        &ws,
        &body,
        json!([{"claim": "Bounded"}, {"claim": "BelowThree"}]),
    ));
    assert_eq!(
        run.item(0)["axes"],
        json!({"activation": "active", "execution": "completed", "truth": "true",
            "settlement": "closed-scope"})
    );
    assert_eq!(
        run.item(1)["axes"],
        json!({"activation": "active", "execution": "completed", "truth": "false",
            "settlement": "decisive-counterexample"})
    );
}
