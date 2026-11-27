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
//! step with `π` the identity. Every lifted step is an admitted canonical
//! graph edge mapped through the symmetry action and counted against the
//! run's remaining `max_transitions`.
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
use ix_trace_rs::implements;

use super::check::{fault, ModelCheckRefusal};

#[cfg(test)]
mod tests {
    use super::*;
    use filament_exact::{LimitKind, ScalarLimits};
    use filament_semantic_value::checking::CheckMode;
    use fsl_eval::simulation::model::ModelSystem;
    use fsl_eval::simulation::{ExpansionLimit, SymmetryClass};
    use ix_trace_rs::trace;

    fn meter(work_units: u64) -> ScalarLimits {
        ScalarLimits {
            work_units,
            ..crate::counter::UNLIMITED
        }
    }

    #[trace("FR-161-AC-5")]
    #[test]
    fn lift_charges_two_work_units_per_edge() {
        let document = crate::counter::symmetric_document();
        let package = crate::counter::package_with_mode(
            &document,
            crate::counter::CONTRACT,
            CheckMode::Linked,
        );
        let mut documents = crate::counter::Documents::new(document);
        let initial = vec![documents.start("lift-work", &[("a", 0), ("b", 0), ("c", 0)])];
        let subject = crate::counter::subject(
            &package,
            &documents,
            initial,
            &["a", "b", "c"],
            crate::counter::UNLIMITED,
        );
        let system = ModelSystem::new(subject.simulation).expect("the counter subject admits");
        let group = SymmetryGroup {
            classes: vec![SymmetryClass {
                population: crate::counter::key("counters"),
                keys: ["a", "b", "c"].map(str::to_owned).to_vec(),
            }],
        };
        let root = system
            .initial_iter()
            .next()
            .expect("one initial state")
            .expect("the initial state builds");
        let (mut canonical, _) = system
            .canonicalise(&root, &group)
            .expect("the initial state canonicalises");
        let (_, mut lift) = match Lift::start(&system, Some(&system), &group, &canonical, 3) {
            Ok(lift) => lift,
            Err(LiftStop::Expansion(_)) => panic!("the canonical root lifts"),
            Err(LiftStop::Transitions) => panic!("the lift has three transition units"),
            Err(LiftStop::Refusal(_)) => panic!("the canonical root is an initial state"),
        };

        let mut path = Vec::new();
        for _ in 0..3 {
            let (transition, target) = system
                .successors(&canonical)
                .expect("the counter expands")
                .successors
                .into_iter()
                .next()
                .expect("the counter has an enabled increment");
            let (next, permutation) = system
                .canonicalise(&target, &group)
                .expect("the successor canonicalises");
            path.push((transition, next.clone(), permutation));
            canonical = next;
        }

        system.share_evaluation_meter(meter(5));
        let stop = path.iter().find_map(|(transition, target, permutation)| {
            match lift.edge(transition, target, permutation) {
                Ok(()) => None,
                Err(LiftStop::Expansion(stop)) => Some(stop),
                Err(LiftStop::Transitions) => panic!("the lift has three transition units"),
                Err(LiftStop::Refusal(refusal)) => panic!("the lift is well formed: {refusal:?}"),
            }
        });
        assert_eq!(
            stop.and_then(|stop| stop.limit),
            Some(ExpansionLimit::EvaluationMeter(LimitKind::WorkUnits))
        );
        assert_eq!(lift.charged(), 3);

        let (_, mut exact_lift) = match Lift::start(&system, Some(&system), &group, &root, 3) {
            Ok(lift) => lift,
            Err(LiftStop::Expansion(_)) => panic!("the canonical root lifts again"),
            Err(LiftStop::Transitions) => panic!("the lift has three transition units"),
            Err(LiftStop::Refusal(_)) => panic!("the root is an initial state"),
        };
        system.share_evaluation_meter(meter(6));
        for (transition, target, permutation) in &path {
            match exact_lift.edge(transition, target, permutation) {
                Ok(()) => {}
                Err(LiftStop::Expansion(_)) => {
                    panic!("three lifted edges use exactly six work units")
                }
                Err(LiftStop::Transitions) => panic!("the lift has three transition units"),
                Err(LiftStop::Refusal(_)) => panic!("the canonical path lifts"),
            }
        }
        assert_eq!(exact_lift.charged(), 3);
    }
}

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

    /// Charge one state or transition permutation application to the run.
    fn charge_permutation_work(&self) -> Result<(), ExpansionStop>;
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

    fn charge_permutation_work(&self) -> Result<(), ExpansionStop> {
        self.charge_concretisation_work()
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
    #[implements("FR-161-AC-5")]
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
        permuter
            .charge_permutation_work()
            .map_err(LiftStop::Expansion)?;
        let step = permuter
            .permute_transition(transition, &self.sigma)
            .map_err(|_| fault("a-concretised-step-encodes"))?;
        let sigma = self.sigma.compose(&permutation.inverse());
        permuter
            .charge_permutation_work()
            .map_err(LiftStop::Expansion)?;
        let next = permuter
            .permute(target, &sigma)
            .map_err(LiftStop::Expansion)?;
        let (bytes, _) = self.system.state_key(&next)?;
        let key = fsl_eval::simulation::StateKey::from_canonical_bytes(bytes);
        // The canonical graph admitted this edge at its source state. The
        // subject symmetry maps its transition identity and target together,
        // so the lifted edge remains a subject step. Re-expanding the
        // concrete source here would charge candidate exploration work to
        // FR-161's permutation allowance.
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
