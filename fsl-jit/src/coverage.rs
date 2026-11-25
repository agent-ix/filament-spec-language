// SPDX-License-Identifier: AGPL-3.0-or-later
//! The conservative construct set the current register backend translates.

use std::sync::OnceLock;

use fsl_eval::execution::{
    BackendCoverage, CoverageOperation, CoverageOperator as Op, CoverageValueType as Ty,
};

/// The expression forms implemented by this backend.
#[must_use]
pub fn coverage() -> &'static BackendCoverage {
    static COVERAGE: OnceLock<BackendCoverage> = OnceLock::new();
    COVERAGE.get_or_init(|| {
        let integer = Ty::Integer;
        let boolean = Ty::Boolean;
        let mut forms = [
            "literal",
            "name",
            "operator",
            "let",
            "if",
            "call",
            "record",
            "collection",
            "population",
            "allinstances",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
        let mut operations = std::collections::BTreeSet::new();
        let mut add = |operator, operands: Vec<Ty>, result: Ty| {
            operations.insert(CoverageOperation {
                operator,
                operand_types: operands,
                result_type: result,
            });
        };
        for operator in [Op::Add, Op::Subtract, Op::Multiply] {
            add(
                operator,
                vec![integer.clone(), integer.clone()],
                integer.clone(),
            );
        }
        add(Op::Negate, vec![integer.clone()], integer.clone());
        for operator in [Op::Divide, Op::Remainder, Op::Modulo] {
            add(
                operator,
                vec![integer.clone(), integer.clone()],
                integer.clone(),
            );
        }
        for operator in [Op::Less, Op::LessOrEqual, Op::Greater, Op::GreaterOrEqual] {
            add(
                operator,
                vec![integer.clone(), integer.clone()],
                boolean.clone(),
            );
        }
        let rational = Ty::Rational;
        let decimal = Ty::Decimal;
        let text = Ty::Text;
        let record = Ty::Record;
        let tuple = Ty::Tuple;
        let collection = Ty::Collection;
        let reference = Ty::Reference;
        for ty in [
            integer.clone(),
            boolean.clone(),
            rational.clone(),
            decimal.clone(),
            text.clone(),
            record.clone(),
            tuple.clone(),
            collection.clone(),
            reference.clone(),
        ] {
            for operator in [Op::Equal, Op::NotEqual] {
                add(operator, vec![ty.clone(), ty.clone()], boolean.clone());
            }
        }
        for (left, right) in [
            (integer.clone(), rational.clone()),
            (rational.clone(), integer.clone()),
            (integer.clone(), decimal.clone()),
            (decimal.clone(), integer.clone()),
            (rational.clone(), decimal.clone()),
            (decimal.clone(), rational.clone()),
        ] {
            for operator in [Op::Equal, Op::NotEqual] {
                add(operator, vec![left.clone(), right.clone()], boolean.clone());
            }
        }
        for operator in [Op::Add, Op::Subtract, Op::Multiply, Op::Divide] {
            add(
                operator,
                vec![rational.clone(), rational.clone()],
                rational.clone(),
            );
        }
        add(Op::Negate, vec![rational.clone()], rational.clone());
        for operator in [Op::Add, Op::Subtract, Op::Multiply, Op::Divide] {
            add(
                operator,
                vec![decimal.clone(), decimal.clone()],
                decimal.clone(),
            );
        }
        add(Op::Negate, vec![decimal.clone()], decimal.clone());
        for ty in [rational.clone(), decimal.clone(), text.clone()] {
            for operator in [Op::Less, Op::LessOrEqual, Op::Greater, Op::GreaterOrEqual] {
                add(operator, vec![ty.clone(), ty.clone()], boolean.clone());
            }
        }
        for ty in [text.clone(), collection.clone()] {
            add(Op::Other("size".to_owned()), vec![ty], integer.clone());
        }
        add(
            Op::Other("coerce".to_owned()),
            vec![integer.clone()],
            integer.clone(),
        );
        for operator in ["flatten", "convert-collection"] {
            add(
                Op::Other(operator.to_owned()),
                vec![collection.clone()],
                collection.clone(),
            );
        }
        add(
            Op::ExactDivide,
            vec![integer.clone(), integer.clone()],
            rational.clone(),
        );
        for source in [rational.clone(), decimal.clone()] {
            add(
                Op::Other("convert-decimal".to_owned()),
                vec![source],
                decimal.clone(),
            );
        }
        for result in [integer.clone(), rational.clone(), decimal.clone()] {
            add(
                Op::Other("convert-text".to_owned()),
                vec![text.clone()],
                result,
            );
        }
        for result in [
            boolean.clone(),
            integer.clone(),
            rational.clone(),
            decimal.clone(),
            text.clone(),
            record.clone(),
            tuple.clone(),
            collection.clone(),
            reference.clone(),
        ] {
            add(
                Op::Other("record-field-read".to_owned()),
                vec![record.clone()],
                result,
            );
        }
        for element in [
            boolean.clone(),
            integer.clone(),
            rational.clone(),
            decimal.clone(),
            text.clone(),
            record.clone(),
            tuple.clone(),
            collection.clone(),
        ] {
            add(
                Op::Other("contains".to_owned()),
                vec![collection.clone(), element],
                boolean.clone(),
            );
        }
        for result in [
            boolean.clone(),
            integer.clone(),
            rational.clone(),
            decimal.clone(),
            text.clone(),
            record.clone(),
            tuple.clone(),
            collection.clone(),
        ] {
            add(
                Op::Other("tuple-project".to_owned()),
                vec![tuple.clone()],
                result.clone(),
            );
            add(
                Op::Other("index".to_owned()),
                vec![collection.clone(), integer.clone()],
                result,
            );
        }
        add(
            Op::Other("drop-first".to_owned()),
            vec![collection.clone()],
            collection.clone(),
        );
        add(
            Op::Other("population-objects".to_owned()),
            vec![],
            collection.clone(),
        );
        add(
            Op::Other("all-instances".to_owned()),
            vec![],
            collection.clone(),
        );
        for element in [
            boolean.clone(),
            integer.clone(),
            rational.clone(),
            decimal.clone(),
            text.clone(),
            record.clone(),
            tuple.clone(),
            collection.clone(),
            reference.clone(),
        ] {
            add(
                Op::Other("remove-one".to_owned()),
                vec![collection.clone(), element],
                collection.clone(),
            );
        }
        for visit in ["map", "filter"] {
            add(
                Op::Other(format!("query-{visit}")),
                vec![collection.clone()],
                collection.clone(),
            );
        }
        add(
            Op::Other("query-count".to_owned()),
            vec![collection.clone()],
            integer.clone(),
        );
        for result in [
            integer.clone(),
            rational.clone(),
            decimal.clone(),
            text.clone(),
            boolean.clone(),
            record.clone(),
            tuple.clone(),
            collection.clone(),
            reference.clone(),
        ] {
            add(
                Op::Other("query-only".to_owned()),
                vec![collection.clone()],
                result,
            );
        }
        for result in [integer.clone(), rational.clone(), decimal.clone()] {
            add(
                Op::Other("query-sum".to_owned()),
                vec![collection.clone()],
                result,
            );
        }
        for result in [
            boolean.clone(),
            integer.clone(),
            rational.clone(),
            decimal.clone(),
            text.clone(),
            record.clone(),
            tuple.clone(),
            collection.clone(),
        ] {
            add(
                Op::Other("fold".to_owned()),
                vec![collection.clone()],
                result,
            );
        }
        for operator in [Op::And, Op::Or, Op::Implies] {
            add(
                operator,
                vec![boolean.clone(), boolean.clone()],
                boolean.clone(),
            );
        }
        add(Op::Not, vec![boolean.clone()], boolean.clone());
        BackendCoverage {
            expression_forms: std::mem::take(&mut forms),
            value_types: [
                integer, boolean, rational, decimal, text, record, tuple, collection, reference,
            ]
            .into_iter()
            .collect(),
            operations,
        }
    })
}
