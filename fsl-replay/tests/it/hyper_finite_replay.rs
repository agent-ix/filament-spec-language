// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-898 finite trace-set counterexample replay.

use fsl_eval::hyper::{
    FiniteFalseTuple, FiniteQuantifier::Forall, FiniteTrace, FiniteTraceClosure,
    FiniteTracePopulation, FiniteTupleOutcome,
};
use fsl_replay::replay_finite_hyper_refutation;
use ix_trace_rs::trace;

fn population() -> FiniteTracePopulation<(u8, u8)> {
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
                records: vec![(2, 0), (1, 2)],
            },
        ],
    )
    .expect("the three trace identities are distinct")
}

fn determinism(tuple: &[&FiniteTrace<(u8, u8)>]) -> FiniteTupleOutcome {
    let length = tuple
        .iter()
        .map(|trace| trace.records.len())
        .min()
        .unwrap_or(0);
    for position in 0..length {
        let (input, output) = tuple[0].records[position];
        if tuple[1].records[position].0 == input && tuple[1].records[position].1 != output {
            return FiniteTupleOutcome::False {
                position: u64::try_from(position).expect("test position fits"),
            };
        }
    }
    if tuple[0].records.len() != tuple[1].records.len() {
        return FiniteTupleOutcome::Incomplete;
    }
    FiniteTupleOutcome::True
}

#[trace("TC-2535", "FR-898-AC-6")]
#[test]
fn finite_hyper_refutation_replays_only_for_its_trace_tuple_and_position() {
    let population = population();
    let quantifiers = [(Forall, "a".into()), (Forall, "b".into())];
    let counterexample = [FiniteFalseTuple {
        traces: vec!["a".into(), "c".into()],
        position: 1,
    }];
    assert!(replay_finite_hyper_refutation(
        &population,
        3,
        &quantifiers,
        &counterexample,
        determinism,
    ));

    let mut edited_trace = counterexample.clone();
    edited_trace[0].traces[1] = "b".into();
    assert!(!replay_finite_hyper_refutation(
        &population,
        3,
        &quantifiers,
        &edited_trace,
        determinism,
    ));

    let mut edited_position = counterexample;
    edited_position[0].position = 0;
    assert!(!replay_finite_hyper_refutation(
        &population,
        3,
        &quantifiers,
        &edited_position,
        determinism,
    ));
}
