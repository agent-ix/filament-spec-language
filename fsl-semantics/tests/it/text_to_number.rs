// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2241: text-to-number conversion type checking and checked lowering.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{
    ApplicationTerm, AssemblyRefusal, BodyTerm, CheckRefusal, CheckedGraph, LawRole, LeafTerm,
    MemberTerm, OperationMode, Operator, PackageDeclarations, SemanticNode,
};
use fsl_semantics::model::intake::SelectedModels;
use fsl_semantics::value::{DefinitionRole, Member};
use ix_trace_rs::trace;

const UNITS: &str =
    "dimension T;\nunit s : T = rational(1, 1);\nunit ms : T = rational(1, 1000) * s;\n";

fn unit_text(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n\
         {declarations}\n"
    )
}

fn assemble(declarations: &str) -> Result<PackageDeclarations, AssemblyRefusal> {
    let text = unit_text(declarations);
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "text-to-number"),
        "text-to-number.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
}

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    assemble(declarations)
        .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
        .check(CheckMode::Linked, CheckingLimits::default())
}

fn assert_refused(declarations: &str, code: Code, cause: &str) {
    let refusals = check(declarations).expect_err("the expression is refused");
    let [refusal] = refusals.as_slice() else {
        panic!("one typed refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause.code(), code, "{refusal:?}");
    assert_eq!(refusal.cause.cause(), Some(cause), "{refusal:?}");
}

fn nodes(graph: &CheckedGraph) -> Vec<&SemanticNode> {
    graph.semantic_graph().nodes().collect()
}

fn applications(graph: &CheckedGraph) -> Vec<&ApplicationTerm> {
    nodes(graph)
        .into_iter()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application) => Some(application),
            _ => None,
        })
        .collect()
}

#[trace("TC-1196", "FR-093-AC-35")]
#[test]
fn text_and_string_conversions_build_their_typed_operations() {
    let graph = check(&format!(
        "{UNITS}\
         function ti using v(t: Text[0, 8; nfc]): Int[0, 99] pure {{ convert<Int[0, 99]>(t) }}\n\
         function td using v(t: Text[0, 8; nfc]): Decimal[0, 1000; 0, 2; nearest-even] pure {{ convert<Decimal[0, 1000; 0, 2; nearest-even]>(t) }}\n\
         function tr using v(t: Text[0, 8; nfc]): Rational[-9, 9; 1, 9] pure {{ convert<Rational[-9, 9; 1, 9]>(t) }}\n\
         function tf32 using v(t: Text[0, 8; nfc]): Float32[toward-zero] pure {{ convert<Float32[toward-zero]>(t) }}\n\
         function tf64 using v(t: Text[0, 8; nfc]): Float64[exact] pure {{ convert<Float64[exact]>(t) }}\n\
         function tq using v(t: Text[0, 8; nfc]): Quantity<ms> pure {{ convert<Quantity<ms>>(t) }}\n\
         function si using v(s: String): Int[0, 99] pure {{ convert<Int[0, 99]>(s) }}\n\
         function sd using v(s: String): Decimal[0, 1000; 0, 2; nearest-even] pure {{ convert<Decimal[0, 1000; 0, 2; nearest-even]>(s) }}\n\
         function sr using v(s: String): Rational[-9, 9; 1, 9] pure {{ convert<Rational[-9, 9; 1, 9]>(s) }}\n\
         function sf32 using v(s: String): Float32[toward-zero] pure {{ convert<Float32[toward-zero]>(s) }}\n\
         function sf64 using v(s: String): Float64[exact] pure {{ convert<Float64[exact]>(s) }}\n\
         function sq using v(s: String): Quantity<ms> pure {{ convert<Quantity<ms>>(s) }}\n"
    ))
    .unwrap_or_else(|refusals| panic!("text number conversions check: {refusals:?}"));
    let expected = [
        ("integer", None, false),
        (
            "decimal",
            Some(filament_exact::RoundingMode::NearestEven),
            false,
        ),
        ("rational", None, false),
        (
            "float32",
            Some(filament_exact::RoundingMode::TowardZero),
            true,
        ),
        ("float64", Some(filament_exact::RoundingMode::Exact), true),
        ("quantity", Some(filament_exact::RoundingMode::Exact), false),
    ];
    let all = applications(&graph);
    for source in ["text", "string"] {
        for (suffix, rounding, ieee) in expected {
            let identity = format!("filament.op.{source}.to_{suffix}");
            let found: Vec<_> = all
                .iter()
                .filter(|application| application.operation.identity() == identity)
                .collect();
            assert_eq!(found.len(), 1, "{identity}");
            let application = found[0];
            assert_eq!(application.operator, Operator::Convert, "{identity}");
            assert_eq!(application.arguments.len(), 1, "{identity}");
            assert!(
                matches!(
                    application.arguments[0],
                    MemberTerm::Leaf(LeafTerm::Reference { .. })
                ),
                "{identity}: argument is a reference"
            );
            let member = application
                .operation
                .member()
                .expect("type_argument member");
            let Member::TypeArgument { declaration } = member else {
                panic!("{identity}: type_argument member, got {member:?}");
            };
            assert_eq!(
                application.result_type.admitted(),
                Some(declaration),
                "{identity}: type_argument is the result type"
            );

            let roles: Vec<_> = application
                .operation
                .laws()
                .iter()
                .map(|law| law.role)
                .collect();
            let mut expected_roles = Vec::new();
            if source == "text" {
                expected_roles.push(LawRole::TextProfile);
            }
            if ieee {
                expected_roles.push(LawRole::IeeeProfile);
            }
            assert_eq!(roles, expected_roles, "{identity}");
            for law in application.operation.laws() {
                let expected_definition = match law.role {
                    LawRole::TextProfile => DefinitionRole::TextProfile.reference(),
                    LawRole::IeeeProfile => DefinitionRole::IeeeProfile.reference(),
                    _ => panic!("unexpected law on {identity}: {:?}", law.role),
                };
                assert_eq!(law.definition, expected_definition, "{identity}");
            }
            assert!(
                !matches!(
                    application.operation.mode(),
                    Some(OperationMode::TextProfile(_))
                ),
                "{identity}: profile belongs to the law, not the mode"
            );
            assert_eq!(
                application.operation.mode(),
                rounding.map(OperationMode::Rounding).as_ref(),
                "{identity}"
            );
        }
    }
    assert_eq!(
        all.iter()
            .filter(|application| application.operator == Operator::Convert)
            .count(),
        12,
        "only the twelve expected text/string conversions are built"
    );
    for application in all.iter().filter(|application| {
        application
            .operation
            .identity()
            .starts_with("filament.op.text.to_")
            || application
                .operation
                .identity()
                .starts_with("filament.op.string.to_")
    }) {
        assert_eq!(application.arguments.len(), 1);
        let MemberTerm::Leaf(LeafTerm::Reference { target }) = &application.arguments[0] else {
            unreachable!("checked above")
        };
        assert!(target.admitted().is_some(), "reference to source parameter");
    }
}

#[trace("TC-1196", "FR-093-AC-47")]
#[test]
fn text_conversion_inside_equality_references_the_convert_node() {
    let graph =
        check("function g using v(t: Text[0, 8; nfc]): Boolean pure { convert<Integer>(t) = 3 }")
            .expect("the converted equality checks");
    let all = nodes(&graph);
    let converted: Vec<_> = all
        .iter()
        .filter(|node| {
            matches!(node.body(), BodyTerm::Application(term)
                if term.operation.identity() == "filament.op.text.to_integer")
        })
        .collect();
    assert_eq!(converted.len(), 1, "one text-to-integer node");
    let equality: Vec<_> = all
        .iter()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(term)
                if term.operation.identity() == "filament.op.integer.eq" =>
            {
                Some(term)
            }
            _ => None,
        })
        .collect();
    assert_eq!(equality.len(), 1, "one integer equality");
    assert_eq!(equality[0].operator, Operator::Binary);
    assert_eq!(equality[0].arguments.len(), 2);
    let MemberTerm::Leaf(LeafTerm::Reference {
        target: converted_ref,
    }) = &equality[0].arguments[0]
    else {
        panic!("first equality argument references the conversion");
    };
    assert_eq!(converted_ref.admitted(), Some(&converted[0].key()));
    assert!(!all.iter().any(|node| {
        matches!(node.body(), BodyTerm::Application(term)
            if term.operation.identity() == "filament.op.numeric.convert")
    }));
}

#[trace("TC-1196", "FR-093-AC-47")]
#[test]
fn text_to_non_numeric_targets_is_refused_as_operator_ineligible() {
    assert_refused(
        "function f using v(t: Text[0, 8; nfc]): Boolean pure { convert<Boolean>(t) }",
        Code::IllTyped,
        "operator-ineligible",
    );
    assert_refused(
        "function f using v(t: Text[0, 8; nfc]): Text[0, 4; nfc] pure { convert<Text[0, 4; nfc]>(t) }",
        Code::IllTyped,
        "operator-ineligible",
    );
    assert_refused(
        "function f using v(s: String): Text[0, 8; nfc] pure { convert<Text[0, 8; nfc]>(s) }",
        Code::IllTyped,
        "operator-ineligible",
    );
}

#[trace("TC-1196", "FR-093-AC-47")]
#[test]
fn conversion_to_the_same_text_type_is_an_identity_without_a_node() {
    let graph = check(
        "function f using v(t: Text[0, 8; nfc]): Text[0, 8; nfc] pure { convert<Text[0, 8; nfc]>(t) }",
    )
    .expect("same-type conversion is identity");
    assert!(!applications(&graph).iter().any(|application| {
        application
            .operation
            .identity()
            .starts_with("filament.op.text.to_")
    }));
}
