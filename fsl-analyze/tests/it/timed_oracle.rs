// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hand-valued dense-time oracle cases for timed analysis regressions.

use filament_exact::{Integer, Meter, Rational};
use fsl_eval::temporal::{
    evaluate_timed_at, AtomReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TimedTrace,
};
use fsl_semantics::check::FormulaNodeId;
use ix_trace_rs::trace;

use crate::timed_world::{self, World};

mod falsifier;
mod models;
mod probability;
mod routes;
mod temporal;

const TIMED: &str = "profile tt = \"filament.temporal.timed/v1\";\n";
const CLAIM: &str = "temporal Dense using tt over (c: M::Call) clock \"model-time\" on origin {\n\
    eventually[0 ms, 4 ms] holds(c.late)\n\
}\n";

fn exact(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("rational")
}

struct GridLetters {
    late: Vec<bool>,
}

impl TemporalLetters for GridLetters {
    fn atom(
        &mut self,
        _atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let index = usize::try_from(position.index()).expect("small test position");
        Ok(if self.late[index] {
            AtomReading::True
        } else {
            AtomReading::False
        })
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        Ok(AtomReading::True)
    }
}

fn world() -> World {
    timed_world::world(
        &timed_world::document(&[], &[]),
        TIMED,
        &format!("{}{CLAIM}", timed_world::DENSE),
    )
}

fn evaluate(world: &World, stamps: Vec<Rational>, late: Vec<bool>) -> TemporalOutcome {
    assert_eq!(stamps.len(), late.len());
    let trace = TimedTrace::through(stamps, exact(4, 1)).expect("complete timed trace");
    let clause = world.package.graph().temporal_clauses()[0].clause();
    let mut meter = Meter::new(world.system().meter_limits());
    evaluate_timed_at(
        clause,
        &trace,
        TemporalPosition::new(0),
        &mut GridLetters { late },
        &mut meter,
    )
    .expect("checked timed formula evaluates")
}

/// A dense falsifier probes several rational grids and compares each reading
/// with the hand value; expected answers never come from an analysis route.
#[trace("FR-234-AC-4", "FSpec-FR-416-AC-4")]
#[test]
fn bounded_eventually_matches_hand_values_on_multiple_rational_grids() {
    let world = world();
    for divisions in [2_i64, 3, 4, 5, 8] {
        let stamps: Vec<_> = (0..=4 * divisions).map(|at| exact(at, divisions)).collect();
        let late = vec![false; stamps.len()];
        assert!(
            matches!(
                evaluate(&world, stamps.clone(), late),
                TemporalOutcome::Violated { at } if at == TemporalPosition::new(0)
            ),
            "an all-false trace must falsify eventually on a {divisions}-part grid"
        );

        let mut late = vec![false; stamps.len()];
        late[usize::try_from(divisions).expect("positive grid")] = true;
        assert_eq!(
            evaluate(&world, stamps, late),
            TemporalOutcome::Holds,
            "a hand-placed true letter at 1 ms must satisfy eventually on a {divisions}-part grid"
        );
    }
}

/// Equal-stamp positions retain step order; inserting a repeated false step
/// does not change the hand-computed value at the origin.
#[trace("FR-234-AC-4", "FSpec-FR-416-AC-4")]
#[test]
fn equal_stamp_step_order_is_stutter_invariant_for_the_timed_value() {
    let world = world();
    assert_eq!(
        evaluate(
            &world,
            vec![exact(0, 1), exact(1, 1), exact(1, 1), exact(3, 1)],
            vec![false, false, true, false],
        ),
        TemporalOutcome::Holds
    );
    assert_eq!(
        evaluate(
            &world,
            vec![
                exact(0, 1),
                exact(1, 1),
                exact(1, 1),
                exact(1, 1),
                exact(3, 1),
            ],
            vec![false, false, false, true, false],
        ),
        TemporalOutcome::Holds
    );
}
