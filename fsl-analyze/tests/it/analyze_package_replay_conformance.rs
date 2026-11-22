// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 "Equality with the source route", FR-940, FR-636 (QSL-2785): a
//! `finite-replay` item over a supplied protocol trace and a
//! `global-conformance` item over a supplied recording settle from the
//! package bytes exactly as they settle from the authored source.

use fsl_eval::simulation::protocol::StepKind;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, litmus_request, order_protocol, package_selection, Run, Workspace, HANDOFF,
};
use crate::protocol_composition::{order_steps, trace_document};

/// The source run of `items` over the order protocol with `documents`
/// provisioned, then the package run of the same request once the authored
/// unit and the handoff's `sources/` directory are deleted.
fn both_routes(name: &str, items: Value, documents: &[(&str, Value)]) -> (Run, Run) {
    let ws = Workspace::new(name);
    let path = litmus_request(&ws, &order_protocol(true), json!({"items": items}));
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
    let (members, _) = package_selection(&ws, &request["request"]);
    std::fs::remove_file(ws.path("program.fsl")).expect("the authored unit is removed");
    let sources = ws.path(&format!("{HANDOFF}/sources"));
    std::fs::remove_dir_all(&sources).expect("the handoff sources are removed");
    assert!(!sources.exists(), "the handoff sources are gone");
    let document = json!({"format": "native-analyze/1", "request": members});
    let package = analyze(&ws.write("analyze-package.json", document.to_string().as_bytes()));
    (source, package)
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
    let recording = json!({
        "format": "filament.protocol-recording/1",
        "protocol": "Flow",
        "observations": [
            {"kind": "send", "node": "Flow::Main::Placed", "role": "Buyer",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
             "record": {"ok": true}},
            {"kind": "receive", "node": "Flow::Main::Got", "role": "Seller",
             "instance": "w-1", "keys": {"message": "m-1", "delivery": "d-1"},
             "record": {"ok": true}},
            {"kind": "case", "node": "Flow::Main::Decide::yes", "role": "Seller",
             "instance": "w-1", "keys": {}},
            {"kind": "finish", "node": "Flow::Done", "role": "Seller",
             "instance": "w-1", "keys": {}, "record": {"zz": true}}
        ],
        "local_order": {"Seller": [1, 2, 3], "Buyer": [0]},
        "correlations": [{"send": 0, "receive": 1}],
        "identity": {"authority": "test", "identity": "recording"},
        "closure": {"state": "closed", "premises": [
            {"kind": "execution", "node": "Flow::Done"},
            {"kind": "branch", "node": "Flow::Main::Decide::no"},
            {"kind": "branch", "node": "Flow::Main::Decide::yes"}
        ]}
    });
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
