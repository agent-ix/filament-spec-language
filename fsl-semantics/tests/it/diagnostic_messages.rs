// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2199: preserve the operator and operand types on arithmetic mismatch.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::{build_unit, BinaryOperator};
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::{AssemblyCause, CheckCause, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"filament.value.complete/v1\";\n";

/// Provenance: QSL-2199.
#[test]
fn arithmetic_type_mismatch_retains_operator_and_operand_types() {
    let source =
        format!("{HEADER}function add using v(a: Integer, b: String): Integer pure {{ a + b }}");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "diagnostic-messages"),
        "diagnostic-messages.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "diagnostic-messages".to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("the assembler accepts the declaration");
    let refusals = declarations
        .check(CheckMode::Linked, CheckingLimits::default())
        .expect_err("Integer + String is not admitted");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(
        refusals[0].cause.code(),
        fsl_foundation::diagnostic::Code::IllTyped
    );
    assert_eq!(refusals[0].cause.cause(), Some("type-mismatch"));
    assert_eq!(
        refusals[0].cause,
        CheckCause::OperandTypeMismatch {
            operator: BinaryOperator::Add,
            left_type: Box::new(filament_exact::ValueType::Integer),
            right_type: Box::new(filament_exact::ValueType::String),
        }
    );
}

/// Provenance: QSL-2199.
#[test]
fn assembler_duplicate_case_refusal_names_its_enum() {
    let source = format!("{HEADER}enum Shade {{ Red, Red }}");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "diagnostic-messages"),
        "diagnostic-messages.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let refusal = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "diagnostic-messages".to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .expect_err("an enum cannot declare the same case twice");
    let AssemblyCause::DuplicateEnumMember {
        enumeration, case, ..
    } = &refusal.errors[0].cause
    else {
        panic!(
            "expected duplicate case refusal, got {:?}",
            refusal.errors[0]
        );
    };
    assert_eq!(enumeration, "Shade");
    assert_eq!(case, "Red");
}
