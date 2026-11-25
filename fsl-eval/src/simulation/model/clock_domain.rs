// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106 "Clock-reference domains": the finite candidate set of a
//! `Reference<Clock>` value, read from declared clock places, the exact
//! admitted owner populations and the selector range, never from a map key,
//! an allocation ordinal or a state-node id.

use std::collections::BTreeSet;
use std::sync::Arc;

use filament_exact::{
    ClockPlaceReference, FieldValue, NodeKey, ObjectReference, Value, ValueType,
};
use fsl_semantics::model::clock_reference::identity::{
    Decimal, PathEntry, PlaceIdentity, SlotIndex,
};
use fsl_semantics::model::clock_reference::{encode_identity, is_clock_type};

use super::ModelSystem;
use super::domain::{
    ClockSelectorMaxima, Domain, clock_reference_as_object, clock_reference_paths,
};

/// Why a `Reference<Clock>` domain cannot be enumerated. None of these is an
/// empty domain: a caller never ranges a hidden field or parameter over fewer
/// values than FR-106 specifies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockDomainRefusal {
    /// A declared place has a collection slot with no declared maximum and no
    /// exploration-derived one; the bound check names it as `RequiresBound`.
    UnboundedSlot {
        /// The clock-place template.
        clock: NodeKey,
        /// The collection entry's position among the place's slots.
        slot: usize,
    },
    /// A declared maximum is not a machine integer.
    UnparseableMaximum {
        /// The clock-place template.
        clock: NodeKey,
        /// The declared maximum, as written.
        maximum: String,
    },
    /// Candidate generation or admitted-seed traversal exceeds its work bound.
    TooMany {
        /// The `max_candidates` bound.
        limit: u64,
    },
    /// This route carries no authenticated declaration table, so no owner
    /// population authority validates a place identity.
    MissingAuthority,
    /// An admitted clock-place value cannot be represented as a model identity.
    InvalidAdmittedSeed,
}

type Candidates = Result<Vec<ClockPlaceReference>, ClockDomainRefusal>;

impl ModelSystem<'_> {
    /// Every valid `Reference<Clock>` identity of this model universe under the
    /// declared selector maxima (FR-106 "Clock-reference domains"): the declared
    /// clock places of each admitted owner, then each identity the initial
    /// snapshots admitted.
    ///
    /// # Errors
    ///
    /// [`ClockDomainRefusal`] when the domain is not finite under the declared
    /// maxima, exceeds the candidate bound, or the route has no declaration
    /// table.
    pub fn clock_reference_identities(
        &self,
    ) -> Result<Arc<Vec<ObjectReference>>, ClockDomainRefusal> {
        Ok(Arc::new(
            self.clock_reference_candidates(None)?
                .iter()
                .map(clock_reference_as_object)
                .collect::<Option<Vec<_>>>()
                .ok_or(ClockDomainRefusal::InvalidAdmittedSeed)?,
        ))
    }

    /// [`Self::clock_reference_identities`] under the exploration-derived
    /// selector maxima `maxima` (one maximum per collection slot of each
    /// clock-place template), as a timed subject derives them from its
    /// `max_clocks` bound.
    ///
    /// # Errors
    ///
    /// As [`Self::clock_reference_identities`].
    pub fn clock_reference_identities_under(
        &self,
        maxima: &std::collections::BTreeMap<NodeKey, Vec<u64>>,
    ) -> Result<Arc<Vec<ObjectReference>>, ClockDomainRefusal> {
        Ok(Arc::new(
            self.clock_reference_candidates(Some(maxima))?
                .iter()
                .map(clock_reference_as_object)
                .collect::<Option<Vec<_>>>()
                .ok_or(ClockDomainRefusal::InvalidAdmittedSeed)?,
        ))
    }

    /// The candidates under the exploration-derived `selectors` when the caller
    /// has them, else the declared maxima. Cached per distinct selector set, so
    /// two timed subjects with different bounds never share one.
    pub(super) fn clock_reference_candidates(
        &self,
        selectors: Option<&ClockSelectorMaxima>,
    ) -> Result<Arc<Vec<ClockPlaceReference>>, ClockDomainRefusal> {
        let mut cache = self
            .clock_candidates
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache
            .entry(selectors.cloned())
            .or_insert_with(|| {
                self.build_clock_reference_candidates(selectors)
                    .map(Arc::new)
            })
            .clone()
    }

    /// The typed clock-place candidates when `value_type` contains one;
    /// `None` when those candidates cannot be enumerated.
    pub(super) fn clock_references_for(
        &self,
        value_type: &ValueType,
        selectors: Option<&ClockSelectorMaxima>,
    ) -> Option<Arc<Vec<ClockPlaceReference>>> {
        if clock_reference_paths(value_type, self.types).is_empty() {
            return Some(Arc::new(Vec::new()));
        }
        self.clock_reference_candidates(selectors).ok()
    }

    /// Whether `value_type` holds no Clock reference, or this universe's Clock
    /// reference candidates can be enumerated for it.
    pub(super) fn clock_candidates_exist(&self, value_type: &ValueType) -> bool {
        clock_reference_paths(value_type, self.types).is_empty()
            || self.clock_reference_candidates(None).is_ok()
    }

    /// FR-120's domain of `value_type` over `objects`, with the Clock
    /// references of this universe where the type holds any; `None` when the
    /// type has no finite domain here, so every route refuses or skips it as it
    /// does an unbounded type, never ranging it over nothing.
    pub(super) fn domain_with_clocks(
        &self,
        value_type: &ValueType,
        objects: &[ObjectReference],
    ) -> Option<Domain> {
        let clocks = self.clock_references_for(value_type, None)?;
        Domain::of_with_clock_references(
            value_type,
            objects,
            &clocks,
            self.types,
            &self.record_orders,
        )
    }

    pub(super) fn map_size_with_clocks(
        &self,
        value_type: &ValueType,
        objects: &[ObjectReference],
    ) -> Option<u128> {
        let clocks = self.clock_references_for(value_type, None)?;
        Domain::map_size_with_clock_references(
            value_type,
            objects,
            &clocks,
            self.types,
            &self.record_orders,
        )
    }

    fn build_clock_reference_candidates(
        &self,
        selectors: Option<&ClockSelectorMaxima>,
    ) -> Candidates {
        let Some(table) = self.input.clock_declarations else {
            return Err(ClockDomainRefusal::MissingAuthority);
        };
        let limit = self.limits.max_candidates;
        let mut found = Vec::new();
        let mut seen = BTreeSet::new();
        let mut charged: u128 = 0;
        let limits =
            quire_canonical::Limits::new(fsl_foundation::IdentityLimits::default().input_bytes);
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
                    .initial_base
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
                    let ranges = selector_ranges(&place.template.path, node, selectors)?;
                    // Charge the count before any identity is built.
                    let count = ranges.iter().fold(1_u128, |total, (_, maximum)| {
                        total.saturating_mul(u128::from(*maximum))
                    });
                    charged = charged.saturating_add(count);
                    if charged > u128::from(limit) {
                        return Err(ClockDomainRefusal::TooMany { limit });
                    }
                    for indices in product(&ranges) {
                        let identity = PlaceIdentity {
                            population: scope.clone(),
                            owner: key.clone(),
                            template: place.template.as_ref().clone(),
                            indices,
                        };
                        let Ok(reference) = encode_identity(&owner, &identity, limits) else {
                            continue;
                        };
                        let Ok(reference) = ClockPlaceReference::new(
                            owner.universe().clone(),
                            reference.object().as_str(),
                        ) else {
                            continue;
                        };
                        if seen.insert(reference.clone()) {
                            found.push(reference);
                        }
                    }
                }
            }
        }
        let mut admitted = Vec::new();
        for state in &self.initial_base {
            for object in state
                .state
                .populations
                .values()
                .flat_map(|objects| objects.values())
            {
                for value in object.fields.values() {
                    collect_clock_references(value, &mut admitted, &mut charged, limit)?;
                }
            }
        }
        admitted.sort();
        for reference in admitted {
            if seen.insert(reference.clone()) {
                found.push(reference);
            }
        }
        Ok(found)
    }
}

/// Gather clock references from every admitted value container. Snapshot
/// seeds are candidates even when they lie outside an exploration selector
/// maximum; converting them back to the model identity form preserves their
/// universe and canonical place id.
fn collect_clock_references(
    value: &Value,
    references: &mut Vec<ClockPlaceReference>,
    work: &mut u128,
    limit: u64,
) -> Result<(), ClockDomainRefusal> {
    match value {
        Value::ClockReference(reference) => {
            charge_seed(work, limit)?;
            references.push(reference.clone());
        }
        Value::Reference(reference) if is_clock_type(reference.object_type()) => {
            charge_seed(work, limit)?;
            references.push(
                ClockPlaceReference::new(reference.universe().clone(), reference.object().as_str())
                    .map_err(|_| ClockDomainRefusal::InvalidAdmittedSeed)?,
            );
        }
        Value::Option(option) => {
            if let Some(payload) = option.payload() {
                collect_clock_references(payload, references, work, limit)?;
            }
        }
        Value::Collection(collection) => {
            for element in collection.elements() {
                collect_clock_references(element, references, work, limit)?;
            }
        }
        Value::Composite(composite) => {
            for slot in composite.slots() {
                if let FieldValue::Present(value) = slot {
                    collect_clock_references(value, references, work, limit)?;
                }
            }
        }
        Value::Union(union) => {
            for value in union.payload() {
                collect_clock_references(value, references, work, limit)?;
            }
        }
        Value::Map(map) => {
            for (_, value) in map.entries() {
                collect_clock_references(value, references, work, limit)?;
            }
        }
        Value::Boolean(_)
        | Value::Integer(_)
        | Value::Rational(_)
        | Value::Decimal(_)
        | Value::Float(_)
        | Value::Quantity(_)
        | Value::Text(_)
        | Value::String(_)
        | Value::Bytes(_)
        | Value::JsonObject(_)
        | Value::Reference(_)
        | Value::Uuid(_)
        | Value::Timestamp(_)
        | Value::Enum(_)
        | Value::Population(_) => {}
    }
    Ok(())
}

fn charge_seed(work: &mut u128, limit: u64) -> Result<(), ClockDomainRefusal> {
    *work = work.saturating_add(1);
    if *work > u128::from(limit) {
        return Err(ClockDomainRefusal::TooMany { limit });
    }
    Ok(())
}

/// The selector range of each collection entry on `path`, in path order.
fn selector_ranges(
    path: &[PathEntry],
    node: &NodeKey,
    selectors: Option<&ClockSelectorMaxima>,
) -> Result<Vec<(Decimal, u64)>, ClockDomainRefusal> {
    let derived = selectors.and_then(|maxima| maxima.get(node));
    let mut slot = 0_usize;
    let mut ranges = Vec::new();
    for (position, entry) in path.iter().enumerate() {
        let PathEntry::Collection { shape, .. } = entry else {
            continue;
        };
        let maximum = match derived.and_then(|maxima| maxima.get(slot)).copied() {
            Some(maximum) => maximum,
            None => match &shape.maximum {
                Some(declared) => declared.as_str().parse::<u64>().map_err(|_| {
                    ClockDomainRefusal::UnparseableMaximum {
                        clock: node.clone(),
                        maximum: declared.as_str().to_owned(),
                    }
                })?,
                None => {
                    return Err(ClockDomainRefusal::UnboundedSlot {
                        clock: node.clone(),
                        slot,
                    });
                }
            },
        };
        slot += 1;
        let position =
            u64::try_from(position).map_err(|_| ClockDomainRefusal::UnparseableMaximum {
                clock: node.clone(),
                maximum: position.to_string(),
            })?;
        ranges.push((Decimal::from(position), maximum));
    }
    Ok(ranges)
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
#[cfg(test)]
mod tests {
    use filament_exact::{MapType, OptionValue, Value, ValueType, from_admitted_map};
    use ix_trace_rs::trace;

    use super::{ClockDomainRefusal, collect_clock_references};

    /// Repeated seed references are still charged during traversal, so a
    /// duplicate-heavy input cannot evade the candidate work bound.
    #[trace("FR-106-AC-80", "TC-3084")]
    #[test]
    fn clock_reference_seed_traversal_stops_at_its_work_bound() {
        let Value::ClockReference(seed) = crate::test_clock_reference::clock_reference_of("x")
        else {
            panic!("a clock-reference fixture");
        };
        let seed_text = seed.text().to_owned();
        let nested = OptionValue::from_admitted(
            ValueType::ClockReference,
            Some(Value::ClockReference(seed)),
        );
        let mut references = Vec::new();
        let mut work = 0;
        collect_clock_references(&nested, &mut references, &mut work, 1)
            .expect("the first seed fits the bound");
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].text(), seed_text);
        assert_eq!(
            collect_clock_references(&nested, &mut references, &mut work, 1),
            Err(ClockDomainRefusal::TooMany { limit: 1 })
        );
        assert_eq!(work, 2);
    }

    /// FR-106-AC-80 excludes clock references held only as map keys while
    /// retaining references held as map values.
    #[trace("FR-106-AC-80", "TC-3084")]
    #[test]
    fn clock_reference_seed_traversal_ignores_map_keys() {
        let Value::ClockReference(key) = crate::test_clock_reference::clock_reference_of("key")
        else {
            panic!("a clock-reference key fixture");
        };
        let Value::ClockReference(value) = crate::test_clock_reference::clock_reference_of("value")
        else {
            panic!("a clock-reference value fixture");
        };
        let key_only = from_admitted_map(
            MapType::new(ValueType::ClockReference, ValueType::Boolean, None)
                .expect("clock references and booleans are map types"),
            vec![(Value::ClockReference(key), Value::Boolean(false))],
        )
        .expect("the map key and value match their declared types");
        let mut references = Vec::new();
        let mut work = 0;
        collect_clock_references(&key_only, &mut references, &mut work, 1)
            .expect("the clock reference map key is ignored");
        assert!(references.is_empty());

        let value_only = from_admitted_map(
            MapType::new(ValueType::Boolean, ValueType::ClockReference, None)
                .expect("booleans and clock references are map types"),
            vec![(Value::Boolean(false), Value::ClockReference(value.clone()))],
        )
        .expect("the map value matches its declared type");
        let mut references = Vec::new();
        work = 0;
        collect_clock_references(&value_only, &mut references, &mut work, 1)
            .expect("the map value seed fits the bound");
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].text(), value.text());
    }
}
