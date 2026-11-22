// SPDX-License-Identifier: AGPL-3.0-or-later
//! The package-bound boundary between native and backend reference identities.

use filament_exact::{
    Cancel, EffectiveId, IdentityCopyError, IntegerDecimalBudget, ObjectId, ObjectReference,
};
use filament_semantic_value::model_transition::{InvalidModelTransition, ModelTransition};
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::model::{ModelKeyError, ModelSystem};
use fsl_package::{CheckedPackage, CheckedPackageV2, ReferenceCatalog, ReferenceDeclaration};
use fsl_semantics::model::intake::SelectedModels;
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::population::PopulationBinding;
use serde_json::{Map, Value};
use std::cell::RefCell;
mod json;
mod values;

mod model_over;
mod model_state;

#[cfg(test)]
#[path = "../../examples/config-version/spine_unit.rs"]
#[allow(
    dead_code,
    reason = "the shared model input is used with a scoped reference test unit"
)]
mod reference_fixture;

/// Semantic reasons a reference boundary refuses an input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceFailure {
    /// The closed typed wire grammar was violated.
    MalformedWire,
    /// The catalog does not belong to this runtime package and native model.
    ForeignCatalog,
    /// The package admitted no matching object declaration.
    UnknownDeclaration,
    /// The reference universe does not belong to its admitted object type.
    IncompatibleUniverse,
    /// Hex spelling or decoded native nonempty UTF-8 identity is invalid.
    InvalidIdentity,
    /// No original population selection was retained by the producer.
    MissingPopulation,
    /// The selected population or admitted member does not match the reference.
    WrongPopulation,
    /// The caller's finite nesting policy cannot admit this input.
    DepthLimit,
}

/// A typed boundary refusal with an RFC 6901 locus; resource causes are retained.
#[derive(Debug, thiserror::Error)]
pub enum ReferenceCodecError {
    /// The native scalar owner refused a reserved certificate integer parse.
    #[error("integer input refused at {pointer}: {cause}")]
    IntegerParse {
        /// Original native integer parse or allocation failure.
        cause: filament_exact::IntegerParseError,
        /// Scalar being read.
        pointer: String,
    },
    /// An existing carrier adapter refused its non-reference metadata.
    #[error("carrier output refused at {pointer}: {cause}")]
    Carrier {
        /// Original carrier failure.
        cause: crate::CounterexampleWireError,
        /// Carrier member being written.
        pointer: String,
    },
    /// The exact integer owner refused charged decimal output.
    #[error("integer output refused at {pointer}: {cause}")]
    Decimal {
        /// Original exact integer refusal.
        cause: filament_exact::IntegerDecimalBudgetError,
        /// Scalar being written.
        pointer: String,
    },
    /// The actual normalized declaration owner's cumulative work stopped.
    #[error("reference normalization stopped at {pointer}: {cause:?}")]
    Normalization {
        /// Original normalization work denial and real charge point.
        cause: fsl_semantics::model::accounting::NormalizationIncomplete,
        /// The reference being encoded.
        pointer: String,
    },
    /// A complete native identity copy stopped under the actual caller budget.
    #[error("reference identity copy stopped at {pointer}: {cause:?}")]
    IdentityCopy {
        /// Original typed copy/caller cause, without display-text branching.
        cause: IdentityCopyError<ModelKeyError>,
        /// The reference being constructed.
        pointer: String,
    },
    /// The normalized model universe could not construct its native identity.
    #[error("reference universe construction stopped at {pointer}: {cause:?}")]
    Universe {
        /// The actual model-owned universe refusal.
        cause: fsl_semantics::model::normalize::UniverseIdentityError,
        /// The reference being constructed.
        pointer: String,
    },
    /// The actual model owner's reference admission refused construction.
    #[error("native reference admission refused at {pointer}: {cause:?}")]
    Native {
        /// Original model admission result, without display-text conversion.
        cause: fsl_semantics::model::observation::AdmissionFailure,
        /// The reference being admitted.
        pointer: String,
    },
    /// The native transition owner refused the restored canonical texts.
    #[error("native transition refused at {pointer}: {source}")]
    Transition {
        /// Original typed constructor refusal.
        source: InvalidModelTransition,
        /// The transition boundary being restored.
        pointer: String,
    },
    /// A structural or semantic reference condition.
    #[error("reference conversion {reason:?} at {pointer}")]
    Refused {
        /// Machine-readable condition, never recovered from display text.
        reason: ReferenceFailure,
        /// First offending member.
        pointer: String,
    },
    /// The original runtime budget or cancellation stopped conversion.
    #[error("reference conversion stopped at {pointer}: {source}")]
    Key {
        /// Original typed runtime refusal.
        source: ModelKeyError,
        /// Member whose next operation was denied.
        pointer: String,
    },
}

/// A codec borrowing the catalog and its actual admitted runtime owner.
pub struct ReferenceCodec<'c, 'm, 'r> {
    catalog: &'c ReferenceCatalog<'m>,
    runtime: ReferenceRuntime<'c, 'r>,
}

/// One carrier conversion borrowing the authentic codec and caller's resources.
/// Nested carrier writers share this budget rather than creating fresh limits.
pub struct ReferenceScope<'s, 'c, 'm, 'r> {
    codec: &'s ReferenceCodec<'c, 'm, 'r>,
    scope: Scope<'s, 'm>,
    limits: quire_canonical::Limits,
}

impl ReferenceScope<'_, '_, '_, '_> {
    /// Compare a certificate's original and respelled metadata without a
    /// recursive JSON walk or another output allowance.
    ///
    /// # Errors
    /// Preserves the original owner's work/storage and cancellation refusal.
    pub fn metadata_equal(
        &mut self,
        left: &Value,
        right: &Value,
    ) -> Result<bool, ReferenceCodecError> {
        let mut pending = Vec::new();
        values::push(&mut self.scope, &mut pending, (left, right))?;
        while let Some((left, right)) = pending.pop() {
            self.scope.work(1)?;
            match (left, right) {
                (Value::Null, Value::Null) => {}
                (Value::Bool(left), Value::Bool(right)) if left == right => {}
                (Value::Number(left), Value::Number(right)) if left == right => {}
                (Value::String(left), Value::String(right)) => {
                    if !self.scope.equal_text(left, right)? {
                        return Ok(false);
                    }
                }
                (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
                    let mut pairs = left.iter().zip(right);
                    loop {
                        self.scope.work(1)?;
                        let Some(pair) = pairs.next() else {
                            break;
                        };
                        values::push(&mut self.scope, &mut pending, pair)?;
                    }
                }
                (Value::Object(left), Value::Object(right)) if left.len() == right.len() => {
                    let mut pairs = left.iter().zip(right);
                    loop {
                        self.scope.work(1)?;
                        let Some(((left_key, left), (right_key, right))) = pairs.next() else {
                            break;
                        };
                        if !self.scope.equal_text(left_key, right_key)? {
                            return Ok(false);
                        }
                        values::push(&mut self.scope, &mut pending, (left, right))?;
                    }
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
    pub(crate) fn metadata_state(
        &mut self,
        value: &Value,
    ) -> Result<fsl_eval::simulation::StateKey, ReferenceCodecError> {
        use base64::Engine as _;
        let map = self.scope.closed(value, &["key"])?;
        self.scope.member("key", |scope| {
            let text = map["key"]
                .as_str()
                .ok_or_else(|| scope.fail(ReferenceFailure::MalformedWire))?;
            scope.work(text.len())?;
            let tail = match text.len() % 4 {
                0 => 0,
                2 => 1,
                3 => 2,
                _ => return Err(scope.fail(ReferenceFailure::MalformedWire)),
            };
            let len = (text.len() / 4)
                .checked_mul(3)
                .and_then(|len| len.checked_add(tail))
                .ok_or_else(|| scope.stopped(ModelKeyError::SizeOverflow))?;
            scope.reserve(len)?;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(len)
                .map_err(|_| scope.allocation(len))?;
            bytes.resize(len, 0);
            let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode_slice(text, &mut bytes)
                .map_err(|_| scope.fail(ReferenceFailure::MalformedWire))?;
            bytes.truncate(decoded);
            Ok(fsl_eval::simulation::StateKey::from_canonical_bytes(
                bytes,
            ))
        })
    }
    pub(crate) fn metadata_text(&mut self, value: &Value) -> Result<String, ReferenceCodecError> {
        let text = value
            .as_str()
            .ok_or_else(|| self.scope.fail(ReferenceFailure::MalformedWire))?;
        self.scope.text(text)
    }
    pub(crate) fn metadata_integer(
        &mut self,
        value: &Value,
    ) -> Result<filament_exact::Integer, ReferenceCodecError> {
        let text = value
            .as_str()
            .ok_or_else(|| self.scope.fail(ReferenceFailure::MalformedWire))?;
        self.scope.work(text.len())?;
        let admission = Admission::new(
            &mut *self.scope.budget,
            self.scope.cancel,
            self.scope.normalization_owner,
        );
        let result =
            filament_exact::Integer::parse_reserved(text, &mut DecimalAdmission(&admission));
        let failure = admission.failure.borrow_mut().take();
        drop(admission);
        if let Some(failure) = failure {
            return Err(self.scope.stopped(failure));
        }
        result.map_err(|cause| ReferenceCodecError::IntegerParse {
            cause,
            pointer: std::mem::take(&mut self.scope.path),
        })
    }

    pub(crate) fn metadata_rational(
        &mut self,
        value: &Value,
    ) -> Result<filament_exact::Rational, ReferenceCodecError> {
        let map = self.scope.closed(value, &["numerator", "denominator"])?;
        let numerator = self.at_pointer("/numerator", |s| s.metadata_integer(&map["numerator"]))?;
        let denominator =
            self.at_pointer("/denominator", |s| s.metadata_integer(&map["denominator"]))?;
        if denominator.is_zero() || denominator.is_negative() {
            return Err(self.scope.fail(ReferenceFailure::MalformedWire));
        }
        // The legacy scalar reduction itself has no work hook and remains in
        // the architecture's explicit scalar-loop exclusion. Do not clone
        // arbitrary-precision operands to check canonicality.
        let result = filament_exact::Rational::new(numerator, denominator)
            .map_err(|_| self.scope.fail(ReferenceFailure::MalformedWire))?;
        let numerator = self.at_pointer("/numerator", |s| s.integer(result.numerator()))?;
        let denominator = self.at_pointer("/denominator", |s| s.integer(result.denominator()))?;
        if !self.scope.equal_text(
            numerator.as_str().unwrap_or_default(),
            map["numerator"].as_str().unwrap_or_default(),
        )? || !self.scope.equal_text(
            denominator.as_str().unwrap_or_default(),
            map["denominator"].as_str().unwrap_or_default(),
        )? {
            return Err(self.scope.fail(ReferenceFailure::MalformedWire));
        }
        Ok(result)
    }

    pub(crate) fn storage<T>(&mut self, len: usize) -> Result<Vec<T>, ReferenceCodecError> {
        self.scope.work(len)?;
        let bytes = len
            .checked_mul(size_of::<T>())
            .ok_or_else(|| self.scope.stopped(ModelKeyError::SizeOverflow))?;
        self.scope.reserve(bytes)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(len)
            .map_err(|_| self.scope.allocation(bytes))?;
        Ok(values)
    }

    pub(crate) fn metadata(&mut self, value: &Value) -> Result<Value, ReferenceCodecError> {
        self.scope.json(value, 0)
    }

    pub(crate) fn clock_place_identity(
        &mut self,
        source: &fsl_eval::simulation::model::TimedSystem<'_, '_>,
        index: usize,
    ) -> Result<String, ReferenceCodecError> {
        self.model_runtime(source.system())?;
        let value = source.clock_place_wire(index, self, |reference, s| {
            let bytes = s.scope.with_admission(|admission| {
                admission.encoded(quire_canonical::Limits::new(u64::MAX), |writer| {
                    fsl_eval::simulation::model::encode_reference(reference, writer)
                })
            })?;
            s.native_json(&bytes)
        })?;
        canonical_text(
            &value,
            &mut self.scope,
            quire_canonical::Limits::new(fsl_foundation::IdentityLimits::default().input_bytes),
        )
    }

    /// Write a certificate identity using the producer's existing work owner.
    ///
    /// # Errors
    /// Preserves identity, metadata, resource and cancellation failures.
    pub fn certificate_identity_wire(
        &mut self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        model: &str,
    ) -> Result<Value, ReferenceCodecError> {
        let identity = crate::ObligationIdentity::from_claim_occurrence(claim);
        let node = self.node(identity.node())?;
        let model = self.text(model)?;
        self.record([
            ("node_id", node),
            ("model", model),
            ("ordinal", Value::from(identity.ordinal())),
        ])
    }
    pub(crate) fn model_runtime(
        &mut self,
        system: &ModelSystem<'_>,
    ) -> Result<(), ReferenceCodecError> {
        match &self.codec.runtime {
            ReferenceRuntime::Model(bound) if std::ptr::eq(*bound, system) => Ok(()),
            _ => Err(self.scope.fail(ReferenceFailure::ForeignCatalog)),
        }
    }
    pub(crate) fn carrier_refusal(&mut self, member: &'static str) -> ReferenceCodecError {
        ReferenceCodecError::Carrier {
            cause: crate::CounterexampleWireError::Unspelled(member),
            pointer: std::mem::take(&mut self.scope.path),
        }
    }

    pub(crate) fn work(&mut self, units: usize) -> Result<(), ReferenceCodecError> {
        self.scope.work(units)
    }

    pub(crate) fn append(
        &mut self,
        values: &mut Vec<Value>,
        value: Value,
    ) -> Result<(), ReferenceCodecError> {
        self.scope.work(1)?;
        if values.len() == values.capacity() {
            let capacity = values
                .len()
                .checked_add(1)
                .ok_or_else(|| self.scope.stopped(ModelKeyError::SizeOverflow))?;
            let bytes = capacity
                .checked_mul(size_of::<Value>())
                .ok_or_else(|| self.scope.stopped(ModelKeyError::SizeOverflow))?;
            self.scope.reserve(bytes)?;
            values
                .try_reserve_exact(1)
                .map_err(|_| self.scope.allocation(bytes))?;
        }
        values.push(value);
        Ok(())
    }

    // This leaf's Encode implementation is borrowed. Protocol identities
    // build an Enc tree in Encode and must use a separate borrowed projection.
    pub(crate) fn fairness_wire(
        &mut self,
        value: &filament_semantic_value::fairness::FairnessConstraint,
    ) -> Result<Value, ReferenceCodecError> {
        use quire_canonical::Encode as _;
        let admission = Admission::new(
            &mut *self.scope.budget,
            self.scope.cancel,
            self.scope.normalization_owner,
        );
        let result = admission.encoded(self.limits, |writer| value.encode_into(writer));
        let bytes = admission
            .finish(result)
            .map_err(|error| self.scope.stopped(error))?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| self.scope.fail(ReferenceFailure::MalformedWire))?;
        parse_text(text, &mut self.scope)
    }

    pub(crate) fn typed_value(&mut self, value: &Value) -> Result<Value, ReferenceCodecError> {
        self.codec.value(value, false, 0, &mut self.scope)
    }

    pub(crate) fn native_json(&mut self, bytes: &[u8]) -> Result<Value, ReferenceCodecError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| self.scope.fail(ReferenceFailure::MalformedWire))?;
        parse_text(text, &mut self.scope)
    }

    pub(crate) fn native_value(
        &mut self,
        value: &filament_exact::Value,
        package: &fsl_package::CheckedPackage,
        owner: &fsl_semantics::model::intake::SelectedModels,
    ) -> Result<Value, ReferenceCodecError> {
        let mut budget = crate::NormalizationReferenceBudget::source(
            owner,
            fsl_semantics::model::accounting::ChargePoint::ReferenceResult,
        );
        let types = package.graph().scope();
        let written = fsl_eval::simulation::model::run_value_bytes(
            value,
            types.types(),
            types.enums(),
            types.variant_types(),
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
        let written = written.map_err(|error| self.scope.stopped(error))?;
        let native = self.native_json(&written)?;
        self.typed_value(&native)
    }

    pub(crate) fn native_state(
        &mut self,
        state: &fsl_eval::simulation::model::ModelSimState,
        system: &fsl_eval::simulation::model::ModelSystem<'_>,
        owner: &fsl_semantics::model::intake::SelectedModels,
    ) -> Result<Value, ReferenceCodecError> {
        let key = self.native_state_key(state, system, owner)?;
        self.state(&key)
    }

    pub(crate) fn native_state_key(
        &mut self,
        state: &fsl_eval::simulation::model::ModelSimState,
        system: &ModelSystem<'_>,
        owner: &SelectedModels,
    ) -> Result<fsl_eval::simulation::StateKey, ReferenceCodecError> {
        self.model_runtime(system)?;
        let mut budget = crate::NormalizationReferenceBudget::source(
            owner,
            fsl_semantics::model::accounting::ChargePoint::ReferenceResult,
        );
        let bytes = system.state_key_bytes_with_budget(
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
        let bytes = bytes.map_err(|error| self.scope.stopped(error))?;
        Ok(fsl_eval::simulation::StateKey::from_canonical_bytes(bytes))
    }

    pub(crate) fn declaration_text(
        &mut self,
        key: &filament_checked::key::DeclarationKey,
    ) -> Result<String, ReferenceCodecError> {
        let package = self.text(&key.package)?;
        let node = self.text(&key.node)?;
        let value = self.record([("package", package), ("node", node)])?;
        canonical_text(&value, &mut self.scope, self.limits)
    }

    pub(crate) fn state(
        &mut self,
        key: &fsl_eval::simulation::StateKey,
    ) -> Result<Value, ReferenceCodecError> {
        let key = self.opaque_key(key)?;
        self.record([("key", key)])
    }
    pub(crate) fn put(
        &mut self,
        object: &mut Value,
        name: &str,
        value: Value,
    ) -> Result<(), ReferenceCodecError> {
        let map = object
            .as_object_mut()
            .ok_or_else(|| self.scope.fail(ReferenceFailure::MalformedWire))?;
        self.scope.put(map, name, value)
    }

    pub(crate) fn display(
        &mut self,
        value: &impl std::fmt::Display,
    ) -> Result<Value, ReferenceCodecError> {
        self.display_text(value).map(Value::String)
    }

    pub(crate) fn display_text(
        &mut self,
        value: &impl std::fmt::Display,
    ) -> Result<String, ReferenceCodecError> {
        struct Text<'a, 'b, 'n> {
            scope: &'a mut Scope<'b, 'n>,
            text: String,
            failure: Option<ReferenceCodecError>,
        }
        impl std::fmt::Write for Text<'_, '_, '_> {
            fn write_str(&mut self, text: &str) -> std::fmt::Result {
                let result = self
                    .scope
                    .work(text.len())
                    .and_then(|()| self.scope.reserve(text.len()))
                    .and_then(|()| {
                        self.text
                            .try_reserve_exact(text.len())
                            .map_err(|_| self.scope.allocation(text.len()))
                    });
                match result {
                    Ok(()) => {
                        self.text.push_str(text);
                        Ok(())
                    }
                    Err(error) => {
                        self.failure = Some(error);
                        Err(std::fmt::Error)
                    }
                }
            }
        }
        let mut writer = Text {
            scope: &mut self.scope,
            text: String::new(),
            failure: None,
        };
        let result = std::fmt::write(&mut writer, format_args!("{value}"));
        if let Some(error) = writer.failure {
            return Err(error);
        }
        if result.is_err() {
            return Err(writer.scope.fail(ReferenceFailure::MalformedWire));
        }
        Ok(writer.text)
    }

    pub(crate) fn opaque_key(
        &mut self,
        key: &fsl_eval::simulation::StateKey,
    ) -> Result<Value, ReferenceCodecError> {
        self.opaque_bytes(key.as_bytes())
    }

    pub(crate) fn opaque_bytes(&mut self, bytes: &[u8]) -> Result<Value, ReferenceCodecError> {
        use base64::Engine as _;
        // The same URL_SAFE_NO_PAD encoding as TimedStateWire; admit and
        // fallibly allocate its complete output before encoding opaque bytes.
        let encoded = bytes
            .len()
            .checked_add(2)
            .and_then(|n| n.checked_div(3))
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| self.scope.stopped(ModelKeyError::SizeOverflow))?;
        self.scope.work(bytes.len())?;
        self.scope.work(encoded)?;
        self.scope.reserve(encoded)?;
        let mut text = String::new();
        text.try_reserve_exact(encoded)
            .map_err(|_| self.scope.allocation(encoded))?;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode_string(bytes, &mut text);
        Ok(Value::String(text))
    }

    pub(crate) fn text(&mut self, value: &str) -> Result<Value, ReferenceCodecError> {
        self.scope.text(value).map(Value::String)
    }

    pub(crate) fn record<const N: usize>(
        &mut self,
        fields: [(&str, Value); N],
    ) -> Result<Value, ReferenceCodecError> {
        self.scope.work(N)?;
        let mut map = Map::new();
        for (key, value) in fields {
            self.scope.put(&mut map, key, value)?;
        }
        Ok(Value::Object(map))
    }

    pub(crate) fn array(&mut self, len: usize) -> Result<Vec<Value>, ReferenceCodecError> {
        self.scope.work(len)?;
        self.scope.array(len)
    }

    pub(crate) fn integer(
        &mut self,
        value: &filament_exact::Integer,
    ) -> Result<Value, ReferenceCodecError> {
        let admission = Admission::new(
            &mut *self.scope.budget,
            self.scope.cancel,
            self.scope.normalization_owner,
        );
        let result =
            value.try_to_decimal_string_with_budget(usize::MAX, &mut DecimalAdmission(&admission));
        let failure = admission.failure.borrow_mut().take();
        drop(admission);
        if let Some(failure) = failure {
            return Err(self.scope.stopped(failure));
        }
        result
            .map(Value::String)
            .map_err(|cause| ReferenceCodecError::Decimal {
                cause,
                pointer: std::mem::take(&mut self.scope.path),
            })
    }

    pub(crate) fn rational(
        &mut self,
        value: &filament_exact::Rational,
    ) -> Result<Value, ReferenceCodecError> {
        let denominator = self.at_pointer("/denominator", |s| s.integer(value.denominator()))?;
        let numerator = self.at_pointer("/numerator", |s| s.integer(value.numerator()))?;
        self.record([("denominator", denominator), ("numerator", numerator)])
    }

    pub(crate) fn node(
        &mut self,
        value: &filament_exact::NodeKey,
    ) -> Result<Value, ReferenceCodecError> {
        let admission = Admission::new(
            &mut *self.scope.budget,
            self.scope.cancel,
            self.scope.normalization_owner,
        );
        let result = admission.node_json(value);
        admission
            .finish(result)
            .map_err(|error| self.scope.stopped(error))
    }

    /// Convert a reference through this carrier's retained runtime and budget.
    ///
    /// # Errors
    /// Retains identity, resource, cancellation and pointer failures.
    pub fn encode_reference(
        &mut self,
        value: &ObjectReference,
    ) -> Result<Value, ReferenceCodecError> {
        self.codec.encode_reference_in(value, &mut self.scope)
    }

    /// Run one nested carrier operation at its existing RFC 6901 prefix.
    /// Pointer text is admitted before storage is grown; the prior prefix is
    /// restored on success and failure.
    ///
    /// # Errors
    /// Preserves the operation's typed failure and pointer-storage denials.
    pub fn at_pointer<T, E: From<ReferenceCodecError>>(
        &mut self,
        pointer: &str,
        action: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        // Carrier pointers are relative to their enclosing carrier. Preserve
        // the outer task_product/edge prefix through nested restoration.
        let before = self.scope.path.len();
        self.scope.work(pointer.len())?;
        self.scope.reserve(pointer.len())?;
        self.scope
            .path
            .try_reserve_exact(pointer.len())
            .map_err(|_| self.scope.allocation(pointer.len()))?;
        self.scope.path.push_str(pointer);
        let result = action(self);
        self.scope.path.truncate(before);
        result
    }

    /// Run an array element under a charged canonical index at `pointer`.
    ///
    /// # Errors
    /// Preserves nested operation, metadata, resource and cancellation failures.
    pub fn at_index<T, E: From<ReferenceCodecError>>(
        &mut self,
        pointer: &str,
        index: usize,
        action: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        self.at_pointer(pointer, |s| {
            let index = s.display_text(&index)?;
            s.at_pointer("/", |s| s.at_pointer(&index, action))
        })
    }

    /// Encode both source-transition arms of a task template with one catalog.
    /// State keys and native template identities are never changed.
    ///
    /// # Errors
    /// Retains reference failures, resource denial and cancellation.
    pub fn encode_task_template(
        &mut self,
        template: &crate::TaskTransitionTemplate,
    ) -> Result<Value, ReferenceCodecError> {
        let mut out = Map::new();
        for (name, text) in [
            ("type", "task-transition-template"),
            (
                "action",
                crate::certificate::task_product::action_name(template.action),
            ),
            ("taskset", template.task_set.as_str()),
            ("task", template.task.as_str()),
        ] {
            let value = Value::String(self.scope.member(name, |s| s.text(text))?);
            self.scope.put(&mut out, name, value)?;
        }
        for (name, number) in [("job_rank", template.job_rank), ("slot", template.slot)] {
            self.scope.member(name, |s| s.count(number))?;
            self.scope.put(&mut out, name, Value::from(number))?;
        }
        if let Some(priority) = template.priority {
            self.scope.member("priority", |s| s.count(priority))?;
            self.scope
                .put(&mut out, "priority", Value::from(priority))?;
        }
        let release = self.task_transition_at("release", &template.release)?;
        self.scope.put(&mut out, "release", release)?;
        let source = match &template.source_transition {
            Some(source) => self.task_transition_at("source_transition", source)?,
            None => Value::Null,
        };
        self.scope.put(&mut out, "source_transition", source)?;
        Ok(Value::Object(out))
    }

    fn task_transition_at(
        &mut self,
        name: &str,
        transition: &ModelTransition,
    ) -> Result<Value, ReferenceCodecError> {
        self.scope.member(name, |scope| {
            let mut wire = self.codec.encode_transition_in(transition, scope)?;
            let operation = wire
                .get_mut("operation")
                .ok_or_else(|| scope.fail(ReferenceFailure::MalformedWire))?;
            let spelling = crate::outcome::task_source_operation(operation, transition.operation());
            let text = scope.member("operation", |s| s.text(spelling))?;
            *operation = Value::String(text);
            Ok(wire)
        })
    }

    /// Restore both task source arms before constructing the native template.
    /// This checks wire identity, not task membership or enabledness.
    ///
    /// # Errors
    /// Refuses malformed metadata and retains all reference/resource failures.
    pub fn restore_task_template(
        &mut self,
        value: &Value,
    ) -> Result<crate::TaskTransitionTemplate, ReferenceCodecError> {
        let fields: &[&str] = if value.get("priority").is_some() {
            &[
                "type",
                "action",
                "taskset",
                "task",
                "job_rank",
                "slot",
                "release",
                "source_transition",
                "priority",
            ]
        } else {
            &[
                "type",
                "action",
                "taskset",
                "task",
                "job_rank",
                "slot",
                "release",
                "source_transition",
            ]
        };
        let map = self.scope.closed(value, fields)?;
        self.scope.member("type", |s| {
            let tag = s.string(&map["type"])?;
            if s.equal_text(tag, "task-transition-template")? {
                Ok(())
            } else {
                Err(s.fail(ReferenceFailure::MalformedWire))
            }
        })?;
        let action = self.scope.member("action", |s| {
            let text = s.string(&map["action"])?;
            for (name, action) in crate::certificate::task_product::ACTIONS {
                if s.equal_text(text, name)? {
                    return Ok(action);
                }
            }
            Err(s.fail(ReferenceFailure::MalformedWire))
        })?;
        let task_set = self.scope.member("taskset", |s| {
            let text = s.string(&map["taskset"])?;
            s.text(text)
        })?;
        let task = self.scope.member("task", |s| {
            let text = s.string(&map["task"])?;
            s.text(text)
        })?;
        let job_rank = self
            .scope
            .member("job_rank", |s| s.read_count(&map["job_rank"]))?;
        let slot = self.scope.member("slot", |s| s.read_count(&map["slot"]))?;
        let priority = map
            .get("priority")
            .map(|value| self.scope.member("priority", |s| s.read_count(value)))
            .transpose()?;
        let release = self.restore_task_at("release", &map["release"])?;
        let source_transition = if map["source_transition"].is_null() {
            None
        } else {
            Some(self.restore_task_at("source_transition", &map["source_transition"])?)
        };
        Ok(crate::TaskTransitionTemplate {
            action,
            task_set,
            task,
            job_rank,
            slot,
            release,
            source_transition,
            priority,
        })
    }

    fn restore_task_at(
        &mut self,
        name: &str,
        value: &Value,
    ) -> Result<ModelTransition, ReferenceCodecError> {
        self.at_pointer("/", |s| {
            s.at_pointer(name, |s| s.restore_task_transition(value))
        })
    }

    /// Encode a native transition with the generic operation identity intact.
    ///
    /// # Errors
    /// Retains the codec's typed identity, resource and cancellation failures.
    pub fn encode_transition(
        &mut self,
        value: &ModelTransition,
    ) -> Result<Value, ReferenceCodecError> {
        self.codec.encode_transition_in(value, &mut self.scope)
    }

    /// Restore native canonical values before transition construction.
    ///
    /// # Errors
    /// Refuses malformed identities and retains resource/cancellation failures.
    pub fn restore_transition(
        &mut self,
        value: &Value,
    ) -> Result<ModelTransition, ReferenceCodecError> {
        self.codec
            .restore_transition_in(value, &mut self.scope, self.limits)
    }

    /// Encode the task-source arm using its existing operation-string adapter.
    ///
    /// # Errors
    /// Retains reference and caller resource/cancellation failures.
    pub fn encode_task_transition(
        &mut self,
        value: &ModelTransition,
    ) -> Result<Value, ReferenceCodecError> {
        let mut wire = self.encode_transition(value)?;
        let operation = wire
            .get_mut("operation")
            .ok_or_else(|| self.scope.fail(ReferenceFailure::MalformedWire))?;
        let spelling = crate::outcome::task_source_operation(operation, value.operation());
        let text = self.scope.member("operation", |s| s.text(spelling))?;
        *operation = Value::String(text);
        Ok(wire)
    }

    /// Restore a task-source arm; its operation must be a string.
    ///
    /// # Errors
    /// Refuses other operation arms and retains all reference refusals.
    pub fn restore_task_transition(
        &mut self,
        value: &Value,
    ) -> Result<ModelTransition, ReferenceCodecError> {
        let spelling = self
            .scope
            .member("operation", |s| s.string(&value["operation"]))?;
        let mut operation = None;
        if let ReferenceRuntime::Model(system) = &self.codec.runtime {
            let mut identities = system.operation_identities();
            loop {
                self.scope.work(1)?;
                let Some(text) = identities.next() else {
                    break;
                };
                let native = parse_text(text, &mut self.scope)?;
                let projected = crate::outcome::task_source_operation(&native, text);
                if self.scope.equal_text(spelling, projected)? {
                    if let Some(previous) = operation.as_deref() {
                        if !self.scope.equal_text(previous, text)? {
                            return Err(self.scope.fail(ReferenceFailure::UnknownDeclaration));
                        }
                    } else {
                        operation = Some(self.scope.text(text)?);
                    }
                }
            }
        }
        self.codec
            .restore_transition_with_operation(value, &mut self.scope, self.limits, operation)
    }
}

enum ReferenceRuntime<'c, 'r> {
    Model(&'c ModelSystem<'r>),
    Call(&'c ObjectEnvironment),
}

impl fsl_eval::simulation::protocol::ProtocolWireSink for ReferenceScope<'_, '_, '_, '_> {
    type Error = ReferenceCodecError;
    fn work(&mut self, units: usize) -> Result<(), Self::Error> {
        self.scope.work(units)
    }
    fn depth(&mut self, depth: usize) -> Result<(), Self::Error> {
        self.scope.work(1)?;
        if depth > self.scope.max_depth.min(64) {
            return Err(self.scope.fail(ReferenceFailure::DepthLimit));
        }
        Ok(())
    }
    fn string(&mut self, text: &str) -> Result<Value, Self::Error> {
        self.text(text)
    }
    fn unsigned(&mut self, value: u64) -> Result<Value, Self::Error> {
        self.display(&value)
    }
    fn object(&mut self) -> Result<Value, Self::Error> {
        self.scope.work(1)?;
        self.record([])
    }
    fn array(&mut self, len: usize) -> Result<Value, Self::Error> {
        ReferenceScope::array(self, len).map(Value::Array)
    }
    fn field(
        &mut self,
        out: &mut Value,
        name: &str,
        value: impl FnOnce(&mut Self) -> Result<Value, Self::Error>,
    ) -> Result<(), Self::Error> {
        let map = out
            .as_object()
            .ok_or_else(|| self.scope.fail(ReferenceFailure::MalformedWire))?;
        let comparisons = name
            .len()
            .checked_mul(map.len())
            .ok_or_else(|| self.scope.stopped(ModelKeyError::SizeOverflow))?;
        self.scope.work(comparisons)?;
        if map.contains_key(name) {
            return Err(self.scope.fail(ReferenceFailure::MalformedWire));
        }
        let before = self.scope.path.len();
        let additional = name
            .len()
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| self.scope.stopped(ModelKeyError::SizeOverflow))?;
        self.scope.work(name.len())?;
        self.scope.reserve(additional)?;
        self.scope
            .path
            .try_reserve_exact(additional)
            .map_err(|_| self.scope.allocation(additional))?;
        self.scope.path.push('/');
        for c in name.chars() {
            match c {
                '~' => self.scope.path.push_str("~0"),
                '/' => self.scope.path.push_str("~1"),
                c => self.scope.path.push(c),
            }
        }
        let result = value(self).and_then(|value| self.put(out, name, value));
        self.scope.path.truncate(before);
        result
    }
    fn element(
        &mut self,
        out: &mut Value,
        index: usize,
        value: impl FnOnce(&mut Self) -> Result<Value, Self::Error>,
    ) -> Result<(), Self::Error> {
        let index = self.display_text(&index)?;
        let value = self.at_pointer("/", |s| s.at_pointer(&index, value))?;
        let values = out
            .as_array_mut()
            .ok_or_else(|| self.scope.fail(ReferenceFailure::MalformedWire))?;
        self.append(values, value)
    }
    fn reference(&mut self, value: &ObjectReference) -> Result<Value, Self::Error> {
        self.encode_reference(value)
    }
    fn reference_parts(
        &mut self,
        universe: &filament_exact::NodeKey,
        object_type: &EffectiveId,
        identity: &str,
    ) -> Result<Value, Self::Error> {
        let previous = self.scope.normalization_owner;
        let result = (|| {
            let mut found = false;
            for declaration in self.codec.catalog.declarations() {
                self.scope.normalization_owner = declaration.normalization_owner();
                if fsl_semantics::model::state::StateModel::reference_identity_equal_with_work(
                    object_type,
                    declaration.effective(),
                    &mut |units| {
                        let units = usize::try_from(units)
                            .map_err(|_| self.scope.stopped(ModelKeyError::SizeOverflow))?;
                        self.scope.work(units)
                    },
                )? {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(self.scope.fail(ReferenceFailure::UnknownDeclaration));
            }
            let universe = self.node(universe)?;
            let object_type = self.scope.effective_json(object_type)?;
            let identity = self.text(identity)?;
            let tag = self.text("reference")?;
            let native = self.record([
                ("type", tag),
                ("universe", universe),
                ("object_type", object_type),
                ("identity", identity),
            ])?;
            self.codec.reference(&native, false, &mut self.scope)
        })();
        self.scope.normalization_owner = previous;
        result
    }
    fn node(&mut self, value: &filament_exact::NodeKey) -> Result<Value, Self::Error> {
        ReferenceScope::node(self, value)
    }
    fn effective(&mut self, value: &EffectiveId) -> Result<Value, Self::Error> {
        self.scope.effective_json(value)
    }
    fn declaration(
        &mut self,
        value: &filament_checked::key::DeclarationKey,
    ) -> Result<Value, Self::Error> {
        self.declaration_text(value).map(Value::String)
    }
    fn order_map(&mut self, entries: &mut Value) -> Result<(), Self::Error> {
        let values = entries
            .as_array_mut()
            .ok_or_else(|| self.scope.fail(ReferenceFailure::MalformedWire))?;
        sort_values(values, true, false, &mut self.scope)
    }
    fn unsupported(&mut self, what: &'static str) -> Self::Error {
        self.carrier_refusal(what)
    }
}

struct Scope<'a, 'n> {
    budget: &'a mut dyn IntegerDecimalBudget,
    cancel: &'a Cancel,
    path: String,
    max_depth: usize,
    normalization_owner: Option<&'n fsl_semantics::model::intake::NormalizedModel>,
}

impl Scope<'_, '_> {
    fn count(&mut self, number: u64) -> Result<(), ReferenceCodecError> {
        self.work(1)?;
        if number > 9_007_199_254_740_991 {
            return Err(self.fail(ReferenceFailure::MalformedWire));
        }
        Ok(())
    }

    fn read_count(&mut self, value: &Value) -> Result<u64, ReferenceCodecError> {
        let number = value
            .as_u64()
            .ok_or_else(|| self.fail(ReferenceFailure::MalformedWire))?;
        self.count(number)?;
        Ok(number)
    }
    fn json(&mut self, value: &Value, depth: usize) -> Result<Value, ReferenceCodecError> {
        if depth > self.max_depth {
            return Err(self.fail(ReferenceFailure::DepthLimit));
        }
        let remaining = self.max_depth - depth;
        match self.with_admission(|admission| admission.borrowed_json(value, remaining)) {
            Err(ReferenceCodecError::Key {
                source: ModelKeyError::Unsupported(json::DEPTH),
                ..
            }) => Err(self.fail(ReferenceFailure::DepthLimit)),
            other => other,
        }
    }
    fn same_json(&mut self, left: &Value, right: &Value) -> Result<bool, ReferenceCodecError> {
        let mut pending = Vec::new();
        values::push(self, &mut pending, (left, right))?;
        while let Some((left, right)) = pending.pop() {
            self.work(1)?;
            match (left, right) {
                (Value::String(left), Value::String(right)) => {
                    if !self.equal_text(left, right)? {
                        return Ok(false);
                    }
                }
                (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
                    for pair in left.iter().zip(right).rev() {
                        values::push(self, &mut pending, pair)?;
                    }
                }
                (Value::Object(left), Value::Object(right)) if left.len() == right.len() => {
                    for ((left_name, left), (right_name, right)) in left.iter().zip(right).rev() {
                        if !self.equal_text(left_name, right_name)? {
                            return Ok(false);
                        }
                        values::push(self, &mut pending, (left, right))?;
                    }
                }
                (Value::Null, Value::Null) => {}
                (Value::Bool(left), Value::Bool(right)) if left == right => {}
                (Value::Number(left), Value::Number(right)) if left == right => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    fn validate_effective(&mut self, mut value: &Value) -> Result<(), ReferenceCodecError> {
        for _ in 0..=self.max_depth {
            self.work(1)?;
            let map = self.closed(value, &["original", "owner_effective_type"])?;
            let key = self.closed(&map["original"], &["package", "node"])?;
            let package = self.string(&key["package"])?;
            let node = self.string(&key["node"])?;
            self.work(package.len())?;
            self.work(node.len())?;
            if package.is_empty() || node.is_empty() {
                return Err(self.fail(ReferenceFailure::MalformedWire));
            }
            value = &map["owner_effective_type"];
            if value.is_null() {
                return Ok(());
            }
        }
        Err(self.fail(ReferenceFailure::DepthLimit))
    }
    fn fail(&mut self, reason: ReferenceFailure) -> ReferenceCodecError {
        ReferenceCodecError::Refused {
            reason,
            pointer: std::mem::take(&mut self.path),
        }
    }

    fn stopped(&mut self, source: ModelKeyError) -> ReferenceCodecError {
        ReferenceCodecError::Key {
            source,
            pointer: std::mem::take(&mut self.path),
        }
    }

    fn allocation(&mut self, requested: usize) -> ReferenceCodecError {
        self.stopped(ModelKeyError::Encoding(
            fsl_eval::simulation::EncodingRefusal::Allocation { requested },
        ))
    }

    fn poll(&mut self) -> Result<(), ReferenceCodecError> {
        self.cancel.poll();
        if let Some(cause) = self.cancel.tripped() {
            return Err(self.stopped(ModelKeyError::Cancelled(cause)));
        }
        Ok(())
    }

    fn work(&mut self, units: usize) -> Result<(), ReferenceCodecError> {
        self.poll()?;
        let units = u64::try_from(units).map_err(|_| self.stopped(ModelKeyError::SizeOverflow))?;
        if let Some(owner) = self.normalization_owner {
            owner
                .charge_reference_work(
                    fsl_semantics::model::accounting::ChargePoint::ReferenceResult,
                    units,
                )
                .map_err(|cause| ReferenceCodecError::Normalization {
                    cause,
                    pointer: std::mem::take(&mut self.path),
                })?;
        } else {
            self.budget
                .work(units)
                .map_err(|_| self.stopped(ModelKeyError::Denied))?;
        }
        self.poll()
    }

    fn with_admission<T>(
        &mut self,
        action: impl FnOnce(&Admission<'_>) -> Result<T, ModelKeyError>,
    ) -> Result<T, ReferenceCodecError> {
        let admission = Admission::new(&mut *self.budget, self.cancel, self.normalization_owner);
        let result = action(&admission);
        let normalization = admission.normalization_failure.borrow_mut().take();
        let result = admission.finish(result);
        if let Some(cause) = normalization {
            return Err(ReferenceCodecError::Normalization {
                cause,
                pointer: std::mem::take(&mut self.path),
            });
        }
        result.map_err(|error| self.stopped(error))
    }

    fn reserve(&mut self, bytes: usize) -> Result<(), ReferenceCodecError> {
        self.reserve_key(bytes).map_err(|error| self.stopped(error))
    }

    fn reserve_key(&mut self, bytes: usize) -> Result<(), ModelKeyError> {
        self.cancel.poll();
        if let Some(cause) = self.cancel.tripped() {
            return Err(ModelKeyError::Cancelled(cause));
        }
        self.budget
            .reserve(bytes)
            .map_err(|_| ModelKeyError::Denied)?;
        self.cancel.poll();
        match self.cancel.tripped() {
            Some(cause) => Err(ModelKeyError::Cancelled(cause)),
            None => Ok(()),
        }
    }

    fn text(&mut self, text: &str) -> Result<String, ReferenceCodecError> {
        self.work(text.len())?;
        self.reserve(text.len())?;
        let mut owned = String::new();
        owned
            .try_reserve_exact(text.len())
            .map_err(|_| self.allocation(text.len()))?;
        owned.push_str(text);
        Ok(owned)
    }

    fn member<T>(
        &mut self,
        name: &str,
        f: impl FnOnce(&mut Self) -> Result<T, ReferenceCodecError>,
    ) -> Result<T, ReferenceCodecError> {
        let before = self.path.len();
        let additional = name
            .len()
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| self.stopped(ModelKeyError::SizeOverflow))?;
        self.work(name.len())?;
        self.reserve(additional)?;
        self.path
            .try_reserve_exact(additional)
            .map_err(|_| self.allocation(additional))?;
        self.path.push('/');
        for c in name.chars() {
            match c {
                '~' => self.path.push_str("~0"),
                '/' => self.path.push_str("~1"),
                c => self.path.push(c),
            }
        }
        let result = f(self);
        if result.is_ok() {
            self.path.truncate(before);
        }
        result
    }

    fn array(&mut self, len: usize) -> Result<Vec<Value>, ReferenceCodecError> {
        let bytes = len
            .checked_mul(size_of::<Value>())
            .ok_or_else(|| self.stopped(ModelKeyError::SizeOverflow))?;
        self.reserve(bytes)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(len)
            .map_err(|_| self.allocation(bytes))?;
        Ok(values)
    }

    fn put(
        &mut self,
        map: &mut Map<String, Value>,
        key: &str,
        value: Value,
    ) -> Result<(), ReferenceCodecError> {
        let mut value = json::DeepJson(Some(value));
        // serde_json's map has no fallible reserve API. Account its node and
        // key before insertion, including tree linkage capacity.
        self.reserve(size_of::<(String, Value)>().saturating_mul(4))?;
        let key = self.text(key)?;
        map.insert(key, value.0.take().expect("admitted member value"));
        Ok(())
    }

    fn equal_text(&mut self, left: &str, right: &str) -> Result<bool, ReferenceCodecError> {
        self.work(left.len().max(right.len()))?;
        Ok(left == right)
    }

    fn closed<'v>(
        &mut self,
        value: &'v Value,
        fields: &[&str],
    ) -> Result<&'v Map<String, Value>, ReferenceCodecError> {
        let Some(map) = value.as_object() else {
            return Err(self.fail(ReferenceFailure::MalformedWire));
        };
        self.work(map.len())?;
        if map.len() != fields.len() {
            return Err(self.fail(ReferenceFailure::MalformedWire));
        }
        for name in map.keys() {
            let comparisons = name
                .len()
                .checked_mul(fields.len())
                .ok_or_else(|| self.stopped(ModelKeyError::SizeOverflow))?;
            self.work(comparisons)?;
        }
        if fields.iter().any(|name| !map.contains_key(*name)) {
            return Err(self.fail(ReferenceFailure::MalformedWire));
        }
        Ok(map)
    }

    fn string<'v>(&mut self, value: &'v Value) -> Result<&'v str, ReferenceCodecError> {
        value
            .as_str()
            .ok_or_else(|| self.fail(ReferenceFailure::MalformedWire))
    }

    fn hex_encode(&mut self, input: &str) -> Result<String, ReferenceCodecError> {
        if input.is_empty() {
            return Err(self.fail(ReferenceFailure::InvalidIdentity));
        }
        let len = input
            .len()
            .checked_mul(2)
            .ok_or_else(|| self.stopped(ModelKeyError::SizeOverflow))?;
        self.work(len)?;
        self.reserve(len)?;
        let mut out = String::new();
        out.try_reserve_exact(len)
            .map_err(|_| self.allocation(len))?;
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for byte in input.bytes() {
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 15)]));
        }
        Ok(out)
    }

    fn hex_decode(&mut self, input: &str) -> Result<String, ReferenceCodecError> {
        self.work(input.len())?;
        if input.is_empty()
            || input.len() % 2 != 0
            || !input
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(self.fail(ReferenceFailure::InvalidIdentity));
        }
        self.reserve(input.len() / 2)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(input.len() / 2)
            .map_err(|_| self.allocation(input.len() / 2))?;
        let nibble = |b: u8| {
            if b.is_ascii_digit() {
                b - b'0'
            } else {
                b - b'a' + 10
            }
        };
        for pair in input.as_bytes().chunks_exact(2) {
            bytes.push((nibble(pair[0]) << 4) | nibble(pair[1]));
        }
        String::from_utf8(bytes).map_err(|_| self.fail(ReferenceFailure::InvalidIdentity))
    }

    fn effective_matches(
        &mut self,
        value: &Value,
        effective: &EffectiveId,
    ) -> Result<bool, ReferenceCodecError> {
        let mut value = value;
        for original in effective.owner_chain().iter().rev() {
            self.work(1)?;
            let map = self.closed(value, &["original", "owner_effective_type"])?;
            let key = self.closed(&map["original"], &["package", "node"])?;
            let package = self.string(&key["package"])?;
            let node = self.string(&key["node"])?;
            if !self.equal_text(package, original.package())?
                || !self.equal_text(node, original.node())?
            {
                return Ok(false);
            }
            value = &map["owner_effective_type"];
        }
        Ok(value.is_null())
    }

    fn declaration_json(
        &mut self,
        declaration: &ReferenceDeclaration<'_>,
    ) -> Result<Value, ReferenceCodecError> {
        let node = declaration.node();
        // The authenticated catalog has checked the entire flat object-type
        // declaration shape; constructing this JSON never unfolds another node.
        let mut path = self.array(node.path.len())?;
        for segment in &node.path {
            let filament_contract_model::CheckedNodePathSegment::Name(name) = segment else {
                return Err(self.fail(ReferenceFailure::UnknownDeclaration));
            };
            path.push(Value::String(self.text(name)?));
        }
        let mut out = Map::new();
        let package = Value::String(self.text(&node.package)?);
        self.put(&mut out, "package", package)?;
        self.put(&mut out, "path", Value::Array(path))?;
        self.put(&mut out, "ordinal", Value::from(node.ordinal))?;
        Ok(Value::Object(out))
    }

    fn effective_json(&mut self, effective: &EffectiveId) -> Result<Value, ReferenceCodecError> {
        if effective.owner_chain().len() > self.max_depth {
            return Err(self.fail(ReferenceFailure::DepthLimit));
        }
        let mut owner = Value::Null;
        for original in effective.owner_chain() {
            self.work(1)?;
            let mut key = Map::new();
            let package = Value::String(self.text(original.package())?);
            let node = Value::String(self.text(original.node())?);
            self.put(&mut key, "package", package)?;
            self.put(&mut key, "node", node)?;
            let mut map = Map::new();
            self.put(&mut map, "original", Value::Object(key))?;
            self.put(&mut map, "owner_effective_type", owner)?;
            owner = Value::Object(map);
        }
        Ok(owner)
    }

    // Validate structural nodes iteratively; JSON copying uses the shared stack.
    fn node(&mut self, value: &Value, depth: usize) -> Result<Value, ReferenceCodecError> {
        let mut pending = Vec::new();
        values::push(self, &mut pending, (value, depth))?;
        while let Some((value, depth)) = pending.pop() {
            self.work(1)?;
            if depth > self.max_depth {
                return Err(self.fail(ReferenceFailure::DepthLimit));
            }
            let map = self.closed(value, &["package", "path", "ordinal"])?;
            let package = self.string(&map["package"])?;
            self.work(package.len())?;
            if package.is_empty()
                || map["ordinal"]
                    .as_u64()
                    .filter(|n| *n <= 9_007_199_254_740_991)
                    .is_none()
            {
                return Err(self.fail(ReferenceFailure::MalformedWire));
            }
            let segments = map["path"]
                .as_array()
                .filter(|path| !path.is_empty())
                .ok_or_else(|| self.fail(ReferenceFailure::MalformedWire))?;
            for segment in segments.iter().rev() {
                match segment {
                    Value::String(text) => self.work(text.len())?,
                    Value::Object(_) => {
                        values::push(self, &mut pending, (segment, depth.saturating_add(1)))?
                    }
                    _ => return Err(self.fail(ReferenceFailure::MalformedWire)),
                }
            }
        }
        self.with_admission(|admission| admission.borrowed_json(value, usize::MAX))
    }
}

impl<'c, 'm, 'r> ReferenceCodec<'c, 'm, 'r> {
    /// Borrow one shared resource scope for a nested external carrier.
    /// `max_depth` is the caller's limit; `usize::MAX` means no depth limit.
    pub fn scope<'s>(
        &'s self,
        budget: &'s mut dyn IntegerDecimalBudget,
        cancel: &'s Cancel,
        max_depth: usize,
        limits: quire_canonical::Limits,
    ) -> ReferenceScope<'s, 'c, 'm, 'r> {
        ReferenceScope {
            codec: self,
            scope: Scope {
                budget,
                cancel,
                normalization_owner: None,
                path: String::new(),
                max_depth,
            },
            limits,
        }
    }
    fn native_reference(
        &self,
        declaration: &ReferenceDeclaration<'_>,
        object: &str,
        scope: &mut Scope<'_, '_>,
    ) -> Result<ObjectReference, ReferenceCodecError> {
        // Account the model-owned declaring/type lookup and its subsequent
        // reference construction before invoking the allocating owner method.
        if let ReferenceRuntime::Model(system) = &self.runtime {
            system
                .reference_model()
                .reference_components_with_work(declaration.key(), &mut |units| {
                    let units = usize::try_from(units)
                        .ok()
                        .and_then(|n| n.checked_mul(2))
                        .ok_or_else(|| scope.stopped(ModelKeyError::SizeOverflow))?;
                    scope.work(units)
                })?
                .ok_or_else(|| scope.fail(ReferenceFailure::UnknownDeclaration))?;
        }
        let owner_storage = size_of::<ObjectReference>()
            .checked_mul(16)
            .ok_or_else(|| scope.stopped(ModelKeyError::SizeOverflow))?;
        scope.reserve(owner_storage)?;
        let model_runtime = matches!(&self.runtime, ReferenceRuntime::Model(_));
        for original in declaration.effective().owner_chain() {
            scope.work(original.package().len())?;
            scope.work(original.node().len())?;
            if model_runtime {
                scope.reserve(size_of::<filament_exact::OriginalDeclarationKey>())?;
                scope.reserve(original.package().len())?;
                scope.reserve(original.node().len())?;
            }
        }
        let root = declaration
            .universe()
            .root_types
            .first()
            .ok_or_else(|| scope.fail(ReferenceFailure::IncompatibleUniverse))?
            .original();
        scope.work(root.package().len())?;
        scope.work(root.node().len())?;
        scope.reserve(root.package().len())?;
        scope.reserve(root.node().len())?;
        if model_runtime {
            scope.work(object.len())?;
            scope.reserve(object.len())?;
        }
        let result = match &self.runtime {
            ReferenceRuntime::Model(system) => system
                .object_reference(declaration.key(), object)
                .map_err(|cause| ReferenceCodecError::Native {
                cause,
                pointer: std::mem::take(&mut scope.path),
            })?,
            ReferenceRuntime::Call(_) => {
                let effective = declaration
                    .effective()
                    .try_clone_with(&mut |bytes| scope.reserve_key(bytes))
                    .map_err(|cause| ReferenceCodecError::IdentityCopy {
                        cause,
                        pointer: std::mem::take(&mut scope.path),
                    })?;
                let universe = declaration.universe().identity().map_err(|cause| {
                    ReferenceCodecError::Universe {
                        cause,
                        pointer: std::mem::take(&mut scope.path),
                    }
                })?;
                let object = ObjectId::new(scope.text(object)?)
                    .map_err(|_| scope.fail(ReferenceFailure::InvalidIdentity))?;
                ObjectReference::new(universe, effective, object)
            }
        };
        scope.poll()?;
        Ok(result)
    }
    fn value_with_children(
        &self,
        value: &Value,
        restore: bool,
        _depth: usize,
        scope: &mut Scope<'_, 'm>,
        children: &mut impl Iterator<Item = Value>,
    ) -> Result<Value, ReferenceCodecError> {
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| scope.fail(ReferenceFailure::MalformedWire))?;
        if kind == "reference" {
            return self.reference(value, restore, scope);
        }
        let fields: &[&str] = match kind {
            "null" => &["type"],
            "boolean" | "integer" | "text" | "option" | "uuid" => &["type", "value"],
            "timestamp" => &["type", "nanoseconds"],
            "float32" | "float64" => &["type", "bits"],
            "bytes" | "json_object" => &["type", "text"],
            "set" | "bag" | "sequence" | "ordered-set" => &["type", "elements"],
            "map" => &["type", "entries"],
            "record" => &["type", "name", "fields"],
            "tuple" => &["type", "name", "components"],
            "enum" => &["type", "name", "member"],
            "union" => &["type", "name", "member", "components"],
            "rational" if value.get("unit").is_some() => {
                &["type", "numerator", "denominator", "unit"]
            }
            "rational" => &["type", "numerator", "denominator"],
            "decimal" if value.get("unit").is_some() => &["type", "coefficient", "scale", "unit"],
            "decimal" => &["type", "coefficient", "scale"],
            _ => return Err(scope.fail(ReferenceFailure::MalformedWire)),
        };
        let map = scope.closed(value, fields)?;
        let mut out = json::DeepMap(Map::new());
        for name in fields {
            let input = &map[*name];
            let output = scope.member(name, |s| {
                if kind == "option" && *name == "value" {
                    return if input.is_null() {
                        Ok(Value::Null)
                    } else {
                        children
                            .next()
                            .ok_or_else(|| s.fail(ReferenceFailure::MalformedWire))
                    };
                }
                if matches!(*name, "elements" | "components" | "entries" | "fields") {
                    let array = input
                        .as_array()
                        .ok_or_else(|| s.fail(ReferenceFailure::MalformedWire))?;
                    let mut items = json::DeepArray(s.array(array.len())?);
                    for (index, element) in array.iter().enumerate() {
                        s.reserve(20)?;
                        let index = index.to_string();
                        let converted = s.member(&index, |s| match *name {
                            "entries" | "fields" => {
                                let names: &[&str] = if *name == "entries" {
                                    &["key", "value"]
                                } else {
                                    &["name", "value"]
                                };
                                let row = s.closed(element, names)?;
                                let mut converted = json::DeepMap(Map::new());
                                for child in names {
                                    let item = s.member(child, |s| {
                                        if *child == "name" {
                                            let text = s.string(&row[*child])?;
                                            Ok(Value::String(s.text(text)?))
                                        } else if *name == "fields" && row[*child].is_null() {
                                            Ok(Value::Null)
                                        } else {
                                            children.next().ok_or_else(|| {
                                                s.fail(ReferenceFailure::MalformedWire)
                                            })
                                        }
                                    })?;
                                    s.put(&mut converted, child, item)?;
                                }
                                Ok(Value::Object(std::mem::take(&mut converted.0)))
                            }
                            _ => children
                                .next()
                                .ok_or_else(|| s.fail(ReferenceFailure::MalformedWire)),
                        })?;
                        items.push(converted);
                    }
                    if matches!(kind, "set" | "bag" | "map") {
                        sort_values(&mut items, kind == "map", kind != "bag", s)?;
                    }
                    return Ok(Value::Array(std::mem::take(&mut items.0)));
                }
                if kind == "boolean" && *name == "value" {
                    return input
                        .as_bool()
                        .map(Value::Bool)
                        .ok_or_else(|| s.fail(ReferenceFailure::MalformedWire));
                }
                let text = s.string(input)?;
                s.work(text.len())?;
                if matches!(
                    *name,
                    "numerator" | "denominator" | "coefficient" | "scale" | "nanoseconds"
                ) || (kind == "integer" && *name == "value")
                {
                    if !integer_text(text)
                        || (*name == "denominator" && (text.starts_with('-') || text == "0"))
                    {
                        return Err(s.fail(ReferenceFailure::MalformedWire));
                    }
                }
                if *name == "bits" {
                    s.work(text.len())?;
                    let width = if kind == "float32" { 8 } else { 16 };
                    if text.len() != width
                        || !text
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    {
                        return Err(s.fail(ReferenceFailure::MalformedWire));
                    }
                }
                Ok(Value::String(s.text(text)?))
            })?;
            scope.put(&mut out, name, output)?;
        }
        Ok(Value::Object(std::mem::take(&mut out.0)))
    }

    /// Bind a catalog to the exact runtime owners that will replay its values.
    ///
    /// # Errors
    /// Refuses a foreign package/model even if its printed identities match.
    pub fn new(
        catalog: &'c ReferenceCatalog<'m>,
        system: &'c ModelSystem<'r>,
    ) -> Result<Self, ReferenceCodecError> {
        let input = system.input();
        let package_matches = match catalog.source() {
            Some(source) => input
                .source_package()
                .is_some_and(|actual| std::ptr::eq(actual, source)),
            None => input.admitted_package().is_some_and(|actual| {
                catalog
                    .package()
                    .is_some_and(|package| std::ptr::eq(actual, package))
            }),
        };
        if !package_matches
            || !catalog
                .model()
                .is_some_and(|model| std::ptr::eq(model, system.reference_model()))
        {
            return Err(ReferenceCodecError::Refused {
                reason: ReferenceFailure::ForeignCatalog,
                pointer: String::new(),
            });
        }
        Ok(Self {
            catalog,
            runtime: ReferenceRuntime::Model(system),
        })
    }

    /// Use the original checked function-call package, model admission and
    /// object environment. No simulator or caller-supplied universe map is
    /// accepted as a substitute for the normalized admission owner.
    ///
    /// # Errors
    /// Refuses a catalog from another package or model-admission operation.
    pub fn for_checked_call(
        catalog: &'c ReferenceCatalog<'m>,
        source: &CheckedPackage,
        models: &SelectedModels,
        objects: &'c ObjectEnvironment,
    ) -> Result<Self, ReferenceCodecError> {
        let matches = catalog
            .source()
            .is_some_and(|actual| std::ptr::eq(actual, source))
            && catalog.package().is_none()
            && catalog
                .selected_models()
                .is_some_and(|actual| std::ptr::eq(actual, models));
        Self::call_context(catalog, objects, matches)
    }

    /// Use a strictly admitted function-call package and its actual model
    /// admission with the same shared recursive converter.
    ///
    /// # Errors
    /// Refuses a catalog from another package or model-admission operation.
    pub fn for_admitted_call(
        catalog: &'c ReferenceCatalog<'m>,
        package: &CheckedPackageV2,
        models: &[fsl_semantics::model::intake::NormalizedModel],
        objects: &'c ObjectEnvironment,
    ) -> Result<Self, ReferenceCodecError> {
        let matches = catalog.source().is_none()
            && catalog
                .package()
                .is_some_and(|actual| std::ptr::eq(actual, package))
            && catalog
                .snapshot_models()
                .is_some_and(|actual| std::ptr::eq(actual, models));
        Self::call_context(catalog, objects, matches)
    }

    fn call_context(
        catalog: &'c ReferenceCatalog<'m>,
        objects: &'c ObjectEnvironment,
        matches: bool,
    ) -> Result<Self, ReferenceCodecError> {
        if !matches || catalog.model().is_some() {
            return Err(ReferenceCodecError::Refused {
                reason: ReferenceFailure::ForeignCatalog,
                pointer: String::new(),
            });
        }
        Ok(Self {
            catalog,
            runtime: ReferenceRuntime::Call(objects),
        })
    }

    fn reference(
        &self,
        value: &Value,
        restore: bool,
        scope: &mut Scope<'_, 'm>,
    ) -> Result<Value, ReferenceCodecError> {
        let owner = scope.normalization_owner;
        let result = self.reference_in(value, restore, scope);
        scope.normalization_owner = owner;
        result
    }

    fn reference_in(
        &self,
        value: &Value,
        restore: bool,
        scope: &mut Scope<'_, 'm>,
    ) -> Result<Value, ReferenceCodecError> {
        let map = scope.closed(value, &["type", "universe", "object_type", "identity"])?;
        if map["type"].as_str() != Some("reference") {
            return Err(scope.fail(ReferenceFailure::MalformedWire));
        }
        let declaration = scope.member("object_type", |s| {
            let wire_node = if restore {
                Some(s.node(&map["object_type"], 0)?)
            } else {
                if self.catalog.snapshot_models().is_none() {
                    s.validate_effective(&map["object_type"])?;
                }
                None
            };
            for declaration in self.catalog.declarations() {
                let previous = s.normalization_owner;
                if !restore {
                    if let Some(owner) = declaration.normalization_owner() {
                        s.normalization_owner = Some(owner);
                        s.validate_effective(&map["object_type"])?;
                    }
                }
                s.work(1)?;
                let matches = if let Some(wire) = &wire_node {
                    let candidate = s.declaration_json(declaration)?;
                    // Both shapes were traversed/charged before this complete
                    // comparison; declaration paths are authenticated and flat.
                    s.same_json(&candidate, wire)?
                } else {
                    s.effective_matches(&map["object_type"], declaration.effective())?
                };
                if matches {
                    return Ok(declaration);
                }
                s.normalization_owner = previous;
            }
            Err(s.fail(ReferenceFailure::UnknownDeclaration))
        })?;
        let universe = scope.member("universe", |s| s.node(&map["universe"], 0))?;
        let object = scope.member("identity", |s| {
            let text = s.string(&map["identity"])?;
            if restore {
                s.hex_decode(text)
            } else {
                if text.is_empty() {
                    return Err(s.fail(ReferenceFailure::InvalidIdentity));
                }
                s.text(text)
            }
        })?;
        // Object admission validates the native universe/type combination;
        // this does not assert enabledness or population membership.
        let native = self.native_reference(declaration, &object, scope)?;
        let expected = native.universe().root_type();
        // Shared NodeId codec receives a shape already bounded and charged by
        // `node`; budget the encoded temporary before the owner codec allocates.
        let expected_json = scope.with_admission(|admission| admission.node_json(expected))?;
        if !scope.same_json(&universe, &expected_json)? {
            return scope.member("universe", |s| {
                Err(s.fail(ReferenceFailure::IncompatibleUniverse))
            });
        }
        let object_type = if restore {
            scope.effective_json(declaration.effective())?
        } else {
            scope.declaration_json(declaration)?
        };
        let identity = if restore {
            object
        } else {
            scope.member("identity", |s| s.hex_encode(&object))?
        };
        let mut out = Map::new();
        let tag = Value::String(scope.text("reference")?);
        scope.put(&mut out, "type", tag)?;
        scope.put(&mut out, "universe", universe)?;
        scope.put(&mut out, "object_type", object_type)?;
        scope.put(&mut out, "identity", Value::String(identity))?;
        Ok(Value::Object(out))
    }

    /// Convert a closed native typed value to the backend reference contract.
    /// `max_depth` is the caller's existing typed-value nesting ceiling.
    ///
    /// # Errors
    /// Preserves typed shape, identity, universe and runtime resource failures.
    pub fn encode_value(
        &self,
        value: &Value,
        budget: &mut dyn IntegerDecimalBudget,
        cancel: &Cancel,
        max_depth: usize,
    ) -> Result<Value, ReferenceCodecError> {
        self.value(
            value,
            false,
            0,
            &mut Scope {
                budget,
                cancel,
                normalization_owner: None,
                path: String::new(),
                max_depth,
            },
        )
    }

    /// Admit the borrowed writer's native typed bytes before converting its
    /// references under the original caller budget.
    /// `max_depth` is the caller's limit; `usize::MAX` means no depth limit.
    ///
    /// # Errors
    /// Original typed wire, authority, normalization and caller failures.
    pub fn encode_value_bytes(
        &self,
        written: &[u8],
        budget: &mut dyn IntegerDecimalBudget,
        cancel: &Cancel,
        max_depth: usize,
    ) -> Result<Value, ReferenceCodecError> {
        let mut scope = Scope {
            budget,
            cancel,
            normalization_owner: None,
            path: String::new(),
            max_depth,
        };
        let text = std::str::from_utf8(written)
            .map_err(|_| scope.fail(ReferenceFailure::MalformedWire))?;
        let value = json::DeepJson(Some(parse_text(text, &mut scope)?));
        self.value(
            value.0.as_ref().expect("admitted native value"),
            false,
            0,
            &mut scope,
        )
    }

    /// Restore a closed backend typed value to its exact native representation.
    ///
    /// # Errors
    /// Preserves typed shape, identity, universe and runtime resource failures.
    pub fn restore_value(
        &self,
        value: &Value,
        budget: &mut dyn IntegerDecimalBudget,
        cancel: &Cancel,
        max_depth: usize,
    ) -> Result<Value, ReferenceCodecError> {
        self.value(
            value,
            true,
            0,
            &mut Scope {
                budget,
                cancel,
                normalization_owner: None,
                path: String::new(),
                max_depth,
            },
        )
    }

    /// Encode a genuine native reference with the same converter as nested values.
    ///
    /// # Errors
    /// Preserves catalog, universe, identity and caller resource refusals.
    pub fn encode_reference(
        &self,
        reference: &ObjectReference,
        budget: &mut dyn IntegerDecimalBudget,
        cancel: &Cancel,
        max_depth: usize,
    ) -> Result<Value, ReferenceCodecError> {
        let mut scope = Scope {
            budget,
            cancel,
            normalization_owner: None,
            path: String::new(),
            max_depth,
        };
        self.encode_reference_in(reference, &mut scope)
    }

    fn encode_reference_in(
        &self,
        reference: &ObjectReference,
        scope: &mut Scope<'_, 'm>,
    ) -> Result<Value, ReferenceCodecError> {
        let previous_owner = scope.normalization_owner;
        let result = (|| {
            if self.catalog.snapshot_models().is_some() {
                scope.normalization_owner = Some(
                    self.catalog
                        .normalization_owner_for(reference)
                        .map_err(|cause| ReferenceCodecError::Normalization {
                            cause,
                            pointer: String::new(),
                        })?
                        .ok_or_else(|| scope.fail(ReferenceFailure::UnknownDeclaration))?,
                );
            }
            let native = scope.with_admission(|admission| {
                admission.encoded(quire_canonical::Limits::new(u64::MAX), |writer| {
                    fsl_eval::simulation::model::encode_reference(reference, writer)
                })
            })?;
            let text = std::str::from_utf8(&native)
                .map_err(|_| scope.fail(ReferenceFailure::MalformedWire))?;
            let value = parse_text(text, scope)?;
            self.reference(&value, false, scope)
        })();
        scope.normalization_owner = previous_owner;
        result
    }

    /// Restore a backend reference using the catalog's actual native model.
    ///
    /// # Errors
    /// Invalid backend octets never become a lossy native ObjectId.
    pub fn restore_reference(
        &self,
        value: &Value,
        budget: &mut dyn IntegerDecimalBudget,
        cancel: &Cancel,
        max_depth: usize,
    ) -> Result<ObjectReference, ReferenceCodecError> {
        let mut scope = Scope {
            budget,
            cancel,
            normalization_owner: None,
            path: String::new(),
            max_depth,
        };
        let native = self.reference(value, true, &mut scope)?;
        for declaration in self.catalog.declarations() {
            if scope.effective_matches(&native["object_type"], declaration.effective())? {
                let identity = scope.string(&native["identity"])?;
                return self.native_reference(declaration, identity, &mut scope);
            }
        }
        Err(scope.fail(ReferenceFailure::UnknownDeclaration))
    }

    /// Restore receiver/arguments before constructing a native transition.
    /// Operation identity is retained verbatim as its canonical JSON value.
    ///
    /// # Errors
    /// Refuses malformed transitions and preserves caller budget/cancellation.
    pub fn restore_transition(
        &self,
        value: &Value,
        budget: &mut dyn IntegerDecimalBudget,
        cancel: &Cancel,
        max_depth: usize,
        limits: quire_canonical::Limits,
    ) -> Result<ModelTransition, ReferenceCodecError> {
        let mut scope = Scope {
            budget,
            cancel,
            normalization_owner: None,
            path: String::new(),
            max_depth,
        };
        self.restore_transition_in(value, &mut scope, limits)
    }

    fn restore_transition_in(
        &self,
        value: &Value,
        scope: &mut Scope<'_, 'm>,
        limits: quire_canonical::Limits,
    ) -> Result<ModelTransition, ReferenceCodecError> {
        self.restore_transition_with_operation(value, scope, limits, None)
    }

    fn restore_transition_with_operation(
        &self,
        value: &Value,
        scope: &mut Scope<'_, 'm>,
        limits: quire_canonical::Limits,
        operation: Option<String>,
    ) -> Result<ModelTransition, ReferenceCodecError> {
        let map = scope.closed(value, &["type", "operation", "receiver", "arguments"])?;
        if map["type"].as_str() != Some("transition") {
            return Err(scope.fail(ReferenceFailure::MalformedWire));
        }
        let operation = match operation {
            Some(operation) => operation,
            None => scope.member("operation", |s| {
                if !matches!(map["operation"], Value::String(_) | Value::Object(_)) {
                    return Err(s.fail(ReferenceFailure::MalformedWire));
                }
                let operation = s.json(&map["operation"], 0)?;
                canonical_text(&operation, s, limits)
            })?,
        };
        let receiver = scope.member("receiver", |s| {
            if map["receiver"].is_null() {
                return Ok(None);
            }
            let restored = self.value(&map["receiver"], true, 0, s)?;
            canonical_text(&restored, s, limits).map(Some)
        })?;
        let arguments = scope.member("arguments", |s| {
            let array = map["arguments"]
                .as_array()
                .ok_or_else(|| s.fail(ReferenceFailure::MalformedWire))?;
            let bytes = array
                .len()
                .checked_mul(size_of::<String>())
                .ok_or_else(|| s.stopped(ModelKeyError::SizeOverflow))?;
            s.reserve(bytes)?;
            let mut arguments = Vec::new();
            arguments
                .try_reserve_exact(array.len())
                .map_err(|_| s.allocation(bytes))?;
            for (index, argument) in array.iter().enumerate() {
                s.reserve(20)?;
                let index = index.to_string();
                arguments.push(s.member(&index, |s| {
                    let restored = self.value(argument, true, 0, s)?;
                    canonical_text(&restored, s, limits)
                })?);
            }
            Ok(arguments)
        })?;
        // The model-transition owner checks canonical text through its arena
        // reader. Reserve its validation storage before invoking that owner.
        let mut bytes = operation.len();
        for text in receiver.iter().chain(arguments.iter()) {
            bytes = bytes
                .checked_add(text.len())
                .ok_or_else(|| scope.stopped(ModelKeyError::SizeOverflow))?;
        }
        scope.work(bytes)?;
        let validation_bytes = bytes
            .checked_mul(64)
            .ok_or_else(|| scope.stopped(ModelKeyError::SizeOverflow))?;
        scope.reserve(validation_bytes)?;
        ModelTransition::new(operation, receiver, arguments, limits).map_err(|source| {
            ReferenceCodecError::Transition {
                source,
                pointer: std::mem::take(&mut scope.path),
            }
        })
    }

    /// Encode every reference in a native transition without changing it.
    ///
    /// # Errors
    /// Preserves typed conversion and caller resource/cancellation refusals.
    pub fn encode_transition(
        &self,
        transition: &ModelTransition,
        budget: &mut dyn IntegerDecimalBudget,
        cancel: &Cancel,
        max_depth: usize,
    ) -> Result<Value, ReferenceCodecError> {
        let mut scope = Scope {
            budget,
            cancel,
            normalization_owner: None,
            path: String::new(),
            max_depth,
        };
        self.encode_transition_in(transition, &mut scope)
    }

    fn encode_transition_in(
        &self,
        transition: &ModelTransition,
        scope: &mut Scope<'_, 'm>,
    ) -> Result<Value, ReferenceCodecError> {
        let operation = scope.member("operation", |s| parse_text(transition.operation(), s))?;
        if !matches!(operation, Value::String(_) | Value::Object(_)) {
            return scope.member(
                "operation",
                |s| Err(s.fail(ReferenceFailure::MalformedWire)),
            );
        }
        let receiver = scope.member("receiver", |s| match transition.receiver() {
            None => Ok(Value::Null),
            Some(text) => {
                let native = parse_text(text, s)?;
                self.value(&native, false, 0, s)
            }
        })?;
        let mut arguments = scope.array(transition.arguments().len())?;
        scope.member("arguments", |s| {
            for (index, argument) in transition.arguments().iter().enumerate() {
                s.reserve(20)?;
                let index = index.to_string();
                arguments.push(s.member(&index, |s| {
                    let native = parse_text(argument, s)?;
                    self.value(&native, false, 0, s)
                })?);
            }
            Ok(())
        })?;
        let mut out = Map::new();
        let tag = Value::String(scope.text("transition")?);
        scope.put(&mut out, "type", tag)?;
        scope.put(&mut out, "operation", operation)?;
        scope.put(&mut out, "receiver", receiver)?;
        scope.put(&mut out, "arguments", Value::Array(arguments))?;
        Ok(Value::Object(out))
    }

    /// Restore an `over` object using the original request's retained selection.
    /// The producer must pass that original selection, never a reconstructed one.
    /// Conversion alone does not authenticate a trace or its enabled transitions.
    ///
    /// # Errors
    /// Refuses absent/wrong population, missing members and reference failures.
    pub fn restore_over_binding(
        &self,
        value: &Value,
        selected: Option<&ModelInstance>,
        binding: &PopulationBinding,
        budget: &mut dyn IntegerDecimalBudget,
        cancel: &Cancel,
        max_depth: usize,
    ) -> Result<ModelInstance, ReferenceCodecError> {
        let mut scope = Scope {
            budget,
            cancel,
            normalization_owner: None,
            path: String::new(),
            max_depth,
        };
        let selected = selected.ok_or_else(|| scope.fail(ReferenceFailure::MissingPopulation))?;
        if let ReferenceRuntime::Call(objects) = &self.runtime {
            let mut bindings = objects.population_bindings();
            let mut owned = false;
            loop {
                scope.work(1)?;
                let Some(actual) = bindings.next() else {
                    break;
                };
                if std::ptr::eq(actual, binding) {
                    owned = true;
                    break;
                }
            }
            if !owned {
                return Err(scope.fail(ReferenceFailure::WrongPopulation));
            }
        }
        if !scope.equal_text(
            &selected.population.package,
            &binding.population_key().package,
        )? || !scope.equal_text(&selected.population.node, &binding.population_key().node)?
        {
            return Err(scope.fail(ReferenceFailure::WrongPopulation));
        }
        let native = self.reference(value, true, &mut scope)?;
        let object = native
            .get("identity")
            .and_then(Value::as_str)
            .ok_or_else(|| scope.fail(ReferenceFailure::MalformedWire))?;
        if !scope.equal_text(object, &selected.key)? {
            return Err(scope.fail(ReferenceFailure::WrongPopulation));
        }
        for (member, _) in binding.members() {
            scope.work(1)?;
            if !scope.equal_text(&member.object, object)? {
                continue;
            }
            let matches = scope.effective_matches(&native["object_type"], &member.type_identity)?;
            if !matches {
                continue;
            }
            let expected = binding.universe().root_type();
            let expected = scope.with_admission(|admission| admission.node_json(expected))?;
            if !scope.same_json(&native["universe"], &expected)? {
                return Err(scope.fail(ReferenceFailure::WrongPopulation));
            }
            let key = scope.text(&selected.key)?;
            let package = scope.text(&selected.population.package)?;
            let node = scope.text(&selected.population.node)?;
            return Ok(ModelInstance {
                population: filament_checked::key::DeclarationKey { package, node },
                key,
            });
        }
        Err(scope.fail(ReferenceFailure::WrongPopulation))
    }
}

fn integer_text(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && text != "-0"
}

/// Read native typed-value bytes using the caller's existing work and
/// physical-reservation policy. Reference authentication remains the codec's.
///
/// # Errors
/// The original caller/cancellation refusal or malformed typed JSON.
pub fn admit_native_value_bytes(
    written: &[u8],
    budget: &mut dyn IntegerDecimalBudget,
    cancel: &Cancel,
    max_depth: usize,
) -> Result<Value, ReferenceCodecError> {
    let mut scope = Scope {
        budget,
        cancel,
        normalization_owner: None,
        path: String::new(),
        max_depth,
    };
    let text =
        std::str::from_utf8(written).map_err(|_| scope.fail(ReferenceFailure::MalformedWire))?;
    parse_text(text, &mut scope)
}

fn parse_text(text: &str, scope: &mut Scope<'_, '_>) -> Result<Value, ReferenceCodecError> {
    match scope.with_admission(|admission| admission.json_value(text.as_bytes())) {
        Err(ReferenceCodecError::Key {
            source: ModelKeyError::Unsupported(json::MALFORMED),
            ..
        }) => Err(scope.fail(ReferenceFailure::MalformedWire)),
        other => other,
    }
}

fn canonical_text(
    value: &Value,
    scope: &mut Scope<'_, '_>,
    limits: quire_canonical::Limits,
) -> Result<String, ReferenceCodecError> {
    let bytes = scope.with_admission(|admission| admission.canonical(value, limits))?;
    String::from_utf8(bytes).map_err(|_| scope.fail(ReferenceFailure::MalformedWire))
}

fn sort_values(
    values: &mut Vec<Value>,
    map: bool,
    unique: bool,
    scope: &mut Scope<'_, '_>,
) -> Result<(), ReferenceCodecError> {
    let capacity = values
        .len()
        .checked_mul(size_of::<(Vec<u8>, Value)>())
        .ok_or_else(|| scope.stopped(ModelKeyError::SizeOverflow))?;
    scope.reserve(capacity)?;
    let mut keyed = Vec::new();
    keyed
        .try_reserve_exact(values.len())
        .map_err(|_| scope.allocation(capacity))?;
    for value in values.drain(..) {
        let key = if map { &value["key"] } else { &value };
        let bytes =
            canonical_text(key, scope, quire_canonical::Limits::new(u64::MAX))?.into_bytes();
        keyed.push((bytes, value));
    }
    // Fallible insertion sort permits charging every compared byte before the
    // comparison; no comparator hides a denied work charge.
    for index in 1..keyed.len() {
        let mut at = index;
        while at > 0 {
            scope.work(
                keyed[at - 1]
                    .0
                    .len()
                    .min(keyed[at].0.len())
                    .saturating_add(1),
            )?;
            match keyed[at - 1].0.cmp(&keyed[at].0) {
                std::cmp::Ordering::Greater => {
                    scope.work(1)?;
                    keyed.swap(at - 1, at);
                    at -= 1;
                }
                std::cmp::Ordering::Equal if unique => {
                    return Err(scope.fail(ReferenceFailure::MalformedWire))
                }
                _ => break,
            }
        }
    }
    values.extend(keyed.into_iter().map(|(_, value)| value));
    Ok(())
}

/// Canonical writer admission shares the caller's budget and cancellation. Both
/// writer scratch and sink output allocations are charged before reservation.
struct Admission<'a> {
    budget: RefCell<&'a mut dyn IntegerDecimalBudget>,
    cancel: &'a Cancel,
    failure: RefCell<Option<ModelKeyError>>,
    normalization_owner: Option<&'a fsl_semantics::model::intake::NormalizedModel>,
    normalization_failure:
        RefCell<Option<fsl_semantics::model::accounting::NormalizationIncomplete>>,
}

struct DecimalAdmission<'a, 'b>(&'a Admission<'b>);

impl filament_exact::Reservation for DecimalAdmission<'_, '_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), filament_exact::ReservationDenied> {
        self.0.reserve(bytes).map_err(|error| {
            *self.0.failure.borrow_mut() = Some(error);
            filament_exact::ReservationDenied
        })
    }
}

impl IntegerDecimalBudget for DecimalAdmission<'_, '_> {
    fn work(&mut self, units: u64) -> Result<(), filament_exact::ReservationDenied> {
        let result = usize::try_from(units)
            .map_err(|_| ModelKeyError::SizeOverflow)
            .and_then(|units| self.0.work(units));
        result.map_err(|error| {
            *self.0.failure.borrow_mut() = Some(error);
            filament_exact::ReservationDenied
        })
    }
}

impl<'a> Admission<'a> {
    fn new(
        budget: &'a mut dyn IntegerDecimalBudget,
        cancel: &'a Cancel,
        normalization_owner: Option<&'a fsl_semantics::model::intake::NormalizedModel>,
    ) -> Self {
        Self {
            budget: RefCell::new(budget),
            cancel,
            failure: RefCell::new(None),
            normalization_owner,
            normalization_failure: RefCell::new(None),
        }
    }

    fn poll(&self) -> Result<(), ModelKeyError> {
        self.cancel.poll();
        match self.cancel.tripped() {
            Some(cause) => Err(ModelKeyError::Cancelled(cause)),
            None => Ok(()),
        }
    }

    fn work(&self, units: usize) -> Result<(), ModelKeyError> {
        self.poll()?;
        let units = u64::try_from(units).map_err(|_| ModelKeyError::SizeOverflow)?;
        if let Some(owner) = self.normalization_owner {
            owner
                .charge_reference_work(
                    fsl_semantics::model::accounting::ChargePoint::ReferenceResult,
                    units,
                )
                .map_err(|cause| {
                    if self.normalization_failure.borrow().is_none() {
                        *self.normalization_failure.borrow_mut() = Some(cause);
                    }
                    ModelKeyError::Denied
                })?;
        } else {
            self.budget
                .borrow_mut()
                .work(units)
                .map_err(|_| ModelKeyError::Denied)?;
        }
        self.poll()
    }

    fn reserve(&self, bytes: usize) -> Result<(), ModelKeyError> {
        self.poll()?;
        self.budget
            .borrow_mut()
            .reserve(bytes)
            .map_err(|_| ModelKeyError::Denied)?;
        self.poll()
    }

    fn denied(&self, error: ModelKeyError) -> quire_canonical::Error {
        if self.failure.borrow().is_none() {
            *self.failure.borrow_mut() = Some(error);
        }
        quire_canonical::Error::ReservationDenied { requested: 0 }
    }

    fn encoded(
        &self,
        limits: quire_canonical::Limits,
        encode: impl FnOnce(
            &mut quire_canonical::Writer<'_, Output<'_, '_>>,
        ) -> Result<(), quire_canonical::Error>,
    ) -> Result<Vec<u8>, ModelKeyError> {
        self.poll()?;
        let mut output = Output {
            admission: self,
            bytes: Vec::new(),
        };
        let mut provision = Provision(self);
        let mut writer =
            quire_canonical::Writer::with_reservation(&mut output, limits, &mut provision);
        let result = encode(&mut writer).and_then(|()| writer.finish().map(|_| ()));
        if let Some(error) = self.failure.borrow_mut().take() {
            return Err(error);
        }
        result.map_err(|error| match error {
            quire_canonical::Error::ReservationDenied { .. } => ModelKeyError::Denied,
            error => ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::of(
                &error,
                fsl_foundation::IdentityLimits {
                    input_bytes: limits.max_bytes(),
                    ..fsl_foundation::IdentityLimits::default()
                },
            )),
        })?;
        self.poll()?;
        Ok(output.bytes)
    }

    fn canonical(
        &self,
        value: &Value,
        limits: quire_canonical::Limits,
    ) -> Result<Vec<u8>, ModelKeyError> {
        use quire_canonical::Encode;
        self.encoded(limits, |writer| value.encode_into(writer))
    }

    fn node_json(&self, node: &filament_exact::NodeKey) -> Result<Value, ModelKeyError> {
        let bytes = self.encoded(quire_canonical::Limits::new(u64::MAX), |writer| {
            fsl_foundation::node_wire::encode_node_key(node, writer)
        })?;
        self.json_value(&bytes)
    }

    fn finish<T>(self, result: Result<T, ModelKeyError>) -> Result<T, ModelKeyError> {
        if let Some(error) = self.failure.into_inner() {
            return Err(error);
        }
        result
    }
}

struct Provision<'s, 'a>(&'s Admission<'a>);
impl quire_canonical::Reservation for Provision<'_, '_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), quire_canonical::ReservationDenied> {
        self.0.reserve(bytes).map_err(|error| {
            self.0.denied(error);
            quire_canonical::ReservationDenied
        })
    }
}

struct Output<'s, 'a> {
    admission: &'s Admission<'a>,
    bytes: Vec<u8>,
}
impl quire_canonical::Sink for Output<'_, '_> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), quire_canonical::Error> {
        self.admission
            .work(bytes.len())
            .map_err(|e| self.admission.denied(e))?;
        let needed = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| self.admission.denied(ModelKeyError::SizeOverflow))?;
        if needed > self.bytes.capacity() {
            let capacity = self
                .bytes
                .capacity()
                .checked_mul(2)
                .unwrap_or(needed)
                .max(needed);
            self.admission
                .reserve(capacity)
                .map_err(|e| self.admission.denied(e))?;
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(|_| {
                    self.admission.denied(ModelKeyError::Encoding(
                        fsl_eval::simulation::EncodingRefusal::Allocation {
                            requested: capacity,
                        },
                    ))
                })?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ReferenceCodecError, ReferenceFailure, Scope};
    use filament_exact::{
        Cancel, Integer, IntegerDecimalBudget, Meter, Reservation, ReservationDenied,
    };
    use fsl_eval::simulation::model::{ModelKeyBudget, ModelKeyError, ModelRestoreBudget};

    struct Budget {
        meter: Meter,
        storage_left: usize,
        work_left: u64,
        storage_used: usize,
    }

    impl Budget {
        fn new(storage_left: usize, work_left: u64) -> Self {
            Self {
                meter: Meter::new(crate::spine::default_accounting(work_left)),
                storage_left,
                work_left,
                storage_used: 0,
            }
        }
    }

    impl Reservation for Budget {
        fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
            let remaining = self
                .storage_left
                .checked_sub(bytes)
                .ok_or(ReservationDenied)?;
            self.storage_used = self
                .storage_used
                .checked_add(bytes)
                .ok_or(ReservationDenied)?;
            self.storage_left = remaining;
            Ok(())
        }
    }

    impl IntegerDecimalBudget for Budget {
        fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
            self.work_left = self.work_left.checked_sub(units).ok_or(ReservationDenied)?;
            Ok(())
        }
    }

    impl ModelKeyBudget for Budget {
        fn integer(&mut self, value: &Integer) -> Result<(), ReservationDenied> {
            self.meter
                .charge(
                    filament_exact::Charge::new(filament_exact::ChargePoint::GraphResultRetain)
                        .size(
                            filament_exact::LimitKind::IntegerBits,
                            value.magnitude_bits(),
                        ),
                )
                .map_err(|_| ReservationDenied)
        }
    }

    impl ModelRestoreBudget for Budget {
        fn meter(&mut self) -> &mut Meter {
            &mut self.meter
        }
    }

    /// Trace: FSpec-FR-181
    #[test]
    fn reference_identity_preserves_unicode_and_hex_looking_text() {
        let mut budget = Budget::new(4096, 4096);
        let cancel = Cancel::new();
        let mut scope = Scope {
            budget: &mut budget,
            cancel: &cancel,
            normalization_owner: None,
            path: String::new(),
            max_depth: 64,
        };
        assert_eq!(scope.hex_encode("雪🙂ab").unwrap(), "e99baaf09f99826162");
        assert_eq!(scope.hex_decode("e99baaf09f99826162").unwrap(), "雪🙂ab");
        assert_eq!(scope.hex_encode("ab").unwrap(), "6162");
        assert_eq!(scope.hex_decode("6162").unwrap(), "ab");
    }

    /// Trace: FSpec-FR-181
    #[test]
    fn reference_identity_refuses_noncanonical_hex_and_non_utf8_octets() {
        for malformed in ["", "0", "0A", "gg", "ff", "c080", "eda080"] {
            let mut budget = Budget::new(4096, 4096);
            let cancel = Cancel::new();
            let mut scope = Scope {
                budget: &mut budget,
                cancel: &cancel,
                normalization_owner: None,
                path: "/identity".to_owned(),
                max_depth: 64,
            };
            assert!(
                matches!(scope.hex_decode(malformed), Err(ReferenceCodecError::Refused { reason: ReferenceFailure::InvalidIdentity, pointer }) if pointer == "/identity"),
                "{malformed}"
            );
        }
    }

    /// Trace: FSpec-FR-181
    #[test]
    fn reference_identity_denies_storage_and_work_before_output_allocation() {
        for (storage, work) in [(0, 4096), (4096, 0)] {
            let mut budget = Budget::new(storage, work);
            let cancel = Cancel::new();
            let mut scope = Scope {
                budget: &mut budget,
                cancel: &cancel,
                normalization_owner: None,
                path: "/identity".to_owned(),
                max_depth: 64,
            };
            assert!(
                matches!(scope.hex_encode("雪"), Err(ReferenceCodecError::Key { source: ModelKeyError::Denied, pointer }) if pointer == "/identity")
            );
            assert_eq!(budget.storage_used, 0);
        }
    }

    /// Trace: FSpec-FR-181
    #[test]
    fn reference_closed_wire_charges_member_text_before_comparing_it() {
        let mut map = serde_json::Map::new();
        map.insert("x".repeat(1024), serde_json::Value::Null);
        let value = serde_json::Value::Object(map);
        let mut budget = Budget::new(0, 1);
        let cancel = Cancel::new();
        let mut scope = Scope {
            budget: &mut budget,
            cancel: &cancel,
            normalization_owner: None,
            path: "/argument".to_owned(),
            max_depth: 64,
        };
        assert!(
            matches!(scope.closed(&value, &["type"]), Err(ReferenceCodecError::Key { source: ModelKeyError::Denied, pointer }) if pointer == "/argument")
        );
        assert_eq!(budget.work_left, 0);
        assert_eq!(budget.storage_used, 0);
    }

    /// Trace: FSpec-FR-181, FSpec-FR-418-AC-11
    #[test]
    fn call_reference_adapter_uses_checked_admission_and_environment() {
        use super::{reference_fixture, ReferenceCodec};
        use filament_semantic_value::model_transition::ModelTransition;
        use fsl_eval::execution::{PrepareLimits, SuppliedBackends};
        use fsl_package::ReferenceCatalog;
        use fsl_semantics::model::intake::{package_input, SelectedModels};
        use fsl_semantics::model::observation::{
            admit_current_snapshot, DocumentRef, ObservationLimits,
        };
        use serde_json::json;
        use std::collections::BTreeMap;

        let packages = package_input([reference_fixture::DOMAIN_PACKAGE.as_bytes()]).unwrap();
        let source = fsl_foundation::SourceIdentity::new("test", "checked-call-reference");
        let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nmodel Config = {:?};\nfunction keep using v(x: Config::ConfigVersion): Config::ConfigVersion pure {{ x }}\n", reference_fixture::PACKAGE_IDENTITY);
        let limits = crate::spine::SpineLimits::default();
        let compiled = crate::spine::compose(
            source,
            "checked-call.native",
            text.as_bytes(),
            &packages,
            &crate::spine::DependencyInput::default(),
            limits,
        )
        .expect("actual function checking and model admission");
        let checked = &compiled.package;
        let models = compiled.models.selected_models();
        let population = "ix://example/config-version/config_history";
        let selected = DocumentRef {
            authority: "test".to_owned(),
            identity: "checked-call-current".to_owned(),
        };
        let snapshot = json!({
            "format": "filament.state.snapshot/v1",
            "identity": {"authority": selected.authority, "identity": selected.identity},
            "observation": "current", "anchor": {"kind": "handler", "name": "read"},
            "model": {"identity": reference_fixture::PACKAGE_IDENTITY},
            "populations": [{"population": population, "complete": true, "objects": [{
                "key": "雪call", "type": "ix://example/config-version/ConfigVersion",
                "fields": {"versionNumber": {"integer": "2"}, "parent": {"absent": {}}}
            }]}]
        });
        let provisions =
            BTreeMap::from([(selected.clone(), serde_json::to_vec(&snapshot).unwrap())]);
        let (objects, _) = admit_current_snapshot(
            checked.graph().model_inputs(),
            checked.graph().scope().types(),
            &packages,
            limits.model,
            &provisions,
            &selected,
            ObservationLimits::default(),
        )
        .expect("actual call object environment");
        let [native] = objects.references() else {
            panic!("one admitted reference");
        };
        let call = crate::spine::Call::new(
            "keep".to_owned(),
            vec![crate::spine::CallArgument {
                parameter: "x".to_owned(),
                value: filament_exact::Value::Reference(native.clone()),
            }],
            crate::spine::default_accounting(1_000_000),
            filament_semantic_value::supplied_stop::SuppliedLimits::default(),
        );
        let cancel = Cancel::new();
        let outcome = crate::spine::run_compiled(
            &compiled.checked,
            &compiled.models,
            crate::spine::CallSources {
                package_name: checked.graph().package_name(),
                packages: &packages,
                model_limits: limits.model,
            },
            &call,
            &objects,
            &SuppliedBackends::default(),
            &PrepareLimits::default(),
            &cancel,
        )
        .expect("direct checked function execution");
        let crate::spine::CallOutcome::Completed(crate::spine::CallValue::Reference(result)) =
            outcome
        else {
            panic!("a reference-valued completed call");
        };
        assert_eq!(&result, native);
        let mut join_budget = Budget::new(1_000_000, 1_000_000);
        let catalog = ReferenceCatalog::from_checked_call(checked, models, &mut join_budget)
            .expect("genuine checked call catalog");
        assert!(catalog.package().is_none());
        assert!(catalog.model().is_none());
        let codec = ReferenceCodec::for_checked_call(&catalog, checked, models, &objects).unwrap();
        let mut storage = Budget::new(1_000_000, 1_000_000);
        // An erased core work/storage budget cannot be downcast to a model
        // simulation/restoration budget. This is the actual call adapter API.
        let budget: &mut dyn IntegerDecimalBudget = &mut storage;
        let wire = codec
            .encode_reference(&result, budget, &cancel, 64)
            .unwrap();
        assert_eq!(wire["identity"], "e99baa63616c6c");
        assert_eq!(
            codec.restore_reference(&wire, budget, &cancel, 64).unwrap(),
            result
        );
        // A genuine call-admitted reference also crosses both task-template
        // transition arms. This exercises the shared operation adapter and
        // nested refusal pointer rather than constructing catalog evidence.
        let native_text = serde_json::to_string(
            &fsl_eval::simulation::model::value_json(&filament_exact::Value::Reference(
                result.clone(),
            ))
            .unwrap(),
        )
        .unwrap();
        let transition = ModelTransition::new(
            r#"{"name":"keep"}"#.to_owned(),
            Some(native_text.clone()),
            vec![native_text],
            quire_canonical::Limits::new(100000),
        )
        .unwrap();
        let template = crate::TaskTransitionTemplate {
            action: crate::TaskTransitionAction::Release,
            task_set: "tasks".to_owned(),
            task: "task".to_owned(),
            job_rank: 0,
            slot: 0,
            release: transition.clone(),
            source_transition: Some(transition),
            priority: Some(1),
        };
        let mut carrier_budget = Budget::new(10_000_000, 10_000_000);
        let mut carrier = codec.scope(
            &mut carrier_budget,
            &cancel,
            64,
            quire_canonical::Limits::new(100000),
        );
        let mut task_wire = carrier.encode_task_template(&template).unwrap();
        assert_eq!(task_wire["release"]["operation"], "keep");
        assert_eq!(task_wire["release"]["receiver"], wire);
        assert_eq!(task_wire["source_transition"]["arguments"][0], wire);
        let restored = carrier.restore_task_template(&task_wire).unwrap();
        assert!(template.same_identity(&restored));
        let original_wire = template
            .to_wire()
            .expect("unchanged unscoped source spelling");
        assert!(template
            .same_identity(&crate::TaskTransitionTemplate::from_wire(&original_wire).unwrap()));
        task_wire["source_transition"]["arguments"][0]["identity"] = json!("FF");
        assert!(
            matches!(carrier.restore_task_template(&task_wire), Err(ReferenceCodecError::Refused { reason: ReferenceFailure::InvalidIdentity, pointer }) if pointer == "/source_transition/arguments/0/identity")
        );
        // The certificate codec transports this opaque key; this is a carrier
        // roundtrip, not a claim that its graph proves an obligation.
        let key = fsl_eval::simulation::StateKey::from_canonical_bytes(b"null".to_vec());
        let certificate = crate::ZoneCertificate {
            obligation: (),
            scale: filament_exact::Integer::from(1_i64),
            lu_bounds: crate::LuBounds {
                lower: Vec::new(),
                upper: Vec::new(),
            },
            nodes: vec![crate::CertNode {
                state_key: key.clone(),
                automaton_state: 0,
                zone: crate::Zone::zero(1),
                locked: false,
                product: None,
            }],
            edges: vec![
                crate::CertEdge {
                    from: 0,
                    to: 0,
                    transition: Some(template.release.clone()),
                    task_transition: None,
                    part: None,
                    horizon: None,
                    silent: None,
                },
                crate::CertEdge {
                    from: 0,
                    to: 0,
                    transition: None,
                    task_transition: Some(template),
                    part: None,
                    horizon: None,
                    silent: None,
                },
            ],
            initials: vec![0],
            components: None,
            non_zeno: None,
            divergence: None,
            admitted: None,
            state_graph: None,
            refinement: None,
            task_product: None,
        };
        let certificate_wire = certificate
            .to_wire_with_references(json!({}), None, &mut carrier)
            .unwrap();
        assert_eq!(certificate_wire["edges"][0]["transition"]["receiver"], wire);
        assert_eq!(
            certificate_wire["edges"][1]["transition"]["release"]["receiver"],
            wire
        );
        let (restored_certificate, identity) = crate::ZoneCertificate::from_wire_with_references(
            &certificate_wire,
            (),
            None,
            &mut carrier,
        )
        .unwrap();
        assert_eq!(
            restored_certificate.nodes[0].state_key.as_bytes(),
            key.as_bytes()
        );
        assert_eq!(
            restored_certificate
                .to_wire_with_references(identity, None, &mut carrier)
                .unwrap(),
            certificate_wire
        );
        let mut malformed_certificate = certificate_wire;
        malformed_certificate["edges"][1]["transition"]["release"]["receiver"]["identity"] =
            json!("FF");
        assert!(
            matches!(crate::ZoneCertificate::from_wire_with_references(&malformed_certificate, (), None, &mut carrier), Err(crate::ZoneCertificateReferenceError::Reference(ReferenceCodecError::Refused { reason: ReferenceFailure::InvalidIdentity, pointer })) if pointer == "/edges/1/transition/release/receiver/identity")
        );
        let graph = crate::model_check::state_graph::GraphEvidence::Trap {
            stem: crate::model_check::state_graph::ModelPath {
                initial: 0,
                steps: vec![crate::model_check::counterexample::ModelStep {
                    transition: certificate.edges[0].transition.clone().unwrap(),
                    post_state: key.clone(),
                    delay: Some(filament_exact::Rational::from_integer(
                        filament_exact::Integer::from(2_i64),
                    )),
                }],
            },
            closure: crate::model_check::state_graph::TrapClosure::StateGraph,
            over: None,
        };
        let graph_wire = carrier
            .graph_evidence_wire(
                &graph,
                crate::StateGraphPropertyForm::Possible,
                Some("x"),
                Some(&result),
                &[],
                None,
            )
            .unwrap();
        assert_eq!(graph_wire["instance"]["key"], wire);
        assert_eq!(
            graph_wire["stem"]["steps"][0]["transition"]["receiver"],
            wire
        );
        assert_eq!(
            graph_wire["stem"]["steps"][0]["transition"]["arguments"][0],
            wire
        );
        assert_eq!(
            graph_wire["stem"]["steps"][0]["post_state"],
            crate::state_key_wire(&key).unwrap()["key"]
        );
        assert_eq!(
            graph_wire["stem"]["steps"][0]["delay"],
            json!({"numerator":"2","denominator":"1"})
        );
        let foreign: SelectedModels = models.clone();
        assert!(matches!(
            ReferenceCodec::for_checked_call(&catalog, checked, &foreign, &objects),
            Err(ReferenceCodecError::Refused {
                reason: ReferenceFailure::ForeignCatalog,
                ..
            })
        ));
        let mut denied = Budget::new(0, 1_000_000);
        assert!(matches!(
            codec.encode_reference(native, &mut denied, &cancel, 64),
            Err(ReferenceCodecError::Key {
                source: ModelKeyError::Denied,
                ..
            })
        ));
        assert_eq!(denied.storage_used, 0);
    }

    /// Trace: FSpec-FR-181, FSpec-FR-364
    #[test]
    fn scoped_temporal_and_protocol_retain_genuine_key_population_and_borrowed_fields() {
        use super::{reference_fixture, ReferenceCodec};
        use filament_checked::key::DeclarationKey;
        use fsl_eval::simulation::model::{
            ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem,
            PopulationUniverse,
        };
        use fsl_package::ReferenceCatalog;
        use fsl_semantics::model::observation::{DocumentRef, ObservationLimits};
        use serde_json::json;
        use std::collections::BTreeMap;
        let packages = fsl_semantics::model::intake::package_input([
            reference_fixture::DOMAIN_PACKAGE.as_bytes(),
        ])
        .unwrap();
        let source = fsl_foundation::SourceIdentity::new("test", "scoped-temporal");
        let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nmodel Config = {:?};\nfunction keep using v(x: Config::ConfigVersion): Config::ConfigVersion pure {{ x }}\n", reference_fixture::PACKAGE_IDENTITY);
        let compiled = crate::spine::compose(
            source,
            "temporal.native",
            text.as_bytes(),
            &packages,
            &crate::spine::DependencyInput::default(),
            crate::spine::SpineLimits::default(),
        )
        .unwrap();
        let checked = &compiled.package;
        let population = DeclarationKey {
            package: reference_fixture::PACKAGE_IDENTITY.to_owned(),
            node: "ix://example/config-version/config_history".to_owned(),
        };
        let object_type = DeclarationKey {
            package: population.package.clone(),
            node: "ix://example/config-version/ConfigVersion".to_owned(),
        };
        let selected = DocumentRef {
            authority: "test".to_owned(),
            identity: "temporal-initial".to_owned(),
        };
        let snapshot = json!({"format":"filament.state.snapshot/v1","identity":{"authority":selected.authority,"identity":selected.identity},"observation":"current","anchor":{"kind":"initialization","name":"start"},"model":{"identity":reference_fixture::PACKAGE_IDENTITY},"populations":[{"population":population.node,"complete":true,"objects":[{"key":"雪temporal","type":object_type.node,"fields":{"versionNumber":{"integer":"2"},"parent":{"absent":{}}}}]}]});
        let snapshots =
            BTreeMap::from([(selected.clone(), serde_json::to_vec(&snapshot).unwrap())]);
        let system = ModelSystem::new(ModelSimulationRequest {
            input: ModelCheckInput::source(checked),
            packages: &packages,
            normalization: fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
            snapshots: &snapshots,
            initial: vec![selected.clone()],
            universes: vec![PopulationUniverse {
                population: population.clone(),
                keys: vec!["雪temporal".to_owned()],
            }],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 10000,
            },
            meter: crate::spine::default_accounting(1_000_000),
            emit: fsl_package::EmitLimits::new(16_777_216),
        })
        .unwrap();
        let (initial, _) = system
            .reference_model()
            .admit_initial(&snapshots, &selected, ObservationLimits::default())
            .unwrap();
        let (_, terminal) = system.state_key_of(&initial).unwrap();
        let native = system.object_reference(&object_type, "雪temporal").unwrap();
        let mut budget = Budget::new(10_000_000, 10_000_000);
        let catalog =
            ReferenceCatalog::from_checked_model(checked, system.reference_model(), &mut budget)
                .unwrap();
        let codec = ReferenceCodec::new(&catalog, &system).unwrap();
        let cancel = Cancel::new();
        let mut scope = codec.scope(
            &mut budget,
            &cancel,
            64,
            quire_canonical::Limits::new(100000),
        );
        let transition = fsl_eval::simulation::protocol::ProtocolTransition::fork(
            fsl_eval::simulation::protocol::ThreadPath::root(7).child("fork/雪", "left", 3),
            "join/雪",
        );
        let protocol = crate::model_check::counterexample::TemporalCounterexample {
            initial_state: 0,
            prefix: vec![crate::model_check::counterexample::ModelStep {
                transition: transition.clone(),
                post_state: terminal.clone(),
                delay: None,
            }],
            over_binding: None,
            fairness: Vec::new(),
            kind: crate::model_check::counterexample::CounterexampleKind::Formula,
            lasso: None,
            blocked: Vec::new(),
            refinement: None,
            final_delay: None,
        };
        let subject = crate::ProtocolWire {
            protocol: "real-runtime-projection".to_owned(),
            over: native.clone(),
            roles: vec![("role/雪".to_owned(), object_type.package.clone())],
            capacities: vec![("queue~雪".to_owned(), 2)],
            max_live_instances: Some(7),
        };
        let written = scope
            .protocol_temporal_wire(&protocol, &subject, None, None)
            .unwrap();
        assert_eq!(
            written["prefix"][0]["transition"],
            fsl_eval::simulation::protocol::transition_json(&transition).unwrap()
        );
        assert_eq!(
            written["prefix"][0]["post_state"],
            crate::state_key_wire(&terminal).unwrap()
        );
        assert_eq!(
            written["subject"]["over"],
            scope.encode_reference(&native).unwrap()
        );
        assert_eq!(written["subject"]["roles"]["role/雪"], object_type.package);
        assert_eq!(written["subject"]["capacities"]["queue~雪"], "2");
        let mut witness = crate::model_check::counterexample::TemporalCounterexample {
            initial_state: 0,
            prefix: Vec::new(),
            over_binding: Some(fsl_eval::model_subject::ModelInstance {
                population,
                key: "雪temporal".to_owned(),
            }),
            fairness: Vec::new(),
            kind: crate::model_check::counterexample::CounterexampleKind::Formula,
            lasso: Some(crate::model_check::counterexample::Lasso {
                entry: 0,
                terminal_stutter: true,
                terminal_stutter_delay: None,
            }),
            blocked: Vec::new(),
            refinement: None,
            final_delay: None,
        };
        let written = scope
            .temporal_wire(&witness, Some(&native), None, Some(&terminal))
            .unwrap();
        assert!(written["prefix"].as_array().unwrap().is_empty());
        assert_eq!(
            written["loop"][0]["transition"],
            json!({"type":"terminal-stutter"})
        );
        assert_eq!(
            written["loop"][0]["post_state"],
            crate::state_key_wire(&terminal).unwrap()
        );
        assert_eq!(
            written["over_binding"],
            scope.encode_reference(&native).unwrap()
        );
        assert!(scope
            .temporal_wire(&witness, Some(&native), None, None)
            .is_err());
        witness.over_binding = None;
        assert!(scope
            .temporal_wire(&witness, Some(&native), None, Some(&terminal))
            .unwrap()["over_binding"]
            .is_null());
        witness.lasso.as_mut().unwrap().terminal_stutter_delay = Some(
            filament_exact::Rational::from_integer(filament_exact::Integer::zero()),
        );
        assert!(scope
            .temporal_wire(&witness, None, None, Some(&terminal))
            .is_err());
        let mut denied = Budget::new(0, 10_000_000);
        let mut scope = codec.scope(
            &mut denied,
            &cancel,
            64,
            quire_canonical::Limits::new(100000),
        );
        assert!(matches!(
            scope.temporal_wire(&witness, None, None, Some(&terminal)),
            Err(ReferenceCodecError::Key {
                source: ModelKeyError::Denied,
                ..
            })
        ));
        assert_eq!(denied.storage_used, 0);
    }

    /// Trace: FSpec-FR-181
    #[test]
    fn reference_catalog_genuine_config_version_source_and_admitted_roundtrip() {
        use super::{reference_fixture, ReferenceCodec};
        use filament_checked::key::DeclarationKey;
        use filament_contract_model::NativeJoinBudget;
        use fsl_eval::model_subject::ModelInstance;
        use fsl_eval::simulation::model::{
            ExpansionLimits, ModelCheckInput, ModelSimulationRequest, ModelSystem,
            PackageModelInput, PopulationUniverse,
        };
        use fsl_package::{BarePackage, EmitLimits, ReferenceCatalog, V2ReadLimits};
        use fsl_semantics::library::LibraryName;
        use fsl_semantics::model::accounting::ModelNormalizationLimits;
        use fsl_semantics::model::intake::{admit_unit, package_input};
        use fsl_semantics::model::observation::{
            admit_current_snapshot_with_provenance, admitted_population_document,
            declared_type_universe_for, DocumentRef, ObservationLimits,
        };
        use serde_json::{json, Value};
        use std::collections::BTreeMap;

        let mut document: Value = serde_json::from_str(reference_fixture::DOMAIN_PACKAGE).unwrap();
        let population = document["populations"][0]["identity"]
            .as_str()
            .unwrap()
            .to_owned();
        let other_population = format!("{population}-second");
        let mut second = document["populations"][0].clone();
        second["identity"] = json!(other_population);
        second["displayName"] = json!(other_population);
        second["origin"]["generated"]["generatorIdentity"] = json!(other_population);
        second["origin"]["generated"]["inputIdentities"] = json!([other_population]);
        document["populations"].as_array_mut().unwrap().push(second);
        let bytes = serde_json::to_vec(&document).unwrap();
        let packages =
            package_input([bytes.as_slice()]).expect("real ConfigVersion model admission");
        let source =
            fsl_foundation::SourceIdentity::new("test", "reference-catalog-config-version");
        let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nmodel Config = {:?};\nfunction keep using v(x: Config::ConfigVersion): Config::ConfigVersion pure {{ x }}\n", reference_fixture::PACKAGE_IDENTITY);
        let limits = crate::spine::SpineLimits::default();
        let cancel = Cancel::new();
        let parsed = crate::spine::parse(
            &crate::spine::ParseRequest {
                source: &source,
                path: "reference.native",
                bytes: text.as_bytes(),
            },
            limits.source,
            &cancel,
        )
        .expect("real source parsing")
        .into_value();
        let models = admit_unit(&parsed.unit().selections().models, &packages, limits.model)
            .expect("real model-stage identities and retained rows");
        let compiled = crate::spine::compose(
            source,
            "reference.native",
            text.as_bytes(),
            &packages,
            &crate::spine::DependencyInput::default(),
            limits,
        )
        .expect("real source checking and S4 link");
        let checked = &compiled.package;
        let type_key = DeclarationKey {
            package: reference_fixture::PACKAGE_IDENTITY.to_owned(),
            node: format!("ix://{}/ConfigVersion", reference_fixture::PACKAGE_IDENTITY),
        };
        let selected = DocumentRef {
            authority: "test".to_owned(),
            identity: "reference-initial".to_owned(),
        };
        let object_key = "雪🙂ab";
        let objects = json!([
            {"key": "a", "type": type_key.node, "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": {}}}},
            {"key": object_key, "type": type_key.node, "fields": {"versionNumber": {"integer": "2"}, "parent": {"absent": {}}}}
        ]);
        let snapshot = json!({"format": "filament.state.snapshot/v1", "identity": {"authority": selected.authority, "identity": selected.identity}, "observation": "current", "anchor": {"kind": "initialization", "name": "start"}, "model": {"identity": reference_fixture::PACKAGE_IDENTITY}, "populations": [
            {"population": population, "complete": true, "objects": objects},
            {"population": other_population, "complete": true, "objects": []}
        ]});
        let provisions =
            BTreeMap::from([(selected.clone(), serde_json::to_vec(&snapshot).unwrap())]);
        let universes = || {
            vec![
                PopulationUniverse {
                    population: DeclarationKey {
                        package: type_key.package.clone(),
                        node: population.clone(),
                    },
                    keys: vec!["a".to_owned(), object_key.to_owned()],
                },
                PopulationUniverse {
                    population: DeclarationKey {
                        package: type_key.package.clone(),
                        node: other_population.clone(),
                    },
                    keys: vec!["a".to_owned(), object_key.to_owned()],
                },
            ]
        };
        fn request<'a>(
            input: ModelCheckInput<'a>,
            packages: &'a fsl_semantics::model::intake::PackageInput,
            snapshots: &'a fsl_semantics::model::observation::DocumentProvision,
            initial: DocumentRef,
            universes: Vec<PopulationUniverse>,
        ) -> ModelSimulationRequest<'a> {
            ModelSimulationRequest {
                input,
                packages,
                normalization: ModelNormalizationLimits::default(),
                snapshots,
                initial: vec![initial],
                universes,
                observation: ObservationLimits::default(),
                expansion: ExpansionLimits {
                    max_candidates: 10000,
                },
                meter: crate::spine::default_accounting(1_000_000),
                emit: EmitLimits::new(16_777_216),
            }
        }
        let system = ModelSystem::new(request(
            ModelCheckInput::source(checked),
            &packages,
            &provisions,
            selected.clone(),
            universes(),
        ))
        .expect("actual source model system");
        let emission = fsl_package::emit_checked(checked, EmitLimits::new(16_777_216))
            .expect("genuine source emission");
        let read = fsl_package::read_evaluation_emission(
            checked,
            &emission,
            LibraryName::new(checked.graph().package_name().to_owned()).unwrap(),
            &packages,
            V2ReadLimits::bounded(16_777_216),
            EmitLimits::new(16_777_216),
            fsl_package::SourceWitnessSupplier::new(&emission),
        );
        let read = read.expect("source emission and strict package admission");
        let mut joins = NativeJoinBudget::new(10_000_000, 10_000_000);
        let source_catalog =
            ReferenceCatalog::from_source(&read, system.reference_model(), &mut joins)
                .expect("source-authenticated catalog");
        let codec =
            ReferenceCodec::new(&source_catalog, &system).expect("actual runtime owner binding");
        let native = system
            .object_reference(&type_key, object_key)
            .expect("genuine native object reference");
        let mut budget = Budget::new(16_000_000, 16_000_000);
        let wire = codec
            .encode_reference(&native, &mut budget, &cancel, 64)
            .unwrap();
        assert_eq!(wire["identity"], "e99baaf09f99826162");
        let actual_node = read
            .admitted()
            .resolved_graph()
            .nodes
            .iter()
            .find(|node| {
                node.node_tag.as_ref() == "model" && node.semantic_form.as_ref() == "object_type"
            })
            .expect("actual admitted declaration");
        assert_eq!(
            wire["object_type"],
            serde_json::to_value(&actual_node.node_id).unwrap()
        );
        assert_eq!(
            codec
                .restore_reference(&wire, &mut budget, &cancel, 64)
                .unwrap(),
            native
        );
        let universe = declared_type_universe_for(
            checked.graph().model_inputs(),
            &packages,
            limits.model,
            &type_key,
        )
        .unwrap()
        .unwrap()
        .1;
        assert_eq!(native.universe(), &universe);
        let initial = system
            .reference_model()
            .admit_initial(&provisions, &selected, ObservationLimits::default())
            .unwrap()
            .0;
        let before = system.state_key_of(&initial).unwrap().0;
        let native_json: Value = serde_json::from_str(
            &fsl_eval::simulation::model::reference_canonical_text(&native).unwrap(),
        )
        .unwrap();
        let nested = json!({"type": "option", "value": {"type": "tuple", "name": "Carrier", "components": [native_json]}});
        let encoded = codec
            .encode_value(&nested, &mut budget, &cancel, 64)
            .unwrap();
        assert_eq!(
            codec
                .restore_value(&encoded, &mut budget, &cancel, 64)
                .unwrap(),
            nested
        );
        let operation_node = document["types"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|ty| ty["operations"].as_array().into_iter().flatten())
            .find(|operation| operation["name"] == "attemptUpdate")
            .unwrap()["identity"]
            .as_str()
            .unwrap()
            .to_owned();
        let operation = fsl_eval::simulation::model::operation_identity_text(&DeclarationKey {
            package: type_key.package.clone(),
            node: operation_node,
        })
        .unwrap();
        let transition = filament_semantic_value::model_transition::ModelTransition::new(
            operation,
            Some(fsl_eval::simulation::model::reference_canonical_text(&native).unwrap()),
            vec![canonical_text_for_test(&nested)],
            quire_canonical::Limits::new(100000),
        )
        .unwrap();
        let encoded = codec
            .encode_transition(&transition, &mut budget, &cancel, 64)
            .unwrap();
        assert_eq!(
            codec
                .restore_transition(
                    &encoded,
                    &mut budget,
                    &cancel,
                    64,
                    quire_canonical::Limits::new(100000)
                )
                .unwrap(),
            transition
        );
        assert_eq!(system.state_key_of(&initial).unwrap().0, before);
        let (environment, _, admitted_models) = admit_current_snapshot_with_provenance(
            checked.graph().model_inputs(),
            checked.graph().scope().types(),
            &packages,
            limits.model,
            &provisions,
            &selected,
            ObservationLimits::default(),
        )
        .expect("genuine admitted population bindings");
        let selection = ModelInstance {
            population: DeclarationKey {
                package: type_key.package.clone(),
                node: population.clone(),
            },
            key: object_key.to_owned(),
        };
        let [admitted_model] = admitted_models.as_slice() else {
            panic!("the fixture selects exactly its original Config model");
        };
        let document = admitted_population_document(
            admitted_model.view(),
            checked.graph().scope().types(),
            &environment,
            &population,
            &["a", object_key],
        )
        .expect("retain the actual selected population's admitted fields");
        use fsl_semantics::model::population::{
            admit_binding, AdmissionMeter, AdmissionOutcome, PopulationAdmissionLimits,
        };
        let mut population_meter = AdmissionMeter::new(PopulationAdmissionLimits::default());
        let AdmissionOutcome::Admitted(binding) = admit_binding(
            admitted_model.view(),
            &document,
            &selection.population,
            fsl_semantics::model::dispatch::GeneralizationClosure::Closed,
            None,
            &mut population_meter,
        ) else {
            panic!("the selected population must pass native binding admission");
        };
        let alternate_ref = DocumentRef {
            authority: selected.authority.clone(),
            identity: "reference-alternate".to_owned(),
        };
        let mut alternate = snapshot.clone();
        alternate["identity"]["identity"] = json!(alternate_ref.identity);
        alternate["populations"][0]["objects"] = json!([]);
        alternate["populations"][1]["objects"] = objects;
        let alternate_provisions = BTreeMap::from([(
            alternate_ref.clone(),
            serde_json::to_vec(&alternate).unwrap(),
        )]);
        let (alternate_environment, _, alternate_models) = admit_current_snapshot_with_provenance(
            checked.graph().model_inputs(),
            checked.graph().scope().types(),
            &packages,
            limits.model,
            &alternate_provisions,
            &alternate_ref,
            ObservationLimits::default(),
        )
        .expect("other population's genuine snapshot admission");
        let [alternate_model] = alternate_models.as_slice() else {
            panic!("the alternate snapshot selects exactly its original Config model");
        };
        let document = admitted_population_document(
            alternate_model.view(),
            checked.graph().scope().types(),
            &alternate_environment,
            &other_population,
            &["a", object_key],
        )
        .expect("retain the explicitly selected alternate population's fields");
        let alternate_population = DeclarationKey {
            package: type_key.package.clone(),
            node: other_population.clone(),
        };
        let AdmissionOutcome::Admitted(other_binding) = admit_binding(
            alternate_model.view(),
            &document,
            &alternate_population,
            fsl_semantics::model::dispatch::GeneralizationClosure::Closed,
            None,
            &mut population_meter,
        ) else {
            panic!("the alternate population must pass native binding admission");
        };
        assert_eq!(binding.universe(), other_binding.universe());
        assert_eq!(
            codec
                .restore_over_binding(&wire, Some(&selection), &binding, &mut budget, &cancel, 64)
                .unwrap(),
            selection
        );
        assert!(matches!(
            codec.restore_over_binding(&wire, None, &binding, &mut budget, &cancel, 64),
            Err(ReferenceCodecError::Refused {
                reason: ReferenceFailure::MissingPopulation,
                ..
            })
        ));
        assert!(matches!(
            codec.restore_over_binding(
                &wire,
                Some(&selection),
                &other_binding,
                &mut budget,
                &cancel,
                64
            ),
            Err(ReferenceCodecError::Refused {
                reason: ReferenceFailure::WrongPopulation,
                ..
            })
        ));
        let rows = emission
            .identity_rows(read.admitted(), &mut joins)
            .expect("the original emission's admitted identity supply");
        let equality = emission
            .equality_witness(read.admitted(), &mut joins)
            .expect("the original emission's admitted equality supply");
        let bare = BarePackage::new(read.admitted())
            .with_supply(
                fsl_package::BareSupply {
                    identities: &rows,
                    equality: Some(&equality),
                },
                &mut joins,
            )
            .expect("the authentic emitted package supply joins");
        let package_input = PackageModelInput::of(&bare, &packages, limits.model)
            .expect("real admitted package model input");
        let package_system = ModelSystem::new(request(
            package_input.input(),
            &packages,
            &provisions,
            selected.clone(),
            universes(),
        ))
        .expect("actual admitted-route system");
        let admitted_catalog = ReferenceCatalog::from_admitted(
            read.admitted(),
            &models,
            package_system.reference_model(),
            &mut joins,
        )
        .expect("model-stage authenticated admitted catalog");
        let admitted_codec = ReferenceCodec::new(&admitted_catalog, &package_system).unwrap();
        assert_eq!(
            admitted_codec
                .encode_reference(&native, &mut budget, &cancel, 64)
                .unwrap(),
            wire
        );
        assert_eq!(
            admitted_codec
                .restore_reference(&wire, &mut budget, &cancel, 64)
                .unwrap(),
            native
        );
        assert!(matches!(
            ReferenceCodec::new(&admitted_catalog, &system),
            Err(ReferenceCodecError::Refused {
                reason: ReferenceFailure::ForeignCatalog,
                ..
            })
        ));
    }

    fn canonical_text_for_test(value: &serde_json::Value) -> String {
        String::from_utf8(
            quire_canonical::to_vec(value, quire_canonical::Limits::new(100000)).unwrap(),
        )
        .unwrap()
    }
}
