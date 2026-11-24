// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Recursive clock readings": the unfolding of a measured recursive
//! clock-reading declaration over constants, and the readings it refuses.

use super::*;

/// The unit with `declarations` and a precondition `call` on `reply`.
fn unit(declarations: &str, call: &str) -> String {
    format!(
        "{UNITS}time Config dense unit ms;\n{declarations}\n\
         pre L using v on Config::Call::reply {{ {call} }}\n"
    )
}

const LOOP: &str = "function loop using v(c: Clock, n: Integer): Boolean pure decreases(n) { \
                    if n = 0 then c <= 3 ms else loop(c, n - 1) }";

/// The one convex part of the guard of `call`, as its clock constraints.
fn constraints_of(declarations: &str, call: &str) -> Vec<(String, ClockOp, Rational)> {
    let graph = check(&rpc_document(&["x", "y"]), &unit(declarations, call))
        .unwrap_or_else(|refusals| panic!("{call} unfolds: {refusals:?}"));
    let [guard] = timed(&graph).guards() else {
        panic!("one guard");
    };
    let [part] = guard.parts() else {
        panic!("one convex part");
    };
    assert!(part.data().is_none(), "no data literal for {call}");
    let model = timed(&graph);
    part.clocks()
        .iter()
        .map(|constraint| {
            (
                model
                    .clock(constraint.clock())
                    .expect("a clock")
                    .name()
                    .to_owned(),
                constraint.op(),
                constraint.bound().clone(),
            )
        })
        .collect()
}

fn x_at_most_3() -> (String, ClockOp, Rational) {
    ("x".to_owned(), ClockOp::LessOrEqual, exact(3, 1))
}

/// An `and` operand that recurses is read without nesting: a small constant
/// unfolds into one part, and a constant of a million refuses at the node
/// limit instead of overflowing the stack.
#[trace("FR-230-AC-68")]
#[test]
fn a_non_tail_recursive_reading_unfolds_and_a_deep_one_refuses() {
    let declarations = "function loop using v(c: Clock, n: Integer): Boolean pure decreases(n) { \
         if n = 0 then c <= 3 ms else (c <= 3 ms and loop(c, n - 1)) }";
    for constraint in constraints_of(declarations, "loop(self.x, 2)") {
        assert_eq!(constraint, x_at_most_3());
    }
    let refusals = check(
        &rpc_document(&["x", "y"]),
        &unit(declarations, "loop(self.x, 1000000)"),
    )
    .expect_err("the deep unfolding refuses");
    assert!(
        matches!(
            refusals.as_slice(),
            [refusal] if matches!(refusal.cause.as_ref(), CheckCause::ResourceExhausted(limit) if limit.kind == CheckingLimitKind::Nodes)
        ),
        "{refusals:?}"
    );
}

/// A measured declaration whose result is a clock place unfolds the same
/// way: `pick(self.x, 3)` is `self.x`, and a non-constant argument names the
/// cycle.
#[trace("FR-230-AC-68")]
#[test]
fn a_measured_recursive_place_expression_unfolds() {
    let declarations = "function pick using v(c: Clock, n: Integer): Clock pure decreases(n) { \
         if n = 0 then c else pick(c, n - 1) }";
    assert_eq!(
        constraints_of(declarations, "pick(self.x, 3) >= 1 ms"),
        [("x".to_owned(), ClockOp::GreaterOrEqual, exact(1, 1))]
    );
    let (cause, text) = refused(
        &rpc_document(&["x", "y"]),
        &unit(declarations, "pick(self.x, self.n) >= 1 ms"),
    );
    assert_eq!(text, "pick(self.x, self.n)");
    assert!(
        matches!(&cause, TemporalCause::ClockReadingCycle { cycle, .. }
            if cycle == &["pick".to_owned(), "pick".to_owned()]),
        "{cause:?}"
    );
}

/// Each form of constant FR-230 evaluates unfolds: `mod`, `div` and `rem`
/// of non-negative operands and a `case` over a union value.
#[trace("FR-230-AC-68")]
#[test]
fn constants_over_every_form_unfold() {
    for call in [
        "loop(self.x, 7 mod 4)",
        "loop(self.x, 7 div 2)",
        "loop(self.x, 8 rem 5)",
    ] {
        assert_eq!(constraints_of(LOOP, call), [x_at_most_3()], "{call}");
    }
    let declarations = "union Shape { Empty, Circle(Integer) }\n\
         function pick using v(c: Clock, s: Shape): Boolean pure { \
             case s { Empty: c <= 1 ms; Circle(r): if r = 0 then c <= 2 ms else c <= 3 ms; } }";
    for (call, bound) in [
        ("pick(self.x, Shape::Empty)", 1),
        ("pick(self.x, Shape::Circle(0))", 2),
        ("pick(self.x, Shape::Circle(5))", 3),
    ] {
        assert_eq!(
            constraints_of(declarations, call),
            [("x".to_owned(), ClockOp::LessOrEqual, exact(bound, 1))],
            "{call}"
        );
    }
}

/// An integer constant past the rational-bit ceiling is that limit, not a
/// non-constant read.
#[trace("FR-230-AC-68")]
#[test]
fn a_constant_past_the_rational_bits_refuses_that_limit() {
    let refusals = admit_and_assemble_with_profiles(
        &rpc_document(&["x", "y"]),
        TEMPORAL,
        &unit(LOOP, "loop(self.x, 100000000000000000000)"),
    )
    .expect("S2 builds every timed form")
    .with_unwritten_timed_members()
    .check(
        CheckMode::Linked,
        filament_semantic_value::checking::CheckingLimits::default().with_rational_bits(64),
        &crate::reward_evaluator::ZeroRewardEvaluator,
    )
    .expect_err("the constant is too wide");
    assert!(
        matches!(
            refusals.as_slice(),
            [refusal] if matches!(refusal.cause.as_ref(), CheckCause::ResourceExhausted(limit) if limit.kind == CheckingLimitKind::RationalBits)
        ),
        "{refusals:?}"
    );
}
