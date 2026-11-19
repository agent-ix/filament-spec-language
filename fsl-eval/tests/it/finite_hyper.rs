// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-898's finite trace population quantifier core.

use fsl_eval::hyper::{
    FiniteFalseTuple, FiniteHyperOutcome,
    FiniteQuantifier::{Exists, Forall},
    FiniteTrace, FiniteTraceClosure, FiniteTracePopulation, FiniteTupleOutcome,
    evaluate_finite_hyper, replays_finite_refutation,
};
use ix_trace_rs::trace;

fn population() -> FiniteTracePopulation<i32> {
    FiniteTracePopulation::new(
        "population-1",
        "example/Obs",
        FiniteTraceClosure::Closed,
        vec![
            FiniteTrace {
                identity: "a".into(),
                records: vec![1],
            },
            FiniteTrace {
                identity: "b".into(),
                records: vec![2],
            },
        ],
    )
    .expect("distinct identities form a population")
}

fn deterministic_three_trace_population() -> FiniteTracePopulation<(u8, u8)> {
    FiniteTracePopulation::new(
        "population-3",
        "example/Obs",
        FiniteTraceClosure::Closed,
        vec![
            FiniteTrace {
                identity: "a".into(),
                records: vec![(0, 0), (1, 1)],
            },
            FiniteTrace {
                identity: "b".into(),
                records: vec![(0, 0), (1, 1)],
            },
            FiniteTrace {
                identity: "c".into(),
                records: vec![(2, 0), (2, 0)],
            },
        ],
    )
    .expect("the three trace identities are distinct")
}

fn deterministic_body(tuple: &[&FiniteTrace<(u8, u8)>]) -> FiniteTupleOutcome {
    for position in 0..tuple
        .iter()
        .map(|trace| trace.records.len())
        .min()
        .unwrap_or(0)
    {
        let input = tuple[0].records[position].0;
        let output = tuple[0].records[position].1;
        if tuple
            .iter()
            .skip(1)
            .any(|trace| trace.records[position].0 == input && trace.records[position].1 != output)
        {
            return FiniteTupleOutcome::False {
                position: u64::try_from(position).expect("test position fits"),
            };
        }
    }
    if tuple.iter().map(|trace| trace.records.len()).min()
        != tuple.iter().map(|trace| trace.records.len()).max()
    {
        return FiniteTupleOutcome::Incomplete;
    }
    FiniteTupleOutcome::True
}

#[trace("TC-2535", "FR-898-AC-3")]
#[test]
fn three_trace_determinism_proves_and_names_a_violating_pair() {
    let population = deterministic_three_trace_population();
    let quantifiers = [(Forall, "a".into()), (Forall, "b".into())];
    assert_eq!(
        evaluate_finite_hyper(&population, 3, &quantifiers, deterministic_body),
        FiniteHyperOutcome::Proved
    );

    let violating = FiniteTracePopulation {
        traces: population
            .traces
            .iter()
            .cloned()
            .map(|mut trace| {
                if trace.identity == "c" {
                    trace.records[1] = (1, 2);
                }
                trace
            })
            .collect(),
        ..population
    };
    assert_eq!(
        evaluate_finite_hyper(&violating, 3, &quantifiers, deterministic_body),
        FiniteHyperOutcome::Refuted {
            false_tuples: vec![FiniteFalseTuple {
                traces: vec!["a".into(), "c".into()],
                position: 1,
            }],
        }
    );
}

#[trace("TC-1786", "FR-739-AC-1")]
#[test]
fn finite_population_rejects_empty_and_duplicate_trace_identities() {
    let row = |identity: &str| FiniteTrace {
        identity: identity.into(),
        records: vec![1],
    };
    assert_eq!(
        FiniteTracePopulation::new(
            "population-1",
            "example/Obs",
            FiniteTraceClosure::Closed,
            vec![row(""), row("b")],
        ),
        Err(fsl_eval::hyper::FinitePopulationError::EmptyTraceIdentity)
    );
    assert_eq!(
        FiniteTracePopulation::new(
            "population-1",
            "example/Obs",
            FiniteTraceClosure::Closed,
            vec![row("a"), row("a")],
        ),
        Err(
            fsl_eval::hyper::FinitePopulationError::DuplicateTraceIdentity {
                identity: "a".into(),
            }
        )
    );
}

#[trace("TC-2535", "FR-898-AC-4")]
#[test]
fn finite_quantifiers_preserve_source_order_and_allow_reused_traces() {
    let population = population();
    let forall_exists = evaluate_finite_hyper(
        &population,
        2,
        &[(Forall, "a".into()), (Exists, "b".into())],
        |tuple| {
            if tuple[0].records[0] == tuple[1].records[0] {
                FiniteTupleOutcome::True
            } else {
                FiniteTupleOutcome::False { position: 0 }
            }
        },
    );
    assert_eq!(forall_exists, FiniteHyperOutcome::Proved);

    let exists_forall = evaluate_finite_hyper(
        &population,
        2,
        &[(Exists, "b".into()), (Forall, "a".into())],
        |tuple| {
            if tuple[0].records[0] == tuple[1].records[0] {
                FiniteTupleOutcome::True
            } else {
                FiniteTupleOutcome::False { position: 0 }
            }
        },
    );
    assert_eq!(
        exists_forall,
        FiniteHyperOutcome::Refuted {
            false_tuples: vec![
                fsl_eval::hyper::FiniteFalseTuple {
                    traces: vec!["a".into(), "b".into()],
                    position: 0,
                },
                fsl_eval::hyper::FiniteFalseTuple {
                    traces: vec!["b".into(), "a".into()],
                    position: 0,
                },
            ],
        }
    );
}

#[trace("TC-2535", "FR-898-AC-4")]
#[test]
fn finite_population_bound_open_closure_and_incomplete_body_are_preserved() {
    let population = population();
    assert_eq!(
        evaluate_finite_hyper(&population, 1, &[(Forall, "a".into())], |_| {
            panic!("over-bound populations must not evaluate the body")
        }),
        FiniteHyperOutcome::RequiresBound { bound: 1, found: 2 }
    );

    let open = FiniteTracePopulation {
        closure: FiniteTraceClosure::Open,
        ..population.clone()
    };
    assert_eq!(
        evaluate_finite_hyper(&open, 2, &[(Forall, "a".into())], |_| {
            FiniteTupleOutcome::True
        }),
        FiniteHyperOutcome::Inconclusive
    );
    assert_eq!(
        evaluate_finite_hyper(&open, 2, &[(Forall, "a".into())], |_| {
            FiniteTupleOutcome::False { position: 0 }
        }),
        FiniteHyperOutcome::Refuted {
            false_tuples: vec![FiniteFalseTuple {
                traces: vec!["a".into()],
                position: 0,
            }],
        }
    );
    assert_eq!(
        evaluate_finite_hyper(&open, 2, &[(Exists, "a".into())], |_| {
            FiniteTupleOutcome::False { position: 0 }
        }),
        FiniteHyperOutcome::Inconclusive
    );
    assert_eq!(
        evaluate_finite_hyper(&open, 2, &[(Forall, "a".into())], |_| {
            FiniteTupleOutcome::False { position: 0 }
        }),
        FiniteHyperOutcome::Inconclusive
    );
    assert_eq!(
        evaluate_finite_hyper(&population, 2, &[(Forall, "a".into())], |_| {
            FiniteTupleOutcome::Incomplete
        }),
        FiniteHyperOutcome::Inconclusive
    );
}

#[trace("TC-2535", "FR-898-AC-6")]
#[test]
fn finite_refutation_replay_rejects_edited_trace_identities_and_positions() {
    let population = population();
    let quantifiers = [(Forall, "a".into()), (Forall, "b".into())];
    let counterexample = [FiniteFalseTuple {
        traces: vec!["a".into(), "b".into()],
        position: 1,
    }];
    let body = |tuple: &[&FiniteTrace<i32>]| {
        if tuple[0].identity == "a" && tuple[1].identity == "b" {
            FiniteTupleOutcome::False { position: 1 }
        } else {
            FiniteTupleOutcome::True
        }
    };
    assert!(replays_finite_refutation(
        &population,
        2,
        &quantifiers,
        &counterexample,
        body,
    ));

    let mut edited = counterexample.clone();
    edited[0].traces[0] = "b".into();
    assert!(!replays_finite_refutation(
        &population,
        2,
        &quantifiers,
        &edited,
        body,
    ));

    let mut edited = counterexample;
    edited[0].position = 0;
    assert!(!replays_finite_refutation(
        &population,
        2,
        &quantifiers,
        &edited,
        body,
    ));
}
