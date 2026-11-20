// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1019 "Refutations": candidate traces from the violating dwell's root
//! box. A trace refutes only through FR-450 exact replay (the core settles
//! it again) or a FSpec FR-652 validated witness the solver's own check
//! accepted; an unknown reading never refutes. A candidate crosses jumps: it
//! leaves a mode at the least rational instant a guard holds, applies the
//! reset exactly and continues in the target mode.

use filament_exact::{Integer, Rational};
use fsl_replay::{
    check_hybrid_certificate, exact_replay, read_hybrid_certificate, ExactCheck, ExactInterval,
    ExactReplay, HybridAnswer, HybridBox, HybridCertificateAcceptance, HybridCertificateExpect,
    HybridCertificateOutcome, HybridCertificateRule, HybridCounterexample, HybridReplayMeter,
    HybridTrace, HybridTri, HybridVerdict, SolverCoverage, SolverMethod, SolverOutcome,
    SolverResult, TraceBefore, TraceJump, TraceState,
};
use fsl_semantics::check::ModeIndex;
use serde_json::Value;

use crate::arith::{bounded, box_round, corner, grid_count, grid_point, mid, midpoint};
use crate::solver::{half, Cap, Halt, Pipe, Run, Start};
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

/// What a refutation searches within.
struct Ctx {
    cap: Cap,
    /// The covered time: candidate instants lie at or before it.
    horizon: Rational,
    /// The most jumps a candidate may take.
    jumps: usize,
}

/// The earliest instant a transition of a dwell can be taken.
enum Next {
    /// None within the horizon.
    Never,
    /// At `at`, over transition `transition`.
    Take { at: Rational, transition: usize },
    /// A guard holds from an instant that is not a rational: no candidate.
    Irrational,
}

fn point_box(point: &[Rational]) -> HybridBox {
    point
        .iter()
        .cloned()
        .map(fsl_replay::HybridInterval::point)
        .collect()
}

fn exact(point: &[Rational]) -> Vec<ExactInterval> {
    point.iter().cloned().map(ExactInterval::point).collect()
}

fn exact_box(bx: &HybridBox) -> Vec<ExactInterval> {
    bx.iter()
        .map(|x| {
            ExactInterval::new(
                x.lower.clone().unwrap_or_else(zero),
                x.upper.clone().unwrap_or_else(zero),
            )
        })
        .collect()
}

fn evaluate(coefficients: &[Rational], at: &Rational) -> Rational {
    coefficients
        .iter()
        .rev()
        .fold(zero(), |acc, c| acc.mul(at).add(c))
}

fn state_at(solution: &[Vec<Rational>], at: &Rational) -> Vec<Rational> {
    solution.iter().map(|c| evaluate(c, at)).collect()
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

fn trace_of(
    mode: usize,
    point: &[Rational],
    jumps: Vec<TraceJump>,
    end: Option<TraceBefore>,
) -> HybridTrace {
    HybridTrace {
        start: TraceState {
            mode: ModeIndex::new(mode),
            time: ExactInterval::point(zero()),
            enclosure: exact(point),
        },
        jumps,
        delta: None,
        loop_to: None,
        end,
    }
}

fn jump_to(
    transition: usize,
    target: usize,
    at: &Rational,
    before: Vec<ExactInterval>,
    after: Vec<ExactInterval>,
) -> TraceJump {
    TraceJump {
        transition: transition as u64,
        before: Some(TraceBefore {
            time: ExactInterval::point(at.clone()),
            enclosure: before,
        }),
        state: TraceState {
            mode: ModeIndex::new(target),
            time: ExactInterval::point(at.clone()),
            enclosure: after,
        },
    }
}

impl Run<'_, '_> {
    /// The answer for a flowpipe whose claim check rejected at step `step`:
    /// a refutation from the chain's root box, else the chain as an
    /// inconclusive trace.
    pub(crate) fn refute(
        &mut self,
        pipe: &Pipe,
        cap: &Cap,
        step: usize,
    ) -> Result<HybridAnswer, Halt> {
        let Some(violating) = pipe
            .dwells
            .iter()
            .position(|dwell| dwell.first <= step && step <= dwell.last)
        else {
            return Err(Halt::Give);
        };
        let ctx = Ctx {
            cap: cap.clone(),
            horizon: pipe.horizon.clone(),
            jumps: pipe.jumps,
        };
        let root = &pipe.starts[pipe.dwells[violating].root];
        for (entry, bx) in &root.entries {
            if let Some(found) =
                self.search(*entry, root.mode, bx, self.set.refine_depth, &ctx)?
            {
                return Ok(HybridAnswer {
                    verdict: HybridVerdict::Refuted(Box::new(found.counterexample)),
                    certificate: found.certificate,
                });
            }
        }
        Ok(Self::chain_trace(pipe, violating, step, root))
    }

    /// FR-1019 "Refutations" step 5: the root box, the chain's jump steps
    /// and an `end` at the violating step.
    fn chain_trace(pipe: &Pipe, violating: usize, step: usize, root: &Start) -> HybridAnswer {
        let mut chain = vec![violating];
        while let Some((from, _)) = pipe.dwells[*chain.last().unwrap_or(&violating)].from {
            chain.push(from);
        }
        chain.reverse();
        let interval = |time: &(Rational, Rational)| ExactInterval::new(time.0.clone(), time.1.clone());
        let jumps = chain
            .iter()
            .skip(1)
            .filter_map(|&d| {
                let first = &pipe.steps[pipe.dwells[d].first];
                let (_, transition) = pipe.dwells[d].from?;
                Some(TraceJump {
                    transition: transition as u64,
                    before: None,
                    state: TraceState {
                        mode: ModeIndex::new(first.mode),
                        time: interval(&first.time),
                        enclosure: exact_box(&first.bx),
                    },
                })
            })
            .collect();
        let at = &pipe.steps[step];
        let trace = HybridTrace {
            start: TraceState {
                mode: ModeIndex::new(root.mode),
                time: ExactInterval::point(zero()),
                enclosure: exact_box(&root.bx),
            },
            jumps,
            delta: None,
            loop_to: None,
            end: Some(TraceBefore {
                time: interval(&at.time),
                enclosure: exact_box(&at.ap),
            }),
        };
        HybridAnswer::solved(SolverResult {
            method: SolverMethod::Unclassified,
            outcome: SolverOutcome::Unknown(Some(Box::new(trace))),
            coverage: SolverCoverage::Inductive,
        })
    }

    /// Whether the flowpipe of `bx` alone still gets a claim rejection.
    fn still_rejects(&mut self, mode: usize, bx: &HybridBox, cap: &Cap) -> Result<bool, Halt> {
        let start = Start {
            mode,
            bx: box_round(bx, self.set.bits),
            entries: Vec::new(),
        };
        let pipe = match self.pipe(std::slice::from_ref(&start), cap) {
            Ok(pipe) => pipe,
            Err(Halt::Give) => return Ok(false),
            Err(stop) => return Err(stop),
        };
        let coverage = Self::coverage(&pipe, cap);
        let value = wire::flowpipe(self.req.model, &pipe.steps);
        Ok(self.probe(&value, &coverage) == Some(fsl_replay::HybridProbeOutcome::ClaimRejected))
    }

    /// Candidate points of `bx`, then the halves whose flowpipe still gets a
    /// claim rejection, to `depth` levels.
    fn search(
        &mut self,
        entry: usize,
        mode: usize,
        bx: &HybridBox,
        depth: u32,
        ctx: &Ctx,
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
            if let Some(found) = self.try_point(entry, mode, &point, ctx)? {
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
            if self.still_rejects(mode, &part, &ctx.cap)? {
                if let Some(found) = self.search(entry, mode, &part, depth - 1, ctx)? {
                    return Ok(Some(found));
                }
            }
        }
        Ok(None)
    }

    /// One candidate point, tested only against the entry that owns the box.
    fn try_point(
        &mut self,
        entry: usize,
        mode: usize,
        point: &[Rational],
        ctx: &Ctx,
    ) -> Result<Option<Found>, Halt> {
        self.tick()?;
        if !self.reader.satisfies_initial(entry, point)? {
            return Ok(None);
        }
        match self.reader.polynomial_solution(mode, point) {
            Some(_) => self.by_replay(mode, point, ctx),
            None => self.by_witness(mode, point, ctx),
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

    /// The earliest rational instant, at or after `now` and no later than the
    /// horizon, at which some transition of `mode` can be taken along
    /// `solution`. The guard's truth changes only at the zeros of its
    /// comparisons, so the instants tested are those, the points between
    /// them and one point after the last.
    fn next_jump(
        &mut self,
        mode: usize,
        solution: &[Vec<Rational>],
        now: &Rational,
        horizon: &Rational,
    ) -> Result<Next, Halt> {
        let mut best: Option<(Rational, usize)> = None;
        let mut doubtful: Option<Rational> = None;
        for t in 0..self.reader.transitions(mode) {
            let Some(mut instants) = self.reader.guard_instants(mode, t, solution) else {
                return Ok(Next::Irrational);
            };
            if instants.first().is_none_or(|first| !first.is_zero()) {
                instants.insert(0, zero());
            }
            let two = Rational::from_integer(Integer::from(2_i64));
            let mut tests: Vec<(Rational, bool)> = Vec::new();
            for (i, c) in instants.iter().enumerate() {
                tests.push((c.clone(), true));
                let after = match instants.get(i + 1) {
                    Some(next) => c.add(next).div(&two).unwrap_or_else(|| c.clone()),
                    None => c.add(&Rational::from_integer(Integer::one())),
                };
                tests.push((after, false));
            }
            for (local, exact_instant) in tests {
                let at = now.add(&local);
                if at > *horizon {
                    break;
                }
                let pre = point_box(&state_at(solution, &local));
                if self.reader.fire(mode, t, &pre)?.is_none() {
                    continue;
                }
                if exact_instant {
                    if best.as_ref().is_none_or(|(b, _)| at < *b) {
                        best = Some((at, t));
                    }
                } else if doubtful.as_ref().is_none_or(|d| at < *d) {
                    doubtful = Some(at);
                }
                break;
            }
        }
        Ok(match (best, doubtful) {
            (Some((at, _)), Some(open)) if open < at => Next::Irrational,
            (None, Some(_)) => Next::Irrational,
            (Some((at, transition)), _) => Next::Take { at, transition },
            (None, None) => Next::Never,
        })
    }

    /// Every dwell polynomial in time: the start-only prefix, then the least
    /// multiple of `2^-precision_bits` at which the prefix is a bad prefix,
    /// each decided by exact replay; when none is, the dwell ends at the
    /// least instant a guard holds, the reset is applied exactly, and the
    /// search continues in the target mode.
    fn by_replay(
        &mut self,
        mode: usize,
        point: &[Rational],
        ctx: &Ctx,
    ) -> Result<Option<Found>, Halt> {
        let bits = self.set.bits;
        let mut current = mode;
        let mut state = point.to_vec();
        let mut now = zero();
        let mut jumps: Vec<TraceJump> = Vec::new();
        loop {
            let Some(solution) = self.reader.polynomial_solution(current, &state) else {
                return Ok(None);
            };
            let next = if jumps.len() < ctx.jumps {
                self.next_jump(current, &solution, &now, &ctx.horizon)?
            } else {
                Next::Never
            };
            let limit = match &next {
                Next::Take { at, .. } => at.clone(),
                Next::Never => ctx.horizon.clone(),
                Next::Irrational => return Ok(None),
            };
            if let Class::Reproduced(found) =
                self.classify(&trace_of(mode, point, jumps.clone(), None))
            {
                return Ok(Some(Found {
                    counterexample: *found,
                    certificate: None,
                }));
            }
            let mut low = grid_count(&now, bits);
            let mut high = grid_count(&limit, bits);
            if grid_point(&high, bits) > limit {
                if high.is_zero() {
                    high = Integer::from(-1_i64);
                } else {
                    high = high.sub(&Integer::one());
                }
            }
            let mut best: Option<HybridCounterexample> = None;
            while low <= high {
                self.tick()?;
                let n = mid(&low, &high);
                let tau = grid_point(&n, bits);
                let end = TraceBefore {
                    time: ExactInterval::point(tau.clone()),
                    enclosure: exact(&state_at(&solution, &tau.sub(&now))),
                };
                match self.classify(&trace_of(mode, point, jumps.clone(), Some(end))) {
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
            if best.is_none() {
                let end = TraceBefore {
                    time: ExactInterval::point(limit.clone()),
                    enclosure: exact(&state_at(&solution, &limit.sub(&now))),
                };
                if let Class::Reproduced(found) =
                    self.classify(&trace_of(mode, point, jumps.clone(), Some(end)))
                {
                    best = Some(*found);
                }
            }
            if let Some(counterexample) = best {
                return Ok(Some(Found {
                    counterexample,
                    certificate: None,
                }));
            }
            let Next::Take { at, transition } = next else {
                return Ok(None);
            };
            let pre = state_at(&solution, &at.sub(&now));
            let Some(post) = self.reader.reset_point(current, transition, &pre)? else {
                return Ok(None);
            };
            let target = self.reader.target(current, transition);
            jumps.push(jump_to(transition, target, &at, exact(&pre), exact(&post)));
            current = target;
            state = post;
            now = at;
        }
    }

    /// A flow whose rates depend on one another cyclically: Picard steps
    /// from the point until a step has a decisive box FSpec FR-652's
    /// `validated-witness` check accepts. First within the start's dwell,
    /// then following jumps whose guard is definitely true over the box
    /// reached.
    fn by_witness(
        &mut self,
        mode: usize,
        point: &[Rational],
        ctx: &Ctx,
    ) -> Result<Option<Found>, Halt> {
        for allowed in [0, ctx.jumps] {
            if allowed == 0 || ctx.jumps > 0 {
                if let Some(found) = self.witness_run(mode, point, ctx, allowed)? {
                    return Ok(Some(found));
                }
            }
        }
        Ok(None)
    }

    fn witness_run(
        &mut self,
        mode: usize,
        point: &[Rational],
        ctx: &Ctx,
        allowed: usize,
    ) -> Result<Option<Found>, Halt> {
        let bits = self.set.bits;
        let mut current = mode;
        let mut bx = point_box(point);
        let mut now = zero();
        let mut dwells: Vec<Vec<Step>> = vec![Vec::new()];
        let mut jumps: Vec<TraceJump> = Vec::new();
        while now <= ctx.horizon {
            let mut chosen: Option<(Rational, HybridBox, bool)> = None;
            let mut h = self.set.step.clone();
            'shrink: for _ in 0..=self.set.refine_depth {
                for uncontracted in [false, true] {
                    if let Some(ap) = self.picard(current, &bx, &h, true, uncontracted)? {
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
            if let Some(open) = dwells.last_mut() {
                open.push(Step {
                    mode: current,
                    time: (now.clone(), next_time.clone()),
                    bx: bx.clone(),
                    ap: ap.clone(),
                    h: h.clone(),
                    from: None,
                });
            }
            let value = wire::witness(self.req.model, &dwells);
            let bare = trace_of(mode, point, jumps.clone(), None);
            match self.check_witness(&bare, &value) {
                Some(HybridCertificateOutcome::Accepted(HybridCertificateAcceptance::Witness(
                    decisive,
                ))) => {
                    let end = TraceBefore {
                        time: decisive.time.clone(),
                        enclosure: decisive.enclosure.clone(),
                    };
                    let trace = trace_of(mode, point, jumps, Some(end));
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
            let rates = self.reader.rates(current, &ap)?;
            let advanced = box_round(&fsl_replay::box_advance(&bx, &h, &rates), bits);
            let mut taken = false;
            if jumps.len() < allowed {
                for t in 0..self.reader.transitions(current) {
                    if !self.reader.guard_defined(current, t, &advanced)?
                        || self.reader.guard_reading(current, t, &advanced)? != HybridTri::True
                        || !self.reader.resets_defined(current, t, &advanced)?
                    {
                        continue;
                    }
                    let Some(after) = self.reader.reset_box(current, t, &advanced)? else {
                        continue;
                    };
                    let after = box_round(&after, bits);
                    let target = self.reader.target(current, t);
                    if let Some(open) = dwells.last_mut() {
                        open.push(Step {
                            mode: current,
                            time: (next_time.clone(), next_time.clone()),
                            bx: advanced.clone(),
                            ap: advanced.clone(),
                            h: zero(),
                            from: None,
                        });
                    }
                    jumps.push(jump_to(
                        t,
                        target,
                        &next_time,
                        exact_box(&advanced),
                        exact_box(&after),
                    ));
                    dwells.push(Vec::new());
                    current = target;
                    bx = after;
                    taken = true;
                    break;
                }
            }
            if !taken {
                bx = advanced;
            }
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
