// SPDX-License-Identifier: AGPL-3.0-or-later
//! A claim read over a set of box readings (FSpec FR-652 "Reading the
//! claim"): each claim atom's three-valued reading over a box, the per-instant
//! meet of the readings holding at an instant, and FR-416's pointwise meaning
//! lifted to three values by [`Timeline`].
//!
//! A reading's box is extended to `[variables, activation variables,
//! captures]`, so an activation variable and a capture read as ordinary
//! variables of the extended box.

use filament_exact::{Integer, Rational};
use fsl_semantics::check::{CheckedHybridClaim, CompareOp, HybridActivation};

use super::super::exact::{signal_of, Converter, Formula};
use super::super::expand::{result_kind, Exp, ResultKind, TermLocus};
use super::super::interval::{Interval, Tri};
use super::super::real::ExactReal;
use super::super::signal::{partition, Piece, Signal, Span, Timeline};
use super::super::UndefinedCause;
use super::machine::{Bx, Def, Machine, Stop, Undefined};

pub(super) struct ClaimEval {
    pub(super) formula: Formula,
    /// The claim's atoms in the extended layout.
    pub(super) atoms: Vec<Exp>,
    /// The `on each` guard, in the extended layout.
    pub(super) guard: Option<Exp>,
    pub(super) captures: Vec<(Exp, ResultKind)>,
    pub(super) n: usize,
    pub(super) each: bool,
}

/// What an activation's captures read as.
pub(super) struct Ctx {
    /// The activation variables, then the captures.
    pub(super) tail: Vec<Interval>,
}

pub(super) enum CapStatus {
    Ok,
    /// Some capture is not definitely defined or not definitely in range.
    Maybe,
    /// A capture is definitely without value or outside its range.
    Undefined(Undefined),
}

#[derive(Clone)]
/// One reading of the claim's atoms over a box, holding at the instants
/// `lo` to `hi` (no upper end: every later instant) of a segment.
pub(super) struct Reading {
    /// The certificate's step the reading is over, in list order.
    pub(super) step: usize,
    /// The timeline segment the reading belongs to.
    pub(super) segment: usize,
    pub(super) lo: Rational,
    pub(super) hi: Option<Rational>,
    pub(super) vals: Vec<(Tri, Def)>,
}

fn one() -> Rational {
    Rational::from_integer(Integer::from(1_i64))
}

fn rewrite(exp: &Exp, n: usize, boolean: &[bool]) -> Exp {
    exp.map_reads(&|read| match read {
        Exp::Activation(i) => Some(Exp::Variable(n + i)),
        Exp::Capture(j) => Some(if boolean.get(*j).copied().unwrap_or(false) {
            Exp::Compare(
                CompareOp::Eq,
                Box::new(Exp::Variable(2 * n + j)),
                Box::new(Exp::Number(one())),
            )
        } else {
            Exp::Variable(2 * n + j)
        }),
        _ => None,
    })
}

impl ClaimEval {
    pub(super) fn build(
        machine: &mut Machine<'_, '_>,
        claim: &CheckedHybridClaim,
        n: usize,
    ) -> Result<Self, Stop> {
        let mut converter = Converter::new(&mut machine.expander, &mut machine.scratch);
        let (formula, _) = converter.convert(&claim.formula)?;
        let raw_atoms = converter.into_atoms();
        let kinds: Vec<ResultKind> = claim
            .captures
            .iter()
            .map(|capture| result_kind(&capture.ty).unwrap_or(ResultKind::Boolean))
            .collect();
        let boolean: Vec<bool> = kinds
            .iter()
            .map(|kind| matches!(kind, ResultKind::Boolean))
            .collect();
        let atoms = raw_atoms
            .iter()
            .map(|atom| rewrite(atom, n, &boolean))
            .collect();
        let mut captures = Vec::new();
        for (capture, kind) in claim.captures.iter().zip(kinds) {
            let init = machine.expand(&capture.value)?;
            captures.push((rewrite(&init, n, &boolean), kind));
        }
        let (guard, each) = match &claim.activation {
            HybridActivation::Origin => (None, false),
            HybridActivation::Each { guard, .. } => {
                let guard = match guard {
                    Some(term) => Some(rewrite(&machine.expand(term)?, n, &boolean)),
                    None => None,
                };
                (guard, true)
            }
        };
        Ok(Self {
            formula,
            atoms,
            guard,
            captures,
            n,
            each,
        })
    }

    /// The captures of an activation at `state` with activation variables
    /// `activation`.
    pub(super) fn context(
        &self,
        machine: &mut Machine<'_, '_>,
        state: &Bx,
        activation: &Bx,
    ) -> Result<(Ctx, CapStatus), Stop> {
        let n = self.n;
        let mut tail: Vec<Interval> = activation.clone();
        tail.extend(std::iter::repeat_n(Interval::whole(), self.captures.len()));
        let mut status = CapStatus::Ok;
        for (j, (init, kind)) in self.captures.iter().enumerate() {
            let mut ext = state.clone();
            ext.extend(tail.iter().cloned());
            match machine.defined(init, &ext)? {
                Def::No(undefined) => return Ok((Ctx { tail }, CapStatus::Undefined(undefined))),
                Def::Maybe => status = CapStatus::Maybe,
                Def::Yes => {}
            }
            match kind {
                ResultKind::Boolean => {
                    tail[n + j] = match machine.tri(init, &ext)? {
                        Tri::True => Interval::point(one()),
                        Tri::False => Interval::point(Rational::from_integer(Integer::from(0_i64))),
                        Tri::Unknown => Interval {
                            lower: Some(Rational::from_integer(Integer::from(0_i64))),
                            upper: Some(one()),
                        },
                    };
                }
                ResultKind::Number(range) => {
                    let value = machine.num(init, &ext)?;
                    let below = matches!((&range.lower, &value.upper), (Some(l), Some(u)) if u < l);
                    let above = matches!((&range.upper, &value.lower), (Some(u), Some(l)) if l > u);
                    if below || above {
                        return Ok((
                            Ctx { tail },
                            CapStatus::Undefined(Undefined {
                                locus: TermLocus::default(),
                                cause: UndefinedCause::OutsideCaptureRange { capture: j },
                            }),
                        ));
                    }
                    let inside = range
                        .lower
                        .as_ref()
                        .is_none_or(|l| value.lower.as_ref().is_some_and(|v| v >= l))
                        && range
                            .upper
                            .as_ref()
                            .is_none_or(|u| value.upper.as_ref().is_some_and(|v| v <= u));
                    if !inside && matches!(status, CapStatus::Ok) {
                        status = CapStatus::Maybe;
                    }
                    tail[n + j] = value;
                }
            }
        }
        Ok((Ctx { tail }, status))
    }

    pub(super) fn ext(state: &Bx, ctx: &Ctx) -> Bx {
        let mut ext = state.clone();
        ext.extend(ctx.tail.iter().cloned());
        ext
    }

    /// Every atom's reading over `state` in `ctx`.
    pub(super) fn atoms_over(
        &self,
        machine: &mut Machine<'_, '_>,
        state: &Bx,
        ctx: &Ctx,
    ) -> Result<Vec<(Tri, Def)>, Stop> {
        let ext = Self::ext(state, ctx);
        self.atoms
            .iter()
            .map(|atom| machine.reading(atom, &ext))
            .collect()
    }

    /// The claim's value at the first position of segment `origin`'s start,
    /// over the readings `include` selects.
    pub(super) fn value(
        &self,
        readings: &[Reading],
        include: &[usize],
        spans: &[Span],
        tail: Option<ExactReal>,
    ) -> (Timeline, Vec<Signal>, Tri) {
        let timeline = Timeline {
            spans: spans.to_vec(),
            tail,
        };
        let signals = self.signals(readings, include, spans);
        let signal = signal_of(&self.formula, &signals, &timeline);
        let value = timeline.at_origin(&signal);
        (timeline, signals, value)
    }

    /// The signal of each atom: at every instant of a segment the meet of
    /// the readings of the segment holding there.
    pub(super) fn signals(
        &self,
        readings: &[Reading],
        include: &[usize],
        spans: &[Span],
    ) -> Vec<Signal> {
        (0..self.atoms.len())
            .map(|atom| {
                spans
                    .iter()
                    .enumerate()
                    .map(|(segment, span)| {
                        let here: Vec<&Reading> = include
                            .iter()
                            .map(|&i| &readings[i])
                            .filter(|reading| reading.segment == segment)
                            .collect();
                        let mut points: Vec<ExactReal> = Vec::new();
                        for reading in &here {
                            points.push(reading.lo.clone().into());
                            if let Some(hi) = &reading.hi {
                                points.push(hi.clone().into());
                            }
                        }
                        partition(span, points)
                            .into_iter()
                            .map(|(low, high, point)| {
                                let at: Rational = if point {
                                    low.as_rational()
                                        .cloned()
                                        .unwrap_or_else(|| low.between(&high))
                                } else {
                                    low.between(&high)
                                };
                                let mut values = here
                                    .iter()
                                    .filter(|reading| {
                                        reading.lo <= at
                                            && reading.hi.as_ref().is_none_or(|hi| at <= *hi)
                                    })
                                    .map(|reading| reading.vals[atom].0)
                                    .peekable();
                                let value = if values.peek().is_none() {
                                    Tri::Unknown
                                } else {
                                    let all: Vec<Tri> = values.collect();
                                    if all.iter().all(|v| *v == Tri::True) {
                                        Tri::True
                                    } else if all.iter().all(|v| *v == Tri::False) {
                                        Tri::False
                                    } else {
                                        Tri::Unknown
                                    }
                                };
                                Piece {
                                    low,
                                    high,
                                    point,
                                    value,
                                }
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect()
    }
}

/// The span `[low, high]`.
pub(super) fn span(low: &Rational, high: &Rational) -> Span {
    Span {
        low: low.clone().into(),
        high: high.clone().into(),
        high_closed: true,
    }
}

impl ClaimEval {
    /// The formula's signal over the readings `include` selects.
    pub(super) fn formula_signal(
        &self,
        readings: &[Reading],
        include: &[usize],
        spans: &[Span],
        tail: Option<ExactReal>,
    ) -> (Timeline, Signal) {
        let timeline = Timeline {
            spans: spans.to_vec(),
            tail,
        };
        let signals = self.signals(readings, include, spans);
        let signal = signal_of(&self.formula, &signals, &timeline);
        (timeline, signal)
    }
}
