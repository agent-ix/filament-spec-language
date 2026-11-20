// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-753 coordinator preflight and dispatch through the existing engines.

use std::cell::RefCell;
use std::collections::BTreeMap;

use filament_exact::Cancel;
use filament_semantic_value::catalog::{CatalogCode, CatalogCoded};
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

fn value_world(parameter: &str) -> World {
    World::with_kernel_functions(
        crate::model_world::counter(),
        crate::counter::CONTRACT,
        &format!("function Subject using v(x: {parameter}): Boolean pure {{ x = x }}\n"),
        crate::model_world::INFINITE,
        &[crate::model_world::Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 1)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    )
}

fn select_value_claim(
    world: &World,
    node: &mut fsl_package::verification_plan::VerificationPlanNode,
) {
    let graph = world.package.graph();
    let callable = graph.callable("Subject").expect("value function");
    let body = graph
        .semantic_graph()
        .node(callable.identity)
        .and_then(|node| node.function_body())
        .expect("Boolean body");
    let (key, origin, _) = graph
        .occurrences()
        .find(|(key, origin, _)| *key == body && origin.role().as_str() == "declaration")
        .expect("value declaration occurrence");
    node.claim.node = serde_json::from_value(
        serde_json::to_value(fsl_semantics::check::wire_id::structural_id(&key))
            .expect("structural identity"),
    )
    .expect("plan identity");
    node.claim.occurrence.role = filament_contract_model::CheckedOccurrenceRole::Declaration;
    node.claim.occurrence.ordinal = origin.ordinal();
    node.method.kind = "value-validity".into();
}

#[trace("FR-753-AC-4", "TC-2103")]
#[test]
fn value_generation_requires_only_its_consumed_bound_and_reports_catalog_fields() {
    let world = value_world("Integer");
    let mut plan = plan(
        &world,
        &[
            ("safe", "model_check"),
            ("value", "model_based_test"),
            ("later", "model_based_test"),
        ],
        &[],
    );
    select_value_claim(&world, &mut plan.nodes[1]);
    select_value_claim(&world, &mut plan.nodes[2]);
    let negotiate = Recording::default();
    let mut inputs = inputs(&world, &plan);
    let failure = execute_plan(
        &world.package,
        &plan,
        &inputs,
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect_err("unbounded value requires size, but no trace bound");
    let StageFailure::Refused(refusal @ PlanRefusal::MissingMember { .. }) = failure else {
        panic!("missing value size");
    };
    assert_eq!(
        refusal.catalog_code(),
        CatalogCode::new("invalid_runtime_input", "missing-member")
    );
    assert_eq!(
        refusal.catalog_fields(),
        Some(BTreeMap::from([
            ("member", "max_value_bits".into()),
            ("step", "value".into()),
        ]))
    );
    assert!(negotiate.calls.borrow().is_empty());
    inputs.max_value_bits = Some(8);
    let outcome = execute_plan(
        &world.package,
        &plan,
        &inputs,
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("the plan runs once its consumed size is supplied")
    .into_value();
    assert_eq!(*negotiate.calls.borrow(), [0]);
    for (name, disposition) in &outcome.dispositions[1..] {
        assert_eq!(disposition.status, PlanStatus::Tested, "{name}");
        assert_eq!(disposition.evidence.as_deref(), Some("tested"), "{name}");
        let Some(PlanResult::ModelBasedTest(record)) = &disposition.result else {
            panic!("{name} carries its FR-190 record: {disposition:?}");
        };
        assert_eq!(record.max_cases, 5, "{name}: the step bound is max_cases");
        assert_eq!(record.seed, 7, "{name}");
        assert_eq!(record.max_value_bits, Some(8), "{name}");
    }
}

#[trace("FR-753-AC-4", "TC-2103")]
#[test]
fn declared_finite_value_generation_consumes_neither_trace_nor_size() {
    for parameter in ["Int[0, 3]", "Text[0, 8; unicode-scalars]"] {
        let world = value_world(parameter);
        let mut plan = plan(&world, &[("value", "model_based_test")], &[]);
        select_value_claim(&world, &mut plan.nodes[0]);
        let negotiate = Recording::default();
        let outcome = execute_plan(
            &world.package,
            &plan,
            &inputs(&world, &plan),
            &Engines::default(),
            &negotiate,
            &AnalyzeLimits::unbounded(),
            &Cancel::new(),
        )
        .expect("a finite value runs on the FR-752 entry without generation bounds")
        .into_value();
        let (_, disposition) = &outcome.dispositions[0];
        assert_eq!(disposition.status, PlanStatus::Tested, "{parameter}");
        let Some(PlanResult::ModelBasedTest(record)) = &disposition.result else {
            panic!("{parameter}: the FR-190 record: {disposition:?}");
        };
        assert_eq!(record.max_value_bits, None, "{parameter}");
        assert!(negotiate.calls.borrow().is_empty());
    }
}

#[trace("FR-753-AC-5", "TC-3028")]
#[test]
fn value_check_negotiates_value_validity_without_generation_inputs() {
    let world = value_world("Integer");
    let mut plan = plan(&world, &[("value", "value_check")], &[]);
    select_value_claim(&world, &mut plan.nodes[0]);
    let negotiate = Recording::default();
    let outcome = execute_plan(
        &world.package,
        &plan,
        &inputs(&world, &plan),
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("negotiated value route")
    .into_value();
    assert_eq!(*negotiate.calls.borrow(), [0]);
    let items = negotiate.items.borrow();
    assert_eq!(
        items[0].kind,
        fsl_semantics::check::Capability::ValueValidity
    );
    assert_eq!(outcome.dispositions[0].1.status, PlanStatus::Unsupported);
    assert_eq!(outcome.dispositions[0].1.evidence, None);
    let Some(PlanResult::Analysis(record)) = &outcome.dispositions[0].1.result else {
        panic!("value terminal record");
    };
    assert_eq!(
        record.diagnostic_detail().expect("unsupported kind")["kind"],
        "value-validity"
    );
}

#[trace("FR-753-AC-4", "FR-753-AC-5", "TC-2103", "TC-3028")]
#[test]
fn successful_simulation_satisfies_a_prerequisite_without_negotiation() {
    let world = world();
    let plan = plan(
        &world,
        &[("runs", "simulate"), ("dependent", "model_check")],
        &[("dependent", "runs")],
    );
    let negotiate = Recording::default();
    let outcome = execute_plan(
        &world.package,
        &plan,
        &inputs(&world, &plan),
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("simulation prerequisite")
    .into_value();
    assert_eq!(*negotiate.calls.borrow(), [1]);
    assert_eq!(outcome.dispositions[0].1.status, PlanStatus::Tested);
    assert_eq!(
        outcome.dispositions[0].1.evidence.as_deref(),
        Some("simulation-runs")
    );
    assert_eq!(outcome.dispositions[1].1.status, PlanStatus::Unsupported);
    assert_eq!(outcome.dispositions[1].1.blocked_by, None);
}

#[trace("FR-753-AC-4", "TC-2103")]
#[test]
fn absent_initial_still_requires_the_first_state_trace_consumer() {
    let world = value_world("Integer");
    let mut plan = plan(
        &world,
        &[("value", "model_based_test"), ("state", "model_based_test")],
        &[],
    );
    select_value_claim(&world, &mut plan.nodes[0]);
    let mut inputs = inputs(&world, &plan);
    for binding in inputs.bindings.values_mut() {
        binding.initial.clear();
    }
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
    .expect_err("state trace bound precedes earlier value size consumer");
    assert!(
        matches!(failure, StageFailure::Refused(PlanRefusal::MissingMember {
        member: "max_trace_length", step,
    }) if step == "state")
    );
    assert!(negotiate.calls.borrow().is_empty());
}

#[trace("FR-753-AC-4", "TC-2103")]
#[test]
fn proved_prerequisite_runs_its_dependent_and_transitive_blocking_skips_descendants() {
    let world = World::new(
        crate::model_world::counter(),
        crate::counter::CONTRACT,
        crate::model_world::INFINITE,
        &[crate::model_world::Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 3)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    );
    let plan = plan(
        &world,
        &[
            ("last", "model_check"),
            ("middle", "model_check"),
            ("first", "model_check"),
            ("independent", "model_check"),
        ],
        &[("last", "middle"), ("middle", "first")],
    );
    let mut engines = Engines::default();
    engines
        .with(Engine::model_check().expect("EN-1 manifest"))
        .expect("one engine");
    for (engines, calls, statuses) in [
        (&engines, vec![2, 1, 0, 3], vec![PlanStatus::Proved; 4]),
        (
            &Engines::default(),
            vec![2, 3],
            vec![
                PlanStatus::Blocked,
                PlanStatus::Blocked,
                PlanStatus::Unsupported,
                PlanStatus::Unsupported,
            ],
        ),
    ] {
        let negotiate = Recording::default();
        let outcome = execute_plan(
            &world.package,
            &plan,
            &inputs(&world, &plan),
            engines,
            &negotiate,
            &AnalyzeLimits::unbounded(),
            &Cancel::new(),
        )
        .expect("dependency graph")
        .into_value();
        assert_eq!(*negotiate.calls.borrow(), calls);
        assert_eq!(
            outcome
                .dispositions
                .iter()
                .map(|(_, step)| step.status)
                .collect::<Vec<_>>(),
            statuses
        );
        if outcome.dispositions[0].1.status == PlanStatus::Blocked {
            assert_eq!(
                outcome.dispositions[0].1.blocked_by.as_deref(),
                Some("middle")
            );
            assert_eq!(
                outcome.dispositions[1].1.blocked_by.as_deref(),
                Some("first")
            );
        }
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
fn finite_model_generation_consumes_no_value_size_and_runs_on_the_model_based_test_entry() {
    let world = world();
    let plan = plan(&world, &[("tested", "model_based_test")], &[]);
    let mut inputs = inputs(&world, &plan);
    inputs.max_trace_length = Some(8);
    let negotiate = Recording::default();
    let outcome = execute_plan(
        &world.package,
        &plan,
        &inputs,
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("a finite model needs no value size")
    .into_value();
    let (name, disposition) = &outcome.dispositions[0];
    assert_eq!(name, "tested");
    let Some(PlanResult::ModelBasedTest(record)) = &disposition.result else {
        panic!("the FR-752 entry's FR-190 record: {disposition:?}");
    };
    assert_eq!(record.max_cases, 5);
    assert_eq!(record.seed, 7);
    assert_eq!(record.max_trace_length, 8);
    assert_eq!(record.max_value_bits, None);
    assert!(matches!(
        disposition.status,
        PlanStatus::Tested | PlanStatus::Refuted
    ));
    assert!(negotiate.calls.borrow().is_empty(), "never negotiated");
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

/// A counter world whose clause holds in every reachable state: `inc` stops
/// at 3 (`CanInc`), so `value <= 3` is never violated.
fn holding_world() -> World {
    World::new(
        crate::model_world::counter(),
        crate::counter::CONTRACT,
        crate::model_world::INFINITE,
        &[crate::model_world::Clause {
            activation: "on each (q: M::Counter)",
            formula: "always holds(c.value <= 3)",
        }],
        &[("c", json!({"value": {"integer": "0"}}))],
    )
}

/// FR-753-AC-5: over `Release` with no `temporal-satisfaction` engine,
/// `safe` settles `unsupported`, `tested` runs on the FR-752 entry with
/// `max_cases` 500, the seed and `max_trace_length` 8 and settles `tested`,
/// and `timely` is blocked by `safe`; only `safe` is negotiated.
#[trace("FR-753-AC-5", "TC-3028")]
#[trace("FR-753-AC-4", "TC-2103")]
#[test]
fn release_dispatches_tested_to_the_model_based_test_entry_and_negotiates_safe_only() {
    let world = holding_world();
    let mut plan = plan(
        &world,
        &[
            ("safe", "model_check"),
            ("tested", "model_based_test"),
            ("timely", "model_check"),
        ],
        &[("timely", "safe")],
    );
    plan.nodes[0].bound = Some("20".into());
    plan.nodes[1].bound = Some("500".into());
    plan.nodes[2].bound = None;
    let mut inputs = inputs(&world, &plan);
    inputs.max_trace_length = Some(8);
    let negotiate = Recording::default();
    let outcome = execute_plan(
        &world.package,
        &plan,
        &inputs,
        &Engines::default(),
        &negotiate,
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("Release runs")
    .into_value();
    assert_eq!(*negotiate.calls.borrow(), [0], "only `safe` is negotiated");
    let [(safe, safe_step), (tested, tested_step), (timely, timely_step)] =
        outcome.dispositions.as_slice()
    else {
        panic!("one disposition per node: {:?}", outcome.dispositions);
    };
    assert_eq!((safe.as_str(), safe_step.status), ("safe", PlanStatus::Unsupported));
    let Some(PlanResult::Analysis(record)) = &safe_step.result else {
        panic!("safe's terminal record");
    };
    assert_eq!(
        record.diagnostic(),
        Some(NegotiationOutcome::UnsupportedRequestedCapability.code())
    );
    assert_eq!(tested, "tested");
    assert_eq!(tested_step.status, PlanStatus::Tested);
    assert_eq!(tested_step.evidence.as_deref(), Some("tested"));
    let Some(PlanResult::ModelBasedTest(record)) = &tested_step.result else {
        panic!("tested's FR-190 record: {tested_step:?}");
    };
    assert_eq!(record.max_cases, 500);
    assert_eq!(record.seed, 7);
    assert_eq!(record.max_trace_length, 8);
    assert_eq!(
        (timely.as_str(), timely_step.status, timely_step.blocked_by.as_deref()),
        ("timely", PlanStatus::Blocked, Some("safe"))
    );
    assert!(timely_step.result.is_none());
}

/// FSpec FR-192: a `model_based_test` step with no `bound` asks for an
/// unbounded extent FR-190 does not have, so it runs no case.
#[trace("FR-753-AC-5", "TC-3028")]
#[test]
fn an_unbounded_model_based_test_step_runs_no_case() {
    let world = holding_world();
    let mut plan = plan(&world, &[("tested", "model_based_test")], &[]);
    plan.nodes[0].bound = None;
    let mut inputs = inputs(&world, &plan);
    inputs.max_trace_length = Some(8);
    let outcome = execute_plan(
        &world.package,
        &plan,
        &inputs,
        &Engines::default(),
        &Recording::default(),
        &AnalyzeLimits::unbounded(),
        &Cancel::new(),
    )
    .expect("the plan runs")
    .into_value();
    let (_, step) = &outcome.dispositions[0];
    assert_eq!(step.status, PlanStatus::Unsupported);
    assert!(matches!(
        step.result,
        Some(PlanResult::NotRun {
            cause: "requires-bound",
            ..
        })
    ));
}
