// SPDX-License-Identifier: AGPL-3.0-or-later
//! `convert` across a dimension equivalence class, from source text to its
//! evaluated value (FR-093 "Dimension equivalence", FR-093-AC-46; FSpec
//! FR-142-AC-26).

use filament_eval::FamilyOutcome;
use filament_exact::{
    Integer, Meter, Outcome, Quantity as ExactQuantity, Rational, Value, ValueType,
};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;

use super::quantity_literals::{package_with, UNLIMITED};

/// FR-091-AC-44's `Time` units, `Beat` units and the equivalence
/// `equivalence` declares between them.
fn units(equivalence: &str) -> String {
    format!(
        "dimension Time;\n\
         unit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         dimension Beat;\n\
         unit beat : Beat = rational(1, 1);\n\
         unit bar : Beat = rational(4, 1) * beat;\n\
         {equivalence}\n"
    )
}

const QUARTER: &str = "dimension Beat ~ Time by 1 beat = rational(1, 4) s;";
const EQUAL: &str = "dimension Beat ~ Time by 1 beat = 1 s;";

fn fraction(numerator: i64, denominator: i64) -> Rational {
    Rational::from_integer(Integer::from(numerator))
        .div(&Rational::from_integer(Integer::from(denominator)))
        .expect("a nonzero denominator")
}

/// The Boolean function `name` over its one quantity parameter =
/// `magnitude` of that parameter's unit.
fn truth_at(package: &CheckedPackage, name: &str, magnitude: Rational) -> bool {
    let callable = package.graph().callable(name).expect("a callable");
    let ValueType::Quantity(unit) = &callable.parameters[0].1 else {
        panic!("{name} takes a quantity");
    };
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            vec![Value::Quantity(ExactQuantity::new(magnitude, unit.clone()))],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .unwrap_or_else(|error| panic!("the {name} call runs: {error:?}"));
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(truth))) => truth,
        other => panic!("a completed Boolean, not {other:?}"),
    }
}

const FUNCTIONS: &str = "\
    function u1 using v(d: Quantity<beat>): Boolean pure { convert<Quantity<s>>(d) = rational(1, 4) s }\n\
    function u2 using v(d: Quantity<s>): Boolean pure { convert<Quantity<beat>>(d) = 12 beat }\n\
    function u3 using v(d: Quantity<bar>): Boolean pure { convert<Quantity<ms>>(d) = 1000 ms }\n";

/// FR-093-AC-46 (root ratio 1/4, then 1): a conversion across the class
/// evaluates to the exact value through the class root.
#[trace("FR-093-AC-46", "FSpec-FR-142-AC-26")]
#[test]
fn a_conversion_across_an_equivalence_class_is_exact() {
    let quarter = package_with(&format!("{}{FUNCTIONS}", units(QUARTER))).expect("checks");
    assert!(truth_at(&quarter, "u1", fraction(1, 1)));
    assert!(!truth_at(&quarter, "u1", fraction(2, 1)));
    assert!(truth_at(&quarter, "u2", fraction(3, 1)));
    assert!(!truth_at(&quarter, "u2", fraction(4, 1)));
    assert!(truth_at(&quarter, "u3", fraction(1, 1)));
    assert!(!truth_at(&quarter, "u3", fraction(2, 1)));

    let equal = package_with(&format!("{}{FUNCTIONS}", units(EQUAL))).expect("checks");
    assert!(!truth_at(&equal, "u1", fraction(1, 1)));
    assert!(truth_at(&equal, "u1", fraction(1, 4)));
    assert!(truth_at(&equal, "u2", fraction(12, 1)));
}

/// FR-093-AC-46 u5: a sum of two units of a class refuses
/// `ill_typed`/`type-mismatch` under either root ratio; only `convert`
/// relates them.
#[trace("FR-093-AC-46")]
#[test]
fn a_sum_across_an_equivalence_class_refuses_type_mismatch() {
    use filament_exact::IllTypedCause;
    use fsl_semantics::check::CheckCause;
    for equivalence in [QUARTER, EQUAL] {
        let refusals = package_with(&format!(
            "{}function u5 using v(d: Quantity<s>, p: Quantity<beat>): Boolean pure \
             {{ d + p = d + p }}\n",
            units(equivalence)
        ))
        .expect_err("the sum refuses");
        assert!(
            matches!(
                refusals.as_slice(),
                [refusal] if refusal.cause == CheckCause::IllTyped(IllTypedCause::TypeMismatch)
            ),
            "{refusals:?}"
        );
    }
}

/// FR-093-AC-46 u4: a quotient takes each operand in the class root, so
/// both quotients of `6 s` are the same dimensionless value.
#[trace("FR-093-AC-46")]
#[test]
fn a_quotient_across_an_equivalence_class_takes_the_class_root() {
    let package = package_with(&format!(
        "{}function u4 using v(d: Quantity<s>): Boolean pure \
         {{ d / 3 beat = d / rational(3, 4) s }}\n",
        units(QUARTER)
    ))
    .expect("checks");
    assert!(truth_at(&package, "u4", fraction(6, 1)));
}
