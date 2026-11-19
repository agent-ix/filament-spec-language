// SPDX-License-Identifier: AGPL-3.0-or-later
//! S4 verification-plan emission and strict reading (FR-753, TC-2103).

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_package::{
    emit_verification_plans, read_verification_plan, CheckedPackage, VerificationPlanRefusal,
};
use fsl_semantics::check::PackageDeclarations;
use ix_trace_rs::trace;

const SOURCE_WITH_PLAN: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile p = \"filament.temporal.infinite-trace/v1\";\n\
    type Config = Integer;\n\
    temporal Reaches using p over (c: Config) clock \"model-steps\" on origin { eventually holds(c > 0) }\n\
    verify Release using p {\n\
      check safe claim Reaches method model_check domain Config bound 20;\n\
      check tested claim Reaches method model_based_test domain Config bound 500;\n\
      check timely claim Reaches method model_check domain Config depends [safe];\n\
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
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "test/package".to_owned(),
        unit,
        Vec::new(),
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
    assert_eq!(plan.name, "Release");
    assert_eq!(plan.nodes.len(), 3);
    assert_eq!(plan.edges.len(), 1);
    assert_eq!(plan.edges[0].from, "timely");
    assert_eq!(plan.edges[0].to, "safe");
    assert_eq!(plan.edges[0].edge_type, "requires");
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
