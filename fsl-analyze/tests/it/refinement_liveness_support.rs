// SPDX-License-Identifier: AGPL-3.0-or-later
//! The refinement-liveness drivers that `refinement_liveness` and the isolated
//! `refinement_liveness_cx1` binary share.

use filament_exact::Cancel;
use fsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::ModelSimulationRequest;
use fsl_package::CheckedPackage;
use fsl_replay::{
    open_systems, replay_refinement_trace, LivenessHalf, ModelCheckLimits, ModelCheckOutcome,
    RefinementOutcome, RefinementTraceSettlement, SafetyOutcome, TemporalCounterexample,
};
use fsl_semantics::check::CheckedRefinement;
use fsl_semantics::model::observation::DocumentRef;

use crate::refinement_aux::{World as AuxWorld, UNLIMITED as AUX_UNLIMITED};

pub(crate) fn check_aux(world: &AuxWorld<'_>) -> RefinementOutcome {
    run(
        world.refinement(),
        world.concrete_request(AUX_UNLIMITED),
        world.package(),
        world.abstract_initials(),
    )
}

pub(crate) fn hidden_tick_refinement(rows: &str) -> String {
    format!(
        "refinement HiddenTick using inf abstract Spec concrete Impl {{\n\
         population Spec::ms from Impl::ms;\n\
         object Spec::M from Impl::M {{ v = self.v; }}\n\
         step Impl::M::tick -> any;\n\
         {rows}}}"
    )
}

pub(crate) fn run(
    refinement: &CheckedRefinement,
    concrete: ModelSimulationRequest<'_>,
    package: &CheckedPackage,
    abstract_initials: Vec<DocumentRef>,
) -> RefinementOutcome {
    check_refinement(
        RefinementCheckRequest {
            refinement,
            concrete: ModelSubject {
                simulation: concrete,
                proof_bounds: Vec::new(),
            },
            abstract_package: package,
            abstract_initial: abstract_initials.into(),
            abstract_protocol_binding: None,
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked")
}

pub(crate) fn holds(outcome: &RefinementOutcome) {
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    let Some(LivenessHalf::Checked(ModelCheckOutcome::HoldsReduced { reductions, .. })) =
        &outcome.liveness
    else {
        panic!("the liveness half holds: {:?}", outcome.liveness);
    };
    assert!(reductions.is_empty());
}

/// The lasso `outcome`'s liveness half refutes with.
pub(crate) fn lasso(outcome: RefinementOutcome) -> TemporalCounterexample {
    assert!(matches!(outcome.safety, SafetyOutcome::Holds { .. }));
    let Some(LivenessHalf::Checked(ModelCheckOutcome::Violated { counterexample, .. })) =
        outcome.liveness
    else {
        panic!("the liveness half is violated: {:?}", outcome.liveness);
    };
    *counterexample
}

/// The settlement FR-145 replay gives `payload` over `world`.
pub(crate) fn replayed(
    world: &AuxWorld<'_>,
    payload: &TemporalCounterexample,
) -> RefinementTraceSettlement {
    let systems = open_systems(
        world.refinement(),
        &world.concrete_request(AUX_UNLIMITED),
        world.package(),
        world.abstract_initials(),
    )
    .expect("the subjects open");
    replay_refinement_trace(world.refinement(), &systems, payload).expect("the trace replays")
}

/// Replay reproduces exactly the failure `payload` records.
pub(crate) fn assert_reproduced(world: &AuxWorld<'_>, payload: &TemporalCounterexample) {
    match replayed(world, payload) {
        RefinementTraceSettlement::Reproduced { failure, .. } => {
            assert_eq!(Some(failure), payload.refinement);
        }
        other => panic!("the failure reproduces: {other:?}"),
    }
}
