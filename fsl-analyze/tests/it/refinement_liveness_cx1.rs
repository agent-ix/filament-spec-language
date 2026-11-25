// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 (TC-548): isolated liveness refinement test with a large search.

use fsl_eval::simulation::model::operation_identity_text;
use fsl_replay::{
    RefinementFailure, RefinementTraceSettlement, SafetyOutcome, TemporalCounterexample,
};
use ix_trace_rs::trace;

use crate::refinement_aux::Unit;
use crate::refinement_liveness_support::{
    assert_reproduced, check_aux, hidden_tick_refinement, holds, lasso, replayed,
};

/// The operation identities of `payload`'s stem and of its loop.
fn stem_and_loop(payload: &TemporalCounterexample) -> (Vec<String>, Vec<String>) {
    let entry = payload.lasso.as_ref().expect("a lasso").entry;
    let identity = |step: &fsl_replay::ModelStep| step.transition.operation().to_owned();
    (
        payload.prefix[..entry].iter().map(identity).collect(),
        payload.prefix[entry..].iter().map(identity).collect(),
    )
}

/// FR-899-AC-6 / TC-2536 CX-1: an abstract fire taken only along a candidate
/// path that cannot recur does not make the concrete tick loop fair.
#[trace("TC-2536", "FR-899-AC-6")]
#[test]
fn cx1_refutes_when_fire_only_leaves_the_candidate_cycle() {
    let contracts = "pre AdvanceEnabled using v on Spec::M::advance { self.h = 0 }\n\
        post AdvanceStep using v on Spec::M::advance { self.h = 1 and self.v = pre(self.v) }\n\
        post FireStep using v on Spec::M::fire {\n\
          (pre(self.h) = 1 and self.h = 2 and self.v = pre(self.v)) or\n\
          (pre(self.h) != 1 and self.h = pre(self.h) and self.v = 1)\n\
        }\n";
    let unit = Unit::hidden_fairness_model(2, 0, &["advance", "fire"], contracts);
    let refinement = hidden_tick_refinement("ensure fair weak Spec::M::fire;\n");
    let world = unit.world(&refinement);
    let outcome = check_aux(&world);
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    let counterexample = lasso(outcome);
    assert!(matches!(
        counterexample.refinement,
        Some(RefinementFailure::HiddenUnfair { .. })
    ));
    let (stem, cycle) = stem_and_loop(&counterexample);
    let tick = operation_identity_text(&unit.concrete_key("M/tick")).expect("the tick identity");
    assert_eq!(stem, [tick.clone(), tick.clone()]);
    assert_eq!(cycle, [tick]);
    assert_reproduced(&world, &counterexample);

    // With `fire` also mapping `h = 2` to `h = 1` keeping `v`, the cycle
    // `1 -> 2 -> 1` is a fair candidate run: the check holds and the earlier
    // counterexample no longer reproduces.
    let edited_contracts = contracts.replace(
        "(pre(self.h) = 1 and self.h = 2 and self.v = pre(self.v)) or\n",
        "(pre(self.h) = 1 and self.h = 2 and self.v = pre(self.v)) or\n\
          (pre(self.h) = 2 and self.h = 1 and self.v = pre(self.v)) or\n",
    );
    assert_ne!(edited_contracts, contracts, "the fire row is edited");
    let edited_unit = Unit::hidden_fairness_model(2, 0, &["advance", "fire"], &edited_contracts);
    let edited_world = edited_unit.world(&refinement);
    holds(&check_aux(&edited_world));
    assert!(
        !matches!(
            replayed(&edited_world, &counterexample),
            RefinementTraceSettlement::Reproduced { .. }
        ),
        "the earlier counterexample does not reproduce once the cycle is fair"
    );
}
