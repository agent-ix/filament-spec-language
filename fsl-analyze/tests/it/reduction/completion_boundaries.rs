// SPDX-License-Identifier: AGPL-3.0-or-later
//! State-constraint boundaries remain stored without inventing successors.

use super::*;

fn constrained_count(clauses: &[Clause<'_>]) -> World {
    World::new(
        shape(false, 3),
        "pre Below using v on M::Counter::bump { self.v < 3 }\n\
         post Up using v on M::Counter::bump { self.v - pre(self.v) = 1 }\n\
         constraint Low using v on M::Counter at current { self.v <= 1 }\n\
         constraint Started using v on M::Counter at current { self.v >= 1 }\n",
        clauses,
        &[start([0, 0, 0])],
        &KEYS,
    )
}

#[trace("FR-158-AC-1", "FR-158-AC-2")]
#[test]
fn the_twentieth_stored_state_completes_the_constrained_cube() {
    let world = constrained_count(&[infinite("always holds(x.v <= 2)")]);
    // Eight expanding 0/1 triples and twelve triples with exactly one 2.
    for maximum in [1, 8, 19, 20, 21] {
        let limits = ModelCheckLimits {
            limits: Limits {
                max_states: maximum,
                ..Limits::default()
            },
            ..ModelCheckLimits::default()
        };
        let actual = world
            .run(
                Some(0),
                vec![atom(|v| Some(v <= 2))],
                constraint("Low"),
                None,
                limits,
            )
            .expect("the checked constrained subject admits");
        if maximum < 20 {
            assert_eq!(
                actual,
                ModelCheckOutcome::Stopped {
                    cause: IncompleteCause::ResourceExhausted,
                    limit: Some(ReachedLimit {
                        limit: ModelCheckLimit::MaxStates,
                        value: u64::try_from(maximum).expect("small budget"),
                    }),
                    stats: actual.stats(),
                },
                "budget={maximum}"
            );
            assert_eq!(
                actual.stats().product_states,
                u64::try_from(maximum).expect("small budget")
            );
        } else {
            assert_eq!(
                actual,
                ModelCheckOutcome::Reduction {
                    cause: ReductionCause::ConstraintReached {
                        boundary_states: 12
                    },
                    stats: actual.stats(),
                }
            );
            assert_eq!(actual.stats().product_states, 20);
            assert_eq!(actual.stats().product_edges, 24);
        }
    }
}

#[trace("FR-158-AC-2", "FR-158-AC-3")]
#[test]
fn an_initial_boundary_never_becomes_a_deadlock_or_a_safety_stutter() {
    let world = constrained_count(&[
        infinite("always holds(x.v <= 3)"),
        infinite("eventually holds(x.v = 3)"),
    ]);
    for selection in [None, Some(0)] {
        let atoms = match selection {
            None => Vec::new(),
            Some(0) => vec![atom(|v| Some(v <= 3))],
            _ => unreachable!("listed selections"),
        };
        let actual = world
            .run(
                selection,
                atoms,
                constraint("Started"),
                None,
                ModelCheckLimits {
                    limits: Limits {
                        max_states: 1,
                        ..Limits::default()
                    },
                    ..ModelCheckLimits::default()
                },
            )
            .expect("one initial state fits exactly");
        assert_eq!(
            actual,
            ModelCheckOutcome::Reduction {
                cause: ReductionCause::ConstraintReached { boundary_states: 1 },
                stats: actual.stats(),
            }
        );
        assert_eq!(actual.stats().product_states, 1);
        assert_eq!(actual.stats().product_edges, 0);
        assert_eq!(actual.stats().stutter_edges, 0);
    }
}

#[trace("FR-158-AC-2", "FR-158-AC-3")]
#[test]
#[ignore = "QSL-2713: oracle contradicts FR-126 Bindings (budgets shared, product_states summed over the 3 Counter bindings): needs a spec ruling"]
fn a_liveness_initial_boundary_never_gains_a_stutter_lasso() {
    let world = constrained_count(&[
        infinite("always holds(x.v <= 3)"),
        infinite("eventually holds(x.v = 3)"),
    ]);
    let actual = world
        .run(
            Some(1),
            vec![atom(|v| Some(v == 3))],
            constraint("Started"),
            None,
            ModelCheckLimits {
                limits: Limits {
                    max_states: 1,
                    ..Limits::default()
                },
                ..ModelCheckLimits::default()
            },
        )
        .expect("one initial boundary fits exactly");
    assert_eq!(
        actual,
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::ConstraintReached { boundary_states: 1 },
            stats: actual.stats(),
        }
    );
    assert_eq!(actual.stats().product_states, 1);
    assert_eq!(actual.stats().product_edges, 0);
    assert_eq!(actual.stats().stutter_edges, 0);
}

#[trace("FR-158-AC-1", "FR-158-AC-3")]
#[test]
fn a_false_initial_letter_refutes_before_the_initial_constraint_cuts() {
    let world = constrained_count(&[infinite("always holds(x.v = 1)")]);
    let outcome = run(
        &world,
        Some(0),
        vec![atom(|v| Some(v == 1))],
        constraint("Started"),
    );
    let evidence = violation(&outcome);
    assert_eq!(evidence.kind, CounterexampleKind::Formula);
    assert!(evidence.prefix.is_empty());
    assert_eq!(path(&world, &outcome), [(0, 0, 0)]);
    assert_eq!(outcome.stats().product_states, 1);
    assert_eq!(outcome.stats().product_edges, 0);
    assert_eq!(
        world
            .settle_clause(0, vec![atom(|v| Some(v == 1))], &outcome)
            .value,
        TerminalValue::Refuted
    );
}

#[trace("FR-158-AC-2", "FR-158-AC-3")]
#[test]
fn a_liveness_initial_boundary_is_stored_per_binding_and_never_stutter_extended() {
    let world = constrained_count(&[
        infinite("always holds(x.v <= 3)"),
        infinite("eventually holds(x.v = 3)"),
    ]);
    // FR-126 "Bindings and ranking": the three `Counter` bindings share the
    // run's `max_states` and `product_states` is the sum over them, so three
    // stored states is the exact budget for one initial boundary state each.
    let actual = world
        .run(
            Some(1),
            vec![atom(|v| Some(v == 3))],
            constraint("Started"),
            None,
            ModelCheckLimits {
                limits: Limits {
                    max_states: 3,
                    ..Limits::default()
                },
                ..ModelCheckLimits::default()
            },
        )
        .expect("one initial boundary per binding fits exactly");
    assert_eq!(
        actual,
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::ConstraintReached { boundary_states: 1 },
            stats: actual.stats(),
        }
    );
    assert_eq!(actual.stats().product_states, 3);
    assert_eq!(actual.stats().product_edges, 0);
    assert_eq!(actual.stats().stutter_edges, 0);
}

#[trace("FR-158-AC-2")]
#[test]
fn a_liveness_claim_under_a_constraint_settles_constraint_reached_never_holds() {
    let world = constrained_count(&[
        infinite("always holds(x.v <= 3)"),
        infinite("eventually holds(x.v = 3)"),
    ]);
    let actual = world
        .run(
            Some(1),
            vec![atom(|v| Some(v == 3))],
            constraint("Low"),
            None,
            ModelCheckLimits::default(),
        )
        .expect("the constrained liveness run admits");
    // The unconstrained run holds (FR-158-AC-2); under `Low` no boundary
    // state is stutter-extended, so no lasso forms and the cut is reported.
    assert_eq!(
        actual,
        ModelCheckOutcome::Reduction {
            cause: ReductionCause::ConstraintReached {
                boundary_states: 12
            },
            stats: actual.stats(),
        }
    );
    assert_eq!(actual.stats().stutter_edges, 0);
}
