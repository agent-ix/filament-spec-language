// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120: [`ModelSimState`] and the memo of its canonical key. The fields are
//! private to this module, so the one way to write a state's populations,
//! `ModelSimState::state_mut`, is the one that drops the memo.

use std::sync::{Arc, OnceLock};

use filament_semantic_value::catalog::CatalogCode;
use fsl_semantics::model::observation::SelectedAnchor;
use fsl_semantics::model::state::ModelState;

use super::{walk, ModelStateProvenance, StepEffect};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::StateKey;

/// A model state and its explicit provenance. Provenance is not part of its key.
#[derive(Clone)]
pub struct ModelSimState {
    state: ModelState,
    provenance: ModelStateProvenance,
    /// The canonical key of `state`, shared with the clones that hold the same
    /// `state`.
    canonical_key: Arc<OnceLock<Arc<StateKey>>>,
}

impl std::fmt::Debug for ModelSimState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelSimState")
            .field("state", &self.state)
            .field("provenance", &self.provenance)
            .finish()
    }
}

impl ModelSimState {
    fn new(state: ModelState, provenance: ModelStateProvenance) -> Self {
        Self {
            state,
            provenance,
            canonical_key: Arc::default(),
        }
    }

    pub(super) fn anchored(state: ModelState, anchor: SelectedAnchor) -> Self {
        Self::new(
            state,
            ModelStateProvenance::Anchored {
                anchor,
                effect: None,
            },
        )
    }

    /// `state` as restored from its canonical key.
    pub(super) fn restored(state: ModelState) -> Self {
        Self::new(
            state,
            ModelStateProvenance::RestoredFromKey { effect: None },
        )
    }

    pub(super) fn reached(
        &self,
        state: ModelState,
        anchor: SelectedAnchor,
        effect: StepEffect,
    ) -> Self {
        let provenance = match self.provenance {
            ModelStateProvenance::Anchored { .. } => ModelStateProvenance::Anchored {
                anchor,
                effect: Some(effect),
            },
            ModelStateProvenance::RestoredFromKey { .. } => ModelStateProvenance::RestoredFromKey {
                effect: Some(effect),
            },
        };
        Self::new(state, provenance)
    }

    pub(super) fn without_effect(&self, state: ModelState) -> Self {
        let provenance = match &self.provenance {
            ModelStateProvenance::Anchored { anchor, .. } => ModelStateProvenance::Anchored {
                anchor: anchor.clone(),
                effect: None,
            },
            ModelStateProvenance::RestoredFromKey { .. } => {
                ModelStateProvenance::RestoredFromKey { effect: None }
            }
        };
        Self::new(state, provenance)
    }

    /// The population state, to write. The key memoized for the state before
    /// the write, and shared with its clones, is not this state's.
    pub(super) fn state_mut(&mut self) -> &mut ModelState {
        self.canonical_key = Arc::default();
        &mut self.state
    }

    /// The closed population state, owned.
    pub(super) fn into_state(self) -> ModelState {
        self.state
    }

    /// The memoized canonical key, made by `encode` from the state when none
    /// is held.
    ///
    /// # Errors
    ///
    /// The refusal of `encode`; nothing is memoized then.
    pub(super) fn memoized_key<E>(
        &self,
        encode: impl FnOnce(&ModelState) -> Result<StateKey, E>,
    ) -> Result<&StateKey, E> {
        if let Some(key) = self.canonical_key.get() {
            return Ok(key.as_ref());
        }
        let key = encode(&self.state)?;
        Ok(self.canonical_key.get_or_init(|| Arc::new(key)).as_ref())
    }

    /// Whether this state and `other` hold one key memo.
    #[cfg(test)]
    pub(super) fn shares_key_memo(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.canonical_key, &other.canonical_key)
    }

    /// The closed population state.
    pub fn state(&self) -> &ModelState {
        &self.state
    }

    /// The state's admitted provenance, including explicit key restoration.
    pub fn provenance(&self) -> &ModelStateProvenance {
        &self.provenance
    }

    /// The actual initialization or handler anchor. A restored state has none.
    pub fn anchor(&self) -> Option<&SelectedAnchor> {
        match &self.provenance {
            ModelStateProvenance::Anchored { anchor, .. } => Some(anchor),
            ModelStateProvenance::RestoredFromKey { .. } => None,
        }
    }

    /// Require actual observation provenance for an API that consumes an anchor.
    ///
    /// # Errors
    /// A key-restored state has no initialization or handler provenance.
    pub fn required_anchor(&self) -> Result<&SelectedAnchor, ExpansionStop> {
        self.anchor().ok_or_else(|| {
            walk::stop(CatalogCode::new(
                "invalid_runtime_input",
                "missing-required-artifact",
            ))
        })
    }

    /// The actual step that reached the state, absent when none was executed.
    pub fn reached_by(&self) -> Option<&StepEffect> {
        match &self.provenance {
            ModelStateProvenance::Anchored { effect, .. }
            | ModelStateProvenance::RestoredFromKey { effect } => effect.as_ref(),
        }
    }
}
