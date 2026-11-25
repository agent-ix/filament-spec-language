// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174/184: canonical finite prefixes. Group bookkeeping uses the run's
//! scalar work meter, independently of the joint-move and stored-state limits.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use filament_exact::{Cancel, Charge, ChargePoint, LimitKind, Meter};
use fsl_eval::hyper::{ClosedExecution, ParameterBinding};
use fsl_eval::simulation::{ExpansionStop, Permutation};
use fsl_replay::{HyperOutcome, ModelCheckLimit};

use super::super::{fault, reduce, widen, Run, Totals};
use crate::model_check::ModelCheckRefusal;

type Position = (Vec<u8>, Option<(Vec<u8>, bool)>);

/// Concrete state and incoming-step keys in joint-position/quantifier order.
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct TupleKey(Vec<Vec<Option<Position>>>);

pub(super) enum Stop {
    Cancelled,
    Meter(LimitKind),
    Limit(ModelCheckLimit, u64),
    Expansion(usize, ExpansionStop),
    Refused(Box<ModelCheckRefusal>),
    Outcome(Box<HyperOutcome>),
}

impl From<ModelCheckRefusal> for Stop {
    fn from(refusal: ModelCheckRefusal) -> Self {
        Self::Refused(Box::new(refusal))
    }
}

/// The same graph-work ownership as model_check::quotient::Analyzer: each
/// traversal/permutation step charges GraphExpand and observes cancellation.
pub(super) fn charge(meter: &mut Meter, cancel: &Cancel) -> Result<(), Stop> {
    if cancel.poll() {
        return Err(Stop::Cancelled);
    }
    meter
        .charge(Charge::new(ChargePoint::GraphExpand))
        .map_err(|incomplete| {
            if cancel.cause().is_some() {
                Stop::Cancelled
            } else {
                Stop::Meter(incomplete.limit_kind)
            }
        })
}

#[derive(Clone, Eq, PartialEq)]
pub(super) struct Member {
    positions: Vec<usize>,
    renaming: Permutation,
}

/// A real traversed tuple, ordered by its actual key rather than a synthetic
/// renamed path. The canonical image is only this member's map key.
pub(super) struct Representative {
    pub(super) actual: TupleKey,
    pub(super) paths: Vec<ClosedExecution>,
}

#[derive(Default)]
pub(super) struct Scope {
    pub(super) complete: BTreeMap<TupleKey, Representative>,
    pub(super) boundaries: BTreeMap<TupleKey, Representative>,
    pub(super) judged: BTreeSet<TupleKey>,
}

type ComponentOrder = (usize, Vec<(Vec<u8>, Vec<u8>)>);

/// A prefix is a lower bound on every completion below it. Ordinary order
/// is component-major; selected order is the least joint-position orbit key.
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Order {
    Ordinary(Vec<ComponentOrder>),
    Selected(TupleKey),
}

#[derive(Clone, Copy)]
pub(super) enum Admission {
    Complete,
    Boundary,
}

impl Run<'_, '_> {
    /// Full actions are needed for a choice box: an action fixing today's
    /// prefix can still exchange its different future successors. Deduping
    /// actions by that prefix's image would discard those future orderings.
    pub(super) fn closed_order_members(
        &self,
        instance: &[ParameterBinding],
        selected: bool,
        meter: &mut Meter,
    ) -> Result<Vec<Member>, Stop> {
        let mut members = vec![Member {
            positions: (0..self.slots.len()).collect(),
            renaming: Permutation::identity(),
        }];
        if !selected {
            return Ok(members);
        }
        let generators = reduce::generators(&self.stabiliser_of(instance));
        let mut next = 0;
        while let Some(member) = members.get(next).cloned() {
            let mut following = Vec::new();
            for generator in &self.copy.generators {
                charge(meter, self.cancel)?;
                let mut image = member.clone();
                image.positions.swap(generator.first, generator.second);
                following.push(image);
            }
            for (_, generator) in &generators {
                charge(meter, self.cancel)?;
                following.push(Member {
                    positions: member.positions.clone(),
                    renaming: generator.compose(&member.renaming),
                });
            }
            for image in following {
                let mut known = false;
                for existing in &members {
                    charge(meter, self.cancel)?;
                    if *existing == image {
                        known = true;
                        break;
                    }
                }
                if !known {
                    members.push(image);
                }
            }
            next += 1;
        }
        Ok(members)
    }

    /// Exact minimum key of a compact Cartesian choice box, without building
    /// its joint tuples. For each fixed group action choices are independent,
    /// so their component minima give the minimum image of the entire box.
    pub(super) fn closed_choice_order(
        &self,
        choices: &[Vec<ClosedExecution>],
        ranges: &[Range<usize>],
        members: &[Member],
        selected: bool,
        meter: &mut Meter,
    ) -> Result<Order, Stop> {
        if !selected {
            let mut order = Vec::with_capacity(choices.len());
            for (component, paths) in choices.iter().enumerate() {
                let mut least = None;
                for index in ranges[component].clone() {
                    let path = &paths[index];
                    let system = &self.systems[self.slots[component]];
                    let mut steps = Vec::with_capacity(path.steps.len());
                    for step in &path.steps {
                        charge(meter, self.cancel)?;
                        steps.push((
                            system
                                .transition_key(&step.transition)
                                .map_err(ModelCheckRefusal::Encoding)?,
                            step.post_state.as_bytes().to_vec(),
                        ));
                    }
                    charge(meter, self.cancel)?;
                    let key = (path.initial, steps);
                    if least.as_ref().is_none_or(|known| key < *known) {
                        least = Some(key);
                    }
                }
                order.push(least.ok_or_else(|| fault("a-frontier-range-is-nonempty"))?);
            }
            return Ok(Order::Ordinary(order));
        }
        let mut least = None;
        for member in members {
            charge(meter, self.cancel)?;
            let mut components = Vec::with_capacity(choices.len());
            for (component, original) in member.positions.iter().copied().enumerate() {
                let mut minimum = None;
                for index in ranges[original].clone() {
                    let positions = self.closed_component_key(
                        &choices[original][index],
                        component,
                        &member.renaming,
                        meter,
                    )?;
                    if minimum.as_ref().is_none_or(|known| positions < *known) {
                        minimum = Some(positions);
                    }
                }
                components.push(minimum.ok_or_else(|| fault("a-frontier-range-is-nonempty"))?);
            }
            let key = Self::joint_key(components);
            if least.as_ref().is_none_or(|known| key < *known) {
                least = Some(key);
            }
        }
        Ok(Order::Selected(least.ok_or_else(|| {
            fault("a-frontier-has-an-identity-action")
        })?))
    }
    /// Retain one genuine completed tuple or cut predecessor per orbit. Only
    /// a newly retained complete member consumes the stored-state allowance.
    pub(super) fn retain_closed(
        &self,
        paths: &[ClosedExecution],
        instance: &[ParameterBinding],
        admission: Admission,
        scope: &mut Scope,
        totals: &mut Totals,
        meter: &mut Meter,
    ) -> Result<TupleKey, Stop> {
        let actual = self.closed_key(paths, meter)?;
        let selected = self.symmetric || self.request.reductions.constraint.is_some();
        let canonical = if selected {
            self.closed_canonical(paths, instance, meter)?
        } else {
            actual.clone()
        };
        let members = match admission {
            Admission::Complete => &mut scope.complete,
            Admission::Boundary => &mut scope.boundaries,
        };
        if let Some(known) = members.get_mut(&canonical) {
            if actual < known.actual {
                known.actual = actual;
                known.paths = paths.to_vec();
            }
            return Ok(canonical);
        }
        match admission {
            Admission::Complete => {
                let bound = widen(self.request.limits.limits.max_states);
                if totals.states >= bound {
                    return Err(Stop::Limit(ModelCheckLimit::MaxStates, bound));
                }
                totals.states += 1;
            }
            Admission::Boundary => totals.boundary = totals.boundary.saturating_add(1),
        }
        members.insert(
            canonical.clone(),
            Representative {
                actual,
                paths: paths.to_vec(),
            },
        );
        Ok(canonical)
    }

    /// The actual key of a concrete prefix, with no action applied.
    pub(super) fn closed_key(
        &self,
        tuple: &[ClosedExecution],
        meter: &mut Meter,
    ) -> Result<TupleKey, Stop> {
        self.closed_image_key(
            tuple,
            &Member {
                positions: (0..tuple.len()).collect(),
                renaming: Permutation::identity(),
            },
            meter,
        )
    }

    /// Least prefix key under the exact product of checked copy exchanges and
    /// the admitted diagonal stabiliser. Images supply keys only: the caller
    /// retains concrete traversed paths and verifies that their least key was
    /// actually visited before it exposes a closed-scope representative.
    pub(super) fn closed_canonical(
        &self,
        tuple: &[ClosedExecution],
        instance: &[ParameterBinding],
        meter: &mut Meter,
    ) -> Result<TupleKey, Stop> {
        let initial = Member {
            positions: (0..tuple.len()).collect(),
            renaming: Permutation::identity(),
        };
        let key = self.closed_image_key(tuple, &initial, meter)?;
        let mut seen = BTreeSet::from([key.clone()]);
        let mut least = key;
        let mut orbit = vec![initial];
        let generators = reduce::generators(&self.stabiliser_of(instance));
        let mut next = 0;
        while let Some(member) = orbit.get(next) {
            let mut found = Vec::new();
            for generator in &self.copy.generators {
                charge(meter, self.cancel)?;
                let mut positions = member.positions.clone();
                let (Some(_), Some(_)) = (
                    positions.get(generator.first),
                    positions.get(generator.second),
                ) else {
                    return Err(fault("a-copy-generator-names-two-closed-components").into());
                };
                positions.swap(generator.first, generator.second);
                let exchanged = Member {
                    positions,
                    renaming: member.renaming.clone(),
                };
                let key = self.closed_image_key(tuple, &exchanged, meter)?;
                if key < least {
                    least = key.clone();
                }
                if seen.insert(key) {
                    found.push(exchanged);
                }
            }
            for (_, generator) in &generators {
                charge(meter, self.cancel)?;
                let renamed = Member {
                    positions: member.positions.clone(),
                    renaming: generator.compose(&member.renaming),
                };
                let key = self.closed_image_key(tuple, &renamed, meter)?;
                if key < least {
                    least = key.clone();
                }
                if seen.insert(key) {
                    found.push(renamed);
                }
            }
            orbit.extend(found);
            next += 1;
        }
        Ok(least)
    }

    fn closed_image_key(
        &self,
        tuple: &[ClosedExecution],
        member: &Member,
        meter: &mut Meter,
    ) -> Result<TupleKey, Stop> {
        let mut components = Vec::with_capacity(tuple.len());
        for (component, original) in member.positions.iter().enumerate() {
            let path = tuple
                .get(*original)
                .ok_or_else(|| fault("a-closed-image-names-an-actual-path"))?;
            components.push(self.closed_component_key(path, component, &member.renaming, meter)?);
        }
        Ok(Self::joint_key(components))
    }

    fn closed_component_key(
        &self,
        path: &ClosedExecution,
        component: usize,
        renaming: &Permutation,
        meter: &mut Meter,
    ) -> Result<Vec<Position>, Stop> {
        let system = self
            .slots
            .get(component)
            .and_then(|slot| self.systems.get(*slot))
            .ok_or_else(|| fault("a-closed-image-has-a-component-system"))?;
        let mut positions = Vec::with_capacity(path.states.len());
        for (position, state) in path.states.iter().enumerate() {
            charge(meter, self.cancel)?;
            let state = system
                .permute(state, renaming)
                .map_err(|_| fault("a-closed-key-permutation-preserves-state-encoding"))?;
            let key = system
                .state_key(&state, fsl_foundation::IdentityLimits::default())
                .map_err(ModelCheckRefusal::Encoding)?
                .0;
            let incoming = position
                .checked_sub(1)
                .and_then(|index| path.steps.get(index))
                .map(|step| -> Result<_, ModelCheckRefusal> {
                    let transition = system
                        .permute_transition(&step.transition, renaming)
                        .map_err(|_| fault("a-closed-key-permutation-preserves-step-encoding"))?;
                    Ok((
                        system
                            .transition_key(&transition)
                            .map_err(ModelCheckRefusal::Encoding)?,
                        step.skipped,
                    ))
                })
                .transpose()?;
            positions.push((key, incoming));
        }
        if positions.is_empty() {
            return Err(fault("a-closed-path-has-an-initial-state").into());
        }
        Ok(positions)
    }

    fn joint_key(components: Vec<Vec<Position>>) -> TupleKey {
        let length = components.iter().map(Vec::len).max().unwrap_or(0);
        TupleKey(
            (0..length)
                .map(|position| {
                    components
                        .iter()
                        .map(|path| path.get(position).cloned())
                        .collect()
                })
                .collect(),
        )
    }
}
