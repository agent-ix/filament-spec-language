// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201 bounded-event checker primitives over the actual property monitor.
//! FR-338 owns the canonical representation; local numbering never enters it.

use fsl_semantics::check::{CheckedFormula, FormulaNodeId, TemporalProfile};

use super::{
    translate_body, AutomatonError, AutomatonStateKey, BoundedKind, BoundedState,
    BoundedTranslation, KeyRefusal, Letter, Monitor, NotTranslated, PropertyAutomaton,
    StateContext, StateId, TranslationRefusal,
};

/// Failure to construct or read a qualified bounded-event monitor.
#[derive(Debug, thiserror::Error)]
pub enum QualifiedBoundedError {
    /// The formula or its context is outside the admitted shape.
    #[error(transparent)]
    Translation(#[from] TranslationRefusal),
    /// Translation, state lookup, or a monitor resource limit failed.
    #[error(transparent)]
    Automaton(#[from] AutomatonError),
    /// The complete canonical key could not be encoded.
    #[error(transparent)]
    Key(#[from] KeyRefusal),
}

/// An origin-activated event-position `eventually[0,h] holds(P)` or
/// `always[0,h] holds(P)` monitor.
/// Its keys and ranks are projected from the reached automaton's real debt,
/// with no separately advanced horizon counter. The caller owns the checked
/// occurrence and binding in `StateContext` and supplies P's truth letters.
#[derive(Debug)]
pub struct QualifiedBoundedMonitor {
    translation: BoundedTranslation,
    monitor: Monitor<PropertyAutomaton>,
}

impl QualifiedBoundedMonitor {
    /// Whether the defined-value probability certificate path admits this
    /// formula. Binding admission remains contextual.
    pub fn supports(formula: &CheckedFormula, profile: TemporalProfile) -> bool {
        profile == TemporalProfile::EventPositionFalseExtension
            && matches!(
                super::bounded::bounded_formula_shape(formula),
                Some((BoundedKind::Eventually | BoundedKind::Always, _, _))
            )
    }

    /// Whether the qualified control can replay observations of this formula,
    /// including an undefined read of a bounded always obligation.
    pub fn supports_observations(formula: &CheckedFormula, profile: TemporalProfile) -> bool {
        profile == TemporalProfile::EventPositionFalseExtension
            && matches!(
                super::bounded::bounded_formula_shape(formula),
                Some((BoundedKind::Eventually | BoundedKind::Always, _, _))
            )
    }

    /// Retain a defined-value bounded producer monitor without copying its
    /// cache. Its complete translation identity must match the checked
    /// formula before any local handle can be projected into a key.
    pub fn from_monitor(
        formula: &CheckedFormula,
        context: &StateContext,
        monitor: Monitor<PropertyAutomaton>,
        max_states: u64,
    ) -> Result<Self, QualifiedBoundedError> {
        Self::retain(Self::new(formula, context, max_states)?, monitor)
    }

    /// Retain a producer monitor for undefined observation replay, including
    /// bounded always. Defined certificate callers use [`Self::from_monitor`].
    pub fn from_observation_monitor(
        formula: &CheckedFormula,
        context: &StateContext,
        monitor: Monitor<PropertyAutomaton>,
        max_states: u64,
    ) -> Result<Self, QualifiedBoundedError> {
        Self::retain(
            Self::for_observations(formula, context, max_states)?,
            monitor,
        )
    }

    fn retain(
        mut expected: Self,
        mut monitor: Monitor<PropertyAutomaton>,
    ) -> Result<Self, QualifiedBoundedError> {
        let initial = monitor.viable_initial()?.ok_or(AutomatonError::Invariant {
            invariant: "qualified-bounded-viable-root",
        })?;
        let root = expected
            .monitor
            .viable_initial()?
            .ok_or(AutomatonError::Invariant {
                invariant: "qualified-bounded-viable-root",
            })?;
        let bytes = fsl_foundation::IdentityLimits::default().input_bytes;
        if monitor.automaton().state_key(initial, bytes)?
            != expected.monitor.automaton().state_key(root, bytes)?
        {
            return Err(AutomatonError::Invariant {
                invariant: "qualified-bounded-matching-translation",
            }
            .into());
        }
        expected.monitor = monitor;
        Ok(expected)
    }

    /// Build an origin event-position bounded monitor for defined values.
    /// Other profiles, operators and activations are explicitly refused.
    pub fn new(
        formula: &CheckedFormula,
        context: &StateContext,
        max_states: u64,
    ) -> Result<Self, QualifiedBoundedError> {
        if !Self::supports(formula, context.profile) {
            return Err(TranslationRefusal::NotTranslated(NotTranslated).into());
        }
        Self::for_observations(formula, context, max_states)
    }

    /// Build control for replaying possibly undefined bounded observations.
    pub fn for_observations(
        formula: &CheckedFormula,
        context: &StateContext,
        max_states: u64,
    ) -> Result<Self, QualifiedBoundedError> {
        let translation = BoundedTranslation::formula(formula, context)?;
        if context.profile != TemporalProfile::EventPositionFalseExtension {
            return Err(TranslationRefusal::NotTranslated(NotTranslated).into());
        }
        let automaton = translate_body(formula, context.profile, max_states)?;
        Ok(Self {
            translation,
            monitor: Monitor::bad_prefix(automaton, max_states)?,
        })
    }

    /// The checked `holds` atom whose truth supplies each observation.
    pub fn atom(&self) -> FormulaNodeId {
        self.translation.atom()
    }

    /// Canonical control before the first observation, for an undefined read.
    pub fn unread_key(&self) -> Result<AutomatonStateKey, QualifiedBoundedError> {
        Ok(self.translation.key(BoundedState::Open {
            remaining: self.translation.horizon(),
        })?)
    }

    /// Read position zero. The returned handle is local storage, never a
    /// certificate identity; use `key` for the full canonical identity.
    pub fn start(&mut self, holds: bool) -> Result<StateId, QualifiedBoundedError> {
        let initial = self.monitor.initial()?;
        self.step(initial, holds)
    }

    /// Advance by one counted event position and read its predicate.
    pub fn step(&mut self, from: StateId, holds: bool) -> Result<StateId, QualifiedBoundedError> {
        // This admitted formula has exactly one holds atom.
        Ok(self
            .monitor
            .step(from, Letter::from_bits(u64::from(holds)))?)
    }

    /// Resolve a reached monitor after an observation into FR-338's parsed
    /// state. Decided states carry no fabricated remaining horizon.
    pub fn state(&self, state: StateId) -> Result<BoundedState, QualifiedBoundedError> {
        let members = self
            .monitor
            .members(state)
            .ok_or(AutomatonError::UnknownState { state })?;
        if members.is_empty() {
            return Ok(BoundedState::Reject);
        }
        let mut remaining = None;
        for member in members {
            if self.monitor.automaton().is_settled(*member)? {
                return Ok(BoundedState::Accept);
            }
            let debt = self.monitor.automaton().bounded_deadline(*member)?;
            if remaining.is_some_and(|previous| previous != debt) {
                return Err(AutomatonError::Invariant {
                    invariant: "qualified-bounded-one-debt",
                }
                .into());
            }
            remaining = Some(debt);
        }
        remaining
            .map(|remaining| BoundedState::Open { remaining })
            .ok_or_else(|| {
                AutomatonError::Invariant {
                    invariant: "qualified-bounded-has-debt",
                }
                .into()
            })
    }

    /// The full canonical monitor key, including the checked item context,
    /// binding, real remaining horizon and pending window debt.
    pub fn key(&self, state: StateId) -> Result<AutomatonStateKey, QualifiedBoundedError> {
        Ok(self.translation.key(self.state(state)?)?)
    }

    /// The context-bearing key for a settled verdict. Product builders may
    /// discard the local monitor handle after settlement; no remaining
    /// horizon is introduced when encoding that verdict.
    pub fn decision_key(&self, accepted: bool) -> Result<AutomatonStateKey, QualifiedBoundedError> {
        Ok(self.translation.key(if accepted {
            BoundedState::Accept
        } else {
            BoundedState::Reject
        })?)
    }

    /// The remaining event horizon for a running state, or zero once
    /// decided. It is independent of the raw model's graph height.
    pub fn rank(&self, state: StateId) -> Result<u64, QualifiedBoundedError> {
        Ok(match self.state(state)? {
            BoundedState::Open { remaining } => remaining,
            BoundedState::Accept | BoundedState::Reject | BoundedState::Idle => 0,
        })
    }

    /// Close a terminal model position by the profile's false extension,
    /// without enumerating artificial stutter positions.
    pub fn terminal(&self, state: StateId) -> Result<BoundedState, QualifiedBoundedError> {
        Ok(self.translation.terminal(self.state(state)?))
    }
}
