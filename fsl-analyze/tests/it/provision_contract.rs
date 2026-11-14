// SPDX-License-Identifier: AGPL-3.0-or-later
//! The provider/analyze byte-provision and evidence boundary.
//!
//! These are the in-process portions of the tagged criteria. Candidate
//! fixtures come from the real engine registry; their dispositions are
//! supplied by this caller. Genuine CG negotiation and the complete FR-331
//! wire projection remain integration obligations, not coverage claimed here.

use filament_exact::Cancel;
use fsl_analyze::analyze::{
    analyze, AnalyzeLimits, AnalyzeRefusal, AnalyzeRequest, ClaimItem, ClaimRefusal, ClauseLimits,
    Counterexample, Engines, ItemForm, ItemOptions, NegotiatedItem, ProbabilisticItem,
    ProbabilisticLimits, SourceProvision, TemporalItem,
};
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use fsl_analyze::provider;
use fsl_foundation::diagnostic::StageFailure;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex};
use fsl_replay::{IncompleteCause, ModelCheckLimit, TerminalValue};
use fsl_route::request::ExtentClassification;
use fsl_route::routing::Disposition;
use fsl_route::{Candidate, EvidenceKind};
use fsl_semantics::check::Capability;
use ix_trace_rs::trace;
use serde_json::json;

use crate::model_world::{self, Clause, World, INFINITE};
use crate::prob_world;

fn counter() -> World {
    World::new(
        model_world::counter(),
        crate::counter::CONTRACT,
        INFINITE,
        &[Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value < 2)",
        }],
        &[("c", json!({"value":{"integer":"0"}}))],
    )
}

fn temporal(world: &World) -> NegotiatedItem {
    let node = &world.package.graph().temporal_clauses()[0];
    let candidate = registered_candidate(Capability::TemporalSatisfaction, None, "en-1");
    NegotiatedItem {
        item: ClaimItem {
            request_index: RequestIndex::new(0),
            claim: OccurrenceKey::new(node.identity(), node.claim().clone()),
            kind: Capability::TemporalSatisfaction,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: true,
            },
            backend: Some(candidate.id().clone()),
            candidates: vec![candidate.clone()],
            options: ItemOptions {
                required: true,
                ..ItemOptions::default()
            },
            form: Some(ItemForm::Temporal(TemporalItem::default())),
        },
        disposition: Disposition::Supported(candidate),
        diagnostic: None,
    }
}

fn exact(world: &prob_world::World, limits: ExactProbLimits) -> NegotiatedItem {
    let candidate = registered_candidate(
        Capability::ProbabilisticSatisfaction,
        Some(EvidenceKind::Exact),
        "en-5",
    );
    NegotiatedItem {
        item: ClaimItem {
            request_index: RequestIndex::new(0),
            claim: world
                .claim("Likely")
                .occurrence()
                .expect("the authored occurrence")
                .clone(),
            kind: Capability::ProbabilisticSatisfaction,
            extent: ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
            backend: Some(candidate.id().clone()),
            candidates: vec![candidate.clone()],
            options: ItemOptions::default(),
            form: Some(ItemForm::Probabilistic(ProbabilisticItem {
                evidence: EvidenceKind::Exact,
                limits: ProbabilisticLimits::Exact(limits),
                method: None,
                seed: None,
                sampler: None,
                max_witnesses: None,
                symmetry: Vec::new(),
                timed_sampling: None,
            })),
        },
        disposition: Disposition::Supported(candidate),
        diagnostic: None,
    }
}

#[trace("TC-763", "FR-281-AC-20", "TC-761", "FR-280-AC-10")]
#[test]
fn missing_source_refuses_analyze_and_declines_provider_items_before_any_engine() {
    let world = counter();
    let request = AnalyzeRequest {
        binding: world.binding(&["c"]),
        provision: SourceProvision::default(),
        items: vec![temporal(&world)],
    };
    let engines = Engines::fsl().expect("the manifests register");
    let limits = AnalyzeLimits::unbounded();
    let failure = analyze(&world.package, &request, &engines, &limits, &Cancel::new())
        .expect_err("the source is absent");
    let StageFailure::Refused(AnalyzeRefusal::Provision(refusal)) = failure else {
        panic!("wrong request refusal: {failure:?}")
    };
    assert_eq!(
        refusal.authority,
        world.package.graph().source().authority()
    );
    assert_eq!(refusal.identity, world.package.graph().source().identity());
    assert_eq!(refusal.catalog_code().code(), "missing_import");
    let served = provider::run(
        &world.package,
        &request.binding,
        &request.provision,
        &request.items,
        &engines,
        &limits,
        &Cancel::new(),
    );
    assert_eq!(served.records.len(), 1);
    assert!(matches!(
        served.records[0].value(),
        TerminalValue::Declined {
            cause: fsl_replay::ProofRefusalCause::IncompleteInput,
            ..
        }
    ));
    assert_eq!(served.records[0].diagnostic(), Some(refusal.catalog_code()));
    assert!(served.records[0].backend().is_none());
    assert!(served.counterexamples.is_empty());
}

#[trace("TC-762", "FR-281-AC-2", "TC-761", "FR-280-AC-9")]
#[test]
fn a_refuted_clause_returns_the_reproduced_counterexample_through_both_entries() {
    let world = counter();
    let provision = SourceProvision::single(world.package.graph().source(), &world.unit);
    let request = AnalyzeRequest {
        binding: world.binding(&["c"]),
        provision,
        items: vec![temporal(&world)],
    };
    let limits = AnalyzeLimits::unbounded();
    let engines = Engines::fsl().expect("the manifests register");
    let outcome = analyze(&world.package, &request, &engines, &limits, &Cancel::new())
        .expect("the request admits")
        .into_value();
    let served = provider::run(
        &world.package,
        &request.binding,
        &request.provision,
        &request.items,
        &engines,
        &limits,
        &Cancel::new(),
    );
    assert_eq!(outcome.records, served.records);
    assert_eq!(outcome.records[0].value(), &TerminalValue::Refuted);
    assert_eq!(outcome.counterexamples.len(), 1);
    assert_eq!(served.counterexamples.len(), 1);
    let Counterexample::Temporal {
        counterexample: payload,
        ..
    } = &outcome.counterexamples[0].counterexample
    else {
        panic!("a temporal counterexample")
    };
    let Counterexample::Temporal {
        counterexample: served_payload,
        ..
    } = &served.counterexamples[0].counterexample
    else {
        panic!("a temporal counterexample")
    };
    assert_eq!(payload, served_payload);
    assert_eq!(payload.kind, fsl_replay::CounterexampleKind::Formula);
    assert_eq!(outcome.counterexamples[0].backend.as_str(), "en-1");
    let node = &world.package.graph().temporal_clauses()[0];
    let subject = fsl_analyze::analyze::ClauseSubject::new(
        &world.package,
        world.binding(&["c"]),
        limits,
        &request.provision,
        &OccurrenceKey::new(node.identity(), node.claim().clone()),
        ClauseLimits::default(),
    )
    .expect("the clause is present");
    assert!(matches!(
        subject
            .replay_counterexample(payload)
            .expect("the replay admits")
            .settlement,
        fsl_replay::ModelTraceSettlement::Reproduced {
            value: fsl_replay::ModelTraceValue::Truth(false),
            ..
        }
    ));
}

const LIKELY: &str = "probabilistic Likely using inf on M over (m: M::Msg) under every scheduler on origin { probability >= 0.95 [ eventually holds(m.delivered) ] }";

#[trace("TC-762", "FR-281-AC-1", "TC-761", "FR-280-AC-2", "FR-280-AC-8")]
#[test]
fn the_unbounded_exact_member_retains_the_checker_owned_certificate() {
    let world = prob_world::World::new(&prob_world::link(LIKELY));
    let request = AnalyzeRequest {
        binding: world.binding(),
        provision: world.provision.clone(),
        items: vec![exact(&world, ExactProbLimits::default())],
    };
    let engines = Engines::fsl().expect("the manifests register");
    let limits = AnalyzeLimits::unbounded();
    let outcome = analyze(&world.package, &request, &engines, &limits, &Cancel::new())
        .expect("the request admits")
        .into_value();
    let served = provider::run(
        &world.package,
        &request.binding,
        &request.provision,
        &request.items,
        &engines,
        &limits,
        &Cancel::new(),
    );
    assert_eq!(outcome.records, served.records);
    assert!(matches!(
        outcome.records[0].value(),
        TerminalValue::Proved { .. }
    ));
    let certificate = outcome.records[0]
        .exact_certificate()
        .expect("the accepted certificate is carried");
    assert!(!certificate.values.is_empty());
    let bytes = outcome.records[0]
        .exact_certificate_document()
        .expect("the public result carries the accepted canonical document");
    assert_eq!(served.records[0].exact_certificate_document(), Some(bytes));
    let document = fsl_replay::read_probability_certificate(
        bytes,
        fsl_replay::ProbabilityDocumentLimits {
            bytes: 1_000_000,
            storage_bytes: 8_000_000,
            states: 100,
            edges: 1000,
            scalar_bits: 128,
            accounting: fsl_replay::AccountingLimits::default().0,
        },
        &Cancel::new(),
    )
    .expect("the retained bytes admit through the qualified reader");
    assert_eq!(document.values, certificate.values);
    assert_eq!(document.ranking.len(), certificate.values.len());
    assert!(outcome.counterexamples.is_empty());
    assert_eq!(
        outcome.records[0].backend().map(BackendId::as_str),
        Some("en-5")
    );
}

#[trace("TC-763", "FR-281-AC-6")]
#[test]
fn the_one_state_exact_budget_returns_no_counterexample_or_certificate() {
    let world = prob_world::World::new(&prob_world::link(LIKELY));
    let mut exact_limits = ExactProbLimits::default();
    exact_limits.model.limits.max_states = 1;
    let request = AnalyzeRequest {
        binding: world.binding(),
        provision: world.provision.clone(),
        items: vec![exact(&world, exact_limits)],
    };
    let outcome = analyze(
        &world.package,
        &request,
        &Engines::fsl().expect("the manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("the request admits")
    .into_value();
    assert_eq!(
        outcome.records[0].value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ExactProbMaxStates,
            value: 1
        })
    );
    assert!(outcome.records[0].exact_certificate().is_none());
    assert!(outcome.counterexamples.is_empty());
}

/// The provision's labels do not grant authority to different source bytes.
/// A counterexample that the supplied unit cannot reproduce is not exported.
#[trace("TC-763", "FR-281-AC-20")]
#[test]
fn changed_source_bytes_never_promote_an_unreproduced_candidate_to_refuted() {
    let world = counter();
    let changed = String::from_utf8(world.unit.clone())
        .expect("fixture UTF-8")
        .replace("c.value < 2", "c.value < 5");
    assert_ne!(
        changed.as_bytes(),
        world.unit.as_slice(),
        "the source mutation changes the bound"
    );
    let request = AnalyzeRequest {
        binding: world.binding(&["c"]),
        provision: SourceProvision::single(world.package.graph().source(), changed.as_bytes()),
        items: vec![temporal(&world)],
    };
    let outcome = analyze(
        &world.package,
        &request,
        &Engines::fsl().expect("the manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("the labels, binding and source provision admit")
    .into_value();
    assert!(
        matches!(outcome.records[0].value(), TerminalValue::Inconclusive(_)),
        "{:?}",
        outcome.records[0]
    );
    assert!(outcome.counterexamples.is_empty());
}

#[trace("TC-763", "FR-281-AC-20")]
#[test]
fn source_completeness_is_checked_even_when_the_package_cannot_emit() {
    let world = prob_world::World::new(&prob_world::service(""));
    fsl_package::emit_checked(&world.package, fsl_package::EmitLimits::fixture())
        .expect_err("probabilistic model refuses S4 emission");
    let request = AnalyzeRequest {
        binding: world.binding(),
        provision: SourceProvision::default(),
        items: Vec::new(),
    };
    let failure = analyze(
        &world.package,
        &request,
        &Engines::fsl().expect("the manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("missing source refuses even this package");
    assert!(matches!(
        failure,
        StageFailure::Refused(AnalyzeRefusal::Provision(_))
    ));
}

fn plain(claim: OccurrenceKey, kind: Capability, backend: &str) -> NegotiatedItem {
    let candidate = registered_candidate(kind, None, backend);
    NegotiatedItem {
        item: ClaimItem {
            request_index: RequestIndex::new(0),
            claim,
            kind,
            extent: ExtentClassification::Bounded,
            backend: Some(candidate.id().clone()),
            candidates: vec![candidate.clone()],
            options: ItemOptions::default(),
            form: None,
        },
        disposition: Disposition::Supported(candidate),
        diagnostic: None,
    }
}

/// Generalized items retain the existing value arm's native replay token;
/// they do not reinterpret a value claim as a probabilistic request.
#[trace("TC-761", "FR-280-AC-2", "TC-763", "FR-281-AC-12")]
#[test]
fn a_value_claim_returns_its_native_replayed_assignment() {
    let declaration = "function Goal using v(x: Int[0,3]): Boolean pure { x < 3 }";
    let (package, source) = crate::value_validity::package_with_source(declaration);
    let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
        .expect("empty package input");
    let snapshots = fsl_semantics::model::observation::DocumentProvision::default();
    let binding = fsl_analyze::analyze::SubjectBinding {
        packages: &packages,
        snapshots: &snapshots,
        initial: Vec::new(),
        universes: Vec::new(),
    };
    let function = package
        .graph()
        .function_identity("Goal")
        .expect("the function identity");
    let body = package
        .graph()
        .semantic_graph()
        .node(function)
        .and_then(|node| node.function_body())
        .expect("the checked Boolean body");
    let mut claims = package.graph().occurrences().filter(|(node, origin, _)| {
        *node == body
            && fsl_package::occurrence_role(origin.role().as_str())
                == Some(fsl_package::CheckedOccurrenceRole::Claim)
    });
    let (claim_node, claim_origin, _) = claims
        .next()
        .expect("the producer recorded the Boolean body's claim");
    assert!(
        claims.next().is_none(),
        "one claim occurrence for this Boolean body"
    );
    let claim = OccurrenceKey::new(claim_node, claim_origin);
    let target = fsl_replay::value_check::ValueTarget::select(&package, "Goal", &claim)
        .expect("the native selector accepts the producer's Boolean-body occurrence");
    let native_request = fsl_analyze::value_check::ValueRequest {
        target,
        max_cases: None,
        reported: None,
    };
    let native = fsl_analyze::value_check::check_value(
        &native_request,
        fsl_analyze::value_check::manifest::DEFAULT_MAX_CASES,
        &Cancel::new(),
    )
    .expect("native value check");
    assert!(
        matches!(native, fsl_analyze::value_check::ValueOutcome::Refuted(_)),
        "{native:?}"
    );
    let direct = fsl_analyze::analyze::analyze_items(
        &Engines::fsl().expect("the manifests register"),
        vec![fsl_analyze::analyze::value_item(
            RequestIndex::new(0),
            &package,
            &claim,
            Ok(native_request),
        )],
        &Cancel::new(),
    );
    assert!(
        matches!(
            direct[0].terminal().map(|record| record.value()),
            Some(TerminalValue::Refuted)
        ),
        "native adapter settlement: {:?}; family: {:?}; node: {:?}",
        direct[0],
        fsl_analyze::claim_family::claim_family(
            &package,
            fsl_route::request::ItemKind::Formula,
            &claim
        ),
        package.graph().semantic_graph().node(claim.node().clone())
    );
    let provision = SourceProvision::single(package.graph().source(), &source);
    let request = AnalyzeRequest {
        binding: binding.clone(),
        provision: provision.clone(),
        items: vec![plain(
            claim.clone(),
            Capability::ValueValidity,
            fsl_analyze::value_check::manifest::BACKEND_ID,
        )],
    };
    let engines = Engines::fsl().expect("the manifests register");
    let mut limited = AnalyzeLimits::unbounded();
    limited.accounting.work_units = 0;
    let stopped = analyze(&package, &request, &engines, &limited, &Cancel::new())
        .expect("the bounded value request admits")
        .into_value();
    let served = provider::run(
        &package,
        &binding,
        &provision,
        &request.items,
        &engines,
        &limited,
        &Cancel::new(),
    );
    assert_eq!(stopped.records, served.records);
    assert_eq!(
        stopped.records[0].value(),
        &TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::EvaluationMeter {
                counter: filament_exact::LimitKind::WorkUnits
            },
            value: 0,
        })
    );
    assert!(stopped.counterexamples.is_empty());
    assert!(served.counterexamples.is_empty());
    let funded = analyze(
        &package,
        &request,
        &engines,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("the adequately funded request admits")
    .into_value();
    assert_eq!(funded.records[0].value(), &TerminalValue::Refuted);
    let result = provider::run(
        &package,
        &binding,
        &provision,
        &[plain(
            claim,
            Capability::ValueValidity,
            fsl_analyze::value_check::manifest::BACKEND_ID,
        )],
        &Engines::fsl().expect("the manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    );
    assert_eq!(
        result.records[0].value(),
        &TerminalValue::Refuted,
        "{:?}",
        result.records[0]
    );
    assert_eq!(
        result.records[0].backend().map(BackendId::as_str),
        Some(fsl_analyze::value_check::manifest::BACKEND_ID)
    );
    assert_eq!(result.counterexamples.len(), 1);
    let Counterexample::Assignments(assignment) = &result.counterexamples[0].counterexample else {
        panic!("native value assignment")
    };
    assert_eq!(assignment.parameters.len(), 1);
    let [filament_exact::Value::Integer(value)] = assignment.reproduced.assignment() else {
        panic!("one native integer assignment")
    };
    assert_eq!(value, &filament_exact::Integer::from(3_i64));
}

#[trace("TC-761", "FR-280-AC-2")]
#[test]
fn a_contract_claim_reaches_the_existing_contract_arm() {
    let world = counter();
    let clause = world
        .package
        .graph()
        .state_clause("CanInc")
        .expect("the precondition is checked");
    let claim = OccurrenceKey::new(clause.identity(), clause.claim().clone());
    let result = provider::run(
        &world.package,
        &world.binding(&["c"]),
        &world.provision,
        &[plain(
            claim,
            Capability::OperationContract,
            fsl_analyze::contract_check::manifest::BACKEND_ID,
        )],
        &Engines::fsl().expect("the manifests register"),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    );
    assert!(
        matches!(result.records[0].value(), TerminalValue::Proved { .. }),
        "{:?}",
        result.records[0]
    );
    assert!(result.counterexamples.is_empty());
    assert_eq!(
        result.records[0].backend().map(BackendId::as_str),
        Some(fsl_analyze::contract_check::manifest::BACKEND_ID)
    );
}

/// Omitted optional sampling settings use the engine's published defaults,
/// rather than turning a valid statistical item into an unsupported one.
#[trace("TC-761", "FR-280-AC-5")]
#[test]
fn omitted_optional_statistical_settings_equal_the_explicit_defaults() {
    let claim = "probabilistic Two using ev on M over (s: M::Server) under Steady on origin with alpha 0.01, beta 0.01, indifference 0.01 { quantile 0.95 of (accumulate duration from holds(s.phase = 1) until holds(s.phase = 2) within 10) <= 2 }";
    let world = prob_world::World::new(&prob_world::service(claim));
    let mut explicit = crate::analyze::routed_service_item(&world);
    let Some(ItemForm::Probabilistic(form)) = &mut explicit.item.form else {
        panic!("statistical form")
    };
    form.max_witnesses = Some(fsl_analyze::statistical::DEFAULT_MAX_WITNESSES);
    form.timed_sampling = Some(fsl_replay::statistical::TimedSampling::default());
    let mut omitted = explicit.clone();
    let Some(ItemForm::Probabilistic(form)) = &mut omitted.item.form else {
        panic!("statistical form")
    };
    form.max_witnesses = None;
    form.timed_sampling = None;
    let limits = AnalyzeLimits {
        accounting: prob_world::UNLIMITED,
        ..AnalyzeLimits::unbounded()
    };
    let engines = Engines::fsl().expect("the manifests register");
    let serve = |item| {
        provider::run(
            &world.package,
            &world.binding(),
            &world.provision,
            &[item],
            &engines,
            &limits,
            &Cancel::new(),
        )
    };
    let expected = serve(explicit);
    assert!(matches!(
        expected.records[0].value(),
        TerminalValue::Measured(_)
    ));
    assert_eq!(serve(omitted).records, expected.records);
}

fn registered_candidate(
    kind: Capability,
    evidence: Option<EvidenceKind>,
    backend: &str,
) -> Candidate {
    let engines = Engines::fsl().expect("the production manifests register");
    let mut registry = fsl_route::Registry::new();
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .expect("a unique backend");
    }
    let evidence = evidence
        .map(fsl_route::ItemEvidence::Kind)
        .unwrap_or(fsl_route::ItemEvidence::Absent);
    let requested = BackendId::new(backend);
    let fsl_route::CandidateOutcome::Candidates(candidates) =
        registry.candidates_for_evidence(kind, &evidence, Some(&requested))
    else {
        panic!("the named backend is registered")
    };
    let [candidate] = candidates.candidates() else {
        panic!("the registered named backend advertises this kind/evidence")
    };
    candidate.clone()
}

#[trace("TC-761", "FR-280-AC-2", "TC-637", "FR-202-AC-9")]
#[test]
fn exact_refutation_retains_the_reproduced_path_set_through_both_entries() {
    let source = LIKELY.replace("0.95", "0.97");
    let world = prob_world::World::new(&prob_world::link(&source));
    let request = AnalyzeRequest {
        binding: world.binding(),
        provision: world.provision.clone(),
        items: vec![exact(&world, ExactProbLimits::default())],
    };
    let engines = Engines::fsl().expect("the manifests register");
    let limits = AnalyzeLimits::unbounded();
    let outcome = analyze(&world.package, &request, &engines, &limits, &Cancel::new())
        .expect("the request admits")
        .into_value();
    let served = provider::run(
        &world.package,
        &request.binding,
        &request.provision,
        &request.items,
        &engines,
        &limits,
        &Cancel::new(),
    );
    assert_eq!(outcome.records, served.records);
    assert_eq!(outcome.records[0].value(), &TerminalValue::Refuted);
    assert!(outcome.records[0].exact_certificate().is_none());
    let system = fsl_eval::simulation::model::ModelSystem::new(world.subject().simulation)
        .expect("the checked subject admits");
    let expected_mass = filament_exact::Rational::new(
        filament_exact::Integer::from(1_i64),
        filament_exact::Integer::from(25_i64),
    )
    .unwrap();
    for counterexamples in [&outcome.counterexamples, &served.counterexamples] {
        assert_eq!(counterexamples.len(), 1);
        assert_eq!(counterexamples[0].request_index, RequestIndex::new(0));
        assert_eq!(counterexamples[0].backend.as_str(), "en-5");
        let Counterexample::Probabilistic(token) = &counterexamples[0].counterexample else {
            panic!("the exact refutation retains its typed reproduced token")
        };
        assert_eq!(token.mass(), &expected_mass);
        let evidence = token.evidence();
        assert_eq!(
            evidence.item,
            fsl_replay::probabilistic_claim_identity(world.claim("Likely"))
                .expect("the producer-owned claim has its checked identity")
        );
        let replay = fsl_replay::replay_probabilistic_witness(
            &world.package,
            &system,
            &evidence.item,
            evidence,
            &ExactProbLimits::default(),
            &Cancel::new(),
        )
        .expect("the retained witness replays on the operation's checked subject");
        let fsl_replay::ProbabilityWitnessReplay::Reproduced(replayed) = replay else {
            panic!("the retained witness must independently reproduce the same refutation")
        };
        assert_eq!(replayed.mass(), token.mass());
    }
}

#[trace("TC-762", "FR-281-AC-21")]
#[test]
fn claim_construction_resolves_the_complete_checked_occurrence() {
    let world = counter();
    let clause = &world.package.graph().temporal_clauses()[0];
    let claim = OccurrenceKey::new(clause.identity(), clause.claim().clone());
    let build = |key| {
        ClaimItem::new(
            &world.package,
            RequestIndex::new(0),
            key,
            Capability::TemporalSatisfaction,
            ExtentClassification::Unbounded {
                finite_bound_available: true,
            },
        )
    };
    assert_eq!(
        build(claim.clone())
            .expect("the checked clause resolves")
            .claim,
        claim
    );
    let wrong = OccurrenceKey::new(
        clause.identity(),
        filament_exact::Origin::new(filament_exact::Role::new("claim"), 99),
    );
    assert!(matches!(build(wrong), Err(ClaimRefusal::UnknownClaim)));
    let (other, _) = crate::value_validity::package_with_source(
        "function Other using v(x: Int[0,3]): Boolean pure { x < 2 }",
    );
    let (node, origin, _) = other
        .graph()
        .occurrences()
        .next()
        .expect("producer-owned occurrence");
    let absent = OccurrenceKey::new(node, origin);
    assert!(!world
        .package
        .graph()
        .occurrences()
        .any(|(node, _, _)| node == *absent.node()));
    assert!(matches!(build(absent), Err(ClaimRefusal::UnknownClaim)));
}
