// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test support: encodes a plain `ValueType` shape as the
//! `forms::TypeForm` syntax `check`'s type-form resolution maps back to
//! that exact `ValueType` -- the exact inverse of that resolution. Lets a
//! fixture keep describing its intended type by the `ValueType` shape it
//! wants, while still handing the checker only syntax (ADR-011 §1: S2 has
//! no semantic identity).
//!
//! `ValueType::Composite`/`Enum`/`Reference` carry only a `NodeKey` digest,
//! with no name to recover -- a fixture that wants one of those builds the
//! matching `TypeForm::name(name, SPAN)` directly, with the same name
//! string it used to derive that `NodeKey` (see each declared type's own
//! registration), rather than through [`type_form`] here.

use filament_exact::IeeeWidth;
use filament_exact::{CollectionType, ValueType};
use fsl_forms::TypeForm;

/// A placeholder span: `TypeForm`'s own span carries no identity
/// (ADR-011 §2.2 row E2, "identity: none: forms carry position only"), so
/// every fixture built through this module uses the same one.
pub const SPAN: fsl_foundation::Span = fsl_foundation::Span { start: 0, end: 0 };

fn rounding_mode_text(mode: filament_exact::RoundingMode) -> &'static str {
    match mode {
        filament_exact::RoundingMode::Exact => "exact",
        filament_exact::RoundingMode::TowardZero => "toward-zero",
        filament_exact::RoundingMode::TowardPositive => "toward-positive",
        filament_exact::RoundingMode::TowardNegative => "toward-negative",
        filament_exact::RoundingMode::NearestEven => "nearest-even",
        filament_exact::RoundingMode::NearestAway => "nearest-away",
    }
}

fn text_profile_text(profile: filament_exact::TextProfile) -> &'static str {
    use filament_exact::TextProfile;
    match profile {
        TextProfile::UnicodeScalars => "unicode-scalars",
        TextProfile::Nfc => "nfc",
        TextProfile::Nfd => "nfd",
        TextProfile::Nfkc => "nfkc",
        TextProfile::Nfkd => "nfkd",
        TextProfile::BinaryUtf8 => "binary-utf8",
    }
}

fn collection_type_form(collection: &CollectionType) -> TypeForm {
    let bounds = collection
        .bound()
        .map(|bound| vec![bound.minimum().to_string(), bound.maximum().to_string()])
        .unwrap_or_default();
    TypeForm::collection(collection.kind(), SPAN)
        .with_arguments(vec![type_form(collection.element())])
        .with_bounds(bounds)
}

/// The exact inverse of `check::Typer::resolve_type` for every `ValueType`
/// shape that carries no name to recover (everything but
/// `Composite`/`Enum`/`Reference` -- see this module's own doc; those
/// panic).
pub fn type_form(value_type: &ValueType) -> TypeForm {
    match value_type {
        ValueType::Boolean => TypeForm::builtin(fsl_forms::BuiltinType::Boolean, SPAN),
        ValueType::Integer => TypeForm::builtin(fsl_forms::BuiltinType::Integer, SPAN),
        ValueType::Int(interval) => TypeForm::builtin(fsl_forms::BuiltinType::Int, SPAN)
            .with_bounds(vec![
                interval.lower().to_string(),
                interval.upper().to_string(),
            ]),
        ValueType::Rational(domain) => TypeForm::builtin(fsl_forms::BuiltinType::Rational, SPAN)
            .with_bounds(vec![
                domain.numerator().lower().to_string(),
                domain.numerator().upper().to_string(),
                domain.denominator().lower().to_string(),
                domain.denominator().upper().to_string(),
            ]),
        ValueType::Decimal(decimal) => TypeForm::builtin(fsl_forms::BuiltinType::Decimal, SPAN)
            .with_bounds(vec![
                decimal.lower().to_string(),
                decimal.upper().to_string(),
                decimal.min_scale().to_string(),
                decimal.max_scale().to_string(),
                rounding_mode_text(decimal.rounding()).to_owned(),
            ]),
        ValueType::Float(float) => TypeForm::builtin(
            match float.width() {
                IeeeWidth::Binary32 => fsl_forms::BuiltinType::Float32,
                IeeeWidth::Binary64 => fsl_forms::BuiltinType::Float64,
            },
            SPAN,
        )
        .with_bounds(vec![rounding_mode_text(float.rounding()).to_owned()]),
        ValueType::Text(text) => {
            TypeForm::builtin(fsl_forms::BuiltinType::Text, SPAN).with_bounds(vec![
                text.min().to_string(),
                text.max().to_string(),
                text_profile_text(text.profile()).to_owned(),
            ])
        }
        ValueType::Option(payload) => TypeForm::builtin(fsl_forms::BuiltinType::Option, SPAN)
            .with_arguments(vec![type_form(payload)]),
        ValueType::Collection(collection) => collection_type_form(collection),
        ValueType::Population(maximum) => TypeForm::new(fsl_forms::TypeFormHead::Population, SPAN)
            .with_bounds(vec![
                maximum
                    .expect("type_form: an unbounded population has no source spelling yet")
                    .to_string(),
            ]),
        ValueType::Composite(_) | ValueType::Enum(_) | ValueType::Reference(_) => {
            panic!(
                "type_form: {value_type:?} carries a NodeKey with no recoverable name -- build \
                 a TypeForm::name(...) directly, with the same name string used to derive the \
                 key, instead of calling this function"
            )
        }
        ValueType::Quantity(_) => panic!(
            "type_form: {value_type:?} has no declared-type syntax -- forms::BuiltinType \
             has no quantity head, so no fixture can build this TypeForm today; a caller reaching this arm wants \
             `value::declaration::EqualityOperand`'s own `ValueType`-based API instead, not a \
             `TypeForm`"
        ),
        ValueType::CarriedQuantity(_) | ValueType::BoundedQuantity(_) | ValueType::Map(_) => {
            panic!("type_form: {value_type:?} has no declared-type syntax in this profile")
        }
        ValueType::String
        | ValueType::Bytes
        | ValueType::JsonObject
        | ValueType::Uuid
        | ValueType::Timestamp => {
            panic!("type_form: {value_type:?} has no FSL source type spelling in this profile")
        }
    }
}

/// [`TypeForm::name`] at [`SPAN`], for a declared record, tuple, enum or
/// model object type's own name (see this module's own doc).
pub fn named_type_form(name: impl Into<String>) -> TypeForm {
    TypeForm::name(name, SPAN)
}
