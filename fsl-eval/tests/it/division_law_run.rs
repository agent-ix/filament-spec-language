// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-670, FR-495-AC-17 and FR-100-AC-20 at S6a: `div` and `rem` run under
//! the law a unit header selects, and a run-time zero divisor is reached only
//! by a body checked under `CheckMode::Kernel`, because `CheckMode::Linked`
//! refuses a possibly-zero divisor at check.

use filament_eval::FamilyOutcome;
use filament_exact::{Integer, Meter, Outcome, ScalarLimits, Undefined, Value};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, Obligation, PackageDeclarations,
};
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

/// The unit's declarations checked under `mode`, with no caller-supplied
/// division law.
fn check(
    headers: &str,
    declarations: &str,
    mode: CheckMode,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.value.complete/v1\";\n{headers}{declarations}\n"
    );
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("a", "division-run"),
        "division-run.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
    .check(mode, CheckingLimits::default())
}

fn call(package: &CheckedPackage, name: &str, arguments: &[i64]) -> (Outcome<Value>, bool) {
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            arguments
                .iter()
                .map(|value| Value::Integer(Integer::from(*value)))
                .collect(),
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call runs");
    let located = evaluation.location.is_some();
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => (outcome, located),
        other => panic!("a kernel outcome, not {other:?}"),
    }
}

fn integer(value: i64) -> String {
    format!(
        "{:?}",
        Outcome::Completed(Value::Integer(Integer::from(value)))
    )
}

fn division_by_zero() -> String {
    format!(
        "{:?}",
        Outcome::<Value>::Undefined(Undefined::DivisionByZero)
    )
}

fn only_nonzero(refusals: &[CheckRefusal]) {
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(
        matches!(refusals[0].cause, CheckCause::Unproved(Obligation::Nonzero)),
        "{refusals:?}"
    );
}

#[trace("TC-1780", "FR-670-AC-1", "FSpec-TC-1280", "FSpec-FR-570-AC-1")]
#[test]
fn div_and_rem_run_under_the_law_the_header_selects() {
    let declarations = "function q using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a div b }\n\
                        function r using v(a: Int[-7, 7], b: Int[1, 3]): Int[-7, 7] pure { a rem b }";
    for (law, quotient, remainder) in [
        ("truncating", -2, -1),
        ("floor", -3, 2),
        ("euclidean", -3, 2),
    ] {
        let header = format!("profile d = \"filament.value.integer-division.{law}/v1\";\n");
        let package = CheckedPackage::link(
            check(&header, declarations, CheckMode::Linked)
                .unwrap_or_else(|refusals| panic!("{law}: {refusals:?}")),
        );
        assert_eq!(
            format!("{:?}", call(&package, "q", &[-7, 3]).0),
            integer(quotient),
            "{law}"
        );
        assert_eq!(
            format!("{:?}", call(&package, "r", &[-7, 3]).0),
            integer(remainder),
            "{law}"
        );
    }
}

/// FR-100-AC-20 as rewritten: the recursive unit is refused at check under
/// `Linked`, so the former run-time `Undefined` of a `Linked` package is no
/// longer asserted; under `Kernel` the call stops undefined at the `mod`.
#[trace("TC-444", "FR-100-AC-20")]
#[test]
#[ignore = "QSL-2002: Linked admits a mod whose divisor may be zero, so it is undefined at run instead of refused at check"]
fn a_possibly_zero_mod_divisor_is_refused_under_linked_and_undefined_only_under_kernel() {
    let unit = "function d using v(n: Int[0, 9]): Integer pure decreases(n) \
                { if n = 0 then 0 else d(n - 1) + 10 mod (n - 1) }\n\
                function e using v(n: Int[0, 9]): Integer pure { d(n) }";
    only_nonzero(&check("", unit, CheckMode::Linked).expect_err("n - 1 may be zero"));
    let package = CheckedPackage::link(
        check("", unit, CheckMode::Kernel).expect("Kernel records no nonzero obligation"),
    );
    assert_eq!(format!("{:?}", call(&package, "d", &[0]).0), integer(0));
    for (name, n) in [("d", 1), ("d", 2), ("d", 3), ("e", 1)] {
        let (outcome, located) = call(&package, name, &[n]);
        assert_eq!(format!("{outcome:?}"), division_by_zero(), "{name}({n})");
        assert!(located, "{name}({n}) is located at the mod");
    }
}

#[trace("TC-1783", "FR-495-AC-17", "FSpec-TC-1281", "FSpec-FR-147-AC-13")]
#[test]
#[ignore = "QSL-1962: Linked records no nonzero obligation for an integer mod yet"]
fn a_divisor_the_checker_cannot_exclude_is_undefined_only_under_kernel() {
    let source = "function m0 using v(a: Int[0, 100], b: Int[0, 12]): Int[0, 11] pure { a mod b }";
    only_nonzero(&check("", source, CheckMode::Linked).expect_err("b may be zero"));
    let package = CheckedPackage::link(
        check("", source, CheckMode::Kernel).expect("Kernel records no nonzero obligation"),
    );
    let (outcome, located) = call(&package, "m0", &[5, 0]);
    assert_eq!(format!("{outcome:?}"), division_by_zero());
    assert!(located);
    assert_eq!(
        format!("{:?}", call(&package, "m0", &[17, 12]).0),
        integer(5)
    );
}
