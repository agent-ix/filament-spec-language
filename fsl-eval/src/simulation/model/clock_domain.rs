// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106 "Clock-reference domains": the finite candidate set of a
//! `Reference<Clock>` value, read from declared clock places, the exact
//! admitted owner populations and the selector range, never from a map key,
//! an allocation ordinal or a state-node id.

use std::collections::BTreeSet;

use filament_exact::ObjectReference;
use fsl_semantics::model::clock_reference::identity::{
    Decimal, PathEntry, PlaceIdentity, SlotIndex,
};
use fsl_semantics::model::clock_reference::{encode_identity, is_clock_type};

use super::domain::ClockSelectorMaxima;
use super::ModelSystem;

impl ModelSystem<'_> {
    /// Every valid `Reference<Clock>` identity of this model universe under the
    /// declared selector maxima (FR-106 "Clock-reference domains"): the declared
    /// clock places of each admitted owner, then each identity the initial
    /// snapshots admitted. A place with a collection slot that declares no
    /// maximum contributes none; the bound check reports it as `RequiresBound`.
    pub fn clock_reference_identities(&self) -> &[ObjectReference] {
        self.clock_reference_candidates(None)
    }

    /// Every valid `Reference<Clock>` identity of this system's model
    /// universe: the identities of declared clock places of admitted owners
    /// with each selector below its maximum, then each valid identity the
    /// initial snapshots already admitted (even when its selector lies beyond
    /// that maximum), in that order and without repeats.
    ///
    /// The maximum of a collection slot is the exploration-derived one in
    /// `selectors` when the caller has it, else the declared one; a template
    /// with a slot that has neither contributes no candidates, because the
    /// bound check refuses that model before exploring it.
    pub(super) fn clock_reference_candidates(
        &self,
        selectors: Option<&ClockSelectorMaxima>,
    ) -> &[ObjectReference] {
        self.clock_candidates[usize::from(selectors.is_some())]
            .get_or_init(|| self.build_clock_reference_candidates(selectors))
    }

    fn build_clock_reference_candidates(
        &self,
        selectors: Option<&ClockSelectorMaxima>,
    ) -> Vec<ObjectReference> {
        let mut found = Vec::new();
        let mut seen = BTreeSet::new();
        let limits =
            quire_canonical::Limits::new(fsl_foundation::IdentityLimits::default().input_bytes);
        if let Some(table) = self.input.clock_declarations {
            let source = table.source_declarations();
            for population in &self.populations {
                let Some(keys) = self.universes.get(&population.key) else {
                    continue;
                };
                let scope = fsl_semantics::model::key::population_node_key(&population.key)
                    .ok()
                    .and_then(|node| source.declaration_scope_for_node(&node))
                    .or_else(|| source.catalog_scope(&population.key))
                    .cloned()
                    .or_else(|| source.scope(&population.key).ok());
                let Some(scope) = scope else {
                    continue;
                };
                for key in keys {
                    let Some(object) = self
                        .initial
                        .iter()
                        .find_map(|state| state.state.populations.get(&population.key)?.get(key))
                    else {
                        continue;
                    };
                    let Ok(owner) = self.model.object_reference(&object.type_identity, key) else {
                        continue;
                    };
                    for (node, place) in table.run_places() {
                        if !self
                            .types
                            .conforms(owner.object_type().clone(), place.owner.clone())
                        {
                            continue;
                        }
                        let Some(selections) =
                            selector_ranges(&place.template.path, node, selectors)
                        else {
                            continue;
                        };
                        for indices in product(&selections) {
                            let identity = PlaceIdentity {
                                population: scope.clone(),
                                owner: key.clone(),
                                template: place.template.as_ref().clone(),
                                indices,
                            };
                            let Ok(reference) = encode_identity(&owner, &identity, limits) else {
                                continue;
                            };
                            if seen.insert(reference.clone()) {
                                found.push(reference);
                            }
                        }
                    }
                }
            }
        }
        let mut admitted = Vec::new();
        for state in &self.initial {
            for object in state
                .state
                .populations
                .values()
                .flat_map(|objects| objects.values())
            {
                for value in object.fields.values() {
                    let mut named = Vec::new();
                    if super::frames::collect_references(value, &mut named).is_err() {
                        continue;
                    }
                    admitted.extend(
                        named
                            .into_iter()
                            .filter(|reference| is_clock_type(reference.object_type()))
                            .cloned(),
                    );
                }
            }
        }
        admitted.sort();
        for reference in admitted {
            if seen.insert(reference.clone()) {
                found.push(reference);
            }
        }
        found
    }
}

/// The selector range of each collection entry on `path`, in path order, or
/// `None` when an entry has no maximum.
fn selector_ranges(
    path: &[PathEntry],
    node: &filament_exact::NodeKey,
    selectors: Option<&ClockSelectorMaxima>,
) -> Option<Vec<(Decimal, u64)>> {
    let derived = selectors.and_then(|maxima| maxima.get(node));
    let mut slot = 0_usize;
    let mut ranges = Vec::new();
    for (position, entry) in path.iter().enumerate() {
        let PathEntry::Collection { shape, .. } = entry else {
            continue;
        };
        let declared = shape
            .maximum
            .as_ref()
            .and_then(|maximum| maximum.as_str().parse::<u64>().ok());
        let maximum = derived
            .and_then(|maxima| maxima.get(slot))
            .copied()
            .or(declared)?;
        slot += 1;
        ranges.push((Decimal::from(u64::try_from(position).ok()?), maximum));
    }
    Some(ranges)
}

/// Every selector tuple below the given maxima, the first slot most
/// significant, ascending.
fn product(ranges: &[(Decimal, u64)]) -> Vec<Vec<SlotIndex>> {
    let mut tuples = vec![Vec::new()];
    for (position, maximum) in ranges {
        let mut next = Vec::new();
        for prefix in &tuples {
            for index in 0..*maximum {
                let mut tuple: Vec<SlotIndex> = prefix.clone();
                tuple.push(SlotIndex {
                    path_position: position.clone(),
                    index: Decimal::from(index),
                });
                next.push(tuple);
            }
        }
        tuples = next;
    }
    tuples
}
