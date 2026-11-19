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
    polynomial_class_gap, HybridAnswer, HybridProvider, HybridReplayMeter, HybridSolveRequest,
    HybridVerdict, OptionRefusal, SolverCoverage, SolverMethod, SolverOutcome, SolverResult,
};

pub use options::OPTION_DEFAULTS;

/// The provider's backend identity.
pub const BACKEND: &str = "fsl-hybrid-reference";

/// The published `hybrid.max_solver_steps` default (FR-255).
pub const MAX_SOLVER_STEPS: u64 = 1_000_000;

/// The published `hybrid.max_solver_time` default, in seconds (FR-255).
pub const MAX_SOLVER_TIME: u64 = 60;

/// The work units the class check may spend expanding calls.
const CLASS_WORK_UNITS: u64 = 1_000_000;

/// The reference provider.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceProvider;

impl HybridProvider for ReferenceProvider {
    fn solve(&self, request: &HybridSolveRequest<'_>, _cancel: &Cancel) -> HybridAnswer {
        let mut meter = HybridReplayMeter::new(CLASS_WORK_UNITS);
        if let Ok(Some(construct)) = polynomial_class_gap(request.graph, request.model, &mut meter)
        {
            return HybridAnswer {
                verdict: HybridVerdict::Unsupported(construct),
                certificate: None,
            };
        }
        HybridAnswer::solved(SolverResult {
            method: SolverMethod::Unclassified,
            outcome: SolverOutcome::Unknown(None),
            coverage: SolverCoverage::Inductive,
        })
    }

    fn check_options(&self, options: &BTreeMap<String, String>) -> Result<(), OptionRefusal> {
        options::check(options)
    }
}

#[cfg(test)]
mod tests;
