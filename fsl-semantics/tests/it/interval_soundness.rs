// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2056 P3 and P4: the checker's integer interval rules against exact
//! interval arithmetic and against enumeration of the parameter domains.
//!
//! "Evaluating `e`" here is done by a small exact `i128` reference evaluator in
//! this file, because `fsl-semantics` cannot see `fsl-eval`. The real evaluator
//! is checked against the checker's node types by P1 and P2 in
//! `fsl-eval/tests/it/typed_value_differential.rs`.
//!
//! Every `+ - *` tree is read-once (each parameter occurs at most once), so
//! interval arithmetic is the exact hull of the tree and the refusal half of
//! P3 is decidable without a relational analysis the checker does not claim.
//!
//! The suite has no generator exclusions: `mod` (QSL-2002) and `count<N>`
//! (QSL-2003) are both fixed on main, so P4 runs for both.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_forms::build_unit;
use fsl_foundation::SourceIdentity;
use fsl_proptest_policy as prop;
use fsl_semantics::check::{CheckCause, CheckRefusal, PackageDeclarations};
use fsl_semantics::model::intake::SelectedModels;
use ix_trace_rs::trace;
use proptest::prelude::*;
use proptest::test_runner::{TestCaseError, TestError};

/// Generator exclusions and the ticket each waits on. QSL-2002 and QSL-2003
/// are Done, so there are none.
const EXCLUSIONS: &[&str] = &[];

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

/// What the checker said about one source.
#[derive(Debug, Eq, PartialEq)]
enum Verdict {
    Accepted,
    Refused(String),
}

fn check(declarations: &str) -> Verdict {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = fsl_cst::parse(
        SourceIdentity::new("test", "interval-soundness"),
        "interval.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let result = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: assembly: {refusal:?}"))
    .check(
        CheckMode::Linked,
        CheckingLimits::default(),
        &crate::reward_evaluator::ZeroRewardEvaluator,
    );
    match result {
        Ok(_) => Verdict::Accepted,
        Err(refusals) => Verdict::Refused(cause_of(&refusals)),
    }
}

fn cause_of(refusals: &[CheckRefusal]) -> String {
    let first = refusals.first().expect("a refusal");
    match &*first.cause {
        CheckCause::InternalFault(_) => format!("internal fault: {refusals:?}"),
        other => other
            .cause()
            .map_or_else(|| format!("{:?}", other.code()), str::to_owned),
    }
}

// ------------------------------------------------------------ the tree

#[derive(Clone, Copy, Debug)]
enum Op {
    Add,
    Sub,
    Mul,
}

/// A read-once arithmetic tree: `Param` leaves are numbered left to right.
#[derive(Clone, Debug)]
enum Tree {
    Param,
    Lit(i64),
    Bin(Op, Box<Tree>, Box<Tree>),
}

fn leaves() -> BoxedStrategy<Tree> {
    prop_oneof![3 => Just(Tree::Param), 1 => (-3i64..=3).prop_map(Tree::Lit)].boxed()
}

fn ops() -> BoxedStrategy<Op> {
    prop_oneof![Just(Op::Add), Just(Op::Sub), Just(Op::Mul)].boxed()
}

/// A left-nested chain whose right operands are leaves or one further
/// operator over two leaves. The chain shrinks by dropping steps and by
/// replacing an operand with a leaf, so a failure shrinks to a small tree.
fn trees() -> BoxedStrategy<Tree> {
    let operand = prop_oneof![
        leaves(),
        (ops(), leaves(), leaves()).prop_map(|(op, l, r)| Tree::Bin(op, Box::new(l), Box::new(r))),
    ];
    (leaves(), proptest::collection::vec((ops(), operand), 1..=3))
        .prop_map(|(base, steps)| {
            steps.into_iter().fold(base, |tree, (op, operand)| {
                Tree::Bin(op, Box::new(tree), Box::new(operand))
            })
        })
        .boxed()
}

impl Tree {
    fn params(&self) -> usize {
        match self {
            Tree::Param => 1,
            Tree::Lit(_) => 0,
            Tree::Bin(_, l, r) => l.params() + r.params(),
        }
    }
    fn operators(&self) -> usize {
        match self {
            Tree::Bin(_, l, r) => 1 + l.operators() + r.operators(),
            _ => 0,
        }
    }
    fn source(&self, next: &mut usize) -> String {
        match self {
            Tree::Param => {
                *next += 1;
                format!("p{}", *next - 1)
            }
            Tree::Lit(k) => format!("({k})"),
            Tree::Bin(op, l, r) => {
                let (l, r) = (l.source(next), r.source(next));
                let symbol = match op {
                    Op::Add => "+",
                    Op::Sub => "-",
                    Op::Mul => "*",
                };
                format!("({l} {symbol} {r})")
            }
        }
    }
    /// Interval arithmetic; `weak` lowers the upper bound of `+` by one.
    fn interval(&self, params: &[(i64, i64)], next: &mut usize, weak: bool) -> (i64, i64) {
        match self {
            Tree::Param => {
                *next += 1;
                params[*next - 1]
            }
            Tree::Lit(k) => (*k, *k),
            Tree::Bin(op, l, r) => {
                let (a, b) = l.interval(params, next, weak);
                let (c, d) = r.interval(params, next, weak);
                match op {
                    Op::Add => (a + c, b + d - i64::from(weak)),
                    Op::Sub => (a - d, b - c),
                    Op::Mul => {
                        let corners = [a * c, a * d, b * c, b * d];
                        (
                            *corners.iter().min().expect("corners"),
                            *corners.iter().max().expect("corners"),
                        )
                    }
                }
            }
        }
    }
    /// The reference value under one assignment.
    fn eval(&self, values: &[i64], next: &mut usize) -> i64 {
        match self {
            Tree::Param => {
                *next += 1;
                values[*next - 1]
            }
            Tree::Lit(k) => *k,
            Tree::Bin(op, l, r) => {
                let (a, b) = (l.eval(values, next), r.eval(values, next));
                match op {
                    Op::Add => a + b,
                    Op::Sub => a - b,
                    Op::Mul => a * b,
                }
            }
        }
    }
}

/// One generated function: tree, parameter domains, and the declared result.
#[derive(Clone, Debug)]
struct Case {
    tree: Tree,
    /// Parameter domains `[-a, b]`, shrinking toward `[0, 0]`.
    domains: Vec<(i64, i64)>,
    /// Offsets of the declared interval from the exact hull.
    slack: (i64, i64),
}

impl Case {
    fn domains(&self) -> &[(i64, i64)] {
        &self.domains[..self.tree.params()]
    }
    fn hull(&self, weak: bool) -> (i64, i64) {
        self.tree.interval(self.domains(), &mut 0, weak)
    }
    fn declared(&self) -> (i64, i64) {
        let (lo, hi) = self.hull(false);
        let a = lo - self.slack.0;
        (a, (hi + self.slack.1).max(a))
    }
    fn source(&self) -> String {
        let parameters = self
            .domains()
            .iter()
            .enumerate()
            .map(|(i, (lo, hi))| format!("p{i}: Int[{lo}, {hi}]"))
            .collect::<Vec<_>>()
            .join(", ");
        let (a, b) = self.declared();
        format!(
            "function f using v({parameters}): Int[{a}, {b}] pure {{ {} }}",
            self.tree.source(&mut 0)
        )
    }
    /// Every assignment of the parameter domains.
    fn assignments(&self) -> Vec<Vec<i64>> {
        let mut all = vec![Vec::new()];
        for (lo, hi) in self.domains() {
            all = all
                .into_iter()
                .flat_map(|prefix| {
                    (*lo..=*hi).map(move |v| {
                        let mut next = prefix.clone();
                        next.push(v);
                        next
                    })
                })
                .collect();
        }
        all
    }
    fn contained(&self, hull: (i64, i64)) -> bool {
        let (a, b) = self.declared();
        a <= hull.0 && hull.1 <= b
    }
}

fn cases() -> BoxedStrategy<Case> {
    (
        trees(),
        proptest::collection::vec((0i64..=3, 0i64..=3), 8),
        (-3i64..=3, -3i64..=3),
    )
        .prop_map(|(tree, bounds, slack)| Case {
            tree,
            domains: bounds.into_iter().map(|(a, b)| (-a, b)).collect(),
            slack,
        })
        .boxed()
}

fn config() -> proptest::test_runner::Config {
    prop::config(256)
}

/// Accepted means every assignment lands inside the declared interval.
fn accepted_stays_inside(case: &Case, accepts: &dyn Fn(&Case) -> Verdict) -> Result<(), String> {
    if accepts(case) != Verdict::Accepted {
        return Ok(());
    }
    let (a, b) = case.declared();
    for values in case.assignments() {
        let value = case.tree.eval(&values, &mut 0);
        if value < a || value > b {
            return Err(format!(
                "accepted, but {values:?} gives {value} outside [{a}, {b}]\n{}",
                case.source()
            ));
        }
    }
    Ok(())
}

fn real(case: &Case) -> Verdict {
    check(&case.source())
}

/// QSL-2056 P3 (FR-495-AC-19): a function the checker accepts never leaves its declared interval.
#[trace("FR-495-AC-19")]
#[test]
fn p3_accepted_functions_stay_inside_the_declared_interval() {
    let accepted = std::cell::Cell::new(0u32);
    prop::run(
        "p3_accepted_functions_stay_inside_the_declared_interval",
        config(),
        &cases(),
        |case| {
            if real(&case) == Verdict::Accepted {
                accepted.set(accepted.get() + 1);
            }
            accepted_stays_inside(&case, &real).map_err(TestCaseError::fail)
        },
    )
    .unwrap();
    assert!(
        accepted.get() >= 40,
        "only {} accepted cases",
        accepted.get()
    );
}

/// QSL-2056 P3 (FR-495-AC-19): a refusal `unproved-range` is real (the exact interval escapes the
/// declared one), and an exact fit is accepted.
#[trace("FR-495-AC-19")]
#[test]
fn p3_unproved_range_refusals_are_real() {
    let refused = std::cell::Cell::new(0u32);
    prop::run(
        "p3_unproved_range_refusals_are_real",
        config(),
        &cases(),
        |case| {
            let hull = case.hull(false);
            match real(&case) {
                Verdict::Accepted if !case.contained(hull) => Err(TestCaseError::fail(format!(
                    "accepted although the exact interval {hull:?} escapes\n{}",
                    case.source()
                ))),
                Verdict::Accepted => Ok(()),
                Verdict::Refused(cause) if cause == "unproved-range" => {
                    refused.set(refused.get() + 1);
                    if case.contained(hull) {
                        Err(TestCaseError::fail(format!(
                            "refused unproved-range although the exact interval {hull:?} fits\n{}",
                            case.source()
                        )))
                    } else {
                        Ok(())
                    }
                }
                Verdict::Refused(other) => Err(TestCaseError::fail(format!(
                    "refused {other}\n{}",
                    case.source()
                ))),
            }
        },
    )
    .unwrap();
    assert!(refused.get() >= 40, "only {} refusals", refused.get());
}

/// Negative: a checker whose `+` upper bound is one too low (modelled by the
/// weakened rule below, accepting by that rule) is caught by P3 and the
/// failure shrinks to one operator.
#[test]
fn weakened_rule_is_caught() {
    let weak = |case: &Case| {
        let (lo, hi) = case.hull(true);
        let (a, b) = case.declared();
        if a <= lo && hi <= b {
            Verdict::Accepted
        } else {
            Verdict::Refused("unproved-range".into())
        }
    };
    let mut config = config();
    config.failure_persistence = None;
    config.max_shrink_iters = 20_000;
    let outcome = prop::run("weakened_rule_is_caught", config, &cases(), |case| {
        accepted_stays_inside(&case, &weak).map_err(TestCaseError::fail)
    });
    let Err(TestError::Fail(reason, minimal)) = outcome else {
        panic!("a `+` rule one too low must fail P3");
    };
    assert!(
        minimal.tree.operators() <= 3,
        "{reason}\n{}",
        minimal.source()
    );
    assert!(minimal.source().contains(" + "), "{}", minimal.source());
}

#[test]
fn known_exclusions_are_none() {
    // QSL-2002 (`mod`) and QSL-2003 (`count<N>`) are Done: nothing is excluded.
    assert!(EXCLUSIONS.is_empty(), "{EXCLUSIONS:?}");
}

// ------------------------------------------------------------ P4: mod

fn zero_free() -> BoxedStrategy<(i64, i64)> {
    prop_oneof![
        (1i64..=6, 0i64..=5).prop_map(|(l, w)| (l, l + w)),
        (1i64..=6, 0i64..=5).prop_map(|(l, w)| (-(l + w), -l))
    ]
    .boxed()
}

/// Exact hull of `x mod d` (Euclidean) over both domains.
fn mod_hull(x: (i64, i64), d: (i64, i64)) -> (i64, i64) {
    let mut hull = (i64::MAX, i64::MIN);
    for a in x.0..=x.1 {
        for b in d.0..=d.1 {
            let r = a.rem_euclid(b);
            hull = (hull.0.min(r), hull.1.max(r));
        }
    }
    hull
}

fn mod_source(x: (i64, i64), d: (i64, i64), result: (i64, i64)) -> String {
    format!(
        "function f using v(x: Int[{}, {}], d: Int[{}, {}]): Int[{}, {}] pure {{ x mod d }}",
        x.0, x.1, d.0, d.1, result.0, result.1
    )
}

/// QSL-2056 P4 (FR-495-AC-17, AC-19): with a zero-free divisor the checked `mod` interval is
/// exactly the smallest one: the exact hull is accepted, and the hull with
/// either endpoint pulled in is refused `unproved-range`.
#[trace("FR-495-AC-17", "FR-495-AC-19")]
#[test]
fn p4_mod_with_a_zero_free_divisor_stays_in_range() {
    prop::run(
        "p4_mod_with_a_zero_free_divisor_stays_in_range",
        config(),
        &((-8i64..=0, 0i64..=8), zero_free()),
        |((xl, xh), d)| {
            let x = (xl, xh);
            let hull = mod_hull(x, d);
            let fits = check(&mod_source(x, d, hull));
            prop_assert_eq!(fits, Verdict::Accepted, "{}", mod_source(x, d, hull));
            if hull.0 < hull.1 {
                for narrowed in [(hull.0 + 1, hull.1), (hull.0, hull.1 - 1)] {
                    let source = mod_source(x, d, narrowed);
                    prop_assert_eq!(
                        check(&source),
                        Verdict::Refused("unproved-range".into()),
                        "{}",
                        source
                    );
                }
            }
            Ok(())
        },
    )
    .unwrap();
}

/// FR-495-AC-19's named narrow case: `Int[0, 2] mod Int[5, 6]` is `Int[0, 2]`.
#[trace("FR-495-AC-19")]
#[test]
fn p4_mod_below_every_divisor_is_not_widened() {
    assert_eq!(
        check(&mod_source((0, 2), (5, 6), (0, 2))),
        Verdict::Accepted
    );
    assert_eq!(
        check(&mod_source((0, 2), (5, 6), (0, 1))),
        Verdict::Refused("unproved-range".into())
    );
}

/// QSL-2056 P4 (FR-495-AC-3, AC-17): a divisor range containing zero is refused `unproved-nonzero`.
#[trace("FR-495-AC-3", "FR-495-AC-17")]
#[test]
fn p4_mod_with_a_zero_divisor_is_refused_at_check() {
    prop::run(
        "p4_mod_with_a_zero_divisor_is_refused_at_check",
        config(),
        &((-8i64..=0, 0i64..=8), (-6i64..=0, 0i64..=6)),
        |((xl, xh), d)| {
            let source = mod_source((xl, xh), d, (-100, 100));
            prop_assert_eq!(
                check(&source),
                Verdict::Refused("unproved-nonzero".into()),
                "{}",
                source
            );
            Ok(())
        },
    )
    .unwrap();
}

// ------------------------------------------------------------ P4: count

/// QSL-2056 P4 (FR-766-AC-1): `count<N>` over a source whose size can exceed `N`'s upper bound is
/// refused `unproved-range`; over a source that cannot, it is accepted.
#[trace("FR-766-AC-1")]
#[test]
fn p4_count_over_the_bound_is_refused_at_check() {
    prop::run(
        "p4_count_over_the_bound_is_refused_at_check",
        config(),
        &(
            proptest::sample::select(vec!["Sequence", "Set", "Bag"]),
            0u64..=6,
            0i64..=6,
        ),
        |(kind, size, bound)| {
            let source = format!(
                "type A = Int[0, {bound}];\nfunction f using v(xs: {kind}<Int[-3, 3]>[0, {size}]): A pure {{ count<A>(x in xs: x < 1) }}"
            );
            let expected = if i64::try_from(size).expect("small") > bound {
                Verdict::Refused("unproved-range".into())
            } else {
                Verdict::Accepted
            };
            prop_assert_eq!(check(&source), expected, "{}", source);
            Ok(())
        },
    )
    .unwrap();
}
