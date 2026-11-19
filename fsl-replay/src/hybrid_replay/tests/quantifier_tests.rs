// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 "Exact replay": real quantifier elimination over the post-jump
//! variable a reset leaves free, at rational and at irrational instants, and
//! the enabledness it decides in a replayed lasso. Every expectation is an
//! exact rational or algebraic value.

use filament_exact::Rational;
use fsl_semantics::check::{ArithmeticOp, CompareOp};
use ix_trace_rs::trace;

use super::*;
use crate::hybrid_replay::eval::{Bi, Env};
use crate::hybrid_replay::expand::{Exp, TermLocus};
use crate::hybrid_replay::poly::Poly;
use crate::hybrid_replay::poly2::{projection, Poly2};
use crate::hybrid_replay::real::ExactReal;
use crate::hybrid_replay::section::Section;

fn num(value: i64) -> Exp {
    Exp::Number(n(value))
}

fn times(a: Exp, b: Exp) -> Exp {
    Exp::Arithmetic(
        ArithmeticOp::Mul,
        Box::new(a),
        Box::new(b),
        TermLocus::default(),
    )
}

fn compare(op: CompareOp, a: Exp, b: Exp) -> Exp {
    Exp::Compare(op, Box::new(a), Box::new(b))
}

fn and(a: Exp, b: Exp) -> Exp {
    Exp::Logic(fsl_semantics::check::LogicOp::And, Box::new(a), Box::new(b))
}

/// Whether some real `x` satisfies `body`, where `Variable(0)` is `x` and
/// `Variable(1)` is `t`, at the instant `at`.
fn exists_at(body: &Exp, at: &ExactReal) -> bool {
    let variables = [
        Poly2::variable(),
        Poly2::constant(Poly::new(vec![n(0), n(1)])),
    ];
    let env = Env::<Bi> {
        variables: &variables,
        captures: &[],
        activation: &[],
    };
    let mut meter = HybridReplayMeter::new(u64::MAX);
    Section::new(at)
        .exists(body, &env, &mut meter)
        .expect("the body evaluates")
        .0
}

fn rational(value: Rational) -> ExactReal {
    ExactReal::Rational(value)
}

/// The one root of `t^3 - 4` (the cube root of 4, irrational, near 1.5874).
fn cube_root_of_four() -> ExactReal {
    let roots = Poly::new(vec![n(-4), n(0), n(0), n(1)]).roots_in(&n(0), &n(3));
    let [root] = roots.as_slice() else {
        panic!("one real root: {roots:?}");
    };
    root.clone()
}

/// `x * x = t` has a real solution exactly where `t >= 0`, at rational and
/// at irrational instants, the root `t = 0` included.
#[trace("TC-1221", "FR-450-AC-6")]
#[test]
fn a_square_has_a_real_root_exactly_where_it_is_not_negative() {
    let body = compare(
        CompareOp::Eq,
        times(Exp::Variable(0), Exp::Variable(0)),
        Exp::Variable(1),
    );
    for (at, expected) in [
        (n(4), true),
        (n(0), true),
        (q(1, 3), true),
        (n(-1), false),
        (q(-1, 1000), false),
    ] {
        assert_eq!(exists_at(&body, &rational(at.clone())), expected, "{at:?}");
    }
    let two = Poly::new(vec![n(-2), n(0), n(1)]).roots_in(&n(-2), &n(2));
    let [negative, positive] = two.as_slice() else {
        panic!("two roots: {two:?}");
    };
    assert!(exists_at(&body, positive), "t = sqrt 2");
    assert!(!exists_at(&body, negative), "t = -sqrt 2");
}

/// The instant at which `x * x = t` and `x * x * x >= 2` have a common real
/// solution begins exactly at the cube root of 4, where the second holds with
/// equality: the section's own value is decided exactly, not by sampling
/// either side of it.
#[trace("TC-1221", "FR-450-AC-6")]
#[test]
fn a_solution_that_exists_only_from_an_irrational_instant_is_decided_exactly() {
    let x = || Exp::Variable(0);
    let body = and(
        compare(CompareOp::Eq, times(x(), x()), Exp::Variable(1)),
        compare(CompareOp::Ge, times(times(x(), x()), x()), num(2)),
    );
    assert!(exists_at(&body, &cube_root_of_four()), "at the end itself");
    assert!(!exists_at(&body, &rational(q(3, 2))), "3/2 < 4^(1/3)");
    assert!(exists_at(&body, &rational(q(8, 5))), "8/5 > 4^(1/3)");
    assert!(!exists_at(&body, &rational(n(0))));
}

/// `x * x * x = 2 and x * x = t` has a real solution at one instant, the
/// cube root of 4, and nowhere else.
#[trace("TC-1221", "FR-450-AC-6")]
#[test]
fn a_solution_at_one_irrational_instant_is_found_and_no_neighbour_has_one() {
    let x = || Exp::Variable(0);
    let body = and(
        compare(CompareOp::Eq, times(x(), x()), Exp::Variable(1)),
        compare(CompareOp::Eq, times(times(x(), x()), x()), num(2)),
    );
    assert!(exists_at(&body, &cube_root_of_four()));
    for near in [q(3, 2), q(8, 5), q(1587, 1000), q(1588, 1000), n(4), n(0)] {
        assert!(!exists_at(&body, &rational(near.clone())), "{near:?}");
    }
}

/// The projection of `x * x - t` has the one root `t = 0`: where its two
/// real roots in `x` merge and then vanish.
#[trace("TC-1221", "FR-450-AC-6")]
#[test]
fn the_projection_of_a_square_has_its_discriminant_root() {
    let square = Poly2::new(vec![
        Poly::new(vec![n(0), n(-1)]),
        Poly::zero(),
        Poly::constant(n(1)),
    ]);
    let projected = projection(&[square]);
    assert!(!projected.is_empty());
    for poly in &projected {
        assert_eq!(poly.roots_in(&n(-100), &n(100)).len(), 1, "{poly:?}");
        assert_eq!(
            poly.roots_in(&n(-100), &n(100))[0],
            rational(n(0)),
            "{poly:?}"
        );
    }
}

/// `x - t` and `x - 1` cross at `t = 1`: the pair's resultant has that
/// root, and no other.
#[trace("TC-1221", "FR-450-AC-6")]
#[test]
fn the_projection_of_two_lines_has_their_crossing() {
    let moving = Poly2::new(vec![Poly::new(vec![n(0), n(-1)]), Poly::constant(n(1))]);
    let fixed = Poly2::new(vec![Poly::constant(n(-1)), Poly::constant(n(1))]);
    let projected = projection(&[moving, fixed]);
    let roots_of = |poly: &Poly| poly.roots_in(&n(-100), &n(100));
    // The resultant is the one projected polynomial that vanishes at the
    // crossing and nowhere else; the coefficient `-t` adds the root 0, where
    // `x - t` loses its constant term.
    assert!(projected
        .iter()
        .any(|poly| roots_of(poly) == vec![rational(n(1))]));
    for poly in &projected {
        for root in roots_of(poly) {
            assert!(root == rational(n(0)) || root == rational(n(1)), "{root}");
        }
    }
}

const PULSE: &str = "hybrid Pulse using p {\n\
       mode Run invariant { v >= 0 and v <= 3 and w >= 0 and w <= 4 }\n\
         flow { v' = 1; w' = 0; }\n\
         transition to Run when { v = 3 } reset { v = 0 };\n\
         transition to Gone when { true } reset { w >= 0 };\n\
       mode Gone invariant { w * w = v and w * w * w = 2 }\n\
         flow { v' = 0; w' = 0; }\n\
     }\n\
     temporal Weak using p over (h: Pulse) clock \"model-time\" on origin {\n\
       fair weak Run::Gone;\n\
       always holds(h.v >= 1)\n\
     }\n\
     temporal Strong using p over (h: Pulse) clock \"model-time\" on origin {\n\
       fair strong Run::Gone;\n\
       always holds(h.v >= 1)\n\
     }\n";

/// The `Pulse` lasso: `v` from 0 to 3 in `Run`, reset to 0, repeated.
fn pulse_trace() -> HybridTrace {
    HybridTrace {
        start: state(0, n(0), &[n(0), n(0)]),
        jumps: vec![TraceJump {
            transition: 0,
            before: Some(before(n(3), &[n(3), n(0)])),
            state: state(0, n(3), &[n(0), n(0)]),
        }],
        delta: None,
        loop_to: Some(0),
        end: None,
    }
}

/// `Run -> Gone` is enabled at one instant of each period, `v = 4^(1/3)`,
/// an irrational one at which `Gone`'s invariant has the real solution
/// `w = 2^(1/3)`. A weak constraint needs the transition enabled throughout,
/// which it is not, so the lasso is fair and the claim, false at `v = 0`,
/// reproduces; a strong constraint needs it taken when enabled infinitely
/// often, which the lasso never does, so it is unfair.
#[trace("TC-1221", "FR-450-AC-6")]
#[test]
fn a_transition_enabled_at_one_irrational_instant_is_fair_weakly_not_strongly() {
    let graph = check(PULSE);
    let lasso = pulse_trace();
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Weak"), &lasso),
        Ok(reproduced_formula(&lasso))
    );
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Strong"), &lasso),
        Ok(ExactReplay::NotReproduced {
            at: JsonPointer::root(),
            check: ExactCheck::Claim,
        })
    );
}

/// Quantifier elimination is charged to the caller's meter before each step,
/// with no fixed cap: a bound of 30 stops it, and its consumption grows with
/// the work rather than stopping at a constant.
#[trace("TC-1221", "FR-450-AC-7")]
#[test]
fn quantifier_elimination_is_charged_to_the_callers_meter() {
    let graph = check(PULSE);
    let lasso = pulse_trace();
    let strong = claim(&graph, "Strong");
    let mut small = HybridReplayMeter::new(30);
    assert_eq!(
        crate::hybrid_replay::exact_replay(&graph, strong, &lasso, &mut small),
        Err(HybridReplayRefusal::ResourceExhausted {
            counter: filament_exact::LimitKind::WorkUnits,
        })
    );
    let mut large = HybridReplayMeter::new(u64::MAX);
    assert!(crate::hybrid_replay::exact_replay(&graph, strong, &lasso, &mut large).is_ok());
    assert!(large.consumed() > 30, "{}", large.consumed());
}

/// Two free variables that the target invariant couples in one condition are
/// the one reset shape exact replay does not yet decide (QSL-2529 remaining
/// work), so the trace settles `NotExact`.
#[trace("TC-1221", "FR-450-AC-6")]
#[test]
fn coupled_free_variables_are_not_yet_decided() {
    let graph = check(
        "hybrid Coupled using p {\n\
           mode Run invariant { v >= 0 and v <= 1 and w >= 0 and w <= 1 }\n\
             flow { v' = 1; w' = 0; }\n\
             transition to Run when { v = 1 } reset { v = 0 };\n\
             transition to Gone when { true } reset { v >= 0 and w >= 0 };\n\
           mode Gone invariant { v + w <= 1 }\n\
             flow { v' = 0; w' = 0; }\n\
         }\n\
         temporal Fairly using p over (h: Coupled) clock \"model-time\" on origin {\n\
           fair Run::Gone;\n\
           always holds(h.v >= 1)\n\
         }\n",
    );
    let lasso = HybridTrace {
        start: state(0, n(0), &[n(0), n(0)]),
        jumps: vec![TraceJump {
            transition: 0,
            before: Some(before(n(1), &[n(1), n(0)])),
            state: state(0, n(1), &[n(0), n(0)]),
        }],
        delta: None,
        loop_to: Some(0),
        end: None,
    };
    assert_eq!(
        exact_replay(&graph, claim(&graph, "Fairly"), &lasso),
        Ok(ExactReplay::NotExact)
    );
}

/// FR-750-AC-12 (b), `Shuttle`'s three fairness variants over the point
/// lasso `L` from `(Moving, v = 0)` at time 0 by transition 0 (`before`
/// `(time 1, v = 1)`) to `(Moving, v = 0)` at time 1, `loop_to` 0.
const SHUTTLE: &str = "hybrid Shuttle using p {\n\
       initial { mode Moving when { v = 0 }; }\n\
       mode Moving invariant { v >= 0 and v <= 1 } flow { v' = 1; }\n\
         transition to Moving when { v = 1 } reset { v = 0 };\n\
         transition to Stopped when { v >= 0 } reset { RESET };\n\
       mode Stopped invariant { v >= 1 and v <= 2 } flow { v' = 0; }\n\
     }\n\
     temporal Whole using p over (h: Shuttle) clock \"model-time\" on origin {\n\
       fair Moving;\n\
       eventually [0, *] holds(h.v > 1)\n\
     }\n\
     temporal Named using p over (h: Shuttle) clock \"model-time\" on origin {\n\
       fair Moving::Stopped;\n\
       eventually [0, *] holds(h.v > 1)\n\
     }\n";

fn shuttle(reset: &str) -> Unit {
    check(&SHUTTLE.replace("RESET", reset))
}

fn shuttle_lasso() -> HybridTrace {
    HybridTrace {
        start: state(0, n(0), &[n(0)]),
        jumps: vec![TraceJump {
            transition: 0,
            before: Some(before(n(1), &[n(1)])),
            state: state(0, n(1), &[n(0)]),
        }],
        delta: None,
        loop_to: Some(0),
        end: None,
    }
}

/// `Moving -> Stopped` is enabled at every instant of every period: its
/// guard `v >= 0` always holds and its reset `v >= 0` admits `v` in `[1, 2]`
/// under `Stopped`'s invariant. The whole constraint `fair Moving;` is taken
/// every period by the self-loop, so `L` is fair and reproduces; naming
/// `Moving::Stopped;` alone is enabled at every instant and never taken, so
/// `L` is unfair and is not reproduced at check 5; with the reset changed to
/// `v <= 1/2`, which no state under `Stopped`'s invariant meets, the enabled
/// set is empty and `L` is fair.
#[trace("TC-3037", "FR-750-AC-12")]
#[test]
fn shuttle_fairness_over_a_nondeterministic_reset_is_decided_exactly() {
    let lasso = shuttle_lasso();
    let always = shuttle("v >= 0");
    assert_eq!(
        exact_replay(&always, claim(&always, "Whole"), &lasso),
        Ok(reproduced_formula(&lasso))
    );
    assert_eq!(
        exact_replay(&always, claim(&always, "Named"), &lasso),
        Ok(ExactReplay::NotReproduced {
            at: JsonPointer::root(),
            check: ExactCheck::Claim,
        })
    );
    let never = shuttle("v <= 1/2");
    assert_eq!(
        exact_replay(&never, claim(&never, "Named"), &lasso),
        Ok(reproduced_formula(&lasso))
    );
}
