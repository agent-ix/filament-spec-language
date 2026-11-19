// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-944: completeness covers the admitted population, independently of its boundary.

use super::*;
use crate::temporal::ProgressCompleteness;
use filament_observation::authority::{completeness, TemporalBoundary};

impl OwnerObservation {
    pub(in crate::temporal::evidence) fn completeness_selection(
        &self,
        asserted: ProgressCompleteness,
        retained: Option<ProgressCompleteness>,
    ) -> Result<completeness::Selection, CallFailure> {
        let through = match asserted {
            ProgressCompleteness::Complete { through } => Some(through),
            ProgressCompleteness::Incomplete => None,
        };
        let coordinate = through.or(match retained {
            Some(ProgressCompleteness::Complete { through }) => Some(through),
            Some(ProgressCompleteness::Incomplete) | None => None,
        });
        let complete = through.is_some();
        let boundary = match self.request.scope.range {
            ClockRange::EventPosition { .. } => {
                let upper = coordinate.unwrap_or(0);
                TemporalBoundary::EventPosition {
                    lower: upper,
                    upper_inclusive: upper,
                    carrier_end_exclusive: upper
                        .checked_add(1)
                        .ok_or_else(|| fault("completeness-boundary-fits"))?,
                    watermark: upper,
                }
            }
            ClockRange::FixedSample { .. } => {
                let upper = coordinate.unwrap_or(0);
                TemporalBoundary::FixedSample {
                    lower: upper,
                    upper_inclusive: upper,
                    carrier_end_exclusive: upper
                        .checked_add(1)
                        .ok_or_else(|| fault("completeness-boundary-fits"))?,
                    watermark: upper,
                }
            }
            ClockRange::Timestamp { .. } => {
                let tick = coordinate.map(i128::from).unwrap_or(i128::from(i64::MIN));
                let upper = self.scale_coordinate(tick)?;
                TemporalBoundary::TimestampedEvent {
                    lower_nanos: upper,
                    upper_inclusive_nanos: upper,
                    carrier_end_exclusive_nanos: upper
                        .checked_add(1)
                        .ok_or_else(|| fault("completeness-boundary-fits"))?,
                    watermark_nanos: upper,
                }
            }
        };
        let identity = Identity::new(canonical_text(
            &json!({"through": through.map(|value| value.to_string())}),
        )?);
        Ok(completeness::Selection::new(
            identity,
            boundary,
            self.completeness_facts(complete),
        ))
    }

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

    pub(in crate::temporal::evidence) fn completeness_view(
        &self,
        asserted: ProgressCompleteness,
        retained: Option<ProgressCompleteness>,
    ) -> Result<completeness::View, CallFailure> {
        let selection = self.completeness_selection(asserted, retained)?;
        let context = self.context();
        let document = completeness::derive(context, &selection, Limits::owner_max())
            .map_err(|error| owner_fault(error.code().as_str(), error.detail()))?;
        completeness::read(document.bytes(), context, &selection, Limits::owner_max())
            .map_err(|error| owner_fault(error.code().as_str(), error.detail()))
    }

    fn scale_coordinate(&self, tick: i128) -> Result<i128, CallFailure> {
        let scaled = self
            .time_factor
            .mul(&filament_exact::Rational::from_integer(
                filament_exact::Integer::from(tick),
            ));
        if !scaled.is_integer() {
            return Err(fault("owner-boundary-is-integral-nanoseconds"));
        }
        scaled
            .numerator()
            .to_string()
            .parse()
            .map_err(|_| fault("owner-boundary-fits-nanoseconds"))
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
