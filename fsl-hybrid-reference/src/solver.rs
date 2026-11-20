// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1019 "Method": the flowpipe and inductive-enclosure construction.
//!
//! Data flow: the model's initial entries give start boxes (one per mode, the
//! hull of the mode's entries, since a certificate holds one initial dwell per
//! mode; each entry keeps its own hull as the root box of a refutation); a
//! start box is a dwell's first box; a dwell is a list of Picard steps (time,
//! box `X`, a-priori box `B`, `h`) until it reaches the horizon, empties or
//! closes; every transition that can fire from a dwell's `B` boxes gives one
//! jump dwell; the dwells are written as a FSpec FR-652 certificate that the
//! core's checker reads; the checker's verdict on the claim names the
//! violating step, from which `refute` builds a candidate trace. The solver
//! never certifies itself: `proved` is returned only when `check_certificate`
//! accepts the certificate it wrote.

use std::collections::{BTreeSet, VecDeque};
use std::time::{Duration, Instant};

use filament_exact::{Cancel, Integer, Rational};
use fsl_replay::{
    box_hull, box_tube, box_within, check_hybrid_certificate, read_hybrid_certificate,
    BoundDimension, HybridAnswer, HybridBox, HybridCertificateExpect, HybridCertificateOutcome,
    HybridCertificateRule, HybridModelReader, HybridProofCoverage, HybridReadStop,
    HybridReplayMeter, HybridSolveRequest, SolverCoverage, SolverHorizon, SolverMethod,
    SolverOutcome, SolverResult, SolverStop,
};
use fsl_semantics::check::{HybridActivation, HybridClock, HybridHorizon};
use serde_json::Value;

use crate::arith::{bounded, box_round, widen};
use crate::options::Settings;
use crate::wire::{self, Step};

/// The work units the reader and the solver's own certificate checks may spend.
const WORK_UNITS: u64 = 1 << 60;

/// Why a run ended without an answer of its own.
#[derive(Debug)]
pub(crate) enum Halt {
    /// A budget stopped it.
    Stop(SolverStop),
    /// It cannot decide: the answer is `solver-inconclusive`.
    Give,
}

impl From<HybridReadStop> for Halt {
    fn from(_: HybridReadStop) -> Self {
        Self::Give
    }
}

/// What bounds a flowpipe: seconds under `model-time`, positions under
/// `model-steps`.
#[derive(Clone, Debug)]
pub(crate) enum Cap {
    /// The claim's horizon in seconds.
    Time(Rational),
    /// The claim's horizon in positions.
    Steps(usize),
}

/// One dwell as built.
#[derive(Clone, Debug)]
pub(crate) struct Dwell {
    pub(crate) first: usize,
    pub(crate) last: usize,
    pub(crate) depth: usize,
    /// The start the chain began from.
    pub(crate) root: usize,
    /// The dwell and transition a jump dwell leaves.
    pub(crate) from: Option<(usize, usize)>,
}

/// The start box of one initial mode.
#[derive(Clone, Debug)]
pub(crate) struct Start {
    pub(crate) mode: usize,
    /// The hull of the mode's entries, rounded outward: the dwell's first box.
    pub(crate) bx: HybridBox,
    /// Each entry's own hull, the root box of a refutation.
    pub(crate) entries: Vec<(usize, HybridBox)>,
}

/// The dwells of a flowpipe.
pub(crate) struct Pipe {
    pub(crate) steps: Vec<Step>,
    pub(crate) dwells: Vec<Dwell>,
    pub(crate) starts: Vec<Start>,
    /// The covered duration the proof reports.
    pub(crate) horizon: Rational,
    pub(crate) jumps: usize,
}

struct Pending {
    mode: usize,
    bx: HybridBox,
    starts: (Rational, Rational),
    stored: Option<(Rational, Rational)>,
    from: Option<(usize, usize)>,
    depth: usize,
    root: usize,
}

/// One run of the solver over one request.
pub(crate) struct Run<'a, 'w> {
    pub(crate) req: &'a HybridSolveRequest<'a>,
    pub(crate) reader: HybridModelReader<'a, 'w>,
    pub(crate) set: Settings,
    steps_max: u64,
    used: u64,
    pub(crate) deadline: Option<Instant>,
    pub(crate) cancel: &'a Cancel,
}

pub(crate) fn half(value: &Rational) -> Rational {
    value.mul(&Rational::new(Integer::one(), Integer::from(2_i64)).expect("2 is nonzero"))
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

pub(crate) fn unknown() -> HybridAnswer {
    HybridAnswer::solved(SolverResult {
        method: SolverMethod::Unclassified,
        outcome: SolverOutcome::Unknown(None),
        coverage: SolverCoverage::Inductive,
    })
}

fn stopped(stop: SolverStop) -> HybridAnswer {
    HybridAnswer::solved(SolverResult {
        method: SolverMethod::Unclassified,
        outcome: SolverOutcome::Stopped(stop),
        coverage: SolverCoverage::Inductive,
    })
}

/// Solve `request`.
pub(crate) fn solve(request: &HybridSolveRequest<'_>, cancel: &Cancel) -> HybridAnswer {
    let mut meter = HybridReplayMeter::new(WORK_UNITS);
    let Ok(reader) = HybridModelReader::new(request.graph, request.model, &mut meter) else {
        return unknown();
    };
    let seconds = request.budget.time_seconds().map(Duration::from_secs);
    // A limit too large for the clock to represent is no deadline.
    let deadline = [seconds, request.deadline_ms.map(Duration::from_millis)]
        .into_iter()
        .flatten()
        .min()
        .and_then(|limit| Instant::now().checked_add(limit));
    let mut run = Run {
        req: request,
        reader,
        set: Settings::read(&request.options),
        steps_max: request.budget.steps().unwrap_or(u64::MAX),
        used: 0,
        deadline,
        cancel,
    };
    match run.decide() {
        Ok(answer) => answer,
        Err(Halt::Stop(stop)) => stopped(stop),
        Err(Halt::Give) => unknown(),
    }
}

impl<'a> Run<'a, '_> {
    /// Stop on cancellation or the wall-clock budget, charging no step.
    pub(crate) fn tick(&self) -> Result<(), Halt> {
        if self.cancel.poll() || self.deadline.is_some_and(|limit| Instant::now() >= limit) {
            return Err(Halt::Stop(SolverStop::Time));
        }
        Ok(())
    }

    /// One Picard attempt: the unit the step and time budgets count.
    pub(crate) fn attempt(&mut self) -> Result<(), Halt> {
        self.tick()?;
        if self.used >= self.steps_max {
            return Err(Halt::Stop(SolverStop::Steps(self.steps_max)));
        }
        self.used += 1;
        Ok(())
    }

    /// The a-priori box of one step from `bx` over `h` (FR-1019 "Dwells"),
    /// or `None` when eight widenings do not satisfy the Picard condition.
    /// `witness` reads the uncontracted inclusion FSpec FR-652
    /// `witness-picard` states, `uncontracted` leaves `B` unclipped by the
    /// invariant.
    pub(crate) fn picard(
        &mut self,
        mode: usize,
        bx: &HybridBox,
        h: &Rational,
        witness: bool,
        uncontracted: bool,
    ) -> Result<Option<HybridBox>, Halt> {
        let bits = self.set.bits;
        let mut grown = bx.clone();
        for _ in 0..8 {
            self.attempt()?;
            let mut cand = widen(&grown, bits);
            if !uncontracted {
                match self.reader.contract_invariant(mode, cand)? {
                    Some(clipped) => cand = clipped,
                    None => return Ok(None),
                }
            }
            if !bounded(&cand) {
                return Ok(None);
            }
            let rates = self.reader.rates(mode, &cand)?;
            let region = box_tube(bx, h, &rates);
            let (fits, next) = if witness {
                (
                    fsl_replay::box_within(&region, &cand),
                    box_hull(&cand, &region),
                )
            } else {
                match self.reader.contract_invariant(mode, region)? {
                    None => (true, cand.clone()),
                    Some(clipped) => (box_within(&clipped, &cand), box_hull(&cand, &clipped)),
                }
            };
            if fits {
                let holds = witness || self.reader.picard_holds(mode, bx, &cand, h)?;
                return Ok(holds.then_some(cand));
            }
            grown = box_round(&next, bits);
        }
        Ok(None)
    }

    /// The step length and a-priori box of the next step: the option `step`,
    /// halved at most `refine_depth` times.
    fn step_for(
        &mut self,
        mode: usize,
        bx: &HybridBox,
    ) -> Result<Option<(Rational, HybridBox)>, Halt> {
        let mut h = self.set.step.clone();
        for _ in 0..=self.set.refine_depth {
            if let Some(ap) = self.picard(mode, bx, &h, false, false)? {
                return Ok(Some((h, ap)));
            }
            h = half(&h);
        }
        Ok(None)
    }

    fn decide(&mut self) -> Result<HybridAnswer, Halt> {
        let bits = self.set.bits;
        let mut starts: Vec<Start> = Vec::new();
        for entry in 0..self.reader.initial_entries() {
            let Some(initial) = self.reader.initial_hull(entry)? else {
                continue;
            };
            if !initial.defined || !bounded(&initial.hull) {
                return Err(Halt::Give);
            }
            let hull = box_round(&initial.hull, bits);
            let mode = self.reader.initial_mode(entry);
            match starts.iter_mut().find(|start| start.mode == mode) {
                Some(start) => {
                    start.bx = box_round(&box_hull(&start.bx, &hull), bits);
                    start.entries.push((entry, initial.hull));
                }
                None => starts.push(Start {
                    mode,
                    bx: hull,
                    entries: vec![(entry, initial.hull)],
                }),
            }
        }
        if starts.is_empty() {
            return Ok(HybridAnswer::solved(SolverResult {
                method: SolverMethod::Unclassified,
                outcome: SolverOutcome::NoInitialState,
                coverage: SolverCoverage::Inductive,
            }));
        }
        let claim = self.req.claim;
        match (&claim.activation, claim.clock, &claim.horizon) {
            (HybridActivation::Each { .. }, _, _) | (_, _, None) => self.inductive(&starts),
            (HybridActivation::Origin, HybridClock::ModelTime, Some(HybridHorizon::Seconds(h))) => {
                self.bounded(&starts, Cap::Time(h.clone()))
            }
            (
                HybridActivation::Origin,
                HybridClock::ModelSteps,
                Some(HybridHorizon::Positions(count)),
            ) => match usize::try_from(*count) {
                Ok(count) => self.bounded(&starts, Cap::Steps(count)),
                Err(_) => Err(Halt::Give),
            },
            _ => Err(Halt::Give),
        }
    }

    /// Check `value` as `expect`'s form against the claim with the solver's
    /// own meter; `None` when the reader refuses it.
    pub(crate) fn check(
        &self,
        expect: HybridCertificateExpect<'_>,
        value: &Value,
        coverage: Option<&HybridProofCoverage>,
    ) -> Option<HybridCertificateOutcome> {
        let certificate = read_hybrid_certificate(self.req.model, expect, value).ok()?;
        let mut meter = HybridReplayMeter::new(WORK_UNITS);
        check_hybrid_certificate(
            self.req.graph,
            self.req.claim,
            &certificate,
            coverage,
            None,
            &mut meter,
        )
        .ok()
    }

    /// The claim rule of `flowpipe` for a sub-box of the initial states:
    /// every rule but the first box covering the initial states.
    pub(crate) fn probe(
        &self,
        value: &Value,
        coverage: &HybridProofCoverage,
    ) -> Option<HybridCertificateOutcome> {
        let certificate =
            read_hybrid_certificate(self.req.model, HybridCertificateExpect::Bounded, value).ok()?;
        let mut meter = HybridReplayMeter::new(WORK_UNITS);
        fsl_replay::probe_hybrid_flowpipe_claim(
            self.req.graph,
            self.req.claim,
            &certificate,
            coverage,
            &mut meter,
        )
        .ok()
    }

    /// The flowpipe from `starts` under `cap` (FR-1019 "Dwells", "Guards and
    /// boundary invariants" and "Bounds").
    ///
    /// Under `model-time` every dwell runs to the horizon, empties or closes.
    /// Under `model-steps` with `n` positions only dwells of depth below `n`
    /// follow transitions; they run until they empty or close on a last step
    /// that fires nothing; the horizon is then the greatest upper end of the
    /// covered instants of a step that can fire, or the greatest time
    /// reached, and the dwells of depth `n` run until they reach it.
    pub(crate) fn pipe(&mut self, starts: &[Start], cap: &Cap) -> Result<Pipe, Halt> {
        let bits = self.set.bits;
        let mut steps: Vec<Step> = Vec::new();
        let mut starts_of: Vec<(Rational, Rational)> = Vec::new();
        let mut dwells: Vec<Dwell> = Vec::new();
        let mut horizon: Option<Rational> = match cap {
            Cap::Time(h) => Some(h.clone()),
            Cap::Steps(_) => None,
        };
        let mut fire_upper: Option<Rational> = None;
        let mut reached = zero();
        let window = match cap {
            Cap::Time(h) => h.clone(),
            Cap::Steps(_) => Rational::from_integer(Integer::one().shifted_left(62)),
        };
        let mut pending: VecDeque<Pending> = starts
            .iter()
            .enumerate()
            .map(|(root, start)| Pending {
                mode: start.mode,
                bx: start.bx.clone(),
                starts: (zero(), zero()),
                stored: None,
                from: None,
                depth: 0,
                root,
            })
            .collect();
        while let Some(p) = pending.pop_front() {
            let index = dwells.len();
            let first = steps.len();
            if let Cap::Steps(n) = cap {
                if p.depth >= *n && horizon.is_none() {
                    horizon = Some(fire_upper.clone().unwrap_or_else(|| reached.clone()));
                }
            }
            let mut bx = p.bx.clone();
            let mut s = p.starts.clone();
            let mut stored = p.stored.clone();
            let closes = loop {
                let Some((h, ap)) = self.step_for(p.mode, &bx)? else {
                    return Err(Halt::Give);
                };
                let time = stored
                    .take()
                    .unwrap_or_else(|| (s.0.clone(), s.1.add(&h)));
                let next = self.reader.next_box(p.mode, &bx, &ap, &h)?;
                steps.push(Step {
                    mode: p.mode,
                    time,
                    bx: bx.clone(),
                    ap,
                    h: h.clone(),
                    from: if steps.len() == first { p.from } else { None },
                });
                starts_of.push(s.clone());
                let end = s.1.add(&h);
                if end > reached {
                    reached = end;
                }
                let reaches = match (&horizon, cap) {
                    (Some(limit), Cap::Time(_)) => s.0.add(&h) >= *limit,
                    (Some(limit), Cap::Steps(n)) => p.depth >= *n && s.0.add(&h) >= *limit,
                    (None, _) => false,
                };
                let (empties, closes) = match &next {
                    None => (true, false),
                    Some(next) => (false, box_within(next, &bx)),
                };
                if reaches || empties || closes {
                    break closes;
                }
                bx = box_round(&next.unwrap_or_default(), bits);
                s = (s.0.add(&h), s.1.add(&h));
            };
            let last = steps.len() - 1;
            dwells.push(Dwell {
                first,
                last,
                depth: p.depth,
                root: p.root,
                from: p.from,
            });
            if let Cap::Steps(n) = cap {
                if p.depth >= *n {
                    continue;
                }
            }
            for t in 0..self.reader.transitions(p.mode) {
                let mut posts: Vec<HybridBox> = Vec::new();
                let mut low: Option<Rational> = None;
                let mut high: Option<Rational> = Some(zero());
                for k in first..=last {
                    if let Cap::Time(limit) = cap {
                        if starts_of[k].0 > *limit {
                            continue;
                        }
                    }
                    let Some(fired) = self.reader.fire(p.mode, t, &steps[k].ap)? else {
                        continue;
                    };
                    posts.push(fired.post);
                    let lo = starts_of[k].0.clone();
                    low = Some(low.map_or(lo.clone(), |l| l.min(lo)));
                    let covered_end = starts_of[k].1.add(&steps[k].h);
                    if closes && k == last {
                        high = None;
                    } else {
                        high = high.map(|h0| h0.max(covered_end.clone()));
                        if matches!(cap, Cap::Steps(_)) && fire_upper.as_ref().is_none_or(|u| covered_end > *u)
                        {
                            fire_upper = Some(covered_end);
                        }
                    }
                }
                let Some(seed) = posts.first().cloned() else {
                    continue;
                };
                if matches!(cap, Cap::Steps(_)) && high.is_none() {
                    // A closing dwell whose last step can fire covers every
                    // later instant: no finite horizon shows the jumps.
                    return Err(Halt::Give);
                }
                let hull = box_round(&posts.iter().fold(seed, |acc, post| box_hull(&acc, post)), bits);
                let narrowed = self
                    .reader
                    .narrowed_time(p.mode, t, &p.bx, &p.starts, &window)?;
                let fallback = (
                    low.clone().unwrap_or_else(zero),
                    high.map_or_else(|| window.clone(), |u| u.min(window.clone())),
                );
                let time = match narrowed {
                    Some(Some(found)) => match (found.lower, found.upper) {
                        (Some(l), Some(u)) => (l, u),
                        _ => fallback,
                    },
                    _ => fallback,
                };
                pending.push_back(Pending {
                    mode: self.reader.target(p.mode, t),
                    bx: hull,
                    starts: time.clone(),
                    stored: Some(time),
                    from: Some((index, t)),
                    depth: p.depth + 1,
                    root: p.root,
                });
            }
        }
        let jumps = match cap {
            Cap::Time(_) => dwells.iter().map(|d| d.depth).max().unwrap_or(0),
            Cap::Steps(n) => *n,
        };
        let horizon = horizon.unwrap_or_else(|| fire_upper.unwrap_or(reached));
        Ok(Pipe {
            steps,
            dwells,
            starts: starts.to_vec(),
            horizon,
            jumps,
        })
    }

    /// The proof coverage a pipe reports.
    pub(crate) fn coverage(pipe: &Pipe, cap: &Cap) -> HybridProofCoverage {
        HybridProofCoverage {
            horizon: pipe.horizon.clone(),
            jumps: pipe.jumps as u64,
            exhaustive: match cap {
                Cap::Time(_) => BTreeSet::from([BoundDimension::Time]),
                Cap::Steps(_) => BTreeSet::from([BoundDimension::Jumps]),
            },
        }
    }

    fn bounded(&mut self, starts: &[Start], cap: Cap) -> Result<HybridAnswer, Halt> {
        let pipe = self.pipe(starts, &cap)?;
        let coverage = Self::coverage(&pipe, &cap);
        let value = wire::flowpipe(self.req.model, &pipe.steps);
        let Some(outcome) = self.check(HybridCertificateExpect::Bounded, &value, Some(&coverage))
        else {
            return Err(Halt::Give);
        };
        match outcome {
            HybridCertificateOutcome::Accepted(_) => {
                let Ok(horizon) = SolverHorizon::new(coverage.horizon.clone()) else {
                    return Err(Halt::Give);
                };
                Ok(HybridAnswer {
                    verdict: fsl_replay::HybridVerdict::Solved(SolverResult {
                        method: SolverMethod::FlowpipeOutwardRounded,
                        outcome: SolverOutcome::EnclosureDisjoint,
                        coverage: SolverCoverage::Bounded {
                            horizon,
                            jumps: coverage.jumps,
                            exhaustive: coverage.exhaustive,
                        },
                    }),
                    certificate: Some(value),
                })
            }
            HybridCertificateOutcome::Rejected(rejection)
                if rejection.rule == HybridCertificateRule::Claim =>
            {
                let step = rejection
                    .at
                    .strip_prefix("/certificate/steps/")
                    .and_then(|k| k.parse::<usize>().ok());
                match step {
                    Some(step) if step < pipe.steps.len() => self.refute(&pipe, &cap, step),
                    _ => Err(Halt::Give),
                }
            }
            HybridCertificateOutcome::Rejected(_) => Err(Halt::Give),
        }
    }

    /// FR-1019 "Claims with no horizon": one box per mode, the hull of the
    /// mode's step boxes, explored until a step adds nothing to its hull.
    fn inductive(&mut self, starts: &[Start]) -> Result<HybridAnswer, Halt> {
        let bits = self.set.bits;
        let modes = self.reader.modes();
        let mut boxes: Vec<Option<HybridBox>> = vec![None; modes];
        for start in starts {
            boxes[start.mode] = Some(start.bx.clone());
        }
        loop {
            let mut changed = false;
            for mode in 0..modes {
                let Some(current) = boxes[mode].clone() else {
                    continue;
                };
                let Some((h, ap)) = self.step_for(mode, &current)? else {
                    return Err(Halt::Give);
                };
                let rates = self.reader.rates(mode, &ap)?;
                let region = box_tube(&current, &h, &rates);
                if let Some(clipped) = self.reader.contract_invariant(mode, region)? {
                    let grown = box_round(&box_hull(&current, &clipped), bits);
                    if grown != current {
                        boxes[mode] = Some(grown);
                        changed = true;
                    }
                }
                let current = boxes[mode].clone().unwrap_or_default();
                for t in 0..self.reader.transitions(mode) {
                    let Some(fired) = self.reader.fire(mode, t, &current)? else {
                        continue;
                    };
                    let target = self.reader.target(mode, t);
                    let post = box_round(&fired.post, bits);
                    let merged = match &boxes[target] {
                        None => post,
                        Some(known) => box_round(&box_hull(known, &post), bits),
                    };
                    if boxes[target].as_ref() != Some(&merged) {
                        boxes[target] = Some(merged);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let mut all = Vec::with_capacity(modes);
        for (mode, known) in boxes.into_iter().enumerate() {
            let bx = match known {
                Some(bx) => bx,
                None => {
                    let whole = vec![fsl_replay::HybridInterval::whole(); self.reader.variables()];
                    match self.reader.contract_invariant(mode, whole)? {
                        Some(hull) if bounded(&hull) => box_round(&hull, bits),
                        _ => return Err(Halt::Give),
                    }
                }
            };
            all.push(bx);
        }
        let value = wire::inductive(self.req.model, &all);
        match self.check(HybridCertificateExpect::Inductive, &value, None) {
            Some(HybridCertificateOutcome::Accepted(_)) => Ok(HybridAnswer {
                verdict: fsl_replay::HybridVerdict::Solved(SolverResult {
                    method: SolverMethod::InductiveEnclosure,
                    outcome: SolverOutcome::EnclosureDisjoint,
                    coverage: SolverCoverage::Inductive,
                }),
                certificate: Some(value),
            }),
            _ => Err(Halt::Give),
        }
    }
}
