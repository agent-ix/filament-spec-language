// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-634 "Invariant findings": the states an enumeration leaves unexpanded
//! are evaluated for their findings alone. The evaluation computes and keys
//! no successor, runs nothing for a system with nothing to evaluate, and
//! never changes how the walk ended.

use filament_exact::{Meter, NodeKey, ScalarLimits, ValueType};
use filament_semantic_value::declaration::TypeEnvironment;
use fsl_eval::simulation::{
    enumerate_request, EnumerationBounds, EnumerationEnd, EnumerationRequest, Expansion,
    ExpansionStop, TransitionSystem, CANCELLED_STOP,
};
use fsl_foundation::IdentityLimits;
use ix_trace_rs::trace;
use quire_canonical::FixedShape;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, FixedShape)]
struct Step {
    #[serde(rename = "type")]
    kind: &'static str,
    operation: String,
    arguments: Vec<u64>,
}

/// States 0 and 1 chain; state 1's successor has a key with no RFC 8785
/// encoding (`2^53 + 1`), and `state_findings` reports `evaluate` for it.
struct Chain {
    evaluate: bool,
    /// The stop `state_findings` returns instead of findings, when set.
    stops: Option<ExpansionStop>,
}

impl TransitionSystem for Chain {
    type State = u64;
    type TransitionId = Step;
    type Key = u64;
    type Finding = String;

    fn initial(&self) -> Vec<u64> {
        vec![0]
    }

    fn key(&self, state: &u64) -> u64 {
        *state
    }

    fn successors(&self, state: &u64) -> Result<Expansion<Step, u64, String>, ExpansionStop> {
        let next = match state {
            0 => 1,
            _ => (1u64 << 53) + 1,
        };
        Ok(Expansion {
            successors: vec![(
                Step {
                    kind: "transition",
                    operation: "next".to_owned(),
                    arguments: vec![],
                },
                next,
            )],
            findings: vec![format!("expanded {state}")],
        })
    }

    fn state_findings(&self, state: &u64) -> Option<Result<Vec<String>, ExpansionStop>> {
        self.evaluate.then(|| match self.stops {
            Some(stop) => Err(stop),
            None => Ok(vec![format!("evaluated {state}")]),
        })
    }

    fn stopped_findings(&self, state: &u64, stop: ExpansionStop) -> Vec<String> {
        vec![format!("undetermined {state}: {}", stop.cause.code())]
    }
}

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

fn walk(system: &Chain, max_steps: usize) -> fsl_eval::simulation::Enumeration<Step, String> {
    walk_polled(system, max_steps, || false)
}

fn walk_polled(
    system: &Chain,
    max_steps: usize,
    poll: impl FnMut() -> bool,
) -> fsl_eval::simulation::Enumeration<Step, String> {
    let domains: [(NodeKey, &ValueType); 0] = [];
    let types = TypeEnvironment::new([], []).expect("an empty type environment admits");
    let mut meter = Meter::new(UNLIMITED);
    enumerate_request(
        system,
        EnumerationRequest {
            domains: &domains,
            types: &types,
            position_limit: 1000,
            bounds: EnumerationBounds {
                max_states: 100,
                max_steps,
                max_state_key_bytes: 1 << 20,
            },
            identity: IdentityLimits::default(),
        },
        &mut meter,
        poll,
    )
    .expect("a hostile key past the horizon is never encoded")
}

/// FR-634-AC-7: a system with nothing to evaluate keeps the end of the walk
/// and stores nothing for the state at the horizon, and a successor past the
/// horizon with no encoding does not refuse the enumeration.
#[trace("TC-2976", "FR-634-AC-7")]
#[test]
fn a_system_with_no_invariants_keeps_the_end_of_a_horizon_limited_walk() {
    let run = walk(
        &Chain {
            evaluate: false,
            stops: None,
        },
        1,
    );
    assert_eq!(run.end, EnumerationEnd::MaxSteps);
    assert_eq!(run.states.len(), 2);
    assert_eq!(run.states[0].findings, ["expanded 0"]);
    assert!(run.states[1].findings.is_empty());
}

/// FR-634 "Invariant findings": the state at the horizon is evaluated for
/// its findings alone, and the walk still ends `MaxSteps`.
#[trace("TC-2976", "FR-634-AC-6")]
#[test]
fn the_state_at_the_horizon_is_evaluated_without_expanding_it() {
    let run = walk(
        &Chain {
            evaluate: true,
            stops: None,
        },
        1,
    );
    assert_eq!(run.end, EnumerationEnd::MaxSteps);
    assert_eq!(run.states[1].findings, ["evaluated 1"]);
}

/// FR-634 "Invariant findings" (QSL-2427): a horizon state whose evaluation
/// stops reports each invariant undetermined with the stop as its cause, and
/// a cancellation that fires before the horizon state is evaluated reports
/// it undetermined with the cancellation stop; neither changes how the walk
/// ended.
#[trace("TC-2976", "FR-634-AC-6")]
#[test]
fn a_stopped_or_cancelled_horizon_evaluation_is_reported_undetermined() {
    let broken = ExpansionStop {
        cause: filament_semantic_value::catalog::CatalogCode::new(
            "runtime_invariant",
            "established-invariant-broken",
        ),
        limit: None,
    };
    let run = walk(
        &Chain {
            evaluate: true,
            stops: Some(broken),
        },
        1,
    );
    assert_eq!(run.end, EnumerationEnd::MaxSteps);
    assert_eq!(run.states[0].findings, ["expanded 0"]);
    assert_eq!(
        run.states[1].findings,
        ["undetermined 1: runtime_invariant"]
    );

    // The walk polls once before expanding state 0; the second poll, before
    // the horizon state is evaluated, reports the cancellation.
    let mut polls = 0;
    let run = walk_polled(
        &Chain {
            evaluate: true,
            stops: None,
        },
        1,
        || {
            polls += 1;
            polls > 1
        },
    );
    assert_eq!(run.end, EnumerationEnd::MaxSteps);
    assert_eq!(run.states[0].findings, ["expanded 0"]);
    assert_eq!(run.states[1].findings, ["undetermined 1: cancelled"]);
    assert_eq!(CANCELLED_STOP.cause.code(), "cancelled");
}
