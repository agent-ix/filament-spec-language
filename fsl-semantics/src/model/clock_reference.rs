// SPDX-License-Identifier: AGPL-3.0-or-later
//! The place-id codec shared by clock-reference evaluation and admission.

use filament_exact::{ObjectId, ObjectReference, Value};

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

/// The FSL runtime type of an admitted Clock-reference DATA scalar.
/// This is the same canonical native target used by source checking.
pub fn reference_type() -> filament_exact::ValueType {
    crate::check::clock_reference_type()
}

/// Whether S names an actual object declaration the real owner conforms to.
/// This uses type provenance independently of the remaining Clock paths, so
/// removing the last Clock does not turn a same-scope stale path into a
/// disagreement with the authenticated model scope.
pub fn is_declared_owner_scope(
    scope: &identity::DeclarationScope,
    owner: &ObjectReference,
    declarations: &normalization::ClockDeclarationTable,
    types: &filament_semantic_value::declaration::TypeEnvironment,
) -> bool {
    types.object_types().any(|declaration| {
        let target = declaration.key();
        let original = target.original();
        let key = filament_checked::key::DeclarationKey {
            package: original.package().to_owned(),
            node: original.node().to_owned(),
        };
        declarations.source_declarations().catalog_scope(&key) == Some(scope)
            && types.conforms(owner.object_type().clone(), target)
    })
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
