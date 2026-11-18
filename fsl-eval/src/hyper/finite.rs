// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-898 finite trace-set quantification.

use std::collections::BTreeMap;

/// A quantifier in a finite trace population prefix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteQuantifier {
    /// Every trace in the population must satisfy the remaining prefix.
    Forall,
    /// At least one trace in the population must satisfy the remaining prefix.
    Exists,
}

/// Whether a supplied trace population is closed to additional traces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteTraceClosure {
    /// The document contains the complete population.
    Closed,
    /// Additional traces may exist outside the document.
    Open,
}

/// One identified finite trace, represented as its ordered state records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteTrace<T> {
    /// Stable trace identity within its population.
    pub identity: String,
    /// The trace's finite state sequence.
    pub records: Vec<T>,
}

/// The request-supplied population quantified by an FR-898 clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteTracePopulation<T> {
    /// Stable identity of this population document.
    pub identity: String,
    /// Qualified record type declared by the finite hyper clause.
    pub record_type: String,
    /// Whether this document closes the population.
    pub closure: FiniteTraceClosure,
    /// Traces in their input order, with repeated identical identities folded.
    pub traces: Vec<FiniteTrace<T>>,
}

/// Why population admission rejected its trace identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FinitePopulationError {
    /// A trace identity is empty.
    EmptyTraceIdentity,
    /// A trace has no states.
    EmptyTrace {
        /// The trace with no states.
        identity: String,
    },
    /// One identity occurs more than once in the population document.
    DuplicateTraceIdentity {
        /// The repeated trace identity.
        identity: String,
    },
}

impl<T: Eq> FiniteTracePopulation<T> {
    /// Construct a population, rejecting empty identities, empty traces, and
    /// every repeated identity in the source document.
    pub fn new(
        identity: impl Into<String>,
        record_type: impl Into<String>,
        closure: FiniteTraceClosure,
        traces: Vec<FiniteTrace<T>>,
    ) -> Result<Self, FinitePopulationError> {
        let mut indices = BTreeMap::new();
        let mut unique: Vec<FiniteTrace<T>> = Vec::with_capacity(traces.len());
        for trace in traces {
            if trace.identity.is_empty() {
                return Err(FinitePopulationError::EmptyTraceIdentity);
            }
            if trace.records.is_empty() {
                return Err(FinitePopulationError::EmptyTrace {
                    identity: trace.identity,
                });
            }
            if indices.contains_key(&trace.identity) {
                return Err(FinitePopulationError::DuplicateTraceIdentity {
                    identity: trace.identity,
                });
            }
            indices.insert(trace.identity.clone(), unique.len());
            unique.push(trace);
        }
        Ok(Self {
            identity: identity.into(),
            record_type: record_type.into(),
            closure,
            traces: unique,
        })
    }
}

/// One all-false trace tuple in a finite-hyperproperty refutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteFalseTuple {
    /// The chosen trace identity for each variable, in quantifier order.
    pub traces: Vec<String>,
    /// The joint position where the body is false.
    pub position: u64,
}

/// The finite population check's result (FR-898).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FiniteHyperOutcome {
    /// The prefix holds over a closed population and every tuple is decided.
    Proved,
    /// A decisive false prefix, with its falsifying tuple or completions.
    Refuted {
        /// The falsifying tuples and their first false positions.
        false_tuples: Vec<FiniteFalseTuple>,
    },
    /// The population is open or a required tuple is incomplete.
    Inconclusive,
    /// The population exceeds the declaration's trace bound.
    RequiresBound {
        /// The declared maximum trace count.
        bound: u64,
        /// The admitted population's trace count.
        found: usize,
    },
}

/// The truth of the clause body on one selected trace tuple.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FiniteTupleOutcome {
    /// The temporal body is decided true on this tuple.
    True,
    /// The temporal body is decided false at this joint position.
    False {
        /// The first joint position where the body is false.
        position: u64,
    },
    /// An aligned read or the temporal body is incomplete on this tuple.
    Incomplete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Truth {
    True,
    False(Vec<FiniteFalseTuple>),
    Incomplete,
}

/// Evaluate an FR-898 prefix in source order over every trace identity.
/// `body` evaluates the checked temporal formula over one full trace tuple,
/// using lockstep alignment; absent aligned records must produce
/// [`FiniteTupleOutcome::Incomplete`].
pub fn evaluate_finite_hyper<T>(
    population: &FiniteTracePopulation<T>,
    bound: u64,
    quantifiers: &[(FiniteQuantifier, String)],
    mut body: impl FnMut(&[&FiniteTrace<T>]) -> FiniteTupleOutcome,
) -> FiniteHyperOutcome {
    if population.traces.len() as u128 > u128::from(bound) {
        return FiniteHyperOutcome::RequiresBound {
            bound,
            found: population.traces.len(),
        };
    }
    let mut selected = Vec::with_capacity(quantifiers.len());
    let truth = quantify(population, quantifiers, 0, &mut selected, &mut body);
    match truth {
        Truth::False(false_tuples) => FiniteHyperOutcome::Refuted { false_tuples },
        Truth::True if population.closure == FiniteTraceClosure::Closed => {
            FiniteHyperOutcome::Proved
        }
        Truth::True | Truth::Incomplete => FiniteHyperOutcome::Inconclusive,
    }
}

/// Re-evaluate a finite-hyperproperty counterexample against the supplied
/// population and accept it only when its named traces and positions match.
pub fn replays_finite_refutation<T>(
    population: &FiniteTracePopulation<T>,
    bound: u64,
    quantifiers: &[(FiniteQuantifier, String)],
    counterexample: &[FiniteFalseTuple],
    body: impl FnMut(&[&FiniteTrace<T>]) -> FiniteTupleOutcome,
) -> bool {
    matches!(
        evaluate_finite_hyper(population, bound, quantifiers, body),
        FiniteHyperOutcome::Refuted { false_tuples }
            if false_tuples.as_slice() == counterexample
    )
}

fn quantify<T>(
    population: &FiniteTracePopulation<T>,
    quantifiers: &[(FiniteQuantifier, String)],
    level: usize,
    selected: &mut Vec<usize>,
    body: &mut impl FnMut(&[&FiniteTrace<T>]) -> FiniteTupleOutcome,
) -> Truth {
    let Some((kind, _)) = quantifiers.get(level) else {
        return evaluate_tuple(population, selected, body);
    };
    let mut any_true = false;
    let mut any_incomplete = false;
    let mut false_tuples = Vec::new();
    for index in 0..population.traces.len() {
        selected.push(index);
        let truth = quantify(population, quantifiers, level + 1, selected, body);
        selected.pop();
        match (kind, truth) {
            (FiniteQuantifier::Forall, Truth::False(false_tuples)) => {
                return Truth::False(false_tuples);
            }
            (FiniteQuantifier::Exists, Truth::True) => return Truth::True,
            (_, Truth::True) => any_true = true,
            (_, Truth::Incomplete) => any_incomplete = true,
            (FiniteQuantifier::Exists, Truth::False(mut tuples)) => {
                false_tuples.append(&mut tuples);
            }
        }
    }
    match kind {
        FiniteQuantifier::Forall if any_incomplete => Truth::Incomplete,
        FiniteQuantifier::Forall => Truth::True,
        FiniteQuantifier::Exists if any_true => Truth::True,
        FiniteQuantifier::Exists if any_incomplete => Truth::Incomplete,
        FiniteQuantifier::Exists => Truth::False(false_tuples),
    }
}

fn evaluate_tuple<T>(
    population: &FiniteTracePopulation<T>,
    selected: &[usize],
    body: &mut impl FnMut(&[&FiniteTrace<T>]) -> FiniteTupleOutcome,
) -> Truth {
    let traces: Vec<_> = selected
        .iter()
        .filter_map(|index| population.traces.get(*index))
        .collect();
    if traces.len() != selected.len() {
        return Truth::Incomplete;
    }
    match body(&traces) {
        FiniteTupleOutcome::True => Truth::True,
        FiniteTupleOutcome::False { position } => Truth::False(vec![FiniteFalseTuple {
            traces: traces.iter().map(|trace| trace.identity.clone()).collect(),
            position,
        }]),
        FiniteTupleOutcome::Incomplete => Truth::Incomplete,
    }
}
