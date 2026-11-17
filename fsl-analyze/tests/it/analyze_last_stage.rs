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

const PROVIDER: &str = "fsl.test-last-stage-provider";

/// A registered provider outside FSL that proves every item it is sent.
#[derive(Debug)]
struct Proving;

impl fsl_analyze::provider::ExternalProvider for Proving {
    fn settle(
        &self,
        request: &fsl_analyze::provider::ProviderRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
        vec![fsl_analyze::provider::ProviderVerdict::Proved; request.items().len()]
    }
}

fn proving_provider() -> fsl_analyze::analyze::Engine {
    use fsl_semantics::check::Capability;
    let descriptor = fsl_route::descriptor_from_manifest(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new(PROVIDER)),
        fsl_route::ProviderOrigin::Process,
        vec![(
            Some(Capability::ValueValidity.to_wire()),
            Some(fsl_route::Mode::Unbounded.to_wire()),
        )],
        fsl_route::ManifestMembers {
            domains: Some(&[fsl_foundation::bound::DomainKind::Integer.to_wire()]),
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .expect("the descriptor advertises the external value arm");
    fsl_analyze::analyze::Engine::external(
        descriptor,
        fsl_analyze::analyze::BackendTool::new("test-provider", "1").expect("a tool"),
        Box::new(Proving),
    )
    .expect("the descriptor is unbounded")
}

const EVEN: &str = "function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n";

/// FR-621 "Last stage": an item settled by a negotiated backend outside FSL
/// names `S6b`, as does an item its negotiation settles `unsupported`; an
/// in-process engine's item names `S6c`.
#[trace("TC-2381", "FR-621-AC-11")]
#[test]
fn negotiated_settlements_name_s6b() {
    let ws = Workspace::new("ls-provider");
    let request = counter_request(
        &ws,
        EVEN,
        json!([{"claim": "Even", "kind": "value-validity", "backend": PROVIDER}]),
    );
    let run =
        crate::analyze_cli_support::analyze_with_providers(&request, vec![proving_provider()]);
    assert_eq!(
        crate::analyze_cli_support::label(&run.item(0)),
        "proved",
        "{}",
        run.item(0)
    );
    assert_eq!(run.document()["last_stage"], "S6b", "{}", run.document());

    let ws = Workspace::new("ls-negotiated-unsupported");
    let request = counter_request(
        &ws,
        &bounded(),
        json!([{"claim": "Bounded", "backend": PROVIDER}]),
    );
    let run =
        crate::analyze_cli_support::analyze_with_providers(&request, vec![proving_provider()]);
    assert_eq!(
        run.item(0)["record"]["result"],
        "unsupported",
        "{}",
        run.item(0)
    );
    assert_eq!(
        run.item(0)["record"]["settled_by"],
        "disposition",
        "{}",
        run.item(0)
    );
    assert_eq!(run.document()["last_stage"], "S6b", "{}", run.document());

    let ws = Workspace::new("ls-in-process");
    let request = counter_request(&ws, &bounded(), json!([{"claim": "Bounded"}]));
    let run =
        crate::analyze_cli_support::analyze_with_providers(&request, vec![proving_provider()]);
    assert_eq!(run.document()["last_stage"], "S6c", "{}", run.document());
}
