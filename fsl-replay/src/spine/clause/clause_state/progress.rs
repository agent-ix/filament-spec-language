// SPDX-License-Identifier: AGPL-3.0-or-later
//! Metadata-only progress admission after a clause has settled.

use serde::{Deserialize, Serialize};

use super::{canonical_order, AdmittedPosition, ClauseRefusal, ClauseState, ClauseTraceKind, Seen};
use crate::spine::clause::progress::{
    LedgerBinding, ObservedPosition, ProgressAssertion, ProgressLedger,
};
use crate::spine::clause::temporal::{
    admit_progress, document_identity, indexed_sample, sample_fingerprint_with, sample_index,
    TimestampedOrderKey, SAMPLE_AUTHORITY,
};
use crate::spine::ClauseDisposition;
use fsl_semantics::model::observation::DocumentRef;

/// Position metadata retained solely for real progress assertions after
/// settlement. No atom, aggregate or clause evaluator runs through this type.
#[derive(Clone)]
pub struct ClauseProgress {
    timestamped: bool,
    first: u64,
    fixed: usize,
    positions: Vec<Seen>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    format: String,
    timestamped: bool,
    first: u64,
    fixed: usize,
    positions: Vec<PositionWire>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PositionWire {
    authority: String,
    identity: String,
    stamp: u64,
    order: Option<OrderWire>,
    absent: Vec<u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderWire {
    authority: String,
    key: i64,
}

impl ClauseProgress {
    pub(super) fn new(state: &ClauseState<'_>) -> Self {
        Self {
            timestamped: state.open.kind == ClauseTraceKind::Timestamped,
            first: 0,
            fixed: state.evaluated.map_or(0, |(seen, _)| seen),
            positions: state.seen.clone(),
        }
    }

    /// The identities retained pending the next real assertion.
    pub fn retained(&self) -> u64 {
        u64::try_from(self.positions.len()).unwrap_or(u64::MAX)
    }

    /// Append an already admitted observation, without reading its value.
    pub fn observe(&mut self, position: &AdmittedPosition<'_>) {
        let mut absent = position.absent.clone();
        absent.sort_unstable();
        absent.dedup();
        self.positions.push(Seen {
            reference: position.reference.clone(),
            stamp: position.stamp,
            order_key: position.order_key.clone(),
            absent,
        });
    }

    /// Validate a real assertion using FR-502's existing admission, retaining
    /// canonical metadata only for positions still beyond its watermark.
    ///
    /// # Errors
    /// The native progress contradiction, capacity limit, or fingerprint fault.
    /// Neither this tracker nor the input ledger changes on refusal.
    pub fn admit(
        &mut self,
        ledger: &ProgressLedger,
        binding: &LedgerBinding,
        assertion: ProgressAssertion,
        capacity: u64,
    ) -> Result<ProgressLedger, Box<ClauseDisposition>> {
        let mut positions = self.positions.clone();
        if self.timestamped {
            positions[self.fixed..].sort_by(canonical_order);
        }
        let mut observed = Vec::with_capacity(positions.len());
        for (offset, position) in positions.iter().enumerate() {
            let reference = if position.reference.authority == SAMPLE_AUTHORITY
                && indexed_sample(&position.reference).is_none()
            {
                let fingerprint =
                    sample_fingerprint_with(&position.reference.identity, &position.absent)
                        .map_err(|fault| Box::new(ClauseDisposition::EvaluateFault(fault)))?;
                let index = self
                    .first
                    .saturating_add(u64::try_from(offset).unwrap_or(u64::MAX));
                sample_index(index, Some(&fingerprint))
            } else {
                position.reference.clone()
            };
            observed.push(ObservedPosition {
                identity: document_identity(&reference),
                stamp: position.stamp,
                fingerprint: indexed_sample(&reference)
                    .and_then(|(_, fingerprint)| fingerprint.map(str::to_owned)),
            });
        }
        let updated = admit_progress(ledger, binding, assertion, &observed, capacity)?;
        let covered = positions
            .iter()
            .take_while(|p| p.stamp <= assertion.watermark)
            .count();
        self.first = self
            .first
            .saturating_add(u64::try_from(covered).unwrap_or(u64::MAX));
        self.fixed = 0;
        self.positions = positions.into_iter().skip(covered).collect();
        Ok(updated)
    }

    /// Serialize the metadata independently of the frozen deciding state.
    ///
    /// # Errors
    /// An internal fault if the typed metadata cannot be serialized.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ClauseRefusal> {
        let wire = Wire {
            format: "filament.clause-progress/1".to_owned(),
            timestamped: self.timestamped,
            first: self.first,
            fixed: self.fixed,
            positions: self
                .positions
                .iter()
                .map(|p| PositionWire {
                    authority: p.reference.authority.clone(),
                    identity: p.reference.identity.clone(),
                    stamp: p.stamp,
                    order: p.order_key.as_ref().map(|o| OrderWire {
                        authority: o.authority.clone(),
                        key: o.key,
                    }),
                    absent: p.absent.clone(),
                })
                .collect(),
        };
        serde_json::to_vec(&wire).map_err(|_| super::fault("progress-state-serializes"))
    }

    /// Read the closed metadata checkpoint, subject to its identity capacity.
    ///
    /// # Errors
    /// `StateMismatch` for malformed, oversized or inconsistent metadata.
    pub fn from_bytes(
        bytes: &[u8],
        kind: ClauseTraceKind,
        seen: u64,
        capacity: u64,
    ) -> Result<Self, ClauseRefusal> {
        let wire: Wire = serde_json::from_slice(bytes).map_err(|_| ClauseRefusal::StateMismatch)?;
        if wire.format != "filament.clause-progress/1"
            || wire.timestamped != (kind == ClauseTraceKind::Timestamped)
            || wire
                .first
                .checked_add(u64::try_from(wire.positions.len()).unwrap_or(u64::MAX))
                != Some(seen)
            || wire.fixed > wire.positions.len()
            || u64::try_from(wire.positions.len()).unwrap_or(u64::MAX) > capacity
            || wire
                .positions
                .iter()
                .any(|p| p.absent.windows(2).any(|pair| pair[0] >= pair[1]))
        {
            return Err(ClauseRefusal::StateMismatch);
        }
        Ok(Self {
            timestamped: wire.timestamped,
            first: wire.first,
            fixed: wire.fixed,
            positions: wire
                .positions
                .into_iter()
                .map(|p| Seen {
                    reference: DocumentRef {
                        authority: p.authority,
                        identity: p.identity,
                    },
                    stamp: p.stamp,
                    order_key: p.order.map(|o| TimestampedOrderKey {
                        authority: o.authority,
                        key: o.key,
                    }),
                    absent: p.absent,
                })
                .collect(),
        })
    }
}
