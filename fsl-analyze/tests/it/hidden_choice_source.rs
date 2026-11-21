// SPDX-License-Identifier: AGPL-3.0-or-later
//! The lock example's hidden choice through native Source acquisition.

use ix_trace_rs::trace;

use crate::analyze_cli_support::{analyze, label, Workspace};

/// Holder knows the received decision; Observer cannot distinguish receiving
/// the accepted branch's notice from ending in the empty denied branch.
#[trace("TC-1411", "FR-571-AC-2", "FR-573-AC-2", "FR-574-AC-1", "FR-286-AC-36")]
#[test]
fn hidden_choice_source_refutes_with_a_counterexample() {
    let ws = Workspace::new("hidden-choice-source");
    for (path, bytes) in [
        (
            "model/lock.semantic-ir.json",
            &include_bytes!("../fixtures/hidden-choice-source/model/lock.semantic-ir.json")[..],
        ),
        (
            "data/lock-free.json",
            &include_bytes!("../fixtures/hidden-choice-source/data/lock-free.json")[..],
        ),
        (
            "rules/protocols/handover-hidden-choice.fsl",
            &include_bytes!(
                "../fixtures/hidden-choice-source/rules/protocols/handover-hidden-choice.fsl"
            )[..],
        ),
        (
            "requests/protocols/protocol-unrealizable.json",
            &include_bytes!(
                "../fixtures/hidden-choice-source/requests/protocols/protocol-unrealizable.json"
            )[..],
        ),
    ] {
        ws.write(path, bytes);
    }
    let run = analyze(&ws.path("requests/protocols/protocol-unrealizable.json"));
    let document = run.document();
    eprintln!("hidden-choice Source exit={} outcome={document}", run.exit);
    assert_eq!(
        run.exit, 10,
        "valid hidden-choice Source must refute: {document}"
    );
    let items = document["items"].as_array().expect("analyze outcome items");
    assert_eq!(
        items.len(),
        1,
        "one requested realizability item: {document}"
    );
    let item = &items[0];
    assert_eq!(label(item), "refuted", "{document}");
    assert!(
        item["counterexample"].is_object(),
        "a refutation carries its counterexample: {document}"
    );
    let entry = &item["counterexample"];
    assert_eq!(entry["request_index"], 0);
    assert_eq!(
        entry["backend"],
        fsl_analyze::protocol_analysis::manifest::BACKEND_ID
    );
    let body = &entry["counterexample"];
    assert_eq!(
        body["type"], "protocol-analysis-counterexample",
        "{document}"
    );
    assert_eq!(body["kind"], "indistinguishable-histories", "{document}");
    assert_eq!(body["key"]["kind"], "realizability", "{document}");
    for member in [
        "role",
        "history_a",
        "history_b",
        "action_a",
        "action_b",
        "obligation",
        "separating_witness",
    ] {
        assert!(body[member].is_object(), "missing {member}: {document}");
    }
    assert_ne!(body["history_a"], body["history_b"], "{document}");
    assert_ne!(body["action_a"], body["action_b"], "{document}");
}
