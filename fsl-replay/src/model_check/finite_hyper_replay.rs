// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-898 finite-trace counterexample replay.

use fsl_eval::hyper::{
    FiniteFalseTuple, FiniteQuantifier, FiniteTrace, FiniteTracePopulation, FiniteTupleOutcome,
    replays_finite_refutation,
};

/// Evidence that a finite-hyperproperty refutation reproduced against its
/// admitted population and checked body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReproducedFiniteHyperRefutation {
    counterexample: Vec<FiniteFalseTuple>,
}

impl ReproducedFiniteHyperRefutation {
    /// The exact witness tuples accepted by replay.
    pub fn counterexample(&self) -> &[FiniteFalseTuple] {
        &self.counterexample
    }
}

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

/// Replay a finite-hyperproperty refutation and return evidence only when the
/// supplied tuple identities and positions reproduce exactly.
pub fn reproduce_finite_hyper_refutation<T>(
    population: &FiniteTracePopulation<T>,
    bound: u64,
    quantifiers: &[(FiniteQuantifier, String)],
    counterexample: &[FiniteFalseTuple],
    body: impl FnMut(&[&FiniteTrace<T>]) -> FiniteTupleOutcome,
) -> Option<ReproducedFiniteHyperRefutation> {
    replay_finite_hyper_refutation(population, bound, quantifiers, counterexample, body).then(
        || ReproducedFiniteHyperRefutation {
            counterexample: counterexample.to_vec(),
        },
    )
}
