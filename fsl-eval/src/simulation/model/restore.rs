// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed inverse of the model owner's canonical state identity.

use super::{ModelKeyBudget, ModelKeyError, ModelSimState, ModelStateProvenance, ModelSystem};
use filament_checked::key::DeclarationKey;
use filament_exact::{
    ClockPlaceReference, CollectionType, FieldValue, Integer, MapType, Meter, NodeKey, Outcome,
    Presence, Rational, UnionMember, UnionValue, Value, ValueType,
};
use filament_semantic_value::declaration::CompositeShape;
use filament_semantic_value::relationship_extent::{RelationshipInstance, RelationshipPositions};
use fsl_foundation::IdentityLimits;
use fsl_semantics::model::state::{ModelState, StateObject};
use quire_canonical::{Node, NodeRef};
use std::collections::{BTreeMap, BTreeSet};

/// Key construction and scalar admission share the actual caller's accounting scope.
pub trait ModelRestoreBudget: ModelKeyBudget {
    /// The caller's existing meter; restoration never creates a fresh budget.
    fn meter(&mut self) -> &mut Meter;
}

/// Restoration preserves malformed input, actual caller stops and key refusals.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ModelRestoreError {
    /// The key fails structural, declaration, type, closure or universe validation.
    #[error("malformed model state key: {0}")]
    Malformed(&'static str),
    /// A key operation stopped under caller admission.
    #[error(transparent)]
    Key(#[from] ModelKeyError),
    /// A scalar value admission exhausted the actual caller meter.
    #[error("model restoration scalar admission stopped: {0:?}")]
    Accounting(Box<filament_exact::Incomplete>),
}
fn read_error(error: quire_canonical::ReadError, identity: IdentityLimits) -> ModelRestoreError {
    match error {
        quire_canonical::ReadError::ReservationDenied { .. } => ModelKeyError::Denied.into(),
        quire_canonical::ReadError::Limit(limit) => {
            ModelKeyError::Encoding(crate::simulation::EncodingRefusal::Limit(
                fsl_foundation::diagnostic::LimitExceeded::new(
                    fsl_foundation::Setting::IdentityInputBytes,
                    identity.input_bytes,
                    u128::from(limit.required),
                ),
            ))
            .into()
        }
        _ => bad(),
    }
}
fn bad() -> ModelRestoreError {
    ModelRestoreError::Malformed("state")
}
fn member<'a>(node: NodeRef<'a>, name: &str) -> Result<NodeRef<'a>, ModelRestoreError> {
    let Node::Object(members) = node.node() else {
        return Err(bad());
    };
    members
        .into_iter()
        .find_map(|(key, value)| (key == name).then_some(value))
        .ok_or_else(bad)
}
fn optional_member<'a>(
    node: NodeRef<'a>,
    name: &str,
) -> Result<Option<NodeRef<'a>>, ModelRestoreError> {
    let Node::Object(members) = node.node() else {
        return Err(bad());
    };
    Ok(members
        .into_iter()
        .find_map(|(key, value)| (key == name).then_some(value)))
}
fn text(node: NodeRef<'_>) -> Result<&str, ModelRestoreError> {
    match node.node() {
        Node::String(value) => Ok(value),
        _ => Err(bad()),
    }
}
fn tagged(node: NodeRef<'_>, expected: &str) -> Result<(), ModelRestoreError> {
    if text(member(node, "type")?)? == expected {
        Ok(())
    } else {
        Err(bad())
    }
}
fn completed<T>(outcome: Outcome<T>) -> Result<T, ModelRestoreError> {
    match outcome {
        Outcome::Completed(value) => Ok(value),
        Outcome::Incomplete(error) => Err(ModelRestoreError::Accounting(Box::new(error))),
        Outcome::Undefined(_) | Outcome::Refused(_) => Err(bad()),
    }
}
struct Restore<'s, 'm, 'b> {
    system: &'s ModelSystem<'m>,
    identity: IdentityLimits,
    budget: &'b mut dyn ModelRestoreBudget,
    cancel: &'b filament_exact::Cancel,
    source_bytes: usize,
    value_nodes: usize,
}
impl Restore<'_, '_, '_> {
    fn tick(&mut self, units: usize) -> Result<(), ModelRestoreError> {
        self.cancel.poll();
        if let Some(cause) = self.cancel.tripped() {
            return Err(ModelKeyError::Cancelled(cause).into());
        }
        self.budget
            .work(u64::try_from(units).map_err(|_| ModelKeyError::SizeOverflow)?)
            .map_err(|_| ModelKeyError::Denied)?;
        Ok(())
    }
    fn retain(&mut self, bytes: usize) -> Result<(), ModelRestoreError> {
        self.tick(1)?;
        self.budget
            .reserve(bytes)
            .map_err(|_| ModelKeyError::Denied)?;
        Ok(())
    }
    fn push<T>(&mut self, values: &mut Vec<T>, value: T) -> Result<(), ModelRestoreError> {
        if values.len() == values.capacity() {
            let target = values.capacity().saturating_mul(2).max(1);
            let bytes = target
                .checked_mul(size_of::<T>())
                .ok_or(ModelKeyError::SizeOverflow)?;
            self.retain(bytes)?;
            values
                .try_reserve_exact(target - values.len())
                .map_err(|_| {
                    ModelKeyError::Encoding(crate::simulation::EncodingRefusal::Allocation {
                        requested: bytes,
                    })
                })?;
        }
        values.push(value);
        Ok(())
    }
    fn array<'a>(&mut self, node: NodeRef<'a>) -> Result<Vec<NodeRef<'a>>, ModelRestoreError> {
        let Node::Array(items) = node.node() else {
            return Err(bad());
        };
        self.retain(
            items
                .len()
                .checked_mul(size_of::<NodeRef<'_>>())
                .ok_or(ModelKeyError::SizeOverflow)?,
        )?;
        self.tick(items.len())?;
        Ok(items.collect())
    }
    fn map<'a>(
        &mut self,
        node: NodeRef<'a>,
    ) -> Result<Vec<(NodeRef<'a>, NodeRef<'a>)>, ModelRestoreError> {
        tagged(node, "map")?;
        let rows = self.array(member(node, "entries")?)?;
        self.retain(
            rows.len()
                .checked_mul(size_of::<(NodeRef<'_>, NodeRef<'_>)>())
                .ok_or(ModelKeyError::SizeOverflow)?,
        )?;
        rows.into_iter()
            .map(|row| Ok((member(row, "key")?, member(row, "value")?)))
            .collect()
    }
    fn integer(&mut self, node: NodeRef<'_>) -> Result<Integer, ModelRestoreError> {
        let spelling = text(node)?;
        let n = spelling.len();
        self.retain(n.checked_mul(4).ok_or(ModelKeyError::SizeOverflow)?)?;
        self.tick(n.checked_mul(n).ok_or(ModelKeyError::SizeOverflow)?)?;
        let value: Integer = spelling.parse().map_err(|_| bad())?;
        self.budget
            .integer(&value)
            .map_err(|_| ModelKeyError::Denied)?;
        Ok(value)
    }
    fn rational(&mut self, node: NodeRef<'_>) -> Result<Rational, ModelRestoreError> {
        let numerator = Rational::from_integer(self.integer(member(node, "numerator")?)?);
        let denominator = Rational::from_integer(self.integer(member(node, "denominator")?)?);
        completed(filament_exact::evaluate_rational_arithmetic(
            filament_exact::RationalArithmetic::Divide(&numerator, &denominator),
            None,
            self.budget.meter(),
        ))
    }
    fn canonical(&mut self, node: NodeRef<'_>) -> Result<Vec<u8>, ModelRestoreError> {
        super::encode::bounded::node_bytes(node, self.identity, self.budget, self.cancel)
            .map_err(ModelRestoreError::Key)
    }
    fn reference(
        &mut self,
        node: NodeRef<'_>,
    ) -> Result<filament_exact::ObjectReference, ModelRestoreError> {
        let bytes = self.canonical(node)?;
        let spelling = std::str::from_utf8(&bytes).map_err(|_| bad())?;
        super::encode::parse_reference(spelling).ok_or_else(bad)
    }
    fn clock_reference(
        &mut self,
        node: NodeRef<'_>,
    ) -> Result<ClockPlaceReference, ModelRestoreError> {
        tagged(node, "clock_reference")?;
        let place_id = text(member(node, "place_id")?)?;
        self.tick(place_id.len())?;
        self.retain(
            place_id
                .len()
                .checked_mul(4)
                .ok_or(ModelKeyError::SizeOverflow)?,
        )?;
        self.system
            .clock_reference_candidates(None)
            .map_err(|_| bad())?
            .iter()
            .find(|reference| reference.text() == place_id)
            .cloned()
            .ok_or_else(bad)
    }
    fn declaration(&mut self, node: NodeRef<'_>) -> Result<DeclarationKey, ModelRestoreError> {
        let spelling = text(node)?;
        self.tick(spelling.len())?;
        self.retain(
            spelling
                .len()
                .checked_mul(4)
                .ok_or(ModelKeyError::SizeOverflow)?,
        )?;
        let document = super::encode::bounded::with_reservation(
            self.identity,
            self.budget,
            self.cancel,
            |reservation| {
                quire_canonical::read_with_reservation(
                    spelling.as_bytes(),
                    self.identity.input_bytes,
                    reservation,
                )
            },
        )?
        .map_err(|error| read_error(error, self.identity))?;
        Ok(DeclarationKey {
            package: text(member(document.root(), "package")?)?.to_owned(),
            node: text(member(document.root(), "node")?)?.to_owned(),
        })
    }
    fn leaf(&mut self, value: Value, expected: &ValueType) -> Result<Value, ModelRestoreError> {
        self.tick(1)?;
        let admitted = match (&value, expected) {
            (Value::Reference(reference), ValueType::Reference(expected)) => self
                .system
                .types
                .conforms(reference.object_type().clone(), expected.clone()),
            _ => expected.admits(&value),
        };
        if admitted {
            Ok(value)
        } else {
            Err(bad())
        }
    }
    fn value(
        &mut self,
        node: NodeRef<'_>,
        expected: &ValueType,
    ) -> Result<Value, ModelRestoreError> {
        let system = self.system;
        // Queued work borrows admitted type metadata; only completed values
        // retain the owned types their constructors require.
        enum Finish<'ty> {
            Option(&'ty ValueType, bool),
            Collection(&'ty CollectionType, usize),
            Map(&'ty MapType, usize),
            Composite(NodeKey, usize),
            Union(UnionMember, usize),
        }
        enum Task<'node, 'ty> {
            Visit(NodeRef<'node>, &'ty ValueType, bool),
            Finish(Finish<'ty>),
        }
        let mut tasks = Vec::new();
        self.push(&mut tasks, Task::Visit(node, expected, false))?;
        let mut values: Vec<FieldValue> = Vec::new();
        while let Some(task) = tasks.pop() {
            self.value_nodes = self
                .value_nodes
                .checked_add(1)
                .ok_or(ModelKeyError::SizeOverflow)?;
            self.tick(1)?;
            self.retain(size_of::<Task<'_, '_>>().saturating_add(size_of::<FieldValue>()))?;
            match task {
                Task::Finish(finish) => {
                    let count = match &finish {
                        Finish::Option(_, present) => usize::from(*present),
                        Finish::Collection(_, n)
                        | Finish::Map(_, n)
                        | Finish::Composite(_, n)
                        | Finish::Union(_, n) => *n,
                    };
                    let start = values.len().checked_sub(count).ok_or_else(bad)?;
                    self.retain(
                        count
                            .checked_mul(size_of::<FieldValue>())
                            .ok_or(ModelKeyError::SizeOverflow)?,
                    )?;
                    let slots = values.split_off(start);
                    let present = |slots: Vec<FieldValue>| {
                        slots
                            .into_iter()
                            .map(|value| match value {
                                FieldValue::Present(value) => Ok(value),
                                _ => Err(bad()),
                            })
                            .collect::<Result<Vec<_>, _>>()
                    };
                    let value = match finish {
                        Finish::Option(ty, _) => filament_exact::OptionValue::from_admitted(
                            ty.clone(),
                            present(slots)?.into_iter().next(),
                        ),
                        Finish::Collection(ty, _) => completed(filament_exact::form(
                            ty,
                            present(slots)?,
                            self.budget.meter(),
                        ))?,
                        Finish::Map(ty, _) => {
                            let mut items = present(slots)?.into_iter();
                            let mut entries = Vec::new();
                            while let Some(key) = items.next() {
                                self.push(&mut entries, (key, items.next().ok_or_else(bad)?))?;
                            }
                            filament_exact::from_admitted_map(ty.clone(), entries)
                                .map_err(|_| bad())?
                        }
                        Finish::Composite(key, _) => {
                            filament_exact::from_admitted_slots(key, slots.into_boxed_slice())
                        }
                        Finish::Union(member, _) => {
                            UnionValue::from_admitted(member, present(slots)?)
                        }
                    };
                    self.push(&mut values, FieldValue::Present(value))?;
                }
                Task::Visit(node, ty, optional) => {
                    if optional && matches!(node.node(), Node::Null) {
                        self.push(&mut values, FieldValue::Absent)?;
                        continue;
                    }
                    let tag = text(member(node, "type")?)?;
                    for name in [
                        "value",
                        "text",
                        "bits",
                        "coefficient",
                        "scale",
                        "nanoseconds",
                    ] {
                        if let Ok(Node::String(value)) = member(node, name).map(|node| node.node())
                        {
                            self.retain(
                                value
                                    .len()
                                    .checked_mul(4)
                                    .ok_or(ModelKeyError::SizeOverflow)?,
                            )?;
                            self.tick(value.len())?;
                        }
                    }

                    let value = match ty {
                        ValueType::Boolean if tag == "boolean" => {
                            match member(node, "value")?.node() {
                                Node::Bool(value) => Value::Boolean(value),
                                _ => return Err(bad()),
                            }
                        }
                        ValueType::Integer | ValueType::Int(_) if tag == "integer" => {
                            Value::Integer(self.integer(member(node, "value")?)?)
                        }
                        ValueType::Rational(_) if tag == "rational" => {
                            Value::Rational(self.rational(node)?)
                        }
                        ValueType::Quantity(unit) if tag == "rational" => Value::Quantity(
                            filament_exact::Quantity::new(self.rational(node)?, unit.clone()),
                        ),
                        ValueType::BoundedQuantity(declared) if tag == "rational" => {
                            Value::Quantity(filament_exact::Quantity::new(
                                self.rational(node)?,
                                declared.unit().clone(),
                            ))
                        }
                        ValueType::CarriedQuantity(declared) if tag == "rational" => {
                            Value::Quantity(filament_exact::Quantity::new(
                                self.rational(node)?,
                                declared.unit().clone(),
                            ))
                        }
                        ValueType::Decimal(_) if tag == "decimal" => {
                            Value::Decimal(filament_exact::Decimal::new(
                                self.integer(member(node, "coefficient")?)?,
                                text(member(node, "scale")?)?.parse().map_err(|_| bad())?,
                            ))
                        }
                        ValueType::Float(float) => {
                            let bits = u64::from_str_radix(text(member(node, "bits")?)?, 16)
                                .map_err(|_| bad())?;
                            Value::Float(match (float.width(), tag) {
                                (filament_exact::IeeeWidth::Binary32, "float32") => {
                                    filament_exact::IeeeValue::binary32(
                                        u32::try_from(bits).map_err(|_| bad())?,
                                    )
                                }
                                (filament_exact::IeeeWidth::Binary64, "float64") => {
                                    filament_exact::IeeeValue::binary64(bits)
                                }
                                _ => return Err(bad()),
                            })
                        }
                        ValueType::String if tag == "string" => {
                            Value::String(text(member(node, "value")?)?.to_owned())
                        }
                        ValueType::Text(declared) if tag == "text" => {
                            Value::Text(completed(filament_exact::admit_text(
                                &filament_exact::TextPayload::from_utf8(
                                    text(member(node, "value")?)?.as_bytes(),
                                )
                                .map_err(|_| bad())?,
                                declared,
                                self.budget.meter(),
                            ))?)
                        }
                        ValueType::Bytes if tag == "bytes" => {
                            let encoded = text(member(node, "value")?)?;
                            let (pairs, remainder) = encoded.as_bytes().as_chunks::<2>();
                            if !remainder.is_empty() {
                                return Err(bad());
                            }
                            let bytes = pairs
                                .iter()
                                .map(|pair| {
                                    std::str::from_utf8(pair)
                                        .ok()
                                        .and_then(|s| u8::from_str_radix(s, 16).ok())
                                        .ok_or_else(bad)
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            Value::Bytes(bytes)
                        }
                        ValueType::JsonObject if tag == "json_object" => Value::JsonObject(
                            filament_exact::JsonObject::from_json_text(text(member(
                                node, "value",
                            )?)?)
                            .map_err(|_| bad())?,
                        ),
                        ValueType::Uuid if tag == "uuid" => Value::Uuid(
                            filament_exact::Uuid::from_canonical_text(text(member(
                                node, "value",
                            )?)?)
                            .map_err(|_| bad())?,
                        ),
                        ValueType::Timestamp if tag == "timestamp" => Value::Timestamp(
                            filament_exact::Timestamp::from_canonical_text(text(member(
                                node,
                                "nanoseconds",
                            )?)?)
                            .map_err(|_| bad())?,
                        ),
                        ValueType::Reference(_) if tag == "reference" => {
                            Value::Reference(self.reference(node)?)
                        }
                        ValueType::ClockReference if tag == "clock_reference" => {
                            Value::ClockReference(self.clock_reference(node)?)
                        }
                        ValueType::Enum(shape) if tag == "enum" => {
                            let case = text(member(node, "member")?)?;
                            let name = text(member(node, "name")?)?;
                            let (rank, variant) = shape
                                .variants()
                                .enumerate()
                                .find(|(_, variant)| {
                                    variant.case() == case
                                        && super::encode::node_declaration_text(variant.owner())
                                            .as_deref()
                                            == Some(name)
                                })
                                .ok_or_else(bad)?;
                            Value::Enum(filament_exact::EnumMember::new(
                                variant.clone(),
                                u32::try_from(rank).map_err(|_| bad())?,
                            ))
                        }
                        ValueType::Option(payload) if tag == "option" => {
                            let inner = member(node, "value")?;
                            let has_value = !matches!(inner.node(), Node::Null);
                            self.push(
                                &mut tasks,
                                Task::Finish(Finish::Option(payload.as_ref(), has_value)),
                            )?;
                            if has_value {
                                self.push(&mut tasks, Task::Visit(inner, payload.as_ref(), false))?;
                            }
                            continue;
                        }
                        ValueType::Collection(declared) => {
                            let expected_tag = match declared.kind() {
                                filament_exact::CollectionKind::Sequence => "sequence",
                                filament_exact::CollectionKind::Set => "set",
                                filament_exact::CollectionKind::Bag => "bag",
                                filament_exact::CollectionKind::OrderedSet => "ordered-set",
                            };
                            if tag != expected_tag {
                                return Err(bad());
                            }
                            let items = self.array(member(node, "elements")?)?;
                            self.push(
                                &mut tasks,
                                Task::Finish(Finish::Collection(declared.as_ref(), items.len())),
                            )?;
                            for item in items.into_iter().rev() {
                                self.push(
                                    &mut tasks,
                                    Task::Visit(item, declared.element(), false),
                                )?;
                            }
                            continue;
                        }
                        ValueType::Map(declared) if tag == "map" => {
                            let items = self.map(node)?;
                            if declared.bound().is_some_and(|bound| {
                                u64::try_from(items.len())
                                    .ok()
                                    .is_none_or(|count| bound.violation(count).is_some())
                            }) {
                                return Err(bad());
                            }
                            self.push(
                                &mut tasks,
                                Task::Finish(Finish::Map(
                                    declared.as_ref(),
                                    items
                                        .len()
                                        .checked_mul(2)
                                        .ok_or(ModelKeyError::SizeOverflow)?,
                                )),
                            )?;
                            for (key, value) in items.into_iter().rev() {
                                self.push(
                                    &mut tasks,
                                    Task::Visit(value, declared.values(), false),
                                )?;
                                self.push(&mut tasks, Task::Visit(key, declared.key(), false))?;
                            }
                            continue;
                        }
                        ValueType::Composite(expected) => {
                            let named = text(member(node, "name")?)?;
                            let own = system
                                .types
                                .composites()
                                .find(|candidate| {
                                    super::encode::node_declaration_text(&candidate.key())
                                        .as_deref()
                                        == Some(named)
                                })
                                .ok_or_else(bad)?;
                            if !system.types.composite_conforms(own.key(), expected.clone()) {
                                return Err(bad());
                            }
                            match (tag, own.shape()) {
                                ("record", CompositeShape::Record(fields)) => {
                                    let rows = self.array(member(node, "fields")?)?;
                                    if rows.len() != fields.len() {
                                        return Err(bad());
                                    }
                                    self.push(
                                        &mut tasks,
                                        Task::Finish(Finish::Composite(own.key(), fields.len())),
                                    )?;
                                    for field in fields.iter().rev() {
                                        self.tick(rows.len())?;
                                        let row = rows
                                            .iter()
                                            .find(|row| {
                                                member(**row, "name").and_then(text).ok()
                                                    == Some(field.name())
                                            })
                                            .ok_or_else(bad)?;
                                        self.push(
                                            &mut tasks,
                                            Task::Visit(
                                                member(*row, "value")?,
                                                field.value_type(),
                                                field.presence() == Presence::Optional,
                                            ),
                                        )?;
                                    }
                                }
                                ("tuple", CompositeShape::Tuple(types)) => {
                                    let items = self.array(member(node, "components")?)?;
                                    if items.len() != types.len() {
                                        return Err(bad());
                                    }
                                    self.push(
                                        &mut tasks,
                                        Task::Finish(Finish::Composite(own.key(), items.len())),
                                    )?;
                                    for (item, ty) in items.into_iter().zip(types).rev() {
                                        self.push(&mut tasks, Task::Visit(item, ty, false))?;
                                    }
                                }
                                ("union", CompositeShape::Union(members)) => {
                                    let name = text(member(node, "member")?)?;
                                    let declared = members
                                        .iter()
                                        .find(|declared| declared.identifier() == name)
                                        .ok_or_else(bad)?;
                                    let items = self.array(member(node, "components")?)?;
                                    if items.len() != declared.positions().len() {
                                        return Err(bad());
                                    }
                                    self.push(
                                        &mut tasks,
                                        Task::Finish(Finish::Union(
                                            declared.member().ok_or_else(bad)?.clone(),
                                            items.len(),
                                        )),
                                    )?;
                                    for (item, ty) in
                                        items.into_iter().zip(declared.positions()).rev()
                                    {
                                        self.push(&mut tasks, Task::Visit(item, ty, false))?;
                                    }
                                }
                                _ => return Err(bad()),
                            }
                            continue;
                        }
                        _ => return Err(bad()),
                    };
                    let value = self.leaf(value, ty)?;
                    self.push(&mut values, FieldValue::Present(value))?;
                }
            }
        }
        match values.pop() {
            Some(FieldValue::Present(value)) if values.is_empty() => Ok(value),
            _ => Err(bad()),
        }
    }
    fn state(&mut self, root: NodeRef<'_>) -> Result<ModelState, ModelRestoreError> {
        let system = self.system;
        let mut field_index = BTreeMap::new();
        for (owner, fields) in &system.fields {
            self.tick(fields.len())?;
            self.retain(
                fields
                    .len()
                    .checked_mul(4 * size_of::<(&DeclarationKey, &str, &super::StateField)>())
                    .ok_or(ModelKeyError::SizeOverflow)?,
            )?;
            for field in fields {
                self.tick(
                    field
                        .name
                        .len()
                        .saturating_add(owner.package.len())
                        .saturating_add(owner.node.len())
                        .saturating_mul(
                            field_index.len().checked_ilog2().unwrap_or(0) as usize + 1,
                        ),
                )?;
                field_index.insert((owner, field.name.as_str()), field);
            }
        }
        let mut universe_index = BTreeSet::new();
        for (population, keys) in &system.universes {
            self.tick(keys.len())?;
            self.retain(
                keys.len()
                    .checked_mul(4 * size_of::<(&DeclarationKey, &str)>())
                    .ok_or(ModelKeyError::SizeOverflow)?,
            )?;
            for key in keys {
                self.tick(
                    key.len()
                        .saturating_add(population.package.len())
                        .saturating_add(population.node.len())
                        .saturating_mul(
                            universe_index.len().checked_ilog2().unwrap_or(0) as usize + 1,
                        ),
                )?;
                universe_index.insert((population, key.as_str()));
            }
        }
        tagged(root, "simulation-state")?;
        for name in [
            "control",
            "queues",
            "roles",
            "observations",
            "bounds",
            "memory",
        ] {
            if !self.map(member(root, name)?)?.is_empty() {
                return Err(bad());
            }
        }
        let mut populations = BTreeMap::new();
        let mut extents = Vec::new();
        for (key, rows) in self.map(member(root, "semantic")?)? {
            tagged(key, "text")?;
            if text(member(key, "value")?)? == "relationships" {
                for (relationship, extent) in self.map(rows)? {
                    tagged(relationship, "text")?;
                    let relationship = self.declaration(member(relationship, "value")?)?;
                    let complete = match member(extent, "complete")?.node() {
                        Node::Bool(value) => value,
                        _ => return Err(bad()),
                    };
                    let mut instances = Vec::new();
                    for instance in self.array(member(extent, "instances")?)? {
                        let source = self.reference(member(instance, "source")?)?;
                        let target = self.reference(member(instance, "target")?)?;
                        let mut positions = RelationshipPositions::default();
                        if let Some(raw_positions) = optional_member(instance, "positions")? {
                            for (name, value) in self.map(raw_positions)? {
                                tagged(name, "text")?;
                                let name = text(member(name, "value")?)?;
                                tagged(value, "text")?;
                                let raw = text(member(value, "value")?)?;
                                let position = raw
                                    .parse::<u64>()
                                    .ok()
                                    .filter(|parsed| parsed.to_string() == raw)
                                    .ok_or_else(bad)?;
                                match name {
                                    "source" => positions.source = Some(position),
                                    "target" => positions.target = Some(position),
                                    _ => return Err(bad()),
                                }
                            }
                        }
                        self.push(
                            &mut instances,
                            RelationshipInstance {
                                source,
                                target,
                                positions,
                            },
                        )?;
                    }
                    self.push(
                        &mut extents,
                        filament_semantic_value::relationship_extent::RelationshipExtent {
                            relationship,
                            complete,
                            instances,
                        },
                    )?;
                }
                continue;
            }
            let population = self.declaration(member(key, "value")?)?;
            self.tick(system.populations.len())?;
            let declared = system
                .populations
                .iter()
                .find(|declared| declared.key == population)
                .ok_or_else(bad)?;
            if populations.contains_key(&population) {
                return Err(bad());
            }
            let mut objects = BTreeMap::new();
            for (reference, record) in self.map(rows)? {
                let reference = self.reference(reference)?;
                let object_type = system
                    .model
                    .type_key(reference.object_type().clone())
                    .ok_or_else(bad)?
                    .clone();
                let expected_reference = system
                    .model
                    .object_reference(&object_type, reference.object().as_str())
                    .map_err(|_| bad())?;
                if reference != expected_reference || system.model.is_abstract(&object_type) {
                    return Err(bad());
                }
                self.tick(declared.member_types.len())?;
                if !declared.member_types.iter().any(|member| {
                    system.model.type_identity(member).is_some_and(|expected| {
                        system
                            .types
                            .conforms(reference.object_type().clone(), expected)
                    })
                }) {
                    return Err(bad());
                }
                self.tick(
                    reference
                        .object()
                        .as_str()
                        .len()
                        .saturating_add(population.package.len())
                        .saturating_add(population.node.len())
                        .saturating_mul(
                            universe_index.len().checked_ilog2().unwrap_or(0) as usize + 1,
                        ),
                )?;
                if system.universes.contains_key(&population)
                    && !universe_index.contains(&(&population, reference.object().as_str()))
                {
                    return Err(bad());
                }
                tagged(record, "record")?;
                let rows = self.array(member(record, "fields")?)?;
                let fields = system.fields.get(&object_type).ok_or_else(bad)?;
                if rows.len() != fields.len() {
                    return Err(bad());
                }
                self.retain(
                    rows.len()
                        .checked_mul(4 * size_of::<&str>())
                        .ok_or(ModelKeyError::SizeOverflow)?,
                )?;
                let mut values = BTreeMap::new();
                let mut names = BTreeSet::new();
                for row in rows {
                    let name = text(member(row, "name")?)?;
                    self.tick(
                        name.len()
                            .saturating_add(object_type.package.len())
                            .saturating_add(object_type.node.len())
                            .saturating_mul(
                                field_index.len().checked_ilog2().unwrap_or(0) as usize + 1,
                            ),
                    )?;
                    if !names.insert(name) {
                        return Err(bad());
                    }
                    let declared = field_index.get(&(&object_type, name)).ok_or_else(bad)?;
                    let raw = member(row, "value")?;
                    if matches!(raw.node(), Node::Null) && declared.optional {
                        continue;
                    }
                    self.retain(
                        size_of::<(DeclarationKey, Value)>()
                            .saturating_mul(4)
                            .saturating_add(
                                declared
                                    .key
                                    .package
                                    .len()
                                    .saturating_add(declared.key.node.len())
                                    .saturating_mul(2),
                            ),
                    )?;
                    values.insert(declared.key.clone(), self.value(raw, &declared.value_type)?);
                }
                self.retain(
                    size_of::<StateObject>().saturating_add(reference.object().as_str().len()),
                )?;
                if objects
                    .insert(
                        reference.object().as_str().to_owned(),
                        StateObject {
                            type_identity: object_type,
                            fields: values,
                        },
                    )
                    .is_some()
                {
                    return Err(bad());
                }
            }
            populations.insert(population, objects);
        }
        if system
            .universes
            .keys()
            .any(|population| !populations.contains_key(population))
        {
            return Err(bad());
        }
        let state = ModelState {
            populations,
            extents,
        };
        // Architecture-authorized atomic interval: all work/storage is admitted
        // before the existing owner clones slots and validates closure/extents.
        // Input bytes cover nested payload/name sizes; declaration units include
        // records not represented in the key. The cross-product also bounds the
        // owner's name/type/reference searches and map comparison traversals.
        // The existing owner remains authoritative. Pre-admit its atomic clone,
        // checked-slot, reference-closure and extent-validation interval; cancellation
        // is polled on both sides, never replaced by a fabricated observation.
        let (declarations, declaration_bytes) = system
            .model
            .observation_work_size(|units| self.tick(units))?;
        let units = self
            .source_bytes
            .checked_add(declaration_bytes)
            .ok_or(ModelKeyError::SizeOverflow)?;
        self.tick(state.populations.len().saturating_add(state.extents.len()))?;
        let objects = state
            .populations
            .values()
            .try_fold(0usize, |n, rows| n.checked_add(rows.len()))
            .ok_or(ModelKeyError::SizeOverflow)?;
        let extent_rows = state
            .extents
            .iter()
            .try_fold(0usize, |n, extent| {
                n.checked_add(extent.instances.len())
                    .and_then(|n| n.checked_add(1))
            })
            .ok_or(ModelKeyError::SizeOverflow)?;
        let rows = self
            .value_nodes
            .checked_add(objects)
            .and_then(|n| n.checked_add(extent_rows))
            .and_then(|n| n.checked_add(field_index.len()))
            .and_then(|n| n.checked_add(universe_index.len()))
            .and_then(|n| n.checked_add(declarations))
            .and_then(|n| n.checked_add(1))
            .ok_or(ModelKeyError::SizeOverflow)?;
        let work = units
            .checked_mul(
                self.value_nodes
                    .checked_add(rows.ilog2() as usize)
                    .and_then(|n| n.checked_add(1))
                    .ok_or(ModelKeyError::SizeOverflow)?,
            )
            .and_then(|n| {
                rows.checked_mul(rows)
                    .and_then(|pairs| n.checked_add(pairs))
            })
            .ok_or(ModelKeyError::SizeOverflow)?;
        self.tick(work)?;
        self.retain(
            units
                .checked_mul(size_of::<Value>().max(size_of::<FieldValue>()))
                .ok_or(ModelKeyError::SizeOverflow)?,
        )?;
        let observation = system.model.observation(&state, super::state_document());
        self.tick(0)?;
        observation.map_err(|_| bad())?;
        Ok(state)
    }
}
impl ModelSystem<'_> {
    /// Restore a typed state from this owner's canonical key, with explicit
    /// absent provenance. This does not establish reachability or initial evidence.
    ///
    /// # Errors
    /// Malformed/foreign state content or the caller's original bounded stop.
    pub fn restore_state_key_with_budget(
        &self,
        bytes: &[u8],
        identity: IdentityLimits,
        budget: &mut dyn ModelRestoreBudget,
        cancel: &filament_exact::Cancel,
    ) -> Result<ModelSimState, ModelRestoreError> {
        cancel.poll();
        if let Some(cause) = cancel.tripped() {
            return Err(ModelKeyError::Cancelled(cause).into());
        }
        budget
            .work(u64::try_from(bytes.len()).map_err(|_| ModelKeyError::SizeOverflow)?)
            .map_err(|_| ModelKeyError::Denied)?;
        let document =
            super::encode::bounded::with_reservation(identity, budget, cancel, |reservation| {
                quire_canonical::read_with_reservation(bytes, identity.input_bytes, reservation)
            })?
            .map_err(|error| read_error(error, identity))?;
        let mut restore = Restore {
            system: self,
            identity,
            budget,
            cancel,
            source_bytes: bytes.len(),
            value_nodes: 0,
        };
        let state = restore.state(document.root())?;
        let state = ModelSimState {
            state,
            provenance: ModelStateProvenance::RestoredFromKey { effect: None },
        };
        let actual = self.state_key_bytes_with_budget(&state, identity, restore.budget, cancel)?;
        restore.tick(bytes.len())?;
        if actual != bytes {
            return Err(bad());
        }
        Ok(state)
    }
}
