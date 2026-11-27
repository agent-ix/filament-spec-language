// SPDX-License-Identifier: AGPL-3.0-or-later
//! UE-6, statistical route: `inc`'s precondition has no value at the
//! reachable state `v = 1`. ADR-018 UE-6: no claim over the subject is
//! proved or accepted, even one decided before that state.

use filament_semantic_value::checking::CheckMode;
use fsl_replay::statistical::{Decision, StatisticalMethod};
use fsl_replay::UndefinedLocus;
use ix_trace_rs::trace;

use crate::prob_world::{self, World};

fn world(claim: &str) -> World {
    let mut model = prob_world::undetermined(claim);
    model.body = format!(
        "function undefined_remainder using vc(x: Integer): Integer pure {{ 6 mod x }}\n\
         pre CanInc using vc on M::Cell::inc {{ undefined_remainder(1 - self.v) >= 0 }}\n\
         post Inc using vc on M::Cell::inc {{ self.v - pre(self.v) = 1 }}\n\
         workload Even on M {{ weight Cell::inc = 1; }}\n{claim}"
    );
    World::new_with_mode(&model, CheckMode::Kernel)
}

/// FR-189-AC-21: the walk reaches the state whose `inc` guard `CanInc` has no
/// value and the test is Rejected with that contract.
#[trace("FR-189-AC-21")]
#[test]
fn ue6_statistical_walk_through_the_state() {
    let claim = "probabilistic Safe using ev on M over (c: M::Cell) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,2] holds(c.v >= 0) ] }";
    let verdict =
        crate::statistical_model::decided(&world(claim), "Safe", StatisticalMethod::Okamoto);
    assert_eq!(verdict.decision, Decision::Rejected);
    let [test] = &verdict.tests[..] else {
        panic!("one test: {:?}", verdict.tests);
    };
    let undefined = test.undefined.as_ref().expect("the undefined contract");
    let site = &undefined.evaluation.site;
    assert!(
        matches!(&site.locus, UndefinedLocus::Contract(clause) if clause == "CanInc"),
        "{:?}",
        site.locus
    );
    assert!(site.transition.is_some(), "the guard names its transition");
}

/// The atom `undefined_remainder(c.v) >= 0` has no value at position 0
/// (`v = 0`), which the walk reads whatever the horizon (ADR-018 UE-1). No
/// criterion states UE-1 alone, so the test carries no criterion tag.
#[test]
fn ue6_statistical_atom_undefined_at_position_zero() {
    let claim = "probabilistic Safe using ev on M over (c: M::Cell) under Even on origin \
         with alpha 0.01, beta 0.01, indifference 0.1 \
         { probability >= 0.5 [ always[0,0] holds(undefined_remainder(c.v) >= 0) ] }";
    let verdict =
        crate::statistical_model::decided(&world(claim), "Safe", StatisticalMethod::Okamoto);
    assert_eq!(verdict.decision, Decision::Rejected);
}
