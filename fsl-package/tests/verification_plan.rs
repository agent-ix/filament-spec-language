// SPDX-License-Identifier: AGPL-3.0-or-later
//! S4 verification-plan emission and strict reading (FR-753, TC-2103).

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{
    CheckedPackage, VerificationPlanRefusal, emit_verification_plans, read_verification_plan,
};
use fsl_semantics::check::PackageDeclarations;
use ix_trace_rs::trace;

const SOURCE_WITH_PLAN: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile p = \"filament.temporal.infinite-trace/v1\";\n\
    model Config = \"acme/orders\";\n\
    temporal Reaches using p over (c: Config::Widget) clock \"model-steps\" on origin { eventually holds(c.code > 0) }\n\
    verify Release using p {\n\
      check safe claim Reaches method model_check domain Config::Widget bound 20;\n\
      check tested claim Reaches method model_based_test domain Config::Widget bound 500;\n\
      check timely claim Reaches method model_check domain Config::Widget depends [safe];\n\
    }\n";

fn checked(source_text: &str) -> CheckedPackage {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:qsl-2509"),
        "unit.native",
        source_text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the source");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the source");
    let model_packages = fsl_semantics::model::intake::package_input([include_bytes!(
        "fixtures/verification-plan-model.semantic-ir.json"
    )
    .as_slice()])
    .expect("the selected model package is admitted");
    let models = fsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &model_packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the unit's model selection resolves");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "test/package".to_owned(),
        unit,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    CheckedPackage::link(
        declarations
            .check(
                CheckMode::Linked,
                CheckingLimits::default(),
                &fsl_semantics::check::test_support::ZeroRewardEvaluator,
            )
            .unwrap_or_else(|refusal| panic!("S3 checks the source: {refusal:?}")),
    )
}

#[trace("TC-2103", "FR-753-AC-3")]
#[test]
fn emission_keeps_package_identity_and_writes_the_plan_graph() {
    let without = checked(
        SOURCE_WITH_PLAN
            .split("    verify Release")
            .next()
            .expect("the source contains the plan"),
    );
    let with = checked(SOURCE_WITH_PLAN);
    assert_eq!(
        with.graph().package_name(),
        without.graph().package_name(),
        "a verify declaration does not change checked package identity"
    );

    let plans = emit_verification_plans(&with);
    assert_eq!(plans.len(), 1);
    let plan = &plans[0];
    assert_eq!(plan.format, "filament.verification-plan/v1");
    assert_eq!(plan.package, "test/package");
    assert_eq!(plan.plan.name, "Release");
    assert_eq!(plan.plan.qualified_name, ["Release"]);
    assert_eq!(plan.plan.source_location.source.authority, "agent-ix");
    assert_eq!(plan.plan.source_location.source.identity, "test:qsl-2509");
    assert!(plan.plan.source_location.start < plan.plan.source_location.end);
    assert!(
        SOURCE_WITH_PLAN
            [plan.plan.source_location.start as usize..plan.plan.source_location.end as usize]
            .starts_with("verify Release")
    );
    assert_eq!(plan.nodes.len(), 3);
    let safe = &plan.nodes[0];
    assert_eq!(safe.name, "safe");
    assert_eq!(
        safe.claim.occurrence.role,
        fsl_package::CheckedOccurrenceRole::Claim
    );
    assert_eq!(safe.claim.occurrence.ordinal, 0);
    assert_eq!(safe.claim.node.package, "test/package");
    let checked_claim = with
        .graph()
        .temporal_clauses()
        .iter()
        .find(|clause| clause.clause().name() == "Reaches")
        .expect("the package retains the checked claim");
    assert_eq!(
        serde_json::to_value(&safe.claim.node).expect("plan node identity serializes"),
        serde_json::to_value(fsl_semantics::check::wire_id::structural_id(
            &checked_claim.identity()
        ))
        .expect("checked node identity serializes")
    );
    assert_eq!(
        safe.claim.occurrence.ordinal,
        checked_claim.claim().ordinal()
    );
    assert_eq!(safe.method.name, "model_check");
    assert_eq!(safe.method.kind, "temporal-satisfaction");
    assert_eq!(safe.method.extent, "bounded");
    assert_eq!(safe.method.evidence_kind, ["proof", "refutation"]);
    assert_eq!(safe.domain.package, "acme/orders");
    assert!(!safe.domain.node.is_empty());
    assert_eq!(safe.bound.as_deref(), Some("20"));
    let tested = &plan.nodes[1];
    assert_eq!(tested.method.name, "model_based_test");
    assert_eq!(tested.method.kind, "temporal-satisfaction");
    assert_eq!(tested.method.extent, "bounded");
    assert_eq!(tested.method.evidence_kind, ["tested", "refutation"]);
    assert_eq!(plan.nodes[2].method.extent, "unbounded");
    assert_eq!(plan.edges.len(), 1);
    assert_eq!(plan.edges[0].from, "timely");
    assert_eq!(plan.edges[0].to, "safe");
    assert_eq!(plan.edges[0].edge_type, "requires");

    let bytes = serde_json::to_vec(plan).expect("the emitted plan serializes");
    let round_trip = read_verification_plan(&with, &bytes)
        .expect("the strict reader accepts the plan for its checked package");
    assert_eq!(
        round_trip.plan.source_location, plan.plan.source_location,
        "the plan source location survives the reader"
    );
    assert_eq!(
        round_trip.nodes[0].claim, plan.nodes[0].claim,
        "the checked clause identity and claim occurrence survive the reader"
    );
    assert_eq!(
        round_trip.nodes[0].method, plan.nodes[0].method,
        "the method kind, extent and evidence semantics survive the reader"
    );
    assert_eq!(
        round_trip.nodes[0].domain, plan.nodes[0].domain,
        "the resolved domain declaration key survives the reader"
    );
}

#[trace("TC-3028", "FR-753-AC-5")]
#[test]
fn strict_reader_refuses_a_plan_bound_to_another_package() {
    let package = checked(SOURCE_WITH_PLAN);
    let mut document = emit_verification_plans(&package).remove(0);
    document.package = "other/package".into();
    let bytes = serde_json::to_vec(&document).expect("the emitted plan serializes");

    assert_eq!(
        read_verification_plan(&package, &bytes),
        Err(VerificationPlanRefusal::PackageIdentityMismatch {
            expected: "test/package".into(),
            actual: "other/package".into(),
        }),
        "reader rejects package mismatch before the admitted plan reaches execution"
    );
}
