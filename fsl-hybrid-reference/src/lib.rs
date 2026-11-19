// SPDX-License-Identifier: AGPL-3.0-or-later
//! The first-party reference hybrid provider `fsl-hybrid-reference` (FR-1019,
//! ADR-011 layer H). It implements [`fsl_replay::HybridProvider`], which
//! `fsl-replay` defines so this crate sits below `fsl-analyze` with no
//! dependency cycle.
//!
//! Implemented: the class check (a model outside it answers `Unsupported`,
//! located), and the option domains `check_options` refuses. The flowpipe
//! method of FR-1019 "Method" is not implemented: a model inside the class is
//! answered `solver-inconclusive` with no trace, a result FR-250 settles as
//! `inconclusive` and which never proves or refutes. The answer names no
//! method, since none ran. FR-751's certificate checker is QSL-2531, so
//! nothing here is certified.

mod options;

use std::collections::BTreeMap;

use filament_exact::Cancel;
use fsl_replay::{
    polynomial_class_gap, HybridAnswer, HybridProvider, HybridReplayMeter, HybridReplayRefusal,
    HybridSolveRequest, HybridVerdict, OptionRefusal, SolverCoverage, SolverMethod, SolverOutcome,
    SolverResult, SolverStop,
};

pub use options::OPTION_DEFAULTS;

/// The provider's backend identity.
pub const BACKEND: &str = "fsl-hybrid-reference";

/// The published `hybrid.max_solver_steps` default (FR-255).
pub const MAX_SOLVER_STEPS: u64 = 1_000_000;

/// The published `hybrid.max_solver_time` default, in seconds (FR-255).
pub const MAX_SOLVER_TIME: u64 = 60;

/// The reference provider.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceProvider;

impl HybridProvider for ReferenceProvider {
    fn solve(&self, request: &HybridSolveRequest<'_>, _cancel: &Cancel) -> HybridAnswer {
        // Expanding a call is a solver step: the class check spends the
        // request's own step budget.
        let steps = request.budget.steps().unwrap_or(MAX_SOLVER_STEPS);
        let mut meter = HybridReplayMeter::new(steps);
        match polynomial_class_gap(request.graph, request.model, &mut meter) {
            Ok(Some(construct)) => HybridAnswer {
                verdict: HybridVerdict::Unsupported(construct),
                certificate: None,
            },
            Ok(None) => HybridAnswer::solved(SolverResult {
                method: SolverMethod::Unclassified,
                outcome: SolverOutcome::Unknown(None),
                coverage: SolverCoverage::Inductive,
            }),
            Err(HybridReplayRefusal::ResourceExhausted { .. }) => {
                HybridAnswer::solved(SolverResult {
                    method: SolverMethod::Unclassified,
                    outcome: SolverOutcome::Stopped(SolverStop::Steps(steps)),
                    coverage: SolverCoverage::Inductive,
                })
            }
            // The checked package holds no body for a call: the class cannot
            // be decided, so nothing is proved or refuted.
            Err(_) => HybridAnswer::solved(SolverResult {
                method: SolverMethod::Unclassified,
                outcome: SolverOutcome::Unknown(None),
                coverage: SolverCoverage::Inductive,
            }),
        }
    }

    fn check_options(&self, options: &BTreeMap<String, String>) -> Result<(), OptionRefusal> {
        options::check(options)
    }
}

#[cfg(test)]
mod tests;
