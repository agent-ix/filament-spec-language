// SPDX-License-Identifier: AGPL-3.0-or-later
//! Generated values are tested against the actual checked expression type.
//! The generated operators are total on these admitted parameter assignments;
//! no runtime-only operation is exempted from successful completion.

use super::typed_value_case::{body_type, call, compile, family, inside, sampled_source};
use fsl_bench::prop::{self, value};
use proptest::prelude::{any, prop_assert, TestCaseError};
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

fn membership(
    input: &value::TypedExpression,
    assignment: u8,
) -> Result<&'static str, TestCaseError> {
    let source = sampled_source(input, assignment);
    let package =
        compile(&source).map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
    let checked = body_type(&package, "f");
    let result = call(&package, "probe", Vec::new())
        .map_err(|error| TestCaseError::fail(format!("{error}\n{source}")))?;
    prop_assert!(
        inside(&result, &checked),
        "source={source}\nassignment seed={assignment}\nchecked={checked:?}\nvalue={result:?}"
    );
    let declared = package
        .graph()
        .callable("f")
        .expect("checked function")
        .result;
    prop_assert!(
        inside(&result, declared),
        "source={source}\nassignment seed={assignment}\ndeclared={declared:?}\nvalue={result:?}"
    );
    family(&result).ok_or_else(|| {
        TestCaseError::fail(format!("unsupported returned value {result:?}\n{source}"))
    })
}

// These six ordinary source fixtures guarantee the required domains even
// with a small PROPTEST_CASES override. Random cases still use the unchanged
// authoritative expression strategy rather than filtering checker outcomes.
fn required_family_cases() -> Vec<value::TypedExpression> {
    use value::{CollectionKind, DecimalMode, Expr, Type, TypedExpression};
    let mut cases = vec![
        TypedExpression {
            parameters: Vec::new(),
            result: Type::BoundedInt {
                lower: -2,
                upper: 3,
            },
            expression: Expr::Integer(3),
        },
        TypedExpression {
            parameters: Vec::new(),
            result: Type::Rational,
            expression: Expr::Rational {
                numerator: -3,
                denominator: 2,
            },
        },
        TypedExpression {
            parameters: Vec::new(),
            result: Type::Decimal {
                mode: DecimalMode::Exact,
            },
            expression: Expr::Decimal {
                coefficient: 17,
                scale: 2,
            },
        },
    ];
    for kind in [
        CollectionKind::Sequence,
        CollectionKind::Set,
        CollectionKind::Bag,
    ] {
        cases.push(TypedExpression {
            parameters: Vec::new(),
            result: Type::Collection {
                kind,
                member: Box::new(Type::Integer),
            },
            expression: Expr::Let {
                name: "item".into(),
                ty: Type::Integer,
                value: Box::new(Expr::Integer(2)),
                body: Box::new(Expr::Collection {
                    kind,
                    members: vec![
                        Expr::Parameter("item".into()),
                        Expr::Parameter("item".into()),
                    ],
                }),
            },
        });
    }
    cases
}

/// Provenance: QSL-2056. The membership oracle reads real S3 node types.
#[test]
fn p1_completed_values_lie_inside_the_checked_type() {
    let covered = RefCell::new(BTreeSet::new());
    let required_done = Cell::new(false);
    prop::run(
        "p1_completed_values_lie_inside_the_checked_type",
        prop::config(256),
        &(value::expressions(), any::<u8>()),
        |(input, assignment)| {
            if !required_done.get() {
                for required in required_family_cases() {
                    covered
                        .borrow_mut()
                        .insert(membership(&required, assignment)?);
                }
                required_done.set(true);
            }
            covered.borrow_mut().insert(membership(&input, assignment)?);
            Ok(())
        },
    )
    .unwrap();
    for expected in ["integer", "rational", "decimal", "sequence", "set", "bag"] {
        assert!(
            covered.borrow().contains(expected),
            "no actual {expected} result was evaluated"
        );
    }
}

/// Provenance: QSL-2056. No refusal, undefined result, or internal failure is
/// allowed for the generator's total expressions and in-domain arguments.
#[test]
fn p2_no_completed_call_returns_runtime_invariant() {
    prop::run(
        "p2_no_completed_call_returns_runtime_invariant",
        prop::config(256),
        &(value::expressions(), any::<u8>()),
        |(input, assignment)| {
            let source = sampled_source(&input, assignment);
            let package = compile(&source)
                .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
            call(&package, "probe", Vec::new())
                .map_err(|error| TestCaseError::fail(format!("{error}\n{source}")))?;
            Ok(())
        },
    )
    .unwrap();
}
