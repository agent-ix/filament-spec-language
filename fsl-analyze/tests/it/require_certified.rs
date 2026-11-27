// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1052 (TC-3240): a `native-analyze/1` request with `require_certified`
//! settles a proof that is not `certified` `inconclusive`,
//! `proof-not-certified`, and leaves every other result as it is.

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, analyze_with_providers, below_three, bounded, counter_request_with, label,
    litmus_request, order_protocol, request_members, Run, Workspace,
};
use crate::analyze_family_cli::{process_provider, PROCESS_PROVIDER};
use crate::protocol_conformance::{order_premises, order_recording, recording_member};

/// The Counter request of `items` with `require_certified` set to `required`,
/// analyzed with a process provider that answers `proved`.
fn counter_run(name: &str, body: &str, items: Value, required: bool) -> Run {
    use fsl_analyze::provider::ProviderVerdict;
    let ws = Workspace::new(name);
    let mut members = request_members(Some(items));
    members["require_certified"] = json!(required);
    analyze_with_providers(
        &counter_request_with(&ws, body, members),
        vec![process_provider(PROCESS_PROVIDER, ProviderVerdict::Proved)],
    )
}

/// The state-constraint proof and the process provider's proof are not
/// certified; the closure-certificate proof beside them is, and stays.
#[trace("TC-3240", "FR-1052-AC-3", "FR-1052-AC-4", "FR-1052-AC-5")]
#[test]
fn constrained_and_trusted_proofs_are_withheld_and_a_certified_proof_stays() {
    let body = format!(
        "function Even using v(n: Integer): Boolean pure {{ n * 2 - n = n }}\n\
         constraint Wide using v on M::Counter at current {{ self.value <= 10 }}\n{}",
        bounded()
    );
    let items = json!([
        {"claim": "Bounded"},
        {"claim": "Bounded", "reductions": {"constraint": "Wide"}},
        {"claim": "Even", "kind": "value-validity", "backend": PROCESS_PROVIDER},
    ]);
    let default = counter_run("require-1052-3-default", &body, items.clone(), false);
    let required = counter_run("require-1052-3-required", &body, items, true);
    for (run, index) in [(&default, 0), (&required, 0)] {
        let item = run.item(index);
        assert_eq!(label(&item), "proved", "{item}");
        assert_eq!(item["record"]["certification"], "certified", "{item}");
    }
    for (index, certification) in [(1, "uncertified"), (2, "trusted")] {
        let proved = default.item(index);
        assert_eq!(label(&proved), "proved", "{proved}");
        assert_eq!(proved["record"]["certification"], certification, "{proved}");
        let withheld = required.item(index);
        assert_eq!(label(&withheld), "inconclusive", "{withheld}");
        assert!(
            withheld["record"].get("certification").is_none(),
            "{withheld}"
        );
        let cause = &withheld["record"]["cause"];
        assert_eq!(cause["kind"], "proof-not-certified", "{withheld}");
        assert_eq!(cause["certification"], certification, "{withheld}");
        assert_eq!(cause["basis"], proved["record"]["basis"], "{withheld}");
    }
    assert_eq!(
        default.item(1)["record"]["basis"]["type"],
        "exhaustive",
        "{}",
        default.item(1)
    );
}

#[trace("TC-3240", "FR-1052-AC-5")]
#[test]
fn results_that_are_not_proofs_are_unchanged_by_the_policy() {
    let body = format!("{}{}", bounded(), below_three());
    let items = json!([
        {"claim": "BelowThree"},
        {"claim": "Bounded", "temporal": {"limits": {"max_states": 1}}},
        {"claim": "Missing"},
    ]);
    let default = counter_run("require-1052-5-default", &body, items.clone(), false);
    let required = counter_run("require-1052-5-required", &body, items, true);
    assert_eq!(label(&default.item(0)), "refuted");
    assert_eq!(label(&default.item(1)), "incomplete");
    assert_eq!(default.item(2)["category"], "refusal");
    assert_eq!(default.document()["items"], required.document()["items"]);
    assert_eq!(default.exit, required.exit);
}

#[trace("TC-3240", "FR-1052-AC-7")]
#[test]
fn a_require_certified_that_is_not_a_boolean_refuses_the_request() {
    for (name, value) in [
        ("require-1052-7-string", json!("yes")),
        ("require-1052-7-number", json!(1)),
    ] {
        let ws = Workspace::new(name);
        let mut members = request_members(Some(json!([{"claim": "Bounded"}])));
        members["require_certified"] = value;
        let run = analyze(&counter_request_with(&ws, &bounded(), members));
        let document = run.document();
        assert_eq!(run.exit, 20, "{document}");
        assert_eq!(
            document["diagnostics"][0]["code"], "invalid-request",
            "{document}"
        );
        assert!(
            document["diagnostics"][0]
                .to_string()
                .contains("require_certified"),
            "{document}"
        );
        assert!(
            document["items"].as_array().is_none_or(Vec::is_empty),
            "{document}"
        );
    }
}

/// A conformance verdict over a recording has no core checker; one the
/// engine closed carries a certified record and stays.
#[trace("TC-3240", "FR-1052-AC-9")]
#[test]
fn a_recording_conformance_proof_is_withheld_when_certification_is_required() {
    let recorded = |name: &str, required: bool| {
        let ws = Workspace::new(name);
        let recording = order_recording(json!({"state": "closed", "premises": order_premises()}));
        ws.write("recording.json", recording.to_string().as_bytes());
        let mut item = json!({"claim": "Flow", "kind": "global-conformance"});
        item["recording"] = recording_member()["recording"].clone();
        let extra = json!({"items": [item], "documents": [{"file": "recording.json"}],
            "require_certified": required});
        analyze(&litmus_request(&ws, &order_protocol(true), extra)).item(0)
    };
    let default = recorded("require-1052-9-default", false);
    assert_eq!(default["global"]["label"], "proved", "{default}");
    assert_eq!(
        default["record"]["certification"], "uncertified",
        "{default}"
    );
    let required = recorded("require-1052-9-required", true);
    assert_eq!(
        required["global"],
        json!({"label": "inconclusive", "basis": "unsettled", "complete": false,
            "cause": "proof-not-certified"}),
        "{required}"
    );
    assert_eq!(required["closure"], default["closure"], "{required}");
    assert_eq!(
        required["obligations"], default["obligations"],
        "{required}"
    );
    assert_eq!(label(&required), "inconclusive", "{required}");

    let engine = |name: &str, required: bool, delivery: &str, receive: bool| {
        let ws = Workspace::new(name);
        let extra = json!({"items": [{"claim": "Flow", "kind": "global-conformance"}],
            "require_certified": required});
        let protocol = order_protocol(receive).replace("exactly-once-premise", delivery);
        analyze(&litmus_request(&ws, &protocol, extra)).item(0)
    };
    // At-most-once and no `requires temporal`: the core closure check covers
    // every obligation, so the proof is `certified` and unchanged.
    let closed = engine("require-1052-9-engine", true, "at-most-once", false);
    assert_eq!(closed["record"]["certification"], "certified", "{closed}");
    assert_eq!(closed["global"]["label"], "proved", "{closed}");
    assert_eq!(
        closed,
        engine("require-1052-9-engine-default", false, "at-most-once", false)
    );
    // FR-896-AC-11: an `exactly-once-premise` channel is not read by the
    // closure check, so the proof is `uncertified` and withheld.
    let premise = engine("require-1052-9-premise", true, "exactly-once-premise", true);
    assert_eq!(premise["global"]["label"], "inconclusive", "{premise}");
}

/// A global-conformance claim over a protocol with an
/// `exactly-once-premise` channel, every obligation satisfied: the default
/// run is `proved`, `uncertified`; with `require_certified` it is withheld.
#[trace("TC-3240", "FR-1052-AC-16")]
#[test]
fn a_premise_channel_conformance_proof_is_uncertified_and_withheld() {
    let run = |name: &str, required: bool| {
        let ws = Workspace::new(name);
        let extra = json!({"items": [{"claim": "Flow", "kind": "global-conformance"}],
            "require_certified": required});
        analyze(&litmus_request(&ws, &order_protocol(true), extra)).item(0)
    };
    let default = run("require-1052-16-default", false);
    assert_eq!(default["global"]["label"], "proved", "{default}");
    assert_eq!(
        default["record"]["certification"], "uncertified",
        "{default}"
    );
    assert!(
        obligations_all_satisfied(&default),
        "every obligation is satisfied: {default}"
    );
    let required = run("require-1052-16-required", true);
    assert_eq!(
        required["global"],
        json!({"label": "inconclusive", "basis": "unsettled", "complete": false,
            "cause": "proof-not-certified"}),
        "{required}"
    );
}

fn obligations_all_satisfied(item: &Value) -> bool {
    item["obligations"]
        .as_array()
        .is_some_and(|all| !all.is_empty() && all.iter().all(|entry| entry["verdict"] == "satisfied"))
}
