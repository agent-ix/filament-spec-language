// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-578: the FSpec FR-351 record of a decisive protocol-level refinement
//! counterexample, for a refinement decided outside FSL and carried under
//! filament-protocol FR-302.
//!
//! The record is the protocol family's record that ADR-036 PA-24 writes on
//! `native-run-result/2`, with the same writer and the same strict reader,
//! so a carried witness has one shape whichever protocol claim decided it.

use filament_contract_model::CheckedNodeId;

use crate::protocol_analysis::{AssessedRoot, MissingMember};
use crate::result::{CauseCodecError, ProtocolWitnessWire, SeparatingWitnessRecord};

/// Why a refinement witness was not built, written or read.
#[derive(Debug, thiserror::Error)]
pub enum RefinementWitnessError {
    /// A record component is absent.
    #[error(transparent)]
    Missing(#[from] MissingMember),
    /// The record is not of the protocol family's shape.
    #[error("invalid separating witness: {0}")]
    Shape(#[from] CauseCodecError),
    /// The bytes are not one JSON object of the record's closed shape.
    #[error("malformed refinement witness: {0}")]
    Json(#[from] serde_json::Error),
    /// The bytes are not RFC 8785 canonical JSON.
    #[error("the refinement witness bytes are not canonical")]
    NotCanonical,
    /// The canonical encoder could not write the record.
    #[error("cannot encode the refinement witness canonically: {0}")]
    Canonical(#[from] quire_canonical::Error),
}

/// The FR-351 record of a decisive protocol refinement counterexample: the
/// deciding protocol element's checked node, the assessed root's value path
/// with no steps, and the trace position. It has no quantifier and no index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementWitness {
    wire: ProtocolWitnessWire,
}

impl RefinementWitness {
    /// The record whose deciding element is `deciding`, at the zero-based
    /// position `trace_position` of the counterexample's ordered step
    /// sequence, under the claim's assessed root `root`.
    ///
    /// # Errors
    ///
    /// [`RefinementWitnessError`] when the node does not encode or the
    /// record is not of the protocol family's shape.
    pub fn new(
        root: &AssessedRoot,
        deciding: &CheckedNodeId,
        trace_position: usize,
    ) -> Result<Self, RefinementWitnessError> {
        let record = root.record(deciding, trace_position)?;
        Ok(Self {
            wire: ProtocolWitnessWire::of(&record)?,
        })
    }

    /// The deciding element's checked node.
    #[must_use]
    pub fn deciding_node(&self) -> Option<CheckedNodeId> {
        self.wire.deciding_node()
    }

    /// The trace position.
    #[must_use]
    pub fn trace_position(&self) -> Option<u64> {
        self.wire.trace_position()
    }

    /// Whether this is `record` on the wire: the same deciding element,
    /// value path and trace position.
    #[must_use]
    pub fn is(&self, record: &SeparatingWitnessRecord) -> bool {
        ProtocolWitnessWire::of(record).is_ok_and(|wire| wire == self.wire)
    }

    /// The record as RFC 8785 bytes: the `witness` member PA-24 writes on a
    /// protocol-analysis record.
    ///
    /// # Errors
    ///
    /// [`RefinementWitnessError`] when the record does not encode, which no
    /// built or read record does.
    pub fn to_bytes(&self) -> Result<Vec<u8>, RefinementWitnessError> {
        let json = serde_json::to_vec(&self.wire)?;
        let parsed = quire_canonical::read(&json, u64::MAX)
            .map_err(|_| RefinementWitnessError::NotCanonical)?;
        Ok(quire_canonical::to_vec(
            &parsed.root(),
            quire_canonical::Limits::new(u64::MAX),
        )?)
    }

    /// Read a record strictly (FSpec FR-351-AC-5): its closed shape, a
    /// deciding element that is a node id's canonical text, the root value
    /// path with no steps, a canonical decimal trace position, and bytes
    /// that are already canonical, so a normalized, reordered or
    /// re-spelled record is refused rather than repaired.
    ///
    /// # Errors
    ///
    /// [`RefinementWitnessError`] for a record outside that shape.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RefinementWitnessError> {
        let wire: ProtocolWitnessWire = serde_json::from_slice(bytes)?;
        wire.validate()?;
        let read = Self { wire };
        if read.to_bytes()? != bytes {
            return Err(RefinementWitnessError::NotCanonical);
        }
        Ok(read)
    }
}
