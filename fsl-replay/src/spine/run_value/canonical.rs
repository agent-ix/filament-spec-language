// SPDX-License-Identifier: AGPL-3.0-or-later
//! The canonical run boundary adapts structural JSON to the existing typed
//! admissions. Numeric, membership and object-world decisions remain theirs.

use super::*;

struct Position<'a> {
    argument: usize,
    next: u64,
    run: &'a std::sync::Mutex<filament_semantic_value::supplied_stop::SuppliedRun>,
}

impl Position<'_> {
    fn charge(
        &self,
        node: u64,
        event: filament_semantic_value::supplied_stop::SuppliedEvent,
        units: usize,
    ) -> Result<(), ReadStop> {
        let mut run = self
            .run
            .lock()
            .expect("conversion admission does not unwind");
        for _ in 0..units {
            run.charge_conversion(
                self.argument,
                event,
                filament_semantic_value::supplied_stop::SuppliedLocus::Witness {
                    argument: self.argument,
                    position: node,
                },
            )
            .map_err(|stop| {
                ReadStop::Refusal(Box::new(super::super::RunRefusal::Supplied(stop)))
            })?;
        }
        Ok(())
    }

    fn preflight(&self, form: &Json, node: u64) -> Result<(), ReadStop> {
        use filament_semantic_value::supplied_stop::SuppliedEvent;
        self.charge(node, SuppliedEvent::WitnessPreflight, 1)?;
        if let Some(object) = form.as_object() {
            for (name, value) in object {
                self.charge(node, SuppliedEvent::DescriptorQuery, name.len())?;
                if let Some(text) = value.as_str() {
                    self.charge(node, SuppliedEvent::WitnessMaterialization, text.len())?;
                }
            }
        }
        self.charge(node, SuppliedEvent::WitnessMaterialization, 1)
    }

    fn array(&self, length: usize) -> Result<Vec<Json>, ReadStop> {
        self.charge(
            self.next.saturating_sub(1),
            filament_semantic_value::supplied_stop::SuppliedEvent::ChildScheduling,
            length,
        )?;
        let bytes = length
            .checked_mul(std::mem::size_of::<Json>())
            .ok_or(ReadStop::Fault("canonical-array-size-fits"))?;
        let mut values = Vec::new();
        values.try_reserve_exact(length).map_err(|_| {
            ReadStop::Refusal(Box::new(super::super::RunRefusal::Reference(Box::new(
                crate::reference_codec::ReferenceCodecError::Key {
                    pointer: String::new(),
                    source: fsl_eval::simulation::model::ModelKeyError::Encoding(
                        fsl_eval::simulation::EncodingRefusal::Allocation { requested: bytes },
                    ),
                },
            ))))
        })?;
        Ok(values)
    }

    fn copied_selector(&self, value: &Json, node: u64) -> Result<Json, ReadStop> {
        use filament_semantic_value::supplied_stop::SuppliedEvent;
        let mut pending = vec![value];
        while let Some(value) = pending.pop() {
            self.charge(node, SuppliedEvent::WitnessMaterialization, 1)?;
            let children = match value {
                Json::Array(values) => values.len(),
                Json::Object(values) => values.len(),
                _ => 0,
            };
            self.charge(node, SuppliedEvent::ChildScheduling, children)?;
            let bytes = children
                .checked_mul(std::mem::size_of::<&Json>())
                .ok_or(ReadStop::Fault("selector-copy-size-fits"))?;
            pending.try_reserve_exact(children).map_err(|_| {
                ReadStop::Refusal(Box::new(super::super::RunRefusal::Reference(Box::new(
                    crate::reference_codec::ReferenceCodecError::Key {
                        pointer: String::new(),
                        source: fsl_eval::simulation::model::ModelKeyError::Encoding(
                            fsl_eval::simulation::EncodingRefusal::Allocation { requested: bytes },
                        ),
                    },
                ))))
            })?;
            match value {
                Json::String(text) => {
                    self.charge(node, SuppliedEvent::DescriptorQuery, text.len())?
                }
                Json::Array(values) => pending.extend(values),
                Json::Object(values) => {
                    for (name, value) in values {
                        self.charge(node, SuppliedEvent::DescriptorQuery, name.len())?;
                        pending.push(value);
                    }
                }
                Json::Null | Json::Bool(_) | Json::Number(_) => {}
            }
        }
        Ok(value.clone())
    }
}

impl Forms<'_> {
    pub(crate) fn read_canonical(
        &self,
        form: &Json,
        declared: &ValueType,
        argument: usize,
        run: &std::sync::Mutex<filament_semantic_value::supplied_stop::SuppliedRun>,
    ) -> Result<Value, ReadStop> {
        let normalized = self.canonical_input(
            form,
            declared,
            &mut Path::new(),
            &mut Position {
                argument,
                next: 0,
                run,
            },
        )?;
        self.read(&normalized, declared).map_err(|stop| match stop {
            ReadStop::Defect(mut defect) => {
                defect.path = canonical_pointer(form, &normalized, &defect.path);
                ReadStop::Defect(defect)
            }
            fault @ ReadStop::Fault(_) => fault,
            refusal @ ReadStop::Refusal(_) => refusal,
        })
    }

    fn canonical_input(
        &self,
        form: &Json,
        declared: &ValueType,
        path: &mut Path,
        position: &mut Position,
    ) -> Result<Json, ReadStop> {
        let node = position.next;
        position.preflight(form, node)?;
        position.next = position
            .next
            .checked_add(1)
            .ok_or(ReadStop::Fault("canonical-preorder-fits-u64"))?;
        if matches!(declared, ValueType::Population(_)) {
            identity_form::closed(form, &["population"], path)?;
            return Ok(json!({"population": position.copied_selector(&form["population"], node)?}));
        }
        let object = form.as_object().ok_or_else(|| wrong_kind(path))?;
        let tag = object
            .get("type")
            .and_then(Json::as_str)
            .ok_or_else(|| wrong_kind(path))?;
        let wanted = match declared {
            ValueType::Boolean => "boolean",
            ValueType::Integer | ValueType::Int(_) => "integer",
            ValueType::Rational(_)
            | ValueType::Quantity(_)
            | ValueType::BoundedQuantity(_)
            | ValueType::CarriedQuantity(_) => "rational",
            ValueType::Decimal(_) => "decimal",
            ValueType::Float(float) => match float.width() {
                IeeeWidth::Binary32 => "float32",
                IeeeWidth::Binary64 => "float64",
            },
            ValueType::Text(_) | ValueType::String => "text",
            ValueType::Bytes => "bytes",
            ValueType::JsonObject => "json_object",
            ValueType::Uuid => "uuid",
            ValueType::Timestamp => "timestamp",
            ValueType::Option(_) => "option",
            ValueType::Collection(collection) => collection_tag(collection.kind()),
            ValueType::Map(_) => "map",
            ValueType::Enum(_) => "enum",
            ValueType::Reference(_) => "reference",
            ValueType::Composite(key) => match self
                .types
                .composite(key.clone())
                .ok_or(ReadStop::Fault("composite-declaration"))?
                .shape()
            {
                CompositeShape::Record(_) => "record",
                CompositeShape::Tuple(_) => "tuple",
                CompositeShape::Union(_) => "union",
            },
            ValueType::Population(_) => return Err(ReadStop::Fault("population-already-read")),
        };
        if tag != wanted {
            return Err(wrong_kind(path));
        }
        match declared {
            ValueType::Boolean | ValueType::Integer | ValueType::Int(_) => {
                closed(object, &["type", "value"], path)?;
                if matches!(declared, ValueType::Boolean) {
                    if !object["value"].is_boolean() {
                        return Err(wrong_kind_at(path, "value"));
                    }
                } else {
                    integer_text(&object["value"], path, "value")?;
                }
                Ok(json!({wanted: object["value"]}))
            }
            ValueType::Rational(_) => {
                closed(object, &["type", "numerator", "denominator"], path)?;
                integer_text(&object["numerator"], path, "numerator")?;
                integer_text(&object["denominator"], path, "denominator")?;
                Ok(native_form(object, "rational"))
            }
            ValueType::Decimal(_) => {
                closed(object, &["type", "coefficient", "scale"], path)?;
                integer_text(&object["coefficient"], path, "coefficient")?;
                integer_text(&object["scale"], path, "scale")?;
                let mut value = native_form(object, "decimal");
                let _ = integer_text(&object["scale"], path, "scale")?;
                value["scale"] = object["scale"].clone();
                Ok(value)
            }
            ValueType::Float(_) => {
                closed(object, &["type", "bits"], path)?;
                string_member(object, "bits", path)?;
                Ok(native_form(object, wanted))
            }
            ValueType::Text(_) | ValueType::String | ValueType::Uuid => {
                closed(object, &["type", "value"], path)?;
                string_member(object, "value", path)?;
                let kind = match declared {
                    ValueType::String => "string",
                    ValueType::Uuid => "uuid",
                    _ => "text",
                };
                Ok(json!({"kind": kind, "text": object["value"]}))
            }
            ValueType::Bytes | ValueType::JsonObject => {
                closed(object, &["type", "text"], path)?;
                string_member(object, "text", path)?;
                Ok(native_form(object, wanted))
            }
            ValueType::Timestamp => {
                closed(object, &["type", "nanoseconds"], path)?;
                integer_text(&object["nanoseconds"], path, "nanoseconds")?;
                Ok(native_form(object, wanted))
            }
            ValueType::Option(payload) => {
                closed(object, &["type", "value"], path)?;
                if object["value"].is_null() {
                    Ok(json!({"absent": {}}))
                } else {
                    let value =
                        self.canonical_child(&object["value"], payload, path, "value", position)?;
                    Ok(json!({"present": value}))
                }
            }
            ValueType::Collection(collection) => {
                closed(object, &["type", "elements"], path)?;
                let values = self.canonical_elements(
                    &object["elements"],
                    collection.element(),
                    path,
                    "elements",
                    position,
                )?;
                Ok(json!({wanted: values}))
            }
            ValueType::Map(map) => {
                closed(object, &["type", "entries"], path)?;
                let entries = object["entries"]
                    .as_array()
                    .ok_or_else(|| wrong_kind_at(path, "entries"))?;
                let mut normalized = position.array(entries.len())?;
                path.push("entries");
                for (index, entry) in entries.iter().enumerate() {
                    path.push(&index.to_string());
                    let entry = identity_form::closed(entry, &["key", "value"], path)?;
                    let key =
                        self.canonical_child(&entry["key"], map.key(), path, "key", position)?;
                    let value = self.canonical_child(
                        &entry["value"],
                        map.values(),
                        path,
                        "value",
                        position,
                    )?;
                    normalized.push(json!({"key": key, "value": value}));
                    path.pop();
                }
                path.pop();
                Ok(json!({"map": normalized}))
            }
            ValueType::Enum(shape) => {
                closed(object, &["type", "name", "member"], path)?;
                let (name, members) = self.enum_cases(shape)?;
                let owner = members
                    .first()
                    .ok_or(ReadStop::Fault("enum-has-members"))?
                    .owner();
                if object["name"] != declaration_form(owner)? {
                    return Err(wrong_kind_at(path, "name"));
                }
                Ok(json!({"variant": {"type": name, "case": object["member"]}}))
            }
            ValueType::Composite(key) => self.canonical_composite(object, key, path, position),
            ValueType::Quantity(_)
            | ValueType::BoundedQuantity(_)
            | ValueType::CarriedQuantity(_) => {
                closed(object, &["type", "numerator", "denominator", "unit"], path)?;
                let unit = string_member(object, "unit", path)?;
                let unit = crate::id_text::unit_from_text(unit)
                    .and_then(|unit| self.unit_name(&unit))
                    .ok_or_else(|| wrong_kind_at(path, "unit"))?;
                Ok(json!({"kind": "quantity", "unit": unit,
                    "magnitude": {"kind": "rational", "numerator": object["numerator"], "denominator": object["denominator"]}}))
            }
            ValueType::Reference(_) => self.canonical_reference(
                form,
                object,
                declared,
                path,
                &mut Position {
                    argument: position.argument,
                    next: node,
                    run: position.run,
                },
            ),
            ValueType::Population(_) => Err(ReadStop::Fault("population-already-read")),
        }
    }

    fn canonical_child(
        &self,
        form: &Json,
        declared: &ValueType,
        path: &mut Path,
        segment: &str,
        position: &mut Position,
    ) -> Result<Json, ReadStop> {
        path.push(segment);
        let value = self.canonical_input(form, declared, path, position);
        path.pop();
        value
    }

    fn canonical_reference(
        &self,
        form: &Json,
        object: &Map<String, Json>,
        _declared: &ValueType,
        path: &mut Path,
        position: &mut Position,
    ) -> Result<Json, ReadStop> {
        closed(
            object,
            &["type", "universe", "object_type", "identity"],
            path,
        )?;
        let world = self.world.ok_or_else(no_snapshot)?;
        let restore = world
            .restore_reference
            .ok_or(ReadStop::Fault("canonical-call-has-reference-adapter"))?;
        let reference = restore(form, position.argument, position.next).map_err(|mut error| {
            if let ReadStop::Refusal(ref mut refusal) = error {
                if let super::super::RunRefusal::Reference(ref mut reference) = **refusal {
                    let pointer = match reference.as_mut() {
                        crate::reference_codec::ReferenceCodecError::IntegerParse {
                            pointer,
                            ..
                        }
                        | crate::reference_codec::ReferenceCodecError::Carrier {
                            pointer, ..
                        }
                        | crate::reference_codec::ReferenceCodecError::Decimal {
                            pointer, ..
                        }
                        | crate::reference_codec::ReferenceCodecError::IdentityCopy {
                            pointer,
                            ..
                        }
                        | crate::reference_codec::ReferenceCodecError::Universe {
                            pointer, ..
                        }
                        | crate::reference_codec::ReferenceCodecError::Native { pointer, .. }
                        | crate::reference_codec::ReferenceCodecError::Transition {
                            pointer, ..
                        }
                        | crate::reference_codec::ReferenceCodecError::Refused {
                            pointer, ..
                        }
                        | crate::reference_codec::ReferenceCodecError::Key { pointer, .. }
                        | crate::reference_codec::ReferenceCodecError::Normalization {
                            pointer,
                            ..
                        } => pointer,
                    };
                    *pointer = format!("{}{pointer}", path.pointer());
                }
            }
            error
        })?;
        Ok(reference_form(&reference))
    }

    fn canonical_elements(
        &self,
        form: &Json,
        declared: &ValueType,
        path: &mut Path,
        segment: &str,
        position: &mut Position,
    ) -> Result<Vec<Json>, ReadStop> {
        let values = form
            .as_array()
            .ok_or_else(|| wrong_kind_at(path, segment))?;
        path.push(segment);
        let mut normalized = position.array(values.len())?;
        for (index, value) in values.iter().enumerate() {
            normalized.push(self.canonical_child(
                value,
                declared,
                path,
                &index.to_string(),
                position,
            )?);
        }
        path.pop();
        Ok(normalized)
    }

    fn canonical_composite(
        &self,
        object: &Map<String, Json>,
        declared: &NodeKey,
        path: &mut Path,
        position: &mut Position,
    ) -> Result<Json, ReadStop> {
        let name = object.get("name").ok_or_else(|| {
            defect_below(Code::InvalidRuntimeInput, "missing-member", path, &["name"])
        })?;
        let declaration = self
            .types
            .composites()
            .find(|candidate| declaration_form(&candidate.key()).is_ok_and(|form| &form == name))
            .filter(|candidate| {
                self.types
                    .composite_conforms(candidate.key(), declared.clone())
            })
            .ok_or_else(|| wrong_kind_at(path, "name"))?;
        let legacy_name = declaration_name(declaration);
        match declaration.shape() {
            CompositeShape::Record(fields) => {
                closed(object, &["type", "name", "fields"], path)?;
                let entries = object["fields"]
                    .as_array()
                    .ok_or_else(|| wrong_kind_at(path, "fields"))?;
                if entries.len() != fields.len() {
                    return Err(defect_below(
                        Code::InvalidRuntimeInput,
                        "invalid-value",
                        path,
                        &["fields"],
                    ));
                }
                let mut normalized = Map::new();
                let parent = position.next.saturating_sub(1);
                position.charge(
                    parent,
                    filament_semantic_value::supplied_stop::SuppliedEvent::ChildScheduling,
                    entries.len(),
                )?;
                path.push("fields");
                for (index, (entry, field)) in entries.iter().zip(fields).enumerate() {
                    path.push(&index.to_string());
                    let entry = identity_form::closed(entry, &["name", "value"], path)?;
                    if let Some(name) = entry["name"].as_str() {
                        position.charge(
                            parent,
                            filament_semantic_value::supplied_stop::SuppliedEvent::DescriptorQuery,
                            name.len(),
                        )?;
                    }
                    if entry["name"].as_str() != Some(field.name()) {
                        return Err(defect_below(
                            Code::InvalidRuntimeInput,
                            "invalid-value",
                            path,
                            &["name"],
                        ));
                    }
                    let value = match (field.presence(), entry["value"].is_null()) {
                        (Presence::Optional, true) => json!({"absent": {}}),
                        (presence, _) => {
                            let value = self.canonical_child(
                                &entry["value"],
                                field.value_type(),
                                path,
                                "value",
                                position,
                            )?;
                            if presence == Presence::Optional {
                                json!({"present": value})
                            } else {
                                value
                            }
                        }
                    };
                    normalized.insert(field.name().to_owned(), value);
                    path.pop();
                }
                path.pop();
                Ok(json!({"record": {"type": legacy_name, "fields": normalized}}))
            }
            CompositeShape::Tuple(positions) => {
                closed(object, &["type", "name", "components"], path)?;
                let values =
                    self.canonical_positions(&object["components"], positions, path, position)?;
                Ok(json!({"tuple": {"type": legacy_name, "components": values}}))
            }
            CompositeShape::Union(members) => {
                closed(object, &["type", "name", "member", "components"], path)?;
                let name = string_member(object, "member", path)?;
                let member = members
                    .iter()
                    .find(|member| member.identifier() == name)
                    .ok_or_else(|| {
                        defect_below(
                            Code::InvalidRuntimeInput,
                            "invalid-value",
                            path,
                            &["member"],
                        )
                    })?;
                let values = self.canonical_positions(
                    &object["components"],
                    member.positions(),
                    path,
                    position,
                )?;
                if values.is_empty() {
                    Ok(json!({"variant": {"type": legacy_name, "case": name}}))
                } else {
                    Ok(
                        json!({"variant": {"type": legacy_name, "case": name, "components": values}}),
                    )
                }
            }
        }
    }

    fn canonical_positions(
        &self,
        form: &Json,
        positions: &[ValueType],
        path: &mut Path,
        position: &mut Position,
    ) -> Result<Vec<Json>, ReadStop> {
        let values = form
            .as_array()
            .ok_or_else(|| wrong_kind_at(path, "components"))?;
        if values.len() != positions.len() {
            return Err(defect_below(
                Code::InvalidRuntimeInput,
                "invalid-value",
                path,
                &["components"],
            ));
        }
        path.push("components");
        let mut normalized = position.array(values.len())?;
        for (index, (value, declared)) in values.iter().zip(positions).enumerate() {
            normalized.push(self.canonical_child(
                value,
                declared,
                path,
                &index.to_string(),
                position,
            )?);
        }
        path.pop();
        Ok(normalized)
    }

    pub(crate) fn write_canonical(&self, value: &Value) -> Result<Json, WriteStop> {
        let native = if let Value::Population(population) = value {
            let world = self
                .world
                .ok_or(WriteStop::Fault("population-result-has-object-world"))?;
            let binding = world
                .objects
                .resolve_population(population.clone())
                .ok_or(WriteStop::Fault("population-result-has-binding"))?;
            let mut elements = binding
                .members()
                .keys()
                .map(|key| {
                    let object = ObjectId::new(key.object.clone())
                        .map_err(|_| WriteStop::Fault("population-member-has-an-identity"))?;
                    let value = Value::Reference(ObjectReference::new(
                        key.universe.clone(),
                        key.type_identity.clone(),
                        object,
                    ));
                    self.canonical_writer
                        .ok_or(WriteStop::Fault("canonical-call-has-typed-writer"))?(
                        &value
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            elements.sort_by_key(order_key);
            json!({"type": "set", "elements": elements})
        } else {
            self.canonical_writer
                .ok_or(WriteStop::Fault("canonical-call-has-typed-writer"))?(value)?
        };
        Ok(native)
    }
}

fn closed(object: &Map<String, Json>, members: &[&str], path: &Path) -> Result<(), ReadStop> {
    identity_form::closed_object(object, members, path)
}

fn native_form(object: &Map<String, Json>, kind: &str) -> Json {
    let mut payload = object.clone();
    payload.remove("type");
    payload.insert("kind".to_owned(), json!(kind));
    Json::Object(payload)
}

fn declaration_form(key: &NodeKey) -> Result<Json, ReadStop> {
    fsl_eval::simulation::model::run_declaration_name(key)
        .map(Json::String)
        .ok_or(ReadStop::Fault("run-type-has-declaration-name"))
}

/// Preserve the typed reader's refusal while locating it in the caller's
/// canonical tree, including record slots and native optional-field wrappers.
fn canonical_pointer(mut canonical: &Json, mut native: &Json, pointer: &str) -> String {
    let mut path = Path::new();
    let mut segments = pointer
        .strip_prefix('/')
        .unwrap_or("")
        .split('/')
        .peekable();
    if pointer.is_empty() {
        return String::new();
    }
    while let Some(segment) = segments.next() {
        let segment = segment.replace("~1", "/").replace("~0", "~");
        let kind = canonical.get("type").and_then(Json::as_str);
        if matches!(
            segment.as_str(),
            "record" | "tuple" | "variant" | "reference"
        ) && native.get(&segment).is_some()
        {
            native = &native[&segment];
            continue;
        }
        if segment == "present" && kind != Some("option") && native.get("present").is_some() {
            native = &native["present"];
            continue;
        }
        if segment == "fields" && kind == Some("record") {
            path.push("fields");
            canonical = &canonical["fields"];
            native = &native["fields"];
            if let Some(name) = segments.next() {
                let name = name.replace("~1", "/").replace("~0", "~");
                let Some((index, field)) = canonical.as_array().and_then(|fields| {
                    fields
                        .iter()
                        .enumerate()
                        .find(|(_, field)| field["name"].as_str() == Some(&name))
                }) else {
                    path.push(&name);
                    continue;
                };
                path.push(&index.to_string());
                path.push("value");
                canonical = &field["value"];
                native = &native[&name];
            }
            continue;
        }
        let mapped = match segment.as_str() {
            "integer" | "boolean" | "present" => "value",
            "sequence" | "set" | "bag" | "ordered-set" => "elements",
            "map" => "entries",
            "case" => "member",
            "type" if kind == Some("reference") => "object_type",
            "type" if matches!(kind, Some("record" | "tuple" | "union" | "enum")) => "name",
            "object_identity" => "identity",
            "text" if canonical.get("value").is_some() => "value",
            other => other,
        };
        path.push(mapped);
        native = native
            .get(&segment)
            .or_else(|| {
                segment
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| native.get(index))
            })
            .unwrap_or(&Json::Null);
        canonical = canonical
            .get(mapped)
            .or_else(|| {
                mapped
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| canonical.get(index))
            })
            .unwrap_or(&Json::Null);
    }
    path.pointer()
}
