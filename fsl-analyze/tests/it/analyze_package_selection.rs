// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 (TC-2410): a `native-analyze/1` request that selects a v2 checked
//! package is analyzed from the package bytes alone, equals the request over
//! the source, and refuses malformed or mismatched package input before any
//! engine runs. FR-850-AC-1 is `analyze_request_wire`'s
//! `a_package_selection_analyzes_as_its_source`.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, below_three, bounded, clause, counter_request, label, request_members, Run, Workspace,
};

fn at_most_two() -> String {
    clause("AtMostTwo", "", "always holds(c.value <= 2)")
}

/// Compile the Counter program with `body` and write its package bytes as
/// `package.json`; returns the request members with `package` selected and no
/// `program`, and the package bytes.
fn package_request(ws: &Workspace, body: &str, items: Value) -> (Value, Vec<u8>) {
    // Writes the program, model and snapshot files beside the request.
    let _ = counter_request(ws, body, items.clone());
    let request = request_members(Some(items));
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
    (selected, package)
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
        ws.write("package.json", &bytes);
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
#[ignore = "QSL-2271: the in-process checked-package builder from package bytes conflicts with FR-087-AC-2"]
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
