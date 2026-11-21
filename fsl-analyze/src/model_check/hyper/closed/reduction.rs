// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174: the diagonal key action and copy exchanges on concrete closed
//! execution tuples. Representatives are actual retained paths, so a reduced
//! refutation needs no synthetic transition or rewritten initial identity.

use std::collections::{BTreeMap, BTreeSet};

use fsl_eval::hyper::{ClosedExecution, ParameterBinding, Tuples};
use fsl_eval::simulation::{Permutation, StateKey};
use fsl_replay::{HyperOutcome, ModelCheckLimit};

use super::super::{exhausted, fault, reduce, widen, Run, Totals};
use crate::model_check::ModelCheckRefusal;

/// A concrete path and whether its last position is a cut predecessor,
/// rather than a terminal or horizon-covering closed execution.
pub(super) struct Execution {
    pub(super) path: ClosedExecution,
    pub(super) boundary: bool,
}

/// A path's actual state and incoming step at each position. An absent
/// incoming step distinguishes the initial position from every real step.
type Position = (StateKey, Option<(Vec<u8>, bool)>);

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct PathKey {
    positions: Vec<Position>,
    boundary: bool,
}

/// Joint positions compare the component keys in quantifier order. A terminal
/// component contributes absence past its last position, never stuttering.
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct TupleKey {
    positions: Vec<Vec<Option<Position>>>,
    boundaries: Vec<Option<usize>>,
}

pub(super) struct Representative {
    pub(super) picked: Vec<usize>,
    pub(super) boundary: Option<TupleKey>,
}

impl Run<'_, '_> {
    /// Build the finite tuple quotient in least-key order. Every generator
    /// image is resolved to a real enumerated path; a missing image cannot
    /// silently discard a tuple or produce a false proof.
    pub(super) fn closed_quotient(
        &self,
        lists: &[Vec<Execution>],
        instance: &[ParameterBinding],
        totals: &mut Totals,
    ) -> Result<Result<Vec<Representative>, HyperOutcome>, ModelCheckRefusal> {
        let mut paths = Vec::with_capacity(lists.len());
        let mut lookups = Vec::with_capacity(lists.len());
        for (subject, list) in lists.iter().enumerate() {
            let mut keys = Vec::with_capacity(list.len());
            let mut lookup = BTreeMap::new();
            for (index, execution) in list.iter().enumerate() {
                let key = self.closed_path_key(subject, execution, &Permutation::identity())?;
                lookup.entry(key.clone()).or_insert(index);
                keys.push(key);
            }
            paths.push(keys);
            lookups.push(lookup);
        }
        let generators = reduce::generators(&self.stabiliser_of(instance));
        let mut images = Vec::with_capacity(generators.len());
        for (_, generator) in &generators {
            let mut subjects = Vec::with_capacity(lists.len());
            for (subject, list) in lists.iter().enumerate() {
                let mut mapped = Vec::with_capacity(list.len());
                for execution in list {
                    if let Some(outcome) = self.closed_charge(totals)? {
                        return Ok(Err(outcome));
                    }
                    let image = self.closed_path_key(subject, execution, generator)?;
                    let index = lookups[subject]
                        .get(&image)
                        .copied()
                        .ok_or_else(|| fault("a-closed-key-orbit-has-real-path-images"))?;
                    mapped.push(index);
                }
                subjects.push(mapped);
            }
            images.push(subjects);
        }
        let lens = self.slots.iter().map(|slot| lists[*slot].len()).collect();
        let mut represented = BTreeSet::new();
        let mut canonical = BTreeMap::new();
        for picked in Tuples::over(lens) {
            if let Some(outcome) = self.closed_charge(totals)? {
                return Ok(Err(outcome));
            }
            let key = self.closed_tuple_key(&picked, &paths, None)?;
            if represented.contains(&key) {
                continue;
            }
            let mut orbit = vec![picked];
            let mut seen = BTreeSet::from([key]);
            let mut next = 0;
            while let Some(member) = orbit.get(next) {
                let mut found = Vec::new();
                for generator in &self.copy.generators {
                    if let Some(outcome) = self.closed_charge(totals)? {
                        return Ok(Err(outcome));
                    }
                    let mut exchanged = member.clone();
                    let (Some(_), Some(_)) = (
                        exchanged.get(generator.first),
                        exchanged.get(generator.second),
                    ) else {
                        return Err(fault("a-copy-generator-names-two-closed-components"));
                    };
                    exchanged.swap(generator.first, generator.second);
                    let key = self.closed_tuple_key(&exchanged, &paths, None)?;
                    if seen.insert(key) {
                        found.push(exchanged);
                    }
                }
                for generator in &images {
                    if let Some(outcome) = self.closed_charge(totals)? {
                        return Ok(Err(outcome));
                    }
                    let mut renamed = Vec::with_capacity(member.len());
                    for (component, index) in member.iter().enumerate() {
                        let image = self
                            .slots
                            .get(component)
                            .and_then(|slot| generator.get(*slot))
                            .and_then(|images| images.get(*index))
                            .copied()
                            .ok_or_else(|| fault("a-closed-generator-maps-every-component"))?;
                        renamed.push(image);
                    }
                    let key = self.closed_tuple_key(&renamed, &paths, None)?;
                    if seen.insert(key) {
                        found.push(renamed);
                    }
                }
                orbit.extend(found);
                next += 1;
            }
            let mut least = None;
            let mut boundary = None;
            for member in orbit {
                let key = self.closed_tuple_key(&member, &paths, None)?;
                if least.as_ref().is_none_or(|(known, _)| key < *known) {
                    least = Some((key, member.clone()));
                }
                let at = member
                    .iter()
                    .enumerate()
                    .filter_map(|(component, index)| {
                        let path = &paths[self.slots[component]][*index];
                        path.boundary.then_some(path.positions.len() - 1)
                    })
                    .min();
                if let Some(at) = at {
                    let key = self.closed_tuple_key(&member, &paths, Some(at))?;
                    if boundary.as_ref().is_none_or(|known| key < *known) {
                        boundary = Some(key);
                    }
                }
            }
            represented.extend(seen);
            let (key, picked) = least.ok_or_else(|| fault("a-closed-orbit-has-a-member"))?;
            canonical.insert(key, Representative { picked, boundary });
            if canonical.len() > self.request.limits.limits.max_states {
                return Ok(Err(exhausted(
                    ModelCheckLimit::MaxStates,
                    widen(self.request.limits.limits.max_states),
                    totals,
                )));
            }
        }
        Ok(Ok(canonical.into_values().collect()))
    }

    /// Charge both tuple enumeration and generator applications. The finite
    /// orbit walk is bounded by the caller's transition budget, including
    /// candidates which canonicalisation later removes.
    fn closed_charge(
        &self,
        totals: &mut Totals,
    ) -> Result<Option<HyperOutcome>, ModelCheckRefusal> {
        if self.cancel.poll() {
            return self.cancelled(totals).map(Some);
        }
        let limit = widen(self.request.limits.limits.max_transitions);
        if totals.edges >= limit {
            return Ok(Some(exhausted(
                ModelCheckLimit::MaxTransitions,
                limit,
                totals,
            )));
        }
        totals.edges += 1;
        Ok(None)
    }

    fn closed_path_key(
        &self,
        subject: usize,
        execution: &Execution,
        permutation: &Permutation,
    ) -> Result<PathKey, ModelCheckRefusal> {
        let system = self
            .systems
            .get(subject)
            .ok_or_else(|| fault("a-closed-path-has-a-subject"))?;
        let mut positions = Vec::with_capacity(execution.path.states.len());
        for (position, state) in execution.path.states.iter().enumerate() {
            let state = system
                .permute(state, permutation)
                .map_err(|_| fault("a-closed-key-permutation-preserves-state-encoding"))?;
            let key = system
                .state_key(&state)
                .map_err(ModelCheckRefusal::Encoding)?
                .0;
            let incoming = position
                .checked_sub(1)
                .and_then(|index| execution.path.steps.get(index))
                .map(|step| {
                    let transition = system
                        .permute_transition(&step.transition, permutation)
                        .map_err(|_| fault("a-closed-key-permutation-preserves-step-encoding"))?;
                    let key = system
                        .transition_key(&transition)
                        .map_err(ModelCheckRefusal::Encoding)?;
                    Ok((key, step.skipped))
                })
                .transpose()?;
            positions.push((key, incoming));
        }
        if positions.is_empty() {
            return Err(fault("a-closed-path-has-an-initial-state"));
        }
        Ok(PathKey {
            positions,
            boundary: execution.boundary,
        })
    }

    fn closed_tuple_key(
        &self,
        picked: &[usize],
        paths: &[Vec<PathKey>],
        through: Option<usize>,
    ) -> Result<TupleKey, ModelCheckRefusal> {
        let mut components = Vec::with_capacity(picked.len());
        for (component, index) in picked.iter().enumerate() {
            let path = self
                .slots
                .get(component)
                .and_then(|slot| paths.get(*slot))
                .and_then(|paths| paths.get(*index))
                .ok_or_else(|| fault("a-closed-tuple-key-names-listed-paths"))?;
            components.push(path);
        }
        let length = through.map_or_else(
            || {
                components
                    .iter()
                    .map(|path| path.positions.len())
                    .max()
                    .unwrap_or(0)
            },
            |position| position.saturating_add(1),
        );
        let positions = (0..length)
            .map(|position| {
                components
                    .iter()
                    .map(|path| path.positions.get(position).cloned())
                    .collect()
            })
            .collect();
        // A boundary identifies its predecessor, independently of which
        // component or how many successors were cut there.
        let boundaries = if through.is_some() {
            Vec::new()
        } else {
            components
                .iter()
                .map(|path| path.boundary.then_some(path.positions.len() - 1))
                .collect()
        };
        Ok(TupleKey {
            positions,
            boundaries,
        })
    }
}
