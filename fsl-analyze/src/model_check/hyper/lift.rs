// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174 "Counterexamples and verdicts" (ADR-023 CS-4): a path of the
//! quotient by the copy group and the selected key symmetry as the concrete
//! joint behaviour it stands for.
//!
//! A stored state is one member of its orbit, and each retained edge keeps the
//! permutation its target was canonicalised by. The concrete tuple a path
//! stands for starts at the stored root, which is itself a tuple of initial
//! states, and follows the edges: where a stored state's component `m` stands
//! for concrete component `k`, the edge's permutation says which stored
//! component of the target the same behaviour continues in. The path is
//! repeated until the concrete tuple closes, at most the order of the
//! permutation it composes to.
//!
//! The permutation also renames keys, the same in every component (ADR-021
//! SYM-6). A stored state is the concrete one with its keys renamed by the
//! composition of the permutations of the edges so far, so the concrete state
//! is the stored one renamed by that composition's inverse, and so is each
//! step taken from it.

use fsl_eval::hyper::{copy_population, ComponentStep, HyperState, HyperTransition};
use fsl_eval::simulation::{Permutation, TransitionSystem};

use crate::model_check::check::fault;
use crate::model_check::graph::{ProductStateId, RetainedEdge};
use crate::model_check::ModelCheckRefusal;

use super::{Explored, Run};

/// A path of retained edges as the concrete joint behaviour it stands for.
pub(super) struct Lifted {
    /// The concrete state at each joint position of the path, the root first.
    pub(super) states: Vec<HyperState>,
    /// The joint step into each position after the first, with the state key of
    /// each component's post-state.
    pub(super) steps: Vec<(HyperTransition, Vec<fsl_eval::simulation::StateKey>)>,
    /// Component `k` of the last concrete state is component `positions[k]` of
    /// the stored state it stands for.
    pub(super) positions: Vec<usize>,
    /// The renaming that takes a key of the last stored state to the key it
    /// stands for in the concrete state.
    pub(super) renaming: Permutation,
}

/// `permutation` without its moves of the trace variables: the renaming of
/// keys alone.
fn key_part(permutation: &Permutation) -> Permutation {
    let copies = copy_population();
    permutation
        .maps()
        .iter()
        .filter(|(population, _)| **population != copies)
        .fold(Permutation::identity(), |part, (population, map)| {
            part.compose(&Permutation::of(
                population,
                map.iter().map(|(from, to)| (from.clone(), to.clone())),
            ))
        })
}

/// `items` read through `positions`: element `k` is `items[positions[k]]`.
/// Elements past `positions` stay in place, as an HP-3 step's existential
/// components do.
pub(super) fn through<T: Clone>(items: &[T], positions: &[usize]) -> Vec<T> {
    positions
        .iter()
        .filter_map(|from| items.get(*from))
        .chain(items.iter().skip(positions.len()))
        .cloned()
        .collect()
}

impl Run<'_, '_> {
    /// The stored component each stored component of an edge's source goes
    /// to in its target: `slots[m]` is the position of the variable that
    /// `permutation` moves component `m`'s variable to.
    fn slots(
        &self,
        permutation: &Permutation,
        components: usize,
    ) -> Result<Vec<usize>, ModelCheckRefusal> {
        let population = copy_population();
        let quantifiers = self.request.clause.quantifiers();
        let names: Vec<&str> = quantifiers
            .iter()
            .take(components)
            .map(|quantifier| quantifier.variable.as_str())
            .collect();
        names
            .iter()
            .map(|name| {
                let moved = permutation.image(&population, name);
                names
                    .iter()
                    .position(|candidate| *candidate == moved)
                    .ok_or_else(|| fault("a-copy-permutation-moves-a-variable-of-the-clause"))
            })
            .collect()
    }

    /// `state` with the keys of every component renamed by `renaming`.
    fn renamed(
        &self,
        state: HyperState,
        renaming: &Permutation,
    ) -> Result<HyperState, ModelCheckRefusal> {
        if renaming.is_identity() {
            return Ok(state);
        }
        let components = state
            .components
            .iter()
            .enumerate()
            .map(|(component, held)| {
                self.system_of(component)?
                    .permute(held, renaming)
                    .map_err(|_| fault("a-key-renaming-permutes-a-stored-state"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(HyperState::new(components, state.property))
    }

    /// The system of the variable at `component`.
    pub(super) fn system_of(
        &self,
        component: usize,
    ) -> Result<&fsl_eval::simulation::model::ModelSystem<'_>, ModelCheckRefusal> {
        self.slots
            .get(component)
            .and_then(|slot| self.systems.get(*slot))
            .ok_or_else(|| fault("a-component-has-a-system"))
    }

    /// `step` with the keys of every component's transition renamed by
    /// `renaming`.
    pub(super) fn renamed_step(
        &self,
        step: &HyperTransition,
        renaming: &Permutation,
    ) -> Result<HyperTransition, ModelCheckRefusal> {
        if renaming.is_identity() {
            return Ok(step.clone());
        }
        step.0
            .iter()
            .enumerate()
            .map(|(component, taken)| match taken {
                ComponentStep::Model(transition) => self
                    .system_of(component)?
                    .permute_transition(transition, renaming)
                    .map(ComponentStep::Model)
                    .map_err(|_| fault("a-key-renaming-permutes-a-transition")),
                other => Ok(other.clone()),
            })
            .collect::<Result<_, _>>()
            .map(HyperTransition)
    }

    /// The state key of each component's post-state of `step` taken from the
    /// concrete `state`.
    pub(super) fn concrete_posts(
        &self,
        state: &HyperState,
        step: &HyperTransition,
    ) -> Result<Vec<fsl_eval::simulation::StateKey>, ModelCheckRefusal> {
        let posts = state
            .components
            .iter()
            .zip(&step.0)
            .enumerate()
            .map(|(component, (held, taken))| {
                let system = self.system_of(component)?;
                let ComponentStep::Model(transition) = taken else {
                    return Ok(held.clone());
                };
                system
                    .successors(held)
                    .map_err(|_| fault("a-concrete-state-expands"))?
                    .successors
                    .into_iter()
                    .find(|(candidate, _)| candidate == transition)
                    .map(|(_, post)| post)
                    .ok_or_else(|| fault("a-concrete-step-is-enabled"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.state_keys(&HyperState::new(posts, fsl_eval::hyper::Property::Dead))
    }

    /// `edges`, a path from `root`, as the concrete behaviour it stands for.
    pub(super) fn lift(
        &self,
        run: &Explored<'_, '_>,
        root: ProductStateId,
        edges: &[&RetainedEdge<HyperTransition>],
    ) -> Result<Lifted, ModelCheckRefusal> {
        let stored = |id: ProductStateId| {
            run.graph
                .state(id)
                .map(|state| &state.state)
                .ok_or_else(|| fault("a-path-runs-through-retained-states"))
        };
        let reorder = |state: &HyperState, positions: &[usize]| {
            HyperState::new(
                through(&state.components, positions),
                state.property.clone(),
            )
        };
        let root_state = stored(root)?;
        let mut positions: Vec<usize> = (0..root_state.components.len()).collect();
        // The renaming that takes the concrete keys to the stored state's.
        let mut stored_keys = Permutation::identity();
        let mut states = vec![root_state.clone()];
        let mut steps = Vec::with_capacity(edges.len());
        for edge in edges {
            let target = stored(edge.target)?;
            let slots = self.slots(&edge.permutation, target.components.len())?;
            let step = self.renamed_step(
                &HyperTransition(through(&edge.transition.0, &positions)),
                &stored_keys.inverse(),
            )?;
            positions = positions
                .iter()
                .map(|from| {
                    slots
                        .get(*from)
                        .copied()
                        .ok_or_else(|| fault("a-slot-for-every-component"))
                })
                .collect::<Result<_, _>>()?;
            stored_keys = key_part(&edge.permutation).compose(&stored_keys);
            let concrete = self.renamed(reorder(target, &positions), &stored_keys.inverse())?;
            steps.push((step, self.state_keys(&concrete)?));
            states.push(concrete);
        }
        Ok(Lifted {
            states,
            steps,
            positions,
            renaming: stored_keys.inverse(),
        })
    }

    /// How many times the loop `edges` is traversed for the concrete tuple to
    /// close: the order of the permutation of components the loop composes
    /// to. One under the trivial group.
    pub(super) fn repeats(
        &self,
        edges: &[&RetainedEdge<HyperTransition>],
        components: usize,
    ) -> Result<usize, ModelCheckRefusal> {
        let identity: Vec<usize> = (0..components).collect();
        let mut once = identity.clone();
        let mut once_keys = Permutation::identity();
        for edge in edges {
            let slots = self.slots(&edge.permutation, components)?;
            once = once
                .iter()
                .map(|from| {
                    slots
                        .get(*from)
                        .copied()
                        .ok_or_else(|| fault("a-slot-for-every-component"))
                })
                .collect::<Result<_, _>>()?;
            once_keys = key_part(&edge.permutation).compose(&once_keys);
        }
        let mut repeats = 1;
        let mut composed = once.clone();
        let mut composed_keys = once_keys.clone();
        while composed != identity || !composed_keys.is_identity() {
            composed = composed.iter().map(|at| once[*at]).collect();
            composed_keys = once_keys.compose(&composed_keys);
            repeats += 1;
        }
        Ok(repeats)
    }
}
