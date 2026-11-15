// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1391: the `analyze` row.

use fsl_conformance::{OutcomeKind, Verb};
use ix_trace_rs::trace;

use crate::support::{
    assert_no_counterexample, assert_one_formula_counterexample, diagnostic, outcome_of, written,
};

/// The one backend the case's one record names.
fn backend_of(outcome: &fsl_conformance::CaseOutcome) -> String {
    let [record] = outcome.records() else {
        panic!("one record: {:?}", outcome.records());
    };
    record
        .backend()
        .map(|id| id.as_str().to_owned())
        .expect("a named engine")
}

/// Each of `analyze`'s applicable cells returns its kind: the EN-5 item
/// `proved`, the EN-1 item `refuted` and `incomplete` under `max_states` 1,
/// the binding refusal, the unsupported clause item, and the cancelled EN-4
/// item. FR-564-AC-4 is not tagged here: its `ItemCounterexample` for the
/// `refuted` cell is not produced yet, so the AC stays open.
#[trace("TC-1391")]
#[test]
fn analyze_returns_each_reachable_kind() {
    let proved = outcome_of(Verb::Analyze, OutcomeKind::Success);
    assert_eq!(proved.exit_code(), 0);
    let item = &written(&proved)["items"][0];
    assert_eq!(item["result"], "proved");
    assert_eq!(backend_of(&proved), "en-5");
    assert_no_counterexample(&proved);

    let refuted = outcome_of(Verb::Analyze, OutcomeKind::Violation);
    assert_eq!(refuted.exit_code(), 10);
    let item = &written(&refuted)["items"][0];
    assert_eq!(item["result"], "refuted");
    assert_eq!(backend_of(&refuted), "en-1");
    assert_one_formula_counterexample(&refuted);

    let incomplete = outcome_of(Verb::Analyze, OutcomeKind::Incomplete);
    assert_eq!(incomplete.exit_code(), 22);
    let item = &written(&incomplete)["items"][0];
    assert_eq!(item["result"], "incomplete");
    assert_eq!(backend_of(&incomplete), "en-1");
    assert_eq!(item["cause"]["limit"], "max_states");
    assert_no_counterexample(&incomplete);

    let refusal = outcome_of(Verb::Analyze, OutcomeKind::Refusal);
    assert_eq!(refusal.exit_code(), 20);
    assert_eq!(written(&refusal)["items"], serde_json::json!([]));
    assert_eq!(
        diagnostic(&refusal),
        (
            "invalid_runtime_input".to_owned(),
            Some("wrong-role-mapping".to_owned())
        )
    );

    let unsupported = outcome_of(Verb::Analyze, OutcomeKind::Unsupported);
    assert_eq!(unsupported.exit_code(), 21);
    assert_eq!(written(&unsupported)["items"][0]["result"], "unsupported");
    // FR-281-AC-3 and FR-286: the item carries the disposition's diagnostic
    // naming the claim kind, and no cause.
    let item = &written(&unsupported)["items"][0];
    assert!(item.get("cause").is_none(), "{item}");
    assert_eq!(item["diagnostic"]["code"], "unsupported_projection");
    assert_eq!(
        item["diagnostic"]["cause"],
        "unsupported-requested-capability"
    );
    assert_eq!(
        item["diagnostic"]["detail"]["kind"],
        "temporal-satisfaction"
    );
    assert!(written(&unsupported)["diagnostics"]
        .as_array()
        .is_none_or(Vec::is_empty));

    for (kind, source) in [
        (OutcomeKind::Timeout, "deadline"),
        (OutcomeKind::Cancellation, "requested"),
    ] {
        let outcome = outcome_of(Verb::Analyze, kind);
        assert_eq!(outcome.exit_code(), 22);
        let item = &written(&outcome)["items"][0];
        assert_eq!(item["result"], "incomplete");
        assert_eq!(
            item["cause"],
            serde_json::json!({"kind": "cancelled", "source": source})
        );
    }
}

/// The FSpec FR-290 settlement the driver's `negotiate_*` gives an item, as
/// a test double: one candidate `supported`, none `unsupported`.
struct OneCandidate;

impl fsl_analyze::analyze::document::Negotiate for OneCandidate {
    fn negotiate(
        &self,
        item: &fsl_analyze::analyze::ClaimItem,
        candidates: &fsl_route::CandidateOutcome,
    ) -> (
        fsl_route::routing::Disposition,
        Option<fsl_analyze::analyze::Diagnostic>,
    ) {
        use fsl_analyze::analyze::{Diagnostic, Payload};
        use fsl_route::routing::Disposition;
        match candidates {
            fsl_route::CandidateOutcome::Candidates(set) if set.candidates().len() == 1 => {
                (Disposition::Supported(set.candidates()[0].clone()), None)
            }
            _ => (
                Disposition::Unsupported,
                Some(Diagnostic {
                    code: fsl_route::NegotiationOutcome::UnsupportedRequestedCapability.code(),
                    payload: Payload::Kind(item.kind),
                }),
            ),
        }
    }
}

/// The Counter-step subject of FR-564 with the holding clause `Q` beside
/// `P`, written with a `native-analyze/1` request of `items` and the extra
/// request members `extra`; the analyze document and its exit code under
/// `cancel`.
fn analyze_wire(
    items: serde_json::Value,
    extra: serde_json::Value,
    cancel: &filament_exact::Cancel,
) -> (serde_json::Value, u8) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let unit = format!(
        "{}temporal Q using t0 over (c: M::Counter) clock \"model-steps\" on origin {{ always holds(c.value <= 3) }}\n",
        std::str::from_utf8(include_bytes!("../../cases/counter-step.native")).expect("UTF-8")
    );
    std::fs::write(dir.path().join("unit.native"), unit).expect("the unit");
    std::fs::write(
        dir.path().join("package.json"),
        include_bytes!("../../cases/counter-step-package.json"),
    )
    .expect("the package");
    std::fs::write(
        dir.path().join("v0.json"),
        include_bytes!("../../cases/counter-step-v0.json"),
    )
    .expect("the snapshot");
    let mut request = serde_json::json!({
        "program": {"source": {"file": "unit.native", "authority": "test",
            "identity": "counter-step", "document": "CounterStep", "formal_revision": 1}},
        "models": [{"format": "semantic-ir/2.0.0", "source": {"file": "package.json",
            "authority": "test", "identity": "counter-step-package", "document": "Package",
            "formal_revision": 1}}],
        "documents": [{"file": "v0.json"}],
        "items": items,
    });
    for (name, value) in extra.as_object().expect("members") {
        request[name] = value.clone();
    }
    let path = dir.path().join("analyze.json");
    std::fs::write(
        &path,
        serde_json::json!({"format": "native-analyze/1", "request": request}).to_string(),
    )
    .expect("the request");
    match fsl_command::command::analyze(&path, &OneCandidate, cancel) {
        Ok(result) => (result.value.as_value().clone(), result.exit_code),
        Err(error) => panic!("the request is admitted: {error}"),
    }
}

/// FR-620, FR-621 (TC-1391's wire cells): the `analyze` row's holds,
/// refuted, incomplete, timeout and cancelled cells read from a
/// `native-analyze/1` document through the command adapter, each with its
/// FR-285 exit code and its FR-621 item.
#[trace("TC-1391")]
#[trace("TC-1581", "FR-621-AC-1")]
#[trace("TC-1581", "FR-621-AC-2")]
#[trace("TC-1581", "FR-621-AC-3")]
#[test]
fn analyze_wire_cells_return_each_kind() {
    let none = serde_json::json!({});
    let live = filament_exact::Cancel::new;
    let (holds, exit) = analyze_wire(serde_json::json!([{"claim": "Q"}]), none.clone(), &live());
    assert_eq!(exit, 0, "{holds}");
    assert_eq!(holds["items"][0]["record"]["result"], "proved");
    assert_eq!(holds["items"][0]["certificate"]["accepted"], true);

    let (refuted, exit) = analyze_wire(serde_json::json!([{"claim": "P"}]), none.clone(), &live());
    assert_eq!(exit, 10, "{refuted}");
    let item = &refuted["items"][0];
    assert_eq!(item["record"]["result"], "refuted");
    assert_eq!(item["counterexample"]["kind"], "formula");
    assert_eq!(item["replay"]["reproduced"], true);

    let (incomplete, exit) = analyze_wire(
        serde_json::json!([{"claim": "P", "temporal": {"limits": {"max_states": 1}}}]),
        none.clone(),
        &live(),
    );
    assert_eq!(exit, 22, "{incomplete}");
    assert_eq!(incomplete["items"][0]["record"]["result"], "incomplete");
    assert_eq!(
        incomplete["items"][0]["record"]["cause"]["limit"],
        "max_states"
    );
    assert!(incomplete["items"][0].get("counterexample").is_none());

    let (timeout, exit) = analyze_wire(
        serde_json::json!([{"claim": "Q"}]),
        serde_json::json!({"deadline_ms": 0}),
        &live(),
    );
    assert_eq!(exit, 22, "{timeout}");
    assert_eq!(
        timeout["items"][0]["record"]["cause"],
        serde_json::json!({"kind": "cancelled", "source": "deadline"})
    );

    let cancel = live();
    cancel.cancel(filament_exact::CancelCause::Requested);
    let (cancelled, exit) = analyze_wire(serde_json::json!([{"claim": "Q"}]), none, &cancel);
    assert_eq!(exit, 22, "{cancelled}");
    assert_eq!(
        cancelled["items"][0]["record"]["cause"],
        serde_json::json!({"kind": "cancelled", "source": "requested"})
    );
}
