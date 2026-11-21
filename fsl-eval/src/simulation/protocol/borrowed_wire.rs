// SPDX-License-Identifier: AGPL-3.0-or-later
//! Borrowed protocol output events. Resource admission belongs to the sink.

use crate::simulation::model::encode::Enc;
use filament_checked::key::DeclarationKey;
use filament_exact::{EffectiveId, NodeKey, ObjectReference};
use serde_json::Value as Json;

/// Output operations used by protocol metadata without building an `Enc` tree.
/// This is a serializer, not an admission/identity authority. Reference leaves
/// must be converted by the sink's actual package-bound reference codec.
pub trait ProtocolWireSink: Sized {
    /// Original typed conversion or resource failure.
    type Error;
    /// Charge a borrowed traversal before inspecting it.
    fn work(&mut self, units: usize) -> Result<(), Self::Error>;
    /// Admit one native value visit and its actual configured nesting policy.
    fn depth(&mut self, depth: usize) -> Result<(), Self::Error>;
    /// Copy text only after admission.
    fn string(&mut self, text: &str) -> Result<Json, Self::Error>;
    /// Format a protocol ordinal as canonical decimal text after admission.
    fn unsigned(&mut self, value: u64) -> Result<Json, Self::Error>;
    /// Begin an object without allocating members.
    fn object(&mut self) -> Result<Json, Self::Error>;
    /// Admit array storage before iteration/materialization.
    fn array(&mut self, len: usize) -> Result<Json, Self::Error>;
    /// Construct a member under its escaped pointer and admit insertion first.
    fn field(
        &mut self,
        out: &mut Json,
        name: &str,
        value: impl FnOnce(&mut Self) -> Result<Json, Self::Error>,
    ) -> Result<(), Self::Error>;
    /// Construct one admitted array slot under its index pointer.
    fn element(
        &mut self,
        out: &mut Json,
        index: usize,
        value: impl FnOnce(&mut Self) -> Result<Json, Self::Error>,
    ) -> Result<(), Self::Error>;
    /// The complete native reference, retained by the protocol runtime.
    fn reference(&mut self, value: &ObjectReference) -> Result<Json, Self::Error>;
    /// A stored native reference leaf; these parts confer no authority.
    fn reference_parts(
        &mut self,
        universe: &NodeKey,
        object_type: &EffectiveId,
        identity: &str,
    ) -> Result<Json, Self::Error>;
    /// Borrowed native structural identity.
    fn node(&mut self, value: &NodeKey) -> Result<Json, Self::Error>;
    /// Borrowed native non-reference effective identity metadata.
    fn effective(&mut self, value: &EffectiveId) -> Result<Json, Self::Error>;
    /// Canonical declaration text under the existing operation adapter.
    fn declaration(&mut self, value: &DeclarationKey) -> Result<Json, Self::Error>;
    /// Canonically order typed-map entries after reference conversion.
    fn order_map(&mut self, entries: &mut Json) -> Result<(), Self::Error>;
    /// Preserve an unencodable native variant as a typed failure.
    fn unsupported(&mut self, what: &'static str) -> Self::Error;
}

pub(super) fn tagged<S: ProtocolWireSink>(sink: &mut S, tag: &str) -> Result<Json, S::Error> {
    let mut out = sink.object()?;
    sink.field(&mut out, "type", |s| s.string(tag))?;
    Ok(out)
}

pub(super) fn value<S: ProtocolWireSink>(
    sink: &mut S,
    value: &Enc,
    depth: usize,
) -> Result<Json, S::Error> {
    sink.depth(depth)?;
    let next = depth
        .checked_add(1)
        .ok_or_else(|| sink.unsupported("protocol value depth"))?;
    match value {
        Enc::Null => Ok(Json::Null),
        Enc::Bool(value) => Ok(Json::Bool(*value)),
        Enc::Str(value) => sink.string(value),
        Enc::Node(value) => sink.node(value),
        Enc::Effective(value) => sink.effective(value),
        Enc::Declaration(value) => sink.declaration(value),
        Enc::Shared(value) => self::value(sink, value.borrowed_value(), depth),
        Enc::Unencodable(what) => Err(sink.unsupported(what)),
        Enc::Array(values) => {
            let mut out = sink.array(values.len())?;
            for (index, value) in values.iter().enumerate() {
                sink.element(&mut out, index, |s| self::value(s, value, depth))?;
            }
            Ok(out)
        }
        Enc::Object(fields) => {
            // Inspect the genuine stored native reference shape before copying
            // identity data. There is no backend/native union fallback.
            for (name, value) in fields {
                sink.work(name.len())?;
                if *name == "type" {
                    if let Enc::Str(tag) = value {
                        sink.work(tag.len())?;
                    }
                }
            }
            let reference = fields.iter().any(|(name, value)| {
                *name == "type" && matches!(value, Enc::Str(tag) if tag == "reference")
            });
            if reference {
                let comparisons = fields
                    .len()
                    .checked_mul(4)
                    .ok_or_else(|| sink.unsupported("protocol field count"))?;
                sink.work(comparisons)?;
                let get = |name| {
                    fields
                        .iter()
                        .find_map(|(key, value)| (*key == name).then_some(value))
                };
                if let (
                    4,
                    Some(Enc::Node(universe)),
                    Some(Enc::Effective(object_type)),
                    Some(Enc::Str(identity)),
                ) = (
                    fields.len(),
                    get("universe"),
                    get("object_type"),
                    get("identity"),
                ) {
                    return sink.reference_parts(universe, object_type, identity);
                }
                return Err(sink.unsupported("native protocol reference shape"));
            }
            let nested = fields.iter().any(|(name, value)| *name == "type" && matches!(value, Enc::Str(tag) if matches!(tag.as_str(), "option" | "collection" | "set" | "sequence" | "bag" | "record" | "tuple" | "union" | "map")));
            let child_depth = if nested { next } else { depth };
            let mut out = sink.object()?;
            for (name, value) in fields {
                sink.field(&mut out, name, |s| self::value(s, value, child_depth))?;
            }
            Ok(out)
        }
        Enc::Map(entries) => {
            let mut written = sink.array(entries.len())?;
            for (index, (key, value)) in entries.iter().enumerate() {
                sink.element(&mut written, index, |s| {
                    let mut row = s.object()?;
                    s.field(&mut row, "key", |s| self::value(s, key, next))?;
                    s.field(&mut row, "value", |s| self::value(s, value, next))?;
                    Ok(row)
                })?;
            }
            sink.order_map(&mut written)?;
            let mut out = tagged(sink, "map")?;
            sink.field(&mut out, "entries", |_| Ok(written))?;
            Ok(out)
        }
    }
}
