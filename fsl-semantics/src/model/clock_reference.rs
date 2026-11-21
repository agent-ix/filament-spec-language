// SPDX-License-Identifier: AGPL-3.0-or-later
//! The place-id codec shared by clock-reference evaluation and admission.

use filament_exact::{NodeKey, ObjectId, ObjectReference, Value};

/// Route-neutral declared clock identity and its canonical text grammar.
pub mod identity;
/// Authenticated source declaration spelling shared by emission and clock ids.
pub mod normalization;
/// Normalized typed clock paths shared by the source and admitted routes.
pub mod path;

/// Whether the target type is the native clock reference's exact target.
pub fn is_clock_type(target: &filament_exact::EffectiveId) -> bool {
    target == &crate::check::clock_reference_target()
}

/// A malformed clock place identity or selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid clock place identity")]
pub struct InvalidClockReference;

/// The decoded instance named by a clock place id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockPlaceId {
    /// The key of the object that owns the place.
    pub owner: String,
    /// The checked model's clock-place template.
    pub template: NodeKey,
    /// The nonnegative slot positions, in path order.
    pub selectors: Vec<u64>,
}

/// Checked declaration context retained for the trusted same-run Rust-state
/// seam. This is not admission authority for externally supplied references.
#[derive(Clone, Debug, Default)]
pub struct DeclaredClockPlaces {
    declarations: normalization::ClockDeclarationTable,
}

impl DeclaredClockPlaces {
    /// A context without original declaration provenance cannot validate a
    /// transported v2 identity. Callers with an admitted table attach it below.
    pub fn new(_models: &[crate::check::TimedModel]) -> Self {
        Self::default()
    }

    /// Retain the exact checked declaration table once for model observations.
    pub fn with_declaration_table(
        _models: &[crate::check::TimedModel],
        table: &normalization::ClockDeclarationTable,
    ) -> Self {
        Self {
            declarations: table.clone(),
        }
    }

    /// Check a reference already produced or admitted within this model run.
    /// The caller has the real owner from that run. Population membership of
    /// hostile raw input belongs to raw admission, never to this trusted seam.
    pub fn contains(
        &self,
        reference: &ObjectReference,
        owner: &ObjectReference,
        types: &filament_semantic_value::declaration::TypeEnvironment,
    ) -> bool {
        if !is_clock_type(reference.object_type()) || reference.universe() != owner.universe() {
            return false;
        }
        let text = reference.object().as_str();
        let limit = u64::try_from(text.len()).unwrap_or(u64::MAX);
        let Ok(identity) =
            identity::PlaceIdentity::decode(text, quire_canonical::Limits::new(limit))
        else {
            return false;
        };
        is_declared_identity(
            &identity,
            &identity.population,
            owner,
            &self.declarations,
            types,
        )
    }
}

/// Validate a decoded identity against an explicitly authenticated population,
/// real owner and the checked declaration table. This never searches for a
/// population by an owner's type, universe or local key.
///
/// The outer K2 universe and native Clock target are retained by
/// [`encode_identity`]; callers validating an existing typed reference check
/// those outer fields as well. Raw admission obtains them from its real owner.
pub fn is_declared_identity(
    identity: &identity::PlaceIdentity,
    population: &identity::DeclarationScope,
    owner: &ObjectReference,
    declarations: &normalization::ClockDeclarationTable,
    types: &filament_semantic_value::declaration::TypeEnvironment,
) -> bool {
    if &identity.population != population
        || identity.owner != owner.object().as_str()
        || declarations.origin(&identity.template).is_none()
    {
        return false;
    }
    if !declarations.run_places().any(|(_, place)| {
        place.template.as_ref() == &identity.template
            && types.conforms(owner.object_type().clone(), place.owner.clone())
    }) {
        return false;
    }
    let mut indices = identity.indices.iter();
    for (position, step) in identity.template.path.iter().enumerate() {
        let identity::PathEntry::Collection { shape, .. } = step else {
            continue;
        };
        let Some(index) = indices.next() else {
            return false;
        };
        if index.path_position.as_str() != position.to_string() {
            return false;
        }
        if let Some(maximum) = &shape.maximum {
            // Canonical nonnegative decimal metadata orders by digit count,
            // then bytes. No selector is truncated to a machine integer.
            let index = index.index.as_str();
            let maximum = maximum.as_str();
            if (index.len(), index) >= (maximum.len(), maximum) {
                return false;
            }
        }
    }
    indices.next().is_none()
}

/// Native-clock reference leaves in data values, including nested wrappers.
pub(crate) fn references_in<'a>(
    values: impl IntoIterator<Item = &'a Value>,
) -> Vec<&'a ObjectReference> {
    let mut pending: Vec<_> = values.into_iter().collect();
    let mut references = Vec::new();
    while let Some(value) = pending.pop() {
        match value {
            Value::Reference(reference) => {
                if is_clock_type(reference.object_type()) {
                    references.push(reference);
                }
            }
            Value::Option(option) => pending.extend(option.payload()),
            Value::Collection(collection) => pending.extend(collection.elements()),
            Value::Composite(composite) => {
                pending.extend(composite.slots().iter().filter_map(|slot| match slot {
                    filament_exact::FieldValue::Present(value) => Some(value),
                    _ => None,
                }))
            }
            Value::Union(union) => pending.extend(union.payload()),
            Value::Map(map) => {
                for (key, value) in map.entries() {
                    pending.extend([key, value]);
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
            | Value::Uuid(_)
            | Value::Timestamp(_)
            | Value::Enum(_)
            | Value::Population(_) => {}
        }
    }
    references
}

/// Materialize a normalized v2 identity in the actual owner's existing K2
/// universe. Declaration and population authentication belong to the caller;
/// this shared encoder neither guesses an owner population nor creates one.
///
/// # Errors
/// The canonical codec's exact refusal, including its caller-supplied byte
/// bound, or a malformed identity whose local owner key does not match.
pub fn encode_identity(
    owner: &ObjectReference,
    identity: &identity::PlaceIdentity,
    limits: quire_canonical::Limits,
) -> Result<ObjectReference, identity::IdentityError> {
    if identity.owner != owner.object().as_str() {
        return Err(identity::IdentityError::Malformed);
    }
    let encoded = identity.encode(limits)?;
    let object = ObjectId::new(encoded).map_err(|_| identity::IdentityError::Malformed)?;
    Ok(ObjectReference::new(
        owner.universe().clone(),
        crate::check::clock_reference_target(),
        object,
    ))
}

/// Encode the checked place and its instance without reading any clock value.
pub fn encode(
    owner: &ObjectReference,
    place: &NodeKey,
    selectors: &[Value],
) -> Result<ObjectReference, InvalidClockReference> {
    let selectors = selectors
        .iter()
        .map(|selector| match selector {
            Value::Integer(index) => index.to_u64().ok_or(InvalidClockReference),
            _ => Err(InvalidClockReference),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let identity = ClockPlaceId {
        owner: owner.object().as_str().to_owned(),
        template: place.clone(),
        selectors,
    }
    .identity()?;
    Ok(ObjectReference::new(
        owner.universe().clone(),
        crate::check::clock_reference_target(),
        ObjectId::new(identity).map_err(|_| InvalidClockReference)?,
    ))
}

impl ClockPlaceId {
    /// Read a canonical place id. Model membership is checked separately.
    pub fn decode(identity: &str) -> Result<Self, InvalidClockReference> {
        let mut parts = identity
            .strip_prefix("place/v1/")
            .ok_or(InvalidClockReference)?
            .split('/');
        let owner = unhex(parts.next().ok_or(InvalidClockReference)?)?;
        if owner.is_empty() {
            return Err(InvalidClockReference);
        }
        let template_text = unhex(parts.next().ok_or(InvalidClockReference)?)?;
        let template = fsl_foundation::node_wire::node_key_from_text(&template_text)
            .map_err(|_| InvalidClockReference)?;
        let slots = parts.next().ok_or(InvalidClockReference)?;
        if parts.next().is_some() {
            return Err(InvalidClockReference);
        }
        let selectors = if slots == "-" {
            Vec::new()
        } else {
            slots
                .split('.')
                .map(|slot| slot.parse::<u64>().map_err(|_| InvalidClockReference))
                .collect::<Result<Vec<_>, _>>()?
        };
        let decoded = Self {
            owner,
            template,
            selectors,
        };
        if decoded.identity()? != identity {
            return Err(InvalidClockReference);
        }
        Ok(decoded)
    }

    fn identity(&self) -> Result<String, InvalidClockReference> {
        let place = fsl_foundation::node_wire::node_key_to_text(&self.template)
            .map_err(|_| InvalidClockReference)?;
        let slots = if self.selectors.is_empty() {
            "-".to_owned()
        } else {
            self.selectors
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(".")
        };
        Ok(format!(
            "place/v1/{}/{}/{slots}",
            hex(self.owner.as_bytes()),
            hex(place.as_bytes())
        ))
    }
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn unhex(text: &str) -> Result<String, InvalidClockReference> {
    let mut bytes = Vec::with_capacity(text.len() / 2);
    let mut digits = text.bytes();
    while let Some(high) = digits.next() {
        let low = digits.next().ok_or(InvalidClockReference)?;
        let digit = |byte| match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            _ => Err(InvalidClockReference),
        };
        bytes.push((digit(high)? << 4) | digit(low)?);
    }
    String::from_utf8(bytes).map_err(|_| InvalidClockReference)
}
