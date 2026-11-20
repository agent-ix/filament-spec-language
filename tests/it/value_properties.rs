// SPDX-License-Identifier: AGPL-3.0-or-later
//! Generated source properties over the real parser, checker and evaluator.
//! Rejected source never supplies an evaluator package. Over-bound `count<N>`
//! remains outside the admitted generator pending QSL-2003.
//! P2 and near misses use Linked mode. P4's deliberately partial conversion
//! functions use public Kernel admission before runtime refusal evaluation.
//! Scalar extrema require nonempty collections in the bounded domain [-5,5].
//! Sequences use folds; unordered extrema use supported exists/if composition,
//! without claiming that arbitrary unordered fold steps are eligible.

use std::collections::BTreeMap;

use filament_eval::FamilyOutcome;
use filament_exact::{FieldValue, Meter, Outcome, Refusal, Undefined, Value, ValueType};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_bench::prop::{self, near_miss, value};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckCause, CheckRefusal, Obligation, PackageDeclarations};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use proptest::prelude::{any, BoxedStrategy, Strategy, TestCaseError};
use proptest::test_runner::{TestCaseResult, TestError, TestRunner};
use proptest::{prop_assert, prop_assert_eq, prop_oneof};

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

fn decimal(coefficient: i128, scale: u32) -> Reference {
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
}

fn compile(source: &str) -> Result<Compiled, CompileFailure> {
    compile_mode(source, CheckMode::Linked)
}

fn compile_mode(source: &str, mode: CheckMode) -> Result<Compiled, CompileFailure> {
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
    declarations
        .check(
            mode,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .map(|graph| Compiled {
            package: CheckedPackage::link(graph),
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
        FamilyOutcome::Evaluated(outcome) => {
            if let Outcome::Completed(value) = &outcome {
                let signature = package.package.graph().callable(name).unwrap();
                assert_result_type(value, signature.result);
            }
            outcome
        }
        other => panic!("kernel value outcome, got {other:?}"),
    }
}

fn call(package: &Compiled) -> Outcome<Value> {
    call_named(package, "invoke")
}

fn assert_result_type(value: &Value, expected: &ValueType) {
    match (value, expected) {
        (Value::Integer(_), ValueType::Integer | ValueType::Int(_))
        | (Value::Boolean(_), ValueType::Boolean)
        | (Value::Rational(_), ValueType::Rational(_))
        | (Value::Decimal(_), ValueType::Decimal(_))
        | (Value::Text(_), ValueType::Text(_)) => {}
        (Value::Composite(composite), ValueType::Composite(key)) => {
            assert_eq!(composite.declaration(), key, "nominal result identity");
        }
        (Value::Option(option), ValueType::Option(member)) => {
            assert_eq!(option.payload_type(), member.as_ref());
            if let Some(value) = option.payload() {
                assert_result_type(value, member);
            }
        }
        (Value::Collection(collection), ValueType::Collection(expected)) => {
            assert_eq!(collection.collection_type().kind(), expected.kind());
            assert_eq!(collection.collection_type().element(), expected.element());
            for value in collection.elements() {
                assert_result_type(value, expected.element());
            }
        }
        other => panic!("result family differs from its checked type: {other:?}"),
    }
}

fn actual(value: &Value) -> Reference {
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
        Value::Option(option) => {
            Reference::Option(option.payload().map(|member| Box::new(actual(member))))
        }
        Value::Composite(composite) => Reference::Composite(
            if composite.slots().len() == 1 {
                "RecordValue"
            } else {
                "TupleValue"
            },
            composite
                .slots()
                .iter()
                .map(|slot| match slot {
                    FieldValue::Present(value) => actual(value),
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
            Reference::Collection(kind, collection.elements().iter().map(actual).collect())
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
    let observed = actual(&value);
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
            (format!("{}function invoke using v(): Rational[-250, 250; 1, 25] pure {{ let left: Rational[-5, 5; 1, 5] = rational({a}, {b}) in let right: Rational[1, 5; 1, 5] = rational({c}, {d}) in left {operator} right }}\n", value::VALUE_HEADER), expected)
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

/// Provenance: QSL-2055 P4 boundary. Linked checking retains the range obligation.
#[test]
fn partial_conversion_keeps_the_linked_range_boundary() {
    let source = format!("{}function f using v(x: Integer): Int[0, 3] pure {{ convert<Int[0, 3]>(x) }}\nfunction invoke using v(): Int[0, 3] pure {{ f(0) }}\n", value::VALUE_HEADER);
    let refusals = match compile(&source) {
        Err(CompileFailure::Check(refusals)) => refusals,
        Err(other) => panic!("a checker range refusal, got {other:?}"),
        Ok(_) => panic!("Linked mode must retain the unproved range obligation"),
    };
    assert_eq!(refusals.len(), 1);
    let CheckCause::Unproved(Obligation::Range { required, .. }) = &refusals[0].cause else {
        panic!("a range obligation, got {:?}", refusals[0]);
    };
    assert_eq!(required.lower, Some(filament_exact::Integer::from(0_i64)));
    assert_eq!(required.upper, Some(filament_exact::Integer::from(3_i64)));
}

/// Provenance: QSL-2055 P4. Public Kernel evaluation observes partial operations.
#[ix_trace_rs::trace("FSpec-FR-140-AC-7")]
#[test]
fn p4_refusals_agree_with_the_reference() {
    prop::run("p4_refusals_agree_with_the_reference", prop::config(1000), &(-20i16..=20, -8i16..=8, 0u8..3), |(number, divisor, kind)| {
        let (source, expected) = match kind {
            0 => {
                let denominator = u8::try_from(divisor.unsigned_abs() + 1).unwrap();
                (decimal_source(number, denominator, 0), rounded(i128::from(number), i128::from(denominator), 0))
            }
            1 => {
                let source = format!("{}function f using v(x: Integer): Int[0, 3] pure {{ convert<Int[0, 3]>(x) }}\nfunction invoke using v(): Int[0, 3] pure {{ f({number}) }}\n", value::VALUE_HEADER);
                (source, (0..=3).contains(&number).then_some(Reference::Integer(i128::from(number))))
            }
            2 => {
                let source = format!("{}profile d = \"filament.value.integer-division.euclidean/v1\";\nfunction f using v(a: Integer, b: Integer): Integer pure {{ a div b }}\nfunction invoke using v(): Integer pure {{ f({number}, {divisor}) }}\n", value::VALUE_HEADER);
                let expected = (divisor != 0).then(|| {
                    let (a, b) = (i128::from(number), i128::from(divisor));
                    Reference::Integer(if a % b < 0 { a / b - b.signum() } else { a / b })
                });
                (source, expected)
            }
            _ => unreachable!("partial operation strategy domain"),
        };
        let package = compile_mode(&source, CheckMode::Kernel).map_err(|error| TestCaseError::fail(format!("{error:?}\n{source}")))?;
        let result = call(&package);
        match (expected, result) {
            (Some(expected), Outcome::Completed(value)) => prop_assert!(agree(&expected, &actual(&value)), "{source}"),
            (None, Outcome::Refused(refusal)) => {
                if kind == 0 {
                    prop_assert_eq!(refusal.code(), Some("inexact_decimal"), "{}", source);
                    prop_assert_eq!(refusal.cause(), Some("nonzero-discarded-digit"), "{}", source);
                } else {
                    prop_assert_eq!(kind, 1, "{}", source);
                    prop_assert!(matches!(refusal, Refusal::IntegerOutOfDomain { .. }), "{}: {:?}", source, refusal);
                }
            }
            (None, Outcome::Undefined(Undefined::DivisionByZero)) => prop_assert_eq!(kind, 2, "{}", source),
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

fn bounded_extremum(minimum: bool) -> String {
    let mut candidates: Vec<i64> = (-5..=5).collect();
    if !minimum {
        candidates.reverse();
    }
    let mut expression = candidates.pop().unwrap().to_string();
    for candidate in candidates.into_iter().rev() {
        expression =
            format!("if exists(x in xs: x = {candidate}) then {candidate} else ({expression})");
    }
    expression
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
    let (result, expression, expected): (&str, String, Reference) = match operation {
        0 => (
            "Integer",
            "sum<Total>(x in xs: x)".into(),
            Reference::Integer(members.iter().map(|x| i128::from(*x)).sum()),
        ),
        1 => (
            "Integer",
            "count<Count>(x in xs: x >= 0)".into(),
            Reference::Integer(
                i128::try_from(members.iter().filter(|x| **x >= 0).count()).unwrap(),
            ),
        ),
        2 => (
            "Integer",
            bounded_extremum(true),
            Reference::Integer(i128::from(*members.iter().min().unwrap())),
        ),
        3 => (
            "Integer",
            bounded_extremum(false),
            Reference::Integer(i128::from(*members.iter().max().unwrap())),
        ),
        4 => (
            "Boolean",
            "exists(x in xs: x >= 0)".into(),
            Reference::Boolean(members.iter().any(|x| *x >= 0)),
        ),
        5 => (
            "Boolean",
            "forall(x in xs: x >= 0)".into(),
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
    if operation == 2 || operation == 3 {
        let (comparison, identity) = if operation == 2 { ("<", 5) } else { (">", -5) };
        source.push_str(&format!("function sequence_fold using v(xs: Sequence<Int[-5, 5]>[1, 4]): Integer pure {{ fold<Total>(a, x in xs: if a {comparison} x then a else x, identity: {identity}) }}\n"));
        for (name, permutation) in [
            ("sequence_original", values.to_vec()),
            ("sequence_permuted", values.iter().rev().copied().collect()),
        ] {
            let values = permutation
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            source.push_str(&format!("function {name} using v(): Integer pure {{ let xs: Sequence<Int[-5, 5]>[1, 4] = sequence[{values}] in sequence_fold(xs) }}\n"));
        }
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
            let mut names = vec!["invoke", "permuted"];
            if operation == 2 || operation == 3 {
                names.extend(["sequence_original", "sequence_permuted"]);
            }
            for name in names {
                let outcome = call_named(&package, name);
                let Outcome::Completed(value) = outcome else {
                    return Err(TestCaseError::fail(format!(
                        "expected {expected:?}, got {outcome:?}\n{source}"
                    )));
                };
                prop_assert!(
                    agree(&expected, &actual(&value)),
                    "{name}: reference {expected:?}, evaluator {value:?}\n{source}"
                );
            }
            Ok(())
        },
    )
    .unwrap();
}

/// Provenance: QSL-2055 P5 boundary. Scalar extrema require nonempty source domains.
#[test]
fn empty_extrema_input_is_refused_before_evaluation() {
    for kind in [value::CollectionKind::Set, value::CollectionKind::Bag] {
        for operation in [2, 3] {
            let (source, _) = aggregate_source(kind, &[0], operation);
            let empty = source
                .replace("set[0]", "set[]")
                .replace("bag[0]", "bag[]")
                .replace("sequence[0]", "sequence[]");
            let refusals = match compile(&empty) {
                Err(CompileFailure::Check(refusals)) => refusals,
                Err(other) => panic!("a checker cardinality refusal, got {other:?}\n{empty}"),
                Ok(_) => panic!("empty input must not enter scalar extrema\n{empty}"),
            };
            assert!(!refusals.is_empty());
            assert!(
                refusals
                    .iter()
                    .all(|refusal| matches!(refusal.cause, CheckCause::Unproved(_))),
                "{refusals:?}\n{empty}"
            );
        }
    }
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
