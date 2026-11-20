// SPDX-License-Identifier: AGPL-3.0-or-later
//! The rules of `validated-witness` (FSpec FR-652 "Rules of
//! `validated-witness`"): `box-shape`, `witness-start`, `witness-picard`,
//! `witness-unique`, `witness-guard`, `witness-violation`, and the decisive
//! reading an accepted witness returns.

use filament_exact::{Integer, Rational};
use fsl_semantics::check::{
    CheckedHybridClaim, CheckedHybridModel, CompareOp, HybridClock, HybridHorizon,
};

use super::super::interval::{compare, Interval, Tri};
use super::super::real::ExactReal;
use super::super::signal::Span;
use super::super::trace::zero;
use super::super::{
    ExactInterval, ExpressionLocus, ExpressionSite, HybridCounterexampleKind, HybridTrace,
    UndefinedInstant,
};
use super::claim::{span, CapStatus, ClaimEval, Reading};
use super::machine::{advance, tube, within, Bx, Def, Machine, ModelX, Undefined};
use super::wire::WireStep;
use super::{names, reject, shaped, At, Decisive, Fail, Rule};

struct Ws {
    mode: usize,
    h: Rational,
    time: (Rational, Rational),
    bx: Bx,
    ap: Bx,
}

fn pointer(d: usize, k: usize) -> String {
    format!("/certificate/dwells/{d}/{k}")
}

fn iv(interval: &ExactInterval) -> Interval {
    Interval {
        lower: Some(interval.lower.clone()),
        upper: Some(interval.upper.clone()),
    }
}

fn exact(bx: &Bx) -> Vec<ExactInterval> {
    bx.iter()
        .map(|x| {
            ExactInterval::new(
                x.lower.clone().unwrap_or_else(zero),
                x.upper.clone().unwrap_or_else(zero),
            )
        })
        .collect()
}

fn natural(value: usize) -> Rational {
    Rational::from_integer(Integer::from(i64::try_from(value).unwrap_or(i64::MAX)))
}

/// One reading of a witness, with where it is.
struct Meta {
    dwell: usize,
    step: usize,
    /// A `box` reading at one instant; otherwise an a-priori box reading.
    at_box: bool,
    time: (Rational, Rational),
    state: Bx,
}

/// An activation of the claim: the reading it is at (`None` for the origin),
/// its captures and its atom readings over every reading.
struct Activation {
    at: Option<usize>,
    status: CapStatus,
    vals: Vec<Vec<(Tri, Def)>>,
}

pub(super) fn check(
    machine: &mut Machine<'_, '_>,
    mx: &ModelX,
    model: &CheckedHybridModel,
    claim: &CheckedHybridClaim,
    dwells: &[Vec<WireStep>],
    trace: &HybridTrace,
) -> Result<Decisive, Fail> {
    let variable_names = names(model);
    let n = mx.n;
    // 1: box-shape
    let mut ws: Vec<Vec<Ws>> = Vec::with_capacity(dwells.len());
    for (d, steps) in dwells.iter().enumerate() {
        let mut list = Vec::with_capacity(steps.len());
        for (k, step) in steps.iter().enumerate() {
            let Some(bx) = shaped(&step.bx, &variable_names) else {
                return reject(Rule::BoxShape, format!("{}/box", pointer(d, k)));
            };
            let Some(ap) = shaped(&step.apriori, &variable_names) else {
                return reject(Rule::BoxShape, format!("{}/apriori", pointer(d, k)));
            };
            list.push(Ws {
                mode: step.mode,
                h: step.step.clone(),
                time: (step.time.lower.clone(), step.time.upper.clone()),
                bx,
                ap,
            });
        }
        ws.push(list);
    }
    let last_dwell = ws.len() - 1;
    let last_step = |d: usize| ws[d].len() - 1;
    let end_pointer = pointer(last_dwell, last_step(last_dwell));

    // 2: witness-start
    let start_at = pointer(0, 0);
    let start = &trace.start;
    let point: Option<Bx> = start
        .enclosure
        .iter()
        .map(|x| x.as_point().map(|q| Interval::point(q.clone())))
        .collect();
    let started = start.time.lower.is_zero() && start.time.upper.is_zero();
    let Some(p) = point.filter(|_| started) else {
        return reject(Rule::WitnessStart, start_at);
    };
    let mut satisfied = false;
    for (mode, condition) in &mx.initial {
        if *mode != start.mode.get() {
            continue;
        }
        let invariant = &mx.modes[*mode].invariant;
        let (c, dc) = machine.reading(condition, &p).at(&start_at)?;
        let (i, di) = machine.reading(invariant, &p).at(&start_at)?;
        if c == Tri::True && i == Tri::True && dc.is_yes() && di.is_yes() {
            satisfied = true;
            break;
        }
    }
    if !satisfied || !ws[0][0].time.0.is_zero() || !within(&p, &ws[0][0].bx) {
        return reject(Rule::WitnessStart, start_at);
    }

    // 3: witness-picard
    for d in 0..ws.len() {
        for k in 0..ws[d].len() {
            let at = pointer(d, k);
            let step = &ws[d][k];
            let mode = &mx.modes[step.mode];
            let start_instant = if k > 0 {
                ws[d][k - 1].time.1.clone()
            } else if d > 0 {
                ws[d - 1][last_step(d - 1)].time.1.clone()
            } else {
                zero()
            };
            if step.time != (start_instant.clone(), start_instant.add(&step.h)) {
                return reject(Rule::WitnessPicard, at);
            }
            for exp in mode.rates.iter().chain(std::iter::once(&mode.invariant)) {
                if !machine.defined(exp, &step.ap).at(&at)?.is_yes() {
                    return reject(Rule::WitnessPicard, at);
                }
            }
            let rates = machine.rates(mode, &step.ap).at(&at)?;
            if !within(&tube(&step.bx, &step.h, &rates), &step.ap) {
                return reject(Rule::WitnessPicard, at);
            }
            if machine.tri(&mode.invariant, &step.bx).at(&at)? != Tri::True {
                return reject(Rule::WitnessPicard, at);
            }
            let last = d == last_dwell && k == last_step(d);
            if !last && machine.tri(&mode.invariant, &step.ap).at(&at)? != Tri::True {
                return reject(Rule::WitnessPicard, at);
            }
            if k > 0 {
                let before = &ws[d][k - 1];
                let before_rates = machine.rates(&mx.modes[before.mode], &before.ap).at(&at)?;
                if !within(&advance(&before.bx, &before.h, &before_rates), &step.bx) {
                    return reject(Rule::WitnessPicard, at);
                }
            }
        }
    }

    // 4: witness-unique
    for (d, list) in ws.iter().enumerate() {
        for (k, step) in list.iter().enumerate() {
            let at = pointer(d, k);
            for rate in &mx.modes[step.mode].rates {
                if !machine.strictly_decided(rate, &step.ap, true).at(&at)? {
                    return reject(Rule::WitnessUnique, at);
                }
            }
        }
    }

    // 5: witness-guard
    for d in 0..last_dwell {
        let k = last_step(d);
        let at = pointer(d, k);
        let jump = &ws[d][k];
        if !jump.h.is_zero() || jump.time.0 != jump.time.1 {
            return reject(Rule::WitnessGuard, at);
        }
        let tau = jump.time.0.clone();
        let mode = &mx.modes[jump.mode];
        let transition_position = trace.jumps[d].transition;
        let Some(transition) = usize::try_from(transition_position)
            .ok()
            .and_then(|t| mode.transitions.get(t))
        else {
            return reject(Rule::WitnessGuard, at);
        };
        let x = &jump.bx;
        let y = &ws[d + 1][0].bx;
        let target = &mx.modes[transition.target];
        if !machine.defined(&transition.guard, x).at(&at)?.is_yes()
            || machine.tri(&transition.guard, x).at(&at)? != Tri::True
        {
            return reject(Rule::WitnessGuard, at);
        }
        for (_, _, reset) in &transition.resets {
            if !machine.defined(reset, x).at(&at)?.is_yes() {
                return reject(Rule::WitnessGuard, at);
            }
        }
        for (i, slot) in y.iter().enumerate() {
            let named = transition
                .resets
                .iter()
                .any(|(variable, _, _)| *variable == i);
            if !named && !within(std::slice::from_ref(&x[i]), std::slice::from_ref(slot)) {
                return reject(Rule::WitnessGuard, at);
            }
        }
        for (variable, op, bound) in &transition.resets {
            let value = machine.num(bound, x).at(&at)?;
            let holds = match op {
                CompareOp::Eq => within(
                    std::slice::from_ref(&value),
                    std::slice::from_ref(&y[*variable]),
                ),
                other => compare(*other, &y[*variable], &value, &zero()) == Tri::True,
            };
            if !holds {
                return reject(Rule::WitnessGuard, at);
            }
        }
        if machine.tri(&target.invariant, y).at(&at)? != Tri::True {
            return reject(Rule::WitnessGuard, at);
        }
        let state = &trace.jumps[d].state;
        let meets = |enclosure: &[ExactInterval], bx: &Bx| {
            enclosure
                .iter()
                .zip(bx)
                .all(|(e, b)| iv(e).intersect(b).is_some())
        };
        if !(state.time.lower <= tau && tau <= state.time.upper) || !meets(&state.enclosure, y) {
            return reject(Rule::WitnessGuard, at);
        }
        if let Some(before) = &trace.jumps[d].before {
            if !(before.time.lower <= tau && tau <= before.time.upper)
                || !meets(&before.enclosure, x)
            {
                return reject(Rule::WitnessGuard, at);
            }
        }
    }

    // 6: witness-violation
    let eval = ClaimEval::build(machine, claim, n).at(&end_pointer)?;
    let steps_clock = claim.clock == HybridClock::ModelSteps;
    let mut metas: Vec<Meta> = Vec::new();
    for (d, list) in ws.iter().enumerate() {
        for (k, step) in list.iter().enumerate() {
            if steps_clock && k > 0 {
                continue;
            }
            metas.push(Meta {
                dwell: d,
                step: k,
                at_box: true,
                time: (step.time.0.clone(), step.time.0.clone()),
                state: step.bx.clone(),
            });
            if steps_clock {
                continue;
            }
            let last = d == last_dwell && k == last_step(d);
            let usable = if last {
                machine
                    .tri(&mx.modes[step.mode].invariant, &step.ap)
                    .at(&pointer(d, k))?
                    == Tri::True
            } else {
                true
            };
            if usable {
                metas.push(Meta {
                    dwell: d,
                    step: k,
                    at_box: false,
                    time: step.time.clone(),
                    state: step.ap.clone(),
                });
            }
        }
    }
    let mut order: Vec<usize> = (0..metas.len()).collect();
    order.sort_by(|a, b| {
        let (a, b) = (&metas[*a], &metas[*b]);
        let end = |m: &Meta| {
            if m.at_box {
                m.time.0.clone()
            } else {
                m.time.1.clone()
            }
        };
        a.dwell
            .cmp(&b.dwell)
            .then_with(|| end(a).cmp(&end(b)))
            .then_with(|| b.at_box.cmp(&a.at_box))
            .then_with(|| a.step.cmp(&b.step))
    });
    let (spans, tail): (Vec<Span>, Option<ExactReal>) = if steps_clock {
        (
            (0..ws.len())
                .map(|d| span(&natural(d), &natural(d)))
                .collect(),
            Some(natural(ws.len()).into()),
        )
    } else {
        let spans = ws
            .iter()
            .map(|list| span(&list[0].time.0, &list[list.len() - 1].time.1))
            .collect();
        (
            spans,
            Some(ws[last_dwell][last_step(last_dwell)].time.1.clone().into()),
        )
    };
    let reading_time = |m: &Meta| -> (Rational, Option<Rational>) {
        if steps_clock {
            (natural(m.dwell), Some(natural(m.dwell)))
        } else {
            (m.time.0.clone(), Some(m.time.1.clone()))
        }
    };
    let whole = vec![Interval::whole(); n];
    // The activations: the origin, or each box reading the guard admits.
    let mut activations: Vec<Activation> = Vec::new();
    let mut guard_undefined: Vec<(usize, Undefined)> = Vec::new();
    let candidates: Vec<Option<usize>> = if eval.each {
        let mut found = Vec::new();
        for (position, meta) in metas.iter().enumerate() {
            if !meta.at_box {
                continue;
            }
            let at = pointer(meta.dwell, meta.step);
            let admitted = match &eval.guard {
                None => true,
                Some(guard) => {
                    let ext = ClaimEval::ext(
                        &meta.state,
                        &super::claim::Ctx {
                            tail: {
                                let mut tail = meta.state.clone();
                                tail.extend(vec![Interval::whole(); eval.captures.len()]);
                                tail
                            },
                        },
                    );
                    let (tri, def) = machine.reading(guard, &ext).at(&at)?;
                    if let Def::No(undefined) = def {
                        guard_undefined.push((position, undefined));
                    }
                    tri == Tri::True
                }
            };
            if admitted {
                found.push(Some(position));
            }
        }
        found
    } else {
        vec![None]
    };
    for at in candidates {
        let (state, act) = match at {
            None => (p.clone(), whole.clone()),
            Some(position) => (metas[position].state.clone(), metas[position].state.clone()),
        };
        let place = at.map_or_else(
            || pointer(0, 0),
            |position| pointer(metas[position].dwell, metas[position].step),
        );
        let (ctx, status) = eval.context(machine, &state, &act).at(&place)?;
        let mut vals = Vec::with_capacity(metas.len());
        for meta in &metas {
            vals.push(
                eval.atoms_over(machine, &meta.state, &ctx)
                    .at(&pointer(meta.dwell, meta.step))?,
            );
        }
        activations.push(Activation { at, status, vals });
    }
    let horizon_end = |origin: &Rational| -> Option<Rational> {
        match (&claim.horizon, claim.clock) {
            (Some(HybridHorizon::Seconds(h)), HybridClock::ModelTime) => Some(origin.add(h)),
            (Some(HybridHorizon::Positions(c)), HybridClock::ModelSteps) => {
                Some(origin.add(&natural(usize::try_from(*c).unwrap_or(usize::MAX))))
            }
            _ => None,
        }
    };
    let decisive_of = |meta: &Meta, kind: HybridCounterexampleKind| Decisive {
        position: meta.dwell,
        time: ExactInterval::new(
            if steps_clock {
                meta.time.0.clone()
            } else {
                meta.time.0.clone()
            },
            meta.time.1.clone(),
        ),
        enclosure: exact(&meta.state),
        kind,
    };
    let undefined_kind = |meta: &Meta, site: ExpressionSite, undefined: &Undefined| {
        HybridCounterexampleKind::UndefinedEvaluation {
            at: Box::new(UndefinedInstant {
                state: meta.dwell,
                time: ExactReal::Rational(meta.time.0.clone()),
                time_interval: (!meta.at_box)
                    .then(|| ExactInterval::new(meta.time.0.clone(), meta.time.1.clone())),
                locus: ExpressionLocus {
                    site,
                    term: undefined.locus.clone(),
                },
            }),
            cause: undefined.cause,
        }
    };
    let mut found: Option<Decisive> = None;
    'search: for (position, &index) in order.iter().enumerate() {
        let meta = &metas[index];
        let included: Vec<usize> = order[..=position].to_vec();
        for (a, activation) in activations.iter().enumerate() {
            let origin_time = activation
                .at
                .map_or_else(zero, |reading| reading_time(&metas[reading]).0);
            let activation_position = activation.at.map(|reading| {
                order
                    .iter()
                    .position(|o| *o == reading)
                    .unwrap_or(usize::MAX)
            });
            if activation_position.is_some_and(|ap| ap > position) {
                continue;
            }
            let in_region = horizon_end(&origin_time).is_none_or(|end| reading_time(meta).0 <= end);
            if let CapStatus::Undefined(undefined) = &activation.status {
                let at_reading = activation
                    .at
                    .map_or(position == 0, |reading| reading == index);
                if at_reading {
                    let site = ExpressionSite::Capture { capture: 0 };
                    found = Some(decisive_of(meta, undefined_kind(meta, site, undefined)));
                    break 'search;
                }
            }
            if let Some((_, undefined)) = guard_undefined
                .iter()
                .find(|(reading, _)| *reading == index && activation.at == Some(*reading))
            {
                found = Some(decisive_of(
                    meta,
                    undefined_kind(meta, ExpressionSite::Activation, undefined),
                ));
                break 'search;
            }
            if in_region {
                for (atom, (_, def)) in activation.vals[index].iter().enumerate() {
                    if let Def::No(undefined) = def {
                        found = Some(decisive_of(
                            meta,
                            undefined_kind(meta, ExpressionSite::ClaimAtom { atom }, undefined),
                        ));
                        break 'search;
                    }
                }
            }
            let readings: Vec<Reading> = metas
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let (lo, hi) = reading_time(m);
                    Reading {
                        step: m.step,
                        segment: m.dwell,
                        lo,
                        hi,
                        vals: activations[a].vals[i].clone(),
                    }
                })
                .collect();
            let (timeline, signal) =
                eval.formula_signal(&readings, &included, &spans, tail.clone());
            let value = match activation.at {
                None => timeline.at_origin(&signal),
                Some(reading) => {
                    let m = &metas[reading];
                    super::super::signal::Timeline::value(
                        &signal,
                        m.dwell,
                        &ExactReal::Rational(reading_time(m).0),
                    )
                }
            };
            if value == Tri::False {
                found = Some(decisive_of(meta, HybridCounterexampleKind::Formula));
                break 'search;
            }
        }
    }
    let Some(decisive) = found else {
        return reject(Rule::WitnessViolation, end_pointer);
    };
    if let Some(end) = &trace.end {
        if end.time != decisive.time || end.enclosure != decisive.enclosure {
            return reject(Rule::WitnessViolation, end_pointer);
        }
    }
    Ok(decisive)
}
