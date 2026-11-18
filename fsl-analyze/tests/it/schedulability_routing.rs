// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSL FR-246 (TC-701 step 1): each checked schedulability claim carries the
//! FR-290 kind `schedulability`, and FR-075's request writer routes it to EN-7,
//! the one backend whose manifest advertises that kind.

use filament_exact::{Origin, Role};
use fsl_analyze::schedulability::manifest;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::Span;
use fsl_route::request::items_from_requirements;
use fsl_route::{CandidateOutcome, ProviderOrigin, Registry};
use fsl_semantics::check::{
    Capability, CheckedScheduleClaim, RequirementMap, RequirementRecord, ScheduleClaim,
    SchedulingPolicy,
};
use ix_trace_rs::trace;

fn claim(name: &str, policy: SchedulingPolicy) -> CheckedScheduleClaim {
    CheckedScheduleClaim {
        name: name.to_owned(),
        task_set: "Ctl".to_owned(),
        claim: ScheduleClaim::Schedulable(policy),
        node_id: None,
        occurrence: 0,
        span: Span { start: 0, end: 1 },
    }
}

#[test]
#[trace("TC-701")]
fn en_7_registers_and_a_schedulability_claim_routes_to_it() {
    let mut registry = Registry::new();
    registry
        .register(
            manifest::descriptor(ProviderOrigin::Linked).expect("EN-7's manifest is admitted"),
        )
        .expect("EN-7 registers");
    let records: RequirementMap = [
        ("CtlFp", SchedulingPolicy::FixedPriority, 1_u8),
        ("CtlEdf", SchedulingPolicy::Edf, 2),
    ]
    .into_iter()
    .map(|(name, policy, fill)| {
        (
            OccurrenceKey::new(
                crate::fixture_ids::node_key([fill]),
                Origin::new(Role::new("claim"), 0),
            ),
            RequirementRecord::model_claim(
                claim(name, policy).requirements(),
                crate::fixture_ids::node_key([99]),
            ),
        )
    })
    .collect();
    let items = items_from_requirements(&records, &registry, None);
    assert_eq!(items.len(), 2);
    for item in &items {
        assert_eq!(item.kind(), Capability::Schedulability);
        let CandidateOutcome::Candidates(set) = item.candidates() else {
            panic!("a candidate set: {:?}", item.candidates());
        };
        let ids: Vec<_> = set.candidates().iter().map(|c| c.id().as_str()).collect();
        assert_eq!(ids, ["en-7"]);
    }
}

/// ADR-026 §12's `Ctl` with the third WCET 5, and the response claims at the
/// fixpoint 12 and one below it.
const CTL: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile t = \"filament.temporal.complete/v1\";\n\
     dimension Time;\nunit ms : Time = rational(1, 1);\n\
     taskset Ctl {\n\
     task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
     task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
     task c periodic 12 ms deadline 12 ms wcet 5 ms priority 3 lo;\n\
     }\n\
     response Twelve on Ctl.c <= 12 ms under fixed_priority;\n\
     response Eleven on Ctl.c <= 11 ms under fixed_priority;\n";

/// The checked `Ctl` package, its empty model input and snapshots, and the
/// provision of its one source.
struct World {
    package: fsl_package::CheckedPackage,
    packages: fsl_semantics::model::intake::PackageInput,
    snapshots: fsl_semantics::model::observation::DocumentProvision,
    provision: fsl_analyze::analyze::SourceProvision,
}

impl World {
    fn new() -> Self {
        use filament_semantic_value::checking::{CheckMode, CheckingLimits};
        let packages = fsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>())
            .expect("no model documents");
        let parsed = fsl_cst::parse(
            fsl_foundation::SourceIdentity::new("agent-ix", "test:schedulability-routing"),
            "ctl.native",
            CTL.as_bytes(),
            fsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let models = fsl_semantics::model::intake::admit_unit(
            &unit.selections().models,
            &packages,
            fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        )
        .expect("no model selection");
        let graph = fsl_semantics::check::PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "fixture".to_owned(),
            unit,
            models,
            Vec::new(),
        )
        .expect("the unit assembles")
        .check(CheckMode::Linked, CheckingLimits::default())
        .expect("the unit checks");
        Self {
            provision: fsl_analyze::analyze::SourceProvision::single(
                graph.source(),
                CTL.as_bytes(),
            ),
            package: fsl_package::CheckedPackage::link(graph),
            packages,
            snapshots: fsl_semantics::model::observation::DocumentProvision::new(),
        }
    }

    fn claim(&self, name: &str) -> OccurrenceKey {
        self.package
            .graph()
            .task_sets()
            .claims()
            .iter()
            .find(|claim| claim.name == name)
            .and_then(CheckedScheduleClaim::occurrence)
            .expect("an authored schedulability claim")
    }

    /// `name`'s item at `index`, its disposition settled from the candidates
    /// `registry` computes for the checked claim's own requirement (FR-246:
    /// `schedulability`), by FR-057's table.
    fn routed(
        &self,
        registry: &Registry,
        index: usize,
        name: &str,
    ) -> fsl_analyze::analyze::NegotiatedItem {
        use fsl_analyze::analyze::{ClaimItem, NegotiatedItem};
        use fsl_route::routing::Disposition;
        use fsl_route::NegotiationSettlement;
        let checked = self
            .package
            .graph()
            .task_sets()
            .claims()
            .iter()
            .find(|claim| claim.name == name)
            .expect("an authored schedulability claim");
        let kind = checked.requirements().kind();
        assert_eq!(kind, Capability::Schedulability);
        let claim = self.claim(name);
        let mut item = ClaimItem::new(
            &self.package,
            fsl_foundation::RequestIndex::new(index),
            claim,
            kind,
            fsl_route::request::ExtentClassification::Unbounded {
                finite_bound_available: false,
            },
        )
        .expect("a checked schedulability claim");
        let (disposition, diagnostic) =
            match fsl_route::settle_candidates(&registry.candidates(kind, None), kind, None) {
                NegotiationSettlement::Single(backend) => {
                    let candidate = fsl_route::Candidate::new(backend);
                    item.candidates = vec![candidate.clone()];
                    (Disposition::Supported(candidate), None)
                }
                NegotiationSettlement::Cause(cause) => (
                    Disposition::Unsupported,
                    Some(fsl_analyze::analyze::Diagnostic::new(cause.code())),
                ),
                NegotiationSettlement::Inconsistent => (Disposition::InvalidRequest, None),
            };
        NegotiatedItem {
            item,
            disposition,
            diagnostic,
        }
    }

    fn analyze(
        &self,
        engines: &fsl_analyze::analyze::Engines,
        items: Vec<fsl_analyze::analyze::NegotiatedItem>,
    ) -> fsl_analyze::analyze::AnalyzeOutcome {
        use fsl_analyze::analyze::{AnalyzeLimits, AnalyzeRequest, SubjectBinding};
        fsl_analyze::analyze::analyze(
            &self.package,
            &AnalyzeRequest {
                binding: SubjectBinding {
                    packages: &self.packages,
                    snapshots: &self.snapshots,
                    initial: Vec::new(),
                    universes: Vec::new(),
                },
                provision: self.provision.clone(),
                items,
            },
            engines,
            &AnalyzeLimits::unbounded(),
            &filament_exact::Cancel::new(),
        )
        .expect("the empty binding admits")
        .into_value()
    }
}

/// The registry of `engines`' descriptors (FR-489).
fn registry_of(engines: &fsl_analyze::analyze::Engines) -> Registry {
    let mut registry = Registry::new();
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .expect("each linked descriptor registers");
    }
    registry
}

/// EN-7's direct result over `Ctl` with `c`'s response bounded by `bound`:
/// its outcome and FR-248's settlement of it.
fn direct(bound: i64) -> (fsl_replay::ClosedFormOutcome, fsl_replay::TerminalValue) {
    use filament_exact::{Identifier, Integer, Rational};
    use fsl_replay::{Arrival, Criticality, QualifiedName, ScheduleSubject, Task, TaskSet, Wcet};
    let n = |value: i64| {
        Rational::new(Integer::from(value), Integer::from(1_i64)).expect("a nonzero divisor")
    };
    let id = |name: &str| Identifier::new(name).expect("an identifier");
    let task = |name: &str, wcet: i64, period: i64, priority: u64| Task {
        name: id(name),
        arrival: Arrival::Periodic { period: n(period) },
        deadline: n(period),
        wcet: Wcet::Single(n(wcet)),
        priority: Some(priority),
        criticality: Criticality::Lo,
        jitter: n(0),
        blocking: n(0),
    };
    let set = TaskSet::new(
        QualifiedName::new(vec![id("Ctl")]).expect("one segment"),
        vec![task("a", 1, 4, 1), task("b", 2, 6, 2), task("c", 5, 12, 3)],
    )
    .expect("an admitted task set");
    let subject = ScheduleSubject::new(
        set,
        fsl_replay::ScheduleClaim::Response {
            task: id("c"),
            bound: n(bound),
        },
    )
    .expect("an admitted claim");
    let cancel = filament_exact::Cancel::new();
    let outcome = fsl_analyze::schedulability::analyze(
        &subject,
        &fsl_analyze::schedulability::ScheduleLimits::default(),
        &cancel,
    );
    let value = fsl_replay::settle(&subject, &outcome, &cancel);
    (outcome, value)
}

/// FR-620-AC-8, FR-281-AC-23, FR-489-AC-5: `Engines::fsl()` registers EN-7
/// under `en-7`, advertising `schedulability` in the `unbounded` mode.
#[test]
#[trace("TC-1580", "FR-620-AC-8")]
#[trace("TC-1580", "FR-281-AC-23")]
#[trace("TC-1272", "FR-489-AC-5")]
fn schedulability_is_registered() {
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the engines register");
    let en7 = engines
        .registrations()
        .find(|descriptor| descriptor.id().as_str() == manifest::BACKEND_ID)
        .expect("Engines::fsl() registers en-7");
    assert_eq!(
        en7.advertises().collect::<Vec<_>>(),
        [(Capability::Schedulability, fsl_route::Mode::Unbounded)]
    );
    assert_eq!(
        engines
            .registrations()
            .filter(|descriptor| descriptor
                .advertises()
                .any(|(kind, _)| kind == Capability::Schedulability))
            .count(),
        1,
        "en-7 is the one engine that serves schedulability"
    );
}

/// FR-281-AC-23, FR-633-AC-2: a response claim routed through the registry
/// of `Engines::fsl()` runs on EN-7 and settles as EN-7 called directly does:
/// `proved` at the fixpoint 12 and `refuted` at 11, with the same evidence.
#[test]
#[trace("TC-1580", "FR-281-AC-23")]
#[trace("TC-1593", "FR-633-AC-2")]
fn registry_route_matches_direct_call() {
    use fsl_foundation::RequestIndex;
    let world = World::new();
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the engines register");
    let registry = registry_of(&engines);
    let items = vec![
        world.routed(&registry, 0, "Twelve"),
        world.routed(&registry, 1, "Eleven"),
    ];
    for item in &items {
        assert_eq!(
            item.disposition,
            fsl_route::routing::Disposition::Supported(fsl_route::Candidate::new(
                fsl_foundation::BackendId::new("en-7")
            ))
        );
    }
    let outcome = world.analyze(&engines, items);
    for (index, bound, proved) in [(0, 12, true), (1, 11, false)] {
        let (direct_outcome, direct_value) = direct(bound);
        let record = &outcome.records[index];
        assert_eq!(record.value(), &direct_value, "bound {bound}");
        assert_eq!(
            matches!(record.value(), fsl_replay::TerminalValue::Proved { .. }),
            proved,
            "bound {bound}"
        );
        assert_eq!(
            matches!(record.value(), fsl_replay::TerminalValue::Refuted),
            !proved,
            "bound {bound}"
        );
        assert_eq!(record.backend().map(|id| id.as_str()), Some("en-7"));
        assert_eq!(
            outcome.closed_form.get(&RequestIndex::new(index)),
            Some(&direct_outcome),
            "bound {bound}: the routed evidence is EN-7's"
        );
    }
    let fsl_replay::ClosedFormOutcome::Miss(fsl_replay::MissEvidence::FixedPriority {
        task,
        iterates,
        ..
    }) = &outcome.closed_form[&RequestIndex::new(1)]
    else {
        panic!("a fixed-priority miss: {:?}", outcome.closed_form);
    };
    assert_eq!(task.as_str(), "c");
    assert_eq!(
        iterates.last(),
        Some(
            &filament_exact::Rational::new(
                filament_exact::Integer::from(12_i64),
                filament_exact::Integer::from(1_i64)
            )
            .expect("a nonzero divisor")
        ),
        "the last iterate, 12, exceeds the bound 11"
    );
}

/// FR-281 rule 4, FR-633 "Refusals": without EN-7 the registry computes no
/// candidate for a schedulability claim, so it settles `unsupported` with the
/// typed no-engine refusal, and an item still dispositioned to `en-7`
/// settles `invalid-request` `unknown-backend` without running.
#[test]
#[trace("TC-1580", "FR-281-AC-23")]
fn without_en7_a_schedulability_claim_has_no_engine() {
    let world = World::new();
    let mut without = fsl_analyze::analyze::Engines::default();
    for engine in [
        fsl_analyze::analyze::Engine::model_check(),
        fsl_analyze::analyze::Engine::statistical(),
        fsl_analyze::analyze::Engine::exact_probabilistic(),
        fsl_analyze::analyze::Engine::zone_check(),
        fsl_analyze::analyze::Engine::value_check(),
        fsl_analyze::analyze::Engine::contract_check(),
        fsl_analyze::analyze::Engine::protocol_analysis(),
    ] {
        without
            .with(engine.expect("the engine registers"))
            .expect("distinct backends");
    }
    let unrouted = world.routed(&registry_of(&without), 0, "Twelve");
    assert_eq!(
        unrouted.disposition,
        fsl_route::routing::Disposition::Unsupported
    );
    let full = fsl_analyze::analyze::Engines::fsl().expect("the engines register");
    let routed = world.routed(&registry_of(&full), 1, "Twelve");
    let outcome = world.analyze(&without, vec![unrouted, routed]);
    let unsupported = &outcome.records[0];
    assert!(
        matches!(
            unsupported.value(),
            fsl_replay::TerminalValue::Unsupported(_)
        ),
        "{unsupported:?}"
    );
    assert_eq!(
        unsupported.diagnostic().map(|code| code.cause()),
        Some("unsupported-requested-capability")
    );
    let unknown = &outcome.records[1];
    assert!(
        matches!(unknown.value(), fsl_replay::TerminalValue::Declined { .. }),
        "{unknown:?}"
    );
    assert_eq!(
        unknown.diagnostic().map(|code| code.cause()),
        Some("unknown-backend")
    );
    assert!(outcome.closed_form.is_empty(), "no item ran on EN-7");
}

/// FR-246-AC-4: S3 writes one `schedulability` requirement record per
/// task-set claim, so FR-075's request builder routes each to EN-7.
#[test]
#[trace("TC-2961", "FR-246-AC-4")]
#[ignore = "QSL-2131: the checked graph writes no requirement record for a task-set claim yet"]
fn task_set_claims_have_requirement_records_that_route_to_en_7() {
    let world = World::new();
    let records = world.package.graph().requirements();
    let schedulability: Vec<_> = records
        .iter()
        .filter(|(_, record)| record.requirements().kind() == Capability::Schedulability)
        .collect();
    assert_eq!(schedulability.len(), 2, "one record per response claim");
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the engines register");
    let items = items_from_requirements(records, &registry_of(&engines), None);
    let routed: Vec<_> = items
        .iter()
        .filter(|item| item.kind() == Capability::Schedulability)
        .collect();
    assert_eq!(routed.len(), 2);
    for item in routed {
        let CandidateOutcome::Candidates(set) = item.candidates() else {
            panic!("EN-7 is a candidate");
        };
        let ids: Vec<_> = set.candidates().iter().map(|c| c.id().as_str()).collect();
        assert_eq!(ids, [manifest::BACKEND_ID]);
    }
}

/// ADR-026 §12's `Ctl` with the third WCET 6, which misses under both
/// policies, and its two schedulability claims.
const CTL_MISSES: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile t = \"filament.temporal.complete/v1\";\n\
     dimension Time;\nunit ms : Time = rational(1, 1);\n\
     taskset Ctl {\n\
     task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;\n\
     task b periodic 6 ms deadline 6 ms wcet 2 ms priority 2 lo;\n\
     task c periodic 12 ms deadline 12 ms wcet 6 ms priority 3 lo;\n\
     }\n\
     schedulable CtlFp on Ctl under fixed_priority;\n\
     schedulable CtlEdf on Ctl under edf;\n";

fn rational(numerator: i64, denominator: i64) -> serde_json::Value {
    serde_json::json!({"type": "rational", "numerator": numerator.to_string(),
        "denominator": denominator.to_string()})
}

/// FR-621-AC-13, FSpec FR-419-AC-13: a refuted EN-7 item carries its miss as
/// the FSpec FR-419 `schedulability-miss` counterexample, with no replay.
#[test]
#[trace("TC-2960", "FR-621-AC-13")]
#[trace("FSpec-TC-2006", "FSpec-FR-419-AC-13")]
#[ignore = "QSL-2131: a refuted EN-7 item fails the document with MissingCounterexample today"]
fn a_refuted_schedulability_item_carries_its_miss_as_its_counterexample() {
    use serde_json::json;
    let ws = crate::analyze_cli_support::Workspace::new("schedulability-miss");
    let request = crate::analyze_cli_support::request_over(
        &ws,
        CTL_MISSES,
        &crate::counter::document(false),
        &[],
        json!({"items": [{"claim": "CtlFp"}, {"claim": "CtlEdf"}]}),
    );
    let run = crate::analyze_cli_support::analyze(&request);
    assert_eq!(run.exit, 10, "{}", run.document());
    let fp = run.item(0);
    assert_eq!(crate::analyze_cli_support::label(&fp), "refuted", "{fp}");
    assert!(fp.get("replay").is_none(), "{fp}");
    let miss = &fp["counterexample"]["counterexample"];
    assert_eq!(miss["type"], "schedulability-miss", "{fp}");
    assert_eq!(miss["analysis"], "fixed-priority", "{fp}");
    assert_eq!(miss["unit"], "ms", "{fp}");
    assert_eq!(
        miss["miss"],
        json!({"kind": "response", "task": "c", "job": "0",
            "iterates": [rational(6, 1), rational(10, 1), rational(13, 1)],
            "response": rational(13, 1), "bound": rational(12, 1)}),
        "{fp}"
    );
    let edf = run.item(1);
    assert_eq!(
        edf["counterexample"]["counterexample"]["miss"],
        json!({"kind": "utilization", "utilization": rational(13, 12)}),
        "{edf}"
    );
    assert_eq!(run.document()["last_stage"], "S7", "{}", run.document());
}
