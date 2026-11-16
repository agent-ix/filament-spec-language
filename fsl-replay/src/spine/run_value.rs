// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-650: the run value form. One reader turns a JSON argument into the
//! `filament_exact::Value` of its parameter's declared type, and one writer
//! turns a completed value into the same form, so a completed result can be
//! passed back as an argument.
//!
//! The forms are FR-106's closed value grammar with FR-650's tuple and union
//! payload forms. A type is named by the declared name of a declaration of the
//! unit's own source.

use std::collections::BTreeSet;

use filament_exact::{
    CollectionKind, CollectionType, Decimal, EnumMember, FieldValue, IeeeValue, IeeeWidth, Integer,
    Meter, NodeKey, OptionValue, Presence, Rational, TextPayload, Value, ValueType,
};
use filament_semantic_value::declaration::{CompositeShape, TypeEnvironment};
use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::EnumBinding;
use serde_json::{Map, Value as Json, json};

/// JSON levels the writer descends before it refuses: the reader's own
/// recursion limit, so every value the writer produces reads back.
const MAXIMUM_DEPTH: usize = 100;

/// One argument defect: the catalog code and cause FR-650's table names, and
/// the JSON pointer of the defect inside the argument's `value`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArgumentDefect {
    /// The refusal's catalog code.
    pub code: Code,
    /// The refusal's cause.
    pub cause: &'static str,
    /// The JSON pointer inside the argument's `value`; empty for the value
    /// itself.
    pub path: String,
}

/// Why a read stopped.
#[derive(Debug)]
pub(crate) enum ReadStop {
    /// The argument is not a value of the parameter's declared type.
    Defect(ArgumentDefect),
    /// The declared type has a form this reader does not carry.
    Unsupported,
    /// An established invariant of the checked package is broken.
    Fault(&'static str),
}

/// Why a write stopped.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum WriteStop {
    /// The value has no run value form, or nests deeper than the form's
    /// bound.
    NoForm,
    /// An established invariant of the checked package is broken.
    Fault(&'static str),
}

/// The declarations of one checked package a value form names.
#[derive(Clone, Copy)]
pub(crate) struct Forms<'a> {
    types: &'a TypeEnvironment,
    enums: &'a [EnumBinding],
}

impl<'a> Forms<'a> {
    pub(crate) fn new(types: &'a TypeEnvironment, enums: &'a [EnumBinding]) -> Self {
        Self { types, enums }
    }

    /// Whether a value of `value_type` has a run value form.
    pub(crate) fn has_form(&self, value_type: &ValueType) -> bool {
        self.has_form_visiting(value_type, &mut BTreeSet::new())
    }

    fn has_form_visiting(&self, value_type: &ValueType, seen: &mut BTreeSet<NodeKey>) -> bool {
        match value_type {
            ValueType::Boolean
            | ValueType::Integer
            | ValueType::Int(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Text(_)
            | ValueType::String
            | ValueType::Uuid
            | ValueType::Timestamp
            | ValueType::Enum(_) => true,
            ValueType::Option(payload) => self.has_form_visiting(payload, seen),
            ValueType::Collection(collection) => self.has_form_visiting(collection.element(), seen),
            ValueType::Composite(key) => {
                if !seen.insert(key.clone()) {
                    return true;
                }
                let Some(declaration) = self.types.composite(key.clone()) else {
                    return false;
                };
                match declaration.shape() {
                    CompositeShape::Record(fields) => fields
                        .iter()
                        .all(|field| self.has_form_visiting(field.value_type(), seen)),
                    CompositeShape::Tuple(positions) => positions
                        .iter()
                        .all(|position| self.has_form_visiting(position, seen)),
                    CompositeShape::Union(members) => members.iter().all(|member| {
                        member
                            .positions()
                            .iter()
                            .all(|position| self.has_form_visiting(position, seen))
                    }),
                }
            }
            ValueType::Quantity(_)
            | ValueType::BoundedQuantity(_)
            | ValueType::CarriedQuantity(_)
            | ValueType::Map(_)
            | ValueType::Reference(_)
            | ValueType::Population(_) => false,
        }
    }

    /// The Boolean-or-integer argument a bare JSON integer binds.
    pub(crate) fn bare_integer(
        &self,
        number: i64,
        value_type: &ValueType,
    ) -> Result<Value, ReadStop> {
        match value_type {
            ValueType::Boolean => match number {
                0 => Ok(Value::Boolean(false)),
                1 => Ok(Value::Boolean(true)),
                _ => Err(defect(
                    Code::InvalidRuntimeInput,
                    "invalid-value",
                    &Path::new(),
                )),
            },
            ValueType::Integer | ValueType::Int(_) => self.leaf(
                Value::Integer(Integer::from(number)),
                value_type,
                &Path::new(),
            ),
            _ => Err(defect(
                Code::InvalidRuntimeInput,
                "wrong-value-kind",
                &Path::new(),
            )),
        }
    }

    /// The value of `value_type` the object `form` spells.
    pub(crate) fn read(&self, form: &Json, value_type: &ValueType) -> Result<Value, ReadStop> {
        self.read_at(form, value_type, &mut Path::new())
    }

    fn read_at(
        &self,
        form: &Json,
        value_type: &ValueType,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let Some(object) = form.as_object() else {
            return Err(defect(Code::InvalidRuntimeInput, "wrong-value-kind", path));
        };
        match value_type {
            ValueType::Boolean => {
                let payload = tagged(object, "boolean", path)?;
                match payload.as_bool() {
                    Some(held) => Ok(Value::Boolean(held)),
                    None => Err(wrong_kind_at(path, "boolean")),
                }
            }
            ValueType::Integer | ValueType::Int(_) => {
                let payload = tagged(object, "integer", path)?;
                let integer = integer_text(payload, path, "integer")?;
                self.leaf(Value::Integer(integer), value_type, path)
            }
            ValueType::Rational(_) => {
                native(object, "rational", &["numerator", "denominator"], path)?;
                let numerator = integer_text(&object["numerator"], path, "numerator")?;
                let denominator = integer_text(&object["denominator"], path, "denominator")?;
                let rational = Rational::new(numerator, denominator)
                    .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", path))?;
                self.leaf(Value::Rational(rational), value_type, path)
            }
            ValueType::Decimal(_) => {
                native(object, "decimal", &["coefficient", "scale"], path)?;
                let coefficient = integer_text(&object["coefficient"], path, "coefficient")?;
                let scale = integer_text(&object["scale"], path, "scale")?
                    .to_u64()
                    .and_then(|scale| u32::try_from(scale).ok())
                    .ok_or_else(|| defect(Code::InvalidRuntimeInput, "invalid-value", path))?;
                self.leaf(
                    Value::Decimal(Decimal::new(coefficient, scale)),
                    value_type,
                    path,
                )
            }
            ValueType::Float(float) => {
                let (kind, digits) = match float.width() {
                    IeeeWidth::Binary32 => ("float32", 8),
                    IeeeWidth::Binary64 => ("float64", 16),
                };
                native(object, kind, &["bits"], path)?;
                let text = string_member(object, "bits", path)?;
                let invalid = || defect(Code::InvalidRuntimeInput, "invalid-value", path);
                if text.len() != digits
                    || !text
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(invalid());
                }
                let bits = u64::from_str_radix(text, 16).map_err(|_| invalid())?;
                let value = match float.width() {
                    IeeeWidth::Binary32 => {
                        IeeeValue::binary32(u32::try_from(bits).map_err(|_| invalid())?)
                    }
                    IeeeWidth::Binary64 => IeeeValue::binary64(bits),
                };
                self.leaf(Value::Float(value), value_type, path)
            }
            ValueType::String => {
                native(object, "string", &["text"], path)?;
                Ok(Value::String(
                    string_member(object, "text", path)?.to_owned(),
                ))
            }
            ValueType::Text(declared) => {
                native(object, "text", &["text"], path)?;
                let text = string_member(object, "text", path)?;
                let mut meter = Meter::new(unbounded());
                filament_exact::admit_text(
                    &TextPayload::from_utf8(text.as_bytes())
                        .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", path))?,
                    declared,
                    &mut meter,
                )
                .completed()
                .map(Value::Text)
                .ok_or_else(|| defect(Code::InvalidRuntimeInput, "invalid-value", path))
            }
            ValueType::Option(payload) => self.read_option(object, payload, path),
            ValueType::Collection(collection) => self.read_collection(object, collection, path),
            ValueType::Enum(shape) => self.read_enum(object, shape, path),
            ValueType::Composite(key) => self.read_composite(object, key, path),
            ValueType::Uuid => {
                native(object, "uuid", &["text"], path)?;
                filament_exact::Uuid::from_canonical_text(string_member(object, "text", path)?)
                    .map(Value::Uuid)
                    .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", path))
            }
            ValueType::Timestamp => {
                native(object, "timestamp", &["nanoseconds"], path)?;
                filament_exact::Timestamp::from_canonical_text(string_member(
                    object,
                    "nanoseconds",
                    path,
                )?)
                .map(Value::Timestamp)
                .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", path))
            }
            ValueType::Quantity(_)
            | ValueType::BoundedQuantity(_)
            | ValueType::CarriedQuantity(_)
            | ValueType::Map(_)
            | ValueType::Reference(_)
            | ValueType::Population(_) => Err(ReadStop::Unsupported),
        }
    }

    /// A scalar the kernel's membership accepts, else `invalid-value`.
    fn leaf(&self, value: Value, value_type: &ValueType, path: &Path) -> Result<Value, ReadStop> {
        match self.types.admits(value_type, &value) {
            Ok(true) => Ok(value),
            Ok(false) | Err(_) => Err(defect(Code::InvalidRuntimeInput, "invalid-value", path)),
        }
    }

    fn read_option(
        &self,
        object: &Map<String, Json>,
        payload: &ValueType,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        if object.contains_key("absent") {
            let absent = tagged(object, "absent", path)?;
            expect_empty_object(absent, path, "absent")?;
            return Ok(OptionValue::none(payload.clone()));
        }
        let present = tagged(object, "present", path)?;
        let value = self.read_member(present, payload, path, "present")?;
        Ok(OptionValue::from_admitted(payload.clone(), Some(value)))
    }

    fn read_member(
        &self,
        form: &Json,
        value_type: &ValueType,
        path: &mut Path,
        segment: &str,
    ) -> Result<Value, ReadStop> {
        path.push(segment);
        let read = self.read_at(form, value_type, path);
        path.pop();
        read
    }

    fn read_collection(
        &self,
        object: &Map<String, Json>,
        collection: &CollectionType,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let tag = tag_of(object).ok_or_else(|| wrong_kind(path))?;
        if crate::spine::run_value::collection_kind(tag) != Some(collection.kind()) {
            return Err(wrong_kind(path));
        }
        let tag = collection_tag(collection.kind());
        let payload = tagged(object, tag, path)?;
        let Some(items) = payload.as_array() else {
            return Err(wrong_kind_at(path, tag));
        };
        if let Some(bound) = collection.bound() {
            if let Some(violation) = bound.violation(filament_exact::length_amount(items.len())) {
                return Err(defect(
                    Code::CardinalityOutOfBound,
                    violation.as_str(),
                    path,
                ));
            }
        }
        path.push(tag);
        let mut elements = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            elements.push(self.read_member(
                item,
                collection.element(),
                path,
                &index.to_string(),
            )?);
        }
        path.pop();
        let kind = collection.kind();
        if kind.is_unique() {
            if let Some(later) = first_duplicate(&elements) {
                path.push(tag);
                path.push(&later.to_string());
                let found = defect(Code::InvalidRuntimeInput, "duplicate-member", path);
                path.pop();
                path.pop();
                return Err(found);
            }
        }
        match kind {
            CollectionKind::Sequence | CollectionKind::OrderedSet => {
                Ok(filament_exact::from_admitted(collection.clone(), elements))
            }
            CollectionKind::Set | CollectionKind::Bag => {
                let mut meter = Meter::new(unbounded());
                filament_exact::form_grouped(collection, elements, &mut meter)
                    .completed()
                    .ok_or_else(|| defect(Code::InvalidRuntimeInput, "invalid-value", path))
            }
        }
    }

    fn read_enum(
        &self,
        object: &Map<String, Json>,
        shape: &filament_exact::EnumShape,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let payload = variant_payload(object, path)?;
        let binding = self
            .enums
            .iter()
            .find(|binding| binding.shape() == *shape)
            .ok_or(ReadStop::Fault("enum-shape-has-a-binding"))?;
        let named = string_member_of(payload, "type", path, "variant")?;
        if named != binding.name {
            return Err(wrong_kind_at2(path, "variant", "type"));
        }
        let case = string_member_of(payload, "case", path, "variant")?;
        let position = binding
            .members
            .iter()
            .position(|member| member.case() == case)
            .ok_or_else(|| invalid_at2(path, "variant", "case"))?;
        if payload.len() != 2 {
            return Err(unknown_member_at(
                path,
                "variant",
                payload,
                &["type", "case"],
            ));
        }
        let member = &binding.members[position];
        let rank = u32::try_from(position).map_err(|_| ReadStop::Fault("enum-rank"))?;
        Ok(Value::Enum(EnumMember::new(member.variant(), rank)))
    }

    fn read_composite(
        &self,
        object: &Map<String, Json>,
        declared: &NodeKey,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let tag = tag_of(object).ok_or_else(|| wrong_kind(path))?;
        let declaration = self
            .types
            .composite(declared.clone())
            .ok_or(ReadStop::Fault("composite-declaration"))?;
        let wanted = match declaration.shape() {
            CompositeShape::Record(_) => "record",
            CompositeShape::Tuple(_) => "tuple",
            CompositeShape::Union(_) => "variant",
        };
        // A descendant of a record or tuple type conforms; the form names it.
        let tagged_payload = tagged(object, tag, path).map_err(|_| wrong_kind(path))?;
        if tag != wanted {
            return Err(wrong_kind(path));
        }
        let Some(payload) = tagged_payload.as_object() else {
            return Err(wrong_kind_at(path, tag));
        };
        match declaration.shape() {
            CompositeShape::Record(_) => self.read_record(payload, declared, path),
            CompositeShape::Tuple(_) => self.read_tuple(payload, declared, path),
            CompositeShape::Union(_) => self.read_union(payload, declared, path),
        }
    }

    /// The declaration the payload's `type` names, which must conform to the
    /// declared type.
    fn named_declaration(
        &self,
        payload: &Map<String, Json>,
        declared: &NodeKey,
        path: &mut Path,
        tag: &'static str,
    ) -> Result<NodeKey, ReadStop> {
        let named = string_member_of(payload, "type", path, tag)?;
        let own = self
            .types
            .composites()
            .find(|candidate| candidate.name() == named)
            .map(filament_semantic_value::declaration::CompositeDeclaration::key)
            .filter(|own| self.types.composite_conforms(own.clone(), declared.clone()))
            .ok_or_else(|| wrong_kind_at2(path, tag, "type"))?;
        Ok(own)
    }

    fn read_record(
        &self,
        payload: &Map<String, Json>,
        declared: &NodeKey,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let own = self.named_declaration(payload, declared, path, "record")?;
        let Some(CompositeShape::Record(fields)) = self
            .types
            .composite(own.clone())
            .map(filament_semantic_value::declaration::CompositeDeclaration::shape)
        else {
            return Err(wrong_kind_at2(path, "record", "type"));
        };
        let fields_form = payload
            .get("fields")
            .ok_or_else(|| missing_member_at(path, "record"))?;
        if payload.len() != 2 {
            return Err(unknown_member_at(
                path,
                "record",
                payload,
                &["type", "fields"],
            ));
        }
        let Some(members) = fields_form.as_object() else {
            return Err(wrong_kind_at2(path, "record", "fields"));
        };
        path.push("record");
        path.push("fields");
        let result = (|| {
            if fields
                .iter()
                .any(|field| !members.contains_key(field.name()))
            {
                return Err(defect(Code::InvalidRuntimeInput, "missing-member", path));
            }
            if let Some(extra) = members
                .keys()
                .find(|name| !fields.iter().any(|field| field.name() == name.as_str()))
            {
                path.push(extra);
                let found = defect(Code::InvalidRuntimeInput, "unknown-member", path);
                path.pop();
                return Err(found);
            }
            let mut slots = Vec::with_capacity(fields.len());
            for field in fields {
                let form = &members[field.name()];
                let slot = match field.presence() {
                    Presence::Required => FieldValue::Present(self.read_member(
                        form,
                        field.value_type(),
                        path,
                        field.name(),
                    )?),
                    Presence::Optional => {
                        path.push(field.name());
                        let slot = self.read_optional_field(form, field.value_type(), path);
                        path.pop();
                        slot?
                    }
                };
                slots.push((field.name(), slot));
            }
            self.types
                .record(own.clone(), slots)
                .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", path))
        })();
        path.pop();
        path.pop();
        result
    }

    fn read_optional_field(
        &self,
        form: &Json,
        value_type: &ValueType,
        path: &mut Path,
    ) -> Result<FieldValue, ReadStop> {
        let Some(object) = form.as_object() else {
            return Err(wrong_kind(path));
        };
        if object.contains_key("absent") {
            let absent = tagged(object, "absent", path)?;
            expect_empty_object(absent, path, "absent")?;
            return Ok(FieldValue::Absent);
        }
        let present = tagged(object, "present", path)?;
        Ok(FieldValue::Present(
            self.read_member(present, value_type, path, "present")?,
        ))
    }

    fn read_tuple(
        &self,
        payload: &Map<String, Json>,
        declared: &NodeKey,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let own = self.named_declaration(payload, declared, path, "tuple")?;
        let Some(CompositeShape::Tuple(positions)) = self
            .types
            .composite(own.clone())
            .map(filament_semantic_value::declaration::CompositeDeclaration::shape)
        else {
            return Err(wrong_kind_at2(path, "tuple", "type"));
        };
        let components = payload
            .get("components")
            .ok_or_else(|| missing_member_at(path, "tuple"))?;
        if payload.len() != 2 {
            return Err(unknown_member_at(
                path,
                "tuple",
                payload,
                &["type", "components"],
            ));
        }
        let Some(items) = components.as_array() else {
            return Err(wrong_kind_at2(path, "tuple", "components"));
        };
        path.push("tuple");
        path.push("components");
        let result = (|| {
            if items.len() != positions.len() {
                return Err(defect(Code::InvalidRuntimeInput, "invalid-value", path));
            }
            let mut values = Vec::with_capacity(items.len());
            for (index, (item, position)) in items.iter().zip(positions).enumerate() {
                values.push(self.read_member(item, position, path, &index.to_string())?);
            }
            self.types
                .tuple(own.clone(), values)
                .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", path))
        })();
        path.pop();
        path.pop();
        result
    }

    fn read_union(
        &self,
        payload: &Map<String, Json>,
        declared: &NodeKey,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let own = self.named_declaration(payload, declared, path, "variant")?;
        let case = string_member_of(payload, "case", path, "variant")?;
        let (_, member) = self
            .types
            .union_member_named(own.clone(), case)
            .ok_or_else(|| invalid_at2(path, "variant", "case"))?;
        let positions = member.positions();
        let Some(binding) = member.member() else {
            return Err(ReadStop::Fault("union-member-is-sealed"));
        };
        let variant = binding.variant().clone();
        let components = payload.get("components");
        let extra = payload
            .keys()
            .find(|name| !["type", "case", "components"].contains(&name.as_str()));
        if let Some(extra) = extra {
            path.push("variant");
            path.push(extra);
            let found = defect(Code::InvalidRuntimeInput, "unknown-member", path);
            path.pop();
            path.pop();
            return Err(found);
        }
        let items: &[Json] = match (components, positions.is_empty()) {
            (None, true) => &[],
            (None, false) => return Err(missing_member_at(path, "variant")),
            (Some(_), true) => {
                path.push("variant");
                path.push("components");
                let found = defect(Code::InvalidRuntimeInput, "unknown-member", path);
                path.pop();
                path.pop();
                return Err(found);
            }
            (Some(components), false) => components
                .as_array()
                .map(Vec::as_slice)
                .ok_or_else(|| wrong_kind_at2(path, "variant", "components"))?,
        };
        path.push("variant");
        path.push("components");
        let result = (|| {
            if items.len() != positions.len() {
                return Err(defect(Code::InvalidRuntimeInput, "invalid-value", path));
            }
            let mut values = Vec::with_capacity(items.len());
            for (index, (item, position)) in items.iter().zip(positions).enumerate() {
                values.push(self.read_member(item, position, path, &index.to_string())?);
            }
            self.types
                .union(own.clone(), variant.clone(), values)
                .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", path))
        })();
        path.pop();
        path.pop();
        result
    }

    /// The run value form of `value`.
    pub(crate) fn write(&self, value: &Value) -> Result<Json, WriteStop> {
        self.write_at(value, 0)
    }

    fn write_at(&self, value: &Value, depth: usize) -> Result<Json, WriteStop> {
        if depth > MAXIMUM_DEPTH {
            return Err(WriteStop::NoForm);
        }
        let below = depth + 1;
        Ok(match value {
            Value::Boolean(held) => json!({"boolean": held}),
            Value::Integer(held) => json!({"integer": held.to_string()}),
            Value::Rational(held) => json!({
                "kind": "rational",
                "numerator": held.numerator().to_string(),
                "denominator": held.denominator().to_string(),
            }),
            Value::Decimal(held) => json!({
                "kind": "decimal",
                "coefficient": held.representation().coefficient().to_string(),
                "scale": held.representation().scale().to_string(),
            }),
            Value::Float(held) => {
                let (kind, digits) = match held.width() {
                    IeeeWidth::Binary32 => ("float32", 8),
                    IeeeWidth::Binary64 => ("float64", 16),
                };
                json!({"kind": kind, "bits": format!("{:0digits$x}", held.bits())})
            }
            Value::String(held) => json!({"kind": "string", "text": held}),
            Value::Text(held) => json!({"kind": "text", "text": held.retained()}),
            Value::Enum(member) => {
                let (binding, case) = self
                    .enums
                    .iter()
                    .find_map(|binding| {
                        binding
                            .members
                            .iter()
                            .find(|candidate| candidate.variant() == *member.variant())
                            .map(|candidate| (binding, candidate.case()))
                    })
                    .ok_or(WriteStop::Fault("enum-member-has-a-binding"))?;
                json!({"variant": {"type": binding.name, "case": case}})
            }
            Value::Option(option) => match option.payload() {
                None => json!({"absent": {}}),
                Some(payload) => json!({"present": self.write_at(payload, below)?}),
            },
            Value::Collection(collection) => {
                let kind = collection.collection_type().kind();
                let mut items = collection
                    .elements()
                    .iter()
                    .map(|element| self.write_at(element, below))
                    .collect::<Result<Vec<_>, _>>()?;
                if matches!(kind, CollectionKind::Set | CollectionKind::Bag) {
                    let mut keyed = items
                        .into_iter()
                        .map(|item| (order_key(&item), item))
                        .collect::<Vec<_>>();
                    keyed.sort_by(|left, right| left.0.cmp(&right.0));
                    items = keyed.into_iter().map(|(_, item)| item).collect();
                }
                let mut object = Map::new();
                object.insert(collection_tag(kind).to_owned(), Json::Array(items));
                Json::Object(object)
            }
            Value::Composite(composite) => {
                let declaration = self
                    .types
                    .composite(composite.declaration().clone())
                    .ok_or(WriteStop::Fault("composite-declaration"))?;
                match declaration.shape() {
                    CompositeShape::Record(fields) => {
                        if fields.len() != composite.slots().len() {
                            return Err(WriteStop::Fault("record-slots-match-fields"));
                        }
                        let mut members = Map::new();
                        for (field, slot) in fields.iter().zip(composite.slots()) {
                            members.insert(
                                field.name().to_owned(),
                                self.write_slot(slot, field.presence(), below)?,
                            );
                        }
                        json!({"record": {"type": declaration.name(), "fields": members}})
                    }
                    CompositeShape::Tuple(_) => {
                        let components = composite
                            .slots()
                            .iter()
                            .map(|slot| self.write_slot(slot, Presence::Required, below))
                            .collect::<Result<Vec<_>, _>>()?;
                        json!({"tuple": {"type": declaration.name(), "components": components}})
                    }
                    CompositeShape::Union(_) => {
                        return Err(WriteStop::Fault("union-value-is-not-a-composite"));
                    }
                }
            }
            Value::Union(union) => {
                let key = union.declaration().clone();
                let declaration = self
                    .types
                    .composite(key.clone())
                    .or_else(|| {
                        self.types
                            .union_handle(key)
                            .and_then(|handle| self.types.composite(handle))
                    })
                    .ok_or(WriteStop::Fault("union-declaration"))?;
                let components = union
                    .payload()
                    .iter()
                    .map(|component| self.write_at(component, below))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut payload = Map::new();
                payload.insert("type".to_owned(), json!(declaration.name()));
                payload.insert("case".to_owned(), json!(union.member().identifier()));
                if !components.is_empty() {
                    payload.insert("components".to_owned(), Json::Array(components));
                }
                json!({"variant": payload})
            }
            Value::Uuid(held) => json!({"kind": "uuid", "text": held.to_string()}),
            Value::Timestamp(held) => {
                json!({"kind": "timestamp", "nanoseconds": held.nanoseconds().to_string()})
            }
            Value::Quantity(_) | Value::Population(_) | Value::Reference(_) => {
                return Err(WriteStop::NoForm);
            }
        })
    }

    fn write_slot(
        &self,
        slot: &FieldValue,
        presence: Presence,
        depth: usize,
    ) -> Result<Json, WriteStop> {
        match (slot, presence) {
            (FieldValue::Present(held), Presence::Required) => self.write_at(held, depth),
            (FieldValue::Present(held), Presence::Optional) => {
                Ok(json!({"present": self.write_at(held, depth)?}))
            }
            (FieldValue::Absent, _) => Ok(json!({"absent": {}})),
            (FieldValue::Null, _) => Err(WriteStop::NoForm),
        }
    }
}

/// The RFC 8785 bytes a set or bag orders its element by (FR-650): the
/// element's form with every Decimal normalized.
fn order_key(form: &Json) -> Vec<u8> {
    let mut normalized = form.clone();
    normalize_decimals(&mut normalized);
    normalized.to_string().into_bytes()
}

/// Rewrite each Decimal object inside `form` with its trailing zeros
/// stripped and its scale lowered by as many.
fn normalize_decimals(form: &mut Json) {
    match form {
        Json::Object(object) => {
            let is_decimal = object.get("kind") == Some(&json!("decimal"));
            if is_decimal {
                let parts = match (object.get("coefficient"), object.get("scale")) {
                    (Some(Json::String(coefficient)), Some(Json::String(scale))) => scale
                        .parse::<u32>()
                        .ok()
                        .map(|scale| (coefficient.clone(), scale)),
                    _ => None,
                };
                if let Some((mut coefficient, mut scale)) = parts {
                    if coefficient == "0" {
                        scale = 0;
                    }
                    while scale > 0 && coefficient.len() > 1 && coefficient.ends_with('0') {
                        coefficient.pop();
                        scale -= 1;
                    }
                    object.insert("coefficient".to_owned(), Json::String(coefficient));
                    object.insert("scale".to_owned(), Json::String(scale.to_string()));
                }
            }
            for member in object.values_mut() {
                normalize_decimals(member);
            }
        }
        Json::Array(items) => items.iter_mut().for_each(normalize_decimals),
        Json::Null | Json::Bool(_) | Json::Number(_) | Json::String(_) => {}
    }
}

/// `ScalarLimits` with every counter at its maximum: an argument's
/// admission is bounded by its size, which the request's byte limits bound.
fn unbounded() -> filament_exact::ScalarLimits {
    crate::spine::default_accounting(u64::MAX)
}

/// The collection kind the form tag `tag` names.
pub(crate) fn collection_kind(tag: &str) -> Option<CollectionKind> {
    fsl_semantics::model::observation::collection_kind_of_tag(tag)
}

fn collection_tag(kind: CollectionKind) -> &'static str {
    fsl_semantics::model::observation::collection_tag(kind)
}

/// The first element, in array order, that equals an earlier one by the
/// canonical key FR-149 `=` agrees with. `None` when the elements are
/// distinct.
fn first_duplicate(elements: &[Value]) -> Option<usize> {
    let mut seen: Vec<&Value> = Vec::with_capacity(elements.len());
    for (index, element) in elements.iter().enumerate() {
        let repeated = seen.iter().any(|earlier| {
            filament_exact::compare_keys(earlier, element) == Some(std::cmp::Ordering::Equal)
        });
        if repeated {
            return Some(index);
        }
        seen.push(element);
    }
    None
}

/// The JSON pointer of the position being read, as segments.
pub(crate) struct Path(Vec<String>);

impl Path {
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }

    fn push(&mut self, segment: &str) {
        self.0.push(segment.to_owned());
    }

    fn pop(&mut self) {
        self.0.pop();
    }

    fn pointer(&self) -> String {
        self.0
            .iter()
            .map(|segment| format!("/{}", segment.replace('~', "~0").replace('/', "~1")))
            .collect()
    }

    fn pointer_with(&self, segments: &[&str]) -> String {
        let mut path = Self(self.0.clone());
        for segment in segments {
            path.push(segment);
        }
        path.pointer()
    }
}

fn defect(code: Code, cause: &'static str, path: &Path) -> ReadStop {
    ReadStop::Defect(ArgumentDefect {
        code,
        cause,
        path: path.pointer(),
    })
}

fn defect_below(code: Code, cause: &'static str, path: &Path, below: &[&str]) -> ReadStop {
    ReadStop::Defect(ArgumentDefect {
        code,
        cause,
        path: path.pointer_with(below),
    })
}

fn wrong_kind(path: &Path) -> ReadStop {
    defect(Code::InvalidRuntimeInput, "wrong-value-kind", path)
}

fn wrong_kind_at(path: &Path, member: &str) -> ReadStop {
    defect_below(
        Code::InvalidRuntimeInput,
        "wrong-value-kind",
        path,
        &[member],
    )
}

fn wrong_kind_at2(path: &Path, outer: &str, member: &str) -> ReadStop {
    defect_below(
        Code::InvalidRuntimeInput,
        "wrong-value-kind",
        path,
        &[outer, member],
    )
}

fn invalid_at2(path: &Path, outer: &str, member: &str) -> ReadStop {
    defect_below(
        Code::InvalidRuntimeInput,
        "invalid-value",
        path,
        &[outer, member],
    )
}

fn missing_member_at(path: &Path, outer: &str) -> ReadStop {
    defect_below(Code::InvalidRuntimeInput, "missing-member", path, &[outer])
}

/// `unknown-member` at the first member of `payload` outside `allowed`.
fn unknown_member_at(
    path: &Path,
    outer: &str,
    payload: &Map<String, Json>,
    allowed: &[&str],
) -> ReadStop {
    match payload
        .keys()
        .find(|name| !allowed.contains(&name.as_str()))
    {
        Some(extra) => defect_below(
            Code::InvalidRuntimeInput,
            "unknown-member",
            path,
            &[outer, extra],
        ),
        None => ReadStop::Fault("payload-has-an-unknown-member"),
    }
}

/// The one tag an object that is no native payload carries.
fn tag_of(object: &Map<String, Json>) -> Option<&str> {
    if object.len() == 1 {
        object.keys().next().map(String::as_str)
    } else {
        None
    }
}

/// The payload of `tag` when `object` is exactly `{tag: payload}`: another
/// single tag is `wrong-value-kind`, an extra member `unknown-member`.
fn tagged<'a>(object: &'a Map<String, Json>, tag: &str, path: &Path) -> Result<&'a Json, ReadStop> {
    let Some(payload) = object.get(tag) else {
        return Err(wrong_kind(path));
    };
    if let Some(extra) = object.keys().find(|name| name.as_str() != tag) {
        return Err(defect_below(
            Code::InvalidRuntimeInput,
            "unknown-member",
            path,
            &[extra],
        ));
    }
    Ok(payload)
}

/// A native payload object `{"kind": kind, <members>}`: a `kind` of another
/// form is `wrong-value-kind`, a missing member `missing-member`, an extra
/// one `unknown-member`.
fn native(
    object: &Map<String, Json>,
    kind: &str,
    members: &[&str],
    path: &Path,
) -> Result<(), ReadStop> {
    match object.get("kind") {
        Some(Json::String(held)) if held == kind => {}
        _ => return Err(wrong_kind(path)),
    }
    if let Some(missing) = members.iter().find(|member| !object.contains_key(**member)) {
        let _ = missing;
        return Err(defect(Code::InvalidRuntimeInput, "missing-member", path));
    }
    if let Some(extra) = object
        .keys()
        .find(|name| name.as_str() != "kind" && !members.contains(&name.as_str()))
    {
        return Err(defect_below(
            Code::InvalidRuntimeInput,
            "unknown-member",
            path,
            &[extra],
        ));
    }
    Ok(())
}

fn string_member<'a>(
    object: &'a Map<String, Json>,
    member: &str,
    path: &Path,
) -> Result<&'a str, ReadStop> {
    match object.get(member) {
        Some(Json::String(text)) => Ok(text),
        _ => Err(wrong_kind_at(path, member)),
    }
}

fn string_member_of<'a>(
    payload: &'a Map<String, Json>,
    member: &str,
    path: &Path,
    outer: &str,
) -> Result<&'a str, ReadStop> {
    match payload.get(member) {
        Some(Json::String(text)) => Ok(text),
        Some(_) => Err(wrong_kind_at2(path, outer, member)),
        None => Err(missing_member_at(path, outer)),
    }
}

/// The payload of a `variant` form: an object.
fn variant_payload<'a>(
    object: &'a Map<String, Json>,
    path: &Path,
) -> Result<&'a Map<String, Json>, ReadStop> {
    let payload = tagged(object, "variant", path)?;
    payload
        .as_object()
        .ok_or_else(|| wrong_kind_at(path, "variant"))
}

fn expect_empty_object(payload: &Json, path: &Path, tag: &str) -> Result<(), ReadStop> {
    match payload.as_object() {
        Some(object) if object.is_empty() => Ok(()),
        Some(object) => match object.keys().next() {
            Some(extra) => Err(defect_below(
                Code::InvalidRuntimeInput,
                "unknown-member",
                path,
                &[tag, extra],
            )),
            None => Ok(()),
        },
        None => Err(wrong_kind_at(path, tag)),
    }
}

/// An integer payload in FR-038's spelling: a JSON string, an optional
/// leading `-`, no `+`, no leading zero, never `-0`. Another JSON kind is
/// `wrong-value-kind`; a noncanonical spelling is `invalid-value`.
fn integer_text(payload: &Json, path: &Path, member: &str) -> Result<Integer, ReadStop> {
    let Json::String(text) = payload else {
        return Err(wrong_kind_at(path, member));
    };
    let invalid = || defect(Code::InvalidRuntimeInput, "invalid-value", path);
    let parsed = text.parse::<Integer>().map_err(|_| invalid())?;
    if parsed.to_string() != *text {
        return Err(invalid());
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn forms() -> Forms<'static> {
        // An empty environment: the types below name no declaration.
        static TYPES: std::sync::OnceLock<TypeEnvironment> = std::sync::OnceLock::new();
        let types = TYPES.get_or_init(|| {
            TypeEnvironment::new(Vec::new(), Vec::new()).expect("the empty environment is valid")
        });
        Forms::new(types, &[])
    }

    /// FR-650-AC-1: Boolean and integer argument forms decode to the
    /// corresponding typed kernel values and write back in the same grammar.
    #[trace("FR-650-AC-1")]
    #[test]
    fn boolean_and_integer_forms_round_trip() {
        let forms = forms();
        for (form, value_type, expected_form) in [
            (
                json!({"boolean": true}),
                ValueType::Boolean,
                json!({"boolean": true}),
            ),
            (
                json!({"integer": "9223372036854775808"}),
                ValueType::Integer,
                json!({"integer": "9223372036854775808"}),
            ),
        ] {
            let value = forms
                .read(&form, &value_type)
                .expect("the closed run value form decodes");
            match (&value_type, &value) {
                (ValueType::Boolean, Value::Boolean(true)) => {}
                (ValueType::Integer, Value::Integer(integer))
                    if integer.to_string() == "9223372036854775808" => {}
                _ => panic!("the reader returned the wrong typed value: {value:?}"),
            }
            assert_eq!(forms.write(&value).unwrap(), expected_form);
        }
    }

    /// FR-650-AC-3: a value whose tag does not match its declared type
    /// returns the typed wrong-value-kind defect at the value root.
    #[trace("FR-650-AC-3")]
    #[test]
    fn mismatched_value_tag_reports_wrong_kind_at_root() {
        let defect = forms()
            .read(
                &json!({"text": {"kind": "text", "text": "x"}}),
                &ValueType::Boolean,
            )
            .expect_err("text is not a Boolean");
        assert!(matches!(
            defect,
            ReadStop::Defect(ArgumentDefect {
                code: Code::InvalidRuntimeInput,
                cause: "wrong-value-kind",
                path,
            }) if path.is_empty()
        ));
    }

    /// FR-650-AC-6: a type with no run value form is refused, never read.
    #[trace("FR-650-AC-6")]
    #[test]
    fn types_with_no_run_value_form_are_not_encodable() {
        let forms = forms();
        let map = ValueType::Map(std::sync::Arc::new(
            filament_exact::MapType::new(ValueType::String, ValueType::Boolean, None)
                .expect("a leaf key"),
        ));
        assert!(!forms.has_form(&map));
        assert!(!forms.has_form(&ValueType::Population(None)));
        assert!(!forms.has_form(&ValueType::option(map.clone())));
        assert!(forms.has_form(&ValueType::option(ValueType::Boolean)));
        let form = serde_json::json!({"boolean": true});
        assert!(matches!(
            forms.read(&form, &map),
            Err(ReadStop::Unsupported)
        ));
    }

    /// FR-650: a set orders its elements by the form with every Decimal
    /// normalized, and writes each element as it is.
    #[trace("FR-650-AC-1")]
    #[test]
    fn a_set_of_decimals_orders_by_the_normalized_form() {
        let forms = forms();
        let decimal = filament_exact::DecimalType::new(
            Integer::from(0_i64),
            Integer::from(1000_i64),
            0,
            2,
            filament_exact::RoundingMode::Exact,
        )
        .expect("a decimal type");
        let collection =
            CollectionType::new(CollectionKind::Set, ValueType::Decimal(decimal), None);
        let ten = Value::Decimal(Decimal::new(Integer::from(10_i64), 0));
        let one = Value::Decimal(Decimal::new(Integer::from(100_i64), 2));
        let set = filament_exact::from_admitted(collection, vec![ten, one]);
        assert_eq!(
            forms.write(&set).expect("a set of decimals writes"),
            json!({"set": [
                {"kind": "decimal", "coefficient": "100", "scale": "2"},
                {"kind": "decimal", "coefficient": "10", "scale": "0"},
            ]})
        );
    }

    /// FR-650: the writer's depth bound keeps every written value readable.
    #[trace("FR-650-AC-1")]
    #[test]
    fn a_value_nested_deeper_than_the_form_bound_has_no_form() {
        let forms = forms();
        let mut value = Value::Boolean(true);
        for _ in 0..=MAXIMUM_DEPTH {
            value = OptionValue::from_admitted(ValueType::Boolean, Some(value));
        }
        assert_eq!(forms.write(&value), Err(WriteStop::NoForm));
    }
}
