// SPDX-License-Identifier: AGPL-3.0-or-later
//! The result identity (FR-501 "Result report"; FR-043-AC-13): a SHA-256
//! content identity over the RFC 8785 forms of everything an evaluation read.
//!
//! Each position contributes one block, in canonical order: its stamp, order
//! key and observation identity, and the valuations read there. The blocks
//! are chained: link `i` is the digest of `[link i-1, block i]`, from an
//! empty link. The identity is the digest of the clause, profile and clock,
//! the last link and the block count, the progress and the meter ceilings.
//! A block is chained once no later evaluation can read another valuation
//! at its position, so [`IdentityStream`] carries the chain and a report
//! chains only the blocks still open: the batch run and the carried run
//! (FR-820) chain the same blocks in the same order and give one identity by
//! construction. Every form is encoded and hashed through `quire-canonical`
//! (ADR-013 §2).

use filament_exact::ScalarLimits;
use filament_semantic_value::catalog::InternalFault;
use quire_canonical::{Encode, Error, Limits, Sink, Writer};

use super::cover::{ClockCover, StampInterval};
use super::lift::Cell;
use super::outcome::ResultIdentity;
use super::trace::{
    LateCutoff, Lateness, LowerBoundary, TimestampedObservation, WindowClock, WindowProgress,
};
use crate::temporal::ledger::{ProgressClosure, ProgressCompleteness};
use crate::temporal::plan::fault;

/// A decimal string: an integer past 2^53 has no RFC 8785 number form.
fn count<S: Sink + ?Sized>(writer: &mut Writer<'_, S>, value: u128) -> Result<(), Error> {
    writer.string(&value.to_string())
}

/// One position's block: its stamp, order key and observation identity, and
/// the `(atom class, valuation)` pairs read there, in class order.
pub(super) struct Block<'a> {
    pub(super) observation: &'a TimestampedObservation,
    /// The position's interval stamp, when it carries one (FR-501).
    pub(super) interval: Option<StampInterval>,
    /// The position's arrival watermark, when it states one (FR-966).
    pub(super) arrival: Option<u64>,
    pub(super) readings: &'a [(usize, Cell)],
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

impl Encode for Block<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        let position = self.observation;
        writer.begin_array()?;
        count(writer, u128::from(position.stamp))?;
        // FR-966 "Identity": an interval stamp and an arrival watermark are
        // members of the position only when it carries them.
        if let Some(interval) = self.interval {
            interval_members(writer, interval)?;
        }
        if let Some(arrival) = self.arrival {
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
        writer.begin_array()?;
        for (class, cell) in self.readings {
            writer.begin_array()?;
            count(writer, *class as u128)?;
            writer.string(tag(*cell))?;
            writer.end_array()?;
        }
        writer.end_array()?;
        writer.end_array()
    }
}

/// One link of the chain: the previous link and the next block.
struct Link<'a> {
    previous: &'a str,
    block: &'a Block<'a>,
}

impl Encode for Link<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_array()?;
        writer.string(self.previous)?;
        self.block.encode_into(writer)?;
        writer.end_array()
    }
}

/// The closing preimage: the header, the chain and the progress and
/// ceilings.
struct Head<'a> {
    clause_identity: &'a str,
    profile: &'a str,
    clock: &'a WindowClock,
    link: &'a str,
    blocks: u64,
    progress: &'a WindowProgress,
    lateness: &'a Lateness,
    ceilings: &'a ScalarLimits,
}

fn closure(value: ProgressClosure) -> &'static str {
    match value {
        ProgressClosure::Closed => "closed",
        ProgressClosure::Open => "open",
    }
}

impl Encode for Head<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_array()?;
        writer.string("filament.fsl.finite-window-result/v2")?;
        writer.string(self.clause_identity)?;
        writer.string(self.profile)?;
        writer.string(&self.clock.unit)?;
        match self.clock.cover {
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
        writer.string(self.link)?;
        count(writer, u128::from(self.blocks))?;
        let progress = self.progress;
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
        lateness_members(writer, self.lateness)?;
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

/// The `sha256:` text of the digest of `value`.
fn digest(value: &impl Encode) -> Result<String, InternalFault> {
    let digest = quire_canonical::sha256(value, Limits::new(u64::MAX))
        .map_err(|_| fault("result-identity-encodes"))?;
    let mut text = String::from("sha256:");
    for byte in digest.as_bytes() {
        text.push_str(&format!("{byte:02x}"));
    }
    Ok(text)
}

/// The chain of the blocks frozen so far.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct IdentityStream {
    link: String,
    blocks: u64,
}

impl IdentityStream {
    /// Chain `block`, the next position's, which no later evaluation reads
    /// again.
    pub(super) fn freeze(&mut self, block: &Block<'_>) -> Result<(), InternalFault> {
        self.link = digest(&Link {
            previous: &self.link,
            block,
        })?;
        self.blocks = self.blocks.saturating_add(1);
        Ok(())
    }

    /// The identity of the evaluation of the clause `clause_identity` under
    /// `profile` and `clock`, with the `open` blocks chained after the frozen
    /// ones, the asserted `progress` and late-data inputs, and `ceilings`.
    pub(super) fn identity(
        &self,
        clause_identity: &str,
        profile: &str,
        clock: &WindowClock,
        open: &[Block<'_>],
        (progress, lateness): (&WindowProgress, &Lateness),
        ceilings: &ScalarLimits,
    ) -> Result<ResultIdentity, InternalFault> {
        let mut chain = self.clone();
        for block in open {
            chain.freeze(block)?;
        }
        digest(&Head {
            clause_identity,
            profile,
            clock,
            link: &chain.link,
            blocks: chain.blocks,
            progress,
            lateness,
            ceilings,
        })
        .map(ResultIdentity)
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
