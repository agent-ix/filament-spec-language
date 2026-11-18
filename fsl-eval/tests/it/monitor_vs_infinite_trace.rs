// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251-AC-41 (TC-1081): for `once`, `historically`, `since` and
//! `triggered` with no upper bound, every definite verdict of the tick
//! monitor equals the verdict of the infinite-trace evaluator on the same
//! trace; an `indeterminate` monitor verdict is not compared. The monitor
//! runs on a position counter (one tick per position, uncertainty 0).

use std::collections::BTreeMap;

use filament_exact::{Integer, Meter, Rational};
use fsl_eval::temporal::{
    evaluate_at, TemporalOutcome, TemporalPosition, TickBound, TickMonitorReference,
    TickMonitorSpec, TickOutcome, TickStep,
};
use fsl_foundation::bound::TemporalUpper;
use fsl_semantics::check::{CheckedInterval, FormulaNode, SCALAR_LIMITS_UNLIMITED};
use ix_trace_rs::trace;

use crate::temporal_trace::{counter, counters, finite, Fixture, Letters, INFINITE};

fn int(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

/// The level an atom `holds(c.level = k)` reads.
fn atom_level(fixture: &Fixture, node: &FormulaNode) -> i64 {
    let FormulaNode::Holds { span } = node else {
        panic!("a holds atom");
    };
    let text = &fixture.text[span.start..span.end];
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    digits.parse().expect("a level")
}

/// The monitor's definite verdict per position against the batch verdict.
fn agree(body: &str, levels: &[i64]) {
    let fixture = counter(INFINITE, body);
    let formula = fixture.clause.formula();
    let mut bounds = BTreeMap::new();
    for operator in fixture.clause.operators() {
        if let Some(CheckedInterval::Steps(interval)) = &operator.interval {
            if let TemporalUpper::Open = interval.upper() {
                bounds.insert(
                    operator.node.index(),
                    TickBound {
                        lower: int(i64::try_from(interval.lower()).expect("a small bound")),
                        lower_closed: true,
                        upper: None,
                        upper_closed: false,
                    },
                );
            }
        }
    }
    let root = formula.root().index();
    let mut capacities = BTreeMap::new();
    capacities.insert(root, 64);
    let mut monitor = TickMonitorReference::new(TickMonitorSpec {
        formula,
        activation: fixture.clause.activation(),
        bounds,
        uncertainty: int(0),
        width_bits: 20,
        max_reading_gap: 1,
        capacities,
        activation_capacity: 8,
        unprojectable: Default::default(),
    });
    let atoms: Vec<(usize, i64)> = formula
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
        .map(|(index, node)| (index, atom_level(&fixture, node)))
        .collect();
    let mut definite = Vec::new();
    for (position, level) in levels.iter().enumerate() {
        let step = TickStep::Reading {
            reading: u64::try_from(position).expect("a position"),
            atoms: atoms
                .iter()
                .map(|(index, k)| (*index, level == k))
                .collect(),
            guard: true,
        };
        for result in monitor.step(&step).expect("the reading applies") {
            if result.subformula == root {
                let at = usize::try_from(result.origin).expect("a tick");
                match result.outcome {
                    TickOutcome::True => definite.push((at, true)),
                    TickOutcome::False => definite.push((at, false)),
                    _ => {}
                }
            }
        }
    }
    // A finite prefix settles `Pending` where no bad prefix shows (FR-328), so
    // `Holds` and `Pending` both read as true here and `Violated` as false.
    for (at, holds) in definite {
        let mut letters = Letters::new(&fixture, counters(levels));
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let batch = evaluate_at(
            &fixture.clause,
            finite(u64::try_from(levels.len()).expect("a length")),
            TemporalPosition::new(u64::try_from(at).expect("a position")),
            &mut letters,
            &mut meter,
        )
        .expect("the trace is readable");
        assert_eq!(
            !matches!(batch, TemporalOutcome::Violated { .. }),
            holds,
            "{body} at {at} over {levels:?}: monitor {holds}, batch {batch:?}"
        );
    }
}

/// Agree on `shapes` over a seeded family of random level traces.
fn agree_on(shapes: &[String]) {
    let mut state = 17_u64;
    let mut next = |bound: u64| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        i64::try_from((state >> 33) % bound).expect("a level")
    };
    for shape in shapes {
        for _ in 0..40 {
            let levels: Vec<i64> = (0..12).map(|_| next(4)).collect();
            agree(shape, &levels);
        }
    }
}

#[trace("TC-1081", "FR-251-AC-41")]
#[test]
fn definite_monitor_verdicts_equal_the_infinite_trace_evaluator() {
    let (p, q) = ("holds(c.level = 1)", "holds(c.level = 2)");
    let mut shapes = vec![format!("{q} since {p}"), format!("{q} triggered {p}")];
    for bound in ["", "[1,*]", "[2,*]", "[3,*]"] {
        shapes.push(format!("once{bound} {p}"));
        shapes.push(format!("historically{bound} {p}"));
    }
    agree_on(&shapes);
}

/// ADR-018 IV-2 and FR-251 read `p since[a,*] q` with `p` required only on
/// the positions `j < k <= i - a`. The tick monitor folds `p` over every
/// position after the witness, the newest `a` included, for every `since`,
/// `triggered`, `until` and `release` with `a > 0`, so the two disagree.
#[trace("TC-1081", "FR-251-AC-41")]
#[test]
#[ignore = "QSL-2321: the monitor requires the left operand on the newest a positions; ADR-018 IV-2 and FR-251 exclude them (next spec batch / follow-up ticket)"]
fn definite_monitor_verdicts_equal_the_infinite_trace_evaluator_with_a_lower_bound_on_since() {
    let (p, q) = ("holds(c.level = 1)", "holds(c.level = 2)");
    let mut shapes = Vec::new();
    for bound in ["[1,*]", "[2,*]", "[3,*]"] {
        shapes.push(format!("{q} since{bound} {p}"));
        shapes.push(format!("{q} triggered{bound} {p}"));
    }
    agree_on(&shapes);
}
