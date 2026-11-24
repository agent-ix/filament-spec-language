// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-750 (TC-2100): `analyze` runs a hybrid item on the hybrid provider
//! negotiation routed it to, and settles a hybrid item with no provider
//! `unsupported`, `HybridProviderAbsent`, and one routed to a backend no
//! engine serves `unsupported`, `tool-unavailable`.

use filament_exact::Cancel;
use filament_semantic_value::catalog::CatalogCode;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_analyze::analyze::{
    AnalyzeLimits, AnalyzeRequest, ClaimItem, Engines, HybridItem, ItemForm, ItemOptions,
    NegotiatedItem, SourceProvision, SubjectBinding, analyze,
};
use fsl_analyze::provider::{self, HybridAnswer, HybridVerdict, UnsupportedConstruct};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::{BackendId, RequestIndex, SourceIdentity};
use fsl_package::CheckedPackage;
use fsl_replay::{
    CauseCodecError, DecidingElement, DisagreementCause, ExpressionSite, HybridTraceDisagreement,
    InconclusiveCause, SeparatingWitnessRecord, TermLocus, TerminalRecord, TerminalValue,
    UnavailabilityCause, Verdict, WitnessFailure,
};
use fsl_route::{Candidate, request::ExtentClassification, routing::Disposition};
use fsl_semantics::check::{Capability, ModeIndex, PackageDeclarations, VariableIndex};
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::{PackageInput, admit_unit, package_input};
use fsl_semantics::model::observation::DocumentProvision;
use ix_trace_rs::trace;
use serde_json::json;

use crate::hybrid_requests::{answering, entry_answer, point_trace, refuted_entry};

/// FR-447's `Braking`, `SlowEnough` (claim 0) and `Positive` (claim 1).
const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"filament.temporal.infinite-trace/v1\";\n\
     hybrid Braking using p {\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; }\n\
     }\n\
     temporal SlowEnough using p over (h: Braking) clock \"model-time\" \
     on origin { always [0, 10] holds(h.v < 30) }\n\
     temporal Positive using p over (h: Braking) clock \"model-time\" \
     on origin { always [0, 12] holds(h.v > 0) }\n";

/// The checked hybrid package, its empty model input and snapshots, and the
/// provision of its one source.
struct World {
    package: CheckedPackage,
    packages: PackageInput,
    snapshots: DocumentProvision,
    provision: SourceProvision,
}

fn world() -> World {
    world_of(UNIT)
}

fn world_of(unit_text: &str) -> World {
    let packages = package_input(std::iter::empty::<&[u8]>()).expect("no model documents");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:hybrid-provider"),
        "hybrid.native",
        unit_text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("no model selection");
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        models,
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &fsl_eval::value::S6aRewardEvaluator,
    )
    .expect("the unit checks");
    World {
        provision: SourceProvision::single(graph.source(), unit_text.as_bytes()),
        package: CheckedPackage::link(graph),
        packages,
        snapshots: DocumentProvision::new(),
    }
}

impl World {
    fn binding(&self) -> SubjectBinding<'_> {
        SubjectBinding {
            packages: &self.packages,
            snapshots: &self.snapshots,
            initial: Vec::new(),
            universes: Vec::new(),
        }
    }

    /// `SlowEnough`'s hybrid item with `candidates` and `disposition`.
    fn item(&self, candidates: Vec<Candidate>, disposition: Disposition) -> NegotiatedItem {
        self.item_at(0, candidates, disposition)
    }

    /// The hybrid item of the claim at `position` (0 `SlowEnough`, 1
    /// `Positive`).
    fn item_at(
        &self,
        position: usize,
        candidates: Vec<Candidate>,
        disposition: Disposition,
    ) -> NegotiatedItem {
        let site = &self.package.graph().hybrid_claim_sites()[position];
        NegotiatedItem {
            item: ClaimItem {
                request_index: RequestIndex::new(0),
                claim: OccurrenceKey::new(site.identity.clone(), site.claim.clone()),
                kind: Capability::TemporalSatisfaction,
                extent: ExtentClassification::Bounded,
                backend: None,
                candidates,
                options: ItemOptions {
                    required: true,
                    ..ItemOptions::default()
                },
                form: Some(ItemForm::Hybrid(HybridItem::default())),
            },
            disposition,
            diagnostic: None,
        }
    }

    /// The item's record through `analyze` and through `provider::run`.
    fn settle(&self, item: NegotiatedItem) -> [TerminalRecord; 2] {
        let engines = Engines::fsl().expect("the layer-A manifests register");
        self.settle_with(item, &engines, &AnalyzeLimits::unbounded())
    }

    /// [`Self::settle`] on `engines` under `limits`.
    fn settle_with(
        &self,
        item: NegotiatedItem,
        engines: &Engines,
        limits: &AnalyzeLimits,
    ) -> [TerminalRecord; 2] {
        let through_analyze = analyze(
            &self.package,
            &AnalyzeRequest {
                binding: self.binding(),
                provision: self.provision.clone(),
                items: vec![item.clone()],
            },
            engines,
            limits,
            &Cancel::new(),
        )
        .expect("the binding is admitted")
        .into_value()
        .records
        .remove(0);
        let through_run = provider::run(
            &self.package,
            &self.binding(),
            &self.provision,
            &[item],
            engines,
            limits,
            &Cancel::new(),
        )
        .records
        .remove(0);
        [through_analyze, through_run]
    }
}

/// FR-750-AC-3: with no hybrid provider registered (an empty candidate set),
/// the hybrid item settles `unsupported`, `HybridProviderAbsent`,
/// `unsupported_projection`/`unsupported-requested-capability`, through
/// `analyze` and through `provider::run`.
#[trace("TC-2100", "FR-750-AC-3")]
#[trace("TC-1187", "FR-447-AC-4")]
#[trace("FSpec-TC-373", "FSpec-FR-193-AC-7")]
#[test]
fn a_hybrid_item_with_no_provider_settles_hybrid_provider_absent() {
    let world = world();
    for record in world.settle(world.item(Vec::new(), Disposition::Unsupported)) {
        assert_eq!(
            record.value(),
            &TerminalValue::Unsupported(UnavailabilityCause::HybridProviderAbsent),
            "{record:?}"
        );
        assert_eq!(
            record.diagnostic(),
            Some(CatalogCode::new(
                "unsupported_projection",
                "unsupported-requested-capability"
            ))
        );
        let detail = record.diagnostic_detail().expect("a diagnostic detail");
        assert_eq!(detail["kind"], "temporal-satisfaction", "{record:?}");
        assert_eq!(detail["requirement"], "hybrid-dynamics", "{record:?}");
    }
}

/// FR-750-AC-4: a hybrid item routed to `hybrid-fixture`, which no
/// registered engine or process provider serves, settles `unsupported`,
/// `unsupported_projection`/`tool-unavailable`, naming that backend.
#[trace("TC-2100", "FR-750-AC-4")]
#[test]
fn a_hybrid_item_routed_to_an_unserved_backend_settles_tool_unavailable() {
    let world = world();
    let fixture = Candidate::new(BackendId::new("hybrid-fixture"));
    for record in world.settle(world.item(
        vec![fixture.clone()],
        Disposition::Supported(fixture.clone()),
    )) {
        assert!(
            matches!(record.value(), TerminalValue::Unsupported(_)),
            "{record:?}"
        );
        assert_eq!(
            record.diagnostic(),
            Some(CatalogCode::new(
                "unsupported_projection",
                "tool-unavailable"
            )),
            "{record:?}"
        );
        assert_eq!(
            record.backend().map(BackendId::as_str),
            Some("hybrid-fixture")
        );
    }
}

/// The fixture engine `hybrid-fixture` answering by `answer`, registered
/// beside the layer-A engines, and the candidate that names it.
fn fixture(
    answer: impl Fn(&fsl_analyze::provider::HybridSolveRequest<'_>) -> HybridAnswer
    + Send
    + Sync
    + 'static,
) -> (
    Engines,
    Candidate,
    std::sync::Arc<crate::hybrid_requests::Seen>,
) {
    let (engine, seen) = answering("hybrid-fixture", answer);
    let mut engines = Engines::fsl().expect("the layer-A manifests register");
    engines.with(engine).expect("the fixture id is new");
    (
        engines,
        Candidate::new(BackendId::new("hybrid-fixture")),
        seen,
    )
}

/// Settle the claim at `position` on the fixture `answer`.
fn settle_on(
    world: &World,
    position: usize,
    answer: impl Fn(&fsl_analyze::provider::HybridSolveRequest<'_>) -> HybridAnswer
    + Send
    + Sync
    + 'static,
) -> [TerminalRecord; 2] {
    let (engines, candidate, _) = fixture(answer);
    let item = world.item_at(
        position,
        vec![candidate.clone()],
        Disposition::Supported(candidate),
    );
    world.settle_with(item, &engines, &AnalyzeLimits::unbounded())
}

/// FR-750-AC-7 (in process), FSpec FR-272-AC-31: a provider's `Unsupported`
/// verdict settles `unsupported`, `unsupported_construct`/`expression-form`
/// with the construct's declaration, mode, variable and located `locus`, the
/// backend in place of the profile and no counterexample; a mode the model
/// does not have settles `failed`.
#[trace("TC-3035", "FR-750-AC-7")]
#[trace("FSpec-TC-2049", "FSpec-FR-272-AC-31")]
#[test]
fn an_unsupported_verdict_settles_a_located_expression_form() {
    let world = world();
    let unsupported = |mode: usize| {
        move |_: &fsl_analyze::provider::HybridSolveRequest<'_>| HybridAnswer {
            verdict: HybridVerdict::Unsupported(UnsupportedConstruct {
                form: "division".to_owned(),
                site: ExpressionSite::Rate { mode, variable: 0 },
                term: TermLocus::default(),
                mode: ModeIndex::new(mode),
                variable: Some(VariableIndex::new(0)),
            }),
            certificate: None,
        }
    };
    for record in settle_on(&world, 0, unsupported(0)) {
        assert!(
            matches!(record.value(), TerminalValue::Unsupported(_)),
            "{record:?}"
        );
        assert_eq!(
            record.diagnostic(),
            Some(CatalogCode::new("unsupported_construct", "expression-form")),
            "{record:?}"
        );
        assert_eq!(
            record.backend().map(BackendId::as_str),
            Some("hybrid-fixture")
        );
        let detail = record.diagnostic_detail().expect("a diagnostic detail");
        assert_eq!(detail["form"], "division", "{detail:?}");
        assert_eq!(detail["declaration"], "Braking", "{detail:?}");
        assert_eq!(detail["mode"], "Moving", "{detail:?}");
        assert_eq!(detail["variable"], "v", "{detail:?}");
        assert_eq!(detail["backend"], "hybrid-fixture", "{detail:?}");
        let locus = &detail["locus"];
        assert!(locus["node"].is_object(), "{detail:?}");
        assert_eq!(locus["occurrence"]["role"], "expression", "{detail:?}");
    }
    for record in settle_on(&world, 0, unsupported(7)) {
        assert!(
            matches!(record.value(), TerminalValue::Failed),
            "{record:?}"
        );
        assert_eq!(
            record.diagnostic(),
            Some(CatalogCode::new(
                "runtime_invariant",
                "established-invariant-broken"
            )),
            "{record:?}"
        );
    }
}

/// FR-750-AC-8 (in process): a refuted entry whose trace replays settles
/// `refuted`; the same trace that does not reproduce settles `inconclusive`,
/// `replay-parity` carrying the claim's trace, never `solver-inconclusive`;
/// a solver `unknown` with a trace that does not replay keeps its
/// `solver-inconclusive` cause.
#[trace("TC-3035", "FR-750-AC-8")]
#[test]
fn a_refuted_trace_that_does_not_reproduce_settles_replay_parity() {
    let world = world();
    for record in settle_on(&world, 1, entry_answer(refuted_entry(point_trace("0")))) {
        assert_eq!(record.value(), &TerminalValue::Refuted, "{record:?}");
    }
    for record in settle_on(&world, 1, entry_answer(refuted_entry(point_trace("1")))) {
        assert!(
            matches!(
                record.value(),
                TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
                    DisagreementCause::HybridTrace(HybridTraceDisagreement::NotReproduced { .. })
                ))
            ),
            "{record:?}"
        );
    }
    let unknown = json!({"result": "inconclusive",
        "cause": {"type": "solver-inconclusive", "trace": point_trace("1")}});
    for record in settle_on(&world, 1, entry_answer(unknown)) {
        assert!(
            matches!(
                record.value(),
                TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive { .. })
            ),
            "{record:?}"
        );
    }
}

/// FR-269-AC-5 (TC-1221): hybrid disagreements use the shared cause codec,
/// whose mode and variable names are resolved through the checked model;
/// the prior disagreement arms retain their model-independent wire.
#[test]
#[trace("TC-1221", "FR-269-AC-5")]
fn hybrid_disagreement_causes_round_trip_through_the_shared_codec() {
    use filament_semantic_value::catalog::Category;
    use fsl_foundation::witness::{
        InvocationBinding, ObservationIdentity, RuntimeValuePath, ValuePathSubject,
    };

    let world = world();
    let record = settle_on(&world, 1, entry_answer(refuted_entry(point_trace("1"))))
        .into_iter()
        .next()
        .expect("the provider result settles one claim");
    let TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(cause)) = record.value() else {
        panic!("a non-reproducing provider trace is a hybrid disagreement: {record:?}");
    };
    assert!(matches!(
        cause,
        DisagreementCause::HybridTrace(HybridTraceDisagreement::NotReproduced { .. })
    ));
    assert_eq!(cause.proved(), None);
    assert_eq!(cause.replayed(), None);

    let checked_model = &world.package.graph().hybrid_models()[0];
    let text = cause
        .to_json(Some(checked_model))
        .expect("a hybrid trace is encoded with its checked model");
    let wire: serde_json::Value = serde_json::from_str(&text).expect("the cause wire is JSON");
    assert_eq!(wire["cause"], "hybrid-trace", "{wire}");
    assert_eq!(wire["disagreement"]["type"], "not-reproduced", "{wire}");
    let counterexample = &wire["disagreement"]["counterexample"];
    assert_eq!(counterexample["type"], "hybrid-counterexample", "{wire}");
    assert_eq!(counterexample["trace"]["start"]["mode"], "Moving", "{wire}");
    assert_eq!(
        counterexample["trace"]["start"]["enclosure"],
        json!({"a": {"lower": {"numerator":"2", "denominator":"1"},
                      "upper": {"numerator":"2", "denominator":"1"}},
              "v": {"lower": {"numerator":"20", "denominator":"1"},
                      "upper": {"numerator":"20", "denominator":"1"}}}),
        "the names resolve through the checked model: {wire}"
    );
    assert_eq!(
        DisagreementCause::from_json(&text, &world.package, Some(checked_model))
            .expect("the checked model resolves the hybrid wire"),
        *cause
    );
    assert!(matches!(
        cause.to_json(None),
        Err(CauseCodecError::MissingHybridModel)
    ));
    assert!(matches!(
        DisagreementCause::from_json(&text, &world.package, None),
        Err(CauseCodecError::MissingHybridModel)
    ));

    let counterexample = match cause {
        DisagreementCause::HybridTrace(disagreement) => disagreement.counterexample().clone(),
        _ => unreachable!(),
    };
    let not_exact = DisagreementCause::HybridTrace(HybridTraceDisagreement::NotExact {
        counterexample: Box::new(counterexample),
    });
    let not_exact_text = not_exact
        .to_json(Some(checked_model))
        .expect("not-exact has the same model-aware wire");
    let not_exact_wire: serde_json::Value =
        serde_json::from_str(&not_exact_text).expect("the not-exact wire is JSON");
    assert_eq!(not_exact_wire["disagreement"]["type"], "not-exact");
    assert_eq!(
        DisagreementCause::from_json(&not_exact_text, &world.package, Some(checked_model))
            .expect("not-exact decodes through the checked model"),
        not_exact
    );

    let success = Verdict::from_category(Category::Success);
    let violation = Verdict::from_category(Category::Violation);
    let claim_site = &world.package.graph().hybrid_claim_sites()[1];
    let witness = SeparatingWitnessRecord {
        quantifier: Some(OccurrenceKey::new(
            claim_site.identity.clone(),
            claim_site.claim.clone(),
        )),
        deciding_comparison: None,
        operands: None,
        decision_path: None,
        deciding_element: DecidingElement::Value(filament_exact::Value::Boolean(false)),
        index: Some(0),
        value_path: RuntimeValuePath {
            observation: ObservationIdentity {
                authority: "test".to_owned(),
                identity: "invocation".to_owned(),
            },
            subject: ValuePathSubject::Invocation {
                invocation: ObservationIdentity {
                    authority: "test".to_owned(),
                    identity: "invocation".to_owned(),
                },
                binding: InvocationBinding::Receiver,
            },
            steps: Vec::new(),
        },
        trace_position: None,
    };
    for existing in [
        DisagreementCause::Verdicts {
            proved: violation,
            replayed: success,
        },
        DisagreementCause::NoValue {
            proved: violation,
            replayed: success,
        },
        DisagreementCause::Witness {
            proved: violation,
            replayed: violation,
            given: Some(Box::new(witness.clone())),
            derived: None,
            failure: WitnessFailure::Mismatch,
        },
    ] {
        let old_text = existing
            .to_json(None)
            .expect("existing disagreement arms remain encodable");
        assert_eq!(
            DisagreementCause::from_json(&old_text, &world.package, None)
                .expect("existing disagreement arms remain decodable"),
            existing
        );
    }
}

/// FR-750: the request's deadline reaches the provider as the caller's
/// `AnalyzeLimits.deadline`, and an unbounded deadline reaches it as none.
/// (No FR-750 criterion states this; it is the request context FR-750 gives
/// a provider, so the test carries no criterion tag.)
#[test]
fn the_providers_deadline_is_the_requests_deadline() {
    use fsl_analyze::analyze::DeadlineSupply;
    let world = world();
    let entry = json!({"request_index": 0, "result": "incomplete",
        "cause": {"type": "timed-out"}});
    for (supply, expected) in [
        (DeadlineSupply::Milliseconds(250), Some(250)),
        (DeadlineSupply::Unbounded, None),
    ] {
        let (engines, candidate, seen) = fixture(entry_answer(entry.clone()));
        let item = world.item(vec![candidate.clone()], Disposition::Supported(candidate));
        let mut limits = AnalyzeLimits::unbounded();
        limits.deadline = supply;
        world.settle_with(item, &engines, &limits);
        let requests = seen.requests();
        assert!(!requests.is_empty());
        assert!(requests.iter().all(|(_, deadline)| *deadline == expected));
    }
}

/// FR-750: a provider trace the model cannot hold (a mode it does not
/// have) is the provider's fault and settles `failed`, `invalid_result`,
/// `invalid-value`.
#[trace("TC-3035", "FR-750-AC-8")]
#[test]
fn a_trace_outside_the_model_settles_invalid_result() {
    let world = world();
    let entry = refuted_entry(point_trace("0"));
    let read = entry_answer(entry);
    let answer = move |request: &fsl_analyze::provider::HybridSolveRequest<'_>| {
        let mut answer = read(request);
        if let HybridVerdict::Refuted(found) = &mut answer.verdict {
            found.trace.start.mode = ModeIndex::new(9);
        }
        answer
    };
    for record in settle_on(&world, 1, answer) {
        assert!(
            matches!(record.value(), TerminalValue::Failed),
            "{record:?}"
        );
        assert_eq!(
            record.diagnostic(),
            Some(CatalogCode::new("invalid_result", "invalid-value")),
            "{record:?}"
        );
    }
}

/// FR-1019's class fixture `BrakingC` with the initial condition `i`, the
/// `Moving` flow `f` for `v` and the guard `g`.
fn class_unit(i: &str, f: &str, g: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid BrakingC using p {{\n\
           initial {{ mode Moving when {{ {i} }}; }}\n\
           mode Moving invariant {{ v >= 0 and v <= 20 and a >= 1 and a <= 2 }}\n\
             flow {{ v' = {f}; a' = 0; }}\n\
             transition to Stopped when {{ {g} }} reset {{ v = 0 }};\n\
           mode Stopped invariant {{ v = 0 }}\n\
             flow {{ v' = 0; a' = 0; }}\n\
         }}\n\
         function half using p(x: Rational[0, 20; 1, 2], d: Rational[2, 2; 1, 1]): Rational[-100, 100; 1, 4] pure {{ x / d }}\n\
         temporal SlowEnoughC using p over (h: BrakingC) clock \"model-time\" \
         on origin {{ always [0, 10] holds(h.v < 30) }}\n"
    )
}

/// The first construct of the class fixture `(i, f, g)` outside the
/// reference provider's class, if any.
fn class_gap(i: &str, f: &str, g: &str) -> Option<UnsupportedConstruct> {
    let world = world_of(&class_unit(i, f, g));
    let graph = world.package.graph();
    let model = &graph.hybrid_models()[0];
    fsl_replay::polynomial_class_gap(
        graph,
        model,
        &mut fsl_replay::HybridReplayMeter::new(1_000_000),
    )
    .expect("the class check finishes")
}

/// FR-1019-AC-14: the first construct outside the class is found in source
/// order (initial entries first), the outermost of nested constructs is
/// reported, a construct inside an expanded call is reported inside the
/// called function's body, and a rational coefficient and a Boolean `if`
/// stay inside the class.
#[trace("TC-2869", "FR-1019-AC-14")]
#[test]
fn the_class_check_locates_the_first_construct_in_source_order() {
    let moving = ModeIndex::new(0);
    let v = Some(VariableIndex::new(0));
    // C1: the initial entry's `/`, not the later flow's.
    let c1 = class_gap("v = 40 / 2 and a = 1", "-a / (1 + v)", "v = 0").expect("C1");
    assert_eq!(c1.form, "division");
    assert_eq!(c1.site, ExpressionSite::Initial { entry: 0 });
    assert_eq!(c1.term.term, vec![0, 1]);
    assert!(c1.term.body.is_empty());
    assert_eq!((c1.mode, c1.variable), (moving, None));
    // C2: the outer `/`, the first node of the pre-order walk.
    let c2 = class_gap("v = 20 and a = 1", "-a / (1 + v / 2)", "v = 0").expect("C2");
    assert_eq!(c2.form, "division");
    assert_eq!(
        c2.site,
        ExpressionSite::Rate {
            mode: 0,
            variable: 0
        }
    );
    assert!(
        c2.term.term.is_empty() || c2.term.term == [0],
        "the outer quotient, not the inner: {:?}",
        c2.term
    );
    assert_eq!((c2.mode, c2.variable), (moving, v));
    // C3: the numeric `if`.
    let c3 = class_gap("v = 20 and a = 1", "if v > 1 then -a else 0", "v = 0").expect("C3");
    assert_eq!(c3.form, "conditional");
    assert_eq!(
        c3.site,
        ExpressionSite::Rate {
            mode: 0,
            variable: 0
        }
    );
    assert!(c3.term.term.is_empty(), "{:?}", c3.term);
    assert_eq!((c3.mode, c3.variable), (moving, v));
    // C4: the `/` in `half`'s body, at its own node inside the function.
    let c4 = class_gap("v = 20 and a = 1", "-a", "half(v, rational(2, 1)) <= 0").expect("C4");
    assert_eq!(c4.form, "division");
    assert_eq!(
        c4.site,
        ExpressionSite::Guard {
            mode: 0,
            transition: 0
        }
    );
    assert_eq!(c4.term.term, vec![0]);
    assert_eq!(c4.term.body.len(), 1, "inside half's body: {:?}", c4.term);
    assert_eq!((c4.mode, c4.variable), (moving, None));
    // C5 and C6 are inside the class.
    assert_eq!(
        class_gap("v = 20 and a = 1", "-rational(1, 2) * a", "v = 0"),
        None
    );
    assert_eq!(
        class_gap("v = 20 and a = 1", "-a", "if a > 1 then v = 0 else v <= 0"),
        None
    );
}

/// FR-1019-AC-14: a mode's flow equations are walked in the declaration
/// order of the variables they define, not the order the mode writes them:
/// `Late` writes `a'` before `v'`, both outside the class, and the first
/// construct reported is `v`'s.
#[trace("TC-2869", "FR-1019-AC-14")]
#[test]
fn flow_equations_are_walked_in_variable_declaration_order() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile p = \"filament.temporal.infinite-trace/v1\";\n\
         hybrid Order using p {\n\
           initial { mode Early when { v = 1 and a = 1 }; }\n\
           mode Early invariant { v >= 0 and a >= 0 } flow { v' = 1; a' = 1; }\n\
           mode Late invariant { v >= 0 and a >= 0 } flow { a' = 1 / 2; v' = 1 / 2; }\n\
         }\n\
         temporal Holds using p over (h: Order) clock \"model-time\" \
         on origin { always [0, 1] holds(h.v >= 0) }\n";
    let world = world_of(unit);
    let graph = world.package.graph();
    let gap = fsl_replay::polynomial_class_gap(
        graph,
        &graph.hybrid_models()[0],
        &mut fsl_replay::HybridReplayMeter::new(1_000_000),
    )
    .expect("the class check finishes")
    .expect("`Late`'s flows are outside the class");
    assert_eq!(gap.form, "division");
    assert_eq!(
        gap.site,
        ExpressionSite::Rate {
            mode: 1,
            variable: 0
        }
    );
    assert_eq!(gap.variable, Some(VariableIndex::new(0)));
}
