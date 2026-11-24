// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-186 enum identity vectors executed through the real source checker.

use filament_eval::FamilyOutcome;
use filament_exact::{
    ComparisonOperator, IllTyped, IllTypedCause, Meter, Outcome, ScalarLimits, Value,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use filament_semantic_value::enumeration::{compare_enum, EnumValue};
use filament_specification::{QualificationVector, TestCase, QUALIFICATION_VECTORS};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckCause, PackageDeclarations};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;

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

fn source_declarations(body: &str) -> PackageDeclarations {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n{body}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("qualification", "enum-row"),
        "enum-row.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source intake succeeds");
    assert!(
        parsed.is_admissible(),
        "source syntax: {:?}",
        parsed.diagnostics()
    );
    let unit = fsl_forms::build_unit(&parsed).expect("admitted source builds forms");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "qualification-enum-rows".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("source declarations assemble");
    declarations
}

fn source_package(body: &str) -> Result<CheckedPackage, Vec<fsl_semantics::check::CheckRefusal>> {
    source_declarations(body)
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .map(CheckedPackage::link)
}

fn enum_members(body: &str) -> Vec<EnumValue> {
    source_declarations(body).enums[0].members.clone()
}

fn enum_member<'a>(members: &'a [EnumValue], case: &str) -> &'a EnumValue {
    members
        .iter()
        .find(|member| member.case() == case)
        .expect("requested case is an admitted member")
}

fn refusal_code(body: &str) -> String {
    let refusals = source_package(body).expect_err("the vector requires a typed refusal");
    let refusal = refusals.first().expect("one first refusal");
    assert!(
        matches!(refusal.cause.as_ref(), CheckCause::IllTyped(_)),
        "expected ill_typed, got {:?}",
        refusal.cause
    );
    refusal.cause.code().as_str().to_owned()
}

fn published_vector(id: &str) -> &'static QualificationVector {
    let vector = QUALIFICATION_VECTORS
        .iter()
        .find(|vector| vector.id == id)
        .expect("published enum vector identity");
    assert_eq!(vector.test_case, TestCase::Tc186);
    vector
}

fn boolean(package: &CheckedPackage, name: &str) -> bool {
    let result = package
        .call(
            &QualifiedName::unqualified(name).expect("valid function name"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("checked enum function runs");
    let FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(value))) = result.outcome else {
        panic!("expected a Boolean outcome, got {:?}", result.outcome);
    };
    value
}

/// TC-186 T07 and T09: spelling equality does not unify declarations, and
/// changing a declaration's member set or order still produces a new type.
#[trace("FSpec-TC-186", "FSpec-FR-141-AC-2", "FSpec-FR-141-AC-5")]
#[test]
fn t07_t09_cross_declaration_members_are_ill_typed() {
    let t07 = published_vector("T07");
    assert!(t07.expected.contains("ill-typed"));
    let body = "enum A { READY }\nenum B { READY }\n\
         function compare using v(a: A, b: B): Boolean pure { a = b }";
    assert_eq!(refusal_code(body), "ill_typed");

    let t09 = published_vector("T09");
    assert!(t09.expected.contains("ill_typed"));
    let original = enum_members("ordered enum B { READY, DONE }");
    let original_again = enum_members("ordered enum B { READY, DONE }");
    assert_eq!(
        compare_enum(
            ComparisonOperator::Equal,
            enum_member(&original, "READY"),
            enum_member(&original_again, "READY"),
            &mut Meter::new(UNLIMITED),
        ),
        Ok(Outcome::Completed(true)),
        "unchanged declaration identity and shape compare equal"
    );

    for (difference, changed_declaration, expected_members) in [
        (
            "case set",
            "ordered enum B { READY, RUN, DONE }",
            ["READY", "RUN", "DONE"].as_slice(),
        ),
        (
            "case order",
            "ordered enum B { DONE, READY }",
            ["DONE", "READY"].as_slice(),
        ),
    ] {
        let changed = enum_members(changed_declaration);
        assert_eq!(
            changed[0]
                .declaration_members()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            expected_members,
            "{difference}: the changed enum has the literal expected members"
        );
        let original_declaration = original[0].declaration().clone();
        let changed_declaration = changed[0].declaration().clone();
        assert_eq!(
            original_declaration, changed_declaration,
            "{difference}: the source owner and enum name are unchanged"
        );
        assert_eq!(
            compare_enum(
                ComparisonOperator::Equal,
                enum_member(&original, "READY"),
                enum_member(&changed, "READY"),
                &mut Meter::new(UNLIMITED),
            ),
            Err(IllTyped {
                cause: IllTypedCause::DistinctEnumDeclarations
            }),
            "{difference}: changed enum content is a distinct identity"
        );
    }
}

/// TC-186 T08: ordered comparison follows declared order; the same operation
/// is refused for an unordered declaration before evaluation.
#[trace("FSpec-TC-186", "FSpec-FR-141-AC-5")]
#[test]
fn t08_ordering_uses_only_an_ordered_enum() {
    let vector = published_vector("T08");
    assert!(vector.expected.contains("READY < DONE"));
    let ordered = source_package(
        "ordered enum Status { READY, DONE }\n\
         function less using v(): Boolean pure { Status::READY < Status::DONE }",
    )
    .expect("ordered enum comparison is admitted");
    assert!(boolean(&ordered, "less"));

    let unordered = "enum Status { READY, DONE }\n\
         function less using v(): Boolean pure { Status::READY < Status::DONE }";
    assert_eq!(refusal_code(unordered), "ill_typed");
}

/// TC-186 T10: equal text payloads under distinct normalization profiles are
/// not interchangeable Boolean operands.
#[trace("FSpec-TC-186", "FSpec-FR-141-AC-4")]
#[test]
fn t10_text_profiles_remain_distinct_types() {
    let vector = published_vector("T10");
    assert!(vector.expected.contains("ill-typed"));
    let body = "function normalized using v(): Text[0,8;nfc] pure { Text(\"é\") }\n\
         function binary using v(): Text[0,8;binary-utf8] pure { Text(\"é\") }\n\
         function compare using v(): Boolean pure { normalized() = binary() }";
    assert_eq!(refusal_code(body), "ill_typed");
}

/// TC-942 step 4: an ordinary scalar enum member name that is not an
/// identifier cannot enter the admitted enum-member row.
#[trace("FSpec-TC-942", "FSpec-FR-141-AC-8")]
#[test]
fn ordinary_enum_member_rows_reject_non_identifier_case_names() {
    let declaration = filament_exact::NodeKey::new(
        "qualification-enum-rows",
        vec![filament_exact::PathSegment::Text("enum".to_owned())],
        0,
    )
    .expect("fixture declaration node key");
    let refusal =
        fsl_semantics::value::enumeration::EnumMemberPreimage::new(declaration, "Gold Plus")
            .expect_err("ordinary enum cases are identifiers");
    assert_eq!(
        refusal.cause,
        filament_semantic_value::semantic_node::SemanticGraphCause::NonCanonicalPreimage
    );
    assert_eq!(
        filament_semantic_value::semantic_node::InvalidSemanticGraph::CODE,
        "invalid_semantic_graph"
    );
}
