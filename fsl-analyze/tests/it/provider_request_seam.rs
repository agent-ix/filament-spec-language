// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-950 (TC-2660): analyze builds the complete FSpec FR-331 provider
//! request after routing and negotiation, and the provider only receives it.

use std::sync::{Arc, Mutex};

use ix_trace_rs::trace;
use serde_json::json;

use crate::analyze_cli_support::{counter_request, label, Workspace};

const PROVIDER: &str = "fsl.test-recording-provider";

/// What the recording provider saw of the request it was given.
#[derive(Debug, Default)]
struct Seen {
    calls: usize,
    complete: Vec<Option<CompleteRequest>>,
}

/// The FR-331 members FR-950 requires the request to carry.
#[derive(Debug)]
struct CompleteRequest {
    package: Vec<u8>,
    request_indexes: Vec<u64>,
    manifest_backend: String,
    deadline_ms: u64,
}

/// The FR-331 members of `request`: the admitted package bytes, the routed
/// items' request indexes, the negotiated manifest entry and the deadline.
fn complete(request: &fsl_analyze::provider::ProviderRequest<'_>) -> Option<CompleteRequest> {
    Some(CompleteRequest {
        package: request.package().bytes().to_vec(),
        request_indexes: request
            .items()
            .iter()
            .map(|item| u64::try_from(item.index().get()).expect("an index fits"))
            .collect(),
        manifest_backend: request.backend().as_str().to_owned(),
        deadline_ms: request.deadline_ms(),
    })
}

#[derive(Debug)]
struct Recording(Arc<Mutex<Seen>>);

impl fsl_analyze::provider::ExternalProvider for Recording {
    fn settle(
        &self,
        request: &fsl_analyze::provider::ProviderRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
        let mut seen = self.0.lock().expect("the record lock");
        seen.calls += 1;
        seen.complete.push(complete(request));
        vec![fsl_analyze::provider::ProviderVerdict::Proved; request.items().len()]
    }
}

fn recording_provider(seen: Arc<Mutex<Seen>>) -> fsl_analyze::analyze::Engine {
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
        Box::new(Recording(seen)),
    )
    .expect("the descriptor is unbounded")
}

/// FR-950-AC-1: the provider is called once with a complete request whose
/// package, items, manifest entry and deadline come from analyze.
#[trace("TC-2660", "FR-950-AC-1")]
#[test]
fn the_provider_receives_the_complete_request_analyze_built() {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let body = "function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n".to_owned()
        + &crate::analyze_cli_support::bounded();
    let ws = Workspace::new("provider-request-seam");
    let request = counter_request(
        &ws,
        &body,
        json!([{"claim": "Even", "kind": "value-validity", "backend": PROVIDER},
            {"claim": "Bounded"}]),
    );
    let run = crate::analyze_cli_support::analyze_with_providers(
        &request,
        vec![recording_provider(Arc::clone(&seen))],
    );
    assert_eq!(label(&run.item(0)), "proved", "{}", run.item(0));
    let seen = seen.lock().expect("the record lock");
    assert_eq!(seen.calls, 1, "one request for the one provider backend");
    let complete = seen.complete[0]
        .as_ref()
        .expect("the request carries the FR-331 package, items, manifest and limits");
    assert!(!complete.package.is_empty(), "the admitted package bytes");
    assert_eq!(complete.request_indexes, vec![0], "only the routed item");
    assert_eq!(complete.manifest_backend, PROVIDER);
    assert_eq!(complete.deadline_ms, 600_000, "FR-287's default deadline");
}
