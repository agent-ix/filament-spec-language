// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retain population-admission input from a checked snapshot's actual fields.

use std::cmp::Ordering;

use filament_exact::{FieldValue, Value};
use filament_semantic_value::declaration::{FieldRef, TypeEnvironment};
use fsl_foundation::diagnostic::Code;

use super::{
    admission_record, allocation_failed, fault, refuse, AdmissionFailure, AdmissionRecordExt,
};
use crate::model::domain_package::DomainPackageRecord;
use crate::model::normalize::EffectiveView;
use crate::model::object_environment::ObjectEnvironment;
use crate::model::population::{MemberFieldValues, PopulationDocument, PopulationMember};

/// Read an already admitted snapshot's population into the existing binding
/// admission input. `member_order` is the original snapshot order; every
/// identity must resolve in that exact admitted population. Values come only
/// from the checked object closure, including inherited and redefined slots.
///
/// This performs no new binding admission or accounting reset. The caller
/// passes the result to `population::admit_binding` with its existing meter.
/// Retention uses fallible reservations; the admission schedule has no extra
/// charge for copying already admitted field facts.
///
/// # Errors
///
/// An invariant fault if the population, member order, selected view or field
/// correspondence does not match the admitted snapshot; allocation refusal if
/// retained storage cannot be reserved.
pub fn admitted_population_document(
    view: &EffectiveView,
    types: &TypeEnvironment,
    objects: &ObjectEnvironment,
    population: &str,
    member_order: &[&str],
) -> Result<PopulationDocument, AdmissionFailure> {
    let provenance = objects
        .population_node(population)
        .and_then(|node| objects.population_provenance(node))
        .ok_or_else(|| fault("population-provenance-unresolved"))?;
    if view.population(provenance.declaration()).is_none() {
        return Err(fault("population-view-mismatch"));
    }
    let admitted = objects
        .population_members(population)
        .ok_or_else(|| fault("population-members-unresolved"))?;
    if member_order.len() != admitted.len() {
        return Err(fault("population-member-order-mismatch"));
    }
    let mut members = Vec::new();
    members.try_reserve(member_order.len()).map_err(|_| {
        allocation_failed(
            member_order
                .len()
                .saturating_mul(std::mem::size_of::<PopulationMember>()),
        )
    })?;
    for identity in member_order {
        if members
            .iter()
            .any(|member: &PopulationMember| member.object == *identity)
        {
            return Err(fault("population-member-order-duplicate"));
        }
        let reference = admitted
            .iter()
            .find(|reference| reference.object().as_str() == *identity)
            .ok_or_else(|| fault("population-member-unresolved"))?;
        let type_identity = view
            .type_identities()
            .iter()
            .find(|(_, effective)| *effective == reference.object_type())
            .map(|(key, _)| key.clone())
            .ok_or_else(|| fault("population-type-unresolved"))?;
        let attributes = types
            .attributes(reference.object_type().clone())
            .ok_or_else(|| fault("population-attributes-unresolved"))?;
        let held = objects
            .objects()
            .object(reference)
            .ok_or_else(|| fault("population-object-unresolved"))?;
        let mut field_values = Vec::new();
        // Iterate original field declarations, retaining both sides of every
        // subset relation. A storage slot can stand for several inherited or
        // redefined declarations; each keeps its own original field key.
        for record in &view.domain_package().records {
            let DomainPackageRecord::FieldMember(field) = record else {
                continue;
            };
            let Some(owner) = view.type_identities().get(&field.owner) else {
                return Err(fault("population-field-owner-unresolved"));
            };
            let prefix = format!("{}/", field.owner.node);
            let name = field
                .key
                .node
                .strip_prefix(&prefix)
                .ok_or_else(|| fault("population-field-key-unresolved"))?;
            let original = FieldRef::new(owner.clone(), name);
            let Some(attribute) = attributes
                .iter()
                .find(|attribute| attribute.stands_for(&original))
            else {
                continue;
            };
            // Resolve the visible declaration through the same native key
            // conversion snapshot admission uses, rather than inventing an id.
            let actual_owner = view
                .type_identities()
                .iter()
                .find(|(_, effective)| **effective == attribute.owner())
                .map(|(key, _)| key)
                .ok_or_else(|| fault("population-slot-owner-unresolved"))?;
            let actual_key = super::initial::field_key_of(actual_owner, attribute);
            if !view.domain_package().records.iter().any(|record| {
                matches!(record,
                DomainPackageRecord::FieldMember(actual) if actual.key == actual_key)
            }) {
                return Err(fault("population-slot-key-unresolved"));
            }
            let slot = held
                .get(attribute.field().name())
                .ok_or_else(|| fault("population-slot-unresolved"))?;
            let values = references(slot)?;
            field_values
                .try_reserve(1)
                .map_err(|_| allocation_failed(std::mem::size_of::<MemberFieldValues>()))?;
            field_values.push(MemberFieldValues {
                field: field.key.clone(),
                values,
            });
        }
        members.push(PopulationMember {
            object: copy(identity)?,
            type_identity,
            field_values,
        });
    }
    Ok(PopulationDocument {
        model_identity: copy(&view.model_selection().identity)?,
        members,
    })
}

fn copy(value: &str) -> Result<String, AdmissionFailure> {
    let mut retained = String::new();
    retained
        .try_reserve(value.len())
        .map_err(|_| allocation_failed(value.len()))?;
    retained.push_str(value);
    Ok(retained)
}

fn references(slot: &FieldValue) -> Result<Vec<String>, AdmissionFailure> {
    let mut values = Vec::new();
    let mut pending = Vec::new();
    if let FieldValue::Present(value) = slot {
        pending
            .try_reserve(1)
            .map_err(|_| allocation_failed(std::mem::size_of::<&Value>()))?;
        pending.push(value);
    }
    while let Some(value) = pending.pop() {
        match value {
            Value::Reference(reference) => {
                values
                    .try_reserve(1)
                    .map_err(|_| allocation_failed(std::mem::size_of::<String>()))?;
                values.push(copy(reference.object().as_str())?);
            }
            Value::Option(option) => {
                if let Some(payload) = option.payload() {
                    pending
                        .try_reserve(1)
                        .map_err(|_| allocation_failed(std::mem::size_of::<&Value>()))?;
                    pending.push(payload);
                }
            }
            Value::Collection(collection) => {
                pending
                    .try_reserve(collection.elements().len())
                    .map_err(|_| {
                        allocation_failed(
                            collection
                                .elements()
                                .len()
                                .saturating_mul(std::mem::size_of::<&Value>()),
                        )
                    })?;
                pending.extend(collection.elements().iter().rev());
            }
            Value::Map(map) => {
                let mut ordered = Vec::new();
                ordered.try_reserve(map.entries().len()).map_err(|_| {
                    allocation_failed(
                        map.entries()
                            .len()
                            .saturating_mul(std::mem::size_of::<&(Value, Value)>()),
                    )
                })?;
                ordered.extend(map.entries().iter());
                let mut incomparable = false;
                ordered.sort_unstable_by(|(left, _), (right, _)| {
                    match map_key_order(left, right) {
                        Some(ordering) => ordering,
                        None => {
                            incomparable = true;
                            Ordering::Equal
                        }
                    }
                });
                if incomparable {
                    return Err(unsupported_map());
                }
                pending.try_reserve(ordered.len()).map_err(|_| {
                    allocation_failed(ordered.len().saturating_mul(std::mem::size_of::<&Value>()))
                })?;
                // FR-106's closure walk uses the canonical key order from
                // FSpec FR-144.
                for (_, value) in ordered.into_iter().rev() {
                    pending.push(value);
                }
            }
            Value::Composite(composite) => {
                pending.try_reserve(composite.slots().len()).map_err(|_| {
                    allocation_failed(
                        composite
                            .slots()
                            .len()
                            .saturating_mul(std::mem::size_of::<&Value>()),
                    )
                })?;
                for field in composite.slots().iter().rev() {
                    if let FieldValue::Present(value) = field {
                        pending.push(value);
                    }
                }
            }
            Value::Union(union) => {
                pending.try_reserve(union.payload().len()).map_err(|_| {
                    allocation_failed(
                        union
                            .payload()
                            .len()
                            .saturating_mul(std::mem::size_of::<&Value>()),
                    )
                })?;
                pending.extend(union.payload().iter().rev());
            }
            Value::Boolean(_)
            | Value::Integer(_)
            | Value::Rational(_)
            | Value::Decimal(_)
            | Value::Float(_)
            | Value::Quantity(_)
            | Value::Text(_)
            | Value::String(_)
            | Value::Uuid(_)
            | Value::Timestamp(_)
            | Value::Enum(_)
            | Value::Population(_) => {}
        }
    }
    Ok(values)
}

fn map_key_order(left: &Value, right: &Value) -> Option<Ordering> {
    filament_exact::compare_keys(left, right)
}

fn unsupported_map() -> AdmissionFailure {
    refuse(admission_record(Code::UnsupportedConstruct, "not-yet-implemented").with("what", "map"))
}

#[cfg(test)]
mod tests {
    use filament_exact::{
        from_admitted_map, Decimal, DecimalType, EffectiveId, Integer, MapType, NodeKey, ObjectId,
        ObjectReference, OriginalDeclarationKey, PathSegment, RoundingMode, UniverseId, ValueType,
    };
    use ix_trace_rs::trace;

    use super::references;
    use filament_exact::{FieldValue, Value};

    fn reference(name: &str, universe: &UniverseId, object_type: &EffectiveId) -> Value {
        Value::Reference(ObjectReference::new(
            universe.clone(),
            object_type.clone(),
            ObjectId::new(name).expect("nonempty identity"),
        ))
    }

    /// FR-106-AC-8: nested map value references follow FSpec FR-144's
    /// mathematical Int and Decimal key order (9 before 10).
    #[test]
    #[trace("FR-106-AC-8")]
    fn map_references_follow_canonical_key_order() {
        let node = NodeKey::new("fixture", vec![PathSegment::Text("Root".to_owned())], 0)
            .expect("valid node key");
        let universe = UniverseId::new(node);
        let object_type = EffectiveId::new(None, OriginalDeclarationKey::new("fixture", "Type"));
        let integer_map_type = MapType::new(
            ValueType::Integer,
            ValueType::Reference(object_type.clone()),
            None,
        )
        .expect("integer map keys are leaves");
        let integer_map = from_admitted_map(
            integer_map_type,
            vec![
                (
                    Value::Integer(filament_exact::Integer::from(10_i64)),
                    reference("value-10", &universe, &object_type),
                ),
                (
                    Value::Integer(filament_exact::Integer::from(9_i64)),
                    reference("value-9", &universe, &object_type),
                ),
            ],
        )
        .expect("distinct integer keys admit");
        let integer_references =
            references(&FieldValue::Present(integer_map)).expect("walk integer map references");
        assert_eq!(integer_references, ["value-9", "value-10"]);

        let decimal_map_type = MapType::new(
            ValueType::Decimal(
                DecimalType::new(
                    Integer::zero(),
                    Integer::from(20_i64),
                    0,
                    0,
                    RoundingMode::Exact,
                )
                .expect("decimal map key type is well formed"),
            ),
            ValueType::Reference(object_type.clone()),
            None,
        )
        .expect("decimal map keys are leaves");
        let decimal_map = from_admitted_map(
            decimal_map_type,
            vec![
                (
                    Value::Decimal(Decimal::new(Integer::from(10_i64), 0)),
                    reference("value-10", &universe, &object_type),
                ),
                (
                    Value::Decimal(Decimal::new(Integer::from(9_i64), 0)),
                    reference("value-9", &universe, &object_type),
                ),
            ],
        )
        .expect("distinct decimal keys admit");
        let decimal_references =
            references(&FieldValue::Present(decimal_map)).expect("walk decimal map references");
        assert_eq!(decimal_references, ["value-9", "value-10"]);
    }
}
