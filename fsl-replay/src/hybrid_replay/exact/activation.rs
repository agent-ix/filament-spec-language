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
//! cut is read at the cut itself, a rational one directly and an algebraic
//! one through the section `t = tau` ([`super::super::section::Section`]):
//! the cells of an atom along a dwell are cut at the roots of the resultant
//! with `tau`'s polynomial of every polynomial read, and each polynomial's
//! sign is read at a point `(tau, x)` of the plane by a Sturm-Tarski query. The cuts are found by the same
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
//! ceiling, and at an algebraic activation instant every capture read and
//! every evaluation of the atoms at an instant. A step that does arithmetic
//! on polynomials is charged by the size of its operands (one word per
//! coefficient and per 64 bits of its largest part, 16 words to the unit,
//! each step at least one): the resultant with `tau`'s polynomial
//! ([`super::super::poly2::eliminate`]) and the subresultants of
//! [`super::super::poly2::projection`] by the determinant of each evaluation
//! point and the interpolation, and every sign at `tau` (see
//! [`super::super::section::Section`]) down to each bisection of `tau`'s
//! isolating interval. The sorting of cuts is not charged.
use std::cmp::Ordering;

use filament_exact::Rational;

use super::super::eval::{
    ceiling_bound, cells, cells_of, eval_in, floor_bound, Algebra, Bi, Constant, Env, Ev, Evaluate,
    Probe,
};
use super::super::expand::{result_kind, Exp, ResultKind};
use super::super::interval::Tri;
use super::super::poly::Poly;
use super::super::poly2::{eliminate, projection, Poly2};
use super::super::real::ExactReal;
use super::super::section::{Sample, Section};
use super::super::signal::{partition, Span, Timeline};
use super::super::trace::zero;
use super::super::{ExactGap, ExpressionSite, HybridReplayMeter, HybridReplayRefusal};
use super::{
    Claim, ClaimSegment, ExpressionLocus, Formula, Period, Record, Records, UndefinedCause,
};

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

/// The point `(tau, x)` of the plane at the activation instant `tau` of the
/// section the polynomials are read through, a real algebraic number.
struct SectionAt<'a, 'u, 'm> {
    section: &'a Section<'u, 'm>,
    at: &'a Sample,
}

impl Probe<Bi> for SectionAt<'_, '_, '_> {
    fn sign(&self, poly: &Poly2) -> Ordering {
        self.section.sign_of(poly, self.at)
    }
}

/// The atoms read along a dwell at an algebraic activation instant: each
/// polynomial in `(tau, x)` a decision reads is split at the roots of its
/// resultant with `tau`'s polynomial, which hold its roots in `x` at `tau`.
struct Atoms<'a, 'u, 'm> {
    section: &'a Section<'u, 'm>,
    tau: &'a ExactReal,
    minimal: &'a Poly,
    atoms: &'a [Exp],
    env: &'a Env<'a, Bi>,
    /// Every polynomial read, and each one's resultant with `tau`'s
    /// polynomial.
    read: Vec<Poly2>,
    eliminated: Vec<Poly>,
}

impl Evaluate for Atoms<'_, '_, '_> {
    fn at(&mut self, at: &ExactReal) -> Result<(Vec<Ev>, Vec<Poly>), HybridReplayRefusal> {
        self.section.charge()?;
        let sample = Sample::of(at);
        let probe = SectionAt {
            section: self.section,
            at: &sample,
        };
        let mut decisions: Vec<Poly2> = Vec::new();
        let mut values = Vec::with_capacity(self.atoms.len());
        for atom in self.atoms {
            let value = eval_in::<Bi, _>(atom, self.env, &probe, &mut decisions)?;
            if let Some(refusal) = self.section.take_failure() {
                return Err(refusal);
            }
            values.push(match value {
                Ev::Boolean(value) => Ev::Boolean(value),
                Ev::Undefined { locus, cause } => Ev::Undefined { locus, cause },
                Ev::Number(_) => return Err(ExactGap::IllKindedTerm.into()),
            });
        }
        let mut splits: Vec<Poly> = Vec::new();
        for decision in decisions {
            let known = self.read.iter().position(|known| *known == decision);
            let eliminated = match known {
                Some(index) => self.eliminated[index].clone(),
                None => {
                    let section = self.section;
                    let eliminated = if decision.degree().is_some_and(|degree| degree > 0) {
                        section.charge_by(decision.size().saturating_mul(self.minimal.size()))?;
                        // `tau`'s polynomial is square-free, not irreducible:
                        // a factor it shares with every coefficient of the
                        // decision makes the resultant vanish. If `tau` is a
                        // root of that factor the decision vanishes at `tau`
                        // for every `x` and has no roots there; else `tau` is
                        // a root of the cofactor, which shares none.
                        let content = decision
                            .coefficients()
                            .iter()
                            .fold(Poly::zero(), |common, c| common.gcd(c));
                        let shared = self.minimal.gcd(&content);
                        if shared.degree().is_some_and(|degree| degree > 0) {
                            if self
                                .tau
                                .sign_with(&shared, &mut |work| section.charge_by(work))?
                                == Ordering::Equal
                            {
                                Poly::zero()
                            } else {
                                eliminate(
                                    &self.minimal.exact_div(&shared),
                                    &decision,
                                    &mut |work| section.charge_by(work),
                                )?
                            }
                        } else {
                            eliminate(self.minimal, &decision, &mut |work| section.charge_by(work))?
                        }
                    } else {
                        Poly::zero()
                    };
                    self.read.push(decision);
                    self.eliminated.push(eliminated.clone());
                    eliminated
                }
            };
            if eliminated.degree().is_some_and(|degree| degree > 0) && !splits.contains(&eliminated)
            {
                splits.push(eliminated);
            }
        }
        Ok((values, splits))
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
    /// The obligation's truth.
    value: Tri,
    /// The earliest expression with no value.
    record: Option<Record>,
}

/// One cell of a dwell's activation instants: the instant `low` when
/// `point`, else the open interval `(low, high)`.
type Cell = (ExactReal, ExactReal, bool);

/// The activation instant an open cell was read at, and the polynomials the
/// reading held.
type Reading = Option<(Rational, Vec<Poly2>)>;

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
    ) -> Result<(Vec<Poly>, Vec<Poly2>), HybridReplayRefusal> {
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
                    let t = if cell.point {
                        cell.low.clone()
                    } else {
                        cell.low.between(&cell.high).into()
                    };
                    // Each atom's polynomials are signed at `t`, the work of
                    // which grows with its size.
                    meter.charge_by(
                        t.size()
                            .saturating_mul(u64::try_from(self.atoms.len()).unwrap_or(u64::MAX)),
                    )?;
                    let at = Instant { tau, t: &t };
                    for atom in &self.atoms {
                        eval_in::<Bi, _>(atom, &env, &at, &mut read)?;
                    }
                }
            }
        }
        self.project(read, &mut |work| meter.charge_by(work))
    }

    /// The projection onto `tau` of `read` and the lines, with the shifted
    /// copies of `read`, and the polynomials `read` held.
    fn project(
        &self,
        read: Vec<Poly2>,
        charge: &mut dyn FnMut(u64) -> Result<(), HybridReplayRefusal>,
    ) -> Result<(Vec<Poly>, Vec<Poly2>), HybridReplayRefusal> {
        let mut set = self.lines.clone();
        let mut fresh: Vec<Poly2> = Vec::new();
        let mut held: Vec<Poly2> = Vec::new();
        for poly in read {
            if held.contains(&poly) {
                continue;
            }
            held.push(poly.clone());
            if !set.contains(&poly) {
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
        Ok((projection(&set, charge)?, held))
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
    ) -> Result<(Outcome, Vec<Poly>, Vec<Poly2>), HybridReplayRefusal> {
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
        let (bounds, read) = self.reads(segment, tau, &state, captures.as_deref(), meter)?;
        Ok((
            Outcome {
                value,
                record: records.0,
            },
            bounds,
            read,
        ))
    }

    /// [`Self::at`] at the activation instant `tau`, a real algebraic
    /// number: every expression is read at the point `(tau, x)` of the
    /// plane, through the section `t = tau` (FR-450 "Exact replay"). The
    /// atoms' cells over each dwell are cut at the roots of the resultant
    /// with `tau`'s polynomial of every polynomial an evaluation read,
    /// which holds the roots of each at `tau` and the roots of its
    /// conjugates, and each cell is read at one sample of it.
    fn at_algebraic(
        &self,
        j: usize,
        segment: &ClaimSegment,
        tau: &ExactReal,
        meter: &mut HybridReplayMeter,
    ) -> Result<(Outcome, Vec<Poly>, Vec<Poly2>), HybridReplayRefusal> {
        let root = match tau {
            ExactReal::Rational(tau) => return self.at(j, segment, tau, meter),
            ExactReal::Root(root) => root,
        };
        let minimal = Poly::new(root.coefficients().to_vec());
        let section = Section::new(tau, meter);
        let activation: Vec<Poly2> = segment
            .values
            .iter()
            .cloned()
            .map(Poly2::constant)
            .collect();
        let env = Env::<Bi> {
            variables: &activation,
            captures: &[],
            activation: &activation,
        };
        let origin = Sample::Rational(zero());
        let probe = SectionAt {
            section: &section,
            at: &origin,
        };
        let mut read: Vec<Poly2> = Vec::new();
        let mut records = Records::default();
        let keep = |records: &mut Records, value: Ev<_>, site: ExpressionSite| {
            if let Ev::Undefined { locus, cause } = value {
                records.keep(Record {
                    segment: segment.segment,
                    time: tau.clone(),
                    open: false,
                    state: segment.state,
                    locus: ExpressionLocus { site, term: locus },
                    cause,
                });
            }
        };
        let guard = match &self.claim.guard {
            None => Tri::True,
            Some(guard) => {
                let value = eval_in::<Bi, _>(guard, &env, &probe, &mut read)?;
                if let Some(refusal) = section.take_failure() {
                    return Err(refusal);
                }
                match value {
                    Ev::Boolean(value) => Tri::from_bool(value),
                    Ev::Number(_) => return Err(ExactGap::IllKindedTerm.into()),
                    undefined => {
                        keep(&mut records, undefined, ExpressionSite::Activation);
                        Tri::Unknown
                    }
                }
            }
        };
        let mut value = Tri::True;
        if guard != Tri::False {
            let mut valid = true;
            for (capture, exp) in self.captures.iter().enumerate() {
                section.charge()?;
                let site = ExpressionSite::Capture { capture };
                let found = eval_in::<Bi, _>(exp, &env, &probe, &mut read)?;
                if let Some(refusal) = section.take_failure() {
                    return Err(refusal);
                }
                match found {
                    Ev::Boolean(_) => {}
                    Ev::Number(number) => {
                        let range = self
                            .claim
                            .claim
                            .captures
                            .get(capture)
                            .and_then(|c| result_kind(&c.ty));
                        let mut inside = true;
                        if let Some(ResultKind::Number(range)) = range {
                            let ends = [
                                range
                                    .lower
                                    .as_ref()
                                    .map(|lower| Bi::sub(&number, &Bi::constant(lower))),
                                range
                                    .upper
                                    .as_ref()
                                    .map(|upper| Bi::sub(&Bi::constant(upper), &number)),
                            ];
                            for end in ends.into_iter().flatten() {
                                let poly = Bi::sign_poly(&end);
                                inside &= probe.sign(&poly) != Ordering::Less;
                                if let Some(refusal) = section.take_failure() {
                                    return Err(refusal);
                                }
                                read.push(poly);
                            }
                        }
                        if !inside && valid {
                            records.keep(Record {
                                segment: segment.segment,
                                time: tau.clone(),
                                open: false,
                                state: segment.state,
                                locus: ExpressionLocus {
                                    site,
                                    term: super::super::TermLocus::default(),
                                },
                                cause: UndefinedCause::OutsideCaptureRange { capture },
                            });
                        }
                        valid &= inside;
                    }
                    undefined => {
                        if valid {
                            keep(&mut records, undefined, site);
                        }
                        valid = false;
                    }
                }
            }
            if valid {
                for _ in self.segments {
                    section.charge()?;
                }
                let signal = self.claim.formula_signal_over(
                    self.timeline,
                    self.segments,
                    &mut records,
                    &mut |k| {
                        let variables: Vec<Poly2> = k.values.iter().map(in_x).collect();
                        let env = Env::<Bi> {
                            variables: &variables,
                            captures: &[],
                            activation: &activation,
                        };
                        let mut atoms = Atoms {
                            section: &section,
                            tau,
                            minimal: &minimal,
                            atoms: &self.atoms,
                            env: &env,
                            read: Vec::new(),
                            eliminated: Vec::new(),
                        };
                        let found =
                            cells_of(&mut atoms, &k.span.low, &k.span.high, k.span.high_closed)?;
                        for poly in atoms.read {
                            if !read.contains(&poly) {
                                read.push(poly);
                            }
                        }
                        Ok(found)
                    },
                )?;
                value = guard.not().or(Timeline::value(&signal, j, tau));
            }
        }
        let (bounds, held) = self.project(read, &mut |work| section.charge_by(work))?;
        Ok((
            Outcome {
                value,
                record: records.0,
            },
            bounds,
            held,
        ))
    }

    /// `record`, an expression with no value at every activation instant
    /// of the open cell `cell` (`tau` one of them), as the one earliest over
    /// the cell. At an activation instant it is the cell's start, not
    /// attained. At a claim atom it is the instant the atom has no value at:
    /// when that is the same instant for every activation instant of the
    /// cell it is the record itself; when it moves with the activation
    /// instant `tau` along `x = tau + c` it is the cell's start shifted by
    /// `c`, not attained. The instant is the root of a polynomial in
    /// `(tau, x)` that the evaluation read (or the end of a dwell, which is
    /// the same for every `tau`), and no root of another read polynomial
    /// meets it inside the cell (the projection cuts the cell where two
    /// meet), so the root curve it lies on is the whole of it.
    fn over(
        &self,
        record: Option<Record>,
        cell: &Cell,
        tau: &Rational,
        read: &[Poly2],
        segment: &ClaimSegment,
        meter: &mut HybridReplayMeter,
    ) -> Result<Option<Record>, HybridReplayRefusal> {
        let Some(mut record) = record else {
            return Ok(None);
        };
        if matches!(
            record.locus.site,
            ExpressionSite::Activation | ExpressionSite::Capture { .. }
        ) {
            record.time = cell.0.clone();
            record.open = true;
            return Ok(Some(record));
        }
        let time = record.time.clone();
        let mut ends = self
            .segments
            .iter()
            .chain(std::iter::once(segment))
            .flat_map(|k| [&k.span.low, &k.span.high]);
        if ends.any(|end| *end == time) {
            return Ok(Some(record));
        }
        for poly in read {
            meter.charge_by(poly.size().saturating_mul(time.size()))?;
            if (Instant { tau, t: &time }).sign(poly) != Ordering::Equal {
                continue;
            }
            // The instant is the same for every `tau`: `poly(tau, time)`
            // is zero as a polynomial in `tau`, that is every coefficient of
            // a power of `tau` vanishes at `time`.
            let width = poly
                .coefficients()
                .iter()
                .filter_map(|c| c.degree())
                .max()
                .map_or(0, |degree| degree + 1);
            let mut stationary = true;
            for power in 0..width {
                let part = Poly::new(
                    poly.coefficients()
                        .iter()
                        .map(|c| c.coefficients().get(power).cloned().unwrap_or_else(zero))
                        .collect(),
                );
                if time.sign_with(&part, &mut |work| meter.charge_by(work))? != Ordering::Equal {
                    stationary = false;
                    break;
                }
            }
            if stationary {
                return Ok(Some(record));
            }
            // The instant moves along `x = tau + c`, `c` rational, when
            // `poly(tau, tau + c)` is zero.
            let Some(time) = time.as_rational() else {
                continue;
            };
            let shift = time.sub(tau);
            let moving = Poly::new(vec![shift.clone(), one()]);
            let along = poly.coefficients().iter().enumerate().fold(
                Poly::zero(),
                |sum, (power, coefficient)| {
                    let mut factor = Poly::constant(one());
                    for _ in 0..power {
                        factor = factor.mul(&moving);
                    }
                    sum.add(&coefficient.mul(&factor))
                },
            );
            if along.is_zero() {
                record.time = cell.0.shifted(&shift);
                record.open = true;
                return Ok(Some(record));
            }
        }
        Err(ExactGap::ActivationInfimum.into())
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
        let mut done: Vec<(Cell, Outcome, Reading)> = Vec::new();
        let (floor, ceiling) = (floor_bound(&span.low), ceiling_bound(&span.high));
        loop {
            meter.charge()?;
            let mut results = Vec::new();
            let mut grew = false;
            for cell in partition(span, cuts.clone()) {
                if let Some((_, outcome, sample)) = done.iter().find(|(known, _, _)| *known == cell)
                {
                    results.push((cell, outcome.clone(), sample.clone()));
                    continue;
                }
                let tau: ExactReal = if cell.2 {
                    cell.0.clone()
                } else {
                    cell.0.between(&cell.1).into()
                };
                meter.charge()?;
                let (outcome, bounds, read) = match &tau {
                    ExactReal::Rational(tau) => self.at(j, segment, tau, meter)?,
                    ExactReal::Root(_) => self.at_algebraic(j, segment, &tau, meter)?,
                };
                for poly in bounds {
                    meter.charge()?;
                    for root in poly.roots_in(&floor, &ceiling) {
                        if root > span.low && root < span.high && !cuts.contains(&root) {
                            cuts.push(root);
                            grew = true;
                        }
                    }
                }
                let sample = match (&tau, cell.2) {
                    (ExactReal::Rational(tau), false) => Some((tau.clone(), read)),
                    _ => None,
                };
                done.push((cell.clone(), outcome.clone(), sample.clone()));
                results.push((cell, outcome, sample));
            }
            if !grew {
                // The cells are final: an expression with no value over an
                // open one is placed only now, for a cell cut later may
                // decide without it.
                let mut cells = Vec::with_capacity(results.len());
                for (cell, mut outcome, sample) in results {
                    if let Some((tau, read)) = sample {
                        outcome.record =
                            self.over(outcome.record, &cell, &tau, &read, segment, meter)?;
                    }
                    cells.push((cell, outcome));
                }
                return Ok(cells);
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
        for (j, segment) in segments.iter().enumerate() {
            let Some(span) = activations(segment, obligations.until.as_ref()) else {
                continue;
            };
            for (_, outcome) in decider.scan(j, segment, &span, meter)? {
                claim = claim.and(outcome.value);
                if let Some(record) = outcome.record {
                    records.keep(record);
                }
            }
        }
        Ok(claim)
    }
}
