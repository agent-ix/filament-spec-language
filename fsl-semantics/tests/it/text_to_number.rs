// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-478, FR-093-AC-35/-47 and TC-1196: text conversions type and lower.

use filament_exact::{IllTypedCause, RoundingMode};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_semantic_value::location::ExpressionOwner as Origin;
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use fsl_semantics::check::{
    ApplicationTerm, BodyTerm, CheckCause, CheckRefusal, CheckedGraph, LawRole, OperationMode,
    PackageDeclarations, SemanticNode,
};
use fsl_semantics::value::{admit_ieee_profile, DefinitionRole};
use ix_trace_rs::trace;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"filament.value.complete/v1\";\n";

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "text-convert"),
        "text-convert.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "text-convert-tests".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the package declarations");
    declarations.ieee_profile = Some(
        admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[])
            .expect("the standard IEEE profile is admitted"),
    );
    declarations.check(CheckMode::Linked, CheckingLimits::default())
}

fn applications(graph: &CheckedGraph) -> Vec<&ApplicationTerm> {
    graph
        .semantic_graph()
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(term) => Some(term),
            _ => None,
        })
        .collect()
}

const CONVERSIONS: &str = "\
    dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;\n\
    function ti using v(t: Text[0, 8; nfc]): Integer pure { convert<Integer>(t) }\n\
    function td using v(t: Text[0, 8; nfc]): Decimal[0, 1000; 0, 2; nearest-even] pure { convert<Decimal[0, 1000; 0, 2; nearest-even]>(t) }\n\
    function tr using v(t: Text[0, 8; nfc]): Rational[-9, 9; 1, 9] pure { convert<Rational[-9, 9; 1, 9]>(t) }\n\
    function tf32 using v(t: Text[0, 8; nfc]): Float32[nearest-even] pure { convert<Float32[nearest-even]>(t) }\n\
    function tf64 using v(t: Text[0, 8; nfc]): Float64 pure { convert<Float64>(t) }\n\
    function tq using v(t: Text[0, 8; nfc]): Quantity<ms> pure { convert<Quantity<ms>>(t) }\n\
    function si using v(t: String): Integer pure { convert<Integer>(t) }\
    function sd using v(t: String): Decimal[0, 1000; 0, 2; nearest-even] pure { convert<Decimal[0, 1000; 0, 2; nearest-even]>(t) }\n\
    function sr using v(t: String): Rational[-9, 9; 1, 9] pure { convert<Rational[-9, 9; 1, 9]>(t) }\n\
    function sf32 using v(t: String): Float32[nearest-even] pure { convert<Float32[nearest-even]>(t) }\n\
    function sf64 using v(t: String): Float64 pure { convert<Float64>(t) }\n\
    function sq using v(t: String): Quantity<ms> pure { convert<Quantity<ms>>(t) }\n";

/// FR-093-AC-35 / TC-1196: each of six targets over Text and String lowers
/// to its text/string operation identity with a type argument and one source.
#[trace("FSpec-FR-093-AC-35", "FSpec-TC-1196", "FSpec-FR-478-AC-3")]
#[test]
fn six_text_and_string_conversions_lower_to_their_identities() {
    let graph = check(CONVERSIONS).expect("all text conversions are admitted");
    let terms = applications(&graph);
    for identity in [
        "filament.op.text.to_integer",
        "filament.op.text.to_decimal",
        "filament.op.text.to_rational",
        "filament.op.text.to_float32",
        "filament.op.text.to_float64",
        "filament.op.text.to_quantity",
        "filament.op.string.to_integer",
        "filament.op.string.to_decimal",
        "filament.op.string.to_rational",
        "filament.op.string.to_float32",
        "filament.op.string.to_float64",
        "filament.op.string.to_quantity",
    ] {
        let found: Vec<_> = terms
            .iter()
            .filter(|term| term.operation.identity() == identity)
            .collect();
        assert_eq!(found.len(), 1, "{identity} is one operation");
        assert!(
            found[0].operation.member().is_some(),
            "{identity} type argument"
        );
        assert_eq!(found[0].arguments.len(), 1, "{identity} has one operand");
        let operation = &found[0].operation;
        let source_is_text = identity.starts_with("filament.op.text.");
        let is_float = identity.ends_with("to_float32") || identity.ends_with("to_float64");
        let expected_laws: Vec<_> = match (source_is_text, is_float) {
            (true, true) => vec![LawRole::TextProfile, LawRole::IeeeProfile],
            (true, false) => vec![LawRole::TextProfile],
            (false, true) => vec![LawRole::IeeeProfile],
            (false, false) => vec![],
        };
        assert_eq!(
            operation
                .laws()
                .iter()
                .map(|law| law.role)
                .collect::<Vec<_>>(),
            expected_laws,
            "{identity} laws"
        );
        let expected_mode = if identity.ends_with("to_decimal") || identity.ends_with("to_float32")
        {
            Some(OperationMode::Rounding(RoundingMode::NearestEven))
        } else if identity.ends_with("to_float64") || identity.ends_with("to_quantity") {
            Some(OperationMode::Rounding(RoundingMode::Exact))
        } else {
            None
        };
        assert_eq!(operation.mode(), expected_mode.as_ref(), "{identity} mode");
    }
}

/// FR-093-AC-47 / TC-1196: conversion under equality remains the typed text
/// operation, while a same-type Text conversion builds no operation.
#[trace("FSpec-FR-093-AC-47", "FSpec-TC-1196")]
#[test]
fn converted_equality_uses_text_operation_and_same_type_is_identity() {
    let graph = check(
        "function g using v(t: Text[0, 8; nfc]): Boolean pure { convert<Integer>(t) = 3 }\n\
         function same using v(t: Text[0, 8; nfc]): Text[0, 8; nfc] pure { convert<Text[0, 8; nfc]>(t) }",
    )
    .expect("the conversion and equality check");
    let terms = applications(&graph);
    assert!(terms
        .iter()
        .any(|term| term.operation.identity() == "filament.op.text.to_integer"));
    assert!(terms
        .iter()
        .any(|term| term.operation.identity() == "filament.op.integer.eq"));
    assert!(terms.iter().all(|term| !term
        .operation
        .identity()
        .starts_with("filament.op.numeric.convert")));
}

/// FR-478-AC-3: unsupported targets refuse as operator-ineligible.
#[trace("FSpec-FR-478-AC-3", "FSpec-FR-093-AC-47", "FSpec-TC-1196")]
#[test]
fn non_numeric_text_conversion_targets_are_operator_ineligible() {
    for target in ["Boolean", "Text[0, 4; nfc]", "String"] {
        let declarations = format!(
            "function bad using v(t: Text[0, 8; nfc]): {target} pure {{ convert<{target}>(t) }}"
        );
        let refusals = check(&declarations).expect_err("target is not a number");
        assert_eq!(refusals.len(), 1, "{target}: {refusals:?}");
        assert_eq!(
            refusals[0].cause,
            CheckCause::IllTyped(IllTypedCause::OperatorIneligible),
            "{target}"
        );
        assert!(matches!(
            refusals[0].location.origin,
            Origin::Body { function, .. } if function == "bad"
        ));
    }
}
