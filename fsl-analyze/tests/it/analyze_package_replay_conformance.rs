// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 "Equality with the source route", FR-940, FR-636 (QSL-2785): a
//! `finite-replay` item over a supplied protocol trace and a
//! `global-conformance` item over a supplied recording settle from the
//! package bytes exactly as they settle from the authored source.

use fsl_eval::simulation::protocol::StepKind;
use ix_trace_rs::trace;
use serde_json::{Value, json};

use crate::analyze_cli_support::{
    HANDOFF, Run, Workspace, analyze, label, litmus_request, order_protocol, package_selection,
    request_over,
};
use crate::protocol_composition::{order_steps, trace_document};

/// The source run of `items` over the order protocol with `documents`
/// provisioned, then the package run of the same request once the authored
/// unit and the handoff's `sources/` directory are deleted.
fn both_routes(name: &str, items: Value, documents: &[(&str, Value)]) -> (Run, Run) {
    let ws = Workspace::new(name);
    let has_recording = documents
        .iter()
        .any(|(_, document)| document["format"] == "filament.protocol-recording/1");
    let protocol = if has_recording {
        // Keep the receive field individually visible to the choice owner.
        // The guards call a checked function over that fact, incurring its
        // function-call charge; a plain Boolean field read has no charge.
        format!(
            "function observed using v(x: Boolean): Boolean pure {{ x }}\n{}",
            order_protocol(true)
                .replace("when { not got.ok }", "when { not observed(got.ok) }")
                .replace("when { got.ok }", "when { observed(got.ok) }")
        )
    } else {
        order_protocol(true)
    };
    let path = litmus_request(&ws, &protocol, json!({"items": items}));
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("json");
    for (file, document) in documents {
        ws.write(file, document.to_string().as_bytes());
        request["request"]["documents"]
            .as_array_mut()
            .expect("documents")
            .push(json!({"file": file}));
    }
    ws.write("analyze.json", request.to_string().as_bytes());
    let source = analyze(&path);
    assert_eq!(source.exit, 0, "{}", source.document());
    let (members, bytes) = package_selection(&ws, &request["request"]);
    for (_, document) in documents {
        if document["format"] == "filament.protocol-recording/1" {
            assert_recording_charge_parity(&ws, &bytes, document);
        }
    }
    std::fs::remove_file(ws.path("program.fsl")).expect("the authored unit is removed");
    let sources = ws.path(&format!("{HANDOFF}/sources"));
    std::fs::remove_dir_all(&sources).expect("the handoff sources are removed");
    assert!(!sources.exists(), "the handoff sources are gone");
    let document = json!({"format": "native-analyze/1", "request": members});
    let package = analyze(&ws.write("analyze-package.json", document.to_string().as_bytes()));
    (source, package)
}

/// [`both_routes`] with a hand-supplied subject snapshot set.
fn both_routes_with_snapshots(
    name: &str,
    items: Value,
    snapshots: &[Vec<u8>],
    subject: Value,
) -> (Run, Run) {
    let ws = Workspace::new(name);
    let protocol = order_protocol(true);
    let path = request_over(
        &ws,
        &crate::litmus::unit_text(&protocol),
        &crate::litmus::resulting_document(),
        snapshots,
        json!({"items": items, "subject": subject}),
    );
    let request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("json");
    let source = analyze(&path);
    assert_eq!(source.exit, 0, "{}", source.document());
    let (members, _) = package_selection(&ws, &request["request"]);
    std::fs::remove_file(ws.path("program.fsl")).expect("the authored unit is removed");
    let sources = ws.path(&format!("{HANDOFF}/sources"));
    std::fs::remove_dir_all(&sources).expect("the handoff sources are removed");
    assert!(!sources.exists(), "the handoff sources are gone");
    let document = json!({"format": "native-analyze/1", "request": members});
    let package = analyze(&ws.write("analyze-package.json", document.to_string().as_bytes()));
    (source, package)
}

fn keyed_core_snapshots(q_r1: &str) -> Vec<Vec<u8>> {
    crate::litmus::Documents::new()
        .snapshots
        .into_values()
        .map(|bytes| {
            let mut snapshot: Value = serde_json::from_slice(&bytes).expect("snapshot JSON");
            for population in snapshot["populations"]
                .as_array_mut()
                .expect("snapshot populations")
            {
                if population["population"].to_string().contains("cores") {
                    let core = |key: &str, r1: &str| {
                        json!({"key": key, "type": "ix://test/litmus/Core", "fields": {
                            "r1": {"integer": r1}, "r2": {"integer": "0"},
                            "r3": {"integer": "0"}, "r4": {"integer": "0"}}})
                    };
                    population["objects"] = json!([core("p", "0"), core("q", q_r1)]);
                }
            }
            serde_json::to_vec(&snapshot).expect("snapshot JSON")
        })
        .collect()
}

fn both_routes_for_unit_with_snapshots(
    name: &str,
    unit: &str,
    items: Value,
    snapshots: &[Vec<u8>],
    subject: Option<Value>,
) -> (Run, Run) {
    let ws = Workspace::new(name);
    let extra = match subject {
        Some(subject) => json!({"items": items, "subject": subject}),
        None => json!({"items": items}),
    };
    let path = request_over(
        &ws,
        &crate::litmus::unit_text(unit),
        &crate::litmus::resulting_document(),
        snapshots,
        extra,
    );
    let request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("request JSON");
    let source = analyze(&path);
    assert_eq!(source.exit, 0, "{}", source.document());
    let (members, _) = package_selection(&ws, &request["request"]);
    std::fs::remove_file(ws.path("program.fsl")).expect("authored unit is removed");
    let sources = ws.path(&format!("{HANDOFF}/sources"));
    std::fs::remove_dir_all(&sources).expect("handoff sources are removed");
    let package = analyze(
        &ws.write(
            "analyze-package.json",
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    );
    (source, package)
}

/// Compare the real recording assessor's causal binder frame and predicate
/// path, including the admitted package evaluator, before testing CLI parity.
fn assert_recording_charge_parity(ws: &Workspace, bytes: &[u8], document: &Value) {
    use filament_exact::{Meter, Value as ExactValue};
    use fsl_eval::simulation::model::PackageModelInput;
    use fsl_package::{BarePackage, BareSupply, ReadEqualityRows, ReadIdentityRows};
    use fsl_semantics::model::accounting::ModelNormalizationLimits;

    let compiled = fsl_command::checked_handoff::compile(&ws.path(HANDOFF))
        .expect("the genuine producer handoff compiles");
    let limits = fsl_package::V2ReadLimits::bounded(1 << 24);
    let admitted = fsl_package::admit_checked_package(bytes, compiled.packages(), limits)
        .expect("the producer bytes admit");
    let mut budget = limits.native_join_limits().budget();
    let identities = ReadIdentityRows::read(
        &std::fs::read(ws.path(&format!("{HANDOFF}/identities.json"))).expect("identity rows"),
        &admitted,
        &mut budget,
    )
    .expect("the producer identity rows join");
    let equality = std::fs::read(ws.path(&format!("{HANDOFF}/equality.json")))
        .ok()
        .map(|bytes| ReadEqualityRows::read(&bytes, &admitted).expect("equality rows join"));
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: equality.as_ref(),
            },
            &mut budget,
        )
        .expect("the actual producer supply joins");
    let owner = PackageModelInput::of(
        &bare,
        compiled.packages(),
        ModelNormalizationLimits::default(),
    )
    .expect("native package input admits");
    let mut payload = document.clone();
    payload
        .as_object_mut()
        .expect("recording document")
        .remove("identity");
    let payload = serde_json::to_vec(&payload).expect("recording payload");
    let mut source_meter = Meter::new(crate::counter::UNLIMITED);
    let mut package_meter = Meter::new(crate::counter::UNLIMITED);
    let source = fsl_analyze::protocol_analysis::recording::recording_predicate_source(
        compiled.checked(),
        &payload,
        "Flow::Main::Decide::yes",
        2,
        &mut source_meter,
    );
    let package = fsl_analyze::protocol_analysis::recording::recording_predicate_package(
        &owner.input(),
        &payload,
        "Flow::Main::Decide::yes",
        2,
        &mut package_meter,
    );
    assert!(matches!(source, Some(ExactValue::Boolean(true))));
    match (&package, &source) {
        (Some(ExactValue::Boolean(actual)), Some(ExactValue::Boolean(expected))) => {
            assert_eq!(actual, expected);
        }
        _ => panic!(
            "recording predicate must evaluate to Boolean on both routes: package={package:?}, source={source:?}"
        ),
    }
    assert!(
        !source_meter.admitted_charges().is_empty(),
        "got.ok must read its recorded binder"
    );
    assert!(!source_meter.charge_log_truncated());
    assert!(!package_meter.charge_log_truncated());
    assert_eq!(
        package_meter.admitted_charges(),
        source_meter.admitted_charges()
    );
    let mut foreign = document.clone();
    foreign["protocol"] = json!("Foreign");
    foreign
        .as_object_mut()
        .expect("recording")
        .remove("identity");
    let foreign = serde_json::to_vec(&foreign).expect("foreign recording");
    assert!(
        fsl_analyze::protocol_analysis::recording::recording_predicate_source(
            compiled.checked(),
            &foreign,
            "Flow::Main::Decide::yes",
            2,
            &mut Meter::new(crate::counter::UNLIMITED),
        )
        .is_none()
    );
    assert!(
        fsl_analyze::protocol_analysis::recording::recording_predicate_package(
            &owner.input(),
            &foreign,
            "Flow::Main::Decide::yes",
            2,
            &mut Meter::new(crate::counter::UNLIMITED),
        )
        .is_none()
    );
}

/// The package run's whole result equals the source run's.
fn assert_equal(source: &Run, package: &Run) {
    for member in ["items", "aggregate", "last_stage", "category", "registry"] {
        assert_eq!(
            package.document()[member],
            source.document()[member],
            "{member}: {}",
            package.document()
        );
    }
    assert_eq!(package.exit, source.exit, "{}", package.document());
}

/// FR-636-AC-3, FR-850-AC-2: a supplied trace of `Placed`, `Got` and `Ship`
/// replays `proved` from the package exactly as from the source.
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn a_finite_replay_of_a_recorded_trace_settles_as_the_source_route() {
    let steps = order_steps(&[StepKind::Send, StepKind::Receive, StepKind::Finish]);
    let mut trace = trace_document("replayed", &steps);
    trace["initial_state"] = json!(0);
    let (source, package) = both_routes(
        "package-finite-replay",
        json!([{"claim": "Flow", "kind": "finite-replay",
            "trace": {"authority": "test", "identity": "replayed"}}]),
        &[("trace.json", trace)],
    );
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_equal(&source, &package);
}

/// FR-895, FR-896, FR-850-AC-2: a supplied non-empty recording is checked in
/// replay mode against its closure premises, and the package settles it
/// exactly as the source does.
#[trace("TC-2410", "FR-850-AC-2")]
#[test]
fn a_conformance_recording_settles_as_the_source_route() {
    let recording = recording_document("Flow");
    let (source, package) = both_routes(
        "package-conformance-recording",
        json!([{"claim": "Flow", "kind": "global-conformance",
            "recording": {"authority": "test", "identity": "recording"}}]),
        &[("recording.json", recording)],
    );
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_eq!(source.item(0)["global"]["complete"], true);
    assert_equal(&source, &package);
}

/// FR-125-AC-13 and FR-990: the source and checked-package conformance routes
/// both report the hand-computed NoInstance cause for an empty admitted set.
#[trace("TC-2410", "FR-125-AC-13", "FR-990-AC-11")]
#[test]
fn empty_protocol_over_settles_no_instance_in_both_routes() {
    let snapshots: Vec<Vec<u8>> = crate::litmus::Documents::new()
        .snapshots
        .into_values()
        .collect();
    let mut initial: Value =
        serde_json::from_slice(snapshots.first().expect("one initial snapshot"))
            .expect("snapshot JSON");
    initial["identity"]["identity"] = json!("s0");
    initial["populations"][2]["objects"] = json!([]);
    let mut universe = initial.clone();
    universe["identity"]["identity"] = json!("u0");
    universe["populations"][2]["objects"] = json!([
        {"key": "q", "type": "ix://test/litmus/Core", "fields": {
            "r1": {"integer": "3"}, "r2": {"integer": "3"},
            "r3": {"integer": "3"}, "r4": {"integer": "3"}
        }}
    ]);
    let snapshots = [
        serde_json::to_vec(&initial).expect("initial snapshot JSON"),
        serde_json::to_vec(&universe).expect("universe snapshot JSON"),
    ];
    let (source, package) = both_routes_with_snapshots(
        "package-no-instance",
        json!([{"claim": "Flow", "kind": "global-conformance"}]),
        &snapshots,
        json!({
            "initial_states": [{"authority": "test", "identity": "s0"}],
            "universes": [{"authority": "test", "identity": "u0"}]
        }),
    );
    for run in [&source, &package] {
        let item = run.item(0);
        assert_eq!(
            item["record"]["result"].as_str(),
            Some("inconclusive"),
            "{}",
            run.document()
        );
        assert_eq!(item["record"]["cause"]["kind"], "no-instance", "{item}");
        for member in [
            "obligations",
            "closure",
            "global",
            "projections",
            "counterexample",
        ] {
            assert!(item[member].is_null(), "NoInstance omits {member}: {item}");
        }
    }
    assert_equal(&source, &package);

    let (source, package) = both_routes(
        "package-admitted-control",
        json!([{"claim": "Flow", "kind": "global-conformance"}]),
        &[],
    );
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_eq!(label(&package.item(0)), "proved", "{}", package.document());
    assert_equal(&source, &package);
}

fn bound_protocol_subject(key: &str) -> Value {
    let model = json!({"type": "model-binding", "selection": "test/litmus"});
    json!({"protocol": {"protocol": "Flow",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": key}},
        "roles": {"Buyer": model, "Seller": model}}})
}

/// FR-896-AC-7/8: the unbound claim checks every admitted key, and a bound
/// claim checks the selected key. p satisfies `Ready`; q does not.
#[trace("TC-2410", "FR-896-AC-7", "FR-896-AC-8", "FR-850-AC-2")]
#[test]
fn source_and_package_global_conformance_check_the_expected_instances() {
    let protocol = order_protocol(true).replace(
        "run sequence Main {",
        "run sequence Main {\ncheck Ready using pp { k.r1 = 0 };",
    );
    let snapshots = keyed_core_snapshots("1");
    let items = json!([{"claim": "Flow", "kind": "global-conformance"}]);
    let (source, package) = both_routes_for_unit_with_snapshots(
        "package-every-key",
        &protocol,
        items.clone(),
        &snapshots,
        None,
    );
    assert_eq!(label(&source.item(0)), "refuted", "q violates Ready");
    assert_eq!(label(&package.item(0)), "refuted", "q violates Ready");
    assert_equal(&source, &package);

    for (key, expected) in [("p", "proved"), ("q", "refuted")] {
        let (source, package) = both_routes_for_unit_with_snapshots(
            &format!("package-bound-{key}"),
            &protocol,
            items.clone(),
            &snapshots,
            Some(bound_protocol_subject(key)),
        );
        assert_eq!(label(&source.item(0)), expected, "bound to {key}");
        assert_eq!(label(&package.item(0)), expected, "bound to {key}");
        assert_equal(&source, &package);
    }
}

/// FR-125-AC-13: an empty initial list settles NoInitialState on both routes.
#[trace("TC-2410", "FR-125-AC-13", "FR-990-AC-11", "FR-850-AC-2")]
#[test]
fn no_initial_protocol_over_settles_on_source_and_package_routes() {
    let (source, package) = both_routes_with_snapshots(
        "package-no-initial",
        json!([{"claim": "Flow", "kind": "global-conformance"}]),
        &[],
        json!({"initial_states": [], "universes": []}),
    );
    for run in [&source, &package] {
        let item = run.item(0);
        assert_eq!(
            item["record"]["result"],
            "inconclusive",
            "{}",
            run.document()
        );
        assert_eq!(
            item["record"]["cause"]["kind"], "no-initial-state",
            "{item}"
        );
    }
    assert_equal(&source, &package);
}

/// FR-896 scalar `over` ranges over Boolean values, including on the package route.
#[trace("TC-2410", "FR-896-AC-7", "FR-850-AC-2")]
#[test]
fn scalar_protocol_over_is_checked_on_source_and_package_routes() {
    let protocol = order_protocol(true).replace("over (k: M::Core)", "over (n: Boolean)");
    let (source, package) = both_routes_for_unit_with_snapshots(
        "package-scalar-over",
        &protocol,
        json!([{"claim": "Flow", "kind": "global-conformance"}]),
        &[],
        None,
    );
    assert_eq!(label(&source.item(0)), "proved", "{}", source.document());
    assert_eq!(label(&package.item(0)), "proved", "{}", package.document());
    assert_equal(&source, &package);
}

/// FR-896-AC-7 and FR-101: the max-state budget is shared by all object keys.
/// One run fits in 20 product states; two such runs do not.
#[trace("TC-2410", "FR-896-AC-7", "FR-101-AC-15", "FR-850-AC-2")]
#[test]
fn global_conformance_shares_limits_across_object_keys() {
    let snapshots = keyed_core_snapshots("0");
    let mut one_key = snapshots.clone();
    for bytes in &mut one_key {
        let mut snapshot: Value = serde_json::from_slice(bytes).expect("snapshot JSON");
        for population in snapshot["populations"]
            .as_array_mut()
            .expect("snapshot populations")
        {
            if population["population"].to_string().contains("cores") {
                population["objects"]
                    .as_array_mut()
                    .expect("core objects")
                    .truncate(1);
            }
        }
        *bytes = serde_json::to_vec(&snapshot).expect("snapshot JSON");
    }
    let item = json!([{"claim": "Flow", "kind": "global-conformance",
        "temporal": {"limits": {"max_states": 20}}}]);
    let (source_one, package_one) = both_routes_for_unit_with_snapshots(
        "package-one-key-budget",
        &order_protocol(true),
        item.clone(),
        &one_key,
        None,
    );
    assert_eq!(
        label(&source_one.item(0)),
        "proved",
        "{}",
        source_one.document()
    );
    assert_eq!(
        label(&package_one.item(0)),
        "proved",
        "{}",
        package_one.document()
    );
    assert_equal(&source_one, &package_one);

    let (source_two, package_two) = both_routes_for_unit_with_snapshots(
        "package-two-key-budget",
        &order_protocol(true),
        item,
        &snapshots,
        None,
    );
    assert_eq!(
        label(&source_two.item(0)),
        "incomplete",
        "{}",
        source_two.document()
    );
    assert_eq!(
        label(&package_two.item(0)),
        "incomplete",
        "{}",
        package_two.document()
    );
    assert_equal(&source_two, &package_two);
}

fn recording_document(protocol: &str) -> Value {
    json!({
        "format": "filament.protocol-recording/1",
        "protocol": protocol,
        "observations": [
            {"kind": "send", "node": format!("{protocol}::Main::Placed"), "role": "Buyer",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
             "record": {"ok": true}},
            {"kind": "receive", "node": format!("{protocol}::Main::Got"), "role": "Seller",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
             "record": {"ok": true}},
            {"kind": "case", "node": format!("{protocol}::Main::Decide::yes"), "role": "Seller",
             "instance": "w-1", "keys": {}},
            {"kind": "finish", "node": format!("{protocol}::Done"), "role": "Seller",
             "instance": "w-1", "keys": {}, "record": {"zz": true}}
        ],
        "local_order": {"Seller": [1, 2, 3], "Buyer": [0]},
        "correlations": [{"send": 0, "receive": 1}],
        "identity": {"authority": "test", "identity": "recording"},
        "closure": {"state": "closed", "premises": [
            {"kind": "execution", "node": format!("{protocol}::Done")},
            {"kind": "branch", "node": format!("{protocol}::Main::Decide::no")},
            {"kind": "branch", "node": format!("{protocol}::Main::Decide::yes")}
        ]}
    })
}

/// FR-895 Admission: an admitted recording of another declared protocol
/// cannot settle the selected claim, even when its own closed execution holds.
#[trace("FR-895", "FR-850-AC-2", "FR-940-AC-6")]
#[test]
fn a_recording_of_another_declared_protocol_refuses_on_both_routes() {
    let flow = order_protocol(true);
    let (_, other) = flow.split_once("protocol Flow").expect("Flow declaration");
    let body = format!("{flow}\nprotocol Other{other}");
    let ws = Workspace::new("package-other-declared-recording");
    let path = litmus_request(
        &ws,
        &body,
        json!({"items": [{"claim": "Other", "kind": "global-conformance",
            "recording": {"authority": "test", "identity": "recording"}}]}),
    );
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("request JSON");
    ws.write(
        "recording.json",
        recording_document("Other").to_string().as_bytes(),
    );
    request["request"]["documents"]
        .as_array_mut()
        .expect("documents")
        .push(json!({"file": "recording.json"}));
    ws.write("analyze.json", request.to_string().as_bytes());
    let own = analyze(&path);
    assert_eq!(own.exit, 0, "{}", own.document());
    assert_eq!(label(&own.item(0)), "proved", "{}", own.document());
    assert_eq!(own.item(0)["global"]["complete"], true);

    request["request"]["items"][0]["claim"] = json!("Flow");
    ws.write("analyze.json", request.to_string().as_bytes());
    let source = analyze(&path);
    assert_eq!(source.exit, 20, "{}", source.document());
    let item = source.item(0);
    assert_eq!(item["claim_name"], "Flow", "{item}");
    assert_eq!(label(&item), "declined", "{item}");
    assert_eq!(
        item["record"]["diagnostic"]["code"], "invalid_runtime_input",
        "{item}"
    );
    assert_eq!(
        item["record"]["diagnostic"]["cause"], "invalid-value",
        "{item}"
    );
    assert!(item.get("global").is_none(), "{item}");
    assert!(item.get("obligations").is_none(), "{item}");

    let (members, _) = package_selection(&ws, &request["request"]);
    std::fs::remove_file(ws.path("program.fsl")).expect("delete source program");
    let sources = ws.path(&format!("{HANDOFF}/sources"));
    std::fs::remove_dir_all(&sources).expect("delete retained source files");
    assert!(!ws.path("program.fsl").exists());
    assert!(!sources.exists());
    let package = analyze(
        &ws.write(
            "other-protocol-package.json",
            json!({
                "format": "native-analyze/1", "request": members,
            })
            .to_string()
            .as_bytes(),
        ),
    );
    assert_equal(&source, &package);
    assert_eq!(
        package.stdout, source.stdout,
        "whole selected-protocol refusal"
    );
}

fn refused_routes(name: &str, item: Value, file: &str, document: &Value) -> (Run, Run) {
    let ws = Workspace::new(name);
    let path = litmus_request(&ws, &order_protocol(true), json!({"items": [item]}));
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("JSON");
    ws.write(file, document.to_string().as_bytes());
    request["request"]["documents"]
        .as_array_mut()
        .expect("documents")
        .push(json!({"file": file}));
    ws.write("analyze.json", request.to_string().as_bytes());
    let source = analyze(&path);
    assert_eq!(source.exit, 20, "{}", source.document());
    let (members, _) = package_selection(&ws, &request["request"]);
    std::fs::remove_file(ws.path("program.fsl")).expect("delete source program");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("delete retained source files");
    assert!(!ws.path("program.fsl").exists());
    assert!(!ws.path(&format!("{HANDOFF}/sources")).exists());
    let package = analyze(
        &ws.write(
            "refused-package.json",
            json!({"format": "native-analyze/1", "request": members})
                .to_string()
                .as_bytes(),
        ),
    );
    (source, package)
}

/// An authentic transition identity does not authenticate a forged post-state.
#[trace("FR-941", "FR-850-AC-2", "FR-940-AC-6")]
#[test]
fn a_package_finite_replay_rejects_a_foreign_post_state_as_the_source_route() {
    let steps = order_steps(&[StepKind::Send, StepKind::Receive, StepKind::Finish]);
    let mut trace = trace_document("stale", &steps);
    trace["initial_state"] = json!(0);
    trace["prefix"][0]["post_state"]["key"] = json!("Zm9yZWlnbi1zdGF0ZQ");
    let (source, package) = refused_routes(
        "package-stale-protocol-state",
        json!({"claim": "Flow", "kind": "finite-replay", "trace": {"authority": "test", "identity": "stale"}}),
        "trace.json",
        &trace,
    );
    assert_equal(&source, &package);
    assert_eq!(package.stdout, source.stdout, "whole stale-step refusal");
}

/// Recording identity never substitutes for membership in the admitted package.
#[trace("FR-895", "FR-850-AC-2", "FR-940-AC-6")]
#[test]
fn a_package_recording_rejects_a_foreign_protocol_as_the_source_route() {
    let recording = json!({"format": "filament.protocol-recording/1", "protocol": "Foreign",
        "identity": {"authority": "test", "identity": "foreign"},
        "observations": [], "local_order": {}, "correlations": [],
        "closure": {"state": "open", "premises": []}});
    let (source, package) = refused_routes(
        "package-foreign-recording",
        json!({"claim": "Flow", "kind": "global-conformance", "recording": {"authority": "test", "identity": "foreign"}}),
        "recording.json",
        &recording,
    );
    assert_equal(&source, &package);
    assert_eq!(
        package.stdout, source.stdout,
        "whole foreign-protocol refusal"
    );
}

/// FR-895-AC-36 (TC-2531): a protocol with a `parallel` has no package form
/// yet, so over a package read from its bytes a predicate join settles
/// `unsupported`, `unsupported_construct`/`not-yet-implemented`, where the
/// source route assesses the recording; the package route never reports the
/// recording as open.
#[trace("TC-2531", "FR-895-AC-36")]
#[test]
fn a_predicate_join_over_a_package_settles_unsupported() {
    let protocol = format!(
        "function both using v(left: Boolean, right: Boolean): Boolean pure {{ left and right }}\n{}",
        order_protocol(false).replace(
            "run sequence Main {",
            "run sequence Main { parallel Both memory sc { branch left sequence Left { event L by Buyer as (l: Ack) { true }; } branch right sequence Right { event R by Buyer as (r: Ack) { true }; } } join predicate(both) [left, right] outstanding continue;",
        )
    );
    let recording = json!({
        "format": "filament.protocol-recording/1", "protocol": "Flow",
        "observations": [], "local_order": {}, "correlations": [],
        "identity": {"authority": "test", "identity": "recording"},
        "closure": {"state": "open", "premises": []}
    });
    let ws = Workspace::new("package-predicate-join");
    let items = json!([{"claim": "Flow", "kind": "global-conformance",
        "recording": {"authority": "test", "identity": "recording"}}]);
    let path = litmus_request(&ws, &protocol, json!({"items": items}));
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("request")).expect("json");
    ws.write("recording.json", recording.to_string().as_bytes());
    request["request"]["documents"]
        .as_array_mut()
        .expect("documents")
        .push(json!({"file": "recording.json"}));
    ws.write("analyze.json", request.to_string().as_bytes());
    let source = analyze(&path);
    assert_eq!(
        label(&source.item(0)),
        "inconclusive",
        "{}",
        source.document()
    );
    let (members, _) = package_selection(&ws, &request["request"]);
    std::fs::remove_file(ws.path("program.fsl")).expect("the authored unit is removed");
    let sources = ws.path(&format!("{HANDOFF}/sources"));
    std::fs::remove_dir_all(&sources).expect("the handoff sources are removed");
    let document = json!({"format": "native-analyze/1", "request": members});
    let package = analyze(&ws.write("analyze-package.json", document.to_string().as_bytes()));
    let settled = package.document();
    assert_eq!(settled["category"], "unsupported", "{settled}");
    let diagnostic = &settled["diagnostics"][0];
    assert_eq!(diagnostic["code"], "unsupported_construct", "{settled}");
    assert_eq!(diagnostic["cause"], "not-yet-implemented", "{settled}");
}
