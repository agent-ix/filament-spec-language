// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 "Exact replay": an `on each` claim under `model-time` whose
//! captures or atoms read the activation position's state. The activation
//! instant `tau` is a second real variable: along a pair of dwells (the
//! activation's and the one an atom is read in) every activation-state read,
//! capture and atom is a rational function of `(tau, t)`. The activation
//! instants of a dwell are cut at the roots of the projection onto `tau` of
//! every polynomial an evaluation of an obligation reads, so the obligation's
//! truth, and the instants at which an expression has no value, are constant
//! over each open cell and are read at one rational sample of it; each
//! rational cut is read at the cut itself. The cuts are found by the same
//! lazy fixpoint as the quantifier elimination of [`super::super::section`]:
//! a cell is evaluated, the polynomials the evaluation read are projected,
//! and the roots found cut the dwell until a pass finds none.
//!
//! Nested windows cut a second kind of critical value: where two root curves
//! of atoms are a window width apart, a region of positions a universal window
//! fits in appears or vanishes. Every polynomial the evaluation reads is also
//! read shifted by each difference of the window offsets, so the projection
//! holds their resultants.
//!
//! Work accounting: the meter is charged before every pass of the fixpoint,
//! every sample of `tau`, every re-evaluation of a segment's atoms at a
//! sample, every cell of an atom read bivariately, every capture read, every
//! root isolation of a projected polynomial, every period of the past-reach
//! ceiling, and (in [`super::super::poly2::projection`]) every evaluation
//! point of a subresultant interpolation. Arithmetic on one polynomial and
//! the sorting of cuts are not charged.

use std::cmp::Ordering;

use filament_exact::Rational;

use super::super::eval::{
    ceiling_bound, cells, eval_in, floor_bound, Algebra, Bi, Constant, Env, Ev, Probe,
};
use super::super::expand::{result_kind, Exp, ResultKind};
use super::super::interval::Tri;
use super::super::poly::Poly;
use super::super::poly2::{projection, Poly2};
use super::super::real::ExactReal;
use super::super::signal::{partition, Span, Timeline};
use super::super::trace::zero;
use super::super::{ExactGap, ExpressionSite, HybridReplayMeter, HybridReplayRefusal};
use super::{Claim, ClaimSegment, Formula, Period, Record, Records};

fn one() -> Rational {
    Rational::from_integer(1_i64.into())
}

/// The activation instants of an `on each` claim read by activation state.
pub(super) struct Obligations {
    /// The last activation instant decided on a lasso: the obligation
    /// activated later than it agrees with the one a period earlier. `None`
    /// on a finite trace, every activation instant of which is decided.
    pub(super) until: Option<Rational>,
}

/// The point `(tau, t)` of the plane: the activation instant and the
/// instant an expression is read at.
struct Instant<'a> {
    tau: &'a Rational,
    t: &'a ExactReal,
}

impl Probe<Bi> for Instant<'_> {
    fn sign(&self, poly: &Poly2) -> Ordering {
        self.t.sign_of(&poly.at_t(self.tau))
    }
}

/// The past reach of `formula` (FR-450 "Exact replay"): how far back from
/// an activation instant it reads, a past operator with upper bound `b`
/// adding `b`, an unbounded one with lower bound `a` adding `a + period`,
/// a future operator nothing, nested operators adding up.
fn past_reach(formula: &Formula, period: &Rational) -> Rational {
    match formula {
        Formula::Atom(_) => zero(),
        Formula::Not(f) => past_reach(f, period),
        Formula::Logic(_, a, b) => past_reach(a, period).max(past_reach(b, period)),
        Formula::Unary {
            window,
            past,
            operand,
            ..
        } => {
            let inner = past_reach(operand, period);
            if *past {
                bound(window.upper.as_ref(), &window.lower, period).add(&inner)
            } else {
                inner
            }
        }
        Formula::Binary {
            window,
            past,
            left,
            right,
            ..
        } => {
            let inner = past_reach(left, period).max(past_reach(right, period));
            if *past {
                bound(window.upper.as_ref(), &window.lower, period).add(&inner)
            } else {
                inner
            }
        }
    }
}

fn bound(upper: Option<&Rational>, lower: &Rational, period: &Rational) -> Rational {
    upper.cloned().unwrap_or_else(|| lower.add(period))
}

/// The last activation instant of a lasso replay decides: `onset` plus
/// `1 + ceil(R / P)` periods, `R` the past reach and `P` the period. The
/// ceiling is taken one charged period at a time.
pub(super) fn decided_until(
    formula: &Formula,
    period: &Period,
    meter: &mut HybridReplayMeter,
) -> Result<Rational, HybridReplayRefusal> {
    if period.period <= zero() {
        return Ok(period.onset.clone());
    }
    let reach = past_reach(formula, &period.period);
    let mut covered = zero();
    let mut periods = one();
    while covered < reach {
        meter.charge()?;
        covered = covered.add(&period.period);
        periods = periods.add(&one());
    }
    Ok(period.onset.add(&period.period.mul(&periods)))
}

/// The clock offsets `c` at which an atom's change of truth, read through
/// `formula`'s windows, meets the activation instant `tau`: `tau + c`.
fn offsets(formula: &Formula) -> Vec<Rational> {
    let shifted = |inner: Vec<Rational>, window: &super::Window, past: bool| {
        let mut out = inner.clone();
        let ends = std::iter::once(&window.lower).chain(window.upper.as_ref());
        for end in ends {
            for offset in &inner {
                out.push(if past {
                    offset.sub(end)
                } else {
                    offset.add(end)
                });
            }
        }
        out
    };
    let mut out = match formula {
        Formula::Atom(_) => vec![zero()],
        Formula::Not(f) => offsets(f),
        Formula::Logic(_, a, b) => {
            let mut out = offsets(a);
            out.extend(offsets(b));
            out
        }
        Formula::Unary {
            window,
            past,
            operand,
            ..
        } => shifted(offsets(operand), window, *past),
        Formula::Binary {
            window,
            past,
            left,
            right,
            ..
        } => {
            let mut inner = offsets(left);
            inner.extend(offsets(right));
            shifted(inner, window, *past)
        }
    };
    out.sort();
    out.dedup();
    out
}

/// What one cell of activation instants decides.
#[derive(Clone)]
struct Outcome {
    /// The obligation's truth; `None` for an irrational instant, which is
    /// not read.
    value: Option<Tri>,
    /// The earliest expression with no value.
    record: Option<Record>,
}

/// One cell of a dwell's activation instants: the instant `low` when
/// `point`, else the open interval `(low, high)`.
type Cell = (ExactReal, ExactReal, bool);

/// A polynomial of the dwell's solution as a polynomial in `x`, the instant
/// an atom is read at.
fn in_x(poly: &Poly) -> Poly2 {
    Poly2::new(
        poly.coefficients()
            .iter()
            .cloned()
            .map(Poly::constant)
            .collect(),
    )
}

/// The activation instants of `segment`, cut at `until`.
fn activations(segment: &ClaimSegment, until: Option<&Rational>) -> Option<Span> {
    let Some(until) = until else {
        return Some(segment.span.clone());
    };
    let until: ExactReal = until.clone().into();
    if segment.span.low > until {
        return None;
    }
    if segment.span.high > until || (segment.span.high == until && !segment.span.high_closed) {
        return Some(Span {
            low: segment.span.low.clone(),
            high: until,
            high_closed: true,
        });
    }
    Some(segment.span.clone())
}

/// The obligations' expressions read at the activation position, and the
/// lines the projection is taken with.
struct Decider<'a, 'c> {
    claim: &'a Claim<'c>,
    segments: &'a [ClaimSegment],
    timeline: &'a Timeline,
    /// Each capture, in terms of the activation position's state.
    captures: Vec<Exp>,
    /// Each atom with its captures replaced by their expressions.
    atoms: Vec<Exp>,
    /// The nonzero differences of the window offsets: the distances at
    /// which two root curves of atoms, read through nested windows, change
    /// the truth of an obligation (a region of positions that a universal
    /// window fits in appears or vanishes where two roots are a window
    /// width apart).
    shifts: Vec<Rational>,
    /// `x - tau - c` for each window offset `c`, and `x - s` for each end
    /// `s` of a segment: where an atom's change of truth can meet the
    /// instant `tau` reads, or a dwell's end.
    lines: Vec<Poly2>,
}

impl<'a, 'c> Decider<'a, 'c> {
    fn new(claim: &'a Claim<'c>, segments: &'a [ClaimSegment], timeline: &'a Timeline) -> Self {
        let mut captures: Vec<Exp> = Vec::with_capacity(claim.captures.len());
        for capture in &claim.captures {
            let inlined = capture.map_reads(&|read| match read {
                Exp::Variable(i) => Some(Exp::Activation(*i)),
                Exp::Capture(i) => captures.get(*i).cloned(),
                _ => None,
            });
            captures.push(inlined);
        }
        let atoms = claim
            .atoms
            .iter()
            .map(|atom| {
                atom.map_reads(&|read| match read {
                    Exp::Capture(i) => captures.get(*i).cloned(),
                    _ => None,
                })
            })
            .collect();
        let line = |constant: Poly| Poly2::new(vec![constant, Poly::constant(one())]);
        let window_offsets = offsets(&claim.formula);
        let mut shifts: Vec<Rational> = Vec::new();
        for a in &window_offsets {
            for b in &window_offsets {
                let difference = a.sub(b);
                if difference != zero() && !shifts.contains(&difference) {
                    shifts.push(difference);
                }
            }
        }
        let mut lines: Vec<Poly2> = window_offsets
            .into_iter()
            .map(|c| line(Poly::new(vec![c.neg(), one().neg()])))
            .collect();
        for segment in segments {
            for end in [&segment.span.low, &segment.span.high] {
                if let Some(end) = end.as_rational() {
                    let end = line(Poly::constant(end.neg()));
                    if !lines.contains(&end) {
                        lines.push(end);
                    }
                }
            }
        }
        Self {
            claim,
            segments,
            timeline,
            captures,
            atoms,
            shifts,
            lines,
        }
    }

    /// The polynomials in `tau` whose roots bound the cells the obligation
    /// activated at `tau` has one truth and one record over: the projection
    /// of every polynomial in `(tau, t)` the obligation's expressions read
    /// at the cell of `tau`, with the lines of [`Self::lines`].
    fn reads(
        &self,
        segment: &ClaimSegment,
        tau: &Rational,
        state: &[Poly],
        captures: Option<&[Constant]>,
        meter: &mut HybridReplayMeter,
    ) -> Result<Vec<Poly>, HybridReplayRefusal> {
        let mut read: Vec<Poly2> = Vec::new();
        let activation: Vec<Poly2> = segment
            .values
            .iter()
            .cloned()
            .map(Poly2::constant)
            .collect();
        let origin: ExactReal = zero().into();
        let here = Instant { tau, t: &origin };
        let env = Env::<Bi> {
            variables: &activation,
            captures: &[],
            activation: &activation,
        };
        if let Some(guard) = &self.claim.guard {
            eval_in::<Bi, _>(guard, &env, &here, &mut read)?;
        }
        for (index, capture) in self.captures.iter().enumerate() {
            meter.charge()?;
            let Ev::Number(value) = eval_in::<Bi, _>(capture, &env, &here, &mut read)? else {
                continue;
            };
            let range = self
                .claim
                .claim
                .captures
                .get(index)
                .and_then(|c| result_kind(&c.ty));
            if let Some(ResultKind::Number(range)) = range {
                if let Some(lower) = &range.lower {
                    read.push(Bi::sign_poly(&Bi::sub(&value, &Bi::constant(lower))));
                }
                if let Some(upper) = &range.upper {
                    read.push(Bi::sign_poly(&Bi::sub(&Bi::constant(upper), &value)));
                }
            }
        }
        if let Some(captures) = captures {
            for k in self.segments {
                meter.charge()?;
                let found = cells(
                    &self.claim.atoms,
                    &Env {
                        variables: &k.values,
                        captures,
                        activation: state,
                    },
                    &k.span.low,
                    &k.span.high,
                    k.span.high_closed,
                )?;
                let variables: Vec<Poly2> = k.values.iter().map(in_x).collect();
                let env = Env::<Bi> {
                    variables: &variables,
                    captures: &[],
                    activation: &activation,
                };
                for cell in found {
                    meter.charge()?;
                    let t = if cell.point {
                        cell.low.clone()
                    } else {
                        cell.low.between(&cell.high).into()
                    };
                    let at = Instant { tau, t: &t };
                    for atom in &self.atoms {
                        eval_in::<Bi, _>(atom, &env, &at, &mut read)?;
                    }
                }
            }
        }
        let mut set = self.lines.clone();
        let mut fresh: Vec<Poly2> = Vec::new();
        for poly in read {
            if !set.contains(&poly) && !fresh.contains(&poly) {
                fresh.push(poly);
            }
        }
        // The shifted copies are of the polynomials read, never of the
        // lines: a shifted line is a line already present.
        let moved: Vec<Poly2> = fresh
            .iter()
            .filter(|poly| poly.degree().is_some_and(|degree| degree > 0))
            .flat_map(|poly| self.shifts.iter().map(|shift| poly.shifted(shift)))
            .collect();
        set.extend(fresh);
        for poly in moved {
            if !set.contains(&poly) {
                set.push(poly);
            }
        }
        projection(&set, &mut || meter.charge())
    }

    /// The obligation activated at `tau` in `segment` (position `j`), its
    /// truth and earliest expression with no value, and the polynomials in
    /// `tau` that bound the cell it is read in.
    fn at(
        &self,
        j: usize,
        segment: &ClaimSegment,
        tau: &Rational,
        meter: &mut HybridReplayMeter,
    ) -> Result<(Outcome, Vec<Poly>), HybridReplayRefusal> {
        let at: ExactReal = tau.clone().into();
        let state: Vec<Poly> = segment
            .values
            .iter()
            .map(|p| Poly::constant(p.eval(tau)))
            .collect();
        let here = ClaimSegment {
            span: Span {
                low: at.clone(),
                high: at.clone(),
                high_closed: true,
            },
            values: state.clone(),
            segment: segment.segment,
            state: segment.state,
            time: None,
        };
        let mut records = Records::default();
        // The guard precedes the captures and cannot read them: a capture
        // is evaluated only where the guard does not fail.
        let guard = match &self.claim.guard {
            None => Tri::True,
            Some(guard) => self.claim.point_truth(
                guard,
                &here,
                &[],
                &state,
                &mut records,
                ExpressionSite::Activation,
            )?,
        };
        let mut value = Tri::True;
        let mut captures = None;
        if guard != Tri::False {
            captures = self.claim.captures(&here, &state, &mut records)?;
            if let Some(captures) = &captures {
                for _ in self.segments {
                    meter.charge()?;
                }
                let signal = self.claim.formula_signal(
                    self.timeline,
                    self.segments,
                    captures,
                    &state,
                    &mut records,
                )?;
                value = guard.not().or(Timeline::value(&signal, j, &at));
            }
        }
        let read = self.reads(segment, tau, &state, captures.as_deref(), meter)?;
        Ok((
            Outcome {
                value: Some(value),
                record: records.0,
            },
            read,
        ))
    }

    /// The cells of `span`, each decided, in order, once the cuts at the
    /// roots of every polynomial read are in.
    fn scan(
        &self,
        j: usize,
        segment: &ClaimSegment,
        span: &Span,
        meter: &mut HybridReplayMeter,
    ) -> Result<Vec<(Cell, Outcome)>, HybridReplayRefusal> {
        let mut cuts: Vec<ExactReal> = Vec::new();
        let mut done: Vec<(Cell, Outcome)> = Vec::new();
        let (floor, ceiling) = (floor_bound(&span.low), ceiling_bound(&span.high));
        loop {
            meter.charge()?;
            let mut results = Vec::new();
            let mut grew = false;
            for cell in partition(span, cuts.clone()) {
                if let Some((_, outcome)) = done.iter().find(|(known, _)| *known == cell) {
                    results.push((cell, outcome.clone()));
                    continue;
                }
                let sample = if cell.2 {
                    cell.0.as_rational().cloned()
                } else {
                    Some(cell.0.between(&cell.1))
                };
                let outcome = match sample {
                    None => Outcome {
                        value: None,
                        record: None,
                    },
                    Some(tau) => {
                        meter.charge()?;
                        let (outcome, bounds) = self.at(j, segment, &tau, meter)?;
                        for poly in bounds {
                            meter.charge()?;
                            for root in poly.roots_in(&floor, &ceiling) {
                                if root > span.low && root < span.high && !cuts.contains(&root) {
                                    cuts.push(root);
                                    grew = true;
                                }
                            }
                        }
                        outcome
                    }
                };
                done.push((cell.clone(), outcome.clone()));
                results.push((cell, outcome));
            }
            if !grew {
                return Ok(results);
            }
        }
    }
}

impl Claim<'_> {
    /// The claim's value over `segments`: the conjunction of the
    /// obligations activated at every activation instant (FR-450 "Exact
    /// replay": `tau` as a second real variable), keeping the earliest
    /// expression with no value in `records`.
    pub(super) fn each_time(
        &self,
        obligations: &Obligations,
        segments: &[ClaimSegment],
        tail: &ExactReal,
        meter: &mut HybridReplayMeter,
        records: &mut Records,
    ) -> Result<Tri, HybridReplayRefusal> {
        let timeline = Timeline {
            spans: segments.iter().map(|s| s.span.clone()).collect(),
            tail: Some(tail.clone()),
        };
        let decider = Decider::new(self, segments, &timeline);
        let mut claim = Tri::True;
        let mut unread: Option<(usize, ExactReal)> = None;
        for (j, segment) in segments.iter().enumerate() {
            let Some(span) = activations(segment, obligations.until.as_ref()) else {
                continue;
            };
            for ((low, _, point), outcome) in decider.scan(j, segment, &span, meter)? {
                let Some(value) = outcome.value else {
                    unread.get_or_insert((segment.segment, low));
                    continue;
                };
                claim = claim.and(value);
                let Some(mut record) = outcome.record else {
                    continue;
                };
                if !point {
                    // The record holds throughout the open cell: its
                    // infimum is the cell's start, when an activation
                    // instant is what it is read at.
                    if !matches!(
                        record.locus.site,
                        ExpressionSite::Activation | ExpressionSite::Capture { .. }
                    ) {
                        return Err(ExactGap::ActivationInfimum.into());
                    }
                    record.time = low;
                    record.open = true;
                }
                records.keep(record);
            }
        }
        match (&records.0, unread) {
            (_, None) => Ok(claim),
            (Some(kept), Some((segment, at))) if (kept.segment, &kept.time) < (segment, &at) => {
                Ok(claim)
            }
            (_, Some(_)) => Err(ExactGap::ActivationInfimum.into()),
        }
    }
}
