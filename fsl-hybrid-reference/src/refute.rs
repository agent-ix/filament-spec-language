// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1019 "Refutations": candidate point traces from the violating dwell's
//! root box. A trace refutes only through FR-450 exact replay (the core
//! settles it again) or a FSpec FR-652 validated witness the solver's own
//! check accepted; an unknown reading never refutes.

use filament_exact::{Integer, Rational};
use fsl_replay::{
    exact_replay, check_hybrid_certificate, read_hybrid_certificate, ExactCheck, ExactInterval,
    ExactReplay, HybridAnswer, HybridBox, HybridCertificateExpect, HybridCertificateOutcome,
    HybridCertificateAcceptance, HybridCertificateRule, HybridCounterexample, HybridReplayMeter,
    HybridTrace, HybridVerdict, TraceBefore, TraceState,
};
use fsl_semantics::check::ModeIndex;
use serde_json::Value;

use crate::arith::{bounded, box_round, corner, grid_count, grid_point, mid, midpoint};
use crate::solver::{half, Halt, Pipe, Run};
use crate::wire::{self, Step};

const WORK_UNITS: u64 = 1 << 60;

enum Class {
    Reproduced(Box<HybridCounterexample>),
    Early,
    Far,
}

struct Found {
    counterexample: HybridCounterexample,
    certificate: Option<Value>,
}

fn point_box(point: &[Rational]) -> HybridBox {
    point.iter().cloned().map(fsl_replay::HybridInterval::point).collect()
}

fn exact(point: &[Rational]) -> Vec<ExactInterval> {
    point.iter().cloned().map(ExactInterval::point).collect()
}

fn evaluate(coefficients: &[Rational], at: &Rational) -> Rational {
    coefficients
        .iter()
        .rev()
        .fold(Rational::from_integer(Integer::zero()), |acc, c| acc.mul(at).add(c))
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

impl Run<'_, '_> {
    /// The candidate trace of the violating dwell's chain, or `Give`.
    pub(crate) fn refute(
        &mut self,
        pipe: &Pipe,
        root: usize,
        horizon: &Rational,
    ) -> Result<HybridAnswer, Halt> {
        let (mode, bx) = pipe.roots[root].clone();
        let depth = self.set.refine_depth.min(3);
        match self.search(mode, &bx, depth, horizon)? {
            Some(found) => Ok(HybridAnswer {
                verdict: HybridVerdict::Refuted(Box::new(found.counterexample)),
                certificate: found.certificate,
            }),
            None => Err(Halt::Give),
        }
    }

    /// Candidate points of `bx`, then its halves, depth first.
    fn search(
        &mut self,
        mode: usize,
        bx: &HybridBox,
        depth: u32,
        horizon: &Rational,
    ) -> Result<Option<Found>, Halt> {
        if !bounded(bx) {
            return Ok(None);
        }
        let mut points: Vec<Vec<Rational>> = Vec::new();
        for point in [corner(bx, false), midpoint(bx), corner(bx, true)]
            .into_iter()
            .flatten()
        {
            if !points.contains(&point) {
                points.push(point);
            }
        }
        for point in points {
            if let Some(found) = self.try_point(mode, &point, horizon)? {
                return Ok(Some(found));
            }
        }
        if depth == 0 {
            return Ok(None);
        }
        let mut widest: Option<(usize, Rational)> = None;
        for (i, x) in bx.iter().enumerate() {
            if let (Some(l), Some(u)) = (&x.lower, &x.upper) {
                let width = u.sub(l);
                if widest.as_ref().is_none_or(|(_, w)| width > *w) {
                    widest = Some((i, width));
                }
            }
        }
        let Some((i, width)) = widest else {
            return Ok(None);
        };
        if width.is_zero() {
            return Ok(None);
        }
        let Some(lower) = bx[i].lower.clone() else {
            return Ok(None);
        };
        let middle = lower.add(&half(&width));
        let mut low = bx.clone();
        low[i].upper = Some(middle.clone());
        let mut high = bx.clone();
        high[i].lower = Some(middle);
        for part in [low, high] {
            if let Some(found) = self.search(mode, &part, depth - 1, horizon)? {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    fn try_point(
        &mut self,
        mode: usize,
        point: &[Rational],
        horizon: &Rational,
    ) -> Result<Option<Found>, Halt> {
        self.tick()?;
        let mut admitted = false;
        for entry in 0..self.reader.initial_entries() {
            if self.reader.initial_mode(entry) == mode
                && self.reader.satisfies_initial(entry, point)?
            {
                admitted = true;
                break;
            }
        }
        if !admitted {
            return Ok(None);
        }
        match self.reader.polynomial_solution(mode, point) {
            Some(solution) => self.by_replay(mode, point, &solution, horizon),
            None => self.by_witness(mode, point, horizon),
        }
    }

    fn trace(mode: usize, point: &[Rational], end: Option<TraceBefore>) -> HybridTrace {
        HybridTrace {
            start: TraceState {
                mode: ModeIndex::new(mode),
                time: ExactInterval::point(zero()),
                enclosure: exact(point),
            },
            jumps: Vec::new(),
            delta: None,
            loop_to: None,
            end,
        }
    }

    fn classify(&self, trace: &HybridTrace) -> Class {
        let mut meter = HybridReplayMeter::new(WORK_UNITS);
        match exact_replay(self.req.graph, self.req.claim, trace, &mut meter) {
            Ok(ExactReplay::Reproduced(found)) => Class::Reproduced(found),
            Ok(ExactReplay::NotReproduced {
                check: ExactCheck::Claim,
                ..
            }) => Class::Early,
            _ => Class::Far,
        }
    }

    /// A mode with a polynomial flow: the exact solution is a polynomial in
    /// time. The start-only trace first, then the least multiple of
    /// `2^-precision_bits` at which the prefix is a bad prefix, each decided
    /// by exact replay.
    fn by_replay(
        &mut self,
        mode: usize,
        point: &[Rational],
        solution: &[Vec<Rational>],
        horizon: &Rational,
    ) -> Result<Option<Found>, Halt> {
        if let Class::Reproduced(found) = self.classify(&Self::trace(mode, point, None)) {
            return Ok(Some(Found {
                counterexample: *found,
                certificate: None,
            }));
        }
        let bits = self.set.bits;
        let (mut low, mut high) = (Integer::zero(), grid_count(horizon, bits));
        let mut best: Option<HybridCounterexample> = None;
        while low <= high {
            self.tick()?;
            let n = mid(&low, &high);
            let tau = grid_point(&n, bits);
            let state: Vec<Rational> = solution.iter().map(|c| evaluate(c, &tau)).collect();
            let end = TraceBefore {
                time: ExactInterval::point(tau),
                enclosure: exact(&state),
            };
            match self.classify(&Self::trace(mode, point, Some(end))) {
                Class::Reproduced(found) => {
                    best = Some(*found);
                    if n.is_zero() {
                        break;
                    }
                    high = n.sub(&Integer::one());
                }
                Class::Early => low = n.add(&Integer::one()),
                Class::Far => {
                    if n.is_zero() {
                        break;
                    }
                    high = n.sub(&Integer::one());
                }
            }
        }
        Ok(best.map(|counterexample| Found {
            counterexample,
            certificate: None,
        }))
    }

    /// A mode whose rates depend on one another cyclically: Picard steps
    /// from the point until a step has a decisive box FSpec FR-652's
    /// `validated-witness` check accepts.
    fn by_witness(
        &mut self,
        mode: usize,
        point: &[Rational],
        horizon: &Rational,
    ) -> Result<Option<Found>, Halt> {
        let bits = self.set.bits;
        let mut bx = point_box(point);
        let mut now = zero();
        let mut steps: Vec<Step> = Vec::new();
        let bare = Self::trace(mode, point, None);
        while now <= *horizon {
            let mut chosen: Option<(Rational, HybridBox, bool)> = None;
            let mut h = self.set.step.clone();
            'shrink: for _ in 0..=self.set.refine_depth {
                for uncontracted in [false, true] {
                    if let Some(ap) = self.picard(mode, &bx, &h, true, uncontracted)? {
                        chosen = Some((h.clone(), ap, uncontracted));
                        break 'shrink;
                    }
                }
                h = half(&h);
            }
            let Some((h, ap, last)) = chosen else {
                return Ok(None);
            };
            let next_time = now.add(&h);
            steps.push(Step {
                mode,
                time: (now.clone(), next_time.clone()),
                bx: bx.clone(),
                ap: ap.clone(),
                h: h.clone(),
                from: None,
            });
            let value = wire::witness(self.req.model, std::slice::from_ref(&steps));
            let outcome = self.check_witness(&bare, &value);
            match outcome {
                Some(HybridCertificateOutcome::Accepted(HybridCertificateAcceptance::Witness(
                    decisive,
                ))) => {
                    let end = TraceBefore {
                        time: decisive.time.clone(),
                        enclosure: decisive.enclosure.clone(),
                    };
                    let trace = Self::trace(mode, point, Some(end));
                    return Ok(match self.check_witness(&trace, &value) {
                        Some(HybridCertificateOutcome::Accepted(_)) => Some(Found {
                            counterexample: HybridCounterexample {
                                kind: decisive.kind,
                                trace,
                            },
                            certificate: Some(value),
                        }),
                        _ => None,
                    });
                }
                Some(HybridCertificateOutcome::Rejected(rejection))
                    if rejection.rule == HybridCertificateRule::WitnessViolation && !last => {}
                _ => return Ok(None),
            }
            let rates = self.reader.rates(mode, &ap)?;
            bx = box_round(&fsl_replay::box_advance(&bx, &h, &rates), bits);
            now = next_time;
        }
        Ok(None)
    }

    fn check_witness(&self, trace: &HybridTrace, value: &Value) -> Option<HybridCertificateOutcome> {
        let certificate = read_hybrid_certificate(
            self.req.model,
            HybridCertificateExpect::Witness(trace),
            value,
        )
        .ok()?;
        let mut meter = HybridReplayMeter::new(WORK_UNITS);
        check_hybrid_certificate(
            self.req.graph,
            self.req.claim,
            &certificate,
            None,
            Some(trace),
            &mut meter,
        )
        .ok()
    }
}
