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

use super::identity_form;
use filament_exact::{
    CollectionKind, CollectionType, Decimal, EffectiveId, EnumMember, FieldValue, IeeeValue,
    IeeeWidth, Integer, JsonObject, MapType, Meter, NodeKey, ObjectId, ObjectReference,
    OptionValue, Presence, Quantity, Rational, TextPayload, UnitId, UniverseId, Value, ValueType,
};
use filament_semantic_value::declaration::{CompositeShape, TypeEnvironment};
use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::EnumBinding;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use serde_json::{Map, Value as Json, json};

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
    /// The package string the unit's own declarations live in.
    own_package: &'a str,
    /// The call's object world, when the call names a snapshot.
    world: Option<&'a World<'a>>,
}

/// The object world of a call (FR-650 "Arguments"): the admitted snapshot's
/// objects and the universe each Reference type's objects live in.
pub(crate) struct World<'a> {
    /// The admitted objects and populations.
    pub objects: &'a ObjectEnvironment,
    /// The universe of the objects of a declared object type.
    pub universe_of: &'a dyn Fn(&EffectiveId) -> Option<UniverseId>,
}

impl<'a> Forms<'a> {
    pub(crate) fn new(types: &'a TypeEnvironment, enums: &'a [EnumBinding]) -> Self {
        Self {
            types,
            enums,
            own_package: "",
            world: None,
        }
    }

    /// These forms for a unit whose own declarations live in `own_package`,
    /// over the call's object world.
    pub(crate) fn in_world(mut self, own_package: &'a str, world: Option<&'a World<'a>>) -> Self {
        self.own_package = own_package;
        self.world = world;
        self
    }

    /// Whether a value of `value_type` has a run value form.
    pub(crate) fn has_form(&self, value_type: &ValueType) -> bool {
        self.has_form_visiting(value_type, true, &mut BTreeSet::new())
    }

    /// One exhaustive mapping from the checked type to its form: a
    /// population below the top level has none (FR-650 "Types with no form").
    fn has_form_visiting(
        &self,
        value_type: &ValueType,
        top: bool,
        seen: &mut BTreeSet<NodeKey>,
    ) -> bool {
        match value_type {
            ValueType::Boolean
            | ValueType::Integer
            | ValueType::Int(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Text(_)
            | ValueType::String
            | ValueType::Bytes
            | ValueType::JsonObject
            | ValueType::Uuid
            | ValueType::Timestamp
            | ValueType::Enum(_) => true,
            ValueType::Option(payload) => self.has_form_visiting(payload, false, seen),
            ValueType::Collection(collection) => {
                self.has_form_visiting(collection.element(), false, seen)
            }
            ValueType::Map(map) => {
                self.has_form_visiting(map.key(), false, seen)
                    && self.has_form_visiting(map.values(), false, seen)
            }
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
                        .all(|field| self.has_form_visiting(field.value_type(), false, seen)),
                    CompositeShape::Tuple(positions) => positions
                        .iter()
                        .all(|position| self.has_form_visiting(position, false, seen)),
                    CompositeShape::Union(members) => members.iter().all(|member| {
                        member
                            .positions()
                            .iter()
                            .all(|position| self.has_form_visiting(position, false, seen))
                    }),
                }
            }
            ValueType::Quantity(_)
            | ValueType::BoundedQuantity(_)
            | ValueType::CarriedQuantity(_)
            | ValueType::Reference(_) => true,
            ValueType::Population(_) => top,
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
                let rational = rational_object(object, path)?;
                self.leaf(Value::Rational(rational), value_type, path)
            }
            ValueType::Decimal(_) => {
                let decimal = decimal_object(object, path)?;
                self.leaf(Value::Decimal(decimal), value_type, path)
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
            ValueType::Bytes => {
                native(object, "bytes", &["text"], path)?;
                let text = string_member(object, "text", path)?;
                let bytes = read_bytes(text)
                    .ok_or_else(|| defect(Code::InvalidRuntimeInput, "invalid-value", path))?;
                self.leaf(Value::Bytes(bytes), value_type, path)
            }
            ValueType::JsonObject => {
                native(object, "json_object", &["text"], path)?;
                let text = string_member(object, "text", path)?;
                let json = JsonObject::from_json_text(text)
                    .map(Value::JsonObject)
                    .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", path))?;
                self.leaf(json, value_type, path)
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
            | ValueType::CarriedQuantity(_) => self.read_quantity(object, value_type, path),
            ValueType::Reference(declared) => self.read_reference(object, declared, path),
            ValueType::Population(_) => self.read_population(object, path),
            ValueType::Map(map_type) => self.read_map(object, map_type, path),
        }
    }

    /// A scalar the kernel's membership accepts, else `invalid-value`.
    fn leaf(&self, value: Value, value_type: &ValueType, path: &Path) -> Result<Value, ReadStop> {
        match self.types.admits(value_type, &value) {
            Ok(true) => Ok(value),
            Ok(false) | Err(_) => Err(defect(Code::InvalidRuntimeInput, "invalid-value", path)),
        }
    }

    /// FR-650 "Quantity": `{"kind": "quantity", "unit": N, "magnitude": M}`,
    /// `M` a Rational or a Decimal object read as the exact normalized
    /// rational.
    fn read_quantity(
        &self,
        object: &Map<String, Json>,
        value_type: &ValueType,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        native(object, "quantity", &["unit", "magnitude"], path)?;
        let (unit, carried) = match value_type {
            ValueType::Quantity(unit) => (unit.clone(), false),
            ValueType::BoundedQuantity(bounded) => (bounded.unit().clone(), false),
            ValueType::CarriedQuantity(carried) => (carried.unit().clone(), true),
            _ => return Err(ReadStop::Fault("quantity-slot")),
        };
        let name = string_member(object, "unit", path)?;
        if self.unit_named(name).as_ref() != Some(&unit) {
            return Err(wrong_kind_at(path, "unit"));
        }
        path.push("magnitude");
        let magnitude = self.read_magnitude(&object["magnitude"], carried, path);
        path.pop();
        let value = Value::Quantity(Quantity::new(magnitude?, unit));
        match self.types.admits(value_type, &value) {
            Ok(true) => Ok(value),
            Ok(false) | Err(_) => Err(defect_below(
                Code::InvalidRuntimeInput,
                "invalid-value",
                path,
                &["magnitude"],
            )),
        }
    }

    /// A quantity's magnitude: a Rational object, or, for a quantity that
    /// does not carry a Rational, also a Decimal object.
    fn read_magnitude(
        &self,
        form: &Json,
        rational_only: bool,
        path: &Path,
    ) -> Result<Rational, ReadStop> {
        let Some(object) = form.as_object() else {
            return Err(wrong_kind(path));
        };
        match object.get("kind").and_then(Json::as_str) {
            Some("rational") => rational_object(object, path),
            Some("decimal") if !rational_only => {
                decimal_object(object, path).map(|decimal| decimal.representation().to_rational())
            }
            _ => Err(wrong_kind(path)),
        }
    }

    /// The unit `name` names, by the registry's one public pair.
    fn unit_named(&self, name: &str) -> Option<UnitId> {
        fsl_semantics::model::observation::unit_id_named(self.types, self.own_package, name)
    }

    /// The name of `unit`, by the registry's one public pair.
    fn unit_name(&self, unit: &UnitId) -> Option<String> {
        fsl_semantics::model::observation::unit_name_of(self.types, self.own_package, unit)
    }

    /// FR-650 "Identity reference": `{"population": P, "key": K}` or the
    /// identity triple `{"universe": U, "type": T, "object_identity": O}`.
    fn read_reference(
        &self,
        object: &Map<String, Json>,
        declared: &EffectiveId,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let payload = tagged(object, "reference", path)?;
        let Some(payload) = payload.as_object() else {
            return Err(wrong_kind_at(path, "reference"));
        };
        path.push("reference");
        let read = self.read_reference_payload(payload, declared, path);
        path.pop();
        read
    }

    fn read_reference_payload(
        &self,
        payload: &Map<String, Json>,
        declared: &EffectiveId,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let world = self.world.ok_or_else(no_snapshot)?;
        if payload.contains_key("population") || payload.contains_key("key") {
            identity_form::closed_object(payload, &["population", "key"], path)?;
            let population = string_member(payload, "population", path)?;
            let key = string_member(payload, "key", path)?;
            if world.objects.population_members(population).is_none() {
                return Err(defect(
                    Code::InvalidRuntimeInput,
                    "wrong-role-mapping",
                    path,
                ));
            }
            let universe = (world.universe_of)(declared)
                .ok_or(ReadStop::Fault("reference-type-has-a-universe"))?;
            return world
                .objects
                .objects()
                .find(universe, key)
                .map(|found| Value::Reference(found.clone()))
                .ok_or_else(|| {
                    defect(
                        Code::DanglingReference,
                        "absent-target-in-complete-population",
                        path,
                    )
                });
        }
        identity_form::closed_object(payload, &["universe", "type", "object_identity"], path)?;
        path.push("universe");
        let universe = identity_form::read_universe(&payload["universe"], path);
        path.pop();
        path.push("type");
        let object_type = identity_form::read_effective(&payload["type"], path);
        path.pop();
        let identity = string_member(payload, "object_identity", path)?;
        let object = ObjectId::new(identity).map_err(|_| {
            defect_below(
                Code::InvalidRuntimeInput,
                "invalid-value",
                path,
                &["object_identity"],
            )
        })?;
        Ok(Value::Reference(ObjectReference::new(
            universe?,
            object_type?,
            object,
        )))
    }

    /// FR-650 "Population argument": `{"population": P}` names a population
    /// of the call's snapshot.
    fn read_population(
        &self,
        object: &Map<String, Json>,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let world = self.world.ok_or_else(no_snapshot)?;
        let population = string_member(object, "population", path)?;
        if let Some(extra) = object.keys().find(|name| name.as_str() != "population") {
            return Err(defect_below(
                Code::InvalidRuntimeInput,
                "unknown-member",
                path,
                &[extra],
            ));
        }
        world
            .objects
            .population_binding(population)
            .map(|binding| Value::Population(binding.population_id()))
            .ok_or_else(|| defect(Code::InvalidRuntimeInput, "wrong-role-mapping", path))
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

    /// FR-650's typed Map value form, with duplicate keys refused at the
    /// later entry's key member.
    fn read_map(
        &self,
        object: &Map<String, Json>,
        map_type: &MapType,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let payload = tagged(object, "map", path)?;
        let Some(items) = payload.as_array() else {
            return Err(wrong_kind_at(path, "map"));
        };
        if let Some(bound) = map_type.bound() {
            if let Some(violation) = bound.violation(filament_exact::length_amount(items.len())) {
                return Err(defect(
                    Code::CardinalityOutOfBound,
                    violation.as_str(),
                    path,
                ));
            }
        }
        path.push("map");
        let result = (|| {
            let mut entries = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                path.push(&index.to_string());
                let Some(entry) = item.as_object() else {
                    return Err(wrong_kind(path));
                };
                if let Some(extra) = entry
                    .keys()
                    .find(|name| name.as_str() != "key" && name.as_str() != "value")
                {
                    path.push(extra);
                    let found = defect(Code::InvalidRuntimeInput, "unknown-member", path);
                    path.pop();
                    return Err(found);
                }
                if !entry.contains_key("key") || !entry.contains_key("value") {
                    return Err(defect(Code::InvalidRuntimeInput, "missing-member", path));
                }
                path.push("key");
                let key = self.read_at(&entry["key"], map_type.key(), path);
                path.pop();
                let key = key?;
                path.push("value");
                let value = self.read_at(&entry["value"], map_type.values(), path);
                path.pop();
                let value = value?;
                entries.push((key, value));
                path.pop();
            }
            filament_exact::from_admitted_map(map_type.clone(), entries).map_err(|refusal| {
                match (refusal.component, refusal.cause) {
                    (
                        filament_exact::Component::Element(index),
                        filament_exact::ConstructionCause::DuplicateMember,
                    ) => {
                        path.push(&index.to_string());
                        path.push("key");
                        let found = defect(Code::InvalidRuntimeInput, "duplicate-member", path);
                        path.pop();
                        path.pop();
                        found
                    }
                    _ => defect(Code::InvalidRuntimeInput, "invalid-value", path),
                }
            })
        })();
        path.pop();
        result
    }

    fn read_enum(
        &self,
        object: &Map<String, Json>,
        shape: &filament_exact::EnumShape,
        path: &mut Path,
    ) -> Result<Value, ReadStop> {
        let payload = variant_payload(object, path)?;
        let (name, variants) = self.enum_cases(shape)?;
        let named = string_member_of(payload, "type", path, "variant")?;
        if named != name {
            return Err(wrong_kind_at2(path, "variant", "type"));
        }
        let case = string_member_of(payload, "case", path, "variant")?;
        let position = variants
            .iter()
            .position(|variant| variant.case() == case)
            .ok_or_else(|| invalid_at2(path, "variant", "case"))?;
        if payload.len() != 2 {
            return Err(unknown_member_at(
                path,
                "variant",
                payload,
                &["type", "case"],
            ));
        }
        let rank = u32::try_from(position).map_err(|_| ReadStop::Fault("enum-rank"))?;
        Ok(Value::Enum(EnumMember::new(
            variants[position].clone(),
            rank,
        )))
    }

    /// The declared name and the variants, in rank order, of the enum `shape`:
    /// the unit's own enum by its declared name, else a model enum by the
    /// `ix://` identity of its declaration.
    fn enum_cases(
        &self,
        shape: &filament_exact::EnumShape,
    ) -> Result<(String, Vec<filament_exact::VariantId>), ReadStop> {
        if let Some(binding) = self.enums.iter().find(|binding| binding.shape() == *shape) {
            return Ok((
                binding.name.clone(),
                binding
                    .members
                    .iter()
                    .map(|member| member.variant().clone())
                    .collect(),
            ));
        }
        let variants = shape.variants().cloned().collect::<Vec<_>>();
        let name = variants
            .first()
            .and_then(|variant| model_declaration_name(variant.owner()))
            .ok_or(ReadStop::Fault("enum-shape-has-a-declaration"))?;
        Ok((name, variants))
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
            .find(|candidate| declaration_name(candidate) == named)
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
            Value::Bytes(held) => json!({"kind": "bytes", "text": bytes_text(held)}),
            Value::JsonObject(held) => {
                json!({"kind": "json_object", "text": held.canonical_text()})
            }
            Value::Enum(member) => {
                let name = self
                    .enums
                    .iter()
                    .find(|binding| {
                        binding
                            .members
                            .iter()
                            .any(|candidate| candidate.variant() == *member.variant())
                    })
                    .map(|binding| binding.name.clone())
                    .or_else(|| model_declaration_name(member.variant().owner()))
                    .ok_or(WriteStop::Fault("enum-member-has-a-declaration"))?;
                json!({"variant": {"type": name, "case": member.variant().case()}})
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
            Value::Map(map) => {
                // FR-650's run form sorts by the RFC 8785 bytes of the
                // normalized key form. Semantic walks use FR-144 order.
                let mut keyed = map
                    .entries()
                    .iter()
                    .map(|(key, value)| {
                        let key_form = self.write_at(key, below)?;
                        Ok((order_key(&key_form), key_form, value))
                    })
                    .collect::<Result<Vec<_>, WriteStop>>()?;
                keyed.sort_by(|left, right| left.0.cmp(&right.0));
                let entries = keyed
                    .into_iter()
                    .map(|(_, key, value)| {
                        Ok(json!({
                            "key": key,
                            "value": self.write_at(value, below)?,
                        }))
                    })
                    .collect::<Result<Vec<_>, WriteStop>>()?;
                json!({"map": entries})
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
                        json!({"record": {"type": declaration_name(declaration), "fields": members}})
                    }
                    CompositeShape::Tuple(_) => {
                        let components = composite
                            .slots()
                            .iter()
                            .map(|slot| self.write_slot(slot, Presence::Required, below))
                            .collect::<Result<Vec<_>, _>>()?;
                        json!({"tuple": {"type": declaration_name(declaration), "components": components}})
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
                payload.insert("type".to_owned(), json!(declaration_name(declaration)));
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
            Value::Quantity(held) => {
                let name = self
                    .unit_name(held.unit())
                    .ok_or(WriteStop::Fault("quantity-unit-has-a-name"))?;
                json!({
                    "kind": "quantity",
                    "unit": name,
                    "magnitude": {
                        "kind": "rational",
                        "numerator": held.magnitude().numerator().to_string(),
                        "denominator": held.magnitude().denominator().to_string(),
                    },
                })
            }
            Value::Reference(reference) => reference_form(reference),
            Value::Population(id) => {
                let world = self.world.ok_or(WriteStop::NoForm)?;
                let binding = world
                    .objects
                    .resolve_population(id.clone())
                    .ok_or(WriteStop::Fault("population-has-a-binding"))?;
                let mut members = binding
                    .members()
                    .keys()
                    .map(|key| {
                        let object = ObjectId::new(key.object.clone())
                            .map_err(|_| WriteStop::Fault("population-member-has-an-identity"))?;
                        Ok(reference_form(&ObjectReference::new(
                            key.universe.clone(),
                            key.type_identity.clone(),
                            object,
                        )))
                    })
                    .collect::<Result<Vec<_>, WriteStop>>()?;
                members.sort_by_key(order_key);
                json!({"set": members})
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
/// The name a composite declaration is spelled by: its `ix://` identity when
/// a selected model declares it, else its declared name.
fn declaration_name(
    declaration: &filament_semantic_value::declaration::CompositeDeclaration,
) -> String {
    model_declaration_name(&declaration.key()).unwrap_or_else(|| declaration.name().to_owned())
}

/// The `ix://<package>/<artifact id>` name of a declaration a selected model
/// declares: the node string of its structural node id (FR-650 "Declaration
/// names").
fn model_declaration_name(owner: &NodeKey) -> Option<String> {
    let segments = owner
        .view()
        .path()
        .map(|segment| match segment {
            filament_exact::PathSegmentView::Text(text) => Some(text),
            filament_exact::PathSegmentView::Node(_) => None,
        })
        .collect::<Option<Vec<_>>>()?;
    match segments.as_slice() {
        [_, _, node] if node.starts_with("ix://") => Some((*node).to_owned()),
        _ => None,
    }
}

/// A Reference result in the identity-reference form: the returned
/// reference's own universe, most-specific type and object identity.
pub(crate) fn reference_form(reference: &ObjectReference) -> Json {
    json!({"reference": {
        "universe": identity_form::universe_form(reference.universe()),
        "type": identity_form::effective_form(reference.object_type()),
        "object_identity": reference.object().as_str(),
    }})
}

fn order_key(form: &Json) -> Vec<u8> {
    let mut normalized = form.clone();
    normalize_decimals(&mut normalized);
    normalized.to_string().into_bytes()
}

/// FR-106's canonical lowercase hexadecimal spelling of a Bytes payload.
fn bytes_text(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        text.push(char::from(HEX[usize::from(*byte >> 4)]));
        text.push(char::from(HEX[usize::from(*byte & 0x0f)]));
    }
    text
}

/// Parse only FR-106's canonical lowercase, even-length hexadecimal payload.
fn read_bytes(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = u8::from_str_radix(std::str::from_utf8(&pair[..1]).ok()?, 16).ok()?;
            let low = u8::from_str_radix(std::str::from_utf8(&pair[1..]).ok()?, 16).ok()?;
            Some((high << 4) | low)
        })
        .collect()
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
#[derive(Clone)]
pub(crate) struct Path(Vec<String>);

impl Path {
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn push(&mut self, segment: &str) {
        self.0.push(segment.to_owned());
    }

    pub(crate) fn pop(&mut self) {
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

pub(crate) fn defect(code: Code, cause: &'static str, path: &Path) -> ReadStop {
    ReadStop::Defect(ArgumentDefect {
        code,
        cause,
        path: path.pointer(),
    })
}

pub(crate) fn defect_below(
    code: Code,
    cause: &'static str,
    path: &Path,
    below: &[&str],
) -> ReadStop {
    ReadStop::Defect(ArgumentDefect {
        code,
        cause,
        path: path.pointer_with(below),
    })
}

pub(crate) fn wrong_kind(path: &Path) -> ReadStop {
    defect(Code::InvalidRuntimeInput, "wrong-value-kind", path)
}

pub(crate) fn wrong_kind_at(path: &Path, member: &str) -> ReadStop {
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
/// The refusal of a Reference or Population argument in a call with no
/// `snapshot` (FR-650 "Arguments").
fn no_snapshot() -> ReadStop {
    ReadStop::Defect(ArgumentDefect {
        code: Code::InvalidRuntimeInput,
        cause: "missing-member",
        path: "/call/snapshot".to_owned(),
    })
}

/// A Rational object `{"kind": "rational", "numerator", "denominator"}`.
fn rational_object(object: &Map<String, Json>, path: &Path) -> Result<Rational, ReadStop> {
    native(object, "rational", &["numerator", "denominator"], path)?;
    let numerator = integer_text(&object["numerator"], path, "numerator")?;
    let denominator = integer_text(&object["denominator"], path, "denominator")?;
    Rational::new(numerator, denominator)
        .map_err(|_| defect(Code::InvalidRuntimeInput, "invalid-value", path))
}

/// A Decimal object `{"kind": "decimal", "coefficient", "scale"}`.
fn decimal_object(object: &Map<String, Json>, path: &Path) -> Result<Decimal, ReadStop> {
    native(object, "decimal", &["coefficient", "scale"], path)?;
    let coefficient = integer_text(&object["coefficient"], path, "coefficient")?;
    let scale = integer_text(&object["scale"], path, "scale")?
        .to_u64()
        .and_then(|scale| u32::try_from(scale).ok())
        .ok_or_else(|| defect(Code::InvalidRuntimeInput, "invalid-value", path))?;
    Ok(Decimal::new(coefficient, scale))
}

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

    /// FR-650-AC-6: population has a top-level form, but not a nested one.
    #[trace("FR-650-AC-6")]
    #[test]
    fn types_with_no_run_value_form_are_not_encodable() {
        let forms = forms();
        // A population is a form only at the top: a collection or option
        // over one has none (a checked package can carry such a type though
        // source cannot).
        assert!(forms.has_form(&ValueType::Population(Some(1))));
        let over_population = ValueType::option(ValueType::Population(Some(1)));
        assert!(!forms.has_form(&over_population));
        let sequence = ValueType::Collection(std::sync::Arc::new(CollectionType::new(
            CollectionKind::Sequence,
            ValueType::Population(Some(1)),
            None,
        )));
        assert!(!forms.has_form(&sequence));
        assert!(forms.has_form(&ValueType::option(ValueType::Boolean)));
    }

    /// FR-650: run forms write Map entries in normalized run-key form order;
    /// FR-144's mathematical order belongs to semantic traversal.
    #[trace("FR-650-AC-11", "FR-144-AC-6")]
    #[test]
    fn maps_write_entries_in_run_form_order() {
        let forms = forms();
        let map = ValueType::Map(std::sync::Arc::new(
            filament_exact::MapType::new(ValueType::Integer, ValueType::Boolean, None)
                .expect("a leaf key"),
        ));
        assert!(forms.has_form(&map));
        let form = serde_json::json!({
            "map": [
                {"key": {"integer": "10"}, "value": {"boolean": true}},
                {"key": {"integer": "9"}, "value": {"boolean": false}},
            ]
        });
        let value = forms.read(&form, &map).expect("the map form is valid");
        assert_eq!(
            forms.write(&value).expect("the map value is writable"),
            serde_json::json!({
                "map": [
                    {"key": {"integer": "10"}, "value": {"boolean": true}},
                    {"key": {"integer": "9"}, "value": {"boolean": false}},
                ]
            })
        );
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
}
