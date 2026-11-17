// SPDX-License-Identifier: AGPL-3.0-or-later
//! The result identity (FR-501 "Result report"; FR-043-AC-13): a SHA-256
//! content identity over the RFC 8785 form of everything an evaluation read.

use filament_exact::ScalarLimits;
use filament_semantic_value::catalog::InternalFault;
use quire_canonical::{Encode, Error, Limits, Sink, Writer};

use super::cover::{ClockCover, StampInterval};
use super::lift::Cell;
use super::outcome::ResultIdentity;
use super::trace::{LateCutoff, Lateness, LowerBoundary, TimestampedTrace};
use crate::temporal::ledger::{ProgressClosure, ProgressCompleteness};
use crate::temporal::plan::fault;

/// What one evaluation read, in a fixed order.
pub(super) struct Preimage<'a> {
    pub(super) trace: &'a TimestampedTrace,
    pub(super) profile: &'static str,
    /// The atoms read, as `(position, atom node, valuation)`, sorted.
    pub(super) readings: &'a [(usize, usize, Cell)],
    pub(super) ceilings: &'a ScalarLimits,
}

/// A decimal string: an integer past 2^53 has no RFC 8785 number form.
fn count<S: Sink + ?Sized>(writer: &mut Writer<'_, S>, value: u128) -> Result<(), Error> {
    writer.string(&value.to_string())
}

fn closure(value: ProgressClosure) -> &'static str {
    match value {
        ProgressClosure::Closed => "closed",
        ProgressClosure::Open => "open",
    }
}

fn tag(cell: Cell) -> &'static str {
    match cell {
        Cell::True => "true",
        Cell::False => "false",
        Cell::Missing => "missing",
        Cell::Conflicting => "conflicting",
    }
}

/// An interval stamp's members: both ends and both flags.
fn interval_members<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    interval: StampInterval,
) -> Result<(), Error> {
    writer.string("interval")?;
    count(writer, u128::from(interval.earliest))?;
    count(writer, u128::from(interval.latest))?;
    writer.bool(interval.earliest_open)?;
    writer.bool(interval.latest_open)
}

/// FR-966 "Identity": the late-data cutoff (its rule and its parameter),
/// every excluded observation and every admitted one, written only when a
/// cutoff is selected or a repair admits an observation.
fn lateness_members<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    lateness: &Lateness,
) -> Result<(), Error> {
    if lateness.cutoff.is_none() && lateness.admitted.is_empty() {
        return Ok(());
    }
    writer.begin_array()?;
    match lateness.cutoff {
        Some(LateCutoff::AllowedLateness { lateness }) => {
            writer.string("allowed-lateness")?;
            count(writer, u128::from(lateness))?;
        }
        Some(LateCutoff::FixedInstant { cutoff }) => {
            writer.string("fixed-instant")?;
            count(writer, u128::from(cutoff))?;
        }
        None => writer.null()?,
    }
    writer.begin_array()?;
    for late in &lateness.late {
        writer.begin_array()?;
        writer.string(&late.observation)?;
        count(writer, u128::from(late.stamp))?;
        if let Some(interval) = late.interval {
            interval_members(writer, interval)?;
        }
        count(writer, u128::from(late.arrival))?;
        writer.end_array()?;
    }
    writer.end_array()?;
    writer.begin_array()?;
    for admitted in &lateness.admitted {
        writer.string(admitted)?;
    }
    writer.end_array()?;
    writer.end_array()
}

impl Encode for Preimage<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        let trace = self.trace;
        writer.begin_array()?;
        writer.string("filament.fsl.finite-window-result/v1")?;
        writer.string(&trace.clause_identity)?;
        writer.string(self.profile)?;
        writer.string(&trace.clock.unit)?;
        match trace.clock.cover {
            ClockCover::Exact => writer.string("exact")?,
            ClockCover::Counter { uncertainty } => {
                writer.string("counter")?;
                count(writer, u128::from(uncertainty))?;
            }
            ClockCover::Bounded { uncertainty } => {
                writer.string("bounded")?;
                count(writer, u128::from(uncertainty))?;
            }
        }
        writer.begin_array()?;
        for (index, position) in trace.positions.iter().enumerate() {
            writer.begin_array()?;
            count(writer, u128::from(position.stamp))?;
            // FR-966 "Identity": an interval stamp and an arrival watermark
            // are members of the position only when it carries them.
            if let Some(interval) = trace.intervals.get(index).copied().flatten() {
                interval_members(writer, interval)?;
            }
            if let Some(arrival) = trace.lateness.arrivals.get(index).copied().flatten() {
                writer.string("arrival")?;
                count(writer, u128::from(arrival))?;
            }
            match &position.order_key {
                Some(key) => {
                    writer.string(&key.authority)?;
                    writer.integer(i128::from(key.key))?;
                }
                None => writer.null()?,
            }
            writer.string(&position.observation)?;
            writer.end_array()?;
        }
        writer.end_array()?;
        writer.begin_array()?;
        for (position, node, cell) in self.readings {
            writer.begin_array()?;
            count(writer, *position as u128)?;
            count(writer, *node as u128)?;
            writer.string(tag(*cell))?;
            writer.end_array()?;
        }
        writer.end_array()?;
        let progress = &trace.asserted;
        writer.integer(progress.watermark.inclusive())?;
        match progress.completeness {
            ProgressCompleteness::Complete { through } => {
                writer.string("complete")?;
                count(writer, u128::from(through))?;
            }
            ProgressCompleteness::Incomplete => writer.string("incomplete")?,
        }
        writer.string(closure(progress.scope))?;
        writer.string(closure(progress.execution))?;
        writer.string(match progress.lower_boundary {
            LowerBoundary::AuthoritativeOrigin => "authoritative-origin",
            LowerBoundary::HistoryCutoff => "history-cutoff",
        })?;
        lateness_members(writer, &trace.lateness)?;
        let ceilings = self.ceilings;
        writer.begin_array()?;
        for ceiling in [
            ceilings.integer_bits,
            ceilings.decimal_digits,
            ceilings.scale_expansion,
            ceilings.text_input_bytes,
            ceilings.text_scalars,
            ceilings.normalized_scalars,
            ceilings.unit_edges,
            ceilings.value_occurrences,
            ceilings.work_units,
            ceilings.result_units,
        ] {
            count(writer, u128::from(ceiling))?;
        }
        writer.end_array()?;
        writer.end_array()
    }
}

impl Preimage<'_> {
    /// The identity of this preimage.
    pub(super) fn identity(&self) -> Result<ResultIdentity, InternalFault> {
        let digest = quire_canonical::sha256(self, Limits::new(u64::MAX))
            .map_err(|_| fault("result-identity-encodes"))?;
        let mut text = String::from("sha256:");
        for byte in digest.as_bytes() {
            text.push_str(&format!("{byte:02x}"));
        }
        Ok(ResultIdentity(text))
    }
}

/// What a correction's result identity reads: the run's input identity and
/// its relation to an earlier result (FR-526).
struct CorrectionPreimage<'a> {
    input: &'a str,
    relation: &'a str,
    predecessor: Option<&'a str>,
}

impl Encode for CorrectionPreimage<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_array()?;
        writer.string("filament.fsl.finite-window-correction/v1")?;
        writer.string(self.input)?;
        writer.string(self.relation)?;
        match self.predecessor {
            Some(predecessor) => writer.string(predecessor)?,
            None => writer.null()?,
        }
        writer.end_array()
    }
}

/// FR-526: the `result_identity` of a finite-window result whose input
/// identity is `input` and whose relation is `relation` (`original`,
/// `superseding` or `invalidating`) to `predecessor`, so two results of
/// equal inputs and different relations have different identities.
///
/// # Errors
///
/// An internal fault when the preimage does not encode.
pub fn correction_identity(
    input: &str,
    relation: &str,
    predecessor: Option<&str>,
) -> Result<ResultIdentity, InternalFault> {
    let preimage = CorrectionPreimage {
        input,
        relation,
        predecessor,
    };
    let digest = quire_canonical::sha256(&preimage, Limits::new(u64::MAX))
        .map_err(|_| fault("correction-identity-encodes"))?;
    let mut text = String::from("sha256:");
    for byte in digest.as_bytes() {
        text.push_str(&format!("{byte:02x}"));
    }
    Ok(ResultIdentity(text))
}
