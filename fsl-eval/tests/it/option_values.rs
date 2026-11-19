// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2169: an expected bounded integer option types and evaluates `Some(3)`.

use filament_eval::FamilyOutcome;
use filament_exact::{Integer, IntegerInterval, Meter, Outcome, ScalarLimits, Value, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"filament.value.complete/v1\";\n\
    function f using v(): Option<Int[0, 9]> pure { Some(3) }\n";

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

fn package() -> CheckedPackage {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "option-values"),
        "option-values.native",
        SOURCE.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let checked = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the declarations assemble")
    .check(CheckMode::Linked, CheckingLimits::default())
    .expect("Some(3) is checked against the expected option payload");
    CheckedPackage::link(checked)
}

#[trace("TC-1823", "FR-680-AC-4", "FSpec-FR-143-AC-21")]
#[test]
fn some_literal_evaluates_to_a_present_bounded_option() {
    let package = package();
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("a function name"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call is admitted");
    let FamilyOutcome::Evaluated(Outcome::Completed(Value::Option(option))) = evaluation.outcome
    else {
        panic!(
            "Some(3) evaluates to a completed option: {:?}",
            evaluation.outcome
        );
    };
    let expected_payload =
        ValueType::Int(IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).unwrap());
    assert_eq!(option.payload_type(), &expected_payload);
    assert!(matches!(
        option.payload(),
        Some(Value::Integer(value)) if value == &Integer::from(3_i64)
    ));
}
