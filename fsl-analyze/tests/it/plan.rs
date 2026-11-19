// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-753 coordinator preflight and dispatch through the existing engines.

use std::cell::RefCell;
use std::collections::BTreeMap;

use filament_exact::Cancel;
use fsl_analyze::analyze::document::Negotiate;
use fsl_analyze::analyze::{AnalyzeLimits, ClaimItem, Diagnostic, Engine, Engines};
use fsl_analyze::plan::{execute_plan, PlanInputs, PlanRefusal, PlanResult, PlanStatus};
use fsl_foundation::diagnostic::StageFailure;
use fsl_package::verification_plan::{VerificationPlan, VerificationPlanRefusal};
use fsl_route::routing::Disposition;
use fsl_route::{CandidateOutcome, NegotiationOutcome};
use ix_trace_rs::trace;
use serde_json::json;

use crate::clause_item::world;
use crate::model_world::World;

#[derive(Default)]
struct Recording {
    calls: RefCell<Vec<usize>>,
    items: RefCell<Vec<ClaimItem>>,
    refuse: bool,
}

impl Negotiate for Recording {
    fn negotiate(
        &self,
        item: &ClaimItem,
        candidates: &CandidateOutcome,
    ) -> (Disposition, Option<Diagnostic>) {
        self.calls.borrow_mut().push(item.request_index.get());
        self.items.borrow_mut().push(item.clone());
        if self.refuse {
            return (
                Disposition::Unsupported,
                Some(Diagnostic::new(
                    NegotiationOutcome::UnsupportedRequestedCapability.code(),
                )),
            );
        }
        match candidates {
            CandidateOutcome::Candidates(set) if set.candidates().len() == 1 => {
                (Disposition::Supported(set.candidates()[0].clone()), None)
            }
            _ => (
                Disposition::Unsupported,
                Some(Diagnostic::new(
                    NegotiationOutcome::UnsupportedRequestedCapability.code(),
                )),
            ),
        }
    }
}

fn plan(world: &World, steps: &[(&str, &str)], edges: &[(&str, &str)]) -> VerificationPlan {
    let clause = &world.package.graph().temporal_clauses()[0];
    let nodes: Vec<_> = steps
        .iter()
        .map(|(name, method)| {
            json!({
                "name": name,
                "claim": {
                    "node": fsl_semantics::check::wire_id::structural_id(&clause.identity()),
                    "occurrence": {"role": "claim", "ordinal": clause.claim().ordinal()},
                },
                "method": {"name": method, "kind": "temporal-satisfaction", "extent": "bounded",
                    "evidence_kind": ["proof", "refutation"]},
                "domain": {"package": "test/counter", "node": "ix://test/counter/Counter"},
                "bound": "5",
            })
        })
        .collect();
    serde_json::from_value(json!({
        "format": "filament.verification-plan/v1", "package": world.package.graph().package_name(),
        "plan": {"name": "Release", "qualified_name": ["Release"],
            "source_location": {"source": {"authority": "test", "identity": "plan"}, "start": 0, "end": 1}},
        "profile": "p", "nodes": nodes,
        "edges": edges.iter().map(|(from, to)| json!({"from": from, "to": to, "type": "requires"})).collect::<Vec<_>>(),
    })).expect("plan document")
}

fn inputs<'a>(world: &'a World, plan: &VerificationPlan) -> PlanInputs<'a> {
    PlanInputs {
        bindings: plan
            .nodes
            .iter()
            .map(|step| (step.name.clone(), world.binding(&["c"])))
            .collect(),
        provision: world.provision.clone(),
        seed: Some(7),
        max_trace_length: None,
        max_value_bits: None,
    }
}

#[trace("FR-753-AC-4", "FR-753-AC-5", "TC-2103", "TC-3028")]
#[test]
fn missing_trace_names_first_consumer_before_any_negotiation() {
    let world = world();
    let plan = plan(
        &world,
        &[
            ("safe", "model_check"),
            ("tested", "model_based_test"),
            ("other", "model_based_test"),
        ],
        &[],
    );
    let negotiate = Recording::default();
    let failure = execute_plan(
        &world.package,
        &plan,
        &inputs(&world, &plan),
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("missing trace bound");
    assert!(
        matches!(failure, StageFailure::Refused(PlanRefusal::MissingMember {
        member: "max_trace_length", step,
    }) if step == "tested")
    );
    assert!(negotiate.calls.borrow().is_empty());
}

#[trace("FR-753-AC-5", "TC-3028")]
#[test]
fn package_identity_precedes_missing_inputs_and_names_both_identities() {
    let world = world();
    let mut plan = plan(&world, &[("tested", "model_based_test")], &[]);
    let expected = plan.package.clone();
    plan.package = "different-package".into();
    let negotiate = Recording::default();
    for max_trace_length in [Some(8), None] {
        let mut inputs = inputs(&world, &plan);
        inputs.bindings = BTreeMap::new();
        inputs.max_trace_length = max_trace_length;
        let failure = execute_plan(
            &world.package,
            &plan,
            &inputs,
            &Engines::default(),
            &negotiate,
            &AnalyzeLimits::unbounded(),
            &Cancel::new(),
        )
        .expect_err("identity mismatch");
        assert!(matches!(failure, StageFailure::Refused(PlanRefusal::Reader(
            VerificationPlanRefusal::PackageIdentityMismatch { expected: wanted, actual }
        )) if wanted == expected && actual == "different-package"));
    }
    assert!(negotiate.calls.borrow().is_empty());
}

#[trace("FR-753-AC-4", "FR-753-AC-5", "TC-2103", "TC-3028")]
#[test]
fn unsupported_blocks_only_descendants_and_does_not_require_trace_bound() {
    let world = world();
    let plan = plan(
        &world,
        &[
            ("safe", "model_check"),
            ("independent", "model_check"),
            ("timely", "model_check"),
        ],
        &[("timely", "safe")],
    );
    let negotiate = Recording::default();
    let result = execute_plan(
        &world.package,
        &plan,
        &inputs(&world, &plan),
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("model-only plan")
    .into_value();
    assert_eq!(*negotiate.calls.borrow(), [0, 1]);
    assert_eq!(
        result
            .dispositions
            .iter()
            .map(|(name, step)| (name.as_str(), step.status))
            .collect::<Vec<_>>(),
        [
            ("safe", PlanStatus::Unsupported),
            ("independent", PlanStatus::Unsupported),
            ("timely", PlanStatus::Blocked)
        ]
    );
    let Some(PlanResult::Analysis(safe)) = result.dispositions[0].1.result.as_ref() else {
        panic!("terminal record");
    };
    let detail = safe
        .diagnostic_detail()
        .expect("unsupported kind and extent");
    assert_eq!(detail["kind"], "temporal-satisfaction");
    assert_eq!(detail["extent"], "bounded");
    assert_eq!(
        safe.diagnostic(),
        Some(NegotiationOutcome::UnsupportedRequestedCapability.code())
    );
    assert_eq!(result.dispositions[2].1.blocked_by.as_deref(), Some("safe"));
    assert!(result.dispositions[2].1.result.is_none());
}

#[trace("FR-753-AC-4", "TC-2103")]
#[test]
fn a_real_refutation_blocks_a_forward_dependent_but_runs_independent_work() {
    let world = world();
    let plan = plan(
        &world,
        &[
            ("timely", "model_check"),
            ("safe", "model_check"),
            ("independent", "model_check"),
        ],
        &[("timely", "safe")],
    );
    let mut engines = Engines::default();
    engines
        .with(Engine::model_check().expect("EN-1 manifest"))
        .expect("one engine");
    let negotiate = Recording::default();
    let result = execute_plan(
        &world.package,
        &plan,
        &inputs(&world, &plan),
        &engines,
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("plan executes")
    .into_value();
    assert_eq!(*negotiate.calls.borrow(), [1, 2]);
    assert_eq!(result.dispositions[0].1.status, PlanStatus::Blocked);
    assert_eq!(result.dispositions[1].1.status, PlanStatus::Refuted);
    assert_eq!(result.dispositions[2].1.status, PlanStatus::Refuted);
    assert_eq!(
        result.dispositions[1].1.evidence.as_deref(),
        Some("refutation")
    );
}

#[trace("FR-753-AC-4", "TC-2103", "FR-1038-AC-4")]
#[test]
fn missing_value_size_follows_trace_bound_and_names_first_unbounded_consumer() {
    let mut domain = crate::model_world::counter();
    domain.document = crate::counter::document(true);
    let world = World::new(
        domain,
        crate::counter::CONTRACT,
        crate::model_world::INFINITE,
        &[crate::model_world::Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 1)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    let plan = plan(&world, &[("tested", "model_based_test")], &[]);
    let negotiate = Recording::default();
    let mut input = inputs(&world, &plan);
    let failure = execute_plan(
        &world.package,
        &plan,
        &input,
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("trace first");
    assert!(matches!(
        failure,
        StageFailure::Refused(PlanRefusal::MissingMember {
            member: "max_trace_length",
            ..
        })
    ));
    input.max_trace_length = Some(8);
    let failure = execute_plan(
        &world.package,
        &plan,
        &input,
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("value size next");
    assert!(
        matches!(failure, StageFailure::Refused(PlanRefusal::MissingMember {
        member: "max_value_bits", step,
    }) if step == "tested")
    );
    assert!(negotiate.calls.borrow().is_empty());
}

#[trace("FR-753-AC-5", "TC-3028")]
#[test]
fn reader_refusal_precedes_identity_and_missing_runtime_inputs() {
    let world = world();
    let mut plan = plan(&world, &[("tested", "model_based_test")], &[]);
    plan.format = "unknown-format".into();
    plan.package = "other".into();
    let negotiate = Recording::default();
    let failure = execute_plan(
        &world.package,
        &plan,
        &inputs(&world, &plan),
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("format refusal");
    assert!(matches!(
        failure,
        StageFailure::Refused(PlanRefusal::Reader(
            VerificationPlanRefusal::InvalidDocument { .. }
        ))
    ));
    assert!(negotiate.calls.borrow().is_empty());
}

#[trace("FR-753-AC-4", "FR-753-AC-5", "TC-2103", "TC-3028")]
#[test]
fn initial_admission_precedes_missing_generation_bounds() {
    let world = world();
    let plan = plan(&world, &[("tested", "model_based_test")], &[]);
    let mut inputs = inputs(&world, &plan);
    inputs.bindings.get_mut("tested").expect("binding").initial =
        vec![fsl_semantics::model::observation::DocumentRef {
            authority: "test".into(),
            identity: "absent-snapshot".into(),
        }];
    let negotiate = Recording::default();
    let failure = execute_plan(
        &world.package,
        &plan,
        &inputs,
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("initial refusal first");
    assert!(matches!(
        failure,
        StageFailure::Refused(PlanRefusal::Initial(_))
    ));
    assert!(negotiate.calls.borrow().is_empty());
}

#[trace("FR-753-AC-4", "TC-2103")]
#[test]
fn finite_model_generation_consumes_no_value_size_and_reaches_only_the_owned_integration_seam() {
    let world = world();
    let plan = plan(&world, &[("tested", "model_based_test")], &[]);
    let mut inputs = inputs(&world, &plan);
    inputs.max_trace_length = Some(8);
    let negotiate = Recording::default();
    let failure = execute_plan(
        &world.package,
        &plan,
        &inputs,
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("FR-752 entry pending");
    let StageFailure::Fault(fault) = failure else {
        panic!("only the owned integration seam remains");
    };
    assert_eq!(
        fault.invariant(),
        "model-based-test-entry-unavailable-QSL-2237"
    );
    assert!(negotiate.calls.borrow().is_empty());
}

#[trace("FR-753-AC-5", "TC-3028")]
#[test]
fn simulation_runs_on_its_language_route_without_negotiation() {
    let world = world();
    let plan = plan(&world, &[("runs", "simulate")], &[]);
    let negotiate = Recording::default();
    let result = execute_plan(
        &world.package,
        &plan,
        &inputs(&world, &plan),
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("simulation route")
    .into_value();
    assert!(negotiate.calls.borrow().is_empty());
    assert_eq!(result.dispositions.len(), 1);
    assert_eq!(
        result.dispositions[0].1.evidence.as_deref(),
        Some("simulation-runs")
    );
    let Some(PlanResult::Simulation(runs)) = &result.dispositions[0].1.result else {
        panic!("simulation runs");
    };
    assert_eq!(runs["kind"], "simulation");
    assert!(runs["coverage"].is_object(), "{runs}");
}

#[trace("FR-753-AC-4", "TC-2103")]
#[test]
fn blocked_by_names_first_prerequisite_in_source_order_not_edge_order() {
    let world = world();
    let plan = plan(
        &world,
        &[
            ("first", "model_check"),
            ("second", "model_check"),
            ("dependent", "model_check"),
        ],
        &[("dependent", "second"), ("dependent", "first")],
    );
    let negotiate = Recording::default();
    let result = execute_plan(
        &world.package,
        &plan,
        &inputs(&world, &plan),
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("plan")
    .into_value();
    assert_eq!(
        result.dispositions[2].1.blocked_by.as_deref(),
        Some("first")
    );
    assert_eq!(*negotiate.calls.borrow(), [0, 1]);
}

#[trace("FR-753-AC-2", "TC-2103")]
#[test]
fn direct_call_cycle_is_rejected_before_negotiation() {
    let world = world();
    let plan = plan(
        &world,
        &[("a", "model_check"), ("b", "model_check")],
        &[("a", "b"), ("b", "a")],
    );
    let negotiate = Recording::default();
    let failure = execute_plan(
        &world.package,
        &plan,
        &inputs(&world, &plan),
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("requires cycle");
    assert!(matches!(failure, StageFailure::Refused(PlanRefusal::Reader(
        VerificationPlanRefusal::InvalidDocument { reason }
    )) if reason == "requires cycle"));
    assert!(negotiate.calls.borrow().is_empty());
}

#[trace("FR-753-AC-5", "TC-3028")]
#[test]
fn contract_method_negotiates_an_operation_contract_item() {
    let world = world();
    let mut plan = plan(&world, &[("contract", "contract_check")], &[]);
    let clause = world
        .package
        .graph()
        .state_clauses()
        .iter()
        .find(|clause| clause.name() == "Inc")
        .expect("postcondition");
    plan.nodes[0].claim.node = serde_json::from_value(
        serde_json::to_value(fsl_semantics::check::wire_id::structural_id(
            &clause.identity(),
        ))
        .expect("identity"),
    )
    .expect("plan identity");
    plan.nodes[0].claim.occurrence.ordinal = clause.claim().ordinal();
    plan.nodes[0].method.kind = "operation-contract".into();
    let negotiate = Recording::default();
    let result = execute_plan(
        &world.package,
        &plan,
        &inputs(&world, &plan),
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("contract route")
    .into_value();
    assert_eq!(*negotiate.calls.borrow(), [0]);
    let items = negotiate.items.borrow();
    assert_eq!(
        items[0].kind,
        fsl_semantics::check::Capability::OperationContract
    );
    assert_eq!(
        items[0].options.max_cases.map(std::num::NonZeroU64::get),
        Some(5)
    );
    assert_eq!(result.dispositions[0].1.status, PlanStatus::Unsupported);
    let Some(PlanResult::Analysis(record)) = &result.dispositions[0].1.result else {
        panic!("contract record");
    };
    assert_eq!(
        record.diagnostic(),
        Some(NegotiationOutcome::UnsupportedRequestedCapability.code())
    );
}

#[trace("FR-753-AC-5", "TC-3028")]
#[test]
fn probability_methods_keep_their_evidence_and_only_statistical_consumes_seed() {
    let model = crate::prob_world::service(
        "probabilistic P95 using ev on M over (s: M::Server) under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 \
         { quantile 0.95 of (accumulate duration from holds(s.phase = 1) \
         until holds(s.phase = 2) within 10) <= 2 }",
    );
    let world = crate::prob_world::World::new(&model);
    let template = crate::clause_item::world();
    let mut plan = plan(
        &template,
        &[
            ("exact", "probabilistic_exact"),
            ("statistical", "probabilistic_statistical"),
        ],
        &[],
    );
    plan.package = world.package.graph().package_name().into();
    let claim = world.package.graph().probabilistic_model().claims()[0]
        .occurrence()
        .expect("claim occurrence");
    for (node, evidence) in plan.nodes.iter_mut().zip(["exact", "statistical"]) {
        node.claim.node = serde_json::from_value(
            serde_json::to_value(fsl_semantics::check::wire_id::structural_id(claim.node()))
                .expect("identity"),
        )
        .expect("plan identity");
        node.claim.occurrence.ordinal = claim.origin().ordinal();
        node.method.kind = "probabilistic-satisfaction".into();
        node.method.evidence_kind = vec![evidence.into()];
    }
    let inputs = PlanInputs {
        bindings: plan
            .nodes
            .iter()
            .map(|node| (node.name.clone(), world.binding()))
            .collect(),
        provision: world.provision.clone(),
        seed: Some(7),
        max_trace_length: None,
        max_value_bits: None,
    };
    let negotiate = Recording {
        refuse: true,
        ..Recording::default()
    };
    let result = execute_plan(
        &world.package,
        &plan,
        &inputs,
        &Engines::fsl().expect("native manifests"),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("probability routes")
    .into_value();
    assert_eq!(result.dispositions.len(), 2);
    let items = negotiate.items.borrow();
    assert_eq!(items.len(), 2);
    for (item, evidence, seed, backend) in [
        (&items[0], fsl_route::EvidenceKind::Exact, None, "en-5"),
        (
            &items[1],
            fsl_route::EvidenceKind::Statistical,
            Some(7),
            "en-4",
        ),
    ] {
        assert_eq!(
            item.kind,
            fsl_semantics::check::Capability::ProbabilisticSatisfaction
        );
        let Some(fsl_analyze::analyze::ItemForm::Probabilistic(form)) = &item.form else {
            panic!("probabilistic item");
        };
        assert_eq!(form.evidence, evidence);
        assert_eq!(form.seed, seed);
        assert_eq!(
            item.candidates
                .iter()
                .map(|candidate| candidate.id().as_str())
                .collect::<Vec<_>>(),
            [backend]
        );
    }
}
