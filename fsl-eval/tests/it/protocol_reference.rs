// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-308: the value of a protocol reference is the value of the protocol
//! clause it names, and no stage evaluates a protocol clause, so evaluating a
//! function whose body is a reference refuses, typed, with a catalog cause.

use filament_eval::{FamilyOutcome, FamilyResult};
use filament_exact::{Meter, ScalarLimits};
use filament_semantic_value::catalog::CatalogCode;
use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::checking::CheckingLimits;
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::object_environment::ObjectEnvironment;

const MODEL: &[u8] = include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");

fn unit() -> String {
    String::from(
        "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    profile p = \"filament.protocol.finite-global/v1\";\n\
    model M = \"acme/orders\";\n\
    function refers using v(): Boolean pure { protocol Fill }\n\
    protocol Fill using p over (input: M::Gadget) on origin {\n\
    role R on M::Gadget;\n\
    run sequence Main { }\n\
    finish End as (outcome: Boolean) { true };\n\
    }\n",
    )
}

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

#[test]
fn evaluating_a_protocol_reference_refuses_unsupported_feature() {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        unit().as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let packages = fsl_semantics::model::intake::package_input([MODEL]).unwrap();
    let models = fsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &packages,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("I1 admits the model");
    let package = CheckedPackage::link(
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "fixture".to_owned(),
            unit,
            models,
            Vec::new(),
        )
        .expect("the unit assembles")
        .check(CheckMode::Linked, CheckingLimits::default())
        .expect("the package checks"),
    );
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("refers").expect("refers is an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) = evaluation.outcome else {
        panic!("a family refusal, not {:?}", evaluation.outcome);
    };
    assert_eq!(
        cause.catalog_code(),
        CatalogCode::new("unknown_required_feature", "unsupported-feature")
    );
}
