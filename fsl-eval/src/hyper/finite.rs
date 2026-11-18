// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-898 finite trace-set quantification.

use fsl_forms::QuantifierKind;
use std::collections::BTreeMap;

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
    /// Whether this document closes the population.
    pub closure: FiniteTraceClosure,
    /// Traces in their input order, with repeated identical identities folded.
    pub traces: Vec<FiniteTrace<T>>,
}

/// Why population admission rejected its trace identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FinitePopulationError {
    /// One identity was supplied with two different record sequences.
    ConflictingTraceIdentity { identity: String },
}

impl<T: Eq> FiniteTracePopulation<T> {
    /// Construct a population, rejecting identity collisions with differing
    /// records and collapsing repeated identical rows to one identity.
    pub fn new(
        identity: impl Into<String>,
        closure: FiniteTraceClosure,
        traces: Vec<FiniteTrace<T>>,
    ) -> Result<Self, FinitePopulationError> {
        let mut indices = BTreeMap::new();
        let mut unique: Vec<FiniteTrace<T>> = Vec::with_capacity(traces.len());
        for trace in traces {
            if let Some(index) = indices.get(&trace.identity).copied() {
                if unique[index].records != trace.records {
                    return Err(FinitePopulationError::ConflictingTraceIdentity {
                        identity: trace.identity,
                    });
                }
                continue;
            }
            indices.insert(trace.identity.clone(), unique.len());
            unique.push(trace);
        }
        Ok(Self {
            identity: identity.into(),
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
    Refuted { false_tuples: Vec<FiniteFalseTuple> },
    /// The population is open or a required tuple is incomplete.
    Inconclusive,
    /// The population exceeds the declaration's trace bound.
    RequiresBound { bound: u64, found: usize },
}

/// The truth of the clause body on one selected trace tuple.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FiniteTupleOutcome {
    /// The temporal body is decided true on this tuple.
    True,
    /// The temporal body is decided false at this joint position.
    False { position: u64 },
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
    quantifiers: &[(QuantifierKind, String)],
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
    quantifiers: &[(QuantifierKind, String)],
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
    quantifiers: &[(QuantifierKind, String)],
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
            (QuantifierKind::Forall, Truth::False(false_tuples)) => {
                return Truth::False(false_tuples);
            }
            (QuantifierKind::Exists, Truth::True) => return Truth::True,
            (_, Truth::True) => any_true = true,
            (_, Truth::Incomplete) => any_incomplete = true,
            (QuantifierKind::Exists, Truth::False(mut tuples)) => {
                false_tuples.append(&mut tuples);
            }
        }
    }
    match kind {
        QuantifierKind::Forall if any_incomplete => Truth::Incomplete,
        QuantifierKind::Forall => Truth::True,
        QuantifierKind::Exists if any_true => Truth::True,
        QuantifierKind::Exists if any_incomplete => Truth::Incomplete,
        QuantifierKind::Exists => Truth::False(false_tuples),
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
