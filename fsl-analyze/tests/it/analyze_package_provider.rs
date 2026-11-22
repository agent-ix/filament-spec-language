// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-850 "Equality with the source route", FR-940-AC-13, FR-644-AC-8
//! (QSL-2767, second slice): a `value-validity` item that an external
//! provider decides settles over a checked package's admitted bytes exactly
//! as it settles over the source, and the provider receives the same
//! request on both routes.

use std::sync::{Arc, Mutex};

use filament_exact::{Integer, Value};
use fsl_analyze::analyze::{BackendTool, Engine};
use fsl_analyze::provider::{ExternalProvider, ProviderRequest, ProviderVerdict};
use fsl_route::Mode;
use ix_trace_rs::trace;
use serde_json::{json, Value as Json};

use crate::analyze_cli_support::{
    analyze_with_providers, bounded, counter_request, label, package_selection, request_members,
    Run, Workspace,
};

const SMT: &str = "fsl.test-smt";

/// A value clause over the mathematical `Integer`, which no bounded
/// candidate decides.
const EVEN: &str = "function Even using v(n: Integer): Boolean pure { n * 2 - n = n }\n";
/// An unbounded clause that fails at every `n` but 3.
const THREE: &str = "function Three using v(n: Integer): Boolean pure { n = 3 }\n";

/// A provider that answers `verdict` for every item and keeps each request
/// it was sent, serialized as the wire member FR-331 `request`.
#[derive(Debug)]
struct Spy {
    verdict: ProviderVerdict,
    seen: Arc<Mutex<Vec<Json>>>,
}

impl ExternalProvider for Spy {
    fn settle(
        &self,
        request: &ProviderRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> Vec<ProviderVerdict> {
        self.seen
            .lock()
            .expect("the spy's log")
            .push(serde_json::to_value(request).expect("the request serializes"));
        vec![self.verdict.clone(); request.items().len()]
    }
}

fn spy(verdict: ProviderVerdict) -> (Engine, Arc<Mutex<Vec<Json>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let members = fsl_route::ManifestMembers {
        domains: Some(&[fsl_foundation::bound::DomainKind::Integer.to_wire()]),
        fairness_kinds: None,
        evidence_kinds: None,
        memory_models: None,
        reductions: None,
    };
    let descriptor = fsl_route::descriptor_from_manifest(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new(SMT)),
        fsl_route::ProviderOrigin::Linked,
        [(
            Some(fsl_route::Capability::ValueValidity.to_wire()),
            Some(Mode::Unbounded.to_wire()),
        )],
        members,
    )
    .expect("a well-formed descriptor");
    let engine = Engine::external(
        descriptor,
        BackendTool::new("test-provider", "1").expect("a tool"),
        Box::new(Spy {
            verdict,
            seen: Arc::clone(&seen),
        }),
    )
    .expect("the descriptor is unbounded");
    (engine, seen)
}

/// What one route settled and what its provider was sent.
struct Outcome {
    run: Run,
    requests: Vec<Json>,
}

/// The source route and the package route of `items` over `declarations`,
/// each with its own provider answering `verdict`.
fn both(
    name: &str,
    declarations: &str,
    items: &Json,
    verdict: &ProviderVerdict,
) -> (Outcome, Outcome) {
    let body = format!("{declarations}{}", bounded());
    let ws = Workspace::new(name);
    let (engine, seen) = spy(verdict.clone());
    let source = analyze_with_providers(
        &counter_request(&ws, &body, items.clone()),
        vec![engine],
    );
    let source = Outcome {
        run: source,
        requests: seen.lock().expect("the log").clone(),
    };
    let (members, _) = package_selection(&ws, &request_members(Some(items.clone())));
    let document = json!({"format": "native-analyze/1", "request": members});
    let (engine, seen) = spy(verdict.clone());
    let package = analyze_with_providers(
        &ws.write("analyze-package.json", document.to_string().as_bytes()),
        vec![engine],
    );
    let package = Outcome {
        run: package,
        requests: seen.lock().expect("the log").clone(),
    };
    (source, package)
}

/// Whole-result equality (items, aggregate, registry, exit) and the same
/// provider request; returns the package route's first item.
fn assert_parity(name: &str, declarations: &str, items: &Json, verdict: &ProviderVerdict) -> Json {
    let (source, package) = both(name, declarations, items, verdict);
    let (left, right) = (source.run.document(), package.run.document());
    for member in ["items", "aggregate", "registry", "diagnostics"] {
        assert_eq!(right[member], left[member], "{member}: {right}");
    }
    assert_eq!(package.run.exit, source.run.exit, "{right}");
    assert_eq!(
        package.requests, source.requests,
        "the provider is sent the same request on both routes"
    );
    right["items"][0].clone()
}

fn integer(n: i128) -> Value {
    Value::Integer(Integer::from(n))
}

fn item(claim: &str) -> Json {
    json!([{"claim": claim, "kind": "value-validity", "backend": SMT}])
}

/// FR-644-AC-8, FR-940-AC-13: an unbounded `Integer` item a provider proves
/// settles `proved` over the package as over the source, and the provider is
/// sent one request, the same on both routes.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn a_provider_proved_value_item_over_a_package_equals_the_source() {
    let item = assert_parity("pkg-provider-proved", EVEN, &item("Even"), &ProviderVerdict::Proved);
    assert_eq!(label(&item), "proved", "{item}");
    assert_eq!(item["record"]["backend"], SMT, "{item}");
}

/// FR-281-AC-14: a provider's counterexample is replayed at the package's
/// own function and settles `refuted` with the source route's replay.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn a_provider_refuted_value_item_over_a_package_equals_the_source() {
    let verdict = ProviderVerdict::Refuted {
        counterexample: vec![integer(4)],
    };
    let item = assert_parity("pkg-provider-refuted", THREE, &item("Three"), &verdict);
    assert_eq!(label(&item), "refuted", "{item}");
    assert_eq!(item["replay"]["reproduced"], true, "{item}");
}

/// FR-281: a counterexample that does not violate the clause is refused the
/// same way on both routes.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn a_provider_counterexample_that_replays_clean_settles_alike() {
    let verdict = ProviderVerdict::Refuted {
        counterexample: vec![integer(3)],
    };
    assert_parity("pkg-provider-bad-cex", THREE, &item("Three"), &verdict);
}

/// FR-331: a provider that answers `unknown` settles the same on both
/// routes.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn a_provider_unknown_value_item_over_a_package_equals_the_source() {
    assert_parity("pkg-provider-unknown", EVEN, &item("Even"), &ProviderVerdict::Unknown);
}

/// FR-290: an item naming a backend nobody registered settles the same
/// typed outcome on both routes and no request is sent.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn an_absent_provider_settles_alike_over_a_package() {
    let items = json!([{"claim": "Even", "kind": "value-validity", "backend": "fsl.absent"}]);
    let (source, package) = both("pkg-provider-absent", EVEN, &items, &ProviderVerdict::Proved);
    assert_eq!(package.run.document()["items"], source.run.document()["items"]);
    assert_eq!(package.run.exit, source.run.exit);
    assert!(package.requests.is_empty() && source.requests.is_empty());
}
