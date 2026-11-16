// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-621 "Last stage" and FR-286 "Monitor documents from clause runs": every
//! `analyze` and `monitor` document names the greatest ADR-011 stage it
//! entered, and `null` only for request intake, cancellation and faults.

use filament_exact::{Cancel, CancelCause};
use fsl_analyze::monitor::{MonitorOutcome, MonitorRefusal};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{
    analyze, below_three, bounded, counter_request, Fr290, Workspace,
};

fn last_stage(items: Value, body: &str, name: &str) -> Value {
    let ws = Workspace::new(name);
    analyze(&counter_request(&ws, body, items)).document()["last_stage"].clone()
}

#[trace("TC-2381", "FR-621-AC-11")]
#[test]
fn settled_documents_name_their_greatest_stage() {
    let body = format!("{}{}", bounded(), below_three());
    assert_eq!(
        last_stage(json!([{"claim": "Bounded"}]), &body, "ls-proved"),
        "S6c"
    );
    assert_eq!(
        last_stage(
            json!([{"claim": "Bounded"}, {"claim": "BelowThree"}]),
            &body,
            "ls-replayed"
        ),
        "S8"
    );
    assert_eq!(
        last_stage(
            json!([{"claim": "Bounded", "temporal": {"limits": {"max_states": 2}}}]),
            &body,
            "ls-incomplete"
        ),
        "S6c"
    );
    assert_eq!(
        last_stage(
            json!([{"claim": "Nothing", "temporal": {"item_kind": "deadlock-freedom"}}]),
            "",
            "ls-disposition"
        ),
        "S4"
    );
}

#[trace("TC-2382", "FR-621-AC-12")]
#[test]
fn refusals_name_their_stage() {
    let ws = Workspace::new("ls-malformed");
    let path = ws.write(
        "analyze.json",
        b"{\"format\": \"native-analyze/1\", \"request\": 7}",
    );
    assert!(analyze(&path).document()["last_stage"].is_null());

    let ws = Workspace::new("ls-check");
    let path = counter_request(
        &ws,
        "function Bad using v(): Boolean pure { 1 }\n",
        json!([]),
    );
    assert_eq!(analyze(&path).document()["last_stage"], "S3");

    let ws = Workspace::new("ls-binding");
    let path = counter_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let snapshot = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "start"},
        "observation": "current", "model": {"identity": "test/counter"},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": "ix://test/counter/nowhere", "complete": true,
            "objects": []}],
    });
    ws.write("start.json", snapshot.to_string().as_bytes());
    let run = analyze(&path);
    assert_eq!(run.exit, 20);
    assert_eq!(run.document()["last_stage"], "S4");

    let ws = Workspace::new("ls-cancel");
    let path = counter_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let document: Value = match fsl_command::command::analyze(&path, &Fr290, &cancel) {
        Ok(result) => {
            serde_json::from_str(&result.value.to_string()).expect("the document is JSON")
        }
        Err(error) => serde_json::from_slice(
            &error
                .outcome_document(fsl_replay::Operation::Analyze)
                .to_bytes()
                .expect("the outcome document serializes"),
        )
        .expect("the document is JSON"),
    };
    assert_eq!(document["category"], "incomplete", "{document}");
    assert!(document["last_stage"].is_null(), "{document}");
}

fn monitor_document(document: fsl_replay::OutcomeDocument) -> Value {
    serde_json::from_slice(&document.to_bytes().expect("the document encodes"))
        .expect("the document is JSON")
}

#[trace("TC-2383", "FR-286-AC-20")]
#[test]
fn monitor_documents_name_their_stage() {
    let settled = MonitorOutcome {
        records: Vec::new(),
        settlements: Vec::new(),
        ledger: None,
    };
    let document = monitor_document(settled.document().expect("no record to refuse"));
    assert_eq!(document["last_stage"], "S6a", "{document}");
    let unknown = monitor_document(
        MonitorRefusal::UnknownClause {
            name: "Nowhere".to_owned(),
        }
        .document(),
    );
    assert_eq!(unknown["last_stage"], "S4", "{unknown}");
    let observation = monitor_document(
        MonitorRefusal::Observation {
            member: "trace",
            cause: "invalid-value",
        }
        .document(),
    );
    assert_eq!(observation["last_stage"], "S6a", "{observation}");
    assert_eq!(observation["category"], "refusal", "{observation}");
}
