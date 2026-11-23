// SPDX-License-Identifier: AGPL-3.0-or-later
//! Property tests of the value layer: generated value-function sources are
//! formatted, checked and evaluated, and every answer is compared with a small
//! exact reference evaluator written in this file (`i128` integer and rational
//! arithmetic; it calls no `fsl-eval` code).
//!
//! Every property runs 1,000 cases under the fixed seed of
//! `fsl_proptest_policy::config` (the near-miss negative runs 256).
//!
//! Families generated: `Integer`, `Int[..]` (parameters and refused results),
//! `div`/`rem` under each division law (euclidean, floor, truncating) and
//! `mod`, `Rational` (nested `+`, `-`, `*` and negation, `/` of a leaf by a
//! zero-free parameter, comparison), `Decimal` under each of the six rounding modes (`+`,
//! `-`, `*`, `/`, negation, comparison), `Boolean`, `Text` length and
//! comparison, `Option`, records, tuples, and the aggregates `sum`, `count`,
//! `min` and `max` (as `reduce`), `exists`, `forall` over `Sequence`, `Set`
//! and `Bag`.
//!
//! Not generated, with the reason:
//! - `size<T>(e)`: QSL-1966 (the typed result form is not available).
//! - `count<N>` over its bound: excluded while QSL-2003 is open; every
//!   generated `count` names an alias of `Integer`, so its range is proved.
//! - `^` (power), `Float32`/`Float64`, quantities, unions, maps, references:
//!   outside this ticket's families.
//! - A divisor range that spans zero is never in the well-typed generator
//!   (FR-495-AC-3): it is generated only for the refusal properties, where it
//!   must be refused at check under `Linked` (`unproved-nonzero`) and is
//!   evaluated under `Kernel`. The well-typed divisors are a nonzero literal,
//!   a parameter whose declared range excludes zero, or a parameter under an
//!   exact `!= 0` guard whose declared range spans zero. A guard on a
//!   parameter whose range already excludes zero is not generated: the
//!   literal `0` in `c != 0` is typed into `c`'s interval and the checker
//!   refuses it `unproved-range` (proved [0, 0], required `c`'s interval).
//! - `Decimal` division in the well-typed generator is by a nonzero
//!   `decimal(c, T)` literal only: `Linked` answers `unproved-nonzero` for a
//!   parameter divisor, and a `!= decimal(0, T)` guard does not discharge it.
//!   A parameter divisor is generated for the refusal properties.
//! - A `rational(0, 1)` literal is not generated: `rational(0, 1) / d` with
//!   `d: Rational[1, 20; 1, 9]` is refused `unproved-rational-range` although
//!   `rational(1, 1) / d` is proved. A quotient nested in another operation
//!   (`(rational(1, 2) / d) + a`) is refused `unproved-rational-range` or
//!   `ambiguous-literal`, so `/` is only the root operation.
//! - A rational operation's right operand must lie in its left operand's
//!   domain: `a + (a + c)` is refused `unproved-rational-range` under
//!   `Linked`, and under `Kernel` the right operand `-1/12` is refused
//!   `rational_out_of_domain` against `a: Rational[-20, 20; 1, 9]`. So a
//!   rational expression is a left-nested chain (`((a + b) * c) - 1/2`), a
//!   literal is only a right operand, and only the root is typed at the
//!   declared domain (an intermediate typed at the wide domain, for example by
//!   a `let`, is refused correctly, FR-495-AC-14), so a `let` binds only a
//!   root.
//! - `min` and `max` are `reduce` over a `Sequence` only: `reduce` over a
//!   `Set` or `Bag` is refused `operator-ineligible`.
//! - Equality of two integers or text lengths is written over
//!   `convert<Integer>(..)`: `=` of two `Int[..]` operands with different
//!   inferred intervals is refused `unproved-range`.

use filament_eval::FamilyOutcome;
use filament_exact::{
    admit_text, form_collection, CardinalityBound, CollectionKind, CollectionType, Decimal,
    FieldValue, Integer, IntegerInterval, Meter, OptionValue, Outcome, Rational, Refusal,
    ScalarLimits, TextPayload, TextProfile, TextType, Undefined, Value, ValueType,
};
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use fsl_package::CheckedPackage;
use fsl_proptest_policy as prop;
use fsl_proptest_policy::value::VALUE_HEADER;
use fsl_semantics::check::{CheckRefusal, PackageDeclarations};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use ix_trace_rs::trace;
use proptest::prelude::{any, BoxedStrategy, Just, Strategy};
use proptest::prop_oneof;
use proptest::test_runner::{TestCaseError, TestError};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

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

// ---------------------------------------------------------------- the reference

/// A refusal category, shared by the reference and the evaluator's outcomes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Refuse {
    DivisionByZero,
    IntegerDomain,
    DecimalInexact,
    DecimalDomain,
    RationalDomain,
}

/// The reference's value, rendered to one canonical string by [`show`].
#[derive(Clone, Debug, PartialEq)]
enum Rv {
    Bool(bool),
    Int(i128),
    /// Reduced numerator and positive denominator.
    Rat(i128, i128),
    /// Coefficient retained at the declared maximum scale, and that scale.
    Dec(i128, u32),
    Opt(Option<Box<Rv>>),
    Comp(Vec<Rv>),
}

type Ref<T> = Result<T, Refuse>;

fn show(value: &Rv) -> String {
    match value {
        Rv::Bool(b) => format!("b:{b}"),
        Rv::Int(i) => format!("i:{i}"),
        Rv::Rat(n, d) => format!("q:{n}/{d}"),
        Rv::Dec(c, s) => format!("d:{c}@{s}"),
        Rv::Opt(Some(inner)) => format!("some({})", show(inner)),
        Rv::Opt(None) => "none".to_owned(),
        Rv::Comp(slots) => format!(
            "comp[{}]",
            slots.iter().map(show).collect::<Vec<_>>().join(",")
        ),
    }
}

/// The same canonical string, read off an evaluator value.
fn render(value: &Value) -> String {
    match value {
        Value::Boolean(b) => format!("b:{b}"),
        Value::Integer(i) => format!("i:{i}"),
        Value::Rational(q) => format!("q:{}/{}", q.numerator(), q.denominator()),
        Value::Decimal(d) => {
            let representation = d.representation();
            format!(
                "d:{}@{}",
                representation.coefficient(),
                representation.scale()
            )
        }
        Value::Option(option) => match option.payload() {
            Some(payload) => format!("some({})", render(payload)),
            None => "none".to_owned(),
        },
        Value::Composite(composite) => format!(
            "comp[{}]",
            composite
                .slots()
                .iter()
                .map(|slot| match slot {
                    FieldValue::Present(value) => render(value),
                    other => format!("{other:?}"),
                })
                .collect::<Vec<_>>()
                .join(",")
        ),
        other => format!("{other:?}"),
    }
}

/// A deliberately wrong reference, for `reference_mutation_is_caught`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mutation {
    None,
    AddIsSubtract,
}

#[derive(Clone, Copy, Debug)]
enum Law {
    Euclidean,
    Floor,
    Truncating,
}

impl Law {
    fn name(self) -> &'static str {
        match self {
            Law::Euclidean => "euclidean",
            Law::Floor => "floor",
            Law::Truncating => "truncating",
        }
    }

    /// The quotient under this law; `b` is nonzero.
    fn quotient(self, a: i128, b: i128) -> i128 {
        match self {
            Law::Euclidean => a.div_euclid(b),
            Law::Truncating => a / b,
            Law::Floor => {
                let q = a / b;
                if a % b != 0 && ((a < 0) != (b < 0)) {
                    q - 1
                } else {
                    q
                }
            }
        }
    }
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// The reduced fraction `n/d`, `d` nonzero.
fn reduce(n: i128, d: i128) -> (i128, i128) {
    let g = gcd(n, d).max(1);
    let (n, d) = (n / g, d / g);
    if d < 0 {
        (-n, -d)
    } else {
        (n, d)
    }
}

/// `n/d` rounded to an integer under a decimal rounding mode; `d` positive.
fn round_units(n: i128, d: i128, mode: &str) -> Option<i128> {
    let floor = n.div_euclid(d);
    let rest = n.rem_euclid(d);
    if rest == 0 {
        return Some(floor);
    }
    Some(match mode {
        "exact" => return None,
        "toward-negative" => floor,
        "toward-positive" => floor + 1,
        "toward-zero" => {
            if n >= 0 {
                floor
            } else {
                floor + 1
            }
        }
        "nearest-even" | "nearest-away" => match (2 * rest).cmp(&d) {
            std::cmp::Ordering::Less => floor,
            std::cmp::Ordering::Greater => floor + 1,
            std::cmp::Ordering::Equal => {
                if mode == "nearest-even" {
                    if floor % 2 == 0 {
                        floor
                    } else {
                        floor + 1
                    }
                } else if n >= 0 {
                    floor + 1
                } else {
                    floor
                }
            }
        },
        other => panic!("unknown rounding mode {other}"),
    })
}

const MODES: [&str; 6] = [
    "exact",
    "toward-zero",
    "toward-positive",
    "toward-negative",
    "nearest-even",
    "nearest-away",
];

// ---------------------------------------------------------------- cases

/// An argument, in the terms the test generates it.
#[derive(Clone, Debug)]
enum Arg {
    Int(i64),
    Rat(i128, i128),
    Dec(i128, u32),
    Text(String),
    Opt {
        lower: i64,
        upper: i64,
        value: Option<i64>,
    },
    /// `K<Int[-9, 9]>[minimum, 6]`.
    Coll {
        kind: CollKind,
        minimum: u64,
        items: Vec<i64>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CollKind {
    Sequence,
    Set,
    Bag,
}

impl CollKind {
    fn name(self) -> &'static str {
        match self {
            CollKind::Sequence => "Sequence",
            CollKind::Set => "Set",
            CollKind::Bag => "Bag",
        }
    }
}

fn int_interval(lower: i64, upper: i64) -> IntegerInterval {
    IntegerInterval::new(Integer::from(lower), Integer::from(upper)).expect("ordered bounds")
}

fn collection_value(kind: CollKind, minimum: u64, items: &[i64]) -> Value {
    let element = ValueType::Int(int_interval(-9, 9));
    let kind = match kind {
        CollKind::Sequence => CollectionKind::Sequence,
        CollKind::Set => CollectionKind::Set,
        CollKind::Bag => CollectionKind::Bag,
    };
    let ty = CollectionType::new(
        kind,
        element,
        Some(CardinalityBound::new(minimum, 6).expect("bound")),
    );
    let members = items
        .iter()
        .map(|v| Value::Integer(Integer::from(*v)))
        .collect();
    form_collection(&ty, members, &mut Meter::new(UNLIMITED))
        .expect("members are admitted")
        .completed()
        .expect("the collection forms")
}

impl Arg {
    fn value(&self) -> Value {
        match self {
            Arg::Int(i) => Value::Integer(Integer::from(*i)),
            Arg::Rat(n, d) => Value::Rational(
                Rational::new(Integer::from(*n), Integer::from(*d)).expect("nonzero denominator"),
            ),
            Arg::Dec(c, s) => Value::Decimal(Decimal::new(Integer::from(*c), *s)),
            Arg::Text(s) => {
                let ty = TextType::new(0, 16, TextProfile::UnicodeScalars).expect("text bounds");
                let payload = TextPayload::from_utf8(s.as_bytes()).expect("utf-8");
                match admit_text(&payload, &ty, &mut Meter::new(UNLIMITED)) {
                    Outcome::Completed(text) => Value::Text(text),
                    other => panic!("text is admitted: {other:?}"),
                }
            }
            Arg::Opt {
                lower,
                upper,
                value,
            } => {
                let ty = ValueType::Int(int_interval(*lower, *upper));
                match value {
                    Some(v) => OptionValue::present(ty, Value::Integer(Integer::from(*v)))
                        .expect("payload admitted"),
                    None => OptionValue::none(ty),
                }
            }
            Arg::Coll {
                kind,
                minimum,
                items,
            } => collection_value(*kind, *minimum, items),
        }
    }
}

/// One generated function, its arguments and the reference's answer.
#[derive(Clone, Debug)]
struct Case {
    family: &'static str,
    source: String,
    kernel: bool,
    args: Vec<Arg>,
    expected: Ref<String>,
    operators: usize,
    /// The rounding mode of a decimal operation that discarded a nonzero digit.
    rounded: Option<&'static str>,
}

/// A unit; `law` selects the integer-division profile, which only `div` and
/// `rem` need.
fn unit(
    law: Option<Law>,
    declarations: &str,
    parameters: &str,
    result: &str,
    body: &str,
) -> String {
    let profile = law.map_or_else(String::new, |law| {
        format!(
            "profile d = \"filament.value.integer-division.{}/v1\";\n",
            law.name()
        )
    });
    format!(
        "{VALUE_HEADER}{profile}{declarations}function f using v({parameters}): {result} pure {{ {body} }}\n"
    )
}

/// Why a source did not compile: before the checker, or by its refusals.
enum Failure {
    Front(String),
    Refused(Vec<CheckRefusal>),
}

impl Failure {
    fn text(&self) -> String {
        match self {
            Failure::Front(text) => text.clone(),
            Failure::Refused(refusals) => format!(
                "check: {:?}",
                refusals
                    .iter()
                    .map(|r| format!("{:?}", r.cause))
                    .collect::<Vec<_>>()
            ),
        }
    }
}

/// The source compiled to a checked, linked package, or why it did not.
fn compile(source: &str, mode: CheckMode) -> Result<CheckedPackage, Failure> {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "value-properties"),
        "value.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .map_err(|d| Failure::Front(format!("parse: {d:?}")))?;
    if !parsed.is_admissible() {
        return Err(Failure::Front(format!("parse: {:?}", parsed.diagnostics())));
    }
    let unit =
        fsl_forms::build_unit(&parsed).map_err(|e| Failure::Front(format!("forms: {e:?}")))?;
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "value-properties".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .map_err(|e| Failure::Front(format!("assemble: {e:?}")))?
    .check(
        mode,
        CheckingLimits::default(),
        &fsl_eval::value::S6aRewardEvaluator,
    )
    .map(CheckedPackage::link)
    .map_err(Failure::Refused)
}

/// [`compile`] with the failure as text.
fn try_compile(source: &str, mode: CheckMode) -> Result<CheckedPackage, String> {
    compile(source, mode).map_err(|failure| failure.text())
}

/// The evaluator's answer for `args`, in the reference's terms.
fn evaluate(package: &CheckedPackage, args: &[Arg]) -> Ref<String> {
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("identifier"),
            args.iter().map(Arg::value).collect(),
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call returns an outcome");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    match outcome {
        Outcome::Completed(value) => Ok(render(&value)),
        Outcome::Undefined(Undefined::DivisionByZero) => Err(Refuse::DivisionByZero),
        Outcome::Refused(Refusal::IntegerOutOfDomain { .. }) => Err(Refuse::IntegerDomain),
        Outcome::Refused(Refusal::InexactDecimal { .. }) => Err(Refuse::DecimalInexact),
        Outcome::Refused(Refusal::DecimalOutOfDomain { .. }) => Err(Refuse::DecimalDomain),
        Outcome::Refused(Refusal::RationalOutOfDomain { .. }) => Err(Refuse::RationalDomain),
        other => panic!("an outcome with no reference category: {other:?}"),
    }
}

fn mode_of(case: &Case) -> CheckMode {
    if case.kernel {
        CheckMode::Kernel
    } else {
        CheckMode::Linked
    }
}

/// `Ok(Ok(()))` when the reference returned a value and the evaluator agreed,
/// `Ok(Err(refusal))` when both refused alike, and the full report otherwise.
fn agree(case: &Case) -> Result<Ref<()>, String> {
    let package = try_compile(&case.source, mode_of(case)).map_err(|e| {
        format!(
            "the checker refused a well-typed program: {e}\n{}",
            describe(case)
        )
    })?;
    let actual = evaluate(&package, &case.args);
    if actual == case.expected {
        Ok(actual.map(drop))
    } else {
        Err(format!(
            "reference {:?}, evaluator {:?}\n{}",
            case.expected,
            actual,
            describe(case)
        ))
    }
}

fn describe(case: &Case) -> String {
    format!("source:\n{}arguments: {:?}", case.source, case.args)
}

// ---------------------------------------------------------------- integers

#[derive(Clone, Copy, Debug)]
enum Op {
    Add,
    Subtract,
    Multiply,
}

#[derive(Clone, Copy, Debug)]
enum DivOp {
    Div,
    Rem,
    Mod,
}

#[derive(Clone, Copy, Debug)]
enum Cmp {
    Less,
    LessEqual,
    Equal,
    NotEqual,
}

/// A divisor: a nonzero literal, the zero-free parameter `d`, a parameter
/// under an exact `!= 0` guard (with its fallback), or a raw parameter or zero
/// the checker cannot exclude.
#[derive(Clone, Debug)]
enum Divisor {
    Literal(i64),
    Safe,
    Guarded(usize, i64),
    Raw(usize),
}

#[derive(Clone, Debug)]
enum Ie {
    Lit(i64),
    Param(usize),
    Negate(Box<Ie>),
    Bin(Op, Box<Ie>, Box<Ie>),
    Div(DivOp, Box<Ie>, Divisor),
    If(Box<Be>, Box<Ie>, Box<Ie>),
}

#[derive(Clone, Debug)]
enum Be {
    Lit(bool),
    Cmp(Cmp, Ie, Ie),
    And(Box<Be>, Box<Be>),
    Or(Box<Be>, Box<Be>),
    Not(Box<Be>),
}

const NAMES: [&str; 4] = ["a", "b", "c", "d"];

fn cmp_symbol(cmp: Cmp) -> &'static str {
    match cmp {
        Cmp::Less => "<",
        Cmp::LessEqual => "<=",
        Cmp::Equal => "=",
        Cmp::NotEqual => "!=",
    }
}

fn op_symbol(op: Op) -> &'static str {
    match op {
        Op::Add => "+",
        Op::Subtract => "-",
        Op::Multiply => "*",
    }
}

fn div_symbol(op: DivOp) -> &'static str {
    match op {
        DivOp::Div => "div",
        DivOp::Rem => "rem",
        DivOp::Mod => "mod",
    }
}

fn ie_source(e: &Ie) -> String {
    match e {
        Ie::Lit(v) if *v < 0 => format!("(0 - {})", -v),
        Ie::Lit(v) => v.to_string(),
        Ie::Param(i) => NAMES[*i].to_owned(),
        Ie::Negate(x) => format!("(-({}))", ie_source(x)),
        Ie::Bin(op, l, r) => format!("({} {} {})", ie_source(l), op_symbol(*op), ie_source(r)),
        Ie::Div(op, l, divisor) => {
            let l = ie_source(l);
            let sym = div_symbol(*op);
            match divisor {
                Divisor::Literal(v) if *v < 0 => format!("({l} {sym} (0 - {}))", -v),
                Divisor::Literal(v) => format!("({l} {sym} {v})"),
                Divisor::Safe => format!("({l} {sym} d)"),
                Divisor::Raw(p) => format!("({l} {sym} {})", NAMES[*p]),
                Divisor::Guarded(p, fallback) => {
                    let p = NAMES[*p];
                    let fallback = ie_source(&Ie::Lit(*fallback));
                    format!("(if {p} != 0 then ({l} {sym} {p}) else {fallback})")
                }
            }
        }
        Ie::If(c, y, n) => format!(
            "(if {} then ({}) else ({}))",
            be_source(c),
            ie_source(y),
            ie_source(n)
        ),
    }
}

fn be_source(e: &Be) -> String {
    match e {
        Be::Lit(b) => b.to_string(),
        // Equality needs one domain: widen both sides to `Integer`.
        Be::Cmp(c @ (Cmp::Equal | Cmp::NotEqual), l, r) => format!(
            "(convert<Integer>({}) {} convert<Integer>({}))",
            ie_source(l),
            cmp_symbol(*c),
            ie_source(r)
        ),
        Be::Cmp(c, l, r) => format!("({} {} {})", ie_source(l), cmp_symbol(*c), ie_source(r)),
        Be::And(l, r) => format!("({} and {})", be_source(l), be_source(r)),
        Be::Or(l, r) => format!("({} or {})", be_source(l), be_source(r)),
        Be::Not(x) => format!("not ({})", be_source(x)),
    }
}

fn ie_operators(e: &Ie) -> usize {
    match e {
        Ie::Lit(_) | Ie::Param(_) => 0,
        Ie::Negate(x) => 1 + ie_operators(x),
        Ie::Bin(_, l, r) => 1 + ie_operators(l) + ie_operators(r),
        Ie::Div(_, l, _) => 1 + ie_operators(l),
        Ie::If(c, y, n) => 1 + be_operators(c) + ie_operators(y) + ie_operators(n),
    }
}

fn be_operators(e: &Be) -> usize {
    match e {
        Be::Lit(_) => 0,
        Be::Cmp(_, l, r) => 1 + ie_operators(l) + ie_operators(r),
        Be::And(l, r) | Be::Or(l, r) => 1 + be_operators(l) + be_operators(r),
        Be::Not(x) => 1 + be_operators(x),
    }
}

struct IntRef {
    law: Law,
    mutation: Mutation,
    env: [i128; 4],
}

impl IntRef {
    fn ie(&self, e: &Ie) -> Ref<i128> {
        Ok(match e {
            Ie::Lit(v) => i128::from(*v),
            Ie::Param(i) => self.env[*i],
            Ie::Negate(x) => -self.ie(x)?,
            Ie::Bin(op, l, r) => {
                let (l, r) = (self.ie(l)?, self.ie(r)?);
                match op {
                    Op::Add if self.mutation == Mutation::AddIsSubtract => l - r,
                    Op::Add => l + r,
                    Op::Subtract => l - r,
                    Op::Multiply => l * r,
                }
            }
            Ie::Div(op, l, divisor) => {
                let guarded_out = match divisor {
                    Divisor::Guarded(p, fallback) if self.env[*p] == 0 => Some(*fallback),
                    _ => None,
                };
                if let Some(fallback) = guarded_out {
                    return Ok(i128::from(fallback));
                }
                let l = self.ie(l)?;
                let b = match divisor {
                    Divisor::Literal(v) => i128::from(*v),
                    Divisor::Safe => self.env[3],
                    Divisor::Guarded(p, _) | Divisor::Raw(p) => self.env[*p],
                };
                if b == 0 {
                    return Err(Refuse::DivisionByZero);
                }
                match op {
                    DivOp::Div => self.law.quotient(l, b),
                    DivOp::Rem => l - b * self.law.quotient(l, b),
                    DivOp::Mod => l.rem_euclid(b),
                }
            }
            Ie::If(c, y, n) => {
                if self.be(c)? {
                    self.ie(y)?
                } else {
                    self.ie(n)?
                }
            }
        })
    }

    fn be(&self, e: &Be) -> Ref<bool> {
        Ok(match e {
            Be::Lit(b) => *b,
            Be::Cmp(c, l, r) => {
                let (l, r) = (self.ie(l)?, self.ie(r)?);
                match c {
                    Cmp::Less => l < r,
                    Cmp::LessEqual => l <= r,
                    Cmp::Equal => l == r,
                    Cmp::NotEqual => l != r,
                }
            }
            Be::And(l, r) => self.be(l)? && self.be(r)?,
            Be::Or(l, r) => self.be(l)? || self.be(r)?,
            Be::Not(x) => !self.be(x)?,
        })
    }
}

#[derive(Clone, Debug)]
struct IParam {
    /// `None` is `Integer`.
    range: Option<(i64, i64)>,
    value: i64,
}

fn iparam(zero_free: bool) -> BoxedStrategy<IParam> {
    if zero_free {
        prop_oneof![
            (1i64..=9).prop_map(|v| IParam {
                range: Some((1, 9)),
                value: v
            }),
            (-9i64..=-1).prop_map(|v| IParam {
                range: Some((-9, -1)),
                value: v
            }),
        ]
        .boxed()
    } else {
        prop_oneof![
            (-30i64..=30).prop_map(|v| IParam {
                range: None,
                value: v
            }),
            (-9i64..=0, 0i64..=9).prop_flat_map(|(lower, upper)| {
                (lower..=upper).prop_map(move |v| IParam {
                    range: Some((lower, upper)),
                    value: v,
                })
            }),
        ]
        .boxed()
    }
}

fn ie_strategy(unsafe_division: bool) -> BoxedStrategy<Ie> {
    let leaf = prop_oneof![
        (-20i64..=20).prop_map(Ie::Lit),
        (0usize..4).prop_map(Ie::Param),
    ];
    leaf.prop_recursive(3, 14, 2, move |inner| {
        let op = proptest::sample::select(vec![Op::Add, Op::Subtract, Op::Multiply]);
        let div_op = proptest::sample::select(vec![DivOp::Div, DivOp::Rem, DivOp::Mod]);
        let cmp =
            proptest::sample::select(vec![Cmp::Less, Cmp::LessEqual, Cmp::Equal, Cmp::NotEqual]);
        let divisor: BoxedStrategy<Divisor> = if unsafe_division {
            prop_oneof![
                (1i64..=7).prop_map(Divisor::Literal),
                Just(Divisor::Literal(0)),
                Just(Divisor::Safe),
                (0usize..3, -5i64..=5).prop_map(|(p, f)| Divisor::Guarded(p, f)),
                (0usize..3).prop_map(Divisor::Raw),
            ]
            .boxed()
        } else {
            prop_oneof![
                (1i64..=7).prop_map(Divisor::Literal),
                (-7i64..=-1).prop_map(Divisor::Literal),
                Just(Divisor::Safe),
                (0usize..3, -5i64..=5).prop_map(|(p, f)| Divisor::Guarded(p, f)),
            ]
            .boxed()
        };
        let condition = prop_oneof![
            (cmp.clone(), inner.clone(), inner.clone()).prop_map(|(c, l, r)| Be::Cmp(c, l, r)),
            (cmp.clone(), inner.clone(), inner.clone(), any::<bool>()).prop_map(
                |(c, l, r, negate)| {
                    let base = Be::Cmp(c, l, r);
                    if negate {
                        Be::Not(Box::new(base))
                    } else {
                        base
                    }
                }
            ),
            (
                cmp.clone(),
                inner.clone(),
                inner.clone(),
                cmp.clone(),
                inner.clone(),
                inner.clone(),
                any::<bool>()
            )
                .prop_map(|(c1, l1, r1, c2, l2, r2, and)| {
                    let (x, y) = (Box::new(Be::Cmp(c1, l1, r1)), Box::new(Be::Cmp(c2, l2, r2)));
                    if and {
                        Be::And(x, y)
                    } else {
                        Be::Or(x, y)
                    }
                }),
            any::<bool>().prop_map(Be::Lit),
        ];
        prop_oneof![
            inner.clone().prop_map(|x| Ie::Negate(Box::new(x))),
            (op.clone(), inner.clone(), inner.clone()).prop_map(|(op, l, r)| Ie::Bin(
                op,
                Box::new(l),
                Box::new(r)
            )),
            (op, inner.clone(), inner.clone()).prop_map(|(op, l, r)| Ie::Bin(
                op,
                Box::new(l),
                Box::new(r)
            )),
            (div_op, inner.clone(), divisor).prop_map(|(op, l, d)| Ie::Div(op, Box::new(l), d)),
            (condition, inner.clone(), inner).prop_map(|(c, y, n)| Ie::If(
                Box::new(c),
                Box::new(y),
                Box::new(n)
            )),
        ]
    })
    .boxed()
}

fn law_strategy() -> BoxedStrategy<Law> {
    proptest::sample::select(vec![Law::Euclidean, Law::Floor, Law::Truncating]).boxed()
}

/// Integer and Boolean functions. `refusing` adds divisors the checker cannot
/// exclude and a narrow `Int[..]` result, both evaluated under `Kernel`.
fn int_cases(refusing: bool, mutation: Mutation) -> BoxedStrategy<Case> {
    let params = (iparam(false), iparam(false), iparam(false), iparam(true));
    let root = prop_oneof![
        3 => ie_strategy(refusing).prop_map(|e| (Some(e), None)),
        1 => (ie_strategy(refusing), ie_strategy(refusing), proptest::sample::select(vec![
            Cmp::Less, Cmp::LessEqual, Cmp::Equal, Cmp::NotEqual,
        ]))
            .prop_map(|(l, r, c)| (None, Some(Be::Cmp(c, l, r)))),
    ];
    let narrow = if refusing {
        prop_oneof![Just(None), (-30i64..=0, 0i64..=30).prop_map(Some)].boxed()
    } else {
        Just(None).boxed()
    };
    (params, root, law_strategy(), narrow)
        .prop_map(move |(params, (int_root, bool_root), law, narrow)| {
            let env = [
                i128::from(params.0.value),
                i128::from(params.1.value),
                i128::from(params.2.value),
                i128::from(params.3.value),
            ];
            let reference = IntRef { law, mutation, env };
            let parameter_list = [&params.0, &params.1, &params.2, &params.3]
                .iter()
                .zip(NAMES)
                .map(|(p, name)| match p.range {
                    None => format!("{name}: Integer"),
                    Some((lo, hi)) => format!("{name}: Int[{lo}, {hi}]"),
                })
                .collect::<Vec<_>>()
                .join(", ");
            let (result, body, expected, operators) = match (&int_root, &bool_root) {
                (Some(e), _) => {
                    let result =
                        narrow.map_or("Integer".to_owned(), |(lo, hi)| format!("Int[{lo}, {hi}]"));
                    let expected = reference.ie(e).and_then(|v| match narrow {
                        Some((lo, hi)) if v < i128::from(lo) || v > i128::from(hi) => {
                            Err(Refuse::IntegerDomain)
                        }
                        _ => Ok(show(&Rv::Int(v))),
                    });
                    (result, ie_source(e), expected, ie_operators(e))
                }
                (_, Some(b)) => (
                    "Boolean".to_owned(),
                    be_source(b),
                    reference.be(b).map(|v| show(&Rv::Bool(v))),
                    be_operators(b),
                ),
                _ => unreachable!("one root"),
            };
            Case {
                family: "integer",
                source: unit(Some(law), "", &parameter_list, &result, &body),
                kernel: refusing,
                args: [params.0, params.1, params.2, params.3]
                    .iter()
                    .map(|p| Arg::Int(p.value))
                    .collect(),
                expected,
                operators,
                rounded: None,
            }
        })
        .boxed()
}

// ---------------------------------------------------------------- rationals

#[derive(Clone, Debug)]
enum Qe {
    Lit(i64, i64),
    Param(usize),
    Negate(Box<Qe>),
    Bin(Op, Box<Qe>, Box<Qe>),
    /// `l / d` by the zero-free parameter `d` when `safe`, else by parameter `p`.
    Div(Box<Qe>, usize, bool),
}

fn qe_source(e: &Qe) -> String {
    match e {
        Qe::Lit(n, d) => format!("rational({n}, {d})"),
        Qe::Param(i) => NAMES[*i].to_owned(),
        Qe::Negate(x) => format!("(-({}))", qe_source(x)),
        Qe::Bin(op, l, r) => format!("({} {} {})", qe_source(l), op_symbol(*op), qe_source(r)),
        Qe::Div(l, _, true) => format!("({} / d)", qe_source(l)),
        Qe::Div(l, p, false) => format!("({} / {})", qe_source(l), NAMES[*p]),
    }
}

fn qe_operators(e: &Qe) -> usize {
    match e {
        Qe::Lit(..) | Qe::Param(_) => 0,
        Qe::Negate(x) => 1 + qe_operators(x),
        Qe::Bin(_, l, r) => 1 + qe_operators(l) + qe_operators(r),
        Qe::Div(l, ..) => 1 + qe_operators(l),
    }
}

fn qe_eval(e: &Qe, env: &[(i128, i128); 4]) -> Ref<(i128, i128)> {
    Ok(match e {
        Qe::Lit(n, d) => reduce(i128::from(*n), i128::from(*d)),
        Qe::Param(i) => env[*i],
        Qe::Negate(x) => {
            let (n, d) = qe_eval(x, env)?;
            (-n, d)
        }
        Qe::Bin(op, l, r) => {
            let ((ln, ld), (rn, rd)) = (qe_eval(l, env)?, qe_eval(r, env)?);
            match op {
                Op::Add => reduce(ln * rd + rn * ld, ld * rd),
                Op::Subtract => reduce(ln * rd - rn * ld, ld * rd),
                Op::Multiply => reduce(ln * rn, ld * rd),
            }
        }
        Qe::Div(l, p, safe) => {
            let (pn, pd) = env[if *safe { 3 } else { *p }];
            let (ln, ld) = qe_eval(l, env)?;
            if pn == 0 {
                return Err(Refuse::DivisionByZero);
            }
            reduce(ln * pd, ld * pn)
        }
    })
}

/// `-e`, except that a constant stays as it is: a negated literal is ambiguous.
fn negate(e: Qe) -> Qe {
    if constant(&e) {
        e
    } else {
        Qe::Negate(Box::new(e))
    }
}

/// A literal-only tree has no type to take from context: add a parameter.
fn anchored(e: Qe) -> Qe {
    if constant(&e) {
        binary(Op::Add, Qe::Param(0), e)
    } else {
        e
    }
}

/// Whether the tree has no typed leaf. A rational literal takes its type from
/// context, so an operation on two such trees is ambiguous.
fn constant(e: &Qe) -> bool {
    match e {
        Qe::Lit(..) => true,
        Qe::Param(_) | Qe::Div(..) => false,
        Qe::Negate(x) => constant(x),
        Qe::Bin(_, l, r) => constant(l) && constant(r),
    }
}

/// `left op right`, with a parameter in place of `right` when both are constant.
fn binary(op: Op, left: Qe, right: Qe) -> Qe {
    let right = if constant(&left) && constant(&right) {
        Qe::Param(0)
    } else {
        right
    };
    Qe::Bin(op, Box::new(left), Box::new(right))
}

/// Rational operations: a left-nested chain `((p op r) op r) ..` from a
/// parameter `p`, each step a negation or an operation with a parameter or a
/// literal on the right, or one division of a leaf at the root. Only the root
/// is typed at the declared domain; the checker infers each intermediate
/// interval exactly.
fn qe_strategy(unguarded: bool) -> BoxedStrategy<Qe> {
    let literal = (1i64..=6, 1i64..=5).prop_map(|(n, d)| {
        let (n, d) = reduce(i128::from(n), i128::from(d));
        Qe::Lit(i64::try_from(n).unwrap(), i64::try_from(d).unwrap())
    });
    let parameter = (0usize..3).prop_map(Qe::Param);
    let right = prop_oneof![literal, parameter.clone()];
    let op = proptest::sample::select(vec![Op::Add, Op::Subtract, Op::Multiply]);
    let step = prop_oneof![
        1 => Just(None),
        4 => (op, right).prop_map(Some),
    ];
    let chain =
        (parameter.clone(), proptest::collection::vec(step, 0..=3)).prop_map(|(start, steps)| {
            steps.into_iter().fold(start, |left, step| match step {
                Some((op, right)) => binary(op, left, right),
                None => negate(left),
            })
        });
    // A parameter divisor can be zero, so it is a `Kernel` case.
    let safe = if unguarded {
        any::<bool>().boxed()
    } else {
        Just(true).boxed()
    };
    let leaf = prop_oneof![parameter, (1i64..=6).prop_map(|n| Qe::Lit(n, 1))];
    prop_oneof![
        5 => chain,
        1 => (leaf, 0usize..3, safe).prop_map(|(l, p, safe)| Qe::Div(Box::new(l), p, safe)),
    ]
    .boxed()
}

/// The node as a value of domain `target`: an operation is typed by its
/// declared domain; a bare leaf is converted into it.
fn qe_node(e: &Qe, target: &str) -> String {
    match e {
        Qe::Lit(..) | Qe::Param(_) => format!("convert<{target}>({})", qe_source(e)),
        _ => qe_source(e),
    }
}

fn rat_param() -> BoxedStrategy<(i128, i128)> {
    (-20i64..=20, 1i64..=9)
        .prop_map(|(n, d)| reduce(i128::from(n), i128::from(d)))
        .boxed()
}

/// The zero-free divisor parameter `d: Rational[1, 20; 1, 9]`.
fn positive_rat_param() -> BoxedStrategy<(i128, i128)> {
    (1i64..=20, 1i64..=9)
        .prop_map(|(n, d)| reduce(i128::from(n), i128::from(d)))
        .boxed()
}

fn rat_cases(refusing: bool) -> BoxedStrategy<Case> {
    let narrow = if refusing {
        (1i64..=12, 1i64..=6).prop_map(|(n, d)| (n, d)).boxed()
    } else {
        Just((1_000_000, 1_000_000)).boxed()
    };
    (
        (rat_param(), rat_param(), rat_param(), positive_rat_param()),
        qe_strategy(refusing),
        qe_strategy(refusing),
        proptest::sample::select(vec![Cmp::Less, Cmp::LessEqual, Cmp::Equal, Cmp::NotEqual]),
        any::<bool>(),
        narrow,
    )
        .prop_map(
            move |((a, b, c, d), left, right, cmp, boolean, (nmax, dmax))| {
                let (left, right) = (anchored(left), anchored(right));
                let env = [a, b, c, d];
                let parameter_list = "a: Rational[-20, 20; 1, 9], b: Rational[-20, 20; 1, 9], \
                                  c: Rational[-20, 20; 1, 9], d: Rational[1, 20; 1, 9]";
                // Each operation is typed by the declared domain `W` of a `let`.
                let declarations = "type W = Rational[-1000000, 1000000; 1, 1000000];\n";
                let (result, body, expected, operators) = if boolean {
                    let value = qe_eval(&left, &env).and_then(|(ln, ld)| {
                        let (rn, rd) = qe_eval(&right, &env)?;
                        let (l, r) = (ln * rd, rn * ld);
                        Ok(match cmp {
                            Cmp::Less => l < r,
                            Cmp::LessEqual => l <= r,
                            Cmp::Equal => l == r,
                            Cmp::NotEqual => l != r,
                        })
                    });
                    let body = format!(
                        "let x: W = {} in let y: W = {} in (x {} y)",
                        qe_node(&left, "W"),
                        qe_node(&right, "W"),
                        cmp_symbol(cmp)
                    );
                    (
                        "Boolean".to_owned(),
                        body,
                        value.map(|v| show(&Rv::Bool(v))),
                        1 + qe_operators(&left) + qe_operators(&right),
                    )
                } else {
                    let value = qe_eval(&left, &env).and_then(|(n, d)| {
                        if n.abs() <= i128::from(nmax) && d <= i128::from(dmax) {
                            Ok(show(&Rv::Rat(n, d)))
                        } else {
                            Err(Refuse::RationalDomain)
                        }
                    });
                    (
                        format!("Rational[-{nmax}, {nmax}; 1, {dmax}]"),
                        qe_node(&left, &format!("Rational[-{nmax}, {nmax}; 1, {dmax}]")),
                        value,
                        qe_operators(&left),
                    )
                };
                Case {
                    family: "rational",
                    source: unit(None, declarations, parameter_list, &result, &body),
                    kernel: refusing,
                    args: env.iter().map(|(n, d)| Arg::Rat(*n, *d)).collect(),
                    expected,
                    operators,
                    rounded: None,
                }
            },
        )
        .boxed()
}

// ---------------------------------------------------------------- decimals

#[derive(Clone, Debug)]
enum De {
    Param(usize),
    Negate(Box<De>),
    Bin(Op, Box<De>, Box<De>),
    Div(Box<De>, DDiv),
}

/// A divisor: a nonzero `decimal(c, T)` literal at the type's scale `T`, or parameter `p` (refusing
/// cases only, since the checker cannot exclude a zero parameter).
#[derive(Clone, Debug)]
enum DDiv {
    Lit(i128),
    Raw(usize),
}

fn de_source(e: &De, scale: u32) -> String {
    match e {
        De::Param(i) => NAMES[*i].to_owned(),
        De::Negate(x) => format!("(-({}))", de_source(x, scale)),
        De::Bin(op, l, r) => {
            format!(
                "({} {} {})",
                de_source(l, scale),
                op_symbol(*op),
                de_source(r, scale)
            )
        }
        De::Div(l, DDiv::Lit(c)) => format!("({} / decimal({c}, {scale}))", de_source(l, scale)),
        De::Div(l, DDiv::Raw(p)) => format!("({} / {})", de_source(l, scale), NAMES[*p]),
    }
}

fn de_operators(e: &De) -> usize {
    match e {
        De::Param(_) => 0,
        De::Negate(x) => 1 + de_operators(x),
        De::Bin(_, l, r) => 1 + de_operators(l) + de_operators(r),
        De::Div(l, ..) => 1 + de_operators(l),
    }
}

/// A declared `Decimal[-limit, limit; min_scale, scale; mode]` target.
#[derive(Clone, Debug)]
struct DecType {
    limit: i128,
    min_scale: u32,
    scale: u32,
    mode: &'static str,
    /// Set when an operation had to round (a `*` or `/` left a nonzero digit).
    rounded: Cell<bool>,
}

impl DecType {
    fn source(&self) -> String {
        format!(
            "Decimal[-{}, {}; {}, {}; {}]",
            self.limit, self.limit, self.min_scale, self.scale, self.mode
        )
    }

    /// Normalise the retained coefficient, lift it to `max(scale, min)`, and
    /// test the bounds (QSpec FR-140 membership).
    fn admits(&self, coefficient: i128) -> bool {
        let (mut c, mut s) = (coefficient, self.scale);
        while s > 0 && c % 10 == 0 {
            c /= 10;
            s -= 1;
        }
        let lifted = s.max(self.min_scale);
        let c = c * 10i128.pow(lifted - s);
        lifted <= self.scale && c.abs() <= self.limit
    }

    /// Place the exact quotient `n/d` (`d` positive) at the declared scale.
    fn place(&self, n: i128, d: i128) -> Ref<i128> {
        let scaled = n * 10i128.pow(self.scale);
        self.rounded
            .set(self.rounded.get() || scaled.rem_euclid(d) != 0);
        let units = round_units(scaled, d, self.mode).ok_or(Refuse::DecimalInexact)?;
        if self.admits(units) {
            Ok(units)
        } else {
            Err(Refuse::DecimalDomain)
        }
    }
}

fn de_eval(e: &De, ty: &DecType, env: &[i128; 3]) -> Ref<i128> {
    let t = 10i128.pow(ty.scale);
    match e {
        De::Param(i) => Ok(env[*i]),
        De::Negate(x) => {
            let v = de_eval(x, ty, env)?;
            ty.place(-v, t)
        }
        De::Bin(op, l, r) => {
            let (l, r) = (de_eval(l, ty, env)?, de_eval(r, ty, env)?);
            match op {
                Op::Add => ty.place(l + r, t),
                Op::Subtract => ty.place(l - r, t),
                Op::Multiply => ty.place(l * r, t * t),
            }
        }
        De::Div(l, divisor) => {
            let l = de_eval(l, ty, env)?;
            match divisor {
                // `l / decimal(c, T)`: both retain scale `T`, so the quotient is `l / c`.
                DDiv::Lit(c) => {
                    let (n, d) = reduce(l, *c);
                    ty.place(n, d)
                }
                DDiv::Raw(p) => {
                    if env[*p] == 0 {
                        return Err(Refuse::DivisionByZero);
                    }
                    let (n, d) = reduce(l, env[*p]);
                    ty.place(n, d)
                }
            }
        }
    }
}

fn de_strategy(unguarded: bool) -> BoxedStrategy<De> {
    (0usize..3)
        .prop_map(De::Param)
        .prop_recursive(2, 8, 2, move |inner| {
            let op = proptest::sample::select(vec![Op::Add, Op::Subtract, Op::Multiply]);
            let literal = (1i128..=9, any::<bool>())
                .prop_map(|(c, negative)| DDiv::Lit(if negative { -c } else { c }));
            let divisor = if unguarded {
                prop_oneof![literal, (0usize..3).prop_map(DDiv::Raw)].boxed()
            } else {
                literal.boxed()
            };
            prop_oneof![
                inner.clone().prop_map(|x| De::Negate(Box::new(x))),
                (op, inner.clone(), inner.clone()).prop_map(|(op, l, r)| De::Bin(
                    op,
                    Box::new(l),
                    Box::new(r)
                )),
                (inner, divisor).prop_map(|(l, d)| De::Div(Box::new(l), d)),
            ]
        })
        .boxed()
}

fn dec_cases(refusing: bool) -> BoxedStrategy<Case> {
    let limit = if refusing {
        (20i128..=900).boxed()
    } else {
        Just(1_000_000i128).boxed()
    };
    (
        proptest::sample::select(MODES.to_vec()),
        0u32..=3,
        any::<bool>(),
        limit,
        proptest::collection::vec(-1500i128..=1500, 3),
        de_strategy(refusing),
        de_strategy(refusing),
        proptest::sample::select(vec![Cmp::Less, Cmp::LessEqual, Cmp::Equal, Cmp::NotEqual]),
        any::<bool>(),
    )
        .prop_map(
            move |(mode, scale, lift, limit, coefficients, left, right, cmp, boolean)| {
                let ty = DecType {
                    limit,
                    min_scale: if lift { scale } else { 0 },
                    scale,
                    mode,
                    rounded: Cell::new(false),
                };
                // Members of the declared type, retained at its maximum scale.
                let env = [coefficients[0], coefficients[1], coefficients[2]].map(|c| {
                    if ty.admits(c) {
                        c
                    } else {
                        0
                    }
                });
                let declarations = format!("type D = {};\n", ty.source());
                let parameter_list = "a: D, b: D, c: D";
                let (result, body, expected, operators) = if boolean {
                    let value = de_eval(&left, &ty, &env).and_then(|l| {
                        let r = de_eval(&right, &ty, &env)?;
                        Ok(match cmp {
                            Cmp::Less => l < r,
                            Cmp::LessEqual => l <= r,
                            Cmp::Equal => l == r,
                            Cmp::NotEqual => l != r,
                        })
                    });
                    (
                        "Boolean",
                        format!(
                            "({} {} {})",
                            de_source(&left, ty.scale),
                            cmp_symbol(cmp),
                            de_source(&right, ty.scale)
                        ),
                        value.map(|v| show(&Rv::Bool(v))),
                        1 + de_operators(&left) + de_operators(&right),
                    )
                } else {
                    (
                        "D",
                        de_source(&left, ty.scale),
                        de_eval(&left, &ty, &env).map(|c| show(&Rv::Dec(c, ty.scale))),
                        de_operators(&left),
                    )
                };
                Case {
                    family: "decimal",
                    source: unit(None, &declarations, parameter_list, result, &body),
                    kernel: refusing,
                    args: env.iter().map(|c| Arg::Dec(*c, ty.scale)).collect(),
                    expected,
                    operators,
                    rounded: ty.rounded.get().then_some(ty.mode),
                }
            },
        )
        .boxed()
}

// ---------------------------------------------------------------- text

#[derive(Clone, Debug)]
enum Te {
    SizeOf(usize),
    Lit(i64),
    Add(Box<Te>, Box<Te>),
}

#[derive(Clone, Debug)]
enum Tb {
    IntCmp(Cmp, Te, Te),
    /// A comparison of parameter `0` or `1`, or the literal, with another.
    TextCmp(Cmp, usize, usize),
    And(Box<Tb>, Box<Tb>),
    Not(Box<Tb>),
}

fn text_name(i: usize) -> &'static str {
    if i == 0 {
        "t"
    } else {
        "u"
    }
}

fn te_source(e: &Te) -> String {
    match e {
        Te::SizeOf(i) => format!("size({})", text_name(*i)),
        Te::Lit(v) => v.to_string(),
        Te::Add(l, r) => format!("({} + {})", te_source(l), te_source(r)),
    }
}

fn tb_source(e: &Tb) -> String {
    match e {
        Tb::IntCmp(c @ (Cmp::Equal | Cmp::NotEqual), l, r) => format!(
            "(convert<Integer>({}) {} convert<Integer>({}))",
            te_source(l),
            cmp_symbol(*c),
            te_source(r)
        ),
        Tb::IntCmp(c, l, r) => format!("({} {} {})", te_source(l), cmp_symbol(*c), te_source(r)),
        Tb::TextCmp(c, l, r) => format!("({} {} {})", text_name(*l), cmp_symbol(*c), text_name(*r)),
        Tb::And(l, r) => format!("({} and {})", tb_source(l), tb_source(r)),
        Tb::Not(x) => format!("not ({})", tb_source(x)),
    }
}

fn tb_operators(e: &Tb) -> usize {
    fn te(e: &Te) -> usize {
        match e {
            Te::SizeOf(_) => 1,
            Te::Lit(_) => 0,
            Te::Add(l, r) => 1 + te(l) + te(r),
        }
    }
    match e {
        Tb::IntCmp(_, l, r) => 1 + te(l) + te(r),
        Tb::TextCmp(..) => 1,
        Tb::And(l, r) => 1 + tb_operators(l) + tb_operators(r),
        Tb::Not(x) => 1 + tb_operators(x),
    }
}

fn te_eval(e: &Te, texts: &[String; 2]) -> i128 {
    match e {
        Te::SizeOf(i) => {
            let text = &texts[*i];
            i128::try_from(text.chars().count()).expect("small")
        }
        Te::Lit(v) => i128::from(*v),
        Te::Add(l, r) => te_eval(l, texts) + te_eval(r, texts),
    }
}

fn apply(cmp: Cmp, ordering: std::cmp::Ordering) -> bool {
    match cmp {
        Cmp::Less => ordering.is_lt(),
        Cmp::LessEqual => ordering.is_le(),
        Cmp::Equal => ordering.is_eq(),
        Cmp::NotEqual => ordering.is_ne(),
    }
}

fn tb_eval(e: &Tb, texts: &[String; 2]) -> bool {
    match e {
        Tb::IntCmp(c, l, r) => apply(*c, te_eval(l, texts).cmp(&te_eval(r, texts))),
        Tb::TextCmp(c, l, r) => {
            let pick = |i: usize| -> String { texts[i].clone() };
            // The `unicode-scalars` profile orders by scalar value, which for
            // this ASCII alphabet is byte order.
            apply(*c, pick(*l).cmp(&pick(*r)))
        }
        Tb::And(l, r) => tb_eval(l, texts) && tb_eval(r, texts),
        Tb::Not(x) => !tb_eval(x, texts),
    }
}

fn text_cases() -> BoxedStrategy<Case> {
    let cmp = proptest::sample::select(vec![Cmp::Less, Cmp::LessEqual, Cmp::Equal, Cmp::NotEqual]);
    let te = (0usize..2).prop_map(Te::SizeOf).boxed();
    let te = prop_oneof![
        te.clone(),
        (0i64..=8).prop_map(Te::Lit),
        (te.clone(), te).prop_map(|(l, r)| Te::Add(Box::new(l), Box::new(r))),
    ];
    let leaf = prop_oneof![
        (cmp.clone(), te.clone(), te).prop_map(|(c, l, r)| Tb::IntCmp(c, l, r)),
        (cmp, 0usize..2, 0usize..2).prop_map(|(c, l, r)| Tb::TextCmp(c, l, r)),
    ];
    let boolean = leaf.prop_recursive(2, 6, 2, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(l, r)| Tb::And(Box::new(l), Box::new(r))),
            inner.prop_map(|x| Tb::Not(Box::new(x))),
        ]
    });
    ("[a-c]{0,6}", "[a-c]{0,6}", boolean)
        .prop_map(|(t, u, expression)| {
            let texts = [t.clone(), u.clone()];
            Case {
                family: "text",
                source: unit(
                    None,
                    "",
                    "t: Text[0, 16; unicode-scalars], u: Text[0, 16; unicode-scalars]",
                    "Boolean",
                    &tb_source(&expression),
                ),
                kernel: false,
                args: vec![Arg::Text(t), Arg::Text(u)],
                expected: Ok(show(&Rv::Bool(tb_eval(&expression, &texts)))),
                operators: tb_operators(&expression),
                rounded: None,
            }
        })
        .boxed()
}

// ---------------------------------------------------------------- option, record, tuple

#[derive(Clone, Copy, Debug)]
enum Shape {
    /// `if present(o) then value(o) + e else e2`.
    Unwrap,
    /// `Some(e)`.
    Some,
    /// `if c then Some(e) else none`.
    MaybeSome,
    /// `R { field: e }`.
    Record,
    /// `T(e, c)`.
    Tuple,
}

fn compound_cases() -> BoxedStrategy<Case> {
    let leaf = prop_oneof![
        (-9i64..=9).prop_map(Ie::Lit),
        (0usize..2).prop_map(Ie::Param)
    ];
    let expression = leaf.prop_recursive(2, 6, 2, |inner| {
        let op = proptest::sample::select(vec![Op::Add, Op::Subtract, Op::Multiply]);
        prop_oneof![
            inner.clone().prop_map(|x| Ie::Negate(Box::new(x))),
            (op, inner.clone(), inner).prop_map(|(op, l, r)| Ie::Bin(op, Box::new(l), Box::new(r))),
        ]
    });
    let shape = proptest::sample::select(vec![
        Shape::Unwrap,
        Shape::Some,
        Shape::MaybeSome,
        Shape::Record,
        Shape::Tuple,
    ]);
    (
        shape,
        expression.clone(),
        expression,
        -9i64..=9,
        -9i64..=9,
        proptest::option::of(-9i64..=9),
        any::<bool>(),
    )
        .prop_map(|(shape, first, second, a, b, option, flag)| {
            // The parameters are `a: Int[-9, 9]`, `b: Int[-9, 9]` and the
            // optional `o: Option<Int[-9, 9]>`.
            let reference = IntRef {
                law: Law::Euclidean,
                mutation: Mutation::None,
                env: [i128::from(a), i128::from(b), 0, 1],
            };
            let value = |e: &Ie| reference.ie(e).expect("no division");
            let (first_text, second_text) = (ie_source(&first), ie_source(&second));
            // A constructor member takes the declared `Integer`, not the inferred interval.
            let widened = format!("convert<Integer>({first_text})");
            let flag_text = if flag { "(a < b)" } else { "(b <= a)" };
            let flag_value = if flag { a < b } else { b <= a };
            let (declarations, result, body, expected) = match shape {
                Shape::Unwrap => (
                    "",
                    "Integer".to_owned(),
                    format!("if present(o) then (value(o) + {first_text}) else ({second_text})"),
                    show(&Rv::Int(match option {
                        Some(v) => i128::from(v) + value(&first),
                        None => value(&second),
                    })),
                ),
                Shape::Some => (
                    "",
                    "Option<Integer>".to_owned(),
                    format!("Some({widened})"),
                    show(&Rv::Opt(Some(Box::new(Rv::Int(value(&first)))))),
                ),
                Shape::MaybeSome => (
                    "",
                    "Option<Integer>".to_owned(),
                    format!("if {flag_text} then Some({widened}) else none"),
                    show(&Rv::Opt(
                        flag_value.then(|| Box::new(Rv::Int(value(&first)))),
                    )),
                ),
                Shape::Record => (
                    "record R { field: Integer; }\n",
                    "R".to_owned(),
                    format!("R {{ field: {widened} }}"),
                    show(&Rv::Comp(vec![Rv::Int(value(&first))])),
                ),
                Shape::Tuple => (
                    "tuple T(Integer, Boolean);\n",
                    "T".to_owned(),
                    format!("T({widened}, {flag_text})"),
                    show(&Rv::Comp(vec![
                        Rv::Int(value(&first)),
                        Rv::Bool(flag_value),
                    ])),
                ),
            };
            Case {
                family: "compound",
                source: unit(
                    None,
                    declarations,
                    "a: Int[-9, 9], b: Int[-9, 9], o: Option<Int[-9, 9]>",
                    &result,
                    &body,
                ),
                kernel: false,
                args: vec![
                    Arg::Int(a),
                    Arg::Int(b),
                    Arg::Opt {
                        lower: -9,
                        upper: 9,
                        value: option,
                    },
                ],
                expected: Ok(expected),
                operators: ie_operators(&first),
                rounded: None,
            }
        })
        .boxed()
}

// ---------------------------------------------------------------- aggregates

#[derive(Clone, Copy, Debug)]
enum Aggregate {
    Sum(i64),
    SumSquares,
    Count(i64),
    Min,
    Max,
    Exists(i64),
    Forall(i64),
}

/// An aggregate over one collection, and the reference fold of its members.
#[derive(Clone, Debug)]
struct AggregateCase {
    kind: CollKind,
    aggregate: Aggregate,
    items: Vec<i64>,
    rotation: usize,
}

impl AggregateCase {
    /// `min` and `max` are `reduce`, which needs a nonempty collection.
    fn minimum(&self) -> u64 {
        u64::from(matches!(self.aggregate, Aggregate::Min | Aggregate::Max))
    }
}

fn aggregate_source(kind: CollKind, aggregate: Aggregate) -> String {
    let nonempty = matches!(aggregate, Aggregate::Min | Aggregate::Max);
    let minimum = u8::from(nonempty);
    let parameters = format!("xs: {}<Int[-9, 9]>[{minimum}, 6]", kind.name());
    let (result, body) = match aggregate {
        Aggregate::Sum(k) => ("A", format!("sum<A>(x in xs: x + ({k}))")),
        Aggregate::SumSquares => ("A", "sum<A>(x in xs: x * x)".to_owned()),
        Aggregate::Count(k) => ("A", format!("count<A>(x in xs: x < ({k}))")),
        Aggregate::Min => (
            "A",
            "reduce<A>(m, x in xs: if x < m then x else m)".to_owned(),
        ),
        Aggregate::Max => (
            "A",
            "reduce<A>(m, x in xs: if m < x then x else m)".to_owned(),
        ),
        Aggregate::Exists(k) => ("Boolean", format!("exists(x in xs: x < ({k}))")),
        Aggregate::Forall(k) => ("Boolean", format!("forall(x in xs: x < ({k}))")),
    };
    let alias = if nonempty {
        "type A = Int[-9, 9];\n"
    } else {
        "type A = Integer;\n"
    };
    unit(None, alias, &parameters, result, &body)
}

/// The fold of `items` (the reference), independent of the order given.
fn aggregate_reference(aggregate: Aggregate, items: &[i64]) -> String {
    let wide = || items.iter().map(|v| i128::from(*v));
    match aggregate {
        Aggregate::Sum(k) => show(&Rv::Int(wide().map(|v| v + i128::from(k)).sum())),
        Aggregate::SumSquares => show(&Rv::Int(wide().map(|v| v * v).sum())),
        Aggregate::Count(k) => show(&Rv::Int(
            wide().filter(|v| *v < i128::from(k)).count() as i128
        )),
        Aggregate::Min => show(&Rv::Int(wide().min().expect("nonempty"))),
        Aggregate::Max => show(&Rv::Int(wide().max().expect("nonempty"))),
        Aggregate::Exists(k) => show(&Rv::Bool(wide().any(|v| v < i128::from(k)))),
        Aggregate::Forall(k) => show(&Rv::Bool(wide().all(|v| v < i128::from(k)))),
    }
}

fn aggregate_cases() -> BoxedStrategy<AggregateCase> {
    let kind = proptest::sample::select(vec![CollKind::Sequence, CollKind::Set, CollKind::Bag]);
    let aggregate = prop_oneof![
        (-5i64..=5).prop_map(Aggregate::Sum),
        Just(Aggregate::SumSquares),
        (-9i64..=9).prop_map(Aggregate::Count),
        Just(Aggregate::Min),
        Just(Aggregate::Max),
        (-9i64..=9).prop_map(Aggregate::Exists),
        (-9i64..=9).prop_map(Aggregate::Forall),
    ];
    (
        kind,
        aggregate,
        proptest::collection::btree_set(-9i64..=9, 1..=6),
        proptest::collection::vec(-9i64..=9, 1..=6),
        0usize..6,
    )
        .prop_map(|(kind, aggregate, unique, bag, rotation)| {
            // `reduce` over a `Set` or `Bag` is refused by the checker
            // (`operator-ineligible`), so `min` and `max` are Sequence-only.
            let kind = if matches!(aggregate, Aggregate::Min | Aggregate::Max) {
                CollKind::Sequence
            } else {
                kind
            };
            AggregateCase {
                kind,
                aggregate,
                items: if kind == CollKind::Set {
                    unique.into_iter().collect()
                } else {
                    bag
                },
                rotation,
            }
        })
        .boxed()
}

// ---------------------------------------------------------------- unions of families

fn well_typed_cases() -> BoxedStrategy<Case> {
    prop_oneof![
        3 => int_cases(false, Mutation::None),
        2 => rat_cases(false),
        2 => dec_cases(false),
        1 => text_cases(),
        1 => compound_cases(),
    ]
    .boxed()
}

fn aggregate_as_case(case: &AggregateCase) -> Case {
    Case {
        family: "aggregate",
        source: aggregate_source(case.kind, case.aggregate),
        kernel: false,
        args: vec![Arg::Coll {
            kind: case.kind,
            minimum: case.minimum(),
            items: case.items.clone(),
        }],
        expected: Ok(aggregate_reference(case.aggregate, &case.items)),
        operators: 1,
        rounded: None,
    }
}

fn refusing_cases() -> BoxedStrategy<Case> {
    prop_oneof![
        3 => int_cases(true, Mutation::None),
        2 => rat_cases(true),
        3 => dec_cases(true),
        2 => well_typed_cases(),
    ]
    .boxed()
}

/// A source for P1: the generated families and the shared generators.
fn any_source() -> BoxedStrategy<String> {
    prop_oneof![
        3 => well_typed_cases().prop_map(|c| c.source),
        1 => aggregate_cases().prop_map(|c| aggregate_source(c.kind, c.aggregate)),
        2 => prop::value::expressions().prop_map(|e| e.source()),
    ]
    .boxed()
}

// ---------------------------------------------------------------- P1

/// Production tree: each node's production and its significant token
/// spellings, depth first, ignoring whitespace and comments.
fn production_tree(parsed: &fsl_cst::ParsedSource) -> String {
    use fsl_cst::{CstElement, TokenKind};
    fn walk(parsed: &fsl_cst::ParsedSource, index: usize, out: &mut String) {
        let cst = parsed.cst();
        let node = &cst.nodes()[index];
        out.push_str(&format!("({:?}", node.production()));
        for child in node.children() {
            match child {
                CstElement::Node(i) => walk(parsed, *i, out),
                CstElement::Token(i) => {
                    let token = &cst.tokens()[*i];
                    if !matches!(token.kind(), TokenKind::Whitespace | TokenKind::Comment) {
                        out.push_str(&format!(" {:?}", String::from_utf8_lossy(token.spelling())));
                    }
                }
            }
        }
        out.push(')');
    }
    let mut out = String::new();
    walk(parsed, 0, &mut out);
    out
}

fn parse(source: &str) -> Result<fsl_cst::ParsedSource, String> {
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "value-properties"),
        "value.native",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .map_err(|d| format!("{d:?}"))?;
    if parsed.is_admissible() {
        Ok(parsed)
    } else {
        Err(format!("{:?}", parsed.diagnostics()))
    }
}

#[trace("FR-003-AC-1")]
#[test]
fn p1_format_round_trip_keeps_the_tree() {
    prop::run(
        "p1_format_round_trip_keeps_the_tree",
        prop::config(1000),
        &any_source(),
        |source| {
            let parsed = parse(&source).map_err(TestCaseError::fail)?;
            let formatted = fsl_cst::format::format(&parsed)
                .map_err(|e| TestCaseError::fail(format!("{e}\n{source}")))?;
            let reparsed = parse(&formatted)
                .map_err(|e| TestCaseError::fail(format!("{e}\nformatted:\n{formatted}")))?;
            proptest::prop_assert_eq!(
                production_tree(&parsed),
                production_tree(&reparsed),
                "{}",
                source
            );
            let again = fsl_cst::format::format(&reparsed)
                .map_err(|e| TestCaseError::fail(format!("{e}")))?;
            proptest::prop_assert_eq!(&formatted, &again);
            Ok(())
        },
    )
    .unwrap();
}

// ---------------------------------------------------------------- P2

#[trace("FR-091-AC-13")]
#[test]
fn p2_generated_well_typed_expressions_check() {
    let sources = prop_oneof![
        4 => well_typed_cases().prop_map(|c| c.source),
        1 => aggregate_cases().prop_map(|c| aggregate_source(c.kind, c.aggregate)),
        2 => prop::value::expressions().prop_map(|e| e.source()),
    ];
    prop::run(
        "p2_generated_well_typed_expressions_check",
        prop::config(1000),
        &sources,
        |source| {
            try_compile(&source, CheckMode::Linked)
                .map(drop)
                .map_err(|e| TestCaseError::fail(format!("{e}\n{source}")))
        },
    )
    .unwrap();
}

// ---------------------------------------------------------------- P3

#[trace("FR-091-AC-13")]
#[test]
fn p3_evaluator_matches_the_exact_reference() {
    let families = RefCell::new(BTreeMap::<&'static str, usize>::new());
    prop::run(
        "p3_evaluator_matches_the_exact_reference",
        prop::config(1000),
        &prop_oneof![
            8 => well_typed_cases(),
            2 => aggregate_cases().prop_map(|c| aggregate_as_case(&c)),
        ],
        |case| {
            // A reference refusal is P4's subject.
            if case.expected.is_ok() {
                *families.borrow_mut().entry(case.family).or_default() += 1;
                agree(&case).map(drop).map_err(TestCaseError::fail)?;
            }
            Ok(())
        },
    )
    .unwrap();
    let families = families.into_inner();
    for family in [
        "integer",
        "rational",
        "decimal",
        "text",
        "compound",
        "aggregate",
    ] {
        assert!(
            families.get(family).copied().unwrap_or(0) > 20,
            "family {family} is under-sampled: {families:?}"
        );
    }
}

// ---------------------------------------------------------------- P4

#[trace("FR-495-AC-3", "FR-495-AC-17")]
#[test]
fn p4_refusals_agree_with_the_reference() {
    let seen = RefCell::new(BTreeMap::<Refuse, usize>::new());
    let rounding = RefCell::new(BTreeMap::<&'static str, usize>::new());
    prop::run(
        "p4_refusals_agree_with_the_reference",
        prop::config(1000),
        &refusing_cases(),
        |case| {
            let outcome = agree(&case).map_err(TestCaseError::fail)?;
            if let Err(refusal) = outcome {
                *seen.borrow_mut().entry(refusal).or_default() += 1;
            }
            if let Some(mode) = case.rounded {
                *rounding.borrow_mut().entry(mode).or_default() += 1;
            }
            Ok(())
        },
    )
    .unwrap();
    // Each rounding mode meets a `*` or `/` that discards a nonzero digit (for
    // `exact` that is the refusal), so no mode is checked only on exact results.
    let rounding = rounding.into_inner();
    for mode in MODES {
        assert!(
            rounding.get(mode).copied().unwrap_or(0) >= 5,
            "mode {mode} met a rounding operation too rarely: {rounding:?}"
        );
    }
    let seen = seen.into_inner();
    for refusal in [
        Refuse::DivisionByZero,
        Refuse::IntegerDomain,
        Refuse::DecimalInexact,
        Refuse::DecimalDomain,
        Refuse::RationalDomain,
    ] {
        assert!(
            seen.get(&refusal).copied().unwrap_or(0) > 10,
            "{refusal:?} is under-sampled: {seen:?}"
        );
    }
}

// ---------------------------------------------------------------- P5

#[trace("FR-701-AC-1")]
#[test]
fn p5_aggregates_equal_the_reference_fold_in_any_order() {
    prop::run(
        "p5_aggregates_equal_the_reference_fold_in_any_order",
        prop::config(1000),
        &aggregate_cases(),
        |case| {
            let source = aggregate_source(case.kind, case.aggregate);
            let expected = aggregate_reference(case.aggregate, &case.items);
            let package = try_compile(&source, CheckMode::Linked)
                .map_err(|e| TestCaseError::fail(format!("{e}\n{source}")))?;
            let mut reversed = case.items.clone();
            reversed.reverse();
            let mut rotated = case.items.clone();
            rotated.rotate_left(case.rotation % case.items.len());
            for order in [case.items.clone(), reversed, rotated] {
                let args = [Arg::Coll {
                    kind: case.kind,
                    minimum: case.minimum(),
                    items: order.clone(),
                }];
                proptest::prop_assert_eq!(
                    evaluate(&package, &args),
                    Ok(expected.clone()),
                    "order {:?}\n{}",
                    order,
                    source
                );
            }
            Ok(())
        },
    )
    .unwrap();
}

// ---------------------------------------------------------------- negatives

#[test]
fn reference_mutation_is_caught() {
    let mut config = prop::config(1000);
    config.failure_persistence = None;
    config.max_shrink_iters = 20_000;
    let outcome = prop::run(
        "reference_mutation_is_caught",
        config,
        &int_cases(false, Mutation::AddIsSubtract),
        |case| agree(&case).map(drop).map_err(TestCaseError::fail),
    );
    let Err(TestError::Fail(reason, minimal)) = outcome else {
        panic!("a reference that turns `+` into `-` must fail P3");
    };
    assert_eq!(
        minimal.operators,
        1,
        "the failure shrinks to a one-operator source: {reason}\n{}",
        describe(&minimal)
    );
    assert!(minimal.source.contains(" + "), "{}", minimal.source);
    assert!(reason.message().contains("reference"), "{reason}");
}

#[trace("FR-495-AC-3")]
#[test]
fn near_miss_is_never_evaluated_to_a_value() {
    // The shared near misses: refused at check with the authored code and
    // cause, and no value under `Kernel`.
    let kernel_runs = Cell::new(0u32);
    prop::run(
        "near_miss_is_never_evaluated_to_a_value",
        prop::config(256),
        &prop::near_miss::near_misses(),
        |miss| {
            let refusals = match compile(&miss.source, CheckMode::Linked) {
                Err(Failure::Refused(refusals)) => refusals,
                Err(other) => {
                    return Err(TestCaseError::fail(format!(
                        "refused before the checker: {}\n{}",
                        other.text(),
                        miss.source
                    )))
                }
                Ok(_) => return Err(TestCaseError::fail(format!("accepted:\n{}", miss.source))),
            };
            for refusal in &refusals {
                proptest::prop_assert_eq!(
                    (refusal.cause.code().as_str(), refusal.cause.cause()),
                    (miss.code, Some(miss.cause)),
                    "{}",
                    miss.source
                );
            }
            if let Ok(package) = compile(&miss.source, CheckMode::Kernel) {
                kernel_runs.set(kernel_runs.get() + 1);
                proptest::prop_assert!(
                    evaluate(&package, &[]).is_err(),
                    "a value for {}",
                    miss.source
                );
            }
            Ok(())
        },
    )
    .unwrap();
    // `OutOfRange` and `DivisionByZero` (two of the three families) compile
    // under `Kernel`; the evaluation half must have run for them.
    assert!(
        kernel_runs.get() >= 100,
        "only {} near misses reached the evaluator",
        kernel_runs.get()
    );
    // A divisor range that spans zero (`mod`, `rem` and `div`): `Linked`
    // refuses `unproved-nonzero`, and under `Kernel` the zero divisor has no value.
    let kernel_runs = Cell::new(0u32);
    prop::run(
        "near_miss_zero_spanning_divisor",
        prop::config(256),
        &(
            law_strategy(),
            proptest::sample::select(vec!["div", "rem", "mod"]),
            -20i64..=20,
            -9i64..=0,
            0i64..=9,
        ),
        |(law, op, dividend, lower, upper)| {
            let source = unit(
                Some(law),
                "",
                &format!("a: Int[-20, 20], b: Int[{lower}, {upper}]"),
                "Integer",
                &format!("a {op} b"),
            );
            match compile(&source, CheckMode::Linked) {
                Err(Failure::Refused(refusals)) => {
                    proptest::prop_assert!(!refusals.is_empty(), "{source}");
                    for refusal in &refusals {
                        proptest::prop_assert_eq!(
                            (refusal.cause.code().as_str(), refusal.cause.cause()),
                            ("undefined_expression", Some("unproved-nonzero")),
                            "{}",
                            source
                        );
                    }
                }
                Err(other) => proptest::prop_assert!(false, "{}\n{source}", other.text()),
                Ok(_) => proptest::prop_assert!(false, "accepted:\n{source}"),
            }
            let package = try_compile(&source, CheckMode::Kernel)
                .map_err(|e| TestCaseError::fail(format!("{e}\n{source}")))?;
            kernel_runs.set(kernel_runs.get() + 1);
            proptest::prop_assert_eq!(
                evaluate(&package, &[Arg::Int(dividend), Arg::Int(0)]),
                Err(Refuse::DivisionByZero),
                "{}",
                source
            );
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        kernel_runs.get(),
        256,
        "every zero-spanning case is evaluated"
    );
}
