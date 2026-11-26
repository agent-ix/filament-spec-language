// SPDX-License-Identifier: AGPL-3.0-or-later
//! The probabilistic rows of the corpus. Each ticket model has a `_hand`
//! test, in which the falsifier's extrema over the dense grids equal the
//! values derived by hand from the specification, and a `_routes` test, in
//! which every route gives exactly the outcome the row names for it: the hand
//! verdict and value, or one named decline. Where EN-5 and EN-4 are tracked
//! apart, `_routes` reads EN-5 and `_sampled` reads EN-4. A `_hand` test runs the falsifier
//! and the readers only, so it carries no criterion tag.
//!
//! The positions a claim reads are FSpec FR-416's: the initial state, the
//! post-state of each discrete step and the idle tail's stutter positions. A
//! delay adds none.

use filament_exact::Rational;
use fsl_eval::simulation::model::ModelSystem;
use fsl_eval::temporal::{timed_elapsed, timed_fraction, timed_steps, TemporalPosition};
use ix_trace_rs::trace;

use super::falsifier::{self, ratio, whole, Behaviour, Extrema, Lock, Space};
use super::models::{self, every, formula, sampled, under};
use super::routes::{self, Verdict};
use crate::timed_world::World;

/// The bound of a claim.
#[derive(Clone)]
enum Bound {
    AtLeast(Rational),
    AtMost(Rational),
}

use Bound::{AtLeast, AtMost};

/// What each behaviour is worth: a formula's indicator, a measure's
/// indicator or a measure's value.
#[derive(Clone)]
enum Payoff {
    /// The temporal declaration of this name holds at position 0.
    Holds(&'static str),
    /// `steps from A until B within h ms >= k`.
    StepsAtLeast(&'static str, &'static str, i64, i64),
    /// `elapsed from A until B within h ms >= c ms`; censored exceeds `c`.
    ElapsedAtLeast(&'static str, &'static str, i64, i64),
    /// `elapsed from A until B within h ms <= c ms`.
    ElapsedAtMost(&'static str, &'static str, i64, i64),
    /// `fraction holds(P) over [0, h ms]`, the value itself.
    Fraction(&'static str, i64),
}

fn pay(payoff: &Payoff, world: &World, system: &ModelSystem<'_>, b: &Behaviour) -> Rational {
    let truths = |name: &str| falsifier::truths(world, system, name, b);
    let meter = || filament_exact::Meter::new(system.meter_limits());
    let origin = TemporalPosition::new(0);
    let indicator = |value: bool| whole(i64::from(value));
    match payoff {
        Payoff::Holds(name) => falsifier::holds(world, system, name, b),
        Payoff::StepsAtLeast(from, until, within, least) => {
            let reading = timed_steps(
                &b.trace(),
                origin,
                &truths(from),
                &truths(until),
                &whole(*within),
                &mut meter(),
            )
            .expect("a represented position");
            indicator(
                falsifier::amount(reading)
                    .is_some_and(|value| value.is_none_or(|v| v >= whole(*least))),
            )
        }
        Payoff::ElapsedAtLeast(from, until, within, bound)
        | Payoff::ElapsedAtMost(from, until, within, bound) => {
            let reading = timed_elapsed(
                &b.trace(),
                origin,
                &truths(from),
                &truths(until),
                &whole(*within),
                &mut meter(),
            )
            .expect("a represented position");
            let value = falsifier::amount(reading).expect("every behaviour activates");
            indicator(match payoff {
                Payoff::ElapsedAtLeast(..) => value.is_none_or(|v| v >= whole(*bound)),
                _ => value.is_some_and(|v| v <= whole(*bound)),
            })
        }
        Payoff::Fraction(name, within) => {
            let reading = timed_fraction(
                &b.trace(),
                origin,
                &truths(name),
                &whole(*within),
                &mut meter(),
            )
            .expect("a represented position");
            falsifier::amount(reading)
                .flatten()
                .expect("a fraction has a value")
        }
    }
}

/// The outcome one route must give for a claim.
#[derive(Clone, Copy)]
enum Expect {
    /// The hand verdict, with the hand value where the route states one.
    Hand,
    /// Exactly this named decline.
    Declines(&'static str),
    /// The hand verdict, or `incomplete`: what a form waiting on the region
    /// process may settle (FSL FR-204 "Admission", ADR-038).
    HandOrIncomplete,
}

/// One claim of a model with its hand extrema.
struct Claim {
    /// The claim the exact route reads, if it has one.
    exact: Option<&'static str>,
    /// The claim the sampled route reads, if it has one.
    sampled: Option<&'static str>,
    exact_expect: Expect,
    sampled_expect: Expect,
    payoff: Payoff,
    bound: Bound,
    min: Rational,
    max: Rational,
}

fn claim(name: &'static str, payoff: Payoff, bound: Bound, min: Rational, max: Rational) -> Claim {
    Claim {
        exact: Some(name),
        sampled: None,
        exact_expect: Expect::Hand,
        sampled_expect: Expect::Hand,
        payoff,
        bound,
        min,
        max,
    }
}

impl Claim {
    fn also_sampled(mut self, name: &'static str) -> Self {
        self.sampled = Some(name);
        self
    }

    fn exact_expects(mut self, expect: Expect) -> Self {
        self.exact_expect = expect;
        self
    }

    fn sampled_expects(mut self, expect: Expect) -> Self {
        self.sampled_expect = expect;
        self
    }

    /// Whether `verdict` is the outcome `expect` names for `route`.
    fn agrees(&self, route: &str, expect: Expect, verdict: &Verdict) -> bool {
        // An exact proof states the compared extreme; a sampled test states
        // its estimate, the value where every behaviour has one.
        let proof_value = |value: &Option<Rational>| match (route, value) {
            ("en-4", Some(value)) => self.min != self.max || value == &self.min,
            (_, Some(value)) => value == self.extreme(),
            (_, None) => false,
        };
        let refutation_value = |value: &Option<Rational>| match (route, value) {
            ("en-4", Some(value)) => self.min != self.max || value == &self.min,
            ("en-4", None) => false,
            (_, value) => value.is_none(),
        };
        let hand = match verdict {
            Verdict::Proved(value) => self.holds() && proof_value(value),
            Verdict::Refuted(value) => !self.holds() && refutation_value(value),
            Verdict::Declined(_) | Verdict::Vacuous(_) | Verdict::Broken(_) => false,
        };
        match expect {
            Expect::Hand => hand,
            Expect::Declines(named) => {
                matches!(verdict, Verdict::Declined(found) if found == named)
            }
            Expect::HandOrIncomplete => {
                hand || matches!(
                    verdict,
                    Verdict::Declined(found) if found.starts_with("incomplete:")
                )
            }
        }
    }

    fn holds(&self) -> bool {
        match &self.bound {
            AtLeast(bound) => &self.min >= bound,
            AtMost(bound) => &self.max <= bound,
        }
    }

    /// The extreme the bound compares.
    fn extreme(&self) -> &Rational {
        match self.bound {
            AtLeast(_) => &self.min,
            AtMost(_) => &self.max,
        }
    }
}

/// A model with the claims read on it.
struct Row {
    name: &'static str,
    world: World,
    space: Space,
    claims: Vec<Claim>,
}

impl Row {
    /// The row read by EN-5 alone.
    fn exact_route(mut self) -> Self {
        for claim in &mut self.claims {
            claim.sampled = None;
        }
        self
    }

    /// The row read by EN-4 alone.
    fn sampled_route(mut self) -> Self {
        for claim in &mut self.claims {
            claim.exact = None;
        }
        self
    }

    /// The row with only the claims named `names`, so that claims a route
    /// answers wrongly are ignored apart from those it answers.
    fn only(mut self, names: &[&str]) -> Self {
        self.claims
            .retain(|claim| names.iter().any(|name| claim.exact == Some(*name)));
        assert_eq!(self.claims.len(), names.len(), "{}", self.name);
        self
    }
}

/// The falsifier's extrema equal the hand values.
fn assert_hand(row: &Row) {
    let system = row.world.system();
    for claim in &row.claims {
        let found = falsifier::extrema(&system, &row.space, &|b| {
            pay(&claim.payoff, &row.world, &system, b)
        });
        assert_eq!(
            found,
            Some(Extrema {
                min: claim.min.clone(),
                max: claim.max.clone()
            }),
            "{} {:?}",
            row.name,
            claim.exact.or(claim.sampled)
        );
    }
}

/// Every route of every claim gives the outcome the claim names for it.
fn assert_routes(row: &Row) {
    let mut wrong = Vec::new();
    for claim in &row.claims {
        let answers = claim
            .exact
            .map(|name| (name, claim.exact_expect, routes::exact(&row.world, name)))
            .into_iter()
            .chain(claim.sampled.map(|name| {
                (
                    name,
                    claim.sampled_expect,
                    routes::sampled(&row.world, name),
                )
            }));
        for (name, expect, answer) in answers {
            let line = format!(
                "{} {name} {} holds={} min={:?} max={:?}: {:?}",
                row.name,
                answer.route,
                claim.holds(),
                claim.min,
                claim.max,
                answer.verdict
            );
            eprintln!("OBSERVED {line}");
            if !claim.agrees(answer.route, expect, &answer.verdict) {
                wrong.push(line);
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

fn zero() -> Rational {
    whole(0)
}

fn one() -> Rational {
    whole(1)
}

fn half() -> Rational {
    ratio(1, 2)
}

/// `Max <= 0.999`, `Min >= 0.001` and `Half <= 0.5` of one formula with
/// dense minimum 0 and maximum 1: all false.
fn extrema_claims(form: &str) -> String {
    [("Max", "<= 0.999"), ("Min", ">= 0.001"), ("Half", "<= 0.5")]
        .map(|(name, bound)| every(name, &format!("probability {bound} [ {form} ]")))
        .concat()
}

fn extrema_rows(payoff: &Payoff) -> Vec<Claim> {
    [
        ("Max", AtMost(ratio(999, 1000))),
        ("Min", AtLeast(ratio(1, 1000))),
        ("Half", AtMost(half())),
    ]
    .map(|(name, bound)| claim(name, payoff.clone(), bound, zero(), one()))
    .into()
}

// --- QSL-3324: windows that hold no position --------------------------------

/// `step` runs at exactly 3 ms: the positions are 0 and 3 ms and the idle
/// tail's after it. None is in [1, 2] ms, so `eventually` there is false and
/// `always` there true; from position 0, `always[0, 0]` reads only position 0
/// (no other is stamped 0), where `eventually[2, 2]` finds no position.
fn empty_window(drawn: bool) -> Row {
    let forms = [
        ("E", "<= 0.01", "eventually[1 ms, 2 ms] holds(not c.late)"),
        ("A", ">= 0.99", "always[1 ms, 2 ms] holds(c.late)"),
        (
            "N",
            "<= 0.01",
            "always[0 ms, 0 ms] eventually[2 ms, 2 ms] holds(not c.late)",
        ),
        (
            "L",
            "<= 0.01",
            "always[0 ms, 0 ms] eventually[2 ms, 2 ms] holds(c.late)",
        ),
    ];
    let mut text = String::new();
    for (name, bound, form) in forms {
        let probability = format!("probability {bound} [ {form} ]");
        text += &every(name, &probability);
        if drawn {
            text += &sampled("One", &format!("{name}S"), &probability);
        }
        text += &formula(&format!("F{name}"), form);
    }
    let mut space = Space::new(4, Lock::Frozen);
    if drawn {
        space.fixed = vec![("step", whole(3))];
    }
    let low = || AtMost(ratio(1, 100));
    let mut claims = vec![
        claim("E", Payoff::Holds("FE"), low(), zero(), zero()),
        claim(
            "A",
            Payoff::Holds("FA"),
            AtLeast(ratio(99, 100)),
            one(),
            one(),
        ),
        claim("N", Payoff::Holds("FN"), low(), zero(), zero()),
        claim("L", Payoff::Holds("FL"), low(), zero(), zero()),
    ];
    if drawn {
        claims = claims
            .into_iter()
            .zip(["ES", "AS", "NS", "LS"])
            .map(|(claim, name)| claim.also_sampled(name))
            .collect();
    }
    Row {
        name: if drawn {
            "3324-empty-drawn"
        } else {
            "3324-empty-guard"
        },
        world: models::one_step(drawn, &text),
        space,
        claims,
    }
}

#[test]
fn row_3324_empty_windows_hand() {
    assert_hand(&empty_window(false));
    assert_hand(&empty_window(true));
}

#[test]
fn row_3324_empty_windows_guard_routes() {
    assert_routes(&empty_window(false).only(&["E", "A", "L"]));
}

#[test]
#[ignore = "QSL-3324: en-5 refutes a nested empty window whose probability is 0"]
fn row_3324_nested_empty_window_guard_routes() {
    assert_routes(&empty_window(false).only(&["N"]));
}

#[test]
fn row_3324_empty_windows_drawn_routes() {
    assert_routes(&empty_window(true).only(&["E", "A"]));
}

#[test]
#[ignore = "QSL-3324: en-5 fails on nested windows over a drawn delay"]
fn row_3324_nested_empty_windows_drawn_routes() {
    assert_routes(&empty_window(true).only(&["N", "L"]));
}

/// `a` runs at `t` in [0, 1] ms and `b` at `t + 2` ms. With `t` strictly
/// inside (0, 1) the positions 0, `t`, `t + 2` and the later idle ones leave
/// [1, 2] ms empty; with `t = 1` a position is at 1 ms. So `always[1, 2]
/// false` has extrema 0 and 1, and so have `eventually[1, 2] true` and
/// `true until[1, 2] true`.
fn workload_windows(drawn: bool) -> Row {
    let forms = [
        ("Always", "<= 0.5", "always[1 ms, 2 ms] holds(false)"),
        ("Eventually", ">= 0.5", "eventually[1 ms, 2 ms] holds(true)"),
        (
            "Until",
            ">= 0.5",
            "holds(true) until[1 ms, 2 ms] holds(true)",
        ),
    ];
    let mut text = String::new();
    for (name, bound, form) in forms {
        text += &under("W", name, &format!("probability {bound} [ {form} ]"));
        text += &formula(&format!("F{name}"), form);
    }
    let mut space = Space::new(3, Lock::Frozen);
    if drawn {
        space.fixed = vec![("b", whole(2))];
    }
    Row {
        name: if drawn {
            "3324-workload-drawn"
        } else {
            "3324-workload-free"
        },
        world: models::workload_pair(drawn, &text),
        space,
        claims: vec![
            claim(
                "Always",
                Payoff::Holds("FAlways"),
                AtMost(half()),
                zero(),
                one(),
            ),
            claim(
                "Eventually",
                Payoff::Holds("FEventually"),
                AtLeast(half()),
                zero(),
                one(),
            ),
            claim(
                "Until",
                Payoff::Holds("FUntil"),
                AtLeast(half()),
                zero(),
                one(),
            ),
        ],
    }
}

#[test]
fn row_3324_workload_windows_hand() {
    assert_hand(&workload_windows(false));
    assert_hand(&workload_windows(true));
}

#[test]
#[ignore = "QSL-3324: en-5 proves three false window claims under a workload"]
fn row_3324_workload_windows_routes() {
    assert_routes(&workload_windows(false));
}

#[test]
#[ignore = "QSL-3324: en-5 proves three false window claims under a workload with a drawn delay"]
fn row_3324_workload_windows_drawn_routes() {
    assert_routes(&workload_windows(true));
}

/// FR-204's `Gap`: `always[1, 2] false` holds where no position is in [1, 2]
/// ms, which `first` strictly inside (0, 1) gives, and fails with `first` at
/// 1 ms.
fn gap() -> Row {
    let form = "always[1 ms, 2 ms] holds(false)";
    Row {
        name: "3324-gap",
        world: models::two_step(&(extrema_claims(form) + &formula("F", form))),
        space: Space::new(3, Lock::Frozen),
        claims: extrema_rows(&Payoff::Holds("F")),
    }
}

#[test]
fn row_3324_gap_hand() {
    assert_hand(&gap());
}

#[test]
fn row_3324_gap_minimum_routes() {
    assert_routes(&gap().only(&["Min"]));
}

#[test]
#[ignore = "QSL-3324: en-5 proves maximum 0 where it is 1"]
fn row_3324_gap_maximum_routes() {
    assert_routes(&gap().only(&["Max", "Half"]));
}

/// `send` at exactly 2 ms: positions 0 and 2 and the idle tail's, none at
/// 1 ms, so `eventually[1, 1]` is false on the only behaviour.
fn nested_inside_a_delay() -> Row {
    let form = "eventually[1 ms, 1 ms] always[0 ms, 0 ms] holds(c.phase = 0)";
    let text = every("Nested", &format!("probability <= 0.5 [ {form} ]")) + &formula("F", form);
    Row {
        name: "3324-nested-in-delay",
        world: models::send_at(2, &text),
        space: Space::new(3, Lock::Frozen),
        claims: vec![claim(
            "Nested",
            Payoff::Holds("F"),
            AtMost(half()),
            zero(),
            zero(),
        )],
    }
}

#[test]
fn row_3324_nested_inside_a_delay_hand() {
    assert_hand(&nested_inside_a_delay());
}

#[test]
#[ignore = "QSL-3324: en-5 refutes a claim whose probability is 0 with bound <= 0.5"]
fn row_3324_nested_inside_a_delay_routes() {
    assert_routes(&nested_inside_a_delay());
}

/// `Retx` over `always[0, 4] always[2, 2] false`, which holds where no
/// position at most 4 ms is followed by one exactly 2 ms later. Sends 1.5 ms
/// apart satisfy it on every draw (maximum 1); a first send at exactly 2 ms
/// breaks it on every draw (minimum 0). Both extrema are the same on every
/// draw, so the walk's equal weighting of the two outcomes does not change
/// them.
///
/// A nested window is decided over the region MDP (FSL FR-204-AC-26), and may
/// stop `incomplete` while that is not built; never `unsupported`.
fn retx_offsets() -> Row {
    let form = "always[0 ms, 4 ms] always[2 ms, 2 ms] holds(false)";
    let mut space = Space::new(2, Lock::Frozen);
    space.delays = Some(vec![ratio(3, 2), whole(2)]);
    space.horizon = Some(6);
    space.drawn = vec!["send"];
    Row {
        name: "3324-retx-offsets",
        world: models::retx(&(extrema_claims(form) + &formula("F", form))),
        space,
        claims: extrema_rows(&Payoff::Holds("F"))
            .into_iter()
            .map(|claim| claim.exact_expects(Expect::HandOrIncomplete))
            .collect(),
    }
}

#[test]
fn row_3324_retx_offsets_hand() {
    assert_hand(&retx_offsets());
}

#[test]
#[ignore = "QSL-3318: en-5 settles a nested window unsupported TimedFormShape"]
fn row_3324_retx_offsets_routes() {
    assert_routes(&retx_offsets());
}

// --- QSL-3444: a region between two clocks' integer crossings ---------------

/// `first` at 0.5 ms and `third` at 1.25 ms reach `late` (maximum 1); a
/// scheduler that never takes `third` never does (minimum 0).
fn two_clocks(bounded: bool) -> Row {
    let form = "eventually[0 ms, 4 ms] holds(c.late)";
    Row {
        name: if bounded {
            "3444-two-clocks-bounded"
        } else {
            "3444-two-clocks"
        },
        world: models::two_clocks(bounded, &(extrema_claims(form) + &formula("F", form))),
        space: Space::new(3, Lock::Frozen),
        claims: extrema_rows(&Payoff::Holds("F")),
    }
}

#[test]
fn row_3444_two_clock_regions_hand() {
    assert_hand(&two_clocks(false));
    assert_hand(&two_clocks(true));
}

#[test]
fn row_3444_two_clock_regions_minimum_routes() {
    assert_routes(&two_clocks(false).only(&["Min"]));
}

#[test]
#[ignore = "QSL-3443: en-5 certificate is rejected where the maximum is 1"]
fn row_3444_two_clock_regions_maximum_routes() {
    assert_routes(&two_clocks(false).only(&["Max", "Half"]));
}

#[test]
fn row_3444_two_clock_regions_bounded_minimum_routes() {
    assert_routes(&two_clocks(true).only(&["Min"]));
}

#[test]
#[ignore = "QSL-3444: en-5 proves maximum 0 where it is 1"]
fn row_3444_two_clock_regions_bounded_maximum_routes() {
    assert_routes(&two_clocks(true).only(&["Max", "Half"]));
}

// --- QSL-3445: dense times below every integer-time value -------------------

/// Lost (draw 1/2): `M = 2 - t`; kept: `M = t + 1`. A `t` strictly inside
/// (0, 1) makes both exceed 1 ms, so `P(M <= 1 ms)` has minimum 0; `t = 0`
/// or `t = 1` gives 1/2, the maximum. `quantile 0.4 <= 1 ms` needs the
/// probability at least 0.4 on every scheduler: false.
fn skew_elapsed() -> Row {
    let measure = "elapsed from holds(c.inflight) until holds(c.late) within 4 ms";
    let text = every("Quantile", &format!("quantile 0.4 of ({measure}) <= 1 ms"))
        + &formula("Flying", "holds(c.inflight)")
        + &formula("Late", "holds(c.late)");
    let mut space = Space::new(3, Lock::Frozen);
    space.drawn = vec!["first"];
    Row {
        name: "3445-elapsed-quantile",
        world: models::skew(&text),
        space,
        claims: vec![claim(
            "Quantile",
            Payoff::ElapsedAtMost("Flying", "Late", 4, 1),
            AtLeast(ratio(2, 5)),
            zero(),
            half(),
        )],
    }
}

#[test]
fn row_3445_elapsed_quantile_hand() {
    assert_hand(&skew_elapsed());
}

#[trace("FSpec-FR-407-AC-26")]
#[test]
#[ignore = "QSL-3445: en-5 proves a false elapsed quantile on integer times"]
fn row_3445_elapsed_quantile_routes() {
    assert_routes(&skew_elapsed());
}

/// The same subject read by a window that starts after position 0: from the
/// position where `inflight` holds and `late` does not, `late` within 1 ms.
/// That position is at `t` when lost, with `late` at 2 ms (holds only at
/// `t = 1`), and at 1 ms when kept, with `late` at `t + 2` (holds only at
/// `t = 0`). So the minimum is 0, at every `t` strictly inside (0, 1), and
/// the maximum 1/2; integer `t` alone gives 1/2 for both. The form waits on
/// the region process, so it may stop `incomplete`, never `unsupported`.
fn skew_nested() -> Row {
    let form = "eventually[0 ms, 4 ms] \
         (holds(c.inflight and not c.late) and eventually[0 ms, 1 ms] holds(c.late))";
    let text = every("Inner", &format!("probability >= 0.4 [ {form} ]")) + &formula("F", form);
    let mut space = Space::new(3, Lock::Frozen);
    space.drawn = vec!["first"];
    Row {
        name: "3445-nested-later-window",
        world: models::skew(&text),
        space,
        claims: vec![claim(
            "Inner",
            Payoff::Holds("F"),
            AtLeast(ratio(2, 5)),
            zero(),
            half(),
        )
        .exact_expects(Expect::HandOrIncomplete)],
    }
}

#[test]
fn row_window_starting_after_position_zero_hand() {
    assert_hand(&skew_nested());
}

#[test]
#[ignore = "QSL-3318: en-5 settles a later-start nested window unsupported TimedFormShape"]
fn row_window_starting_after_position_zero_routes() {
    assert_routes(&skew_nested());
}

/// The same subject read by an activation after position 0: `on each` call,
/// activated at the first position where `inflight` holds and `late` does
/// not, `late` within 1 ms. The activating positions and the position where
/// `late` first holds are those of [`skew_nested`], so the extrema are the
/// same: minimum 0, maximum 1/2.
fn skew_each() -> Row {
    let claim = "probabilistic Each using ev on M under every scheduler \
         on each (k: M::Call) when (k.inflight and not k.late) \
         { probability >= 0.4 [ eventually[0 ms, 1 ms] holds(k.late) ] }\n";
    let from_activation = "holds(not (c.inflight and not c.late)) until[0 ms, 10 ms] \
         (holds(c.inflight and not c.late) and eventually[0 ms, 1 ms] holds(c.late))";
    let mut space = Space::new(3, Lock::Frozen);
    space.drawn = vec!["first"];
    Row {
        name: "3445-on-each-later-activation",
        world: models::skew(&format!("{claim}{}", formula("F", from_activation))),
        space,
        // A free subject read from a later position waits on the region
        // process (FSpec FR-421 "Closed and strict conditions").
        claims: vec![claim_each().exact_expects(Expect::HandOrIncomplete)],
    }
}

fn claim_each() -> Claim {
    claim(
        "Each",
        Payoff::Holds("F"),
        AtLeast(ratio(2, 5)),
        zero(),
        half(),
    )
}

#[test]
fn row_activation_after_position_zero_hand() {
    assert_hand(&skew_each());
}

#[test]
#[ignore = "QSL-3531: en-5 settles an on-each window unsupported"]
fn row_activation_after_position_zero_routes() {
    assert_routes(&skew_each());
}

// --- QSL-3447, FR-254: sampled measures and windows -------------------------

/// The exact and sampled claims of one `form` under the workload `Even`.
fn pair(name: &str, form: &str) -> String {
    under("Even", name, form) + &sampled("Even", &format!("{name}S"), form)
}

fn fixed_row(name: &'static str, send: u32, reply: Option<u32>, text: &str, claim: Claim) -> Row {
    let delays = [("send", whole(i64::from(send)))]
        .into_iter()
        .chain(reply.map(|reply| ("reply", whole(i64::from(reply)))))
        .collect();
    let mut space = Space::new(i64::from(send.max(reply.unwrap_or(0))) + 1, Lock::Frozen);
    space.fixed = delays;
    Row {
        name,
        world: models::send_reply(send, reply, text),
        space,
        claims: vec![claim],
    }
}

fn phases() -> String {
    formula("P0", "holds(c.phase = 0)")
        + &formula("P1", "holds(c.phase = 1)")
        + &formula("P2", "holds(c.phase = 2)")
}

/// Positions stamped 0 and 2 are in [0, 5] ms and `reply` at 12 is not:
/// half of them have `phase` 1, so the mean 1/2 is not `<= 0.25`.
fn mean_fraction() -> Row {
    let form = "mean of (fraction holds(c.phase = 1) over [0, 5 ms]) <= 0.25";
    fixed_row(
        "3447-mean-fraction",
        2,
        Some(10),
        &(pair("Mean", form) + &phases()),
        claim(
            "Mean",
            Payoff::Fraction("P1", 5),
            AtMost(ratio(1, 4)),
            half(),
            half(),
        )
        .also_sampled("MeanS")
        .exact_expects(Expect::HandOrIncomplete),
    )
}

#[test]
fn row_3447_mean_fraction_hand() {
    assert_hand(&mean_fraction());
}

#[test]
#[ignore = "QSL-3531: en-5 settles an unweighted window fraction unsupported"]
fn row_3447_mean_fraction_routes() {
    assert_routes(&mean_fraction().exact_route());
}

#[trace("FR-254-AC-5")]
#[test]
fn row_3447_mean_fraction_sampled() {
    assert_routes(&mean_fraction().sampled_route());
}

/// `phase` 0 at position 0 and 1 at the position `send` reaches, at 2 ms:
/// one step, so `steps >= 2` has probability 0.
fn two_steps() -> Row {
    let form = "probability >= 0.5 [ steps from holds(c.phase = 0) \
         until holds(c.phase = 1) within 5 ms >= 2 ]";
    fixed_row(
        "3447-steps",
        2,
        None,
        &(pair("Steps", form) + &phases()),
        claim(
            "Steps",
            Payoff::StepsAtLeast("P0", "P1", 5, 2),
            AtLeast(half()),
            zero(),
            zero(),
        )
        .also_sampled("StepsS"),
    )
}

#[test]
fn row_3447_steps_hand() {
    assert_hand(&two_steps());
}

#[test]
#[ignore = "QSL-3531: en-5 settles a steps measure within a time horizon unsupported"]
fn row_3447_steps_routes() {
    assert_routes(&two_steps().exact_route());
}

#[test]
fn row_3447_steps_sampled() {
    assert_routes(&two_steps().sampled_route());
}

/// `phase` is first 1 at 2 ms and first 2 at `2 + reply` ms: the elapsed
/// time is `reply`, censored when it passes the horizon (FR-254-AC-9,
/// QSL-3463).
fn elapsed(
    name: &'static str,
    send: u32,
    reply: u32,
    form: &str,
    payoff: Payoff,
    bound: Bound,
    value: Rational,
) -> Row {
    fixed_row(
        name,
        send,
        Some(reply),
        &(pair("Q", form) + &phases()),
        claim("Q", payoff, bound, value.clone(), value).also_sampled("QS"),
    )
}

fn elapsed_form(within: u32, comparison: &str) -> String {
    format!(
        "probability {comparison} [ elapsed from holds(c.phase = 1) until holds(c.phase = 2) \
         within {within} ms <= {within} ms ]"
    )
}

fn at_most(within: i64) -> Payoff {
    Payoff::ElapsedAtMost("P1", "P2", within, within)
}

fn elapsed_ten() -> Row {
    let form = elapsed_form(10, ">= 0.5");
    elapsed(
        "254-ac9-within-10",
        2,
        10,
        &form,
        at_most(10),
        AtLeast(half()),
        one(),
    )
}

fn elapsed_nine() -> Row {
    let form = elapsed_form(9, ">= 0.5");
    elapsed(
        "254-ac9-within-9",
        2,
        10,
        &form,
        at_most(9),
        AtLeast(half()),
        zero(),
    )
}

fn elapsed_one() -> Row {
    let form = elapsed_form(1, ">= 0.5");
    elapsed(
        "254-ac9-reply-1",
        2,
        1,
        &form,
        at_most(1),
        AtLeast(half()),
        one(),
    )
}

#[test]
fn row_254_elapsed_from_the_first_a_hand() {
    assert_hand(&elapsed_ten());
    assert_hand(&elapsed_nine());
    assert_hand(&elapsed_one());
}

#[test]
#[ignore = "QSL-3531: en-5 settles an elapsed event unsupported"]
fn row_254_elapsed_within_ten_routes() {
    assert_routes(&elapsed_ten().exact_route());
}

#[trace("FR-254-AC-9")]
#[test]
fn row_254_elapsed_within_ten_sampled() {
    assert_routes(&elapsed_ten().sampled_route());
}

#[test]
#[ignore = "QSL-3531: en-5 settles an elapsed event unsupported"]
fn row_254_elapsed_within_nine_routes() {
    assert_routes(&elapsed_nine().exact_route());
}

#[trace("FR-254-AC-9")]
#[test]
fn row_254_elapsed_within_nine_sampled() {
    assert_routes(&elapsed_nine().sampled_route());
}

#[test]
#[ignore = "QSL-3531: en-5 settles an elapsed event unsupported"]
fn row_254_elapsed_reply_one_routes() {
    assert_routes(&elapsed_one().exact_route());
}

#[trace("FR-254-AC-9")]
#[test]
fn row_254_elapsed_reply_one_sampled() {
    assert_routes(&elapsed_one().sampled_route());
}

const FOUR: &str = "elapsed from holds(c.phase = 1) until holds(c.phase = 2) within 4 ms";

fn at_least_four(name: &'static str, send: u32, comparison: &str, bound: Bound) -> Row {
    let form = format!("probability {comparison} [ {FOUR} >= 4 ms ]");
    let mut row = elapsed(
        name,
        send,
        3,
        &form,
        Payoff::ElapsedAtLeast("P1", "P2", 4, 4),
        bound,
        zero(),
    );
    // `M >= c` is not a digital elapsed form (FSpec FR-421): it waits on the
    // region process.
    for claim in &mut row.claims {
        claim.exact_expect = Expect::HandOrIncomplete;
    }
    row
}

/// `M` is 3 ms whenever `send` runs.
fn quantile_three() -> Row {
    let form = format!("quantile 0.5 of ({FOUR}) <= 3 ms");
    elapsed(
        "3463-quantile",
        3,
        3,
        &form,
        Payoff::ElapsedAtMost("P1", "P2", 4, 3),
        AtLeast(half()),
        one(),
    )
}

fn four_after_three() -> Row {
    at_least_four("3463-at-least-send-3", 3, ">= 0.5", AtLeast(half()))
}

fn four_after_five() -> Row {
    at_least_four("3463-at-most-send-5", 5, "<= 0.5", AtMost(half()))
}

#[test]
fn row_3463_elapsed_past_the_horizon_of_position_zero_hand() {
    assert_hand(&four_after_three());
    assert_hand(&quantile_three());
    assert_hand(&four_after_five());
}

#[test]
#[ignore = "QSL-3531: en-5 settles an elapsed event unsupported"]
fn row_3463_at_least_four_after_three_routes() {
    assert_routes(&four_after_three().exact_route());
}

#[trace("FR-254-AC-3")]
#[test]
fn row_3463_at_least_four_after_three_sampled() {
    assert_routes(&four_after_three().sampled_route());
}

#[test]
#[ignore = "QSL-3531: en-5 settles an elapsed quantile unsupported"]
fn row_3463_quantile_routes() {
    assert_routes(&quantile_three().exact_route());
}

#[trace("FR-254-AC-3")]
#[test]
fn row_3463_quantile_sampled() {
    assert_routes(&quantile_three().sampled_route());
}

#[test]
#[ignore = "QSL-3531: en-5 settles an elapsed event unsupported"]
fn row_3463_at_most_after_five_routes() {
    assert_routes(&four_after_five().exact_route());
}

#[trace("FR-254-AC-3")]
#[test]
fn row_3463_at_most_after_five_sampled() {
    assert_routes(&four_after_five().sampled_route());
}

// --- QSL-3448: no position at the horizon -----------------------------------

/// `send` at exactly 5 ms: positions 0 and 5 ms, none at 4 ms, so
/// `eventually[4, 4]` is false on the only behaviour.
fn horizon_without_a_position() -> Row {
    let form = "eventually[4 ms, 4 ms] holds(c.phase = 0)";
    let text = pair("Low", &format!("probability >= 0.5 [ {form} ]"))
        + &pair("High", &format!("probability <= 0.5 [ {form} ]"))
        + &formula("F", form);
    let mut space = Space::new(6, Lock::Frozen);
    space.fixed = vec![("send", whole(5))];
    let row = |name, sample, bound| {
        claim(name, Payoff::Holds("F"), bound, zero(), zero()).also_sampled(sample)
    };
    Row {
        name: "3448-empty-horizon",
        world: models::send_by_five(&text),
        space,
        claims: vec![
            row("Low", "LowS", AtLeast(half())),
            row("High", "HighS", AtMost(half())),
        ],
    }
}

#[test]
fn row_3448_empty_horizon_hand() {
    assert_hand(&horizon_without_a_position());
}

#[test]
fn row_3448_empty_horizon_routes() {
    assert_routes(&horizon_without_a_position());
}

// --- QSL-3468: a time-locked run ends with time frozen ----------------------

/// Nothing is ever enabled and time stops at 1 ms, so the run ends there
/// (FR-204 "Time-locks"): `late` stays false, `always[0, 2] not late` holds,
/// and so does `always[2, 3]`, which reads no position or only frozen ones
/// where `late` is false.
fn time_lock(invariant: &str) -> Row {
    let early = "always[0 ms, 2 ms] holds(not c.late)";
    let later = "always[2 ms, 3 ms] holds(not c.late)";
    let text = every("Early", &format!("probability >= 0.5 [ {early} ]"))
        + &every("Later", &format!("probability >= 0.5 [ {later} ]"))
        + &formula("FEarly", early)
        + &formula("FLater", later)
        + &formula("FPast", "eventually[2 ms, 3 ms] holds(true)");
    Row {
        name: if invariant.contains("<=") {
            "3468-lock-closed"
        } else {
            "3468-lock-strict"
        },
        world: models::time_locked(invariant, &text),
        space: Space::new(2, Lock::Frozen),
        claims: vec![
            claim(
                "Early",
                Payoff::Holds("FEarly"),
                AtLeast(half()),
                one(),
                one(),
            ),
            claim(
                "Later",
                Payoff::Holds("FLater"),
                AtLeast(half()),
                one(),
                one(),
            ),
        ],
    }
}

#[test]
fn row_3468_time_lock_hand() {
    for invariant in ["self.x <= 1 ms", "self.x < 1 ms"] {
        let row = time_lock(invariant);
        assert_hand(&row);
        // A time-locked run adds no later position (FSpec FR-416), so no
        // position is stamped in [2, 3] ms.
        let system = row.world.system();
        let past = falsifier::extrema(&system, &row.space, &|b| {
            whole(i64::from(b.stamps.iter().any(|stamp| stamp > &whole(1))))
                .add(&falsifier::holds(&row.world, &system, "FPast", b))
        });
        assert_eq!(
            past,
            Some(Extrema {
                min: zero(),
                max: zero()
            }),
            "{invariant}"
        );
    }
}

#[test]
#[ignore = "QSL-3468: en-5 refutes always at a time lock, whose value is 1"]
fn row_3468_time_lock_closed_routes() {
    assert_routes(&time_lock("self.x <= 1 ms"));
}

#[test]
#[ignore = "QSL-3468: en-5 refutes always at a time lock, whose value is 1"]
fn row_3468_time_lock_strict_routes() {
    assert_routes(&time_lock("self.x < 1 ms"));
}

// --- TC-374 `Split` and `Burst`, FR-254-AC-4 `Lock` -------------------------

/// `first` at `t`, then one of two equally likely branches. `early` runs only
/// where `t + 1 <= 1`, so `t = 0`; `tardy` only where `x = t >= 1` before any
/// time passes, so `t = 1`; every other run is time-locked before phase 2.
/// Phase 2 is reached by 3 ms with probability 1/2 at `t` 0 or 1 and 0 for
/// `t` strictly inside (0, 1): minimum 0, maximum 1/2, and integer times
/// alone give 1/2 for both. `probability >= 0.25` over every scheduler is
/// false.
fn split() -> Row {
    let form = "eventually[0 ms, 3 ms] holds(c.phase = 2)";
    let text = every("Split", &format!("probability >= 0.25 [ {form} ]")) + &formula("F", form);
    let mut space = Space::new(2, Lock::Frozen);
    space.drawn = vec!["first"];
    Row {
        name: "tc374-split",
        world: models::split(&text),
        space,
        claims: vec![claim(
            "Split",
            Payoff::Holds("F"),
            AtLeast(ratio(1, 4)),
            zero(),
            half(),
        )
        .exact_expects(Expect::HandOrIncomplete)],
    }
}

#[test]
fn row_split_reachable_lock_hand() {
    assert_hand(&split());
}

#[trace("FSpec-FR-421-AC-8")]
#[test]
#[ignore = "QSL-3494: en-5 proves minimum 1/2 on integer times where the dense minimum is 0"]
fn row_split_reachable_lock_routes() {
    assert_routes(&split());
}

/// `send` runs at 0 ms and the idle tail stutters after it at any positive
/// delays. With `m` stutter positions in (0, 1] the positions of [0, 1] ms
/// are the initial one and `1 + m` with phase 1: the fraction is
/// `(1 + m) / (2 + m)` for every `m >= 0`, from 1/2 to a supremum of 1. So
/// `<= 0.7` is false. The walk's idle delays 7/3 and 1/100 ms give `m` 0 and
/// 100: 1/2 and 101/102. Integer stamps allow `m <= 1`, at most 2/3.
fn burst() -> Row {
    let form = "mean of (fraction holds(c.phase = 1) over [0, 1 ms]) <= 0.7";
    let text = pair("Burst", form) + &phases();
    let mut space = Space::new(1, Lock::Frozen);
    space.fixed = vec![("send", whole(0))];
    space.idle = vec![ratio(7, 3), ratio(1, 100)];
    Row {
        name: "tc374-burst",
        world: models::send_reply(0, None, &text),
        space,
        claims: vec![claim(
            "Burst",
            Payoff::Fraction("P1", 1),
            AtMost(ratio(7, 10)),
            half(),
            ratio(101, 102),
        )
        .also_sampled("BurstS")
        .exact_expects(Expect::HandOrIncomplete)
        .sampled_expects(Expect::Declines("unsupported:NotStochasticIdleTail"))],
    }
}

#[test]
fn row_burst_stutter_fraction_hand() {
    assert_hand(&burst());
}

#[trace("FSpec-FR-421-AC-9")]
#[test]
#[ignore = "QSL-3531: en-5 settles the Burst fraction unsupported"]
fn row_burst_stutter_fraction_routes() {
    assert_routes(&burst().exact_route());
}

#[trace("FR-254-AC-8")]
#[test]
fn row_burst_stutter_fraction_sampled() {
    assert_routes(&burst().sampled_route());
}

/// `go` at 1/2 ms reaches phase 1, and the run is time-locked at 1 ms: the
/// position stamped 1/2 ms is in phase 1 on every sample, so the event has
/// probability 1 and `<= 0.5` is rejected with estimate 1 (FR-254-AC-4).
fn sampled_lock() -> Row {
    let form = "eventually[0 ms, 2 ms] holds(c.phase = 1)";
    let text =
        sampled("One", "LockS", &format!("probability <= 0.5 [ {form} ]")) + &formula("F", form);
    let mut space = Space::new(1, Lock::Frozen);
    space.delays = Some(vec![ratio(1, 2)]);
    space.fixed = vec![("go", ratio(1, 2))];
    Row {
        name: "254-ac4-sampled-lock",
        world: models::lock_after_a_step(&text),
        space,
        claims: vec![Claim {
            exact: None,
            sampled: Some("LockS"),
            exact_expect: Expect::Hand,
            sampled_expect: Expect::Hand,
            payoff: Payoff::Holds("F"),
            bound: AtMost(half()),
            min: one(),
            max: one(),
        }],
    }
}

#[test]
fn row_sampled_time_lock_hand() {
    assert_hand(&sampled_lock());
}

/// The sample is decided: rejected with estimate 1.
#[trace("FR-254-AC-4")]
#[test]
fn row_sampled_time_lock_routes() {
    let row = sampled_lock();
    let answer = routes::sampled(&row.world, "LockS");
    eprintln!("OBSERVED {} en-4: {:?}", row.name, answer.verdict);
    assert!(
        matches!(&answer.verdict, Verdict::Refuted(Some(estimate)) if estimate == &one()),
        "{:?}",
        answer.verdict
    );
}
