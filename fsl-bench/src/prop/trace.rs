// SPDX-License-Identifier: AGPL-3.0-or-later
//! Nonempty finite traces and lassos over three propositions.
use fsl_eval::temporal::TemporalTrace;
use fsl_semantics::check::TemporalProfile;
use proptest::prelude::{BoxedStrategy, Strategy};

/// A trace's valuations and real evaluator shape, under one selected profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropositionTrace {
    /// The profile the downstream suite must check under.
    pub profile: TemporalProfile,
    /// Three-bit proposition valuations in represented position order.
    pub valuations: Vec<u8>,
    /// Finite-prefix or lasso shape passed directly to the evaluator.
    pub shape: TemporalTrace,
    /// Strictly increasing integer timestamps for time-aware profiles.
    pub timestamps: Vec<u64>,
}

/// Generate finite traces for every profile, and lassos for InfiniteTrace.
/// Timed and timestamped profiles retain explicit timestamp data.
pub fn traces() -> BoxedStrategy<PropositionTrace> {
    (
        proptest::sample::select(TemporalProfile::ALL.to_vec()),
        proptest::collection::vec(0u8..8, 1..=12),
        proptest::bool::ANY,
        0usize..12,
    )
        .prop_map(|(profile, valuations, lasso, choice)| {
            let len = u64::try_from(valuations.len()).expect("at most twelve positions");
            let shape = if profile == TemporalProfile::InfiniteTrace && lasso {
                let prefix = u64::try_from(choice % valuations.len())
                    .expect("at most eleven prefix positions");
                TemporalTrace::Lasso {
                    prefix,
                    loop_len: len - prefix,
                }
            } else {
                TemporalTrace::Finite { len }
            };
            let timestamps = match profile {
                TemporalProfile::Timed | TemporalProfile::TimestampedEventFiniteWindow => {
                    (0..len).collect()
                }
                TemporalProfile::EventPositionFalseExtension
                | TemporalProfile::FixedSampleFalseExtension
                | TemporalProfile::InfiniteTrace => Vec::new(),
            };
            PropositionTrace {
                profile,
                valuations,
                shape,
                timestamps,
            }
        })
        .boxed()
}
