// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 (TC-2410): a `native-analyze/1` request that selects a v2 checked
//! package is analyzed from the package bytes alone, equals the request over
//! the source, and refuses malformed or mismatched package input before any
//! engine runs. FR-850-AC-1 is `analyze_request_wire`'s
//! `a_package_selection_analyzes_as_its_source`.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, below_three, bounded, clause, counter_request, label, package_selection,
    request_members, Run, Workspace, HANDOFF,
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
#[ignore = "QSL-2271: the in-process checked-package builder from package bytes conflicts with FR-087-AC-2"]
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
        run.item(0)["record"]["diagnostic"]["cause"], "unknown-claim",
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
#[ignore = "QSL-2271: the analysis engines do not read a package through AnalysisView yet"]
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
    let items =
        json!([{"claim": "Bounded"}, {"claim": "AtMostTwo"}, {"claim": "NotAboveThree"}]);
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
#[ignore = "QSL-2271: package forms of the source-checked evaluators are not implemented yet"]
fn the_package_route_reaches_the_source_checked_evaluators() {
    let body = format!("{}{}{}", bounded(), at_most_two(), not_above_three());
    let items = json!([{"claim": "NotAboveThree"}, {"claim": "AtMostTwo"}]);
    let ws = Workspace::new("package-route-evaluators");
    let source = analyze(&counter_request(&ws, &body, items.clone()));
    let (members, _) = package_request(&ws, &body, items);
    let run = analyze_members(&ws, "analyze-evaluators.json", &members);
    assert_eq!(run.item(0)["record"], source.item(0)["record"], "{}", run.document());
    assert_eq!(run.item(1)["record"], source.item(1)["record"], "{}", run.document());
    assert_eq!(run.item(1)["counterexample"], source.item(1)["counterexample"]);
    assert_eq!(run.item(1)["replay"], source.item(1)["replay"]);
    assert_eq!(run.item(1)["replay"]["reproduced"], true);
}

/// FR-940-AC-6: every item family settles equally on both routes, and a
/// package the strict reader refuses settles no
/// item and is never analyzed from source instead.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
#[ignore = "QSL-2271: the package route without a source fallback is not implemented yet"]
fn a_refused_package_never_falls_back_to_source() {
    let families = format!("{}{}{}", bounded(), at_most_two(), not_above_three());
    let all = json!([{"claim": "Bounded"}, {"claim": "AtMostTwo"}, {"claim": "NotAboveThree"}]);
    let ws = Workspace::new("package-equal-families");
    let source = analyze(&counter_request(&ws, &families, all.clone()));
    let (members, _) = package_request(&ws, &families, all);
    let package = analyze_members(&ws, "analyze-families.json", &members);
    assert_eq!(package.document()["items"], source.document()["items"]);
    for index in 0..3 {
        assert_ne!(label(&package.item(index)), "unsupported", "{}", package.document());
    }
    let body = format!("{}{}", bounded(), not_above_three());
    let items = json!([{"claim": "NotAboveThree"}]);
    let ws = Workspace::new("package-no-fallback");
    let (members, _) = package_request(&ws, &body, items);
    ws.write(&handoff_file("checked-package.json"), b"{}");
    let run = analyze_members(&ws, "analyze-refused.json", &members);
    assert_eq!(run.exit, 20, "{}", run.document());
    assert!(
        run.document()["items"].as_array().is_some_and(Vec::is_empty),
        "{}",
        run.document()
    );
    assert_eq!(run.document()["last_stage"], Value::Null, "{}", run.document());
    assert_eq!(first_diagnostic(&run)["code"], "invalid_package", "{}", run.document());
}

/// FR-850-AC-6 (the cases the admission tests above leave): an identity file
/// with a foreign `format` refuses `invalid_package`/`unknown-format`, and a
/// handoff with no `equality.json` is admitted with no equality rows.
#[trace("TC-2966", "FR-850-AC-6")]
#[test]
#[ignore = "QSL-2271: the unknown-format identity cause and an absent equality file not implemented yet"]
fn a_foreign_identity_format_refuses_and_an_absent_equality_file_is_admitted() {
    let ws = Workspace::new("package-handoff-identities");
    let (members, _) = package_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let identities = ws.path(&handoff_file("identities.json"));
    let rows = std::fs::read(&identities).expect("the handoff writes identities.json");

    let mut foreign: Value = serde_json::from_slice(&rows).expect("identities JSON");
    foreign["format"] = json!("filament.checked-handoff.identities/0");
    ws.write(&handoff_file("identities.json"), foreign.to_string().as_bytes());
    let run = analyze_members(&ws, "analyze-foreign-identities.json", &members);
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["code"], "invalid_package", "{diagnostic}");
    assert_eq!(diagnostic["cause"], "unknown-format", "{diagnostic}");
    assert_eq!(run.document()["items"], json!([]));
    assert_eq!(run.exit, 20);

    ws.write(&handoff_file("identities.json"), &rows);
    std::fs::remove_file(ws.path(&handoff_file("equality.json"))).expect("remove equality");
    let run = analyze_members(&ws, "analyze-no-equality.json", &members);
    assert_eq!(label(&run.item(0)), "proved", "{}", run.document());
    assert_eq!(run.exit, 0);
}

/// FSpec FR-520-AC-7: the `package` member is the closed `{directory}`
/// selection; the package file spelling `{file}` refuses `invalid-request`
/// naming `file`.
#[trace("FSpec-TC-2009", "FSpec-FR-520-AC-7")]
#[test]
#[ignore = "QSL-2271: refusing the package file spelling not verified yet"]
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

/// FR-850-AC-7 and FR-940-AC-7: the package names its invariant and
/// state-graph claims in FSpec FR-322's `claim_names`, the package route
/// resolves them through it with the source route's records, an absent name
/// is `unknown-claim`, and an uncovered or duplicate name table refuses the
/// request before any engine runs.
#[trace("TC-2967", "FSpec-TC-2011", "FR-850-AC-7", "FR-940-AC-7", "FSpec-FR-322-AC-457")]
#[test]
#[ignore = "QSL-2402: the checked package's claim_names table and package-route claim resolution through it not implemented yet"]
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
        assert_eq!(run.item(index)["record"], source.item(index)["record"], "{index}");
    }
    assert_eq!(
        run.item(2)["record"]["diagnostic"]["cause"], "unknown-claim",
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

/// FR-850, FR-940 (review round 1): an `invariant` state clause carries no
/// name row in the package, so a request naming it over the package is never
/// settled `unknown-claim` while the source route decides it; until the
/// invariant family has a package form the request is refused whole, naming
/// the item and the family.
#[trace("TC-2410", "FR-850-AC-5")]
#[trace("TC-2958", "FR-940-AC-6")]
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
    let diagnostic = first_diagnostic(&run);
    assert_eq!(diagnostic["cause"], "not-yet-implemented", "{diagnostic}");
    let message = diagnostic["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("items[0]") && message.contains("unnamed-claim"),
        "{diagnostic}"
    );
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
        assert_eq!(diagnostic["code"], "invalid_package", "{name}: {diagnostic}");
        assert_eq!(diagnostic["cause"], "missing-file", "{name}: {diagnostic}");
        assert_eq!(run.document()["items"], json!([]), "{name}");
        assert_eq!(run.exit, 20, "{name}");
    }
}
