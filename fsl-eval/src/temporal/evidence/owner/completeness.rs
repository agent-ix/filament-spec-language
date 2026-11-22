// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-944: completeness covers the admitted population, independently of its boundary.

use super::*;
use filament_observation::authority::{completeness, TemporalBoundary};

impl OwnerObservation {
    pub(super) fn completeness_facts(&self, complete: bool) -> Vec<completeness::Fact> {
        self.request
            .scope
            .members
            .iter()
            .map(|member| {
                completeness::Fact::new(
                    member.object_identity.clone(),
                    complete.then(|| member.record_identity.clone()),
                    if complete {
                        completeness::FactStatus::Available
                    } else {
                        completeness::FactStatus::Incomplete
                    },
                )
            })
            .collect()
    }

    pub(super) fn closure_boundary(&self) -> Result<TemporalBoundary, CallFailure> {
        match self.request.scope.range {
            ClockRange::EventPosition {
                start,
                end_exclusive,
            } => Ok(TemporalBoundary::EventPosition {
                lower: start,
                upper_inclusive: end_exclusive
                    .checked_sub(1)
                    .ok_or_else(|| fault("owner-range-is-nonempty"))?,
                carrier_end_exclusive: end_exclusive,
                watermark: end_exclusive,
            }),
            ClockRange::FixedSample {
                start,
                end_exclusive,
                ..
            } => Ok(TemporalBoundary::FixedSample {
                lower: start,
                upper_inclusive: end_exclusive
                    .checked_sub(1)
                    .ok_or_else(|| fault("owner-range-is-nonempty"))?,
                carrier_end_exclusive: end_exclusive,
                watermark: end_exclusive,
            }),
            ClockRange::Timestamp {
                start_nanos,
                end_nanos,
            } => Ok(TemporalBoundary::TimestampedEvent {
                lower_nanos: start_nanos,
                upper_inclusive_nanos: end_nanos
                    .checked_sub(1)
                    .ok_or_else(|| fault("owner-range-is-nonempty"))?,
                carrier_end_exclusive_nanos: end_nanos,
                watermark_nanos: end_nanos,
            }),
        }
    }
}

pub(super) fn boundary_value(boundary: TemporalBoundary) -> Value {
    match boundary {
        TemporalBoundary::EventPosition {
            lower,
            upper_inclusive,
            carrier_end_exclusive,
            watermark,
        } => {
            json!({"kind":"event-position", "lower":lower.to_string(), "upper_inclusive":upper_inclusive.to_string(), "carrier_end_exclusive":carrier_end_exclusive.to_string(), "watermark":watermark.to_string()})
        }
        TemporalBoundary::FixedSample {
            lower,
            upper_inclusive,
            carrier_end_exclusive,
            watermark,
        } => {
            json!({"kind":"fixed-sample", "lower":lower.to_string(), "upper_inclusive":upper_inclusive.to_string(), "carrier_end_exclusive":carrier_end_exclusive.to_string(), "watermark":watermark.to_string()})
        }
        TemporalBoundary::TimestampedEvent {
            lower_nanos,
            upper_inclusive_nanos,
            carrier_end_exclusive_nanos,
            watermark_nanos,
        } => {
            json!({"kind":"timestamped-event", "lower_nanos":lower_nanos.to_string(), "upper_inclusive_nanos":upper_inclusive_nanos.to_string(), "carrier_end_exclusive_nanos":carrier_end_exclusive_nanos.to_string(), "watermark_nanos":watermark_nanos.to_string()})
        }
    }
}
