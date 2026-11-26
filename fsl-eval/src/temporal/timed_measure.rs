// SPDX-License-Identifier: AGPL-3.0-or-later
//! The reference readings of FR-407's measures on one timed behaviour:
//! `elapsed`, and `steps`, `accumulate` and a `fraction` within a time
//! horizon (FSpec FR-407 "Bounded measures" and "Time windows").
//!
//! The behaviour is a [`TimedTrace`] and each predicate its truth at each
//! represented position; a lasso's later position reads the position it
//! repeats. A reader of a concrete run takes the trace and the truths from
//! the run; this module is the one reading of a measure over them. A finite trace is a prefix of unknown continuation, so a reading
//! it does not decide is [`MeasureReading::Pending`].
//!
//! Work: one `temporal.visit` unit for each position read.

use filament_exact::{Charge, Incomplete, Integer, Meter, Rational};
use ix_trace_rs::implements;

use super::outcome::TemporalRefusal;
use super::plan::fault;
use super::position::{PositionRefusal, TemporalPosition};
use super::timed::{TimedTrace, VISIT};

/// What one measure reads on one behaviour.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeasureReading {
    /// The behaviour has no position that starts the measure.
    NotActivated,
    /// The horizon ended the measure before its end predicate held: greater
    /// than every finite threshold.
    Censored,
    /// The measure's value.
    Value(Rational),
    /// The positions of a weighted fraction's window weigh nothing, so it
    /// has no value.
    NoWeight,
    /// The recorded prefix does not decide the measure.
    Pending,
    /// The meter stopped the reading.
    Incomplete(Incomplete),
}

/// Why a scan ended without the position it looked for.
enum Missed {
    /// No later position lies within the horizon.
    Closed,
    /// The prefix ends before the horizon does.
    Open,
}

enum Stop {
    Incomplete(Incomplete),
    Refused(TemporalRefusal),
}

/// One trace read position by position.
struct Reader<'a> {
    trace: &'a TimedTrace,
    meter: &'a mut Meter,
}

impl Reader<'_> {
    /// The represented position that position `k` reads and its time stamp,
    /// or `None` past the end of a finite trace.
    fn at(&mut self, k: usize) -> Result<Option<(usize, Rational)>, Stop> {
        self.meter
            .charge(Charge::new(VISIT))
            .map_err(Stop::Incomplete)?;
        if let Some(stamp) = self.trace.stamps.get(k) {
            return Ok(Some((k, stamp.clone())));
        }
        let Some(looping) = &self.trace.looping else {
            return Ok(None);
        };
        let past = k - self.trace.stamps.len();
        let length = self.trace.stamps.len() - looping.start;
        let image = looping.start + past % length;
        let repetitions = u64::try_from(past / length)
            .ok()
            .and_then(|whole| whole.checked_add(1))
            .ok_or(Stop::Refused(TemporalRefusal::TraceTooLong))?;
        let shift = looping
            .delay
            .mul(&Rational::from_integer(Integer::from(repetitions)));
        let stamp = self
            .trace
            .stamps
            .get(image)
            .ok_or(Stop::Refused(TemporalRefusal::Fault(fault(
                "position-has-a-stamp",
            ))))?;
        Ok(Some((image, stamp.add(&shift))))
    }

    /// The first position at or after `from` whose stamp is at most `horizon`
    /// after `origin` and at which `truths` holds, with its time stamp.
    fn first(
        &mut self,
        from: usize,
        origin: &Rational,
        horizon: &Rational,
        truths: &[bool],
    ) -> Result<Result<(usize, Rational), Missed>, Stop> {
        let mut k = from;
        loop {
            let Some((image, stamp)) = self.at(k)? else {
                return Ok(Err(self.end_of_prefix(origin, horizon)));
            };
            if &stamp.sub(origin) > horizon {
                return Ok(Err(Missed::Closed));
            }
            if truth(truths, image)? {
                return Ok(Ok((k, stamp)));
            }
            k += 1;
        }
    }

    fn end_of_prefix(&self, origin: &Rational, horizon: &Rational) -> Missed {
        if self
            .trace
            .through
            .as_ref()
            .is_some_and(|through| &through.sub(origin) >= horizon)
        {
            Missed::Closed
        } else {
            Missed::Open
        }
    }

    /// The represented position `position` and its stamp.
    fn represented(&self, position: TemporalPosition) -> Result<(usize, Rational), Stop> {
        usize::try_from(position.index())
            .ok()
            .and_then(|index| Some((index, self.trace.stamps.get(index)?.clone())))
            .ok_or_else(|| outside(position))
    }
}

fn outside(position: TemporalPosition) -> Stop {
    Stop::Refused(PositionRefusal::OutsideTrace { position }.into())
}

fn truth(truths: &[bool], image: usize) -> Result<bool, Stop> {
    truths.get(image).copied().ok_or_else(|| {
        outside(TemporalPosition::new(
            u64::try_from(image).unwrap_or(u64::MAX),
        ))
    })
}

fn settled(reading: Result<MeasureReading, Stop>) -> Result<MeasureReading, TemporalRefusal> {
    match reading {
        Ok(reading) => Ok(reading),
        Err(Stop::Incomplete(incomplete)) => Ok(MeasureReading::Incomplete(incomplete)),
        Err(Stop::Refused(refusal)) => Err(refusal),
    }
}

fn count(positions: usize) -> Rational {
    Rational::from_integer(Integer::from(u64::try_from(positions).unwrap_or(u64::MAX)))
}

/// `elapsed from holds(A) until holds(B) within horizon`, read from the
/// activating position `activation`: the time from the first position at or
/// after it where `from` holds, however late, to the first position at or
/// after that where `until` holds, censored when the elapsed time passes
/// `horizon` first.
///
/// # Errors
///
/// A position the trace or a truth slice does not represent.
#[implements("FSpec-FR-407-AC-26")]
pub fn timed_elapsed(
    trace: &TimedTrace,
    activation: TemporalPosition,
    from: &[bool],
    until: &[bool],
    horizon: &Rational,
    meter: &mut Meter,
) -> Result<MeasureReading, TemporalRefusal> {
    let mut reader = Reader { trace, meter };
    settled((|| {
        let (mut start, _) = reader.represented(activation)?;
        // A lasso repeats its letters, so a predicate that holds at no
        // represented position from here on holds at none later.
        let reachable = trace
            .looping
            .as_ref()
            .map_or(start, |looping| start.max(looping.start))
            .saturating_add(trace.stamps.len());
        let started = loop {
            let Some((image, stamp)) = reader.at(start)? else {
                return Ok(MeasureReading::Pending);
            };
            if truth(from, image)? {
                break stamp;
            }
            start += 1;
            if trace.is_lasso() && start > reachable {
                return Ok(MeasureReading::NotActivated);
            }
        };
        Ok(match reader.first(start, &started, horizon, until)? {
            Ok((_, ended)) => MeasureReading::Value(ended.sub(&started)),
            Err(Missed::Closed) => MeasureReading::Censored,
            Err(Missed::Open) => MeasureReading::Pending,
        })
    })())
}

/// The positions a `steps` or `accumulate` measure runs between: the first
/// position of the window where `from` holds and the first position of the
/// window at or after it where `until` holds, or the reading where there are
/// not both.
fn between(
    reader: &mut Reader<'_>,
    activation: TemporalPosition,
    from: &[bool],
    until: &[bool],
    horizon: &Rational,
) -> Result<Result<(usize, usize), MeasureReading>, Stop> {
    let (start, origin) = reader.represented(activation)?;
    let first = match reader.first(start, &origin, horizon, from)? {
        Ok((first, _)) => first,
        Err(Missed::Closed) => return Ok(Err(MeasureReading::NotActivated)),
        Err(Missed::Open) => return Ok(Err(MeasureReading::Pending)),
    };
    Ok(match reader.first(first, &origin, horizon, until)? {
        Ok((last, _)) => Ok((first, last)),
        Err(Missed::Closed) => Err(MeasureReading::Censored),
        Err(Missed::Open) => Err(MeasureReading::Pending),
    })
}

/// `steps from holds(A) until holds(B) within horizon` with a time horizon,
/// read from the activating position `activation`: the steps from the first
/// position of the window where `from` holds to the first position of the
/// window at or after it where `until` holds.
///
/// # Errors
///
/// As [`timed_elapsed`].
#[implements("FSpec-FR-407-AC-23")]
pub fn timed_steps(
    trace: &TimedTrace,
    activation: TemporalPosition,
    from: &[bool],
    until: &[bool],
    horizon: &Rational,
    meter: &mut Meter,
) -> Result<MeasureReading, TemporalRefusal> {
    let mut reader = Reader { trace, meter };
    settled((|| {
        Ok(
            match between(&mut reader, activation, from, until, horizon)? {
                Ok((first, last)) => MeasureReading::Value(count(last - first)),
                Err(reading) => reading,
            },
        )
    })())
}

/// `accumulate R from holds(A) until holds(B) within horizon` with a time
/// horizon over a finite trace, between the positions [`timed_steps`] reads:
/// the sum of `rewards[k]`, `R` of the step into position `k`, over the
/// positions after the first up to the second.
///
/// # Errors
///
/// As [`timed_elapsed`], and a position `rewards` has no entry for.
#[implements("FSpec-FR-407-AC-23")]
pub fn timed_accumulate(
    trace: &TimedTrace,
    activation: TemporalPosition,
    from: &[bool],
    until: &[bool],
    rewards: &[Rational],
    horizon: &Rational,
    meter: &mut Meter,
) -> Result<MeasureReading, TemporalRefusal> {
    let mut reader = Reader { trace, meter };
    settled((|| {
        let (first, last) = match between(&mut reader, activation, from, until, horizon)? {
            Ok(positions) => positions,
            Err(reading) => return Ok(reading),
        };
        let mut sum = Rational::from_integer(Integer::zero());
        for k in first + 1..=last {
            sum = sum.add(entry(rewards, k)?);
        }
        Ok(MeasureReading::Value(sum))
    })())
}

fn entry(values: &[Rational], k: usize) -> Result<&Rational, Stop> {
    values
        .get(k)
        .ok_or_else(|| outside(TemporalPosition::new(u64::try_from(k).unwrap_or(u64::MAX))))
}

/// How a fraction weighs the positions of its window.
#[derive(Clone, Copy)]
enum Weigh<'w> {
    /// Every position 1.
    Positions,
    /// Position `k` by the `k`th entry.
    Given(&'w [Rational]),
    /// Each position by the time the trace spends there within the window.
    Delay,
}

fn fraction(
    trace: &TimedTrace,
    activation: TemporalPosition,
    holds: &[bool],
    horizon: &Rational,
    weigh: Weigh<'_>,
    meter: &mut Meter,
) -> Result<MeasureReading, TemporalRefusal> {
    let mut reader = Reader { trace, meter };
    settled((|| {
        let (start, origin) = reader.represented(activation)?;
        let close = origin.add(horizon);
        let zero = Rational::from_integer(Integer::zero());
        let one = Rational::from_integer(Integer::one());
        let (mut held, mut total) = (zero.clone(), zero);
        let mut read = false;
        let mut k = start;
        loop {
            let Some((image, stamp)) = reader.at(k)? else {
                if matches!(reader.end_of_prefix(&origin, horizon), Missed::Open) {
                    return Ok(MeasureReading::Pending);
                }
                break;
            };
            if stamp > close {
                break;
            }
            read = true;
            let weight = match weigh {
                Weigh::Positions => one.clone(),
                Weigh::Given(weights) => entry(weights, k)?.clone(),
                // The step that leaves the position lands at the next stamp;
                // one past the window, and an idle tail, give `close`.
                Weigh::Delay => reader
                    .at(k + 1)?
                    .map(|(_, next)| next)
                    .filter(|next| *next < close)
                    .unwrap_or_else(|| close.clone())
                    .sub(&stamp),
            };
            if truth(holds, image)? {
                held = held.add(&weight);
            }
            total = total.add(&weight);
            k += 1;
        }
        // The activating position is in its own window, so only a negative
        // horizon leaves the window empty.
        if !read {
            return Ok(MeasureReading::NotActivated);
        }
        Ok(held
            .div(&total)
            .map_or(MeasureReading::NoWeight, MeasureReading::Value))
    })())
}

/// `fraction holds(P) over [0, horizon]` with a time horizon, unweighted,
/// read from the activating position `activation`: the positions of the
/// window where `holds` holds over the positions of the window.
///
/// # Errors
///
/// As [`timed_elapsed`].
#[implements("FSpec-FR-407-AC-21")]
pub fn timed_fraction(
    trace: &TimedTrace,
    activation: TemporalPosition,
    holds: &[bool],
    horizon: &Rational,
    meter: &mut Meter,
) -> Result<MeasureReading, TemporalRefusal> {
    fraction(trace, activation, holds, horizon, Weigh::Positions, meter)
}

/// [`timed_fraction`] `weighted by R` over a finite trace: `weights[k]` is
/// `R` of the step that leaves position `k`, 0 where none does.
///
/// # Errors
///
/// As [`timed_elapsed`], and a position `weights` has no entry for.
#[implements("FSpec-FR-407-AC-22")]
pub fn timed_weighted_fraction(
    trace: &TimedTrace,
    activation: TemporalPosition,
    holds: &[bool],
    weights: &[Rational],
    horizon: &Rational,
    meter: &mut Meter,
) -> Result<MeasureReading, TemporalRefusal> {
    fraction(
        trace,
        activation,
        holds,
        horizon,
        Weigh::Given(weights),
        meter,
    )
}

/// [`timed_fraction`] `weighted by delay`: each position of the window
/// weighs the time to the next position, clipped to the window; the last
/// position of a complete trace weighs the time to the window's end.
///
/// # Errors
///
/// As [`timed_elapsed`].
#[implements("FSpec-FR-407-AC-20")]
pub fn timed_delay_fraction(
    trace: &TimedTrace,
    activation: TemporalPosition,
    holds: &[bool],
    horizon: &Rational,
    meter: &mut Meter,
) -> Result<MeasureReading, TemporalRefusal> {
    fraction(trace, activation, holds, horizon, Weigh::Delay, meter)
}
