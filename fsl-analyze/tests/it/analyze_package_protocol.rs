// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940-AC-6: protocol litmus and memory-model items settle equally from a
//! source-checked package and its admitted handoff bytes.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, label, litmus_request, package_selection, Run, Workspace,
};
use crate::litmus::{sb_run, sb_unit};

fn subject() -> Value {
    let binding = json!({"type": "model-binding", "selection": "test/litmus"});
    json!({"protocol": {
        "protocol": "SB",
        "over": {"reference": {"population": "ix://test/litmus/cores", "key": "core"}},
        "roles": {"w": binding, "c": binding},
    }})
}

fn request_extra(items: Value) -> Value {
    json!({
        "subject": subject(),
        "memory": [{"parallel": "SB::Both", "model": "tso"}],
        "items": items,
    })
}

fn request_without_memory(items: Value) -> Value {
    json!({
        "subject": subject(),
        "items": items,
    })
}

fn package_run(ws: &Workspace, request_path: &std::path::Path) -> Run {
    let document: Value =
        serde_json::from_slice(&std::fs::read(request_path).expect("the source request exists"))
            .expect("the source request is JSON");
    let (members, _) = package_selection(ws, &document["request"]);
    let package_document = json!({"format": "native-analyze/1", "request": members});
    analyze(&ws.write(
        "analyze-package.json",
        package_document.to_string().as_bytes(),
    ))
}

/// FR-940-AC-6: the store-buffering litmus claim under TSO has equal records,
/// counterexamples and package-byte replay on both analyze routes.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_tso_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "release", "acquire"));
    let ws = Workspace::new("package-litmus-tso");
    let source_request =
        litmus_request(&ws, &body, request_extra(json!([{"claim": "NoBothZero"}])));
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
    assert_eq!(
        package.item(0)["counterexample"],
        source.item(0)["counterexample"]
    );
    assert_eq!(package.item(0)["replay"], source.item(0)["replay"]);
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
}

/// FR-940-AC-6: the protocol's race-freedom result reads the same admitted
/// protocol and memory-model facts on the package route.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn the_tso_race_freedom_item_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "", ""));
    let ws = Workspace::new("package-race-tso");
    let source_request = litmus_request(
        &ws,
        &body,
        request_extra(json!([{
            "claim": "SB::Both",
            "temporal": {"item_kind": "race-freedom"},
        }])),
    );
    let source = analyze(&source_request);
    assert_eq!(label(&source.item(0)), "refuted", "{}", source.document());
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(label(&package.item(0)), "refuted", "{}", package.document());
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
    assert_eq!(
        package.item(0)["counterexample"],
        source.item(0)["counterexample"]
    );
    assert_eq!(package.item(0)["replay"], source.item(0)["replay"]);
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
}

/// FR-850-AC-11: the same SB protocol with no request memory member uses its
/// source `sc` default on both analyze routes.
#[trace("TC-3050", "FR-850-AC-11")]
#[test]
fn the_default_sc_litmus_claim_has_source_and_package_route_parity() {
    let body = sb_unit(&sb_run("", "", ""));
    let ws = Workspace::new("package-litmus-default-sc");
    let source_request = litmus_request(
        &ws,
        &body,
        request_without_memory(json!([{"claim": "NoBothZero"}])),
    );
    let source = analyze(&source_request);
    assert_eq!(source.item(0)["replay"]["reproduced"], true);

    let package = package_run(&ws, &source_request);
    assert_eq!(package.item(0)["record"], source.item(0)["record"]);
    assert_eq!(
        package.item(0)["counterexample"],
        source.item(0)["counterexample"]
    );
    assert_eq!(package.item(0)["replay"], source.item(0)["replay"]);
    assert_eq!(package.item(0)["replay"]["reproduced"], true);
}
