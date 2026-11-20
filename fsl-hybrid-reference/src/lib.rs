// SPDX-License-Identifier: AGPL-3.0-or-later
//! The first-party reference hybrid provider `fsl-hybrid-reference` (FR-1019,
//! ADR-011 layer H). It implements [`fsl_replay::HybridProvider`], which
//! `fsl-replay` defines so this crate sits below `fsl-analyze` with no
//! dependency cycle.
//!
//! The class check answers a model outside the class `Unsupported`, located;
//! `check_options` refuses an option value outside its domain; a model inside
//! the class goes to the flowpipe solver of FR-1019 "Method" (`solver`), whose
//! `proved` answers carry a certificate the solver's own `check_certificate`
//! accepted and whose refutations carry a trace exact replay reproduces or a
//! validated witness (`refute`). The core checks every certificate again.

mod arith;
mod options;
mod refute;
mod solver;
mod wire;

use std::collections::BTreeMap;

use filament_exact::Cancel;
use fsl_replay::{
    polynomial_class_gap, HybridAnswer, HybridProvider, HybridReplayMeter, HybridSolveRequest,
    HybridVerdict, OptionRefusal, SolverCoverage, SolverMethod, SolverOutcome, SolverResult,
};

pub use options::OPTION_DEFAULTS;

/// The provider's backend identity.
pub const BACKEND: &str = "fsl-hybrid-reference";

/// FR-286: this provider produces no diagnostic cause pairs of its own.
pub const CATALOG_PAIRS: &[fsl_foundation::diagnostic::CatalogCode] = &[];

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
    fn solve(&self, request: &HybridSolveRequest<'_>, cancel: &Cancel) -> HybridAnswer {
        // Expanding a call is no Picard attempt, so the class check has a
        // bound of its own and spends none of `hybrid.max_solver_steps`.
        let mut meter = HybridReplayMeter::new(CLASS_WORK_UNITS);
        match polynomial_class_gap(request.graph, request.model, &mut meter) {
            Ok(Some(construct)) => HybridAnswer {
                verdict: HybridVerdict::Unsupported(construct),
                certificate: None,
            },
            Ok(None) => solver::solve(request, cancel),
            // A class check that could not finish (its own bound spent, or no
            // body for a call) leaves the class unestablished: nothing is
            // proved or refuted.
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
