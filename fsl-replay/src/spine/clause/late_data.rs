// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-501 "Bounded cover and interval stamps" and "Late-data cutoff", FR-966:
//! admit a timestamped trace's interval stamps and classify its late
//! observations before the evaluator reads any position.
//!
//! A late observation is excluded from evaluation, so it takes no position:
//! the trace the evaluator reads holds the on-time observations only, and
//! the result's `late` list names each excluded one by its place in the
//! request. A superseding repair (FR-526) names late observations it admits,
//! which are then read as on-time.

use std::collections::BTreeMap;

use fsl_eval::temporal::{ClockCover, LateObservation, Lateness, StampInterval};
use fsl_semantics::check::Activation;

use super::progress::ProgressAssertion;
use super::temporal::{
    document_identity, observation_identity, refused, AdmittedLate, LateEntry, LateInputs,
    TimestampedCover, TimestampedMissingValuation, TimestampedObservation, TimestampedWindow,
    TraceInput,
};
use super::ClauseDisposition;

/// The cover a trace's stamps are read under, as the evaluator names it.
pub(super) fn eval_cover(cover: Option<TimestampedCover>) -> ClockCover {
    match cover.unwrap_or(TimestampedCover::Exact) {
        TimestampedCover::Exact => ClockCover::Exact,
        TimestampedCover::Counter { uncertainty } => ClockCover::Counter { uncertainty },
        TimestampedCover::Bounded { uncertainty } => ClockCover::Bounded { uncertainty },
    }
}

/// A timestamped trace with its late observations taken out.
pub(super) struct Screened {
    /// The trace of the on-time observations, the admitted late ones
    /// included, in input order.
    pub(super) trace: TraceInput,
    /// The `late` list, when a late-data cutoff is selected; each entry's
    /// `anchors` are filled once canonical positions exist.
    pub(super) late: Option<Vec<LateEntry>>,
    /// The late observations a repair admitted.
    pub(super) admitted: Vec<AdmittedLate>,
    /// The `admit_late` names no late observation carries.
    pub(super) unmatched: Vec<String>,
    /// What the result identity reads of the cutoff and the classification.
    pub(super) lateness: Lateness,
    /// The input positions of the observations `trace` keeps, in input
    /// order.
    pub(super) kept: Vec<usize>,
}

/// FR-966 "Interval stamp admission": the first observation, in input order,
/// whose interval stamp is malformed or does not hold its nominal stamp,
/// refused at stage `admit` naming member `interval` and its position.
pub(super) fn admit_intervals(
    observations: &[TimestampedObservation],
    late: &LateInputs,
) -> Result<(), Box<ClauseDisposition>> {
    for (position, (observed, interval)) in observations.iter().zip(&late.intervals).enumerate() {
        if interval.is_some_and(|interval| !interval.admits(observed.stamp)) {
            return Err(Box::new(refused(
                "invalid-value",
                BTreeMap::from([
                    ("member", "interval".to_owned()),
                    ("position", position.to_string()),
                ]),
            )));
        }
    }
    Ok(())
}

/// FR-501 "Late-data cutoff": classify every observation of the timestamped
/// trace and keep the on-time ones. `None` when the trace selects no cutoff
/// and names no observation to admit, so it is read as it is.
///
/// # Errors
///
/// The `admit` refusal of a trace whose every observation is late, which
/// has no position to evaluate (FR-966 "Positions of on-time observations").
pub(super) fn screen(
    observations: &[TimestampedObservation],
    progress: ProgressAssertion,
    window: &TimestampedWindow,
) -> Result<Option<Screened>, Box<ClauseDisposition>> {
    let late_inputs = &window.late;
    let Some(cutoff) = late_inputs.cutoff else {
        if late_inputs.admit_late.is_empty() {
            return Ok(None);
        }
        return Ok(Some(Screened {
            trace: TraceInput::Timestamped {
                observations: observations.to_vec(),
                progress,
                window: Box::new(window.clone()),
            },
            late: None,
            admitted: Vec::new(),
            unmatched: late_inputs.admit_late.clone(),
            lateness: Lateness::default(),
            kept: (0..observations.len()).collect(),
        }));
    };
    let cover = eval_cover(window.cover);
    let mut arrival = progress.watermark;
    let mut kept = Vec::new();
    let mut late = Vec::new();
    let mut excluded = Vec::new();
    let mut admitted = Vec::new();
    for (position, observed) in observations.iter().enumerate() {
        let interval = interval_at(late_inputs, position);
        // An observation with no arrival watermark is admitted at the one of
        // the observation before it, or at `W` for the first.
        arrival = late_inputs
            .arrivals
            .get(position)
            .copied()
            .flatten()
            .unwrap_or(arrival);
        let instant = cover.latest_instant(observed.stamp, interval);
        let identity = document_identity(&observed.document);
        let index = u64::try_from(position).unwrap_or(u64::MAX);
        if cutoff.is_late(instant, arrival) {
            if late_inputs.admit_late.contains(&identity) {
                admitted.push(AdmittedLate {
                    position: index,
                    observation: identity,
                });
            } else {
                late.push(LateEntry {
                    position: index,
                    observation: identity,
                    stamp: instant,
                    arrival,
                    anchors: Vec::new(),
                });
                excluded.push(LateObservation {
                    observation: observation_identity(&observed.document),
                    stamp: observed.stamp,
                    interval,
                    arrival,
                });
                continue;
            }
        }
        kept.push((position, arrival));
    }
    if kept.is_empty() {
        return Err(Box::new(refused("invalid-value", BTreeMap::new())));
    }
    let unmatched = late_inputs
        .admit_late
        .iter()
        .filter(|name| !admitted.iter().any(|entry| &entry.observation == *name))
        .cloned()
        .collect();
    let renumbered: BTreeMap<u64, u64> = kept
        .iter()
        .enumerate()
        .filter_map(|(new, (old, _))| Some((u64::try_from(*old).ok()?, u64::try_from(new).ok()?)))
        .collect();
    let on_time = TimestampedWindow {
        missing_valuations: window
            .missing_valuations
            .iter()
            .filter_map(|missing| {
                Some(TimestampedMissingValuation {
                    position: *renumbered.get(&missing.position)?,
                    atom: missing.atom,
                })
            })
            .collect(),
        late: LateInputs {
            intervals: kept
                .iter()
                .map(|(position, _)| interval_at(late_inputs, *position))
                .collect(),
            arrivals: kept.iter().map(|(_, arrival)| Some(*arrival)).collect(),
            cutoff: Some(cutoff),
            ..late_inputs.clone()
        },
        ..window.clone()
    };
    Ok(Some(Screened {
        trace: TraceInput::Timestamped {
            observations: kept
                .iter()
                .filter_map(|(position, _)| observations.get(*position).cloned())
                .collect(),
            progress,
            window: Box::new(on_time),
        },
        late: Some(late),
        lateness: Lateness {
            cutoff: Some(cutoff),
            arrivals: Vec::new(),
            late: excluded,
            admitted: admitted
                .iter()
                .map(|entry| entry.observation.clone())
                .collect(),
        },
        admitted,
        unmatched,
        kept: kept.iter().map(|(position, _)| *position).collect(),
    }))
}

/// The interval stamp of the observation at input `position`, if any.
pub(super) fn interval_at(late: &LateInputs, position: usize) -> Option<StampInterval> {
    late.intervals.get(position).copied().flatten()
}

/// The canonical positions of the activations whose windows hold `instant`:
/// an activation anchored at a position stamped `a` reads `[a - past, a +
/// future]` (FR-501 "Late-data cutoff"). An `on origin` clause activates at
/// position 0 only.
pub(super) fn anchors(
    activation: &Activation,
    stamps: &[u64],
    (past, future): (u64, u64),
    instant: u64,
) -> Vec<u64> {
    let count = match activation {
        Activation::Origin => stamps.len().min(1),
        Activation::Each { .. } => stamps.len(),
    };
    stamps
        .iter()
        .take(count)
        .enumerate()
        .filter(|(_, stamp)| {
            let stamp = i128::from(**stamp);
            let instant = i128::from(instant);
            stamp - i128::from(past) <= instant && instant <= stamp + i128::from(future)
        })
        .filter_map(|(position, _)| u64::try_from(position).ok())
        .collect()
}
