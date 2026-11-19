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
        &fsl_analyze::provider::ProviderOptions::default(),
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
#[ignore = "QSL-2564: Engine::external refuses a provider not advertising (value-validity, unbounded); FR-968 registers an operation-contract provider"]
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

/// A process descriptor `id` advertising `pairs`.
fn advertising(
    id: &str,
    pairs: &[(fsl_semantics::check::Capability, fsl_route::Mode)],
) -> fsl_route::BackendDescriptor {
    fsl_route::descriptor_from_manifest(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new(id)),
        fsl_route::ProviderOrigin::Process,
        pairs
            .iter()
            .map(|(kind, mode)| (Some(kind.to_wire()), Some(mode.to_wire())))
            .collect::<Vec<_>>(),
        fsl_route::ManifestMembers {
            domains: None,
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    )
    .expect("a well-formed descriptor")
}

/// FR-968-AC-1: a provider registers for the kinds it advertises; one that
/// advertises `value-validity` without `unbounded`, or nothing, is refused
/// naming its `BackendId`, and a refused provider is never called.
#[trace("TC-2725", "FR-968-AC-1")]
#[test]
#[ignore = "QSL-2564: Engine::external still requires (value-validity, unbounded) of every provider and names no refusal reason"]
fn a_provider_registers_for_the_kinds_it_advertises() {
    use fsl_route::Mode;
    use fsl_semantics::check::Capability::{
        OperationContract, Schedulability, TemporalSatisfaction, ValueValidity,
    };
    #[derive(Debug)]
    struct Never;
    impl fsl_analyze::provider::ExternalProvider for Never {
        fn settle(
            &self,
            _request: &fsl_analyze::provider::ProviderRequest<'_>,
            _cancel: &filament_exact::Cancel,
        ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
            panic!("registration calls no provider")
        }
    }
    let build = |descriptor| {
        fsl_analyze::analyze::Engine::external(
            descriptor,
            fsl_analyze::analyze::BackendTool::new("test-provider", "1").expect("a tool"),
            Box::new(Never),
        )
    };
    for (id, pairs) in [
        (
            "fsl.test-contract",
            vec![(OperationContract, Mode::Unbounded)],
        ),
        (
            "fsl.test-both",
            vec![
                (OperationContract, Mode::Bounded),
                (ValueValidity, Mode::Unbounded),
            ],
        ),
    ] {
        let engine = build(advertising(id, &pairs))
            .unwrap_or_else(|refusal| panic!("{id} registers: {refusal:?}"));
        assert_eq!(engine.id().as_str(), id);
    }
    let id = "fsl.test-bounded-validity";
    let pairs = [
        (OperationContract, Mode::Unbounded),
        (ValueValidity, Mode::Bounded),
    ];
    let refusal = build(advertising(id, &pairs)).expect_err("refused at registration");
    assert_eq!(refusal.backend.as_str(), id);
    // FR-968-AC-4: the refusal names the rule the descriptor breaks.
    let bounded_only = build(advertising(
        "fsl.test-bounded-only",
        &[(ValueValidity, Mode::Bounded)],
    ))
    .expect_err("value-validity in bounded mode only is refused");
    assert_eq!(bounded_only.backend.as_str(), "fsl.test-bounded-only");
    assert!(
        bounded_only
            .to_string()
            .contains("value-validity-not-unbounded"),
        "{bounded_only}"
    );
    // FR-968-AC-4: any other kind registers, whatever its mode.
    for (id, pairs) in [
        ("fsl.test-temporal", [(TemporalSatisfaction, Mode::Bounded)]),
        (
            "fsl.test-schedulability",
            [(Schedulability, Mode::Unbounded)],
        ),
    ] {
        build(advertising(id, &pairs))
            .unwrap_or_else(|refusal| panic!("{id} registers: {refusal:?}"));
    }
    // No advertised pair: refused by the manifest reader (FR-288-AC-3) or at
    // registration, never registered.
    let empty = fsl_route::descriptor_from_manifest(
        fsl_route::Candidate::new(fsl_foundation::BackendId::new("fsl.test-nothing")),
        fsl_route::ProviderOrigin::Process,
        Vec::new(),
        fsl_route::ManifestMembers {
            domains: None,
            fairness_kinds: None,
            evidence_kinds: None,
            memory_models: None,
            reductions: None,
        },
    );
    if let Ok(descriptor) = empty {
        let refusal = build(descriptor).expect_err("an empty advertisement is refused");
        assert_eq!(refusal.backend.as_str(), "fsl.test-nothing");
        assert!(
            refusal.to_string().contains("no-advertised-pair"),
            "{refusal}"
        );
    }
}

/// The Counter `post` clause the operation `setTo` keeps.
const KEEPS: &str = "post Keeps using v on M::Counter::setTo { self.value = n }\n";

/// A Counter `post` clause `setTo` violates at every case.
const WRONG: &str = "post Wrong using v on M::Counter::setTo { self.value = n + 1 }\n";

/// A provider `fsl.test-contract` advertising (`operation-contract`,
/// `unbounded`) that answers every item with `verdict` and records the
/// FSpec FR-331 request it is sent.
#[derive(Debug)]
struct Answering {
    verdict: fn() -> fsl_analyze::provider::ProviderVerdict,
    sent: std::sync::Arc<Mutex<Vec<Value>>>,
}

impl fsl_analyze::provider::ExternalProvider for Answering {
    fn settle(
        &self,
        request: &fsl_analyze::provider::ProviderRequest<'_>,
        _cancel: &filament_exact::Cancel,
    ) -> Vec<fsl_analyze::provider::ProviderVerdict> {
        let wire = serde_json::to_value(request).expect("the request serializes");
        self.sent.lock().expect("the record lock").push(wire);
        request.items().iter().map(|_| (self.verdict)()).collect()
    }
}

fn contract_provider(
    verdict: fn() -> fsl_analyze::provider::ProviderVerdict,
) -> (
    fsl_analyze::analyze::Engine,
    std::sync::Arc<Mutex<Vec<Value>>>,
) {
    use fsl_semantics::check::Capability::OperationContract;
    let sent = std::sync::Arc::new(Mutex::new(Vec::new()));
    let engine = fsl_analyze::analyze::Engine::external(
        advertising(
            "fsl.test-contract",
            &[(OperationContract, fsl_route::Mode::Unbounded)],
        ),
        fsl_analyze::analyze::BackendTool::new("test-contract", "1").expect("a tool"),
        Box::new(Answering {
            verdict,
            sent: std::sync::Arc::clone(&sent),
        }),
    )
    .expect("an operation-contract provider registers (FR-968)");
    (engine, sent)
}

/// FR-968-AC-2: with the provider registered, an unnamed `post` item matches
/// exactly `[contract-checker, <provider>]` (FR-962 excludes `en-1`) and
/// settles `ambiguous-backend`; naming the provider sends it one
/// `operation-contract` request item.
#[trace("TC-2725", "FR-968-AC-2")]
#[test]
#[ignore = "QSL-2564: Engine::external refuses an operation-contract provider and sends no operation-contract item to a provider"]
fn a_post_item_with_a_provider_matches_contract_checker_and_the_provider() {
    use fsl_analyze::provider::ProviderVerdict;
    let ws = Workspace::new("post-item-with-provider");
    let (provider, sent) = contract_provider(|| ProviderVerdict::Unknown);
    let request = counter_request(&ws, KEEPS, json!([{"claim": "Keeps"}]));
    let (document, seen) = analyze_recording(&request, vec![provider]);
    assert_eq!(
        seen,
        vec![vec![
            "contract-checker".to_owned(),
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
    assert!(sent.lock().expect("the record lock").is_empty());

    let ws = Workspace::new("post-item-named-provider");
    let (provider, sent) = contract_provider(|| ProviderVerdict::Unknown);
    let request = counter_request(
        &ws,
        KEEPS,
        json!([{"claim": "Keeps", "backend": "fsl.test-contract"}]),
    );
    let (document, _) = analyze_recording(&request, vec![provider]);
    let sent = sent.lock().expect("the record lock");
    assert_eq!(sent.len(), 1, "{document}");
    let items = sent[0]["items"].as_array().expect("request items");
    assert_eq!(items.len(), 1, "{document}");
    assert_eq!(items[0]["kind"], "operation-contract", "{}", sent[0]);
}

/// FR-968-AC-3: a provider's verdict on an `operation-contract` item settles
/// by FR-281's provider-arm rules: `proved` `trusted`, `unknown` as
/// `solver-inconclusive`, and an `assignments` counterexample, which is not
/// an `operation-contract` counterexample, as `replay-parity` with no
/// counterexample. The `Contract` counterexample rows of FR-968-AC-3 join
/// this test with the verdict variant that carries one.
#[trace("TC-2725", "FR-968-AC-3")]
#[test]
#[ignore = "QSL-2564: no operation-contract item reaches a provider and its verdict is not settled by FR-968"]
fn a_provider_contract_verdict_settles_after_replay() {
    use fsl_analyze::provider::ProviderVerdict;
    let settle = |name: &str, verdict: fn() -> ProviderVerdict| {
        let ws = Workspace::new(name);
        let (provider, _) = contract_provider(verdict);
        let request = counter_request(
            &ws,
            WRONG,
            json!([{"claim": "Wrong", "backend": "fsl.test-contract"}]),
        );
        let (document, _) = analyze_recording(&request, vec![provider]);
        document
    };
    let proved = settle("contract-provider-proved", || ProviderVerdict::Proved);
    let item = &proved["items"][0];
    assert_eq!(label(item), "proved", "{item}");
    assert_eq!(item["record"]["backend"], "fsl.test-contract", "{item}");
    assert_eq!(item["record"]["certification"], "trusted", "{item}");

    let unknown = settle("contract-provider-unknown", || ProviderVerdict::Unknown);
    let item = &unknown["items"][0];
    assert_eq!(label(item), "inconclusive", "{item}");
    assert!(
        item["record"].to_string().contains("solver-inconclusive"),
        "{item}"
    );

    let assignments = settle("contract-provider-assignments", || {
        ProviderVerdict::Refuted {
            counterexample: vec![filament_exact::Value::Integer(0_i64.into())],
        }
    });
    let item = &assignments["items"][0];
    assert_eq!(label(item), "inconclusive", "{item}");
    assert!(
        item["record"].to_string().contains("replay-parity"),
        "{item}"
    );
    assert!(
        assignments["counterexamples"]
            .as_array()
            .is_none_or(Vec::is_empty),
        "{assignments}"
    );
}
