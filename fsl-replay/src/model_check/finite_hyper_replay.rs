// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-898 finite-trace counterexample replay.

use fsl_eval::hyper::{
    replays_finite_refutation, FiniteFalseTuple, FiniteQuantifier, FiniteTrace,
    FiniteTracePopulation, FiniteTupleOutcome,
};

/// Re-evaluate a finite-hyperproperty refutation against the supplied trace
/// population and accept only the same trace identities and false positions.
pub fn replay_finite_hyper_refutation<T>(
    population: &FiniteTracePopulation<T>,
    bound: u64,
    quantifiers: &[(FiniteQuantifier, String)],
    counterexample: &[FiniteFalseTuple],
    body: impl FnMut(&[&FiniteTrace<T>]) -> FiniteTupleOutcome,
) -> bool {
    replays_finite_refutation(population, bound, quantifiers, counterexample, body)
}
