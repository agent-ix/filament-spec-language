// SPDX-License-Identifier: AGPL-3.0-or-later
//! The core's own timed machinery (ADR-026 CF-3): the fresh exploration
//! FR-237's replay runs, apart from EN-6's, over the certificate checker's
//! zones.

pub(crate) mod explore;

use std::collections::{BTreeMap, BTreeSet};

use filament_exact::Rational;
use fsl_eval::simulation::model::TimedSystem;
use fsl_semantics::check::{ClockConstraint, ConvexPart, PlaceStep, Written};
use ix_trace_rs::implements;

/// Every atomic clock constraint the model compares a clock with: each guard
/// part's, those of each quantifier family below it (FR-230 "Family parts"),
/// and each time invariant's bounds. The core's own walk, apart from every
/// engine's (ADR-026 CF-6).
#[implements("FR-245-AC-2")]
pub(crate) fn clock_constraints<'a>(timed: &'a TimedSystem<'_, '_>) -> Vec<&'a ClockConstraint> {
    let model = timed.model();
    let mut parts: Vec<&ConvexPart> = model
        .guards()
        .iter()
        .flat_map(|guard| guard.parts())
        .collect();
    let mut found = Vec::new();
    let mut next = 0;
    while let Some(part) = parts.get(next).copied() {
        next += 1;
        found.extend(part.clocks());
        parts.extend(part.families().iter().flat_map(|family| family.parts()));
    }
    found.extend(
        model
            .invariants()
            .iter()
            .flat_map(|invariant| invariant.bounds()),
    );
    found
}

/// Every constant the model compares a clock with or sets one to: each of
/// [`clock_constraints`]'s bounds and each reset's value.
pub(crate) fn model_constants<'a>(timed: &'a TimedSystem<'_, '_>) -> Vec<&'a Rational> {
    clock_constraints(timed)
        .into_iter()
        .map(|constraint| constraint.bound())
        .chain(
            timed
                .model()
                .resets()
                .iter()
                .flat_map(|resets| resets.resets())
                .map(|reset| reset.value()),
        )
        .collect()
}

/// Raise each clock field's entry of `largest`, indexed by
/// [`fsl_semantics::check::ClockId`], to the entry of every field its value
/// reaches through collection writes (FR-230 "Collection-valued writes"):
/// the constants of the field a value is carried into are compared with it.
/// The core's own reading of the writes, apart from every engine's (ADR-026
/// CF-6).
pub(crate) fn follow_carries<T: Clone + Ord>(timed: &TimedSystem<'_, '_>, largest: &mut [T]) {
    let model = timed.model();
    // Each clock field in a collection: the collection field, and the path
    // below its slot with collection shapes erased.
    let slotted: Vec<(usize, &str, Vec<Option<&PlaceStep>>)> = model
        .clock_fields()
        .filter_map(|(id, field)| match field.path() {
            [PlaceStep::Field(collection), PlaceStep::Slot(_), below @ ..] => Some((
                id.index(),
                collection.as_str(),
                below
                    .iter()
                    .map(|step| (!matches!(step, PlaceStep::Slot(_))).then_some(step))
                    .collect(),
            )),
            _ => None,
        })
        .collect();
    // For each collection field, the collection fields it is written into.
    let mut written_into: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for write in model.resets().iter().flat_map(|resets| resets.writes()) {
        let mut pending = vec![write.written()];
        while let Some(written) = pending.pop() {
            match written {
                Written::Field { field, .. } => {
                    written_into
                        .entry(field.as_str())
                        .or_default()
                        .insert(write.field());
                }
                Written::Element(_) | Written::Fresh(_) => {}
                Written::Literal { occurrences, .. } => pending.extend(occurrences),
                Written::Filter(inner) | Written::Flatten(inner) | Written::Convert(inner) => {
                    pending.push(inner);
                }
                Written::Map { source, body } => pending.extend([&**source, &**body]),
            }
        }
    }
    // Worklist: a field whose entry rose may raise the fields it reads from.
    let mut pending: Vec<usize> = (0..slotted.len()).collect();
    while let Some(next) = pending.pop() {
        let Some((into, collection, below)) = slotted.get(next) else {
            continue;
        };
        let Some(value) = largest.get(*into).cloned() else {
            continue;
        };
        for (at, (from, source, source_below)) in slotted.iter().enumerate() {
            let carried = source_below == below
                && written_into
                    .get(source)
                    .is_some_and(|targets| targets.contains(collection));
            if !carried {
                continue;
            }
            if let Some(entry) = largest.get_mut(*from) {
                if *entry < value {
                    *entry = value.clone();
                    pending.push(at);
                }
            }
        }
    }
}
