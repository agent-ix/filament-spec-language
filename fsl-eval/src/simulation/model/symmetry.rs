// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 SYM-1 and SYM-7 (FR-151, FR-152): how a permutation of object
//! keys acts on a model state and a transition identity, and the sort
//! canonicaliser.
//!
//! A permutation renames every reference whose universe and key it moves,
//! in every field, collection and record, and moves each object of a
//! permuted population to its key's image; a set or bag is re-sorted into
//! its canonical-key order after the renaming. A transition identity is
//! renamed in its receiver and its reference arguments.

use std::collections::BTreeMap;

use filament_checked::key::DeclarationKey;
use filament_exact::{
    compare_keys, from_admitted, from_admitted_map, from_admitted_slots, CollectionKind,
    FieldValue, ObjectId, ObjectReference, OptionValue, UnionValue, UniverseId, Value,
};
use filament_semantic_value::model_transition::ModelTransition;
use fsl_semantics::model::state::{ModelState, StateObject};
use quire_canonical::Limits;

use super::encode::{self, declaration_text, Enc};
use super::walk::stop;
use super::{identity_limits, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::reduction::{Permutation, SymmetryGroup};

/// Where a permutation's keys live: each moved population's universes and
/// its key map.
struct Renaming<'p> {
    moved: Vec<(
        Vec<UniverseId>,
        &'p DeclarationKey,
        &'p BTreeMap<String, String>,
    )>,
}

impl Renaming<'_> {
    /// The reference `reference` is renamed to, when the permutation moves
    /// its key.
    fn reference(&self, reference: &ObjectReference) -> Option<ObjectReference> {
        self.moved.iter().find_map(|(universes, _, map)| {
            if !universes.contains(reference.universe()) {
                return None;
            }
            let image = map.get(reference.object().as_str())?;
            let object = ObjectId::new(image.clone()).ok()?;
            Some(ObjectReference::new(
                reference.universe().clone(),
                reference.object_type().clone(),
                object,
            ))
        })
    }

    /// `value` with every reference renamed; a set or bag is re-sorted into
    /// its canonical-key order.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when two elements
    /// of a renamed set or bag have no canonical-key order.
    fn value(&self, value: &Value) -> Result<Value, ExpansionStop> {
        Ok(match value {
            Value::Reference(reference) => self
                .reference(reference)
                .map_or_else(|| value.clone(), Value::Reference),
            Value::Option(option) => OptionValue::from_admitted(
                option.payload_type().clone(),
                option
                    .payload()
                    .map(|payload| self.value(payload))
                    .transpose()?,
            ),
            Value::Collection(collection) => {
                let mut elements = collection
                    .elements()
                    .iter()
                    .map(|element| self.value(element))
                    .collect::<Result<Vec<Value>, _>>()?;
                if matches!(
                    collection.collection_type().kind(),
                    CollectionKind::Set | CollectionKind::Bag
                ) {
                    let mut unordered = false;
                    elements.sort_by(|left, right| {
                        compare_keys(left, right).unwrap_or_else(|| {
                            unordered = true;
                            std::cmp::Ordering::Equal
                        })
                    });
                    if unordered {
                        return Err(stop(BROKEN));
                    }
                }
                from_admitted(collection.collection_type().clone(), elements)
            }
            Value::Composite(composite) => {
                let slots = composite
                    .slots()
                    .iter()
                    .map(|slot| {
                        Ok(match slot {
                            FieldValue::Present(value) => FieldValue::Present(self.value(value)?),
                            FieldValue::Absent => FieldValue::Absent,
                            FieldValue::Null => FieldValue::Null,
                        })
                    })
                    .collect::<Result<Vec<FieldValue>, ExpansionStop>>()?;
                from_admitted_slots(composite.declaration().clone(), slots.into_boxed_slice())
            }
            Value::Union(union) => UnionValue::from_admitted(
                union.member().clone(),
                union
                    .payload()
                    .iter()
                    .map(|value| self.value(value))
                    .collect::<Result<Vec<Value>, _>>()?,
            ),
            Value::Map(map) => {
                let entries = map
                    .entries()
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), self.value(value)?)))
                    .collect::<Result<Vec<_>, ExpansionStop>>()?;
                from_admitted_map(map.map_type().clone(), entries).map_err(|_| stop(BROKEN))?
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
            | Value::Uuid(_)
            | Value::Timestamp(_)
            | Value::Enum(_)
            | Value::ClockReference(_)
            | Value::Population(_) => value.clone(),
        })
    }

    fn object(&self, object: &StateObject) -> Result<StateObject, ExpansionStop> {
        Ok(StateObject {
            type_identity: object.type_identity.clone(),
            fields: object
                .fields
                .iter()
                .map(|(field, value)| Ok((field.clone(), self.value(value)?)))
                .collect::<Result<_, ExpansionStop>>()?,
        })
    }

    fn key<'k>(&'k self, population: &DeclarationKey, key: &'k str) -> &'k str {
        self.moved
            .iter()
            .find(|(_, moved, _)| *moved == population)
            .and_then(|(_, _, map)| map.get(key))
            .map_or(key, String::as_str)
    }
}

impl ModelSystem<'_> {
    /// Whether the population `population` is declared `symmetric`
    /// (FR-150); `None` when no selected model declares it.
    pub fn is_symmetric(&self, population: &DeclarationKey) -> Option<bool> {
        self.populations
            .iter()
            .find(|declared| declared.key == *population)
            .map(|declared| declared.symmetric)
    }

    /// The keys of `population`'s universe, in the order the subject lists
    /// them; `None` when the subject has no universe for it.
    pub fn universe(&self, population: &DeclarationKey) -> Option<&[String]> {
        self.universes.get(population).map(Vec::as_slice)
    }

    /// The universes the objects of `population` are referenced in: one per
    /// declared member type.
    pub(super) fn universes_of(&self, population: &DeclarationKey) -> Vec<UniverseId> {
        let mut universes = Vec::new();
        let Some(declared) = self
            .populations
            .iter()
            .find(|declared| declared.key == *population)
        else {
            return universes;
        };
        for member in &declared.member_types {
            // A reference's universe is its type's; any key names it.
            if let Ok(reference) = self.model.object_reference(member, "k") {
                if !universes.contains(reference.universe()) {
                    universes.push(reference.universe().clone());
                }
            }
        }
        universes
    }

    fn renaming<'p>(&self, permutation: &'p Permutation) -> Renaming<'p> {
        Renaming {
            moved: permutation
                .maps()
                .iter()
                .map(|(population, map)| (self.universes_of(population), population, map))
                .collect(),
        }
    }

    /// ADR-021 SYM-1: `permutation` applied to `state`: every object of a
    /// moved population moves to its key's image, and every reference whose
    /// key it moves is renamed, in every field, collection, record and
    /// relationship extent. The result has no reaching step.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when a renamed set
    /// or bag has no canonical-key order.
    pub fn permute(
        &self,
        state: &ModelSimState,
        permutation: &Permutation,
    ) -> Result<ModelSimState, ExpansionStop> {
        Ok(state.without_effect(self.permute_state(state.state(), permutation)?))
    }

    /// The state-only projection of [`Self::permute`], for initial recipes
    /// whose key is decided before a `ModelSimState` is materialized.
    pub(super) fn permute_state(
        &self,
        state: &ModelState,
        permutation: &Permutation,
    ) -> Result<ModelState, ExpansionStop> {
        if permutation.is_identity() {
            return Ok(state.clone());
        }
        let renaming = self.renaming(permutation);
        let populations = state
            .populations
            .iter()
            .map(|(population, objects)| {
                let objects = objects
                    .iter()
                    .map(|(key, object)| {
                        Ok((
                            renaming.key(population, key).to_owned(),
                            renaming.object(object)?,
                        ))
                    })
                    .collect::<Result<_, ExpansionStop>>()?;
                Ok((population.clone(), objects))
            })
            .collect::<Result<_, ExpansionStop>>()?;
        let extents = state
            .extents
            .iter()
            .map(|extent| {
                let mut extent = extent.clone();
                for instance in &mut extent.instances {
                    if let Some(renamed) = renaming.reference(&instance.source) {
                        instance.source = renamed;
                    }
                    if let Some(renamed) = renaming.reference(&instance.target) {
                        instance.target = renamed;
                    }
                }
                extent
            })
            .collect();
        Ok(ModelState {
            populations,
            extents,
        })
    }

    /// ADR-021 SYM-1: `permutation` applied to `transition`: its receiver
    /// and each argument that is a reference, or an option of one, renamed.
    /// An argument that holds a reference inside another value is left as it
    /// is; a caller that needs the image to be a transition of the subject
    /// checks it against an expansion.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when a renamed
    /// identity does not encode.
    pub fn permute_transition(
        &self,
        transition: &ModelTransition,
        permutation: &Permutation,
    ) -> Result<ModelTransition, ExpansionStop> {
        if permutation.is_identity() {
            return Ok(transition.clone());
        }
        let renaming = self.renaming(permutation);
        let limits = Limits::new(identity_limits().input_bytes);
        let text = |enc: Enc| enc.canonical_text(limits).map_err(|_| stop(BROKEN));
        let rename = |argument: &str| -> Result<String, ExpansionStop> {
            // A `Clock` parameter's place (FR-230 "Operation parameters"):
            // the place of the same clock at the same slots on the image
            // of its object.
            if let Some((before, reference, after)) = encode::split_clock_place(argument) {
                return match renaming.reference(&reference) {
                    Some(renamed) => Ok(format!(
                        "{before}{}{after}",
                        text(encode::reference_enc(&renamed))?
                    )),
                    None => Ok(argument.to_owned()),
                };
            }
            if let Some(reference) = encode::parse_reference(argument) {
                return match renaming.reference(&reference) {
                    Some(renamed) => text(encode::reference_enc(&renamed)),
                    None => Ok(argument.to_owned()),
                };
            }
            match encode::parse_optional_reference(argument) {
                Some(Some(reference)) => match renaming.reference(&reference) {
                    Some(renamed) => text(Enc::tagged(
                        "option",
                        vec![("value", encode::reference_enc(&renamed))],
                    )),
                    None => Ok(argument.to_owned()),
                },
                Some(None) | None => Ok(argument.to_owned()),
            }
        };
        let receiver = transition.receiver().map(rename).transpose()?;
        let arguments = transition
            .arguments()
            .iter()
            .map(|argument| rename(argument))
            .collect::<Result<Vec<_>, _>>()?;
        ModelTransition::new(
            transition.operation().to_owned(),
            receiver,
            arguments,
            limits,
        )
        .map_err(|_| stop(BROKEN))
    }

    /// ADR-021 SYM-7 (FR-152): the sort canonicaliser. For each class of
    /// `group`, each key's sort key is the JCS bytes of the key-free
    /// encoding of the object it names (empty when the state holds none);
    /// the keys sorted ascending by sort key, ties in key order, are mapped
    /// in turn to the class's keys in key order. The canonical form is that
    /// permutation applied to `state`.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when an object's
    /// encoding has no canonical bytes.
    pub fn canonicalise(
        &self,
        state: &ModelSimState,
        group: &SymmetryGroup,
    ) -> Result<(ModelSimState, Permutation), ExpansionStop> {
        let mut permutation = Permutation::identity();
        if !group.is_trivial() {
            let limits = Limits::new(identity_limits().input_bytes);
            let mut placeholders: Vec<(Vec<UniverseId>, &DeclarationKey, &[String], Enc)> =
                Vec::new();
            for class in &group.classes {
                let text = declaration_text(&class.population, limits).map_err(|_| stop(BROKEN))?;
                placeholders.push((
                    self.universes_of(&class.population),
                    &class.population,
                    &class.keys,
                    Enc::Object(vec![
                        ("type", Enc::text("symmetric")),
                        ("population", Enc::text(text)),
                    ]),
                ));
            }
            let placeholder = |reference: &ObjectReference| {
                placeholders
                    .iter()
                    .find(|(universes, _, keys, _)| {
                        universes.contains(reference.universe())
                            && keys.iter().any(|key| key == reference.object().as_str())
                    })
                    .map(|(_, _, _, enc)| enc.clone())
            };
            for class in &group.classes {
                let objects = state.state().populations.get(&class.population);
                let mut sorted = Vec::with_capacity(class.keys.len());
                for key in &class.keys {
                    let bytes = match objects.and_then(|objects| objects.get(key)) {
                        Some(object) => quire_canonical::to_vec(
                            &encode::object_enc(&self.model, object, self.typing(), &placeholder),
                            limits,
                        )
                        .map_err(|_| stop(BROKEN))?,
                        None => Vec::new(),
                    };
                    sorted.push((bytes, key));
                }
                // A stable sort keeps equal sort keys in key order.
                sorted.sort_by(|left, right| left.0.cmp(&right.0));
                permutation.insert(
                    &class.population,
                    sorted
                        .into_iter()
                        .zip(&class.keys)
                        .map(|((_, from), to)| (from.clone(), to.clone())),
                );
            }
        }
        Ok((self.permute(state, &permutation)?, permutation))
    }
}

#[cfg(test)]
mod tests {
    use filament_checked::key::DeclarationKey;
    use filament_exact::{Integer, Value};
    use filament_semantic_value::relationship_extent::{
        RelationshipExtent, RelationshipInstance,
    };
    use ix_trace_rs::trace;
    use quire_canonical::Limits;
    use serde_json::json;

    use crate::simulation::model::{ModelSimState, ModelSystem};
    use crate::simulation::sampled::world::{Model, World};
    use crate::simulation::{Permutation, SymmetryClass, SymmetryGroup, TransitionSystem};

    fn fixture() -> (World, SymmetryGroup) {
        let package = "test/symmetry";
        let model = Model {
            package,
            object: "Counter",
            population: "counters",
            key: "a",
            integers: vec![("Level", 0, 2)],
            fields: vec![("value", format!("ix://{package}/Level"))],
            operations: Vec::new(),
            body: String::new(),
            initial: json!({"value": {"integer": "0"}}),
        };
        let population = DeclarationKey {
            package: package.to_owned(),
            node: format!("ix://{package}/counters"),
        };
        let world = World::new(&model).with_universe_keys(&["a", "b", "c"]);
        let group = SymmetryGroup {
            classes: vec![SymmetryClass {
                population,
                keys: vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
            }],
        };
        (world, group)
    }

    fn distinct_state(system: &ModelSystem<'_>, group: &SymmetryGroup) -> ModelSimState {
        let class = &group.classes[0];
        let mut state = system.initial().into_iter().next().expect("initial state");
        let objects = state
            .state_mut()
            .populations
            .get_mut(&class.population)
            .expect("class population exists");
        let template = objects
            .get(&class.keys[0])
            .expect("initial class object exists")
            .clone();
        let value_key = template
            .fields
            .keys()
            .find(|field| field.node.ends_with("/value"))
            .expect("value field exists")
            .clone();
        for (identity, value) in class.keys.iter().zip([0_i64, 1, 2]) {
            let mut object = template.clone();
            object
                .fields
                .insert(value_key.clone(), Value::Integer(Integer::from(value)));
            objects.insert(identity.clone(), object);
        }
        state
    }

    fn permutations() -> [[usize; 3]; 6] {
        [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ]
    }

    fn permute(
        system: &ModelSystem<'_>,
        state: &ModelSimState,
        group: &SymmetryGroup,
        images: [usize; 3],
    ) -> ModelSimState {
        system
            .permute(
                state,
                &Permutation::of(
                    &group.classes[0].population,
                    group.classes[0]
                        .keys
                        .iter()
                        .zip(images)
                        .map(|(from, to)| (from.clone(), group.classes[0].keys[to].clone())),
                ),
            )
            .expect("permutation applies")
    }

    /// FR-152-AC-2: the evaluator canonicaliser maps permuted states to one
    /// representative of the same symmetry orbit.
    #[trace("TC-569", "FR-152-AC-2")]
    #[test]
    fn canonicalise_is_permutation_invariant() {
        let (world, group) = fixture();
        let system = world.system();
        let state = distinct_state(&system, &group);
        let canonical_key = system
            .canonicalise(&state, &group)
            .expect("canonicalises")
            .0;
        let canonical_key = system.state_key(&canonical_key).expect("canonical key").0;
        for images in permutations() {
            let permuted = permute(&system, &state, &group, images);
            let canonical_permuted = system
                .canonicalise(&permuted, &group)
                .expect("permuted state canonicalises")
                .0;
            let shares_canonical_key = system
                .state_key(&canonical_permuted)
                .expect("permuted canonical key")
                .0
                == canonical_key;
            assert!(
                shares_canonical_key,
                "permutation {images:?} of distinct values must share one representative",
            );
        }
    }

    /// FR-152-AC-4: canonicalising an evaluator state twice preserves its
    /// canonical key.
    #[trace("TC-569", "FR-152-AC-4")]
    #[test]
    fn canonicalise_is_idempotent() {
        let (world, group) = fixture();
        let system = world.system();
        let state = distinct_state(&system, &group);
        for images in permutations() {
            let permuted = permute(&system, &state, &group, images);
            let once = system
                .canonicalise(&permuted, &group)
                .expect("state canonicalises")
                .0;
            let twice = system
                .canonicalise(&once, &group)
                .expect("canonical state canonicalises")
                .0;
            let keys_are_equal = system.state_key(&once).expect("canonical key").0
                == system.state_key(&twice).expect("twice canonical key").0;
            assert!(
                keys_are_equal,
                "canonicalising permutation {images:?} twice preserves its key",
            );
        }
    }

    /// Initial recipe projection applies symmetry renames to relationship
    /// extent endpoints just as it does to object and field references.
    #[trace("TC-569", "FR-152-AC-2")]
    #[test]
    fn initial_projection_renames_relationship_extent_endpoints() {
        let (world, group) = fixture();
        let system = world.system();
        let mut state = system.initial().into_iter().next().expect("initial state");
        let class = &group.classes[0];
        let object_type = state
            .state()
            .populations
            .get(&class.population)
            .and_then(|objects| objects.get("a"))
            .expect("initial class object exists")
            .type_identity
            .clone();
        let relationship = DeclarationKey {
            package: "test/symmetry".to_owned(),
            node: "ix://test/symmetry/links".to_owned(),
        };
        state.state_mut().extents.push(RelationshipExtent {
            relationship,
            complete: true,
            instances: vec![RelationshipInstance {
                source: system
                    .model
                    .object_reference(&object_type, "a")
                    .expect("source reference resolves"),
                target: system
                    .model
                    .object_reference(&object_type, "b")
                    .expect("target reference resolves"),
                positions: Default::default(),
            }],
        });

        let permutation = Permutation::of(
            &class.population,
            [("a".to_owned(), "b".to_owned()), ("b".to_owned(), "a".to_owned())],
        );
        let universes = system.universes_of(&class.population);
        let renaming = permutation
            .maps()
            .values()
            .map(|mapping| (universes.clone(), mapping.clone()))
            .collect();
        let projected = super::super::encode::initial_model_key(
            &system.model,
            state.state(),
            system.typing(),
            &Default::default(),
            &renaming,
        );
        let permuted = system
            .permute_state(state.state(), &permutation)
            .expect("permutation applies");
        let expected = super::super::encode::model_key(&system.model, &permuted, system.typing());

        let limits = Limits::new(u64::MAX);
        assert_eq!(
            quire_canonical::to_vec(&projected.0, limits).expect("projected key encodes"),
            quire_canonical::to_vec(&expected.0, limits).expect("permuted key encodes"),
        );
    }
}
