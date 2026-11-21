// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174: the diagonal key action and copy exchanges on concrete closed
//! execution tuples. Representatives are actual retained paths, so a reduced
//! refutation needs no synthetic transition or rewritten initial identity.

use std::collections::{BTreeMap, BTreeSet};

use fsl_eval::hyper::{ClosedExecution, ParameterBinding, Tuples};
use fsl_eval::simulation::Permutation;
use fsl_replay::{HyperOutcome, ModelCheckLimit};

use super::super::{exhausted, fault, reduce, widen, Run, Totals};
use crate::model_check::ModelCheckRefusal;

/// A path's actual state and incoming step at each position. An absent
/// incoming step distinguishes the initial position from every real step.
type Position = (Vec<u8>, Option<(Vec<u8>, bool)>);

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct PathKey {
    positions: Vec<Position>,
}

/// Joint positions compare the component keys in quantifier order. A terminal
/// component contributes absence past its last position, never stuttering.
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct TupleKey {
    positions: Vec<Vec<Option<Position>>>,
}

pub(super) struct Representative {
    pub(super) picked: Vec<usize>,
    pub(super) orbit: Vec<Vec<usize>>,
}

pub(super) struct Quotient {
    pub(super) representatives: Vec<Representative>,
    paths: Vec<Vec<PathKey>>,
}

impl Run<'_, '_> {
    /// Build the finite tuple quotient in least-key order. Every generator
    /// image is resolved to a real enumerated path; a missing image cannot
    /// silently discard a tuple or produce a false proof.
    pub(super) fn closed_quotient(
        &self,
        lists: &[Vec<ClosedExecution>],
        instance: &[ParameterBinding],
        totals: &mut Totals,
    ) -> Result<Result<Quotient, HyperOutcome>, ModelCheckRefusal> {
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
        for raw in Tuples::over(lens) {
            if let Some(outcome) = self.closed_charge(totals)? {
                return Ok(Err(outcome));
            }
            // Duplicate initial records or transitions can describe the same
            // concrete path. Keep its first real index and cache orbit members
            // by these compact indices, rather than copying every state's
            // encoded bytes into the cache for every tuple.
            let mut picked = Vec::with_capacity(raw.len());
            for (component, index) in raw.into_iter().enumerate() {
                let subject = self.slots[component];
                let first = lookups[subject]
                    .get(&paths[subject][index])
                    .copied()
                    .ok_or_else(|| fault("an-enumerated-closed-path-has-a-key"))?;
                picked.push(first);
            }
            if represented.contains(&picked) {
                continue;
            }
            let mut seen = BTreeSet::from([picked.clone()]);
            let mut orbit = vec![picked];
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
                    if seen.insert(exchanged.clone()) {
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
                    if seen.insert(renamed.clone()) {
                        found.push(renamed);
                    }
                }
                orbit.extend(found);
                next += 1;
            }
            let mut least = None;
            for member in &orbit {
                let key = self.closed_tuple_key(member, &paths, None)?;
                if least.as_ref().is_none_or(|(known, _)| key < *known) {
                    least = Some((key, member.clone()));
                }
            }
            represented.extend(seen);
            let (key, picked) = least.ok_or_else(|| fault("a-closed-orbit-has-a-member"))?;
            canonical.insert(key, Representative { picked, orbit });
            if canonical.len() > self.request.limits.limits.max_states {
                return Ok(Err(exhausted(
                    ModelCheckLimit::MaxStates,
                    widen(self.request.limits.limits.max_states),
                    totals,
                )));
            }
        }
        Ok(Ok(Quotient {
            representatives: canonical.into_values().collect(),
            paths,
        }))
    }

    /// The least concrete predecessor prefix under the same diagonal and
    /// copy action, independent of which component's successor was cut.
    pub(super) fn closed_boundary_key(
        &self,
        representative: &Representative,
        quotient: &Quotient,
        at: usize,
    ) -> Result<TupleKey, ModelCheckRefusal> {
        let mut least = None;
        for member in &representative.orbit {
            let key = self.closed_tuple_key(member, &quotient.paths, Some(at))?;
            if least.as_ref().is_none_or(|known| key < *known) {
                least = Some(key);
            }
        }
        least.ok_or_else(|| fault("a-closed-boundary-orbit-has-a-predecessor"))
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
        execution: &ClosedExecution,
        permutation: &Permutation,
    ) -> Result<PathKey, ModelCheckRefusal> {
        let system = self
            .systems
            .get(subject)
            .ok_or_else(|| fault("a-closed-path-has-a-subject"))?;
        let mut positions = Vec::with_capacity(execution.states.len());
        for (position, state) in execution.states.iter().enumerate() {
            let state = system
                .permute(state, permutation)
                .map_err(|_| fault("a-closed-key-permutation-preserves-state-encoding"))?;
            let key = system
                .state_key(&state)
                .map_err(ModelCheckRefusal::Encoding)?
                .0;
            let incoming = position
                .checked_sub(1)
                .and_then(|index| execution.steps.get(index))
                .map(|step| -> Result<_, ModelCheckRefusal> {
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
        Ok(PathKey { positions })
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
        Ok(TupleKey { positions })
    }
}
