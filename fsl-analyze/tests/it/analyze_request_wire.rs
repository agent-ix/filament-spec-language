// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-620, FR-621, FR-622, FR-281-AC-23 (TC-1580, TC-1581, TC-1582): the
//! `native-analyze/1` request document is read, its items resolve by name and
//! round-trip through the echoed FSpec FR-331 request item, and the outcome
//! carries the FR-621 item members.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, below_three, bounded, counter_request, counter_request_with, label, request_members,
    Workspace,
};

/// FR-620-AC-1, AC-2: the admitted item is echoed as its FR-331 request item,
/// and that echo, written back as a document item, reads to an equal item.
#[trace("TC-1580", "FR-620-AC-1")]
#[trace("TC-1580", "FR-620-AC-2")]
#[trace("TC-1080", "FSpec-FR-520-AC-1")]
#[test]
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
#[ignore = "QSL-2130: EN-7 is not registered in Engines::fsl() yet"]
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
    // FR-285-AC-2: items with codes {0, 10, 22} exit 22, the most severe.
    assert_eq!(run.exit, 22);
}

/// FR-621-AC-6: each item carries the members the driver's text line is
/// rendered from (`request_index`, `claim_name`, `record.result`), so text
/// mode reads no other source. The driver tests the rendered line.
#[trace("TC-1581", "FR-621-AC-6")]
#[test]
fn items_carry_the_members_text_mode_renders() {
    let ws = Workspace::new("result-text");
    let run = analyze(&counter_request(
        &ws,
        &below_three(),
        json!([{"claim": "BelowThree"}]),
    ));
    assert_eq!(run.exit, 10);
    let item = run.item(0);
    assert_eq!(item["request_index"], 0, "{item}");
    assert_eq!(item["claim_name"], "BelowThree");
    assert_eq!(label(&item), "refuted");
}

/// FR-622-AC-3: analyze items carry the mapped axes.
#[trace("TC-1582", "FR-622-AC-3")]
#[trace("TC-1081", "FSpec-FR-521-AC-2")]
#[test]
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

/// FR-621-AC-8 (TC-2184): a proved EN-1 item names its certificate by FSpec
/// FR-412's structural identity, a refuted item names its FR-323 replay
/// target, and an unproved required item is an aggregate gap `not-proved`.
#[trace("TC-2184", "FR-621-AC-8")]
#[test]
fn analyze_items_name_certificate_replay_target_and_gaps() {
    let ws = Workspace::new("wire-identity");
    let body = format!("{}{}", bounded(), below_three());
    let path = counter_request(
        &ws,
        &body,
        json!([{"claim": "Bounded", "required": true}, {"claim": "BelowThree", "required": true}]),
    );
    let run = analyze(&path);
    let proved = run.item(0);
    assert_eq!(label(&proved), "proved", "{proved}");
    let identity = proved["certificate"]["identity"]
        .as_object()
        .expect("a certificate identity object");
    let mut members: Vec<&str> = identity.keys().map(String::as_str).collect();
    members.sort_unstable();
    assert_eq!(members, ["model", "node_id", "ordinal"]);
    assert_eq!(identity["model"], "test/counter", "{proved}");
    assert_eq!(identity["node_id"], proved["request"]["claim"]["node"]);
    assert_eq!(
        identity["ordinal"],
        proved["request"]["claim"]["occurrence"]["ordinal"]
    );
    let refuted = run.item(1);
    assert_eq!(label(&refuted), "refuted", "{refuted}");
    let target = refuted["replay"]["target"]["state_clause"]
        .as_str()
        .expect("a state_clause target");
    assert!(target.ends_with("BelowThree"), "{target}");
    let aggregate = &run.document()["aggregate"];
    assert_eq!(aggregate["status"], "incomplete");
    assert_eq!(
        aggregate["gaps"],
        json!([{"request_index": 1, "reason": "not-proved", "result": "refuted"}])
    );
}

/// The message of a whole-request refusal's first diagnostic.
fn refusal_message(run: &crate::analyze_cli_support::Run) -> String {
    let document = run.document();
    assert_eq!(
        document["diagnostics"][0]["code"], "invalid-request",
        "{document}"
    );
    document["diagnostics"][0]["message"]
        .as_str()
        .expect("a diagnostic message")
        .to_owned()
}

/// FR-620-AC-10 (TC-2185): default items follow source order, an
/// unprovisioned universe refuses, and normalization limits take their
/// setting names.
#[trace("TC-2185", "FR-620-AC-10")]
#[test]
fn analyze_defaults_universes_and_limit_names() {
    let ws = Workspace::new("wire-defaults");
    let body = format!("{}{}", below_three(), bounded());
    let run = analyze(&counter_request_with(&ws, &body, request_members(None)));
    let names: Vec<Value> = run.document()["items"]
        .as_array()
        .expect("items")
        .iter()
        .take(2)
        .map(|item| item["claim_name"].clone())
        .collect();
    assert_eq!(names, [json!("BelowThree"), json!("Bounded")]);
    assert_eq!(
        run.item(2)["request"]["temporal"]["item_kind"],
        "deadlock-freedom"
    );

    let start = json!({"authority": "test", "identity": "start"});
    let mut universes = request_members(Some(json!([{"claim": "Bounded"}])));
    universes["subject"] = json!({"initial_states": [start], "universes": [start],
        "proof_bounds": []});
    let ws = Workspace::new("wire-universes-provisioned");
    let provisioned = analyze(&counter_request_with(&ws, &bounded(), universes.clone()));
    assert_eq!(
        label(&provisioned.item(0)),
        "proved",
        "{}",
        provisioned.document()
    );
    universes["subject"]["universes"] = json!([{"authority": "test", "identity": "nowhere"}]);
    let ws = Workspace::new("wire-universes");
    let run = analyze(&counter_request_with(&ws, &bounded(), universes));
    assert_eq!(run.exit, 20);
    let message = refusal_message(&run);
    assert!(message.contains("subject.universes[0]"), "{message}");
    assert!(message.contains("nowhere"), "{message}");

    for (normalization, exit) in [
        (json!({"work_units": 5_000_000}), 0),
        (json!({"bogus": 1}), 20),
    ] {
        let mut request = request_members(Some(json!([{"claim": "Bounded"}])));
        request["limits"] = json!({"normalization": normalization});
        let ws = Workspace::new("wire-normalization");
        let run = analyze(&counter_request_with(&ws, &bounded(), request));
        assert_eq!(run.exit, exit, "{normalization}");
        if exit == 20 {
            let message = refusal_message(&run);
            assert!(message.contains("bogus"), "{message}");
        }
    }
}

/// FR-767-AC-9 (TC-2196): a `native-analyze/1` request's `integer_division`
/// member supplies the law its program's `div` needs.
#[trace("TC-2196", "FR-767-AC-9", "FSpec-TC-1654", "FSpec-FR-520-AC-5")]
#[test]
#[ignore = "QSL-1989: the native-analyze/1 integer_division member is not implemented yet"]
fn an_analyze_request_supplies_the_integer_division_law() {
    let body = format!(
        "function half using v(a: Int[0, 9]): Int[0, 9] pure {{ a div 2 }}\n{}",
        bounded()
    );
    let with_law = |law: Option<Value>| {
        let mut request = request_members(Some(json!([{"claim": "Bounded"}])));
        if let Some(law) = law {
            request["integer_division"] = law;
        }
        request
    };

    let ws = Workspace::new("division-law-floor");
    let run = analyze(&counter_request_with(
        &ws,
        &body,
        with_law(Some(json!("floor"))),
    ));
    assert_ne!(run.exit, 20, "{}", run.document());
    assert!(run.item(0)["request"].is_object(), "{}", run.document());

    let ws = Workspace::new("division-law-absent");
    let run = analyze(&counter_request_with(&ws, &body, with_law(None)));
    assert_eq!(run.exit, 20);
    let document = run.document();
    assert_eq!(document["items"], json!([]), "{document}");
    assert_eq!(document["diagnostics"][0]["code"], "missing_declaration");
    assert_eq!(document["diagnostics"][0]["cause"], "missing-selection");

    let ws = Workspace::new("division-law-unknown");
    let run = analyze(&counter_request_with(
        &ws,
        &body,
        with_law(Some(json!("ceiling"))),
    ));
    assert_eq!(run.exit, 20);
    let document = run.document();
    assert_eq!(document["diagnostics"][0]["code"], "unknown_profile");
    assert_eq!(document["diagnostics"][0]["cause"], "unsupported-selection");
}

/// FR-620-AC-11 (TC-2215): `hashed_bytes` bounds no work in FSL, so a
/// `limits.normalization` member of that name refuses the whole request.
#[trace("TC-2215", "FR-620-AC-11")]
#[test]
#[ignore = "QSL-2269: the analyze request still accepts limits.normalization.hashed_bytes"]
fn hashed_bytes_is_not_a_normalization_limit() {
    for (n, (normalization, exit)) in [
        (json!({"hashed_bytes": 1}), 20),
        (json!({"hashed_bytes": 1, "work_units": 5_000_000}), 20),
        (json!({"work_units": 5_000_000}), 0),
    ]
    .into_iter()
    .enumerate()
    {
        let mut request = request_members(Some(json!([{"claim": "Bounded"}])));
        request["limits"] = json!({"normalization": normalization});
        let ws = Workspace::new(&format!("wire-hashed-bytes-{n}"));
        let run = analyze(&counter_request_with(&ws, &bounded(), request));
        assert_eq!(run.exit, exit, "{normalization}");
        if exit == 20 {
            let document = run.document();
            assert_eq!(document["items"], json!([]), "{document}");
            let diagnostic = &document["diagnostics"][0];
            assert_eq!(diagnostic["code"], "invalid-request", "{document}");
            assert!(
                diagnostic.to_string().contains("hashed_bytes"),
                "{document}"
            );
        }
    }
}

/// FR-620 "Subject, compilation and default items": the units rank in
/// request order, the program first and then each library as listed, so a
/// library whose labels sort before the program's moves no program claim.
/// A library's declarations live in its own checked package, so none of
/// them is a default item of the program.
#[trace("TC-2185", "FR-620-AC-10")]
#[test]
fn default_items_rank_units_in_request_order() {
    let ws = Workspace::new("wire-default-library");
    ws.write(
        "library.fsl",
        b"language \"ix:native\" edition \"1-draft\";\n\
          profile v = \"filament.value.complete/v1\";\n\
          function Early using v(): Boolean pure { true }\n",
    );
    let mut request = request_members(None);
    request["libraries"] = json!([{"identity": "aaa/library", "source": {"file": "library.fsl",
        "authority": "aaa", "identity": "aaa-library", "document": "Library",
        "formal_revision": 1}}]);
    let body = format!("{}{}", below_three(), bounded());
    let run = analyze(&counter_request_with(&ws, &body, request));
    let document = run.document();
    let names: Vec<Value> = document["items"]
        .as_array()
        .unwrap_or_else(|| panic!("items in {document}"))
        .iter()
        .map(|item| item["claim_name"].clone())
        .collect();
    assert_eq!(
        names[..2],
        [json!("BelowThree"), json!("Bounded")],
        "{document}"
    );
    assert!(!names.contains(&json!("Early")), "{document}");
}

/// FR-620: the default subject's universes are the complete populations
/// its snapshots declare, so the default binding leaves an incomplete
/// population out and the reader refuses no universe for it; an explicit
/// `subject.universes` naming that snapshot refuses `incomplete_population`
/// at the request stage, exit 22.
#[trace("TC-2185", "FR-620-AC-10")]
#[test]
fn only_an_explicit_universe_refuses_an_incomplete_population() {
    use fsl_analyze::analyze::document::{SubjectSelection, UniverseRefusal};
    let mut snapshot: Value =
        serde_json::from_slice(&crate::analyze_cli_support::counter_snapshot(&[("a", 0)]))
            .expect("the snapshot is JSON");
    snapshot["populations"][0]["complete"] = json!(false);
    let bytes = snapshot.to_string().into_bytes();
    let mut documents = fsl_semantics::model::observation::DocumentProvision::new();
    let reference = fsl_semantics::model::observation::document_reference(&bytes)
        .expect("the snapshot declares its identity");
    documents.insert(reference, bytes.clone());
    let default = SubjectSelection::default_over(&documents);
    assert_eq!(default.initial_states.len(), 1);
    let packages = fsl_semantics::model::intake::PackageInput::default();
    assert!(default.binding(&packages, &documents).universes.is_empty());

    let start = json!({"authority": "test", "identity": "start"});
    let mut request = request_members(Some(json!([{"claim": "Bounded"}])));
    request["subject"] = json!({"initial_states": [start], "universes": [start],
        "proof_bounds": []});
    let ws = Workspace::new("wire-incomplete-explicit");
    let path = counter_request_with(&ws, &bounded(), request);
    ws.write("start.json", &bytes);
    let run = analyze(&path);
    let document = run.document();
    assert_eq!(
        document["diagnostics"][0]["code"], "incomplete_population",
        "{document}"
    );
    assert_eq!(
        document["diagnostics"][0]["message"],
        UniverseRefusal::Incomplete {
            population: "ix://test/counter/counters".to_owned()
        }
        .to_string(),
        "{document}"
    );
    assert_eq!(run.exit, 22, "{document}");
}

/// FR-620 "Reading": a `temporal.limits` or `temporal.max_depth` count past
/// 2^53 - 1 refuses the whole request naming the member, never a dropped
/// bound.
#[trace("TC-1580", "FR-620-AC-3")]
#[test]
fn an_oversized_temporal_count_refuses_naming_the_member() {
    for (temporal, member) in [
        (
            json!({"limits": {"max_states": 9_007_199_254_740_992_u64}}),
            "temporal.limits.max_states",
        ),
        (
            json!({"max_depth": 9_007_199_254_740_992_u64}),
            "temporal.max_depth",
        ),
    ] {
        let ws = Workspace::new("wire-oversized-count");
        let run = analyze(&counter_request(
            &ws,
            &bounded(),
            json!([{"claim": "Bounded", "temporal": temporal}]),
        ));
        assert_eq!(run.exit, 20, "{member}");
        let message = refusal_message(&run);
        assert!(message.contains(member), "{message}");
    }
}

/// FR-621-AC-7 (TC-1581), first half: a temporal clause still pending at the
/// end of its supplied trace is evaluated by the FR-641 clause run, whose
/// document carries `disposition.temporal` kind `pending`, category
/// `pending`, basis `unsettled` and `axes.settlement` `unsettled`, and exits
/// 0. A `finite-replay` analyze item naming the same temporal clause settles
/// `inapplicable-kind` with `family` `temporal-trace` (FR-507, FR-627-AC-4).
#[trace("TC-1581", "FR-621-AC-7")]
#[test]
#[ignore = "QSL-2272: the pending clause run and the inapplicable finite-replay item are not both reported yet"]
fn a_pending_clause_run_reports_its_pending_disposition() {
    let ws = Workspace::new("wire-pending-run");
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile i = \"filament.temporal.infinite-trace/v1\";\n\
         record Reading { level: Int[0, 9]; }\n\
         temporal Reaches using i over (r: Reading) clock \"steps\" on origin { eventually holds(r.level = 2) }\n";
    ws.write("unit.native", unit.as_bytes());
    let sample = |level: i64| json!({"sample": {"level": {"integer": level.to_string()}}});
    let request = json!({"format": "native-run/1", "request": {
        "models": [],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:pending-run", "document": "PendingRun", "formal_revision": 1}},
        "clause": {"selection": {"temporal": {"name": "Reaches",
            "trace": {"finite": [sample(0), sample(1)]}}}, "observations": {}}}});
    let path = ws.write("run.json", request.to_string().as_bytes());
    let result = fsl_command::command::run(&path).expect("the clause runs");
    assert_eq!(result.exit_code, 0);
    let document: Value =
        serde_json::from_str(&result.value.to_string()).expect("a result document");
    assert_eq!(document["disposition"]["category"], "pending", "{document}");
    assert_eq!(
        document["disposition"]["temporal"]["kind"], "pending",
        "{document}"
    );
    assert_eq!(document["basis"], "unsettled", "{document}");
    assert_eq!(document["axes"]["settlement"], "unsettled", "{document}");

    let reaches =
        crate::analyze_cli_support::clause("Reaches", "", "eventually holds(c.value = 3)");
    let trace = json!({"format": "filament.protocol-trace/v1", "identity":
        {"authority": "test", "identity": "recorded"}, "steps": []});
    ws.write("recorded.json", trace.to_string().as_bytes());
    let mut request = request_members(Some(json!([
        {"claim": "Reaches", "kind": "finite-replay",
         "trace": {"authority": "test", "identity": "recorded"}}
    ])));
    request["documents"]
        .as_array_mut()
        .expect("documents")
        .push(json!({"file": "recorded.json"}));
    let run = analyze(&counter_request_with(&ws, &reaches, request));
    assert_eq!(run.exit, 20);
    let item = run.item(0);
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "inapplicable-kind",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["detail"]["family"], "temporal-trace",
        "{item}"
    );
}

/// FR-620 "Subject, compilation and default items" (FSpec FR-520
/// `package`): a request that selects the checked package compiled from
/// the Counter program, with no program source, analyzes the same items as
/// the request over the source and gives an equal outcome.
#[trace("TC-1580", "FR-620-AC-1")]
#[trace("TC-2410", "FR-850-AC-1")]
#[test]
#[ignore = "QSL-2271: the in-process checked-package builder from package bytes conflicts with FR-087-AC-2"]
fn a_package_selection_analyzes_as_its_source() {
    let items = json!([{"claim": "Bounded"}, {"claim": "BelowThree"}]);
    let body = format!("{}{}", bounded(), below_three());
    let ws = Workspace::new("wire-package-selection");
    let source = analyze(&counter_request(&ws, &body, items.clone()));
    let request = request_members(Some(items));
    let compile = json!({"format": "native-compile/1", "request": {
        "models": request["models"], "program": request["program"]}});
    let compile_path = ws.write("compile.json", compile.to_string().as_bytes());
    let package = fsl_command::command::compile(&compile_path).expect("the program compiles");
    ws.write("package.json", &package);
    let mut selected = request.clone();
    selected
        .as_object_mut()
        .expect("a request object")
        .remove("program");
    selected["package"] = json!({"file": "package.json"});
    let document = json!({"format": "native-analyze/1", "request": selected});
    let path = ws.write("analyze-package.json", document.to_string().as_bytes());
    let run = analyze(&path);
    assert_eq!(run.exit, source.exit, "{}", run.document());
    assert_eq!(run.document()["items"], source.document()["items"]);

    // The same equality with no `items` member: the default items, in the
    // source map's order.
    let default_source = analyze(&counter_request_with(&ws, &body, request_members(None)));
    selected
        .as_object_mut()
        .expect("a request object")
        .remove("items");
    let document = json!({"format": "native-analyze/1", "request": selected});
    let path = ws.write(
        "analyze-package-default.json",
        document.to_string().as_bytes(),
    );
    let default_run = analyze(&path);
    assert_eq!(
        default_run.exit,
        default_source.exit,
        "{}",
        default_run.document()
    );
    assert_eq!(
        default_run.document()["items"],
        default_source.document()["items"]
    );
}

/// A package that admits reaches the analysis gap: building the in-process
/// checked package from package bytes conflicts with FR-087-AC-2, so
/// analysis refuses `unsupported_construct`/`not-yet-implemented` naming
/// `package`, never `invalid-request` or a profile prohibition (FR-763).
#[trace("TC-1580", "FR-620-AC-3")]
#[test]
fn a_package_selection_names_its_implementation_gap() {
    let ws = Workspace::new("wire-package-gap");
    let request = request_members(Some(json!([{"claim": "Bounded"}])));
    let _ = counter_request_with(&ws, &bounded(), request.clone());
    let compile = json!({"format": "native-compile/1", "request": {
        "models": request["models"], "program": request["program"]}});
    let compile_path = ws.write("compile.json", compile.to_string().as_bytes());
    let package = fsl_command::command::compile(&compile_path).expect("the program compiles");
    ws.write("package.json", &package);
    let mut selected = request;
    selected
        .as_object_mut()
        .expect("a request object")
        .remove("program");
    selected["package"] = json!({"file": "package.json"});
    let document = json!({"format": "native-analyze/1", "request": selected});
    let path = ws.write("analyze-package-gap.json", document.to_string().as_bytes());
    let run = analyze(&path);
    let document = run.document();
    let diagnostic = &document["diagnostics"][0];
    assert_eq!(diagnostic["code"], "unsupported_construct", "{document}");
    assert_eq!(diagnostic["cause"], "not-yet-implemented", "{document}");
    assert!(
        diagnostic["message"]
            .as_str()
            .is_some_and(|message| message.contains("package")),
        "{document}"
    );
    assert_eq!(run.exit, 21, "{document}");
}
