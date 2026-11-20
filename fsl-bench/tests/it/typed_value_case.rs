// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public source/check/evaluation helpers for the type-soundness properties.

use filament_eval::FamilyOutcome;
use filament_exact::{Integer, Meter, Outcome, Value, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_bench::prop::value::{CollectionKind, Expr, Type, TypedExpression};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckRefusal, PackageDeclarations};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::value::{admit_integer_division, DefinitionRole};

pub(super) fn compile(source: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "typed-value-properties"),
        "typed-values.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("bounded generated input parses");
    assert!(
        parsed.is_admissible(),
        "{:?}\n{source}",
        parsed.diagnostics()
    );
    let unit = fsl_forms::build_unit(&parsed).expect("generated unit builds forms");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "typed-value-properties".into(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .unwrap_or_else(|error| panic!("assembly: {error:?}\n{source}"));
    declarations.integer_division = Some(
        admit_integer_division(&[DefinitionRole::IntegerDivisionFloor.reference()], None)
            .expect("floor is an admitted public division profile"),
    );
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .map(CheckedPackage::link)
}

pub(super) fn call(
    package: &CheckedPackage,
    name: &str,
    arguments: Vec<Value>,
) -> Result<Value, String> {
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("test function has a valid identifier"),
            arguments,
            &ObjectEnvironment::default(),
            &mut Meter::new(fsl_bench::world::UNLIMITED),
        )
        .map_err(|error| format!("call refused: {error:?}"))?;
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => Ok(value),
        other => Err(format!(
            "a defined generated expression did not complete: {other:?}"
        )),
    }
}

pub(super) fn body_type(package: &CheckedPackage, name: &str) -> ValueType {
    let callable = package
        .graph()
        .callable(name)
        .expect("generated function exists");
    package
        .graph()
        .function_by_identity(callable.identity.clone())
        .expect("callable identity resolves to its checked body")
        .body
        .value_type()
        .clone()
}

// Parameter assignments are independent boundary/interior inhabitants of the
// generator's public domain descriptors, rendered by its existing source AST.
fn argument(domain: &Type, seed: u8) -> Expr {
    match domain {
        Type::Integer => Expr::Integer(i64::from(seed % 33) - 16),
        Type::BoundedInt { lower, upper } => {
            let width = upper - lower + 1;
            Expr::Integer(lower + i64::from(seed) % width)
        }
        Type::Boolean => Expr::Boolean(seed % 2 == 0),
        Type::Rational => Expr::Rational {
            numerator: i64::from(seed % 11) - 5,
            denominator: 1 + seed % 3,
        },
        Type::Decimal { .. } => Expr::Decimal {
            coefficient: i64::from(seed % 17) - 8,
            scale: seed % 3,
        },
        Type::Text => Expr::Text("x".repeat(usize::from(seed % 17))),
        Type::Option(member) => Expr::OptionSome(Box::new(argument(member, seed))),
        Type::Record(member) | Type::Tuple(member) => Expr::Let {
            name: "arg_member".into(),
            ty: (**member).clone(),
            value: Box::new(argument(member, seed)),
            body: Box::new(match domain {
                Type::Record(_) => Expr::Record(Box::new(Expr::Parameter("arg_member".into()))),
                Type::Tuple(_) => Expr::Tuple(
                    Box::new(Expr::Parameter("arg_member".into())),
                    seed % 2 == 0,
                ),
                _ => unreachable!("this arm admits only record and tuple domains"),
            }),
        },
        Type::Collection { kind, member } => Expr::Collection {
            kind: *kind,
            members: (0..seed % 5)
                .map(|offset| argument(member, seed.wrapping_add(offset)))
                .collect(),
        },
    }
}

pub(super) fn sampled_source(input: &TypedExpression, seed: u8) -> String {
    let arguments = input
        .parameters
        .iter()
        .enumerate()
        .map(|(index, (_, domain))| {
            argument(domain, seed.wrapping_add(u8::try_from(index).unwrap())).source()
        })
        .collect::<Vec<_>>()
        .join(", ");
    let result = input.result.source().0;
    format!(
        "{}\nfunction probe using v(): {result} pure {{ f({arguments}) }}\n",
        input.source()
    )
}

pub(super) fn exact_integer(integer: &Integer) -> i128 {
    integer
        .to_string()
        .parse()
        .expect("the generated arithmetic domain fits i128")
}

pub(super) fn inside(value: &Value, checked: &ValueType) -> bool {
    match (value, checked) {
        (Value::Integer(_), ValueType::Integer) => true,
        (Value::Integer(value), ValueType::Int(bounds)) => {
            exact_integer(bounds.lower()) <= exact_integer(value)
                && exact_integer(value) <= exact_integer(bounds.upper())
        }
        (Value::Rational(value), ValueType::Rational(bounds)) => {
            let numerator = exact_integer(value.numerator());
            let denominator = exact_integer(value.denominator());
            exact_integer(bounds.numerator().lower()) <= numerator
                && numerator <= exact_integer(bounds.numerator().upper())
                && exact_integer(bounds.denominator().lower()) <= denominator
                && denominator <= exact_integer(bounds.denominator().upper())
        }
        (Value::Decimal(value), ValueType::Decimal(bounds)) => {
            let representation = value.representation();
            let mut coefficient = exact_integer(representation.coefficient());
            let mut scale = representation.scale();
            while scale > 0 && coefficient % 10 == 0 {
                coefficient /= 10;
                scale -= 1;
            }
            let member_scale = scale.max(bounds.min_scale());
            if member_scale > bounds.max_scale() {
                return false;
            }
            coefficient *= 10_i128.pow(member_scale - scale);
            exact_integer(bounds.lower()) <= coefficient
                && coefficient <= exact_integer(bounds.upper())
        }
        (Value::Collection(value), ValueType::Collection(checked)) => {
            let count = u64::try_from(value.elements().len()).expect("bounded collection length");
            value.collection_type().kind() == checked.kind()
                && checked
                    .bound()
                    .is_none_or(|bound| bound.minimum() <= count && count <= bound.maximum())
                && value
                    .elements()
                    .iter()
                    .all(|member| inside(member, checked.element()))
        }
        // These generator families carry their nominal/payload type in the
        // returned value; the public predicate compares that retained type.
        (Value::Boolean(_), ValueType::Boolean)
        | (Value::Text(_), ValueType::Text(_))
        | (Value::Option(_), ValueType::Option(_))
        | (Value::Composite(_), ValueType::Composite(_)) => checked.admits(value),
        _ => false,
    }
}

pub(super) fn family(domain: &Type) -> &'static str {
    match domain {
        Type::Integer | Type::BoundedInt { .. } => "integer",
        Type::Rational => "rational",
        Type::Decimal { .. } => "decimal",
        Type::Collection {
            kind: CollectionKind::Sequence,
            ..
        } => "sequence",
        Type::Collection {
            kind: CollectionKind::Set,
            ..
        } => "set",
        Type::Collection {
            kind: CollectionKind::Bag,
            ..
        } => "bag",
        Type::Boolean => "boolean",
        Type::Text => "text",
        Type::Option(_) => "option",
        Type::Record(_) => "record",
        Type::Tuple(_) => "tuple",
    }
}
