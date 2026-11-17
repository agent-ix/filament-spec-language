// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-981 (TC-2774) and FSpec FR-783: `size<N>(c)` is typed `N` and its
//! length interval must lie inside `N`'s domain.
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::{Code, SourceIdentity};
use fsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, Obligation, PackageDeclarations,
};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;

/// S1 to S3 of `header` and `declarations` under `mode`; S1 and S2 must
/// accept the unit.
fn check_in(
    header: &str,
    declarations: &str,
    mode: CheckMode,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("language \"ix:native\" edition \"1-draft\";\n{header}{declarations}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "w17"),
        "w17.native",
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
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
    .check(mode, CheckingLimits::default())
}

const V: &str = "profile v = \"filament.value.complete/v1\";\n";

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    check_in(V, declarations, CheckMode::Linked)
}

fn assert_checks(declarations: &str) {
    if let Err(refusals) = check(declarations) {
        panic!("{declarations} checks: {refusals:?}");
    }
}

/// The first refusal of `declarations` has `code` and `cause`.
fn assert_refused_with(
    header: &str,
    declarations: &str,
    code: Code,
    cause: Option<&str>,
) -> CheckRefusal {
    let refusals =
        check_in(header, declarations, CheckMode::Linked).expect_err("the unit is refused");
    let first = refusals.first().expect("a refusal").clone();
    assert_eq!(first.cause.code(), code, "{declarations}: {refusals:?}");
    if cause.is_some() {
        assert_eq!(first.cause.cause(), cause, "{declarations}: {refusals:?}");
    }
    first
}

fn assert_refused(declarations: &str, code: Code, cause: &str) -> CheckRefusal {
    assert_refused_with(V, declarations, code, Some(cause))
}

fn sized(result: &str, body: &str) -> String {
    format!("function f using v(c: Sequence<Integer>[0, 4]): {result} pure {{ {body} }}")
}

/// FR-981-AC-3 and AC-5: the range obligation, its guard narrowing, the
/// type-argument and operand refusals, and an alias result domain.
#[trace("TC-2774", "FR-981-AC-3", "FSpec-TC-1900", "FSpec-FR-783-AC-1")]
#[trace("TC-2774", "FR-981-AC-5", "FSpec-TC-1900", "FSpec-FR-783-AC-2")]
#[test]
fn a_typed_size_is_typed_at_its_domain_and_proves_its_range() {
    assert_checks(&sized("Int[0, 4]", "size<Int[0, 4]>(c)"));
    let narrow = assert_refused(
        &sized("Int[0, 3]", "size<Int[0, 3]>(c)"),
        Code::UndefinedExpression,
        "unproved-range",
    );
    assert!(
        matches!(narrow.cause, CheckCause::Unproved(Obligation::Range { .. })),
        "{narrow:?}"
    );
    assert_checks(&sized(
        "Int[0, 3]",
        "if size(c) <= 3 then size<Int[0, 3]>(c) else 0",
    ));
    assert_refused(
        &sized("Boolean", "size<Boolean>(c) = size<Boolean>(c)"),
        Code::IllTyped,
        "type-mismatch",
    );
    assert_refused(
        &sized("Int[0, 4]", "size<Int[0, 4]>(3)"),
        Code::IllTyped,
        "type-mismatch",
    );
    assert_checks(
        "type Count = Int[0, 1000];\n\
         function few using v(c: Sequence<Integer>[0, 4]): Boolean pure { size<Count>(c) >= 0 }",
    );
}
