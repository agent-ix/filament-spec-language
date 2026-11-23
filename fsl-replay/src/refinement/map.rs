// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 (ADR-020 RM-2, RM-3, RM-7): `map(s)`, the abstract state a
//! concrete state maps to.
//!
//! Each abstract population is the key-preserving image of its source
//! population. Each image object has the abstract type whose object row names
//! the concrete object's most-specific type, and each visible field is the
//! value of its row, evaluated by the one clause evaluator over the concrete
//! state's synthesized observation with `self` bound to the concrete object
//! and a fresh meter per evaluation. A reference value maps to the reference
//! with the same key, and a whole-population reference row maps element by
//! element.
//!
//! The rows are read in an order that does not depend on how they were
//! written: populations by abstract population node-key digest, objects by ascending key, and
//! per object its object row, then its field rows by ascending field name as
//! UTF-8 bytes. Every row is read, because an internal fault in any of them
//! outranks every row outcome; of the row outcomes the first undefined one
//! outranks the first undetermined one (FSpec FR-377 "Failure precedence").

use std::collections::BTreeMap;

use super::read::{FieldMapping, ObjectMapping, PopulationMapping, RefinementRead};
use filament_checked::key::DeclarationKey;
use filament_exact::{EffectiveId, ObjectReference, Value, ValueType};
use filament_semantic_value::catalog::CatalogCode;
use fsl_eval::simulation::model::{ClauseOutcome, ModelSystem, RowReading, StateObservation};
use fsl_eval::temporal::UndefinedCause;
use fsl_semantics::model::state::{ModelState, StateField, StateObject};

use crate::refinement::history::HistoryValues;

/// An object of a population, by the population and the object's key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObjectKey {
    /// The object's population.
    pub population: DeclarationKey,
    /// The object's key in that population.
    pub key: String,
}

/// The mapping row a failure is about (FR-140 `MappingRowRef`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MappingRowRef {
    /// A visible field row of an object row.
    Field {
        /// The abstract object type the row is of.
        object_type: DeclarationKey,
        /// The abstract field the row defines.
        field: DeclarationKey,
    },
    /// The object map itself: the object row of a concrete object's
    /// most-specific type, which names the abstract type it maps to.
    Object {
        /// The concrete most-specific type.
        concrete_type: DeclarationKey,
    },
    /// An expression of a `step` row, evaluated over the concrete pre-state.
    Argument {
        /// The concrete operation the row is for.
        operation: DeclarationKey,
        /// The expression.
        argument: StepArgument,
    },
    /// The update of a history field by one concrete operation (FR-138).
    History {
        /// The concrete object type the field is added to.
        object_type: DeclarationKey,
        /// The history field.
        field: String,
        /// The concrete operation whose update it is.
        operation: DeclarationKey,
    },
}

/// Which expression of an abstract-operation `step` row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepArgument {
    /// The receiver.
    Receiver,
    /// The abstract parameter at this zero-based position; a `_` argument
    /// has no expression.
    Parameter(usize),
}

/// An abstract field a refinement leaves open, on one abstract object
/// (ADR-020 AX-2).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HiddenSlot {
    /// The abstract object.
    pub object: ObjectKey,
    /// The abstract field no row defines.
    pub field: DeclarationKey,
}

/// Why a row evaluation gave no value and no refutation (ADR-020 RE-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvalFailure {
    /// The evaluator refused the row or ran out of meter.
    Evaluation(Box<ClauseOutcome>),
    /// No object row names the concrete object's most-specific type.
    UnmappedType(DeclarationKey),
    /// A history update's value lies outside the field's declared type
    /// (FR-138).
    OutOfType,
}

/// A mapping row that is refused or incomplete (FR-140 `MappingUndetermined`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingUndetermined {
    /// The row the mapping could not evaluate.
    pub row: MappingRowRef,
    /// The concrete object it was evaluated for.
    pub object: Option<ObjectKey>,
    /// Why.
    pub cause: EvalFailure,
}

/// Why `map_state` returned no state (FR-140).
#[derive(Clone, Debug)]
pub enum MappingFailure {
    /// A row evaluated undefined: the refinement is refuted at this state
    /// (ADR-020 RE-5).
    Undefined {
        /// The row.
        row: MappingRowRef,
        /// The concrete object it was evaluated for.
        object: Option<ObjectKey>,
        /// The evaluator's undefined cause.
        cause: Box<UndefinedCause>,
    },
    /// A row was refused or incomplete: the mapping is undetermined here
    /// (ADR-020 RE-4).
    Undetermined(MappingUndetermined),
    /// A system that is not the refinement's own was passed:
    /// `invalid_runtime_input`/`wrong-role-mapping`. It is not a verdict;
    /// the caller passed the wrong system, an internal fault of the calling
    /// code (FR-141).
    WrongSystem(CatalogCode),
    /// The evaluator hit an internal fault, with this catalog code. It is
    /// no row's outcome and takes no part in the failure order; the item
    /// settles `failed` (FSpec FR-383).
    Fault(CatalogCode),
}

/// The abstract state a concrete state maps to.
#[derive(Clone, Debug)]
pub struct MappedState {
    /// The abstract state with every hidden field unset.
    pub visible: ModelState,
    /// The abstract fields left open, in (object, field) order.
    pub hidden: Vec<HiddenSlot>,
}

/// `invalid_runtime_input`/`wrong-role-mapping`: a system over another model
/// than the refinement's.
const WRONG_ROLE_MAPPING: CatalogCode =
    CatalogCode::new("invalid_runtime_input", "wrong-role-mapping");

/// `runtime_invariant`/`established-invariant-broken`: the concrete state
/// names an object its own system cannot reference.
pub(super) const BROKEN_INVARIANT: CatalogCode =
    CatalogCode::new("runtime_invariant", "established-invariant-broken");

/// The first row outcome of each kind, in reading order.
#[derive(Default)]
pub(super) struct Outcomes {
    undefined: Option<MappingFailure>,
    undetermined: Option<MappingFailure>,
}

impl Outcomes {
    /// Keep `failure` when no earlier one of its kind was kept.
    pub(super) fn record(&mut self, failure: MappingFailure) {
        let slot = match failure {
            MappingFailure::Undefined { .. } => &mut self.undefined,
            MappingFailure::Undetermined(_) => &mut self.undetermined,
            MappingFailure::WrongSystem(_) | MappingFailure::Fault(_) => return,
        };
        slot.get_or_insert(failure);
    }

    /// The failure that outranks the rest: the first undefined row, else the
    /// first undetermined one.
    pub(super) fn first(self) -> Option<MappingFailure> {
        self.undefined.or(self.undetermined)
    }

    /// Both kept outcomes, the undefined one first.
    pub(super) fn into_failures(self) -> impl Iterator<Item = MappingFailure> {
        self.undefined.into_iter().chain(self.undetermined)
    }
}

/// FR-140: `map(state)`, the abstract state that the concrete `state` of
/// `concrete_system` maps to under `refinement`, built in `abstract_system`'s
/// state model. A function of its inputs only.
///
/// The rows are evaluated by the concrete package's clause evaluator over the
/// concrete state's observation, which only `concrete_system` holds, so it is
/// a parameter beside FR-140's listed inputs. A row reads a history field
/// (FR-138) from `history`, the values at `state`.
///
/// # Errors
///
/// [`MappingFailure::WrongSystem`] before any row is read when a system is
/// not over its model; [`MappingFailure::Fault`] at once for an internal
/// fault; otherwise [`MappingFailure::Undefined`] for the first row that
/// evaluates undefined, else [`MappingFailure::Undetermined`] for the first
/// that is refused or incomplete, in the reading order above. Neither
/// returns a partial state.
#[allow(
    clippy::result_large_err,
    reason = "FR-140 fixes MappingFailure's shape: a row, an object and the evaluator's cause"
)]
pub fn map_state(
    refinement: &dyn RefinementRead,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    state: &ModelState,
    history: &HistoryValues,
) -> Result<MappedState, MappingFailure> {
    check_systems(refinement, concrete_system, abstract_system)?;
    let populations = ordered_populations(refinement.populations().collect());
    let mut visible = ModelState {
        populations: BTreeMap::new(),
        extents: Vec::new(),
    };
    let mut hidden = Vec::new();
    let mut outcomes = Outcomes::default();
    // The concrete state's observation, built once for every row read.
    let mut observed: Option<StateObservation> = None;
    for row in populations {
        let mut image = BTreeMap::new();
        for (key, object) in state
            .populations
            .get(row.concrete_population())
            .into_iter()
            .flatten()
        {
            let at = ObjectKey {
                population: row.concrete_population().clone(),
                key: key.clone(),
            };
            let Some(object_row) = refinement
                .objects()
                .find(|candidate| candidate.concrete_type() == &object.type_identity)
            else {
                outcomes.record(unmapped(&object.type_identity, at));
                continue;
            };
            let observation = match &observed {
                Some(observation) => observation,
                None => observed.insert(
                    concrete_system
                        .observe_with_history(
                            state,
                            history
                                .environment(concrete_system, state)
                                .map_err(MappingFailure::Fault)?,
                        )
                        .map_err(|stop| MappingFailure::Fault(stop.cause))?,
                ),
            };
            if let Some(mapped) = map_object(
                refinement,
                concrete_system,
                abstract_system,
                observation,
                (object_row, &at),
                &mut outcomes,
            )? {
                for field in object_row.hidden() {
                    hidden.push(HiddenSlot {
                        object: ObjectKey {
                            population: row.abstract_population().clone(),
                            key: key.clone(),
                        },
                        field: field.clone(),
                    });
                }
                image.insert(key.clone(), mapped);
            }
        }
        visible
            .populations
            .insert(row.abstract_population().clone(), image);
    }
    if let Some(failure) = outcomes.first() {
        return Err(failure);
    }
    hidden.sort();
    Ok(MappedState { visible, hidden })
}

/// The model-node digest order FR-140 uses for the population rows it reads.
fn ordered_populations(
    mut populations: Vec<&dyn PopulationMapping>,
) -> Vec<&dyn PopulationMapping> {
    populations.sort_by_key(|row| row.order_key().clone());
    populations
}

/// The identities of the two domain packages `refinement` maps between,
/// concrete first, read off the declarations its rows name; `None` for a
/// refinement with no population or object row.
pub(crate) fn subject_models(refinement: &dyn RefinementRead) -> Option<(&str, &str)> {
    refinement.subject_models()
}

/// Check that `concrete_system` is over the refinement's concrete model and
/// `abstract_system` over its abstract one.
#[allow(
    clippy::result_large_err,
    reason = "FR-140 fixes MappingFailure's shape: a row, an object and the evaluator's cause"
)]
fn check_systems(
    refinement: &dyn RefinementRead,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
) -> Result<(), MappingFailure> {
    let over = |system: &ModelSystem<'_>, model: &str| system.model() == Some(model);
    match subject_models(refinement) {
        Some((concrete, abstracted))
            if !(over(concrete_system, concrete) && over(abstract_system, abstracted)) =>
        {
            Err(MappingFailure::WrongSystem(WRONG_ROLE_MAPPING))
        }
        _ => Ok(()),
    }
}

/// The population and key of `reference` in `state`: the object of that key
/// whose most-specific type is the reference's.
pub(super) fn object_of(
    system: &ModelSystem<'_>,
    state: &ModelState,
    reference: &ObjectReference,
) -> Option<ObjectKey> {
    let type_key = system.type_key_of(reference)?;
    state.populations.iter().find_map(|(population, objects)| {
        objects
            .get(reference.object().as_str())
            .filter(|object| object.type_identity == *type_key)
            .map(|_| ObjectKey {
                population: population.clone(),
                key: reference.object().as_str().to_owned(),
            })
    })
}

/// The failure of a concrete object whose type no object row maps. S3
/// requires an object row for each abstract type, not for each concrete
/// subtype a source population may hold, so the case is reported as a row
/// that cannot be evaluated, never as a dropped object.
fn unmapped(concrete_type: &DeclarationKey, at: ObjectKey) -> MappingFailure {
    MappingFailure::Undetermined(MappingUndetermined {
        row: MappingRowRef::Object {
            concrete_type: concrete_type.clone(),
        },
        object: Some(at),
        cause: EvalFailure::UnmappedType(concrete_type.clone()),
    })
}

/// The abstract object that the concrete object at `at` maps to, or `None`
/// once a row of it is undefined or undetermined (recorded in `outcomes`).
///
/// # Errors
///
/// [`MappingFailure::Fault`] at once for an internal fault.
#[allow(
    clippy::result_large_err,
    reason = "FR-140 fixes MappingFailure's shape: a row, an object and the evaluator's cause"
)]
fn map_object(
    refinement: &dyn RefinementRead,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    observed: &StateObservation,
    (object_row, at): (&dyn ObjectMapping, &ObjectKey),
    outcomes: &mut Outcomes,
) -> Result<Option<StateObject>, MappingFailure> {
    let receiver = concrete_system
        .object_reference(object_row.concrete_type(), &at.key)
        .map_err(|_| MappingFailure::Fault(BROKEN_INVARIANT))?;
    let declared = abstract_system
        .fields_of(object_row.abstract_type())
        .ok_or(MappingFailure::Fault(BROKEN_INVARIANT))?;
    let mut rows: Vec<(&dyn FieldMapping, &StateField)> = object_row
        .fields()
        .map(|row| {
            declared
                .iter()
                .find(|field| &field.key == row.field())
                .map(|field| (row, field))
                .ok_or(MappingFailure::Fault(BROKEN_INVARIANT))
        })
        .collect::<Result<_, _>>()?;
    rows.sort_by(|left, right| left.1.name.as_bytes().cmp(right.1.name.as_bytes()));
    let mut fields = BTreeMap::new();
    let mut mapped = true;
    for (row, field) in rows {
        let reading = row
            .expression()
            .read(
                concrete_system,
                observed,
                vec![Value::Reference(receiver.clone())],
            )
            .map_err(|stop| MappingFailure::Fault(stop.cause))?;
        let value = match reading {
            RowReading::Value(value) => value,
            RowReading::Undefined(cause) => {
                outcomes.record(MappingFailure::Undefined {
                    row: field_ref(object_row, row),
                    object: Some(at.clone()),
                    cause: Box::new(cause),
                });
                mapped = false;
                continue;
            }
            RowReading::Undetermined(outcome) => {
                outcomes.record(undetermined(
                    object_row,
                    row,
                    at,
                    EvalFailure::Evaluation(Box::new(outcome)),
                ));
                mapped = false;
                continue;
            }
        };
        match lift_field(refinement, concrete_system, abstract_system, field, value) {
            Ok(Some(lifted)) => {
                fields.insert(row.field().clone(), lifted);
            }
            Ok(None) => {}
            Err(Lifting::Unmapped(concrete_type)) => {
                outcomes.record(undetermined(
                    object_row,
                    row,
                    at,
                    EvalFailure::UnmappedType(concrete_type),
                ));
                mapped = false;
            }
            Err(Lifting::Fault) => return Err(MappingFailure::Fault(BROKEN_INVARIANT)),
        }
    }
    Ok(mapped.then(|| StateObject {
        type_identity: object_row.abstract_type().clone(),
        fields,
    }))
}

fn field_ref(object_row: &dyn ObjectMapping, row: &dyn FieldMapping) -> MappingRowRef {
    MappingRowRef::Field {
        object_type: object_row.abstract_type().clone(),
        field: row.field().clone(),
    }
}

fn undetermined(
    object_row: &dyn ObjectMapping,
    row: &dyn FieldMapping,
    at: &ObjectKey,
    cause: EvalFailure,
) -> MappingFailure {
    MappingFailure::Undetermined(MappingUndetermined {
        row: field_ref(object_row, row),
        object: Some(at.clone()),
        cause,
    })
}

/// Why a row's value could not be lifted into the abstract model.
pub(super) enum Lifting {
    /// A reference names an object whose most-specific type no object row
    /// names.
    Unmapped(DeclarationKey),
    /// The concrete state names an object its own system cannot reference.
    Fault,
}

/// What `field` of the abstract model takes from the row value `value`
/// (RM-3, RM-7): `None` leaves the field unset.
///
/// A field declared `T?` takes `v` from `Some(v)` and stays unset for `none`;
/// a field declared `Option<T>` takes the option value unchanged, so `none`
/// stays an explicit `none`. A reference maps to the abstract reference with
/// the same key, and a reference nested in an option, collection or map, a
/// whole-population reference row (`Set<Reference<T>>`) among them, maps
/// element by element at every depth.
fn lift_field(
    refinement: &dyn RefinementRead,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    field: &StateField,
    value: Value,
) -> Result<Option<Value>, Lifting> {
    let Some(value) = unwrap_optional(field, value) else {
        return Ok(None);
    };
    lift_nested(
        &mut |reference| lift_reference(refinement, concrete_system, abstract_system, reference),
        &field.value_type,
        value,
    )
    .map(Some)
}

/// `value` as the abstract type `declared` reads it: a reference maps to the
/// abstract reference with the same key, and an option, a collection or a
/// map maps its elements, and a map its reference keys, the same way at
/// every depth (FR-140, FR-630). `reference` lifts one reference. A value of
/// a leaf type is unchanged; an option, collection or map value whose
/// declared type has another shape is a fault. A reference nested in a
/// record, tuple or union is not lifted (QSL-2959).
fn lift_nested(
    reference: &mut dyn FnMut(&ObjectReference) -> Result<ObjectReference, Lifting>,
    declared: &ValueType,
    value: Value,
) -> Result<Value, Lifting> {
    match (value, declared) {
        (Value::Reference(found), _) => reference(&found).map(Value::Reference),
        (Value::Option(option), ValueType::Option(payload_type)) => {
            let payload = option
                .payload()
                .map(|payload| lift_nested(reference, payload_type, payload.clone()))
                .transpose()?;
            Ok(filament_exact::OptionValue::from_admitted(
                (**payload_type).clone(),
                payload,
            ))
        }
        (Value::Collection(collection), ValueType::Collection(declared)) => {
            let elements = collection
                .elements()
                .iter()
                .map(|element| lift_nested(reference, declared.element(), element.clone()))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(filament_exact::from_admitted(
                (**declared).clone(),
                elements,
            ))
        }
        (Value::Map(map), ValueType::Map(declared)) => {
            let entries = map
                .entries()
                .iter()
                .map(|(key, value)| {
                    Ok((
                        lift_nested(reference, declared.key(), key.clone())?,
                        lift_nested(reference, declared.values(), value.clone())?,
                    ))
                })
                .collect::<Result<Vec<_>, Lifting>>()?;
            filament_exact::from_admitted_map((**declared).clone(), entries)
                .map_err(|_| Lifting::Fault)
        }
        (Value::Option(_) | Value::Collection(_) | Value::Map(_), _) => Err(Lifting::Fault),
        (plain, _) => Ok(plain),
    }
}

/// What `field` takes from the row value `value` before lifting: a field
/// declared `T?` takes `v` from `Some(v)` and stays unset (`None`) for
/// `none`; any other field takes the value unchanged, so a field declared
/// `Option<T>` keeps an explicit `none`.
fn unwrap_optional(field: &StateField, value: Value) -> Option<Value> {
    match value {
        Value::Option(option) if field.optional => option.payload().cloned(),
        other => Some(other),
    }
}

fn lift_reference(
    refinement: &dyn RefinementRead,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    reference: &ObjectReference,
) -> Result<ObjectReference, Lifting> {
    let concrete_type = concrete_system
        .type_key_of(reference)
        .ok_or(Lifting::Fault)?;
    let row = refinement
        .objects()
        .find(|row| row.concrete_type() == concrete_type)
        .ok_or_else(|| Lifting::Unmapped(concrete_type.clone()))?;
    abstract_system
        .object_reference(row.abstract_type(), reference.object().as_str())
        .map_err(|_| Lifting::Fault)
}

/// `value` as the abstract model reads it: a reference maps to the abstract
/// reference with the same key (RM-3), and so does a reference nested in an
/// option, collection or map; any other value is unchanged. Used for the
/// receiver and arguments of an abstract-operation `step` row, which carry no
/// declared type here, so the abstract type is the value's own type with each
/// reference type replaced by the abstract type of the object row that maps it.
pub(super) fn lift_value(
    refinement: &dyn RefinementRead,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    value: Value,
) -> Result<Value, Lifting> {
    lift_own(
        &mut |found| lift_reference(refinement, concrete_system, abstract_system, found),
        &mut |concrete| {
            let row = refinement
                .objects()
                .find(|row| {
                    concrete_system.type_identity(row.concrete_type()).as_ref() == Some(concrete)
                })
                .ok_or(Lifting::Fault)?;
            abstract_system
                .type_identity(row.abstract_type())
                .ok_or(Lifting::Fault)
        },
        value,
    )
}

/// `value` with each nested reference lifted by `reference`, read as the
/// type its own type becomes when `types` lifts each reference type.
fn lift_own(
    reference: &mut dyn FnMut(&ObjectReference) -> Result<ObjectReference, Lifting>,
    types: &mut dyn FnMut(&EffectiveId) -> Result<EffectiveId, Lifting>,
    value: Value,
) -> Result<Value, Lifting> {
    let Some(own) = type_of(&value) else {
        return Ok(value);
    };
    match lift_type(types, &own) {
        Ok(declared) => lift_nested(reference, &declared, value),
        // A type no row maps is named by the reference that holds it.
        Err(fault) => lift_nested(reference, &own, value).and(Err(fault)),
    }
}

/// The type of an option, collection, map or reference `value`; `None` for
/// any other value.
fn type_of(value: &Value) -> Option<ValueType> {
    match value {
        Value::Reference(reference) => Some(ValueType::Reference(reference.object_type().clone())),
        Value::Option(option) => Some(ValueType::option(option.payload_type().clone())),
        Value::Collection(collection) => {
            Some(ValueType::collection(collection.collection_type().clone()))
        }
        Value::Map(map) => Some(ValueType::Map(std::sync::Arc::new(map.map_type().clone()))),
        _ => None,
    }
}

/// `value_type` with each reference type replaced by `lift`'s result, at
/// every depth of an option, collection or map.
fn lift_type(
    lift: &mut dyn FnMut(&EffectiveId) -> Result<EffectiveId, Lifting>,
    value_type: &ValueType,
) -> Result<ValueType, Lifting> {
    match value_type {
        ValueType::Reference(object) => lift(object).map(ValueType::Reference),
        ValueType::Option(payload) => Ok(ValueType::option(lift_type(lift, payload)?)),
        ValueType::Collection(collection) => {
            Ok(ValueType::collection(filament_exact::CollectionType::new(
                collection.kind(),
                lift_type(lift, collection.element())?,
                collection.bound(),
            )))
        }
        ValueType::Map(map) => Ok(ValueType::Map(std::sync::Arc::new(
            filament_exact::MapType::new(
                lift_type(lift, map.key())?,
                lift_type(lift, map.values())?,
                map.bound(),
            )
            .map_err(|_| Lifting::Fault)?,
        ))),
        other => Ok(other.clone()),
    }
}

#[cfg(test)]
mod tests {
    use fsl_semantics::check::PopulationRow;
    use ix_trace_rs::trace;

    use super::*;

    fn field(optional: bool) -> StateField {
        StateField {
            key: DeclarationKey {
                package: "p".to_owned(),
                node: "head".to_owned(),
            },
            name: "head".to_owned(),
            value_type: ValueType::Boolean,
            optional,
        }
    }

    /// TC-1086 step 8: a field declared `T?` is unset for
    /// `none` and takes `v` from `Some(v)`; a field declared `Option<T>`
    /// keeps the option value unchanged, so `none` stays an explicit `none`.
    /// The constructors `none` and `Some(e)` are not yet a spelling of a
    /// refinement row (S2), so the rule is exercised on the values.
    #[trace("TC-1086")]
    #[test]
    fn an_optional_field_is_unset_for_none_and_takes_the_payload_of_some() {
        let none = filament_exact::OptionValue::none(ValueType::Boolean);
        let some = filament_exact::OptionValue::present(ValueType::Boolean, Value::Boolean(true))
            .expect("a present option");
        assert!(unwrap_optional(&field(true), none.clone()).is_none());
        assert!(matches!(
            unwrap_optional(&field(true), some.clone()),
            Some(Value::Boolean(true))
        ));
        assert!(matches!(
            unwrap_optional(&field(false), none),
            Some(Value::Option(_))
        ));
        assert!(matches!(
            unwrap_optional(&field(false), some),
            Some(Value::Option(_))
        ));
    }

    fn undefined(field: &str) -> MappingFailure {
        MappingFailure::Undefined {
            row: MappingRowRef::Object {
                concrete_type: DeclarationKey {
                    package: "p".to_owned(),
                    node: field.to_owned(),
                },
            },
            object: None,
            cause: Box::new(UndefinedCause::InsufficientData(
                fsl_eval::temporal::InsufficientData {
                    count: 0,
                    min_count: 1,
                },
            )),
        }
    }

    fn undetermined(field: &str) -> MappingFailure {
        unmapped(
            &DeclarationKey {
                package: "p".to_owned(),
                node: field.to_owned(),
            },
            ObjectKey {
                population: DeclarationKey {
                    package: "p".to_owned(),
                    node: "population".to_owned(),
                },
                key: "k".to_owned(),
            },
        )
    }

    /// FR-140-AC-13 (TC-1086 step 4): of the row outcomes read, the first
    /// undefined one is reported even when an undetermined one was read
    /// first, and a later outcome of a kind never replaces an earlier one.
    #[trace("TC-1086", "FR-140-AC-13")]
    #[test]
    fn the_first_undefined_row_outranks_every_undetermined_one() {
        let mut outcomes = Outcomes::default();
        outcomes.record(undetermined("first-undetermined"));
        outcomes.record(undefined("first-undefined"));
        outcomes.record(undefined("second-undefined"));
        outcomes.record(undetermined("second-undetermined"));
        let Some(MappingFailure::Undefined {
            row: MappingRowRef::Object { concrete_type },
            ..
        }) = outcomes.first()
        else {
            panic!("the first undefined row is reported");
        };
        assert_eq!(concrete_type.node, "first-undefined");

        let mut outcomes = Outcomes::default();
        outcomes.record(undetermined("first-undetermined"));
        outcomes.record(undetermined("second-undetermined"));
        let Some(MappingFailure::Undetermined(found)) = outcomes.first() else {
            panic!("the first undetermined row is reported");
        };
        assert_eq!(
            found.cause,
            EvalFailure::UnmappedType(DeclarationKey {
                package: "p".to_owned(),
                node: "first-undetermined".to_owned()
            })
        );
    }

    /// FR-140-AC-15 (TC-545): population row precedence follows the named
    /// abstract population's node-key digest, even when declaration order is
    /// the reverse.
    #[trace("TC-545", "FR-140-AC-15")]
    #[test]
    fn population_rows_sort_by_abstract_node_key_digest() {
        let later_digest = PopulationRow {
            abstract_population: DeclarationKey {
                package: "p".to_owned(),
                node: "a".to_owned(),
            },
            order_key: crate::fixture_node(0xff),
            concrete_population: DeclarationKey {
                package: "c".to_owned(),
                node: "first".to_owned(),
            },
        };
        let earlier_digest = PopulationRow {
            abstract_population: DeclarationKey {
                package: "p".to_owned(),
                node: "z".to_owned(),
            },
            order_key: crate::fixture_node(0x01),
            concrete_population: DeclarationKey {
                package: "c".to_owned(),
                node: "second".to_owned(),
            },
        };
        let ordered = ordered_populations(vec![&later_digest, &earlier_digest]);
        assert_eq!(ordered[0].abstract_population().node, "z");
        assert_eq!(ordered[1].abstract_population().node, "a");
    }

    fn effective(name: &str) -> EffectiveId {
        EffectiveId::new(
            None,
            filament_exact::OriginalDeclarationKey::new("test", name.to_owned()),
        )
    }

    fn reference_to(type_name: &str, key: &str) -> ObjectReference {
        let root = filament_exact::NodeKey::new(
            "test",
            vec![filament_exact::PathSegment::Text("Root".to_owned())],
            0,
        )
        .expect("a valid structural key");
        ObjectReference::new(
            filament_exact::UniverseId::new(root),
            effective(type_name),
            filament_exact::ObjectId::new(key).expect("a key"),
        )
    }

    /// Lifts `Ring` references to `Queue` references with the same key; a
    /// `Sub` reference is unmapped.
    fn ring_to_queue(found: &ObjectReference) -> Result<ObjectReference, Lifting> {
        if *found.object_type() == effective("Ring") {
            Ok(reference_to("Queue", found.object().as_str()))
        } else {
            Err(Lifting::Unmapped(DeclarationKey {
                package: "test".to_owned(),
                node: "Sub".to_owned(),
            }))
        }
    }

    fn bound() -> Option<filament_exact::CardinalityBound> {
        Some(filament_exact::CardinalityBound::new(0, 2).expect("a bound"))
    }

    fn sequence_of(element: ValueType) -> filament_exact::CollectionType {
        filament_exact::CollectionType::new(
            filament_exact::CollectionKind::Sequence,
            element,
            bound(),
        )
    }

    fn option_of(reference: &str, type_name: &str) -> Value {
        filament_exact::OptionValue::from_admitted(
            ValueType::Reference(effective(type_name)),
            Some(Value::Reference(reference_to(type_name, reference))),
        )
    }

    /// FR-140-AC-16 (TC-1086 step 10): a `some` payload is lifted, a `none`
    /// stays `none`, and an unmapped payload is `Unmapped`.
    #[trace("TC-1086", "FR-140-AC-16")]
    #[test]
    fn an_option_payload_is_lifted_and_none_stays_none() {
        let declared = ValueType::option(ValueType::Reference(effective("Queue")));
        let lifted = lift_nested(&mut ring_to_queue, &declared, option_of("a", "Ring"))
            .expect("the payload lifts");
        assert_eq!(lifted, option_of("a", "Queue"));

        let none = filament_exact::OptionValue::none(ValueType::Reference(effective("Ring")));
        let kept = lift_nested(&mut ring_to_queue, &declared, none).expect("none lifts");
        assert_eq!(
            kept,
            filament_exact::OptionValue::from_admitted(
                ValueType::Reference(effective("Queue")),
                None
            )
        );

        let unmapped = lift_nested(&mut ring_to_queue, &declared, option_of("a", "Sub"));
        assert!(matches!(unmapped, Err(Lifting::Unmapped(found)) if found.node == "Sub"));
    }

    /// FR-140-AC-16 (TC-1086 step 10): a map keeps its entry order and the
    /// keys' identities while its values and its reference keys are lifted,
    /// and an unmapped value or key is `Unmapped`.
    #[trace("TC-1086", "FR-140-AC-16")]
    #[test]
    fn a_map_lifts_its_values_and_reference_keys_and_keeps_its_order() {
        let map = |key: &str, value: &str, entries: &[(&str, &str)]| {
            let map_type = filament_exact::MapType::new(
                ValueType::Reference(effective(key)),
                ValueType::Reference(effective(value)),
                bound(),
            )
            .expect("a leaf key");
            let entries = entries
                .iter()
                .map(|(entry_key, entry_value)| {
                    (
                        Value::Reference(reference_to(key, entry_key)),
                        Value::Reference(reference_to(value, entry_value)),
                    )
                })
                .collect();
            filament_exact::from_admitted_map(map_type, entries).expect("a map")
        };
        let declared = ValueType::Map(std::sync::Arc::new(
            filament_exact::MapType::new(
                ValueType::Reference(effective("Queue")),
                ValueType::Reference(effective("Queue")),
                bound(),
            )
            .expect("a leaf key"),
        ));
        let lifted = lift_nested(
            &mut ring_to_queue,
            &declared,
            map("Ring", "Ring", &[("a", "b"), ("c", "a")]),
        )
        .expect("the map lifts");
        assert_eq!(lifted, map("Queue", "Queue", &[("a", "b"), ("c", "a")]));

        let value_unmapped = lift_nested(
            &mut ring_to_queue,
            &declared,
            map("Ring", "Sub", &[("a", "b")]),
        );
        assert!(matches!(value_unmapped, Err(Lifting::Unmapped(_))));
        let key_unmapped = lift_nested(
            &mut ring_to_queue,
            &declared,
            map("Sub", "Ring", &[("a", "b")]),
        );
        assert!(matches!(key_unmapped, Err(Lifting::Unmapped(_))));
    }

    /// FR-135-AC-6 "at every depth" (TC-1086 step 10): a `Sequence<Option<
    /// Reference<Ring>>>` lifts through both carriers, in order.
    #[trace("TC-1086", "FR-140-AC-16")]
    #[test]
    fn a_reference_two_carriers_deep_is_lifted_in_order() {
        let sequence = |type_name: &str| {
            filament_exact::from_admitted(
                sequence_of(ValueType::option(ValueType::Reference(effective(
                    type_name,
                )))),
                vec![
                    option_of("b", type_name),
                    filament_exact::OptionValue::none(ValueType::Reference(effective(type_name))),
                    option_of("a", type_name),
                ],
            )
        };
        let declared = ValueType::collection(sequence_of(ValueType::option(ValueType::Reference(
            effective("Queue"),
        ))));
        let lifted = lift_nested(&mut ring_to_queue, &declared, sequence("Ring"))
            .expect("the sequence lifts");
        assert_eq!(lifted, sequence("Queue"));
    }

    /// FR-141 (TC-1086 step 10): the argument of an abstract-operation row
    /// carries no declared type, so a `Sequence<Option<Reference<Ring>>>` is
    /// read as the `Sequence<Option<Reference<Queue>>>` its own type lifts to,
    /// each element lifted and the type replaced; a leaf value is unchanged.
    #[trace("TC-1086", "FR-140-AC-16")]
    #[test]
    fn an_argument_without_a_declared_type_is_lifted_by_its_own_type() {
        let sequence = |type_name: &str| {
            filament_exact::from_admitted(
                sequence_of(ValueType::option(ValueType::Reference(effective(
                    type_name,
                )))),
                vec![option_of("b", type_name), option_of("a", type_name)],
            )
        };
        let mut types = |concrete: &EffectiveId| {
            if *concrete == effective("Ring") {
                Ok(effective("Queue"))
            } else {
                Err(Lifting::Fault)
            }
        };
        let lifted =
            lift_own(&mut ring_to_queue, &mut types, sequence("Ring")).expect("the argument lifts");
        assert_eq!(lifted, sequence("Queue"));
        let kept = lift_own(&mut ring_to_queue, &mut types, Value::Boolean(true));
        assert!(matches!(kept, Ok(Value::Boolean(true))));
        let unmapped = lift_own(&mut ring_to_queue, &mut types, sequence("Sub"));
        assert!(matches!(unmapped, Err(Lifting::Unmapped(_))));
    }

    /// FR-140: a value whose shape is not the declared type's is a fault, as
    /// is an option value declared as a leaf type; a leaf value is unchanged.
    #[trace("TC-1086", "FR-140-AC-16")]
    #[test]
    fn a_value_of_another_shape_than_its_declared_type_is_a_fault() {
        let lifted = lift_nested(
            &mut ring_to_queue,
            &ValueType::Boolean,
            option_of("a", "Ring"),
        );
        assert!(matches!(lifted, Err(Lifting::Fault)));
        let kept = lift_nested(
            &mut ring_to_queue,
            &ValueType::Boolean,
            Value::Boolean(true),
        );
        assert!(matches!(kept, Ok(Value::Boolean(true))));
    }
}
