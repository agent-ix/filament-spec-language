// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2056 P1 and P2: the checker's node types against the evaluator's values.
//!
//! A generated expression over `Int[..]`, `Rational`, `Decimal` and the three
//! collection kinds is checked as a standalone expression under
//! `CheckMode::Linked` with no expected type, so the checked node type is the
//! checker's own. The expression is evaluated at random members of its
//! parameter types. P1: a completed value lies inside the checked type. P2:
//! no call returns an internal fault (`runtime_invariant`) or panics, and a
//! non-completed outcome occurs only for an operation named in
//! [`RUN_TIME_ONLY`].
//!
//! An expression the checker refuses is discarded, and each family must still
//! be sampled often enough to mean something.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

use filament_eval::{CallFailure, FamilyOutcome};
use filament_exact::{
    CardinalityBound, CollectionKind, CollectionType, Decimal, DecimalType, Integer,
    IntegerInterval, Meter, Outcome, Rational, RationalDomain, RoundingMode, ScalarLimits, Value,
    ValueType,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::value::CheckedPackageEvaluation;
use fsl_forms::Expression;
use fsl_package::CheckedPackage;
use fsl_proptest_policy as prop;
use fsl_semantics::check::{CheckedExpression, PackageDeclarations};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

/// Operations whose result the checker leaves to run time, so that a refused
/// outcome of a checked expression is allowed for them. The checker's
/// obligations (`fsl_semantics::check::Obligation`) are nonzero, presence,
/// integer range, rational range and nonempty reduction: none covers a decimal
/// result, and none covers the member count of a collection literal. So a decimal
/// `+ - *` that leaves its domain or cannot be placed exactly, and a literal
/// with more members than its annotated bound, are refused by the evaluator
/// after a `Linked` check. Each entry names the one refusal the operation may
/// end in. Every other generated operation
/// has its obligation proved at check. A new unchecked operation is added here
/// on purpose, together with its row in [`OPERATIONS`].
const RUN_TIME_ONLY: &[(&str, &str)] = &[
    ("decimal-add", "DecimalOutOfDomain"),
    ("decimal-sub", "DecimalOutOfDomain"),
    ("decimal-mul", "DecimalOutOfDomain"),
    ("decimal-mul", "InexactDecimal"),
    // A collection literal's member count against the annotated bound.
    ("sequence", "CardinalityOutOfBound"),
    ("set", "CardinalityOutOfBound"),
    ("bag", "CardinalityOutOfBound"),
];

/// Every operation the generator can write, and whether it is run-time-only.
const OPERATIONS: &[(&str, bool)] = &[
    ("int-add", false),
    ("int-sub", false),
    ("int-mul", false),
    ("int-less-if", false),
    ("rational-add", false),
    ("rational-sub", false),
    ("rational-mul", false),
    ("decimal-add", true),
    ("decimal-sub", true),
    ("decimal-mul", true),
    ("sequence", true),
    ("set", true),
    ("bag", true),
];

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

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

// ------------------------------------------------------------ expressions

#[derive(Clone, Copy, Debug)]
enum Arith {
    Add,
    Sub,
    Mul,
}

impl Arith {
    fn symbol(self) -> &'static str {
        match self {
            Arith::Add => "+",
            Arith::Sub => "-",
            Arith::Mul => "*",
        }
    }
    fn name(self, family: &str) -> String {
        format!(
            "{family}-{}",
            match self {
                Arith::Add => "add",
                Arith::Sub => "sub",
                Arith::Mul => "mul",
            }
        )
    }
}

fn arith() -> BoxedStrategy<Arith> {
    prop_oneof![Just(Arith::Add), Just(Arith::Sub), Just(Arith::Mul)].boxed()
}

/// An integer expression over the parameters `a`, `b` and `c`.
#[derive(Clone, Debug)]
enum IntExpr {
    Param(usize),
    Lit(i64),
    Bin(Arith, Box<IntExpr>, Box<IntExpr>),
    If(Box<IntExpr>, Box<IntExpr>, Box<IntExpr>, Box<IntExpr>),
}

fn int_exprs() -> BoxedStrategy<IntExpr> {
    let leaf = prop_oneof![
        (0usize..3).prop_map(IntExpr::Param),
        (-4i64..=4).prop_map(IntExpr::Lit)
    ];
    leaf.prop_recursive(3, 12, 2, |inner| {
        prop_oneof![
            3 => (arith(), inner.clone(), inner.clone())
                .prop_map(|(op, l, r)| IntExpr::Bin(op, Box::new(l), Box::new(r))),
            1 => (inner.clone(), inner.clone(), inner.clone(), inner)
                .prop_map(|(x, y, t, e)| IntExpr::If(Box::new(x), Box::new(y), Box::new(t), Box::new(e))),
        ]
    })
    .boxed()
}

impl IntExpr {
    fn source(&self) -> String {
        match self {
            IntExpr::Param(i) => ["a", "b", "c"][*i].to_owned(),
            IntExpr::Lit(k) => format!("({k})"),
            IntExpr::Bin(op, l, r) => format!("({} {} {})", l.source(), op.symbol(), r.source()),
            IntExpr::If(x, y, t, e) => format!(
                "(if {} < {} then {} else {})",
                x.source(),
                y.source(),
                t.source(),
                e.source()
            ),
        }
    }
    fn operations(&self, out: &mut Vec<String>) {
        match self {
            IntExpr::Bin(op, l, r) => {
                out.push(op.name("int"));
                l.operations(out);
                r.operations(out);
            }
            IntExpr::If(x, y, t, e) => {
                out.push("int-less-if".into());
                for part in [x, y, t, e] {
                    part.operations(out);
                }
            }
            _ => {}
        }
    }
}

/// A left-nested chain `((base op leaf) op leaf)` over rational or decimal
/// leaves (a literal only as a right operand).
#[derive(Clone, Debug)]
struct Chain {
    base: usize,
    steps: Vec<(Arith, ChainLeaf)>,
}

#[derive(Clone, Debug)]
enum ChainLeaf {
    Param(usize),
    Lit(i64, u8),
}

fn chains() -> BoxedStrategy<Chain> {
    (
        0usize..3,
        proptest::collection::vec(
            (
                arith(),
                prop_oneof![
                    (0usize..3).prop_map(ChainLeaf::Param),
                    (-3i64..=3, 1u8..=3).prop_map(|(n, d)| ChainLeaf::Lit(n, d)),
                ],
            ),
            1..=3,
        ),
    )
        .prop_map(|(base, steps)| Chain { base, steps })
        .boxed()
}

impl Chain {
    fn source(&self, decimal: bool) -> String {
        let name = |i: usize| ["a", "b", "c"][i].to_owned();
        let mut text = name(self.base);
        for (op, leaf) in &self.steps {
            let leaf = match leaf {
                ChainLeaf::Param(i) => name(*i),
                ChainLeaf::Lit(n, d) if decimal => format!("decimal({n}, {})", d - 1),
                ChainLeaf::Lit(n, d) => format!("rational({n}, {d})"),
            };
            text = format!("({text} {} {leaf})", op.symbol());
        }
        text
    }
    fn operations(&self, family: &str, out: &mut Vec<String>) {
        out.extend(self.steps.iter().map(|(op, _)| op.name(family)));
    }
}

#[derive(Clone, Copy, Debug)]
enum Kind {
    Sequence,
    Set,
    Bag,
}

/// One generated case: a body, its parameter types and its arguments.
#[derive(Clone, Debug)]
struct Case {
    family: &'static str,
    body: String,
    operations: Vec<String>,
    parameters: Vec<(String, ValueType)>,
    arguments: Vec<Value>,
    /// The argument selectors, kept for the failure report.
    shown: String,
}

fn interval(lower: i64, upper: i64) -> IntegerInterval {
    IntegerInterval::new(Integer::from(lower), Integer::from(upper)).expect("ordered bounds")
}

fn rational_domain() -> ValueType {
    ValueType::Rational(RationalDomain::new(interval(-3, 3), interval(1, 4)).expect("domain"))
}

fn decimal_type() -> ValueType {
    ValueType::Decimal(
        DecimalType::new(
            Integer::from(-20_000_i64),
            Integer::from(20_000_i64),
            0,
            3,
            RoundingMode::Exact,
        )
        .expect("decimal type"),
    )
}

fn int_cases() -> BoxedStrategy<Case> {
    (
        int_exprs(),
        proptest::collection::vec((0i64..=5, 0i64..=5, any::<u8>()), 3),
    )
        .prop_map(|(expr, params)| {
            let mut operations = Vec::new();
            expr.operations(&mut operations);
            Case {
                family: "int",
                body: expr.source(),
                operations,
                parameters: params
                    .iter()
                    .enumerate()
                    .map(|(i, (lo, hi, _))| {
                        (
                            ["a", "b", "c"][i].to_owned(),
                            ValueType::Int(interval(-lo, *hi)),
                        )
                    })
                    .collect(),
                arguments: params
                    .iter()
                    .map(|(lo, hi, pick)| {
                        let width = lo + hi + 1;
                        Value::Integer(Integer::from(-lo + i64::from(*pick) % width))
                    })
                    .collect(),
                shown: format!("{params:?}"),
            }
        })
        .boxed()
}

fn rational_cases() -> BoxedStrategy<Case> {
    (
        chains(),
        proptest::collection::vec((-3i64..=3, 1i64..=4), 3),
    )
        .prop_map(|(chain, values)| {
            let mut operations = Vec::new();
            chain.operations("rational", &mut operations);
            Case {
                family: "rational",
                body: chain.source(false),
                operations,
                parameters: ["a", "b", "c"]
                    .iter()
                    .map(|n| ((*n).to_owned(), rational_domain()))
                    .collect(),
                arguments: values
                    .iter()
                    .map(|(n, d)| {
                        Value::Rational(
                            Rational::new(Integer::from(*n), Integer::from(*d)).expect("nonzero"),
                        )
                    })
                    .collect(),
                shown: format!("{values:?}"),
            }
        })
        .boxed()
}

fn decimal_cases() -> BoxedStrategy<Case> {
    (
        chains(),
        proptest::collection::vec((-20i64..=20, 0u32..=3), 3),
    )
        .prop_map(|(chain, values)| {
            let mut operations = Vec::new();
            chain.operations("decimal", &mut operations);
            Case {
                family: "decimal",
                body: chain.source(true),
                operations,
                parameters: ["a", "b", "c"]
                    .iter()
                    .map(|n| ((*n).to_owned(), decimal_type()))
                    .collect(),
                arguments: values
                    .iter()
                    .map(|(c, s)| Value::Decimal(Decimal::new(Integer::from(*c), *s)))
                    .collect(),
                shown: format!("{values:?}"),
            }
        })
        .boxed()
}

fn collection_cases() -> BoxedStrategy<Case> {
    (
        prop_oneof![Just(Kind::Sequence), Just(Kind::Set), Just(Kind::Bag)],
        proptest::collection::vec(int_exprs(), 0..=4),
        proptest::collection::vec((0i64..=5, 0i64..=5, any::<u8>()), 3),
        (1i64..=12, 0u64..=4),
    )
        .prop_map(|(kind, members, params, (element, size))| {
            let mut operations = vec![match kind {
                Kind::Sequence => "sequence",
                Kind::Set => "set",
                Kind::Bag => "bag",
            }
            .to_owned()];
            for member in &members {
                member.operations(&mut operations);
            }
            let constructor = match kind {
                Kind::Sequence => "sequence",
                Kind::Set => "set",
                Kind::Bag => "bag",
            };
            Case {
                family: "collection",
                body: format!(
                    "let xs: {}<Int[-{element}, {element}]>[0, {size}] = {constructor}[{}] in xs",
                    constructor[..1].to_uppercase() + &constructor[1..],
                    members
                        .iter()
                        .map(IntExpr::source)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                operations,
                parameters: params
                    .iter()
                    .enumerate()
                    .map(|(i, (lo, hi, _))| {
                        (
                            ["a", "b", "c"][i].to_owned(),
                            ValueType::Int(interval(-lo, *hi)),
                        )
                    })
                    .collect(),
                arguments: params
                    .iter()
                    .map(|(lo, hi, pick)| {
                        Value::Integer(Integer::from(-lo + i64::from(*pick) % (lo + hi + 1)))
                    })
                    .collect(),
                shown: format!("{params:?}"),
            }
        })
        .boxed()
}

fn cases() -> BoxedStrategy<Case> {
    prop_oneof![
        3 => int_cases(),
        2 => rational_cases(),
        2 => decimal_cases(),
        2 => collection_cases(),
    ]
    .boxed()
}

// ------------------------------------------------------------ the pipeline

/// A case, checked: the package, the checked expression and its node type.
struct Checked {
    package: CheckedPackage,
    expression: CheckedExpression,
    node_type: ValueType,
}

/// The declarations of one parsed function.
fn parsed_unit(function: &str) -> PackageDeclarations {
    let text = format!("{HEADER}{function}\n");
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "typed-value-differential"),
        "typed.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "typed-value-differential".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles")
}

/// The refusal text when the checker refuses the expression.
fn check(case: &Case) -> Result<Checked, String> {
    // The body is read from a function that is never checked; the package
    // that is checked holds a trivial function.
    let body: Expression = parsed_unit(&format!(
        "function g using v(): Boolean pure {{ {} }}",
        case.body
    ))
    .functions
    .remove(0)
    .body;
    let declarations = parsed_unit("function f using v(): Boolean pure { true }");
    let graph = declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .expect("a trivial package checks");
    let expression = graph
        .check_expression(
            case.parameters.clone(),
            &body,
            None,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .map_err(|refusal| format!("{refusal:?}"))?;
    let node_type = expression.value_type().clone();
    Ok(Checked {
        package: CheckedPackage::link(graph),
        expression,
        node_type,
    })
}

/// What one evaluation did.
enum Run {
    Completed(Value),
    NotCompleted(String),
    Failed(String),
}

fn run(case: &Case, checked: &Checked) -> Run {
    let result = checked.package.evaluate(
        &checked.expression,
        case.arguments.clone(),
        &ObjectEnvironment::default(),
        &mut Meter::new(UNLIMITED),
    );
    match result {
        Ok(evaluation) => match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => Run::Completed(value),
            other => Run::NotCompleted(format!("{other:?}")),
        },
        Err(CallFailure::Fault(fault)) => Run::Failed(format!("internal fault: {fault:?}")),
        Err(other) => Run::Failed(format!("call failure: {other:?}")),
    }
}

/// Whether `value` lies inside `ty`, from the declarations alone.
fn inside(value: &Value, ty: &ValueType) -> Result<(), String> {
    match (ty, value) {
        (ValueType::Integer, Value::Integer(_)) | (ValueType::Boolean, Value::Boolean(_)) => Ok(()),
        (ValueType::Int(range), Value::Integer(i)) if range.contains(i) => Ok(()),
        (ValueType::Rational(domain), Value::Rational(r)) if domain.contains(r) => Ok(()),
        (ValueType::Decimal(decimal), Value::Decimal(d)) => {
            let scale = d.representation().scale();
            if decimal.contains(d) && decimal.min_scale() <= scale && scale <= decimal.max_scale() {
                Ok(())
            } else {
                Err(format!(
                    "decimal {d:?} (scale {scale}) is outside {decimal:?}"
                ))
            }
        }
        (ValueType::Collection(collection), Value::Collection(c)) => {
            let count = u64::try_from(c.elements().len()).expect("small");
            if c.collection_type().kind() != collection.kind() {
                return Err(format!(
                    "kind {:?} against {:?}",
                    c.collection_type().kind(),
                    collection.kind()
                ));
            }
            if let Some(bound) = collection.bound() {
                if bound.violation(count).is_some() {
                    return Err(format!("{count} members outside {bound:?}"));
                }
            }
            c.elements()
                .iter()
                .try_for_each(|element| inside(element, collection.element()))
        }
        _ => Err(format!("{value:?} is outside {ty:?}")),
    }
}

fn describe(case: &Case, checked: &Checked) -> String {
    format!(
        "source: {}\nchecked type: {:?}\nparameters: {:?}\nselectors: {}\narguments: {:?}",
        case.body, checked.node_type, case.parameters, case.shown, case.arguments
    )
}

fn sampled() -> std::rc::Rc<RefCell<BTreeMap<&'static str, usize>>> {
    std::rc::Rc::default()
}

fn assert_sampled(seen: &BTreeMap<&'static str, usize>) {
    for family in ["int", "rational", "decimal", "collection"] {
        assert!(
            seen.get(family).copied().unwrap_or(0) >= 15,
            "family {family} is under-sampled: {seen:?}"
        );
    }
}

/// QSL-2056 P1 (FR-096-AC-23): a completed value lies inside the checked node type.
#[trace("TC-3207", "FR-096-AC-23")]
#[test]
fn p1_completed_values_lie_inside_the_checked_type() {
    let seen = sampled();
    let counter = seen.clone();
    prop::run(
        "p1_completed_values_lie_inside_the_checked_type",
        prop::config(256),
        &cases(),
        move |case| {
            let Ok(checked) = check(&case) else {
                return Ok(());
            };
            if let Run::Completed(value) = run(&case, &checked) {
                *counter.borrow_mut().entry(case.family).or_default() += 1;
                inside(&value, &checked.node_type).map_err(|why| {
                    TestCaseError::fail(format!(
                        "{why}\n{}\nvalue: {value:?}",
                        describe(&case, &checked)
                    ))
                })?;
            }
            Ok(())
        },
    )
    .unwrap();
    assert_sampled(&seen.borrow());
}

/// QSL-2056 P2 (FR-096-AC-23): no call returns an internal fault or panics, and only a
/// run-time-only operation may end in anything but a value.
#[trace("TC-3207", "FR-096-AC-23")]
#[test]
fn p2_no_completed_call_returns_runtime_invariant() {
    let seen = sampled();
    let counter = seen.clone();
    prop::run(
        "p2_no_completed_call_returns_runtime_invariant",
        prop::config(256),
        &cases(),
        move |case| {
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                check(&case).ok().map(|checked| {
                    let result = run(&case, &checked);
                    (describe(&case, &checked), result)
                })
            }));
            let Ok(checked) = outcome else {
                return Err(TestCaseError::fail(format!("panic\n{case:?}")));
            };
            let Some((shown, result)) = checked else {
                return Ok(());
            };
            *counter.borrow_mut().entry(case.family).or_default() += 1;
            match result {
                Run::Completed(_) => Ok(()),
                Run::Failed(why) => Err(TestCaseError::fail(format!("{why}\n{shown}"))),
                Run::NotCompleted(why) => {
                    let allowed = RUN_TIME_ONLY.iter().any(|(name, refusal)| {
                        case.operations.iter().any(|op| op == name)
                            && why.contains(&format!("Refused({refusal}"))
                    });
                    if allowed {
                        Ok(())
                    } else {
                        Err(TestCaseError::fail(format!(
                            "{why} that no run-time-only operation of the case may end in\n{shown}"
                        )))
                    }
                }
            }
        },
    )
    .unwrap();
    assert_sampled(&seen.borrow());
}

/// Negative: the list of run-time-only operations is exact, so an operation
/// that the checker leaves to run time must be added here on purpose.
#[test]
fn the_run_time_only_operations_are_exactly_the_listed_ones() {
    let flagged: Vec<&str> = OPERATIONS
        .iter()
        .filter(|(_, run_time)| *run_time)
        .map(|(name, _)| *name)
        .collect();
    let mut listed: Vec<&str> = RUN_TIME_ONLY.iter().map(|(name, _)| *name).collect();
    listed.dedup();
    assert_eq!(flagged, listed);
    // Every operation a generated case reports has a row in the table.
    let _ = CollectionKind::Sequence;
    let _ = CollectionType::new(
        CollectionKind::Set,
        ValueType::Boolean,
        CardinalityBound::new(0, 1).ok(),
    );
    let names: Vec<&str> = OPERATIONS.iter().map(|(name, _)| *name).collect();
    let mut runner = proptest::test_runner::TestRunner::new(prop::config(256));
    for _ in 0..256 {
        let case =
            proptest::strategy::ValueTree::current(&cases().new_tree(&mut runner).expect("tree"));
        for op in &case.operations {
            assert!(names.contains(&op.as_str()), "{op} has no OPERATIONS row");
        }
    }
}
