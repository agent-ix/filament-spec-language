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
    spy_for(verdict, fsl_semantics::check::Capability::ValueValidity)
}

fn spy_for(
    verdict: ProviderVerdict,
    kind: fsl_semantics::check::Capability,
) -> (Engine, Arc<Mutex<Vec<Json>>>) {
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
        [(Some(kind.to_wire()), Some(Mode::Unbounded.to_wire()))],
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
    let source = analyze_with_providers(&counter_request(&ws, &body, items.clone()), vec![engine]);
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
    let item = assert_parity(
        "pkg-provider-proved",
        EVEN,
        &item("Even"),
        &ProviderVerdict::Proved,
    );
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
    assert_parity(
        "pkg-provider-unknown",
        EVEN,
        &item("Even"),
        &ProviderVerdict::Unknown,
    );
}

/// FR-290: an item naming a backend nobody registered settles the same
/// typed outcome on both routes and no request is sent.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn an_absent_provider_settles_alike_over_a_package() {
    let items = json!([{"claim": "Even", "kind": "value-validity", "backend": "fsl.absent"}]);
    let (source, package) = both(
        "pkg-provider-absent",
        EVEN,
        &items,
        &ProviderVerdict::Proved,
    );
    assert_eq!(
        package.run.document()["items"],
        source.run.document()["items"]
    );
    assert_eq!(package.run.exit, source.run.exit);
    assert!(package.requests.is_empty() && source.requests.is_empty());
}

/// Emit an authentic package, then remove the source before the strict package
/// intake and analysis. The selected claim and its occurrence come from the
/// emitted name table rather than a test-authored requirement row.
fn contract_package(name: &str, body: &str, items: Json, engines: Vec<Engine>) -> Run {
    let ws = Workspace::new(name);
    let _ = counter_request(&ws, body, items.clone());
    let (members, _) = package_selection(&ws, &request_members(Some(items)));
    std::fs::remove_file(ws.path("program.fsl")).expect("the source is removed");
    let document = json!({"format": "native-analyze/1", "request": members});
    analyze_with_providers(
        &ws.write(
            "analyze-contract-package.json",
            document.to_string().as_bytes(),
        ),
        engines,
    )
}

fn assert_contract_record_refusal(run: &Run, index: usize, name: &str) {
    let document = run.document();
    assert_eq!(document["items"], json!([]), "{document}");
    let diagnostic = &document["diagnostics"][0];
    assert_eq!(diagnostic["code"], "unsupported_construct", "{document}");
    assert_eq!(diagnostic["cause"], "not-yet-implemented", "{document}");
    let message = diagnostic["message"].as_str().expect("a refusal message");
    assert!(message.contains(&format!("items[{index}]")), "{message}");
    assert!(
        message.contains(&format!("`{name}` (claim occurrence 0)")),
        "{message}"
    );
    assert!(
        message.contains("operation-contract requirement record has no package form"),
        "{message}"
    );
    assert!(message.contains("CheckedNodeId"), "{message}");
    assert!(
        !message.contains("requirement row"),
        "no invented row index: {message}"
    );
}

/// Trace: FR-940-AC-13, FR-950-AC-1, FR-968-AC-2.
/// Pre/post selections carry source operation-contract records that the package
/// does not represent, even though it can resolve their authentic claim names.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn contract_package_refuses_missing_operation_contract_requirement_record() {
    for name in ["CanInc", "Inc"] {
        for backend in [None, Some("contract-checker"), Some(SMT)] {
            let (engine, seen) = spy_for(
                ProviderVerdict::Proved,
                fsl_semantics::check::Capability::OperationContract,
            );
            let mut selection = json!({"claim": name, "kind": "operation-contract"});
            if let Some(backend) = backend {
                selection["backend"] = json!(backend);
            }
            let engines = if backend == Some(SMT) {
                vec![engine]
            } else {
                vec![]
            };
            let run = contract_package(
                &format!("pkg-contract-{name}-{}", backend.unwrap_or("default")),
                &bounded(),
                json!([selection]),
                engines,
            );
            assert_contract_record_refusal(&run, 0, name);
            assert!(seen.lock().expect("the provider log").is_empty());
        }
    }
}

/// Trace: FR-962-AC-1, FR-950-AC-1.
/// A matching external contract provider leaves two candidates after EN-1 is
/// excluded. Negotiation must report the ambiguity before requesting a form.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn contract_package_keeps_ambiguous_contract_candidates() {
    let (engine, seen) = spy_for(
        ProviderVerdict::Proved,
        fsl_semantics::check::Capability::OperationContract,
    );
    let run = contract_package(
        "pkg-contract-ambiguous",
        &bounded(),
        json!([{"claim": "Inc", "kind": "operation-contract"}]),
        vec![engine],
    );
    let item = run.item(0);
    assert_eq!(
        item["request"]["candidates"]["backends"],
        json!(["contract-checker", SMT]),
        "{item}"
    );
    assert!(item.to_string().contains("ambiguous-backend"), "{item}");
    assert!(!run
        .document()
        .to_string()
        .contains("operation-contract requirement record"));
    assert!(seen.lock().expect("the provider log").is_empty());
}

/// Trace: FR-621-AC-1, FR-950-AC-2, FR-968-AC-3.
/// An earlier provider item must not run or settle when a later contract needs
/// an unrepresented record; the request refuses as a whole.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn contract_package_mixed_request_refuses_before_any_provider_runs() {
    let (engine, seen) = spy(ProviderVerdict::Proved);
    let run = contract_package(
        "pkg-contract-mixed",
        &format!("{EVEN}{}", bounded()),
        json!([
            {"claim": "Even", "kind": "value-validity", "backend": SMT},
            {"claim": "Inc", "kind": "operation-contract"}
        ]),
        vec![engine],
    );
    assert_contract_record_refusal(&run, 1, "Inc");
    assert!(seen.lock().expect("the provider log").is_empty());
}

/// Trace: FR-962-AC-1, FR-940-AC-13.
/// EN-1's invariant/model tie reads the package's invariant form and keeps its
/// existing result. Naming EN-2 requires the missing contract record instead.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn contract_package_keeps_the_native_invariant_tie() {
    let body = "invariant Safe using v on M::Counter at current { self.value <= 3 }\n";
    for backend in [None, Some("en-1"), Some("contract-checker")] {
        let mut item = json!({"claim": "Safe", "kind": "operation-contract"});
        if let Some(backend) = backend {
            item["backend"] = json!(backend);
        }
        let run = contract_package(
            &format!("pkg-contract-invariant-{}", backend.unwrap_or("default")),
            body,
            json!([item]),
            vec![],
        );
        if backend == Some("contract-checker") {
            assert_contract_record_refusal(&run, 0, "Safe");
        } else {
            assert_eq!(label(&run.item(0)), "proved", "{}", run.document());
            assert_eq!(run.item(0)["record"]["backend"], "en-1");
        }
    }
}

/// Trace: FR-950-AC-1, FR-968-AC-2.
/// Name lookup and candidate failures keep their ordinary dispositions before
/// any operation-contract form is requested.
#[trace("TC-2410", "FR-940-AC-13")]
#[test]
fn contract_package_keeps_unknown_names_and_candidate_refusals() {
    for (name, backend, expected) in [
        ("Missing", SMT, "unknown-claim"),
        ("Inc", "fsl.absent", "unknown-backend"),
        ("Inc", SMT, "unsupported-requested-capability"),
    ] {
        // This provider advertises only value-validity: the named contract's
        // candidate set is empty, rather than a request for a missing record.
        let (engine, seen) = spy(ProviderVerdict::Proved);
        let run = contract_package(
            &format!("pkg-contract-precedence-{name}-{backend}"),
            &bounded(),
            json!([{"claim": name, "kind": "operation-contract", "backend": backend}]),
            vec![engine],
        );
        let document = run.document();
        assert_eq!(
            document["items"].as_array().map(Vec::len),
            Some(1),
            "{document}"
        );
        assert!(document.to_string().contains(expected), "{document}");
        assert!(
            !document
                .to_string()
                .contains("operation-contract requirement record"),
            "{document}"
        );
        assert!(seen.lock().expect("the provider log").is_empty());
    }
}

/// FSpec FR-290-AC-14 (TC-950), FR-335-AC-2: a per-step contract item whose
/// extent is unbounded, on the contract checker that advertises only `bounded`,
/// is written `unsupported` with `requires_bound`/`requires-bound`, settled by
/// its disposition, with no engine result members, when the request's extent
/// says a finite bound is available.
#[trace("FSpec-TC-950", "FSpec-FR-290-AC-14", "TC-845", "FR-335-AC-2")]
#[test]
fn unbounded_operation_contract_item_is_written_requires_bound() {
    let ws = Workspace::new("contract-requires-bound-wire");
    let path = crate::analyze_cli_support::staying_counter_request(
        &ws,
        "",
        json!([{"claim": "Keeps", "kind": "operation-contract"}]),
    );
    let run = analyze_with_providers(&path, vec![]);
    let item = run.item(0);
    assert_eq!(
        item["request"]["extent"],
        json!({"classification": "unbounded", "finite_bound_available": true}),
        "{item}"
    );
    assert_eq!(item["category"], "unsupported", "{item}");
    let record = &item["record"];
    assert_eq!(record["settled_by"], "disposition", "{item}");
    assert_eq!(record["result"], "unsupported", "{item}");
    assert_eq!(
        record["diagnostic"],
        json!({"code": "requires_bound", "cause": "requires-bound", "detail": {}}),
        "{item}"
    );
    let mut members: Vec<&str> = record
        .as_object()
        .expect("a record object")
        .keys()
        .map(String::as_str)
        .collect();
    members.sort_unstable();
    assert_eq!(
        members,
        ["diagnostic", "result", "settled_by"],
        "a disposition has no backend, basis or certification: {item}"
    );
}

/// FR-290-AC-20, FR-281-AC-24, FR-621-AC-18, FR-286-AC-31: a refuted
/// provider verdict carrying each closed unusable-counterexample reason in
/// turn settles `inconclusive`/`replay-parity` with no `counterexample` or
/// `replay`, exactly one item `diagnostics` entry spelling the reason, and
/// the document-level `diagnostics` unchanged; it validates against the
/// published schema.
#[trace("TC-774", "FR-290-AC-20")]
#[trace("TC-774", "FR-281-AC-24")]
#[trace("TC-774", "FR-621-AC-18")]
#[trace("TC-774", "FR-286-AC-31")]
#[test]
fn each_unusable_provider_counterexample_reason_settles_replay_parity_with_one_item_diagnostic() {
    use fsl_analyze::provider::UnusableCounterexample as Reason;
    let schema: Json = serde_json::from_str(fsl_replay::outcome::OUTCOME_SCHEMA_1).unwrap();
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .expect("the schema compiles");
    for (reason, spelling) in [
        (Reason::PackageGraphUnreadable, "package-graph-unreadable"),
        (Reason::NodeIdUnresolved, "node-id-unresolved"),
        (
            Reason::AssignmentNotClaimParameter,
            "assignment-not-claim-parameter",
        ),
        (Reason::ClaimParameterUnassigned, "claim-parameter-unassigned"),
        (
            Reason::ParameterValueKindMismatch,
            "parameter-value-kind-mismatch",
        ),
    ] {
        let verdict = ProviderVerdict::RefutedUnusable { reason };
        let (source, package) = both("unusable-reason", THREE, &item("Three"), &verdict);
        let (left, right) = (source.run.document(), package.run.document());
        assert_eq!(right["items"], left["items"], "{spelling}: {right}");
        for outcome in [&source, &package] {
            let document = outcome.run.document();
            let item = &document["items"][0];
            assert_eq!(label(item), "inconclusive", "{spelling}: {item}");
            assert_eq!(item["record"]["cause"]["kind"], "replay-parity", "{item}");
            assert!(item.get("counterexample").is_none(), "{item}");
            assert!(item.get("replay").is_none(), "{item}");
            assert!(item["record"].get("diagnostic").is_none(), "{item}");
            assert_eq!(
                item["diagnostics"],
                json!([{"code": "invalid_result", "cause": "invalid-value", "reason": spelling}]),
                "{item}"
            );
            assert_eq!(document["diagnostics"], json!([]), "{document}");
            assert_eq!(outcome.requests.len(), 1, "the provider is asked once");
            assert!(compiled.is_valid(&document), "{spelling}: {document}");
        }
        assert_eq!(reason.as_str(), spelling);
        assert_eq!(Reason::from_spelling(spelling), Some(reason));
    }
}

/// FR-290-AC-20: mapped assignments still replay, and carry no item
/// `diagnostics` entry; a reproduced violation settles `refuted`.
#[trace("TC-774", "FR-290-AC-20")]
#[trace("TC-774", "FR-621-AC-18")]
#[test]
fn mapped_provider_assignments_keep_the_replay_path_and_write_no_item_diagnostic() {
    let verdict = ProviderVerdict::Refuted {
        counterexample: vec![integer(4)],
    };
    let item = assert_parity("unusable-control", THREE, &item("Three"), &verdict);
    assert_eq!(label(&item), "refuted", "{item}");
    assert!(item.get("diagnostics").is_none(), "{item}");
    let unreplayed = ProviderVerdict::Refuted {
        counterexample: vec![integer(3)],
    };
    let item = assert_parity("unusable-mapped", THREE, &self::item("Three"), &unreplayed);
    assert_eq!(label(&item), "inconclusive", "{item}");
    assert!(item.get("diagnostics").is_none(), "{item}");
}

/// FR-290-AC-20: several coexisting defects select the first reason in the
/// declared order, whatever order they are found in; a spelling outside the
/// closed set (a provider's own text) names no reason.
#[trace("TC-774", "FR-290-AC-20")]
#[test]
fn coexisting_defects_select_the_declared_priority_and_unknown_spellings_name_no_reason() {
    use fsl_analyze::provider::UnusableCounterexample as Reason;
    let pairs = [
        (
            Reason::PackageGraphUnreadable,
            Reason::NodeIdUnresolved,
            Reason::PackageGraphUnreadable,
        ),
        (
            Reason::NodeIdUnresolved,
            Reason::AssignmentNotClaimParameter,
            Reason::NodeIdUnresolved,
        ),
        (
            Reason::AssignmentNotClaimParameter,
            Reason::ClaimParameterUnassigned,
            Reason::AssignmentNotClaimParameter,
        ),
        (
            Reason::ClaimParameterUnassigned,
            Reason::ParameterValueKindMismatch,
            Reason::ClaimParameterUnassigned,
        ),
    ];
    for (high, low, expected) in pairs {
        assert_eq!(Reason::select([high, low]), Some(expected));
        assert_eq!(Reason::select([low, high]), Some(expected));
    }
    assert_eq!(Reason::select([]), None);
    for text in ["", "node-id-unresolved ", "Node-Id-Unresolved", "boom", "other"] {
        assert_eq!(Reason::from_spelling(text), None, "{text:?}");
    }
}
