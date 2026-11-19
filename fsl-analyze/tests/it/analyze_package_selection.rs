// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 (TC-2410): a `native-analyze/1` request that selects a v2 checked
//! package is analyzed from the package bytes alone, equals the request over
//! the source, and refuses malformed or mismatched package input before any
//! engine runs. FR-850-AC-1 is `analyze_request_wire`'s
//! `a_package_selection_analyzes_as_its_source`.

use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::{
    HANDOFF, Run, Workspace, analyze, below_three, bounded, clause, counter_request, label,
    package_selection, request_members,
};

fn at_most_two() -> String {
    clause("AtMostTwo", "", "always holds(c.value <= 2)")
}

/// Compile the Counter program with `body` into its FR-517 handoff
/// directory; returns the request members with `package` selecting it and no
/// `program`, and the package bytes.
fn package_request(ws: &Workspace, body: &str, items: Value) -> (Value, Vec<u8>) {
    // Writes the program, model and snapshot files beside the request.
    let _ = counter_request(ws, body, items.clone());
    package_selection(ws, &request_members(Some(items)))
}

/// The handoff file `name` of FR-850's `package` selection.
fn handoff_file(name: &str) -> String {
    format!("{HANDOFF}/{name}")
}

fn analyze_members(ws: &Workspace, name: &str, members: &Value) -> Run {
    let document = json!({"format": "native-analyze/1", "request": members});
    analyze(&ws.write(name, document.to_string().as_bytes()))
}

fn first_diagnostic(run: &Run) -> Value {
    run.document()["diagnostics"][0].clone()
}

/// FR-850-AC-2: the package route refutes `AtMostTwo` with the source
/// route's counterexample and replay.
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn a_package_refutation_equals_the_source_refutation() {
    let body = format!("{}{}{}", bounded(), below_three(), at_most_two());
    let items = json!([{"claim": "AtMostTwo"}]);
    let ws = Workspace::new("package-refutes");
    let source = analyze(&counter_request(&ws, &body, items.clone()));
    let (members, _) = package_request(&ws, &body, items);
    let run = analyze_members(&ws, "analyze-package.json", &members);
    assert_eq!(label(&run.item(0)), "refuted", "{}", run.item(0));
    assert_eq!(
        run.item(0)["counterexample"],
        source.item(0)["counterexample"]
    );
    assert_eq!(run.item(0)["replay"], source.item(0)["replay"]);
    assert_eq!(run.item(0)["replay"]["reproduced"], true);
    assert_eq!(run.exit, source.exit);
}

/// FR-850-AC-3 (negative): bytes the strict reader refuses refuse the whole
/// request `invalid_package` at the request stage, with no item.
#[trace("TC-2410", "FR-850-AC-3")]
#[test]
fn malformed_package_bytes_refuse_invalid_package() {
    let ws = Workspace::new("package-malformed");
    let (members, package) = package_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let good: Value = serde_json::from_slice(&package).expect("package JSON");
    let mut no_entry = good.clone();
    if let Some(entries) = no_entry["source_map"].as_array_mut() {
        entries.remove(0);
    }
    let mut v1 = good;
    v1["contract_version"] = json!("filament.checked-package/v1");
    for (name, bytes) in [
        ("empty", b"{}".to_vec()),
        ("v1", v1.to_string().into_bytes()),
        ("no-entry", no_entry.to_string().into_bytes()),
        ("not-json", b"not json".to_vec()),
    ] {
        ws.write(&handoff_file("checked-package.json"), &bytes);
        let run = analyze_members(&ws, &format!("analyze-{name}.json"), &members);
        let diagnostic = first_diagnostic(&run);
        assert_eq!(
            diagnostic["code"], "invalid_package",
            "{name}: {diagnostic}"
        );
        assert_eq!(run.document()["items"], json!([]), "{name}");
        assert_eq!(run.exit, 20, "{name}");
    }
}

/// FR-850 "Admission" (lead ruling on the handoff selection): a handoff
/// with no `identities.json`, or one the identity reader refuses, refuses
/// the whole request `invalid_package` with a named cause, exit 20, before
/// any item resolves; a present equality file the reader refuses refuses
/// the same way.
#[trace("TC-2410", "FR-850-AC-3")]
#[trace("TC-2966", "FR-850-AC-6", "FSpec-TC-2009", "FSpec-FR-520-AC-7")]
#[test]
fn a_handoff_without_its_identity_rows_refuses_invalid_package() {
    let ws = Workspace::new("package-identities");
    let (members, _) = package_request(&ws, &bounded(), json!([{"claim": "Missing"}]));
    let identities = ws.path(&handoff_file("identities.json"));
    let rows = std::fs::read(&identities).expect("the handoff writes identities.json");
    let equality = ws.path(&handoff_file("equality.json"));
    let equality_rows = std::fs::read(&equality).expect("the handoff writes equality.json");

    std::fs::remove_file(&identities).expect("the identity file is removed");
    let run = analyze_members(&ws, "analyze-no-identities.json", &members);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "invalid_package", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "missing-file", "{diagnostic}");
    assert!(
        diagnostic["message"]
            .as_str()
            .is_some_and(|message| message.contains("identities.json")),
        "{diagnostic}"
    );
    assert_eq!(run.document()["items"], json!([]));
    assert_eq!(run.exit, 20);

    ws.write(&handoff_file("identities.json"), b"not json");
    let run = analyze_members(&ws, "analyze-bad-identities.json", &members);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "invalid_package", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "malformed-file", "{diagnostic}");
    assert_eq!(run.exit, 20);

    ws.write(&handoff_file("identities.json"), &rows);
    ws.write(&handoff_file("equality.json"), b"{}");
    let run = analyze_members(&ws, "analyze-bad-equality.json", &members);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "invalid_package", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "refused-file", "{diagnostic}");
    assert_eq!(run.exit, 20);

    // With both files restored the request is admitted and its unknown
    // claim settles as an item.
    ws.write(&handoff_file("equality.json"), &equality_rows);
    let run = analyze_members(&ws, "analyze-restored.json", &members);
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["cause"],
        "unknown-claim",
        "{}",
        run.document()
    );
}

/// FR-850-AC-4 (negative): compile members beside `package`, missing models
/// and an unrecorded model refuse before any engine runs.
#[trace("TC-2410", "FR-850-AC-4")]
#[test]
fn compile_members_and_mismatched_models_refuse_beside_a_package() {
    let ws = Workspace::new("package-members");
    let (members, _) = package_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let source = request_members(None);
    let mut with_program = members.clone();
    with_program["program"] = source["program"].clone();
    let mut with_library = members.clone();
    with_library["libraries"] =
        json!([{"identity": "test/lib", "source": source["program"]["source"]}]);
    let mut with_division = members.clone();
    with_division["integer_division"] = json!("floor");
    for (member, request) in [
        ("program", with_program),
        ("libraries", with_library),
        ("integer_division", with_division),
    ] {
        let run = analyze_members(&ws, &format!("analyze-{member}.json"), &request);
        let diagnostic = first_diagnostic(&run);
        assert_eq!(
            diagnostic["code"], "invalid-request",
            "{member}: {diagnostic}"
        );
        assert!(
            diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains(member)),
            "{member}: {diagnostic}"
        );
        assert_eq!(run.exit, 20, "{member}");
    }
    let mut no_models = members.clone();
    no_models["models"] = json!([]);
    let run = analyze_members(&ws, "analyze-no-models.json", &no_models);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "missing_import", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "missing-selection", "{diagnostic}");
    assert!(
        diagnostic["message"]
            .as_str()
            .is_some_and(|message| message.contains("test/counter")),
        "{diagnostic}"
    );
    assert_eq!(run.exit, 20);

    let mut extra = members;
    let mut entry = extra["models"][0].clone();
    entry["source"]["identity"] = json!("other-model");
    extra["models"].as_array_mut().expect("models").push(entry);
    let run = analyze_members(&ws, "analyze-extra-model.json", &extra);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "invalid_package", "{diagnostic}");
    assert_eq!(
        diagnostic["cause"], "conflicting-definition",
        "{diagnostic}"
    );
    assert_eq!(run.exit, 20);
}

/// FR-850-AC-5 (negative): a name the package does not carry settles that
/// item `unknown-claim`, as the source route does.
#[trace("TC-2410", "FR-850-AC-5")]
#[test]
fn an_unknown_claim_over_a_package_settles_unknown_claim() {
    let ws = Workspace::new("package-unknown-claim");
    let (members, _) = package_request(&ws, &bounded(), json!([{"claim": "Missing"}]));
    let run = analyze_members(&ws, "analyze-missing.json", &members);
    let item = run.item(0);
    assert_eq!(item["category"], "refusal", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_capability",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "unknown-claim",
        "{item}"
    );
    assert_eq!(run.exit, 20);
}

/// FR-940-AC-1 and AC-3: the package route's `BarePackage` reads the same
/// facts through `AnalysisView` as the source route's `AdmittedPackageRead`
/// of the compiled package, so a `package` request is no longer
/// refused as not implemented and its items equal the source route's.
#[trace("TC-2620", "FR-940-AC-1", "FR-940-AC-3")]
#[test]
fn a_package_request_reads_the_source_route_facts_through_the_view() {
    let body = format!("{}{}{}", bounded(), below_three(), at_most_two());
    let items = json!([{"claim": "Bounded"}, {"claim": "BelowThree"}, {"claim": "AtMostTwo"}]);
    let ws = Workspace::new("package-view");
    let source = analyze(&counter_request(&ws, &body, items.clone()));
    let (members, _) = package_request(&ws, &body, items);
    let run = analyze_members(&ws, "analyze-view.json", &members);
    assert!(
        !run.document().to_string().contains("not-yet-implemented"),
        "{}",
        run.document()
    );
    assert_eq!(run.document()["items"], source.document()["items"]);
    assert_eq!(run.exit, source.exit);
}

fn not_above_three() -> &'static str {
    "invariant NotAboveThree using v on M::Counter at current { self.value <= 3 }\n"
}

/// FR-940-AC-4: analysis from source decides the temporal, refuted and
/// invariant items over its compiled package, and no record is `unsupported`
/// for a fact the package bytes lack.
#[trace("TC-2958", "FR-940-AC-4")]
#[test]
fn source_analysis_decides_every_family_over_its_checked_graph() {
    let body = format!("{}{}{}", bounded(), at_most_two(), not_above_three());
    let items = json!([{"claim": "Bounded"}, {"claim": "AtMostTwo"}, {"claim": "NotAboveThree"}]);
    let ws = Workspace::new("source-route-families");
    let run = analyze(&counter_request(&ws, &body, items));
    assert_eq!(label(&run.item(0)), "proved", "{}", run.document());
    assert_eq!(label(&run.item(1)), "refuted", "{}", run.document());
    assert_eq!(run.item(1)["replay"]["reproduced"], true);
    assert_eq!(label(&run.item(2)), "proved", "{}", run.document());
    assert_eq!(
        run.item(2)["record"]["basis"]["type"],
        "exhaustive",
        "{}",
        run.document()
    );
    for index in 0..3 {
        assert_ne!(label(&run.item(index)), "unsupported", "{}", run.document());
    }
}

/// FR-940-AC-5: the package route reaches the invariant and model-check
/// evaluators through their package forms with the source route's records.
#[trace("TC-2958", "FR-940-AC-5")]
#[test]
fn the_package_route_reaches_the_source_checked_evaluators() {
    let body = format!("{}{}{}", bounded(), at_most_two(), not_above_three());
    let items = json!([{"claim": "NotAboveThree"}, {"claim": "AtMostTwo"}]);
    let ws = Workspace::new("package-route-evaluators");
    let source = analyze(&counter_request(&ws, &body, items.clone()));
    let (members, _) = package_request(&ws, &body, items);
    let run = analyze_members(&ws, "analyze-evaluators.json", &members);
    assert_eq!(
        run.item(0)["record"],
        source.item(0)["record"],
        "{}",
        run.document()
    );
    assert_eq!(
        run.item(1)["record"],
        source.item(1)["record"],
        "{}",
        run.document()
    );
    assert_eq!(
        run.item(1)["counterexample"],
        source.item(1)["counterexample"]
    );
    assert_eq!(run.item(1)["replay"], source.item(1)["replay"]);
    assert_eq!(run.item(1)["replay"]["reproduced"], true);
}

/// FR-940-AC-6: every item family settles equally on both routes, and a
/// package the strict reader refuses settles no
/// item and is never analyzed from source instead.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_refused_package_never_falls_back_to_source() {
    let families = format!("{}{}{}", bounded(), at_most_two(), not_above_three());
    let all = json!([{"claim": "Bounded"}, {"claim": "AtMostTwo"}, {"claim": "NotAboveThree"}]);
    let ws = Workspace::new("package-equal-families");
    let source = analyze(&counter_request(&ws, &families, all.clone()));
    let (members, _) = package_request(&ws, &families, all);
    let package = analyze_members(&ws, "analyze-families.json", &members);
    assert_eq!(package.document()["items"], source.document()["items"]);
    for index in 0..3 {
        assert_ne!(
            label(&package.item(index)),
            "unsupported",
            "{}",
            package.document()
        );
    }
    let body = format!("{}{}", bounded(), not_above_three());
    let items = json!([{"claim": "NotAboveThree"}]);
    let ws = Workspace::new("package-no-fallback");
    let (members, _) = package_request(&ws, &body, items);
    ws.write(&handoff_file("checked-package.json"), b"{}");
    let run = analyze_members(&ws, "analyze-refused.json", &members);
    assert_eq!(run.exit, 20, "{}", run.document());
    assert!(
        run.document()["items"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "{}",
        run.document()
    );
    assert_eq!(
        run.document()["last_stage"],
        Value::Null,
        "{}",
        run.document()
    );
    assert_eq!(
        first_diagnostic(&run)["code"],
        "invalid_package",
        "{}",
        run.document()
    );
}

/// FR-850-AC-6 (the cases the admission tests above leave): an identity file
/// with a foreign `format` refuses `invalid_package`/`unknown-format`, and a
/// handoff with no `equality.json` of a package holding an equality
/// application refuses `invalid_package`/`missing-file`.
#[trace("TC-2966", "FR-850-AC-6")]
#[test]
fn a_foreign_identity_format_and_an_absent_equality_file_refuse_invalid_package() {
    let ws = Workspace::new("package-handoff-identities");
    let (members, _) = package_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let identities = ws.path(&handoff_file("identities.json"));
    let rows = std::fs::read(&identities).expect("the handoff writes identities.json");

    let mut foreign: Value = serde_json::from_slice(&rows).expect("identities JSON");
    foreign["format"] = json!("filament.checked-handoff.identities/0");
    ws.write(
        &handoff_file("identities.json"),
        foreign.to_string().as_bytes(),
    );
    let run = analyze_members(&ws, "analyze-foreign-identities.json", &members);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "invalid_package", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "unknown-format", "{diagnostic}");
    assert_eq!(run.document()["items"], json!([]));
    assert_eq!(run.exit, 20);

    // The Counter package holds the `Inc` postcondition's equality
    // application, so its equality rows are required (FR-850-AC-10).
    ws.write(&handoff_file("identities.json"), &rows);
    std::fs::remove_file(ws.path(&handoff_file("equality.json"))).expect("remove equality");
    let run = analyze_members(&ws, "analyze-no-equality.json", &members);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "invalid_package", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "missing-file", "{diagnostic}");
    assert_eq!(run.document()["items"], json!([]));
    assert_eq!(run.exit, 20);
}

/// FSpec FR-520-AC-7: the `package` member is the closed `{directory}`
/// selection; the package file spelling `{file}` refuses `invalid-request`
/// naming `file`.
#[trace("FSpec-TC-2009", "FSpec-FR-520-AC-7")]
#[test]
fn a_package_selection_names_a_directory_not_a_file() {
    let ws = Workspace::new("package-selection-directory");
    let (members, _) = package_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let run = analyze_members(&ws, "analyze-directory.json", &members);
    assert_eq!(run.exit, 0, "{}", run.document());

    let mut file = members;
    file["package"] = json!({"file": handoff_file("checked-package.json")});
    let run = analyze_members(&ws, "analyze-file.json", &file);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "invalid-request", "{diagnostic}");
    assert!(diagnostic.to_string().contains("file"), "{diagnostic}");
    assert_eq!(run.exit, 20);
}

fn reach_three() -> &'static str {
    "possible ReachThree using v over (c: M::Counter) { c.value = 3 }\n"
}

/// FR-940-AC-11: the four state-graph claims, one of each form, with an
/// `always possible` claim written without `from`.
fn state_graph_claims() -> &'static str {
    concat!(
        "possible ReachThree using v over (c: M::Counter) { c.value = 3 }\n",
        "always possible Back using v over (c: M::Counter) { c.value = 0 }\n",
        "always possible FromOne using v over (c: M::Counter) from (c.value = 1) { c.value = 0 }\n",
        "unique path Straight using v over (c: M::Counter) from (c.value = 0) to (c.value = 1);\n",
    )
}

/// FR-850-AC-7 and FR-940-AC-7: the package names its invariant and
/// state-graph claims in FSpec FR-322's `claim_names`, the package route
/// resolves them through it with the source route's records, an absent name
/// is `unknown-claim`, and an uncovered or duplicate name table refuses the
/// request before any engine runs.
#[trace(
    "TC-2967",
    "FSpec-TC-2011",
    "FR-850-AC-7",
    "FR-940-AC-7",
    "FSpec-FR-322-AC-457"
)]
#[test]
fn package_claims_resolve_through_the_claim_name_table() {
    let body = format!("{}{}{}", bounded(), not_above_three(), reach_three());
    let items = json!([{"claim": "NotAboveThree"}, {"claim": "ReachThree"}, {"claim": "Missing"}]);
    let ws = Workspace::new("package-claim-names");
    let source = analyze(&counter_request(&ws, &body, items.clone()));
    let (members, _) = package_request(&ws, &body, items);
    let package_path = ws.path(&handoff_file("checked-package.json"));
    let package: Value =
        serde_json::from_slice(&std::fs::read(&package_path).expect("package")).expect("JSON");
    let rows = package["claim_names"].as_array().expect("claim_names rows");
    let names: Vec<&Value> = rows.iter().map(|row| &row["qualified_name"]).collect();
    assert!(names.contains(&&json!(["NotAboveThree"])), "{rows:?}");
    assert!(names.contains(&&json!(["ReachThree"])), "{rows:?}");

    let run = analyze_members(&ws, "analyze-claim-names.json", &members);
    for index in 0..2 {
        assert_eq!(
            run.item(index)["record"],
            source.item(index)["record"],
            "{index}"
        );
    }
    assert_eq!(
        run.item(2)["record"]["diagnostic"]["cause"],
        "unknown-claim",
        "{}",
        run.item(2)
    );

    let reach = rows
        .iter()
        .find(|row| row["qualified_name"] == json!(["ReachThree"]))
        .expect("the ReachThree row")
        .clone();
    let mut uncovered = package.clone();
    uncovered["claim_names"] = json!([reach.clone()]);
    std::fs::write(&package_path, uncovered.to_string()).expect("write");
    let run = analyze_members(&ws, "analyze-uncovered.json", &members);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "invalid_package", "{diagnostic}");
    assert_eq!(run.document()["items"], json!([]));

    let mut duplicate = package;
    for row in duplicate["claim_names"].as_array_mut().expect("rows") {
        row["qualified_name"] = json!(["ReachThree"]);
        row["name"] = json!("ReachThree");
    }
    std::fs::write(&package_path, duplicate.to_string()).expect("write");
    let run = analyze_members(&ws, "analyze-duplicate.json", &members);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "ambiguous_declaration", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "ambiguous-name", "{diagnostic}");
}

/// FR-940-AC-11: the four state-graph claims, one of each form, settle over
/// a package with the source route's records, witnesses or counterexamples,
/// replays and certificates, and no package-route record is `unsupported`.
#[trace("TC-3034", "FR-940-AC-11")]
#[test]
fn a_package_state_graph_claim_settles_as_its_source_claim() {
    let body = format!("{}{}", bounded(), state_graph_claims());
    let names = ["ReachThree", "Back", "FromOne", "Straight"];
    let items = json!(names.map(|name| json!({"claim": name})));
    let ws = Workspace::new("package-state-graph");
    let source = analyze(&counter_request(&ws, &body, items.clone()));
    let (members, _) = package_request(&ws, &body, items);
    let run = analyze_members(&ws, "analyze-state-graph.json", &members);
    for (index, name) in names.iter().enumerate() {
        assert_eq!(
            run.item(index),
            source.item(index),
            "{name}: {}",
            run.document()
        );
        assert!(
            !run.item(index).to_string().contains("unsupported"),
            "{name}: {}",
            run.item(index)
        );
    }
    for member in ["items", "aggregate", "last_stage", "category"] {
        assert_eq!(
            run.document()[member],
            source.document()[member],
            "{member}"
        );
    }
    assert_eq!(run.exit, source.exit);
}

/// FR-850-AC-7, FR-940 (review round 1): an `invariant` state clause is
/// named by its `claim_names` row, so a request naming it over the package
/// is never settled `unknown-claim` while the source route decides it: it
/// settles with the source route's document, member by member.
#[trace("TC-2410", "FR-850-AC-5")]
#[trace("TC-2958", "FR-940-AC-6")]
#[trace("TC-2967", "FR-850-AC-7")]
#[test]
fn an_invariant_named_over_a_package_is_never_an_unknown_claim() {
    let items = json!([{"claim": "NotAboveThree"}]);
    let ws = Workspace::new("package-invariant-name");
    let source = analyze(&counter_request(&ws, not_above_three(), items.clone()));
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    let (members, _) = package_request(&ws, not_above_three(), items);
    let run = analyze_members(&ws, "analyze-invariant.json", &members);
    assert!(
        !run.document().to_string().contains("unknown-claim"),
        "{}",
        run.document()
    );
    for member in ["items", "aggregate", "last_stage", "category", "registry"] {
        assert_eq!(
            run.document()[member],
            source.document()[member],
            "{member}: {}",
            run.document()
        );
    }
    assert_eq!(run.exit, source.exit);
}

/// FR-850 "Admission" (FSpec FR-520): an absent handoff directory, and a
/// handoff with no `checked-package.json`, refuse the request
/// `invalid_package`/`missing-file` at the request stage, exit 20.
#[trace("TC-2410", "FR-850-AC-3")]
#[trace("TC-2966", "FR-850-AC-6")]
#[test]
fn an_absent_handoff_or_package_file_refuses_missing_file() {
    let ws = Workspace::new("package-absent");
    let (members, _) = package_request(&ws, &bounded(), json!([{"claim": "Missing"}]));
    let mut elsewhere = members.clone();
    elsewhere["package"] = json!({"directory": "no-such-handoff"});
    std::fs::remove_file(ws.path(&handoff_file("checked-package.json")))
        .expect("the package file is removed");
    for (name, request) in [("no-directory", &elsewhere), ("no-package", &members)] {
        let run = analyze_members(&ws, &format!("analyze-{name}.json"), request);
        let diagnostic = first_diagnostic(&run);
        assert_eq!(
            diagnostic["code"], "invalid_package",
            "{name}: {diagnostic}"
        );
        assert_eq!(diagnostic["cause"], "missing-file", "{name}: {diagnostic}");
        assert_eq!(run.document()["items"], json!([]), "{name}");
        assert_eq!(run.exit, 20, "{name}");
    }
}

/// FR-940-AC-8, FR-940-AC-12 and FSpec FR-322-AC-459: claim names and every
/// other declaration's name are one namespace, and the later declaration in
/// source order is the one refused. A source unit declaring the temporal
/// clause `Bounded` and the state-graph claim `Bounded`, in either order, is
/// refused `ambiguous_declaration`/`ambiguous-name` at the later name before
/// any item is analyzed (exit 20, category `refusal`, last stage `S3`).
#[trace("TC-2970", "FR-940-AC-8", "FSpec-TC-2012", "FSpec-FR-322-AC-459")]
#[trace("TC-3034", "FR-940-AC-12")]
#[test]
fn a_name_in_both_name_tables_is_refused_at_the_source_route() {
    let claim = "possible Bounded using v over (c: M::Counter) { c.value = 3 }\n";
    let orders = [
        ("temporal-first", format!("{}{claim}", bounded())),
        ("claim-first", format!("{claim}{}", bounded())),
    ];
    for (name, body) in orders {
        let ws = Workspace::new(&format!("claim-name-namespace-source-{name}"));
        let run = analyze(&counter_request(&ws, &body, json!([{"claim": "Bounded"}])));
        let document = run.document();
        let diagnostic = first_diagnostic(&run);
        assert_eq!(
            diagnostic["code"], "ambiguous_declaration",
            "{name}: {document}"
        );
        assert_eq!(diagnostic["cause"], "ambiguous-name", "{name}: {document}");
        assert_eq!(run.exit, 20, "{name}: {document}");
        assert_eq!(document["category"], "refusal", "{name}: {document}");
        assert_eq!(document["last_stage"], "S3", "{name}: {document}");
        assert_eq!(document["items"], json!([]), "{name}: {document}");
        let text = std::fs::read_to_string(ws.path("program.fsl")).expect("the unit");
        let later = text.rfind("Bounded").expect("the later name");
        assert_eq!(
            (
                diagnostic["locus"]["start"].as_u64(),
                diagnostic["locus"]["end"].as_u64()
            ),
            (Some(later as u64), Some((later + "Bounded".len()) as u64)),
            "{name}: located at the later declaration's name: {document}"
        );
    }
}

/// FR-940-AC-12, package half: a package whose `claim_names` row is renamed
/// to a `temporal_clause_names` row's name is refused the same at that row
/// (`details.path` `/claim_names/{i}/name`) before any engine runs; with
/// distinct names both resolve.
#[trace("TC-3034", "FR-940-AC-12")]
#[test]
fn a_name_in_both_name_tables_is_refused_at_the_package_route() {
    let items = json!([{"claim": "Bounded"}, {"claim": "ReachThree"}]);
    let ws = Workspace::new("claim-name-namespace-package");
    let body = format!("{}{}", bounded(), reach_three());
    let source = analyze(&counter_request(&ws, &body, items.clone()));
    let (members, _) = package_request(&ws, &body, items);
    let run = analyze_members(&ws, "analyze-distinct.json", &members);
    for index in 0..2 {
        assert_eq!(
            run.item(index)["record"],
            source.item(index)["record"],
            "{index}"
        );
    }
    let package_path = ws.path(&handoff_file("checked-package.json"));
    let mut package: Value =
        serde_json::from_slice(&std::fs::read(&package_path).expect("package")).expect("JSON");
    for row in package["claim_names"]
        .as_array_mut()
        .expect("claim_names rows")
    {
        if row["name"] == json!("ReachThree") {
            row["name"] = json!("Bounded");
            row["qualified_name"] = json!(["Bounded"]);
        }
    }
    std::fs::write(&package_path, package.to_string()).expect("write");
    let run = analyze_members(&ws, "analyze-colliding.json", &members);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "ambiguous_declaration", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "ambiguous-name", "{diagnostic}");
    let path = diagnostic["details"]["path"].as_str().unwrap_or_default();
    assert!(
        path.starts_with("/claim_names/") && path.ends_with("/name"),
        "{diagnostic}"
    );
    assert_eq!(run.exit, 20, "{diagnostic}");
    assert_eq!(run.document()["category"], "refusal", "{diagnostic}");
    assert_eq!(run.document()["last_stage"], Value::Null, "{diagnostic}");
    assert_eq!(run.document()["items"], json!([]));
}

/// The `request` members of the `native-analyze/1` request at `path`.
fn request_of(path: &std::path::Path) -> Value {
    let document: Value =
        serde_json::from_slice(&std::fs::read(path).expect("the request")).expect("JSON");
    document["request"].clone()
}

/// The item `{claim, temporal: {item_kind: deadlock-freedom}}` over the
/// model selection `identity`.
fn deadlock_item(identity: &str) -> Value {
    json!({"claim": identity, "temporal": {"item_kind": "deadlock-freedom"}})
}

/// Analyze `unit` over `model` from counter `a` at 0 with `items`, from
/// source and from its handoff directory; returns both runs.
fn both_routes(name: &str, unit: &str, model: &[u8], items: &Value) -> (Run, Run) {
    let ws = Workspace::new(name);
    let path = crate::analyze_cli_support::request_over(
        &ws,
        unit,
        model,
        &[crate::analyze_cli_support::counter_snapshot(&[("a", 0)])],
        json!({ "items": items }),
    );
    let source = analyze(&path);
    let (members, _) = package_selection(&ws, &request_of(&path));
    let package = analyze_members(&ws, "analyze-package.json", &members);
    (source, package)
}

/// FR-850 "Equality with the source route", FR-940-AC-3 and AC-6 (QSL-2406):
/// `fsl analyze` over the handoff directory of a Counter unit whose
/// operations come from its domain package gives the source route's
/// document: the temporal clauses `Bounded` (proved) and `AtMostTwo`
/// (refuted, its counterexample replayed from the package bytes) and the
/// derived deadlock-freedom item over `test/counter` settle with equal
/// items, aggregate, last stage and exit code; over the Counter with no
/// operation the deadlock item is refuted on both routes with the same
/// counterexample and replay.
#[trace("TC-2410", "FR-850-AC-2")]
#[trace("TC-2620", "FR-940-AC-3")]
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_package_request_settles_temporal_and_deadlock_items_as_the_source_route() {
    let unit = crate::counter::unit_text(&format!("{}{}", bounded(), at_most_two()));
    let items =
        json!([{"claim": "Bounded"}, {"claim": "AtMostTwo"}, deadlock_item("test/counter")]);
    let (source, package) = both_routes(
        "package-route-families",
        &unit,
        &crate::counter::document(false),
        &items,
    );
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_eq!(label(&source.item(1)), "refuted", "{}", source.document());
    assert_eq!(label(&source.item(2)), "proved", "{}", source.document());
    assert_eq!(package.item(1)["replay"]["reproduced"], true);
    for member in ["items", "aggregate", "last_stage", "category", "registry"] {
        assert_eq!(
            package.document()[member],
            source.document()[member],
            "{member}: {}",
            package.document()
        );
    }
    assert_eq!(package.exit, source.exit);

    let still = crate::model_world::still_counter();
    let (source, package) = both_routes(
        "package-route-deadlock",
        &crate::counter::unit_text(&bounded()),
        &still.document,
        &json!([deadlock_item("test/counter")]),
    );
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
    assert_eq!(
        package.document()["items"],
        source.document()["items"],
        "{}",
        package.document()
    );
    assert_eq!(package.exit, source.exit);
}

/// FR-940 "Package forms of the source-checked evaluators" (QSL-2406,
/// QSL-2432): a request whose item names a family the package route has
/// no form of yet is refused whole, typed and naming the family, never
/// settled from source or `unsupported`: a clause with a fairness set (naming
/// the fact and its node) and the default items. A model alias settles its
/// own item `model-alias` instead (FR-850-AC-9).
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn a_pending_package_family_refuses_typed_naming_it() {
    let fair = clause(
        "Fair",
        "fair weak M::Counter::inc;",
        "eventually holds(c.value = 3)",
    );
    let unit = crate::counter::unit_text(&format!("{}{fair}", bounded()));
    for (items, expected) in [(
        json!([{"claim": "Bounded"}, {"claim": "Fair"}]),
        "items[1] names a temporal clause whose fairness has no package form yet at",
    )] {
        let (_, package) = both_routes(
            "package-route-pending",
            &unit,
            &crate::counter::document(false),
            &items,
        );
        let diagnostic = first_diagnostic(&package);
        assert_eq!(diagnostic["cause"], "not-yet-implemented", "{diagnostic}");
        assert!(
            diagnostic["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected)),
            "{diagnostic}"
        );
        assert_eq!(package.document()["items"], json!([]));
    }
}

/// FR-850-AC-9 (TC-2987): on the package route a derived item names its
/// model by the package identity its model selection records; the source
/// alias `M` settles that item `invalid-request` `model-alias` naming the
/// identity to use, never refusing the request as not implemented, and a
/// unit importing the model as `N` writes the same package bytes.
#[trace("TC-2987", "FR-850-AC-9")]
#[test]
fn a_package_request_names_its_model_by_identity_and_settles_an_alias_model_alias() {
    let unit = crate::counter::unit_text(&bounded());
    let identity = json!([{"claim": "Bounded"}, deadlock_item("test/counter")]);
    let (source, package) = both_routes(
        "package-model-identity",
        &unit,
        &crate::counter::document(false),
        &identity,
    );
    for index in 0..2 {
        assert_eq!(
            package.item(index)["record"],
            source.item(index)["record"],
            "{index}"
        );
    }

    let aliased = json!([{"claim": "Bounded"}, deadlock_item("M")]);
    let (source_alias, package_alias) = both_routes(
        "package-model-alias",
        &unit,
        &crate::counter::document(false),
        &aliased,
    );
    assert_eq!(
        source_alias.item(1)["record"],
        source.item(1)["record"],
        "{}",
        source_alias.document()
    );
    assert_eq!(
        package_alias.item(0)["record"],
        source.item(0)["record"],
        "{}",
        package_alias.document()
    );
    let item = package_alias.item(1);
    assert_eq!(item["category"], "refusal", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_capability",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "model-alias",
        "{item}"
    );
    let text = item["record"]["diagnostic"].to_string();
    assert!(
        text.contains("\"M\"") || text.contains("`M`"),
        "names the alias: {item}"
    );
    assert!(
        text.contains("test/counter"),
        "names the identity to use: {item}"
    );
    let document = package_alias.document().to_string();
    assert!(!document.contains("not-yet-implemented"), "{document}");
    assert!(!document.contains("\"unsupported\""), "{document}");

    let package_bytes = |name: &str, unit: &str| {
        let ws = Workspace::new(name);
        let path = crate::analyze_cli_support::request_over(
            &ws,
            unit,
            &crate::counter::document(false),
            &[crate::analyze_cli_support::counter_snapshot(&[("a", 0)])],
            json!({ "items": identity.clone() }),
        );
        package_selection(&ws, &request_of(&path)).1
    };
    let renamed = unit.replace("model M =", "model N =").replace("M::", "N::");
    assert_eq!(
        package_bytes("package-model-m", &unit),
        package_bytes("package-model-n", &renamed),
        "units importing one model as M and as N write equal package bytes"
    );
}

/// The `claim` ordinal an item's request names.
fn claim_ordinal(item: &Value) -> Value {
    item["request"]["claim"]["occurrence"]["ordinal"].clone()
}

/// FR-850-AC-10 (QSL-2406): a package holding an equality application is
/// read with its producer's equality rows, so a handoff without
/// `equality.json` refuses the whole request `invalid_package`/
/// `missing-file` at the request stage; a package holding none is admitted
/// without the file and settles as the source route does.
#[trace("TC-3025", "FR-850-AC-10")]
#[test]
fn an_absent_equality_file_refuses_a_package_that_holds_an_equality() {
    let items = json!([{"claim": "Bounded"}]);
    let ws = Workspace::new("package-equality-held");
    let source = analyze(&counter_request(&ws, &bounded(), items.clone()));
    let (members, package) = package_request(&ws, &bounded(), items.clone());
    let package: Value = serde_json::from_slice(&package).expect("package JSON");
    assert!(
        package["checked_term_data"]["equality_decisions"]
            .as_array()
            .is_some_and(|decisions| !decisions.is_empty()),
        "the Inc postcondition's `=` is an equality application"
    );
    let equality = ws.path(&handoff_file("equality.json"));
    let rows = std::fs::read(&equality).expect("the handoff writes equality.json");
    std::fs::remove_file(&equality).expect("remove equality");
    let run = analyze_members(&ws, "analyze-equality-absent.json", &members);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "invalid_package", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "missing-file", "{diagnostic}");
    assert!(
        diagnostic["message"]
            .as_str()
            .is_some_and(|message| message.contains("equality.json")),
        "{diagnostic}"
    );
    assert_eq!(run.document()["items"], json!([]));
    assert_eq!(run.exit, 20);

    ws.write(&handoff_file("equality.json"), &rows);
    let run = analyze_members(&ws, "analyze-equality-present.json", &members);
    assert_eq!(label(&run.item(0)), "proved", "{}", run.document());
    assert_eq!(run.item(0)["record"], source.item(0)["record"]);

    let greater = format!(
        "pre CanInc using v on M::Counter::inc {{ self.value < 3 }}\n\
         post Inc using v on M::Counter::inc {{ self.value > pre(self.value) }}\n{}",
        bounded()
    );
    let ws = Workspace::new("package-equality-none");
    let path = crate::analyze_cli_support::request_over(
        &ws,
        &crate::counter::unit_text(&greater),
        &crate::counter::document(false),
        &[crate::analyze_cli_support::counter_snapshot(&[("a", 0)])],
        json!({ "items": items }),
    );
    let source = analyze(&path);
    let (members, package) = package_selection(&ws, &request_of(&path));
    let package: Value = serde_json::from_slice(&package).expect("package JSON");
    assert!(
        package["checked_term_data"]["equality_decisions"]
            .as_array()
            .is_none_or(Vec::is_empty),
        "no equality application"
    );
    std::fs::remove_file(ws.path(&handoff_file("equality.json"))).expect("remove equality");
    let run = analyze_members(&ws, "analyze-no-equality-held.json", &members);
    assert_eq!(label(&run.item(0)), "proved", "{}", run.document());
    assert_eq!(run.item(0)["record"], source.item(0)["record"]);
    assert_eq!(run.exit, 0);
}

/// FR-940-AC-10 (QSL-2402): `BelowThree` and `AtMostTwo` share one
/// `temporal_clause` node, each its own `claim` occurrence numbered in
/// source order. Both routes refute each item with a reproduced replay at
/// its own declaration's occurrence, whether both are requested or only the
/// later one, and swapping the declarations swaps the ordinals.
#[trace("TC-3026", "FR-940-AC-10")]
#[test]
fn declarations_of_a_shared_clause_node_keep_their_own_occurrence() {
    for (name, order, items) in [
        (
            "both",
            [("BelowThree", 0), ("AtMostTwo", 1)],
            json!([{"claim": "BelowThree"}, {"claim": "AtMostTwo"}]),
        ),
        (
            "later",
            [("BelowThree", 0), ("AtMostTwo", 1)],
            json!([{"claim": "AtMostTwo"}]),
        ),
        (
            "swapped",
            [("AtMostTwo", 0), ("BelowThree", 1)],
            json!([{"claim": "BelowThree"}, {"claim": "AtMostTwo"}]),
        ),
    ] {
        let body = if name == "swapped" {
            format!("{}{}{}", bounded(), at_most_two(), below_three())
        } else {
            format!("{}{}{}", bounded(), below_three(), at_most_two())
        };
        let ws = Workspace::new(&format!("shared-node-{name}"));
        let source = analyze(&counter_request(&ws, &body, items.clone()));
        let (members, _) = package_request(&ws, &body, items.clone());
        let run = analyze_members(&ws, &format!("analyze-shared-{name}.json"), &members);
        let requested = items.as_array().expect("items").len();
        for index in 0..requested {
            let claim = items[index]["claim"].as_str().expect("claim name");
            let ordinal = order
                .iter()
                .find(|(declared, _)| *declared == claim)
                .map(|(_, ordinal)| json!(ordinal))
                .expect("a declared name");
            for (route, result) in [("source", &source), ("package", &run)] {
                let item = result.item(index);
                assert_eq!(label(&item), "refuted", "{name} {route} {claim}: {item}");
                assert_eq!(item["replay"]["reproduced"], true, "{name} {route} {claim}");
                assert_eq!(claim_ordinal(&item), ordinal, "{name} {route} {claim}");
            }
        }
        assert_eq!(
            run.document()["items"],
            source.document()["items"],
            "{name}"
        );
        assert_eq!(run.exit, source.exit, "{name}");
    }
}
