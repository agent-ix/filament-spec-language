// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 EI-6 (FR-161 "Start", "Steps", "Length and limits"): turning a
//! path of a symmetry-reduced graph into a concrete path of the subject.
//!
//! [`Lift`] keeps `σ`, with the concrete state `σ(canonical state)`. It
//! starts at the first initial state in subject order whose canonical form
//! is the path's first state, with `σ = π0⁻¹` for the permutation `π0` the
//! canonicaliser returned for it. Each retained edge `(source, t, target,
//! π)` becomes the concrete step `σ(t)` to `σ ∘ π⁻¹ (target)`, and `σ`
//! becomes `σ ∘ π⁻¹`; a successor that was never canonicalised is the same
//! step with `π` the identity. Every concrete step is checked against an
//! expansion of the concrete source and counted against the run's
//! remaining `max_transitions`.
//!
//! Under the trivial group a model path is already concrete. Protocol
//! paths additionally lift delivery-label canonical edges by re-executing
//! them from the concrete source (FR-205), retaining fresh delivery indices.
//! Protocol subjects supply no [`Permuter`].

use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::{
    EncodingRefusal, ExpansionStop, Permutation, SubjectSystem, SymmetryGroup, TransitionSystem,
};
use fsl_replay::{CounterexampleKind, ModelStep};

use super::check::{fault, ModelCheckRefusal};

/// Why a path could not be concretised.
pub(super) enum LiftStop {
    /// An expansion or a permutation of a concrete state stopped.
    Expansion(ExpansionStop),
    /// The concrete steps exhausted what is left of `max_transitions`.
    Transitions,
    /// An identity has no canonical encoding, or an invariant broke.
    Refusal(ModelCheckRefusal),
}

impl From<EncodingRefusal> for LiftStop {
    fn from(refusal: EncodingRefusal) -> Self {
        Self::Refusal(ModelCheckRefusal::Encoding(refusal))
    }
}

impl From<ModelCheckRefusal> for LiftStop {
    fn from(refusal: ModelCheckRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

/// What lifting a reduced path reads of a subject's symmetry (ADR-021
/// EI-6): the canonical form of a state, and the image of a state and of a
/// transition identity under a permutation.
pub(super) trait Permuter<State, Transition> {
    fn canonicalise(
        &self,
        state: &State,
        group: &SymmetryGroup,
    ) -> Result<(State, Permutation), ExpansionStop>;

    fn permute(&self, state: &State, permutation: &Permutation) -> Result<State, ExpansionStop>;

    fn permute_transition(
        &self,
        transition: &Transition,
        permutation: &Permutation,
    ) -> Result<Transition, ExpansionStop>;

    /// The image of the FR-120 identity an undefined guard names.
    fn permute_model_transition(
        &self,
        transition: &ModelTransition,
        permutation: &Permutation,
    ) -> Result<ModelTransition, ExpansionStop>;
}

impl Permuter<ModelSimState, ModelTransition> for ModelSystem<'_> {
    fn canonicalise(
        &self,
        state: &ModelSimState,
        group: &SymmetryGroup,
    ) -> Result<(ModelSimState, Permutation), ExpansionStop> {
        ModelSystem::canonicalise(self, state, group)
    }

    fn permute(
        &self,
        state: &ModelSimState,
        permutation: &Permutation,
    ) -> Result<ModelSimState, ExpansionStop> {
        ModelSystem::permute(self, state, permutation)
    }

    fn permute_transition(
        &self,
        transition: &ModelTransition,
        permutation: &Permutation,
    ) -> Result<ModelTransition, ExpansionStop> {
        ModelSystem::permute_transition(self, transition, permutation)
    }

    fn permute_model_transition(
        &self,
        transition: &ModelTransition,
        permutation: &Permutation,
    ) -> Result<ModelTransition, ExpansionStop> {
        ModelSystem::permute_transition(self, transition, permutation)
    }
}

/// A subject's symmetry, as [`Lift`] reads it.
pub(super) type PermuterOf<'s, S> =
    &'s dyn Permuter<<S as TransitionSystem>::State, <S as TransitionSystem>::TransitionId>;

/// A path being concretised: `σ`, the concrete state it has reached and the
/// concrete steps so far.
pub(super) struct Lift<'s, S: SubjectSystem> {
    system: &'s S,
    /// The subject's symmetry; `Some` on a reduced path.
    permuter: Option<PermuterOf<'s, S>>,
    sigma: Permutation,
    concrete: S::State,
    key: fsl_eval::simulation::StateKey,
    steps: Vec<ModelStep<S::TransitionId>>,
    /// The concrete steps still allowed when a path needs lifting.
    budget: usize,
    charged_steps: usize,
}

impl<'s, S: SubjectSystem> Lift<'s, S> {
    /// FR-161 "Start": the path from `root`, a state of the graph reduced by
    /// `group`, with the index of the subject's initial state it starts
    /// from. At most `budget` concrete steps may follow. `permuter` is the
    /// subject's symmetry, which a non-trivial `group` needs.
    ///
    /// # Errors
    ///
    /// [`LiftStop`] when the canonicaliser stops, `group` is not trivial and
    /// the subject has no symmetry, or no initial state has `root`'s key.
    pub(super) fn start(
        system: &'s S,
        permuter: Option<PermuterOf<'s, S>>,
        group: &SymmetryGroup,
        root: &S::State,
        budget: usize,
    ) -> Result<(usize, Self), LiftStop> {
        let permuter = if group.is_trivial() {
            None
        } else {
            Some(permuter.ok_or_else(|| fault("a-reduced-path-has-a-symmetry"))?)
        };
        let (wanted, _) = system.state_key(root)?;
        for (index, initial) in system.initial_iter().enumerate() {
            let initial = initial.map_err(|error| ModelCheckRefusal::Protocol(error.into()))?;
            let (canonical, permutation) = match permuter {
                Some(permuter) => permuter
                    .canonicalise(&initial, group)
                    .map_err(LiftStop::Expansion)?,
                None => (initial.clone(), Permutation::identity()),
            };
            if system.state_key(&canonical)?.0 != wanted {
                continue;
            }
            let (bytes, _) = system.state_key(&initial)?;
            let key = fsl_eval::simulation::StateKey::from_canonical_bytes(bytes);
            return Ok((
                index,
                Self {
                    system,
                    permuter,
                    sigma: permutation.inverse(),
                    concrete: initial,
                    key,
                    steps: Vec::new(),
                    budget,
                    charged_steps: 0,
                },
            ));
        }
        Err(fault("a-root-is-an-initial-state").into())
    }

    /// FR-161 "Steps": the retained edge labelled `transition` to the
    /// canonical `target`, canonicalised by `permutation`; a successor that
    /// was never canonicalised passes the identity.
    ///
    /// # Errors
    ///
    /// [`LiftStop`] when an expansion stops, the budget is spent, or the
    /// concrete step is not a step of the subject.
    pub(super) fn edge(
        &mut self,
        transition: &S::TransitionId,
        target: &S::State,
        permutation: &Permutation,
    ) -> Result<(), LiftStop> {
        let Some(permuter) = self.permuter else {
            if let Some((step, state)) = self
                .system
                .concrete_cycle_step(&self.concrete, transition, target)
                .map_err(LiftStop::Expansion)?
            {
                if self.budget == 0 {
                    return Err(LiftStop::Transitions);
                }
                self.budget -= 1;
                self.charged_steps += 1;
                self.key = self.system.canonical_state_key(&state)?;
                self.steps.push(ModelStep {
                    transition: self
                        .system
                        .reported(step, &state)
                        .map_err(LiftStop::Expansion)?,
                    post_state: self.key.clone(),
                    delay: None,
                });
                self.concrete = state;
                return Ok(());
            }
            let (bytes, _) = self.system.state_key(target)?;
            self.key = fsl_eval::simulation::StateKey::from_canonical_bytes(bytes);
            self.concrete = target.clone();
            self.steps.push(ModelStep {
                transition: self
                    .system
                    .reported(transition.clone(), target)
                    .map_err(LiftStop::Expansion)?,
                post_state: self.key.clone(),
                delay: None,
            });
            return Ok(());
        };
        if self.budget == 0 {
            return Err(LiftStop::Transitions);
        }
        self.budget -= 1;
        self.charged_steps += 1;
        let step = permuter
            .permute_transition(transition, &self.sigma)
            .map_err(|_| fault("a-concretised-step-encodes"))?;
        let sigma = self.sigma.compose(&permutation.inverse());
        let next = permuter
            .permute(target, &sigma)
            .map_err(LiftStop::Expansion)?;
        let (bytes, _) = self.system.state_key(&next)?;
        let key = fsl_eval::simulation::StateKey::from_canonical_bytes(bytes);
        let expansion = self
            .system
            .successors(&self.concrete)
            .map_err(LiftStop::Expansion)?;
        let mut matched = false;
        for (candidate, state) in &expansion.successors {
            if *candidate == step && self.system.state_key(state)?.0 == key.as_bytes() {
                matched = true;
                break;
            }
        }
        if !matched {
            return Err(fault("a-concretised-step-is-a-subject-step").into());
        }
        self.steps.push(ModelStep {
            transition: step,
            post_state: key.clone(),
            delay: None,
        });
        self.sigma = sigma;
        self.concrete = next;
        self.key = key;
        Ok(())
    }

    /// `σ`: the concrete state is `σ(canonical state)`.
    pub(super) fn sigma(&self) -> &Permutation {
        &self.sigma
    }

    /// Loop closure uses the cycle key; reported post-states use `key`.
    pub(super) fn loop_key(&self) -> Result<fsl_eval::simulation::StateKey, LiftStop> {
        Ok(self.system.loop_state_key(&self.concrete)?)
    }

    /// The number of concrete steps so far.
    pub(super) fn len(&self) -> usize {
        self.steps.len()
    }

    /// The steps charged to the budget: every lifted step, none of a path
    /// that was already concrete.
    pub(super) fn charged(&self) -> usize {
        self.charged_steps
    }

    /// The concrete steps.
    pub(super) fn into_steps(self) -> Vec<ModelStep<S::TransitionId>> {
        self.steps
    }

    /// ADR-021 RV-7: an undefined evaluation's site at the path's end,
    /// concretised: the concrete state's state key and, for a guard, `σ` of
    /// the guard, which is checked against the concrete state's own
    /// expansion as each step is. Nothing changes under the trivial group.
    ///
    /// # Errors
    ///
    /// [`LiftStop`] when the expansion stops or the concrete guard is not a
    /// guard of the subject.
    pub(super) fn site(&self, kind: &mut CounterexampleKind) -> Result<(), LiftStop> {
        let CounterexampleKind::UndefinedEvaluation(evaluation) = kind else {
            return Ok(());
        };
        evaluation.site.state_key = self.key.clone();
        let Some(permuter) = self.permuter else {
            return Ok(());
        };
        if let Some(guard) = &evaluation.site.transition {
            let guard = permuter
                .permute_model_transition(guard, &self.sigma)
                .map_err(|_| fault("a-concretised-guard-encodes"))?;
            let expansion = self
                .system
                .successors(&self.concrete)
                .map_err(LiftStop::Expansion)?;
            let found = expansion.findings.iter().any(|finding| {
                finding
                    .undefined_contract()
                    .is_some_and(|contract| contract.guard_of.as_ref() == Some(&guard))
            });
            if !found {
                return Err(fault("a-concretised-guard-is-a-subject-guard").into());
            }
            evaluation.site.transition = Some(guard);
        }
        Ok(())
    }
}
