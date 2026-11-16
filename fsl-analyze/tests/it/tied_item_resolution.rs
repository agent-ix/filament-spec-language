// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-962 (TC-2702): an unnamed item whose matching set is the linked
//! `operation-contract` tie reaches negotiation with the one provider FR-489's
//! claim-form rule selects, and any other multi-candidate set stays whole.

use std::sync::Mutex;

use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::analyze_cli_support::{counter_request, label, Fr290, Workspace};

const AT_MOST_THREE: &str =
    "invariant AtMostThree using v on M::Counter at current { self.value <= 3 }\n";

/// FSpec FR-290's double, recording the candidate identities of each item it
/// is handed.
#[derive(Default)]
struct Recording(Mutex<Vec<Vec<String>>>);

impl fsl_analyze::analyze::document::Negotiate for Recording {
    fn negotiate(
        &self,
        item: &fsl_analyze::analyze::ClaimItem,
        candidates: &fsl_route::CandidateOutcome,
    ) -> (
        fsl_route::routing::Disposition,
        Option<fsl_analyze::analyze::Diagnostic>,
    ) {
        let ids = match candidates {
            fsl_route::CandidateOutcome::Candidates(set) => set
                .candidates()
                .iter()
                .map(|candidate| candidate.id().as_str().to_owned())
                .collect(),
            fsl_route::CandidateOutcome::UnknownBackend(backend) => {
                vec![format!("unknown:{}", backend.as_str())]
            }
        };
        self.0.lock().expect("the record lock").push(ids);
        fsl_analyze::analyze::document::Negotiate::negotiate(&Fr290, item, candidates)
    }
}

fn analyze_recording(
    request: &std::path::Path,
    providers: Vec<fsl_analyze::analyze::Engine>,
) -> (Value, Vec<Vec<String>>) {
    let negotiate = Recording::default();
    let document = match fsl_command::command::analyze_with_providers(
        request,
        &negotiate,
        providers,
        &filament_exact::Cancel::new(),
    ) {
        Ok(result) => serde_json::from_str(&result.value.to_string()).expect("one JSON document"),
        Err(error) => serde_json::from_slice::<Value>(
            &error
                .outcome_document(fsl_replay::Operation::Analyze)
                .to_bytes()
                .expect("the outcome document serializes"),
        )
        .expect("one JSON document"),
    };
    let seen = negotiate.0.into_inner().expect("the record lock");
    (document, seen)
}

/// FR-962-AC-1, FR-962-AC-2: the unnamed invariant item over a model subject
/// reaches the negotiator with candidates exactly `[en-1]`, settles on `en-1`,
/// and its request item names no backend.
#[trace("TC-2702", "FR-962-AC-1")]
#[trace("TC-2702", "FR-962-AC-2")]
#[test]
#[ignore = "QSL-2192: FSL does not narrow an unnamed tied item by FR-489's claim-form rule yet"]
fn an_unnamed_tied_item_reaches_negotiation_with_one_candidate() {
    let ws = Workspace::new("tied-item-one-candidate");
    let request = counter_request(&ws, AT_MOST_THREE, json!([{"claim": "AtMostThree"}]));
    let (document, seen) = analyze_recording(&request, Vec::new());
    assert_eq!(seen, vec![vec!["en-1".to_owned()]], "{document}");
    let item = &document["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["record"]["backend"], "en-1", "{item}");
    assert!(item["request"].get("backend").is_none(), "{item}");
    assert_eq!(
        item["request"]["candidates"],
        json!({"type": "set", "backends": ["en-1"]}),
        "{item}"
    );
    assert!(
        !document.to_string().contains("ambiguous-backend"),
        "{document}"
    );
}

/// FR-962-AC-3: a third backend matching the tied item leaves the set whole,
/// so the unnamed item settles `ambiguous-backend` naming all three.
#[trace("TC-2702", "FR-962-AC-3")]
#[test]
#[ignore = "QSL-2192: no third operation-contract provider registers and FR-489's rule is not applied yet"]
fn an_undecided_tie_settles_ambiguous_backend() {
    use fsl_semantics::check::Capability;
    let descriptor = fsl_route::descriptor_from_manifest(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new("fsl.test-contract")),
        fsl_route::ProviderOrigin::Process,
        vec![(
            Some(Capability::OperationContract.to_wire()),
            Some(fsl_route::Mode::Unbounded.to_wire()),
        )],
        fsl_route::ManifestMembers {
            domains: None,
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .expect("the descriptor advertises operation-contract");
    #[derive(Debug)]
    struct Never;
    impl fsl_analyze::provider::ExternalProvider for Never {
        fn settle(
            &self,
            _request: &fsl_analyze::provider::ProviderRequest<'_>,
            _cancel: &filament_exact::Cancel,
        ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
            panic!("an ambiguous item is sent to no provider")
        }
    }
    let provider = fsl_analyze::analyze::Engine::external(
        descriptor,
        fsl_analyze::analyze::BackendTool::new("test-contract", "1").expect("a tool"),
        fsl_analyze::provider::ProviderLimits::default(),
        Box::new(Never),
    )
    .expect("an operation-contract process provider registers (FR-635)");
    let ws = Workspace::new("tied-item-undecided");
    let request = counter_request(&ws, AT_MOST_THREE, json!([{"claim": "AtMostThree"}]));
    let (document, seen) = analyze_recording(&request, vec![provider]);
    assert_eq!(
        seen,
        vec![vec![
            "contract-checker".to_owned(),
            "en-1".to_owned(),
            "fsl.test-contract".to_owned()
        ]],
        "{document}"
    );
    assert!(
        document["items"][0]
            .to_string()
            .contains("ambiguous-backend"),
        "{document}"
    );
}
