// SPDX-License-Identifier: AGPL-3.0-or-later
//! Borrowed projection of an original opaque source clock-place identity.

use super::*;
use crate::simulation::protocol::ProtocolWireSink;
use serde_json::Value;

impl TimedSystem<'_, '_> {
    /// Project the original clock place without constructing an `Enc` tree.
    /// The reference callback writes its opaque native identity; this is not
    /// an external TypedValue reference or a new clock-place admission.
    ///
    /// # Errors
    /// Preserves the sink's missing-place, resource and cancellation failures.
    pub fn clock_place_wire<S: ProtocolWireSink>(
        &self,
        index: usize,
        s: &mut S,
        mut native_reference: impl FnMut(&ObjectReference, &mut S) -> Result<Value, S::Error>,
    ) -> Result<Value, S::Error> {
        s.work(1)?;
        let place = self
            .places
            .get(index)
            .ok_or_else(|| s.unsupported("source clock place"))?;
        let field = self
            .model
            .and_then(|model| model.clock(place.clock))
            .ok_or_else(|| s.unsupported("source clock field"))?;
        let mut out = s.object()?;
        s.field(&mut out, "type", |s| s.string("clock_place"))?;
        s.field(&mut out, "object", |s| native_reference(&place.object, s))?;
        s.field(&mut out, "field", |s| {
            let mut out = s.object()?;
            s.field(&mut out, "node", |s| s.string(&field.field().node))?;
            s.field(&mut out, "package", |s| s.string(&field.field().package))?;
            Ok(out)
        })?;
        s.field(&mut out, "path", |s| {
            let mut out = s.array(field.path().len())?;
            let mut slots = place.slots.iter();
            for (index, step) in field.path().iter().enumerate() {
                s.element(&mut out, index, |s| {
                    let mut out = s.object()?;
                    match step {
                        PlaceStep::Field(name) => {
                            s.field(&mut out, "type", |s| s.string("field"))?;
                            s.field(&mut out, "name", |s| s.string(name))?;
                        }
                        PlaceStep::Union(case) => {
                            s.field(&mut out, "type", |s| s.string("union"))?;
                            s.field(&mut out, "case", |s| s.string(case))?;
                        }
                        PlaceStep::Slot(_) => {
                            s.field(&mut out, "type", |s| s.string("slot"))?;
                            s.field(&mut out, "index", |s| match slots.next() {
                                Some(index) => {
                                    let index = u64::try_from(*index)
                                        .map_err(|_| s.unsupported("source slot ordinal"))?;
                                    s.unsigned(index)
                                }
                                None => s.string(""),
                            })?;
                        }
                    }
                    Ok(out)
                })?;
            }
            Ok(out)
        })?;
        Ok(out)
    }
}
