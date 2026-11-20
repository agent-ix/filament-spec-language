// SPDX-License-Identifier: AGPL-3.0-or-later
//! Generated source properties over the real parser, checker and evaluator.
//! Rejected source never supplies an evaluator package. Over-bound `count<N>`
//! remains outside the admitted generator pending QSL-2003.

use std::collections::BTreeMap;

use filament_eval::FamilyOutcome;
use filament_exact::{FieldValue, Meter, NodeKey, Outcome, Refusal, Value, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_bench::prop::{self, near_miss, value};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckRefusal, PackageDeclarations};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use proptest::prelude::*;
use proptest::test_runner::{TestCaseResult, TestError, TestRunner};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Reference {
    Integer(i128),
    Boolean(bool),
    Rational(i128, i128),
    Decimal(i128, u32),
    Text(String),
    Option(Option<Box<Reference>>),
    Composite(&'static str, Vec<Reference>),
    Collection(value::CollectionKind, Vec<Reference>),
}

fn rational(numerator: i128, denominator: i128) -> Reference {
    let (mut a, mut b) = (numerator.abs(), denominator.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    let divisor = a.max(1);
    let sign = if denominator < 0 { -1 } else { 1 };
    Reference::Rational(numerator / divisor * sign, denominator / divisor * sign)
}

fn decimal(mut coefficient: i128, mut scale: u32) -> Reference {
    while scale > 0 && coefficient % 10 == 0 {
        coefficient /= 10;
        scale -= 1;
    }
    Reference::Decimal(coefficient, scale)
}

fn reference(expression: &value::Expr, environment: &BTreeMap<String, Reference>) -> Reference {
    use value::Expr;
    match expression {
        Expr::Parameter(name) => environment[name].clone(),
        Expr::Integer(number) => Reference::Integer(i128::from(*number)),
        Expr::Boolean(truth) => Reference::Boolean(*truth),
        Expr::Text(text) => Reference::Text(text.clone()),
        Expr::Rational {
            numerator,
            denominator,
        } => rational(i128::from(*numerator), i128::from(*denominator)),
        Expr::Decimal { coefficient, scale } => {
            decimal(i128::from(*coefficient), u32::from(*scale))
        }
        Expr::OptionSome(member) => {
            Reference::Option(Some(Box::new(reference(member, environment))))
        }
        Expr::Let {
            name, value, body, ..
        } => {
            let mut local = environment.clone();
            local.insert(name.clone(), reference(value, environment));
            reference(body, &local)
        }
        Expr::Record(member) => {
            Reference::Composite("RecordValue", vec![reference(member, environment)])
        }
        Expr::Tuple(member, flag) => Reference::Composite(
            "TupleValue",
            vec![reference(member, environment), Reference::Boolean(*flag)],
        ),
        Expr::Collection { kind, members } => {
            let mut values = Vec::new();
            for member in members {
                let value = reference(member, environment);
                if *kind != value::CollectionKind::Set || !values.contains(&value) {
                    values.push(value);
                }
            }
            Reference::Collection(*kind, values)
        }
        Expr::Add(a, b) | Expr::Subtract(a, b) | Expr::Multiply(a, b) | Expr::Less(a, b) => {
            let (Reference::Integer(a), Reference::Integer(b)) =
                (reference(a, environment), reference(b, environment))
            else {
                panic!("integer generator operands");
            };
            match expression {
                Expr::Add(_, _) => Reference::Integer(a + b),
                Expr::Subtract(_, _) => Reference::Integer(a - b),
                Expr::Multiply(_, _) => Reference::Integer(a * b),
                Expr::Less(_, _) => Reference::Boolean(a < b),
                _ => unreachable!("the outer match selected an integer operator"),
            }
        }
        Expr::And(a, b) | Expr::Or(a, b) => {
            let (Reference::Boolean(a), Reference::Boolean(b)) =
                (reference(a, environment), reference(b, environment))
            else {
                panic!("Boolean generator operands");
            };
            Reference::Boolean(if matches!(expression, Expr::And(_, _)) {
                a && b
            } else {
                a || b
            })
        }
        Expr::Not(operand) => {
            let Reference::Boolean(value) = reference(operand, environment) else {
                panic!("Boolean generator operand");
            };
            Reference::Boolean(!value)
        }
        Expr::Equal(a, b) => {
            Reference::Boolean(reference(a, environment) == reference(b, environment))
        }
        Expr::ForAll {
            binder,
            collection,
            predicate,
        } => {
            let Reference::Collection(_, members) = reference(collection, environment) else {
                panic!("collection generator operand");
            };
            Reference::Boolean(members.into_iter().all(|member| {
                let mut local = environment.clone();
                local.insert(binder.clone(), member);
                reference(predicate, &local) == Reference::Boolean(true)
            }))
        }
        Expr::If { condition, yes, no } => match reference(condition, environment) {
            Reference::Boolean(true) => reference(yes, environment),
            Reference::Boolean(false) => reference(no, environment),
            other => panic!("Boolean condition, got {other:?}"),
        },
    }
}

// Assignments are independent of the expression generator. Typed let bindings
// preserve the declared element domains when constructors appear as arguments.
fn assignment(ty: &value::Type, seed: i16) -> value::Expr {
    use value::{Expr, Type};
    match ty {
        Type::Integer => Expr::Integer(i64::from(seed)),
        Type::BoundedInt { lower, upper } => {
            Expr::Integer(lower + i64::from(seed).rem_euclid(upper - lower + 1))
        }
        Type::Boolean => Expr::Boolean(seed.rem_euclid(2) == 0),
        Type::Rational => Expr::Rational {
            numerator: i64::from(seed % 20),
            denominator: u8::try_from(seed.rem_euclid(9) + 1).unwrap(),
        },
        Type::Decimal { .. } => Expr::Decimal {
            coefficient: i64::from(seed % 20),
            scale: u8::try_from(seed.rem_euclid(4)).unwrap(),
        },
        Type::Text => Expr::Text(
            ["", "abc", "é", "Ab", "🙂"]
                .get(usize::try_from(seed.rem_euclid(5)).unwrap())
                .unwrap()
                .to_string(),
        ),
        Type::Option(member) => Expr::Let {
            name: "assigned_member".into(),
            ty: (**member).clone(),
            value: Box::new(assignment(member, seed)),
            body: Box::new(Expr::OptionSome(Box::new(Expr::Parameter(
                "assigned_member".into(),
            )))),
        },
        Type::Record(member) => Expr::Record(Box::new(assignment(member, seed))),
        Type::Tuple(member) => {
            Expr::Tuple(Box::new(assignment(member, seed)), seed.rem_euclid(2) == 0)
        }
        Type::Collection { kind, member } => Expr::Collection {
            kind: *kind,
            members: (0..seed.rem_euclid(4))
                .map(|index| assignment(member, seed + index))
                .collect(),
        },
    }
}

#[derive(Debug)]
enum CompileFailure {
    Source(String),
    Check(Vec<CheckRefusal>),
}

impl std::fmt::Display for CompileFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(detail) => formatter.write_str(detail),
            Self::Check(refusals) => write!(formatter, "check: {refusals:?}"),
        }
    }
}

struct Compiled {
    package: CheckedPackage,
    composite_keys: BTreeMap<NodeKey, &'static str>,
}

fn compile(source: &str) -> Result<Compiled, CompileFailure> {
    let parsed = fsl_bench::parse::parse(source)
        .map_err(|error| CompileFailure::Source(format!("parse: {error:?}")))?;
    if !parsed.is_admissible() {
        return Err(CompileFailure::Source(format!(
            "parse: {:?}",
            parsed.diagnostics()
        )));
    }
    let unit = fsl_forms::build_unit(&parsed)
        .map_err(|error| CompileFailure::Source(format!("forms: {error:?}")))?;
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "value-properties".into(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .map_err(|error| CompileFailure::Source(format!("assembly: {error:?}")))?;
    let mut composite_keys = BTreeMap::new();
    for name in ["RecordValue", "TupleValue"] {
        if let Some(ValueType::Composite(key)) = declarations.types.named(name) {
            composite_keys.insert(key.clone(), name);
        }
    }
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .map(|graph| Compiled {
            package: CheckedPackage::link(graph),
            composite_keys,
        })
        .map_err(CompileFailure::Check)
}

fn call_named(package: &Compiled, name: &str) -> Outcome<Value> {
    let evaluation = package
        .package
        .call(
            &QualifiedName::unqualified(name).unwrap(),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(fsl_bench::world::UNLIMITED),
        )
        .expect("an admitted generated function call");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("kernel value outcome, got {other:?}"),
    }
}

fn call(package: &Compiled) -> Outcome<Value> {
    call_named(package, "invoke")
}

fn actual(value: &Value, compiled: &Compiled) -> Reference {
    match value {
        Value::Integer(number) => Reference::Integer(number.to_string().parse().unwrap()),
        Value::Boolean(truth) => Reference::Boolean(*truth),
        Value::Rational(number) => rational(
            number.numerator().to_string().parse().unwrap(),
            number.denominator().to_string().parse().unwrap(),
        ),
        Value::Decimal(number) => Reference::Decimal(
            number
                .representation()
                .coefficient()
                .to_string()
                .parse()
                .unwrap(),
            number.representation().scale(),
        ),
        Value::Text(text) => Reference::Text(text.payload().as_str().into()),
        Value::Option(option) => Reference::Option(
            option
                .payload()
                .map(|member| Box::new(actual(member, compiled))),
        ),
        Value::Composite(composite) => Reference::Composite(
            compiled.composite_keys[composite.declaration()],
            composite
                .slots()
                .iter()
                .map(|slot| match slot {
                    FieldValue::Present(value) => actual(value, compiled),
                    other => panic!("generated present field, got {other:?}"),
                })
                .collect(),
        ),
        Value::Collection(collection) => {
            let kind = match collection.collection_type().kind() {
                filament_exact::CollectionKind::Sequence => value::CollectionKind::Sequence,
                filament_exact::CollectionKind::Set => value::CollectionKind::Set,
                filament_exact::CollectionKind::Bag => value::CollectionKind::Bag,
                other => panic!("generated collection kind, got {other:?}"),
            };
            Reference::Collection(
                kind,
                collection
                    .elements()
                    .iter()
                    .map(|value| actual(value, compiled))
                    .collect(),
            )
        }
        other => panic!("generated value family, got {other:?}"),
    }
}

fn agree(expected: &Reference, actual: &Reference) -> bool {
    match (expected, actual) {
        (Reference::Collection(kind, left), Reference::Collection(other_kind, right)) => {
            if kind != other_kind {
                return false;
            }
            if *kind == value::CollectionKind::Sequence {
                return left == right;
            }
            let mut remaining = right.clone();
            for member in left {
                let Some(position) = remaining
                    .iter()
                    .position(|candidate| agree(member, candidate))
                else {
                    return false;
                };
                remaining.remove(position);
            }
            remaining.is_empty()
        }
        _ => expected == actual,
    }
}

fn invocation(input: &value::TypedExpression, seed: i16) -> (String, Reference) {
    let mut environment = BTreeMap::new();
    let mut locals = String::new();
    let mut arguments = Vec::new();
    for (index, (name, ty)) in input.parameters.iter().enumerate() {
        let argument = assignment(ty, seed + i16::try_from(index).unwrap());
        environment.insert(name.clone(), reference(&argument, &BTreeMap::new()));
        locals.push_str(&format!(
            "let argument{index}: {} = ({}) in ",
            ty.source().0,
            argument.source()
        ));
        arguments.push(format!("argument{index}"));
    }
    let source = format!(
        "{}function invoke using v(): {} pure {{ {locals}f({}) }}\n",
        input.source(),
        input.result.source().0,
        arguments.join(", ")
    );
    (source, reference(&input.expression, &environment))
}

fn evaluate_agreement(source: &str, expected: &Reference) -> TestCaseResult {
    let package =
        compile(source).map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
    let result = call(&package);
    let Outcome::Completed(value) = result else {
        return Err(TestCaseError::fail(format!(
            "expected {expected:?}, got {result:?}\n{source}"
        )));
    };
    let observed = actual(&value, &package);
    prop_assert!(
        agree(expected, &observed),
        "reference {expected:?}, evaluator {observed:?}\n{source}"
    );
    Ok(())
}

const ROUNDING_MODES: [&str; 6] = [
    "exact",
    "toward-zero",
    "toward-positive",
    "toward-negative",
    "nearest-even",
    "nearest-away",
];

fn rounded(numerator: i128, denominator: i128, mode: usize) -> Option<Reference> {
    let scaled = numerator * 100;
    let quotient = scaled / denominator;
    let remainder = scaled % denominator;
    if remainder == 0 {
        return Some(decimal(quotient, 2));
    }
    let sign = scaled.signum();
    let correction = match mode {
        0 => return None,
        1 => 0,
        2 => i128::from(sign > 0),
        3 => -i128::from(sign < 0),
        4 => {
            if remainder.abs() * 2 > denominator
                || (remainder.abs() * 2 == denominator && quotient % 2 != 0)
            {
                sign
            } else {
                0
            }
        }
        5 => {
            if remainder.abs() * 2 >= denominator {
                sign
            } else {
                0
            }
        }
        _ => unreachable!("rounding strategy domain"),
    };
    Some(decimal(quotient + correction, 2))
}

fn decimal_source(numerator: i16, denominator: u8, mode: usize) -> String {
    let target = format!("Decimal[-10000, 10000; 2, 2; {}]", ROUNDING_MODES[mode]);
    format!("{}function f using v(q: Rational[-20, 20; 1, 9]): {target} pure {{ convert<{target}>(q) }}\nfunction invoke using v(): {target} pure {{ f(rational({numerator}, {denominator})) }}\n", value::VALUE_HEADER)
}

#[derive(Clone, Debug)]
enum AgreementCase {
    Generated(value::TypedExpression, i16),
    Decimal(i16, u8, usize),
    Rational(i16, u8, i16, u8, u8),
    Text(u8, u8, u8),
}

fn agreement_cases() -> BoxedStrategy<AgreementCase> {
    prop_oneof![
        6 => (value::expressions(), -100i16..=100).prop_map(|(input, seed)| AgreementCase::Generated(input, seed)),
        2 => (-20i16..=20, 1u8..=9, 0usize..6).prop_map(|(n, mut d, mode)| {
            if mode == 0 { d = [1, 2, 4, 5][usize::from(d % 4)]; }
            AgreementCase::Decimal(n, d, mode)
        }),
        1 => (-5i16..=5, 1u8..=5, 1i16..=5, 1u8..=5, 0u8..4)
            .prop_map(|(a, b, c, d, op)| AgreementCase::Rational(a, b, c, d, op)),
        1 => (0u8..6, 0u8..6, 0u8..4).prop_map(|(a, b, op)| AgreementCase::Text(a, b, op)),
    ].boxed()
}

fn agreement_case(case: &AgreementCase) -> (String, Reference) {
    match case {
        AgreementCase::Generated(input, seed) => invocation(input, *seed),
        AgreementCase::Decimal(n, d, mode) => (
            decimal_source(*n, *d, *mode),
            rounded(i128::from(*n), i128::from(*d), *mode).unwrap(),
        ),
        AgreementCase::Rational(a, b, c, d, op) => {
            let (a, b, c, d) = (
                i128::from(*a),
                i128::from(*b),
                i128::from(*c),
                i128::from(*d),
            );
            let (operator, expected) = match op {
                0 => ("+", rational(a * d + c * b, b * d)),
                1 => ("-", rational(a * d - c * b, b * d)),
                2 => ("*", rational(a * c, b * d)),
                3 => ("/", rational(a * d, b * c)),
                _ => unreachable!("rational strategy domain"),
            };
            (format!("{}function invoke using v(): Rational[-250, 250; 1, 25] pure {{ rational({a}, {b}) {operator} rational({c}, {d}) }}\n", value::VALUE_HEADER), expected)
        }
        AgreementCase::Text(a, b, op) => {
            let texts = ["", "abc", "ab", "é", "e\u{301}", "🙂"];
            let (a, b) = (texts[usize::from(*a)], texts[usize::from(*b)]);
            let a_source = format!("Text({})", serde_json::to_string(a).unwrap());
            let b_source = format!("Text({})", serde_json::to_string(b).unwrap());
            let (result, expression, expected) = match op {
                0 => (
                    "Integer",
                    format!("size({a_source})"),
                    Reference::Integer(i128::try_from(a.chars().count()).unwrap()),
                ),
                1 => (
                    "Boolean",
                    format!("{a_source} = {b_source}"),
                    Reference::Boolean(a == b),
                ),
                2 => (
                    "Boolean",
                    format!("{a_source} < {b_source}"),
                    Reference::Boolean(a < b),
                ),
                3 => (
                    "Boolean",
                    format!("{a_source} != {b_source}"),
                    Reference::Boolean(a != b),
                ),
                _ => unreachable!("text strategy domain"),
            };
            (
                format!(
                    "{}function invoke using v(): {result} pure {{ {expression} }}\n",
                    value::VALUE_HEADER
                ),
                expected,
            )
        }
    }
}

fn tree(parsed: &fsl_cst::ParsedSource) -> Vec<(fsl_cst::Production, Vec<usize>)> {
    parsed
        .cst()
        .nodes()
        .iter()
        .map(|node| {
            (
                node.production(),
                node.children()
                    .iter()
                    .filter_map(|child| match child {
                        fsl_cst::CstElement::Node(index) => Some(*index),
                        fsl_cst::CstElement::Token(_) => None,
                    })
                    .collect(),
            )
        })
        .collect()
}

/// Provenance: QSL-2055 P1. Production shape and token spelling survive formatting.
#[ix_trace_rs::trace("FR-003-AC-1", "FR-003-AC-3")]
#[test]
fn p1_format_round_trip_keeps_the_tree() {
    prop::run(
        "p1_format_round_trip_keeps_the_tree",
        prop::config(1000),
        &value::expressions(),
        |input| {
            let source = input.source();
            let parsed = fsl_bench::parse::parse(&source).unwrap();
            let formatted = fsl_cst::format::format(&parsed)
                .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
            let reparsed = fsl_bench::parse::parse(&formatted).unwrap();
            prop_assert!(
                reparsed.is_admissible(),
                "{:?}\n{formatted}",
                reparsed.diagnostics()
            );
            prop_assert_eq!(tree(&parsed), tree(&reparsed), "{}", source);
            let tokens = |parsed: &fsl_cst::ParsedSource| {
                parsed
                    .cst()
                    .tokens()
                    .iter()
                    .filter(|token| !token.spelling().iter().all(u8::is_ascii_whitespace))
                    .map(|token| (token.kind(), token.spelling().to_vec()))
                    .collect::<Vec<_>>()
            };
            prop_assert_eq!(tokens(&parsed), tokens(&reparsed), "{}", source);
            prop_assert_eq!(fsl_cst::format::format(&reparsed).unwrap(), formatted);
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2055 P2. Every sampled source reaches the Linked checker.
#[test]
fn p2_generated_well_typed_expressions_check() {
    prop::run(
        "p2_generated_well_typed_expressions_check",
        prop::config(1000),
        &value::expressions(),
        |input| {
            let source = input.source();
            compile(&source)
                .map(|_| ())
                .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))
        },
    )
    .unwrap();
}

/// Provenance: QSL-2055 P3. Independent exact arithmetic and random typed assignments.
#[ix_trace_rs::trace("FSpec-FR-140-AC-7")]
#[test]
fn p3_evaluator_matches_the_exact_reference() {
    prop::run(
        "p3_evaluator_matches_the_exact_reference",
        prop::config(1000),
        &agreement_cases(),
        |case| {
            let (source, expected) = agreement_case(&case);
            evaluate_agreement(&source, &expected)
        },
    )
    .unwrap();
}

fn rejected_near_miss(input: &near_miss::NearMiss) -> TestCaseResult {
    use fsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
    let identity = fsl_foundation::SourceIdentity::new("agent-ix", "value-properties");
    let cancel = filament_exact::Cancel::new();
    let limits = SpineLimits::default();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "generated.native",
            bytes: input.source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .unwrap()
    .into_value();
    let models = spine::select(&parsed, &BTreeMap::new(), limits.model, &cancel)
        .unwrap()
        .into_value();
    let failure = spine::check(
        &parsed,
        "value-properties".into(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect_err("rejected source cannot supply a checked evaluator package");
    let refusal = failure
        .into_refused()
        .map_err(|stopped| TestCaseError::fail(format!("a typed refusal, got {stopped:?}")))?;
    prop_assert_eq!(refusal.code(), input.code, "{}", input.source);
    prop_assert_eq!(refusal.cause(), Some(input.cause), "{}", input.source);
    let region = refusal
        .region()
        .ok_or_else(|| TestCaseError::fail("missing source region"))?;
    prop_assert_eq!(
        region.start(),
        u64::try_from(input.expression_span.start).unwrap()
    );
    prop_assert_eq!(
        region.end(),
        u64::try_from(input.expression_span.end).unwrap()
    );
    Ok(())
}

/// Provenance: QSL-2055 P4. Exact conversion and bound membership determine definedness.
#[ix_trace_rs::trace("FSpec-FR-140-AC-7")]
#[test]
fn p4_refusals_agree_with_the_reference() {
    prop::run("p4_refusals_agree_with_the_reference", prop::config(1000), &(-20i16..=20, 1u8..=9, any::<bool>()), |(number, denominator, decimal_case)| {
        let (source, expected, code, cause) = if decimal_case {
            (decimal_source(number, denominator, 0), rounded(i128::from(number), i128::from(denominator), 0), "inexact_decimal", "nonzero-discarded-digit")
        } else {
            let source = format!("{}function f using v(x: Integer): Int[0, 3] pure {{ convert<Int[0, 3]>(x) }}\nfunction invoke using v(): Int[0, 3] pure {{ f({number}) }}\n", value::VALUE_HEADER);
            (source, (0..=3).contains(&number).then_some(Reference::Integer(i128::from(number))), "undefined_expression", "outside-domain")
        };
        let package = compile(&source).map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
        let result = call(&package);
        match (expected, result) {
            (Some(expected), Outcome::Completed(value)) => prop_assert!(agree(&expected, &actual(&value, &package)), "{source}"),
            (None, Outcome::Refused(refusal)) => {
                if decimal_case {
                    prop_assert_eq!(refusal.code(), Some(code), "{}", source);
                    prop_assert_eq!(refusal.cause(), Some(cause), "{}", source);
                } else {
                    prop_assert!(matches!(refusal, Refusal::IntegerOutOfDomain { .. }), "{}: {:?}", source, refusal);
                }
            }
            (expected, result) => return Err(TestCaseError::fail(format!("reference {expected:?}, evaluator {result:?}\n{source}"))),
        }
        Ok(())
    }).unwrap();
}

/// Provenance: QSL-2055 P4 negative. No checked package exists for an invalid source.
#[test]
fn near_miss_is_never_evaluated_to_a_value() {
    prop::run(
        "near_miss_is_never_evaluated_to_a_value",
        prop::config(1000),
        &near_miss::near_misses(),
        |input| rejected_near_miss(&input),
    )
    .unwrap();
}

fn aggregate_source(
    kind: value::CollectionKind,
    values: &[i64],
    operation: u8,
) -> (String, Reference) {
    let (kind_name, constructor) = match kind {
        value::CollectionKind::Set => ("Set", "set"),
        value::CollectionKind::Bag => ("Bag", "bag"),
        value::CollectionKind::Sequence => unreachable!("unordered aggregate strategy"),
    };
    let mut members = values.to_vec();
    if kind == value::CollectionKind::Set {
        members.sort_unstable();
        members.dedup();
    }
    let (result, expression, expected) = match operation {
        0 => (
            "Integer",
            "sum<Total>(x in xs: x)",
            Reference::Integer(members.iter().map(|x| i128::from(*x)).sum()),
        ),
        1 => (
            "Integer",
            "count<Count>(x in xs: x >= 0)",
            Reference::Integer(
                i128::try_from(members.iter().filter(|x| **x >= 0).count()).unwrap(),
            ),
        ),
        2 => (
            "Integer",
            "reduce<Total>(a, x in xs: if a < x then a else x)",
            Reference::Integer(i128::from(*members.iter().min().unwrap())),
        ),
        3 => (
            "Integer",
            "reduce<Total>(a, x in xs: if a > x then a else x)",
            Reference::Integer(i128::from(*members.iter().max().unwrap())),
        ),
        4 => (
            "Boolean",
            "exists(x in xs: x >= 0)",
            Reference::Boolean(members.iter().any(|x| *x >= 0)),
        ),
        5 => (
            "Boolean",
            "forall(x in xs: x >= 0)",
            Reference::Boolean(members.iter().all(|x| *x >= 0)),
        ),
        _ => unreachable!("aggregate strategy domain"),
    };
    let mut source = format!("{}type Total = Integer;\ntype Count = Int[0, 4];\nfunction f using v(xs: {kind_name}<Int[-5, 5]>[1, 4]): {result} pure {{ {expression} }}\n", value::VALUE_HEADER);
    for (name, permutation) in [
        ("invoke", values.to_vec()),
        ("permuted", values.iter().rev().copied().collect()),
    ] {
        let values = permutation
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        source.push_str(&format!("function {name} using v(): {result} pure {{ let xs: {kind_name}<Int[-5, 5]>[1, 4] = {constructor}[{values}] in f(xs) }}\n"));
    }
    (source, expected)
}

/// Provenance: QSL-2055 P5. Independent folds preserve Set uniqueness and Bag multiplicity.
#[test]
fn p5_aggregates_equal_the_reference_fold_in_any_order() {
    let strategy = (
        any::<bool>(),
        proptest::collection::vec(-5i64..=5, 1..=4),
        0u8..6,
    );
    prop::run(
        "p5_aggregates_equal_the_reference_fold_in_any_order",
        prop::config(1000),
        &strategy,
        |(set, values, operation)| {
            let kind = if set {
                value::CollectionKind::Set
            } else {
                value::CollectionKind::Bag
            };
            let (source, expected) = aggregate_source(kind, &values, operation);
            let package = compile(&source)
                .map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
            for name in ["invoke", "permuted"] {
                let outcome = call_named(&package, name);
                let Outcome::Completed(value) = outcome else {
                    return Err(TestCaseError::fail(format!(
                        "expected {expected:?}, got {outcome:?}\n{source}"
                    )));
                };
                prop_assert!(
                    agree(&expected, &actual(&value, &package)),
                    "{name}: reference {expected:?}, evaluator {value:?}\n{source}"
                );
            }
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2055 mutation control. A wrong reference must expose a real evaluator mismatch.
#[test]
fn reference_mutation_is_caught() {
    let directory = tempfile::tempdir().unwrap();
    let mut config = prop::config(1000);
    config.source_file = Some("value_properties_reference_mutation");
    config.failure_persistence = Some(Box::new(prop::SeedPersistence::new(directory.path())));
    let result = TestRunner::new(config).run(&(1i16..=20, 1i64..=20), |(parameter, literal)| {
        let input = value::TypedExpression {
            parameters: vec![("x".into(), value::Type::Integer)],
            result: value::Type::Integer,
            expression: value::Expr::Add(
                Box::new(value::Expr::Parameter("x".into())),
                Box::new(value::Expr::Integer(literal)),
            ),
        };
        let (source, _) = invocation(&input, parameter);
        evaluate_agreement(
            &source,
            &Reference::Integer(i128::from(parameter) - i128::from(literal)),
        )
    });
    let Err(TestError::Fail(reason, (parameter, literal))) = result else {
        panic!("the deliberately wrong reference must fail and shrink: {result:?}");
    };
    assert_eq!((parameter, literal), (1, 1));
    assert!(
        reason
            .to_string()
            .contains("reference Integer(0), evaluator Integer(2)"),
        "{reason}"
    );
    assert!(
        reason.to_string().contains("x + 1"),
        "shrunk one-operator source: {reason}"
    );
    assert!(
        std::fs::read_dir(directory.path())
            .unwrap()
            .next()
            .is_some(),
        "a replay seed was persisted"
    );
}
