// SPDX-License-Identifier: AGPL-3.0-or-later
//! Admission of the original typed witness packet, before execution overlays.

use filament_exact::{Origin, Role};
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_semantics::check::wire_id::WireGraph;

use super::{ContextError, EnvelopeDescriptor};
use crate::witness::{EnvelopeSource, FamilyPayload, ReplaySource, WitnessEnvelope, WitnessPacket};
use crate::{ReplayLimits, ReplayRequestWire, TracePosition};

impl EnvelopeDescriptor {
    /// Resolve the original claim occurrence from the immutable admitted graph.
    pub fn occurrence(&self, original: &ReplayRequestWire) -> Result<OccurrenceKey, ContextError> {
        self.identities(original).map(|(occurrence, _)| occurrence)
    }

    fn identities(
        &self,
        original: &ReplayRequestWire,
    ) -> Result<(OccurrenceKey, filament_exact::NodeKey), ContextError> {
        let graph = WireGraph::of_package_bytes(original.package.bytes())
            .map_err(|error| ContextError::invalid("/request/package", error.to_string()))?;
        let occurrence: OccurrenceDescriptor = serde_json::from_value(self.occurrence_key.clone())?;
        let node = graph
            .key_of_value(&occurrence.node_id)
            .map_err(|error| ContextError::invalid("/envelope/occurrence_key/node_id", error))?;
        let ordinal = super::unsigned(&occurrence.ordinal, "/envelope/occurrence_key/ordinal")?;
        let occurrence = OccurrenceKey::new(node, Origin::new(Role::new(occurrence.role), ordinal));
        let clause = graph
            .key_of_value(&self.clause_node)
            .map_err(|error| ContextError::invalid("/envelope/clause_node", error))?;
        if original.obligation_identity.node() != &clause || occurrence.node() != &clause {
            return Err(ContextError::invalid(
                "/envelope/occurrence_key",
                "packet node differs from original obligation",
            ));
        }
        Ok((occurrence, clause))
    }

    /// Restore a native witness envelope from authenticated common members and
    /// the owning family's readback payload. No proving member is overwritten.
    pub fn reconstruct<P: FamilyPayload>(
        &self,
        original: &ReplayRequestWire,
        payload: P,
        limits: ReplayLimits,
    ) -> Result<WitnessEnvelope<P>, ContextError> {
        let (occurrence_key, clause_node) = self.identities(original)?;
        let source: SourceDescriptor = serde_json::from_value(self.source.clone())?;
        if source.kind != "witness" {
            return Err(ContextError::invalid(
                "/envelope/source/kind",
                "temporal packet requires its actual witness source",
            ));
        }
        let ReplaySource::Witness(original_witness) = &original.source else {
            return Err(ContextError::invalid(
                "/envelope/source",
                "request source differs from original witness",
            ));
        };
        if source.transcript != original_witness.transcript() {
            return Err(ContextError::invalid(
                "/envelope/source/transcript",
                "original witness mismatch",
            ));
        }
        let trace_position = match &self.trace_position {
            serde_json::Value::Null => None,
            serde_json::Value::String(position) => Some(TracePosition::new(position.clone())),
            _ => {
                return Err(ContextError::invalid(
                    "/envelope/trace_position",
                    "expected explicit null or trace position text",
                ))
            }
        };
        WitnessEnvelope::reconstruct(
            WitnessPacket {
                obligation_identity: Some(original.obligation_identity.clone()),
                occurrence_key: Some(occurrence_key),
                clause_node: Some(clause_node),
                replay_target: original.replay_target.members.clone(),
                profile_selections: Some(original.profile_selections.clone()),
                run_limits: Some(super::accounting_from_wire(
                    &self.run_limits,
                    "/envelope/run_limits",
                )?),
                declared_domains: Some(original.declared_domains.clone()),
                backend: Some(original.backend.clone()),
                trace_position: Some(trace_position),
                source: Some(EnvelopeSource::Witness(original_witness.clone())),
                family_payload: Some(payload),
            },
            limits,
        )
        .map_err(|error| ContextError::invalid("/envelope", error.to_string()))
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OccurrenceDescriptor {
    node_id: serde_json::Value,
    role: String,
    ordinal: String,
}

/// The existing witness source mapping retained by a temporal packet.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceDescriptor {
    /// The existing source kind, required to equal `witness`.
    pub kind: String,
    /// Complete original witness transcript.
    pub transcript: String,
}
