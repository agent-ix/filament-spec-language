// SPDX-License-Identifier: AGPL-3.0-or-later
//! The first-party reference hybrid provider `fsl-hybrid-reference` (FR-1019,
//! ADR-011 layer H). It implements [`fsl_replay::HybridProvider`], which
//! `fsl-replay` defines so this crate sits below `fsl-analyze` with no
//! dependency cycle.
//!
//! Nothing here is certified yet. FR-751's certificate checker is a later
//! slice, so a certificate-bearing answer settles `proved`/`uncertified` and
//! its certificate is kept unchecked; the record becomes `certified` only
//! once the checker accepts the certificate. Until then a proof rests on
//! this crate's correctness.
//!
//! The flowpipe method of FR-1019 "Method" is not implemented: every request
//! is answered `solver-inconclusive` with no trace, a result FR-250 settles
//! as `inconclusive` and which never proves or refutes. The answer names no
//! method, since none ran.

use filament_exact::Cancel;
use fsl_replay::{
    HybridAnswer, HybridProvider, HybridSolveRequest, SolverCoverage, SolverMethod, SolverOutcome,
    SolverResult,
};

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
    fn solve(&self, _request: &HybridSolveRequest<'_>, _cancel: &Cancel) -> HybridAnswer {
        HybridAnswer::solved(SolverResult {
            method: SolverMethod::Unclassified,
            outcome: SolverOutcome::Unknown(None),
            coverage: SolverCoverage::Inductive,
        })
    }
}
