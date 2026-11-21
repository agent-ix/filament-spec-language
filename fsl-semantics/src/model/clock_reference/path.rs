// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230's one normalization of checked clock paths for both routes.
//! Resolution rows have already been checked; original declaration scopes
//! come from their actual source or admitted-package declaration context.

use filament_semantic_value::catalog::InternalFault;

use crate::check::{CollectionShape as CheckedCollectionShape, PlaceRow, PlaceStep};

use super::identity::{
    Carrier, CarrierLayer, CarrierLeaf, CollectionKind, CollectionShape, Decimal, DeclarationScope,
    PathEntry, PlaceTemplate,
};

/// Normalize a checked path, preserving declared types, presence and slot
/// shapes. `scope_of` resolves only complete checked type spellings against
/// the caller's authenticated declaration context; it never resolves names.
///
/// # Errors
/// A fault if checked rows no longer correspond to their typed path, a
/// required declaration scope is absent, or a map reaches positional encoding.
pub fn normalize(
    scope: DeclarationScope,
    steps: &[PlaceStep],
    rows: &[PlaceRow],
    normalize_text: impl Fn(&str) -> String,
    normalize_member: impl Fn(&str, &str) -> Result<String, InternalFault>,
    scope_of: impl Fn(&str) -> Result<Option<DeclarationScope>, InternalFault>,
) -> Result<PlaceTemplate, InternalFault> {
    if steps.is_empty() || steps.len() != rows.len() {
        return Err(fault("clock-path-and-resolution-rows-correspond"));
    }
    let mut path = Vec::new();
    for (position, (step, row)) in steps.iter().zip(rows).enumerate() {
        let carrier = carrier(
            &normalize_text(&row.member_type),
            &steps[position + 1..],
            &scope_of,
        )?;
        if position + 1 == steps.len() && carrier.leaf != CarrierLeaf::Clock {
            return Err(fault("clock-path-ends-in-validated-native-clock"));
        }
        let selecting = match step {
            PlaceStep::Field(name) => {
                if let Ok(position) = Decimal::new(name) {
                    PathEntry::Tuple {
                        position,
                        carrier: carrier.clone(),
                    }
                } else {
                    let rendered = normalize_text(&row.declaring_type);
                    let declaring = scope_of(without_optional(&rendered))?
                        .ok_or_else(|| fault("clock-field-has-original-declaring-scope"))?;
                    let member =
                        normalize_member(without_optional(&rendered), &row.member_identity)?;
                    if !member.starts_with("ix://") {
                        return Err(fault("clock-field-has-original-member-uri"));
                    }
                    PathEntry::Field {
                        declaring,
                        member,
                        carrier: carrier.clone(),
                    }
                }
            }
            PlaceStep::Union(name) => PathEntry::Case {
                name: name.clone(),
                carrier: carrier.clone(),
            },
            PlaceStep::Slot(shape) => PathEntry::Collection {
                shape: collection(*shape)?,
                carrier: carrier.clone(),
            },
        };
        path.push(selecting);
        // A collection has its own selecting step. Only the leading
        // optional layers belong between this step and the next traversal.
        for layer in &carrier.layers {
            match layer {
                CarrierLayer::Option => path.push(PathEntry::Option),
                CarrierLayer::Collection(_) => break,
            }
        }
    }
    Ok(PlaceTemplate { scope, path })
}

/// A member below an optional record is still declared by that record;
/// the separate selecting carrier and option entry retain its presence.
#[ix_trace_rs::string_edge]
fn without_optional(mut rendered: &str) -> &str {
    while let Some(payload) = rendered
        .strip_prefix("Option<")
        .and_then(|value| value.strip_suffix('>'))
    {
        rendered = payload;
    }
    rendered
}

/// Read the wrapper spelling which FR-322 resolution rows retain. The
/// positional collection bounds come from the typed slot steps, never from
/// an inferred default or a parsed integer type rendering.
#[ix_trace_rs::string_edge]
fn carrier(
    mut text: &str,
    following: &[PlaceStep],
    scope_of: &impl Fn(&str) -> Result<Option<DeclarationScope>, InternalFault>,
) -> Result<Carrier, InternalFault> {
    let mut layers = Vec::new();
    let mut traversed = 0;
    loop {
        if let Some(payload) = text
            .strip_prefix("Option<")
            .and_then(|value| value.strip_suffix('>'))
        {
            layers.push(CarrierLayer::Option);
            text = payload;
            continue;
        }
        let wrapper = [
            ("Sequence<", CollectionKind::Sequence),
            ("Set<", CollectionKind::Set),
            ("Bag<", CollectionKind::Bag),
            ("OrderedSet<", CollectionKind::OrderedSet),
        ]
        .into_iter()
        .find_map(|(prefix, kind)| Some((text.strip_prefix(prefix)?.strip_suffix('>')?, kind)));
        let Some((payload, kind)) = wrapper else {
            break;
        };
        let Some(PlaceStep::Slot(shape)) = following.get(traversed) else {
            return Err(fault("clock-collection-carrier-has-typed-slot"));
        };
        let shape = collection(*shape)?;
        if shape.kind != kind {
            return Err(fault("clock-collection-carrier-agrees-with-typed-slot"));
        }
        layers.push(CarrierLayer::Collection(shape));
        traversed += 1;
        text = payload;
    }
    let leaf = if text == "ix://filament/native/Clock" {
        if following.get(traversed).is_some() {
            return Err(fault("clock-leaf-ends-its-typed-path"));
        }
        CarrierLeaf::Clock
    } else if let Some(scope) = scope_of(text)? {
        CarrierLeaf::Declared(scope)
    } else {
        match following.get(traversed) {
            Some(PlaceStep::Field(name)) if Decimal::new(name).is_ok() => CarrierLeaf::Tuple,
            Some(PlaceStep::Field(_)) => CarrierLeaf::Record,
            Some(PlaceStep::Union(_)) => CarrierLeaf::Union,
            Some(PlaceStep::Slot(_)) | None => {
                return Err(fault(
                    "clock-carrier-has-authenticated-declared-or-structural-type",
                ))
            }
        }
    };
    Ok(Carrier { layers, leaf })
}

fn collection(shape: CheckedCollectionShape) -> Result<CollectionShape, InternalFault> {
    if shape.map {
        return Err(fault("clock-map-has-no-positional-identity"));
    }
    Ok(CollectionShape {
        kind: match (shape.ordered, shape.unique) {
            (true, false) => CollectionKind::Sequence,
            (false, true) => CollectionKind::Set,
            (false, false) => CollectionKind::Bag,
            (true, true) => CollectionKind::OrderedSet,
        },
        minimum: shape.min.into(),
        maximum: shape.max.map(Into::into),
    })
}

fn fault(invariant: &'static str) -> InternalFault {
    InternalFault::new("clock-path-normalization", invariant)
}
