// SPDX-License-Identifier: AGPL-3.0-or-later
//! External mapped-state carriers, separate from opaque native StateKey bytes.

use super::*;

impl ReferenceScope<'_, '_, '_, '_> {
    /// Project a mapped state with its actual abstract/concrete runtime and
    /// retained source owner. Every reference leaf uses this shared codec.
    ///
    /// # Errors
    /// Preserves typed state shape, identity, caller and cancellation errors.
    pub fn encode_model_state(
        &mut self,
        state: &fsl_semantics::model::state::ModelState,
        system: &ModelSystem<'_>,
        owner: &SelectedModels,
    ) -> Result<Value, ReferenceCodecError> {
        self.model_runtime(system)?;
        let mut budget = crate::NormalizationReferenceBudget::source(
            owner,
            fsl_semantics::model::accounting::ChargePoint::ReferenceResult,
        );
        let bytes = system.model_state_bytes_with_budget(
            state,
            fsl_foundation::IdentityLimits::default(),
            &mut budget,
            self.scope.cancel,
        );
        if let Some(cause) = budget.take_normalization_failure() {
            return Err(ReferenceCodecError::Normalization {
                cause,
                pointer: std::mem::take(&mut self.scope.path),
            });
        }
        let bytes = bytes.map_err(|cause| self.scope.stopped(cause))?;
        let native = self.native_json(&bytes)?;
        self.model_state(&native, false)
    }

    /// Restore an external mapped-state carrier to its exact native JSON
    /// identity. Native state admission and refinement replay still run.
    ///
    /// # Errors
    /// Preserves malformed wire, reference, caller and cancellation errors.
    pub fn restore_model_state(&mut self, value: &Value) -> Result<Value, ReferenceCodecError> {
        self.model_state(value, true)
    }

    fn model_state(&mut self, value: &Value, restore: bool) -> Result<Value, ReferenceCodecError> {
        let fields = [
            "type",
            "semantic",
            "control",
            "queues",
            "roles",
            "observations",
            "bounds",
            "memory",
        ];
        let map = self.scope.closed(value, &fields)?;
        if map["type"].as_str() != Some("simulation-state") {
            return Err(self.scope.fail(ReferenceFailure::MalformedWire));
        }
        let mut out = Map::new();
        for field in fields {
            let item = if field == "type" {
                self.text("simulation-state")?
            } else if field == "semantic" {
                self.at_pointer("/semantic", |s| s.semantic(&map[field], restore))?
            } else {
                self.scope.member(field, |scope| {
                    self.codec.value(&map[field], restore, 0, scope)
                })?
            };
            self.scope.put(&mut out, field, item)?;
        }
        Ok(Value::Object(out))
    }

    fn semantic(&mut self, value: &Value, restore: bool) -> Result<Value, ReferenceCodecError> {
        let map = self.scope.closed(value, &["type", "entries"])?;
        if map["type"].as_str() != Some("map") {
            return Err(self.scope.fail(ReferenceFailure::MalformedWire));
        }
        let rows = map["entries"]
            .as_array()
            .ok_or_else(|| self.scope.fail(ReferenceFailure::MalformedWire))?;
        let mut entries = self.array(rows.len())?;
        for (index, row) in rows.iter().enumerate() {
            entries.push(self.at_index("/entries", index, |s| {
                let row = s.scope.closed(row, &["key", "value"])?;
                let key = s
                    .scope
                    .member("key", |scope| s.codec.value(&row["key"], restore, 0, scope))?;
                let relationships = key.get("type").and_then(Value::as_str) == Some("string")
                    && key
                        .get("value")
                        .and_then(Value::as_str)
                        .is_some_and(|text| text == "relationships");
                // The key has already been copied/compared under the shared
                // typed-value owner before selecting this fixed carrier arm.
                let value = s.at_pointer("/value", |s| {
                    if relationships {
                        s.extents(&row["value"], restore)
                    } else {
                        s.codec.value(&row["value"], restore, 0, &mut s.scope)
                    }
                })?;
                s.record([("key", key), ("value", value)])
            })?);
        }
        sort_values(&mut entries, true, true, &mut self.scope)?;
        let tag = self.text("map")?;
        self.record([("type", tag), ("entries", Value::Array(entries))])
    }

    fn extents(&mut self, value: &Value, restore: bool) -> Result<Value, ReferenceCodecError> {
        let map = self.scope.closed(value, &["type", "entries"])?;
        if map["type"].as_str() != Some("map") {
            return Err(self.scope.fail(ReferenceFailure::MalformedWire));
        }
        let rows = map["entries"]
            .as_array()
            .ok_or_else(|| self.scope.fail(ReferenceFailure::MalformedWire))?;
        let mut entries = self.array(rows.len())?;
        for (index, row) in rows.iter().enumerate() {
            entries.push(self.at_index("/entries", index, |s| {
                let row = s.scope.closed(row, &["key", "value"])?;
                let key = s
                    .scope
                    .member("key", |scope| s.codec.value(&row["key"], restore, 0, scope))?;
                let value = s.at_pointer("/value", |s| {
                    let extent = s.scope.closed(&row["value"], &["complete", "instances"])?;
                    let complete = extent["complete"]
                        .as_bool()
                        .ok_or_else(|| s.scope.fail(ReferenceFailure::MalformedWire))?;
                    let rows = extent["instances"]
                        .as_array()
                        .ok_or_else(|| s.scope.fail(ReferenceFailure::MalformedWire))?;
                    let mut instances = s.array(rows.len())?;
                    for (index, pair) in rows.iter().enumerate() {
                        instances.push(s.at_index("/instances", index, |s| {
                            let pair = s.scope.closed(pair, &["source", "target"])?;
                            let source = s.scope.member("source", |scope| {
                                s.codec.reference(&pair["source"], restore, scope)
                            })?;
                            let target = s.scope.member("target", |scope| {
                                s.codec.reference(&pair["target"], restore, scope)
                            })?;
                            s.record([("source", source), ("target", target)])
                        })?);
                    }
                    sort_values(&mut instances, false, true, &mut s.scope)?;
                    s.record([
                        ("complete", Value::Bool(complete)),
                        ("instances", Value::Array(instances)),
                    ])
                })?;
                s.record([("key", key), ("value", value)])
            })?);
        }
        sort_values(&mut entries, true, true, &mut self.scope)?;
        let tag = self.text("map")?;
        self.record([("type", tag), ("entries", Value::Array(entries))])
    }
}
