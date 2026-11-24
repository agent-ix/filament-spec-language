// SPDX-License-Identifier: AGPL-3.0-or-later
//! Timed MeanTime's checked undefined-path settlement seam.

use super::*;
use crate::exact_probabilistic::UndefinedPath;
use fsl_eval::automaton::{Monitor, PropertyAutomaton};
use fsl_eval::simulation::model::ModelSimulationRequest;

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    simulation: ModelSimulationRequest<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    path: UndefinedPath,
    monitor: Option<Monitor<PropertyAutomaton>>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    undefined::run(
        simulation, claim, path, monitor, limits, cancel, subject, tamper,
    )
}
