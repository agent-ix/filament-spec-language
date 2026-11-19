// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124 "Deadlocked states" (ADR-018 DL-1, DL-2): how a model state reads
//! against the package's `terminal` declarations.
//!
//! A state is terminal when no transition identity is enabled at it, which
//! the caller reads from the state's own expansion. A terminal state is
//! intended when a declaration marks it so: `terminal any`, or `terminal
//! when P` with `P` true at the state. `P` is read only at a terminal state,
//! through the one clause evaluator (FR-107), and like an invariant it holds
//! at a state when it holds for every object of the state whose type
//! conforms to `P`'s `self`. The state graph alone decides; no fairness
//! constraint is read.

use super::input::{ModelTerminalDeclaration, TerminalWhen};
use filament_exact::Meter;

use super::contracts::{verdict, Verdict};
use super::walk::{exhausted, stop, walk_of};
use super::{ClauseOutcome, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::{ExpansionLimit, ExpansionStop};
use crate::temporal::UndefinedCause;

/// The model a `terminal` declaration is on, named by its selection
/// identity and its ordinal among that identity's declarations, the key both
/// routes compare by (FR-940). The label is the name
/// a user reads: the source alias on the source route, the identity on the
/// package route, which records no alias.
#[derive(Clone, Eq)]
pub struct TerminalModel {
    identity: String,
    ordinal: usize,
    label: String,
}

impl TerminalModel {
    /// The model with selection identity `identity`, shown as `label`.
    pub fn new(identity: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            identity: identity.into(),
            ordinal: 0,
            label: label.into(),
        }
    }

    /// A subject with no selection identity, named by `name` alone.
    pub fn named(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            identity: name.clone(),
            ordinal: 0,
            label: name,
        }
    }

    /// This declaration as the `ordinal`th of the identity's declarations
    /// (FR-411): the order of the aliases that select the identity on the
    /// source route, of the terminal rows on the package route.
    pub fn with_ordinal(mut self, ordinal: usize) -> Self {
        self.ordinal = ordinal;
        self
    }

    /// The declaration's ordinal among its identity's declarations.
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// The selection identity, the comparison key.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The display label.
    pub fn label(&self) -> &str {
        &self.label
    }
}

impl std::fmt::Debug for TerminalModel {
    // The label is display only, so a record of either route prints the
    // identity the routes compare by.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TerminalModel")
            .field(&self.identity)
            .field(&self.ordinal)
            .finish()
    }
}

impl PartialEq for TerminalModel {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity && self.ordinal == other.ordinal
    }
}

impl std::hash::Hash for TerminalModel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.identity.hash(state);
        self.ordinal.hash(state);
    }
}

/// FR-124's classification of one state of a subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalClass {
    /// A transition identity is enabled at the state.
    NotTerminal,
    /// The state is terminal and a declaration intends it.
    Intended,
    /// The state is terminal and no declaration intends it: a deadlock.
    Deadlocked,
    /// A `terminal when` predicate is undefined at the terminal state, so
    /// the state is neither intended nor deadlocked (ADR-018 UE-6).
    Undefined {
        /// The model whose declaration it is.
        model: TerminalModel,
        /// The first undefined evaluation, in object order.
        cause: UndefinedCause,
    },
    /// A `terminal when` predicate is refused at the terminal state, with
    /// none undefined and none true: whether the state is intended is not
    /// known (ADR-018 V-6 `UndecidedSuccessor`).
    Undecided {
        /// The model whose declaration it is.
        model: TerminalModel,
    },
}

/// How one `terminal when` predicate read at one state.
enum Reading {
    True,
    False,
    Undefined(UndefinedCause),
    Refused,
}

impl ModelSystem<'_> {
    /// FR-124: the class of `state`, whose expansion gave a successor
    /// exactly when `has_successor`. The package's declarations are read in
    /// model-alias order: `Any` intends every terminal state; `When(P)`
    /// intends it when `P` is true. An undefined `P` reads before a later
    /// declaration is consulted, so the class is a function of the state.
    ///
    /// # Errors
    ///
    /// `resource_exhausted`/`insufficient-next-charge` naming the evaluation
    /// meter when `P`'s evaluation exhausts it;
    /// `runtime_invariant`/`established-invariant-broken` when the state
    /// cannot be observed or the evaluator faults.
    pub fn terminal_class(
        &self,
        state: &ModelSimState,
        has_successor: bool,
    ) -> Result<TerminalClass, ExpansionStop> {
        if has_successor {
            return Ok(TerminalClass::NotTerminal);
        }
        for terminal in self.terminals() {
            match terminal.declaration {
                ModelTerminalDeclaration::Any => return Ok(TerminalClass::Intended),
                ModelTerminalDeclaration::When(predicate) => match self.read(state, predicate)? {
                    Reading::True => return Ok(TerminalClass::Intended),
                    Reading::False => {}
                    Reading::Undefined(cause) => {
                        return Ok(TerminalClass::Undefined {
                            model: terminal.model.clone(),
                            cause,
                        })
                    }
                    Reading::Refused => {
                        return Ok(TerminalClass::Undecided {
                            model: terminal.model.clone(),
                        })
                    }
                },
            }
        }
        Ok(TerminalClass::Deadlocked)
    }

    /// `predicate` at `state`: each conforming object in ascending
    /// reference order, each with a fresh meter. A false object outweighs
    /// undefined objects. A predicate without `self` is read once.
    fn read(
        &self,
        state: &ModelSimState,
        predicate: TerminalWhen<'_>,
    ) -> Result<Reading, ExpansionStop> {
        let walk = walk_of(self, state, None)?;
        let mut objects: Vec<_> = if let Some(context) = predicate.context() {
            walk.rows
                .iter()
                .filter(|row| {
                    self.types
                        .conforms(row.reference.object_type().clone(), context.clone())
                })
                .map(|row| Some(row.reference.clone()))
                .collect()
        } else {
            vec![None]
        };
        objects.sort();
        let mut holds = true;
        let mut undefined = None;
        for object in objects {
            let evaluation = self
                .evaluator()
                .evaluate_terminal_predicate(
                    predicate,
                    &walk.observation.environment,
                    object,
                    &mut Meter::new(self.meter),
                )
                .map_err(|_| stop(BROKEN))?;
            match verdict(&evaluation.outcome) {
                Verdict::True => {}
                Verdict::False => holds = false,
                Verdict::Undecided(outcome) => {
                    if let Some(cause) = outcome.undefined_cause() {
                        undefined.get_or_insert(cause);
                        continue;
                    }
                    if let ClauseOutcome::Incomplete(incomplete) = outcome {
                        return Err(exhausted(ExpansionLimit::EvaluationMeter(
                            incomplete.limit_kind,
                        )));
                    }
                    return Ok(Reading::Refused);
                }
            }
        }
        Ok(if !holds {
            Reading::False
        } else if let Some(cause) = undefined {
            Reading::Undefined(cause)
        } else {
            Reading::True
        })
    }
}
