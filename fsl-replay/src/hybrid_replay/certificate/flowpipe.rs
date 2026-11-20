// SPDX-License-Identifier: AGPL-3.0-or-later
//! The rules of `flowpipe` (FSpec FR-652 "Rules of `flowpipe`"):
//! `box-shape`, `initial`, `picard`, `successor`, `jump`, `coverage`,
//! `claim`.

use std::collections::BTreeMap;

use filament_exact::{Integer, Rational};
use fsl_semantics::check::{
    CheckedHybridClaim, CheckedHybridModel, CompareOp, HybridActivation, HybridClock, HybridHorizon,
};

use super::super::expand::Exp;
use super::super::interval::{Interval, Tri};
use super::super::trace::zero;
use super::claim::{span, CapStatus, ClaimEval, Reading};
use super::machine::{
    advance, bound_form, conjuncts, hull_boxes, tube, within, Bx, Machine, ModeX, ModelX, Stop,
};
use super::wire::WireStep;
use super::{names, reject, shaped, step_pointer, At, Fail, ProofCoverage, Rule};
use crate::hybrid::BoundDimension;

struct St {
    mode: usize,
    h: Rational,
    time: (Rational, Rational),
    bx: Bx,
    ap: Bx,
    from: Option<(usize, usize)>,
}

struct Dwell {
    first: usize,
    last: usize,
    mode: usize,
    depth: usize,
    from: Option<(usize, usize)>,
}

struct Layout {
    st: Vec<St>,
    dw: Vec<Dwell>,
    /// `S`, the start instants of each step.
    s: Vec<(Rational, Rational)>,
    /// The jump dwell leaving `(dwell, transition)`.
    jump_dwell: BTreeMap<(usize, usize), usize>,
}

impl Layout {
    fn new(steps: Vec<St>) -> Self {
        let mut dw: Vec<Dwell> = Vec::new();
        for (k, step) in steps.iter().enumerate() {
            let starts = k == 0 || step.from.is_some() || step.mode != steps[k - 1].mode;
            if starts {
                let depth = step.from.map_or(0, |(parent, _)| dw[parent].depth + 1);
                dw.push(Dwell {
                    first: k,
                    last: k,
                    mode: step.mode,
                    depth,
                    from: step.from,
                });
            } else if let Some(current) = dw.last_mut() {
                current.last = k;
            }
        }
        let mut s: Vec<(Rational, Rational)> = Vec::with_capacity(steps.len());
        for dwell in &dw {
            for k in dwell.first..=dwell.last {
                s.push(if k == dwell.first {
                    match dwell.from {
                        None => (zero(), zero()),
                        Some(_) => steps[k].time.clone(),
                    }
                } else {
                    let (lo, hi) = &s[k - 1];
                    (lo.add(&steps[k - 1].h), hi.add(&steps[k - 1].h))
                });
            }
        }
        let jump_dwell = dw
            .iter()
            .enumerate()
            .filter_map(|(d, dwell)| dwell.from.map(|from| (from, d)))
            .collect();
        Self {
            st: steps,
            dw,
            s,
            jump_dwell,
        }
    }

    /// The covered instants `C` of step `k`: `[S.lower, S.upper + h]`, no
    /// upper end for the last step of a closing dwell.
    fn covered(&self, k: usize, closing: bool) -> (Rational, Option<Rational>) {
        let (lo, hi) = &self.s[k];
        let last = self.dw.iter().any(|d| d.last == k);
        if closing && last {
            (lo.clone(), None)
        } else {
            (lo.clone(), Some(hi.add(&self.st[k].h)))
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Ending {
    reaches: bool,
    empties: bool,
    closes: bool,
}

fn interval_of(time: &(Rational, Rational)) -> Interval {
    Interval {
        lower: Some(time.0.clone()),
        upper: Some(time.1.clone()),
    }
}

/// The contracted advance of step `k`: `None` when empty.
fn next_box(
    machine: &mut Machine<'_, '_>,
    mx: &ModelX,
    lay: &Layout,
    k: usize,
) -> Result<Option<Bx>, Stop> {
    let step = &lay.st[k];
    let mode: &ModeX = &mx.modes[step.mode];
    let rates = machine.rates(mode, &step.ap)?;
    let advanced = advance(&step.bx, &step.h, &rates);
    machine.contract(&[&mode.invariant], advanced)
}

fn endings(
    machine: &mut Machine<'_, '_>,
    mx: &ModelX,
    lay: &Layout,
    horizon: &Rational,
) -> Result<Vec<Ending>, Fail> {
    let mut out = Vec::new();
    for dwell in &lay.dw {
        let pointer = step_pointer(dwell.last);
        let last = &lay.st[dwell.last];
        let reaches = lay.s[dwell.last].0.add(&last.h) >= *horizon;
        let next = next_box(machine, mx, lay, dwell.last).at(&pointer)?;
        let (empties, closes) = match next {
            None => (true, false),
            Some(next) => (false, last.h > zero() && within(&next, &last.bx)),
        };
        out.push(Ending {
            reaches,
            empties,
            closes,
        });
    }
    Ok(out)
}

fn contains(time: &(Rational, Rational), need: &Interval) -> bool {
    matches!((&need.lower, &need.upper), (Some(l), Some(u)) if time.0 <= *l && time.1 >= *u)
}

/// The narrowed jump time `N` of the transition `t` of dwell `dwell`
/// (FSpec FR-652 "Following transitions").
pub(super) fn narrowed_over(
    machine: &mut Machine<'_, '_>,
    mx: &ModelX,
    mode_index: usize,
    transition: usize,
    first_bx: &Bx,
    s_first: &(Rational, Rational),
    horizon: &Rational,
) -> Result<Option<Option<Interval>>, Stop> {
    let mode = &mx.modes[mode_index];
    let guard = &mode.transitions[transition].guard;
    let mut list = Vec::new();
    conjuncts(guard, &mut list);
    for conjunct in list {
        let Some((x, c, CompareOp::Eq)) = bound_form(conjunct) else {
            continue;
        };
        let constant = !c.any_node(|node| {
            matches!(
                node,
                Exp::Variable(_)
                    | Exp::Activation(_)
                    | Exp::Capture(_)
                    | Exp::Conditional(..)
                    | Exp::Recursive(_)
                    | Exp::Parameter(_)
            )
        });
        if !constant || x >= mode.rates.len() {
            continue;
        }
        let rate = &mode.rates[x];
        let reads_fixed = (0..mx.n).all(|j| {
            !rate.reads_variable(j) || matches!(&mode.rates[j], Exp::Number(q) if q.is_zero())
        });
        if !reads_fixed
            || rate.any_node(|node| matches!(node, Exp::Conditional(..) | Exp::Recursive(_)))
        {
            continue;
        }
        let r = machine.num(rate, first_bx)?;
        if r.contains_zero() {
            continue;
        }
        let value = machine.num(c, first_bx)?;
        let numerator = value.add(&first_bx[x].neg());
        let quotient = numerator.div(&r);
        let shifted = quotient.add(&interval_of(s_first));
        let window = Interval {
            lower: Some(zero()),
            upper: Some(horizon.clone()),
        };
        return Ok(Some(shifted.intersect(&window)));
    }
    Ok(None)
}

/// The narrowed jump time of dwell `dwell`'s transition `transition`.
fn narrowed(
    machine: &mut Machine<'_, '_>,
    mx: &ModelX,
    lay: &Layout,
    dwell: &Dwell,
    transition: usize,
    horizon: &Rational,
) -> Result<Option<Option<Interval>>, Stop> {
    narrowed_over(
        machine,
        mx,
        dwell.mode,
        transition,
        &lay.st[dwell.first].bx,
        &lay.s[dwell.first],
        horizon,
    )
}

pub(super) fn check(
    machine: &mut Machine<'_, '_>,
    mx: &ModelX,
    model: &CheckedHybridModel,
    claim: &CheckedHybridClaim,
    steps: &[WireStep],
    coverage: Option<&ProofCoverage>,
    initial_containment: bool,
) -> Result<(), Fail> {
    let variable_names = names(model);
    let n = mx.n;
    // 1: box-shape
    let mut typed = Vec::with_capacity(steps.len());
    for (k, step) in steps.iter().enumerate() {
        let Some(bx) = shaped(&step.bx, &variable_names) else {
            return reject(Rule::BoxShape, format!("{}/box", step_pointer(k)));
        };
        let Some(ap) = shaped(&step.apriori, &variable_names) else {
            return reject(Rule::BoxShape, format!("{}/apriori", step_pointer(k)));
        };
        typed.push(St {
            mode: step.mode,
            h: step.step.clone(),
            time: (step.time.lower.clone(), step.time.upper.clone()),
            bx,
            ap,
            from: step.from,
        });
    }
    let lay = Layout::new(typed);
    let Some(coverage) = coverage else {
        return reject(Rule::Coverage, step_pointer(0));
    };
    let horizon = &coverage.horizon;
    let jumps = usize::try_from(coverage.jumps).unwrap_or(usize::MAX);

    // 2: initial
    for dwell in lay.dw.iter().filter(|dwell| dwell.from.is_none()) {
        let first = &lay.st[dwell.first];
        if first.time != (zero(), first.h.clone()) {
            return reject(Rule::Initial, step_pointer(dwell.first));
        }
    }
    for (mode, condition) in mx.initial.iter().filter(|_| initial_containment) {
        let invariant = &mx.modes[*mode].invariant;
        let dwell = lay
            .dw
            .iter()
            .find(|dwell| dwell.from.is_none() && dwell.mode == *mode);
        let at = dwell.map_or_else(|| step_pointer(0), |dwell| step_pointer(dwell.first));
        let Some(hull) = machine.hull(&[condition, invariant], n).at(&at)? else {
            continue;
        };
        let defined = machine.defined(condition, &hull).at(&at)?.is_yes()
            && machine.defined(invariant, &hull).at(&at)?.is_yes();
        match dwell {
            Some(dwell) if defined && within(&hull, &lay.st[dwell.first].bx) => {}
            _ => return reject(Rule::Initial, at),
        }
    }

    // 3: picard
    for (k, step) in lay.st.iter().enumerate() {
        let at = step_pointer(k);
        let mode = &mx.modes[step.mode];
        if !machine
            .picard_holds(mode, &step.bx, &step.ap, &step.h)
            .at(&at)?
        {
            return reject(Rule::Picard, at);
        }
    }

    // 4: successor
    for dwell in &lay.dw {
        for k in dwell.first + 1..=dwell.last {
            let at = step_pointer(k);
            let expected = (lay.s[k].0.clone(), lay.s[k].1.add(&lay.st[k].h));
            if lay.st[k].time != expected {
                return reject(Rule::Successor, at);
            }
            let next = next_box(machine, mx, &lay, k - 1).at(&at)?;
            if let Some(next) = next {
                if !within(&next, &lay.st[k].bx) {
                    return reject(Rule::Successor, at);
                }
            }
        }
    }

    let ends = endings(machine, mx, &lay, horizon)?;
    // 5: jump
    for (d, dwell) in lay.dw.iter().enumerate() {
        if dwell.depth >= jumps {
            continue;
        }
        let mode = &mx.modes[dwell.mode];
        for (t, transition) in mode.transitions.iter().enumerate() {
            let target = &mx.modes[transition.target];
            let scope: Vec<usize> = (dwell.first..=dwell.last)
                .filter(|k| lay.s[*k].0 <= *horizon)
                .collect();
            for k in &scope {
                let at = step_pointer(*k);
                if !machine
                    .defined(&transition.guard, &lay.st[*k].ap)
                    .at(&at)?
                    .is_yes()
                {
                    return reject(Rule::Jump, at);
                }
            }
            let mut fired: Vec<(usize, Bx)> = Vec::new();
            for k in &scope {
                let at = step_pointer(*k);
                if let Some((g, post)) = machine
                    .fire(transition, &lay.st[*k].ap, &target.invariant)
                    .at(&at)?
                {
                    for (_, _, reset) in &transition.resets {
                        if !machine.defined(reset, &g).at(&at)?.is_yes() {
                            return reject(Rule::Jump, at);
                        }
                    }
                    fired.push((*k, post));
                }
            }
            let Some((first_fired, _)) = fired.first() else {
                continue;
            };
            let Some(jump_dwell) = lay.jump_dwell.get(&(d, t)) else {
                return reject(Rule::Jump, step_pointer(*first_fired));
            };
            let j = lay.dw[*jump_dwell].first;
            let at = step_pointer(j);
            let mut posts = fired.iter().map(|(_, post)| post.clone());
            let seed = posts.next().unwrap_or_default();
            let hull = posts.fold(seed, |acc, post| hull_boxes(&acc, &post));
            if !within(&hull, &lay.st[j].bx) {
                return reject(Rule::Jump, at);
            }
            let need = match narrowed(machine, mx, &lay, dwell, t, horizon).at(&at)? {
                Some(window) => window,
                None => {
                    let mut low: Option<Rational> = None;
                    let mut high: Option<Rational> = Some(zero());
                    for (k, _) in &fired {
                        let (lo, hi) = lay.covered(*k, ends[d].closes);
                        low = Some(low.map_or(lo.clone(), |l| l.min(lo)));
                        high = match (high, hi) {
                            (Some(a), Some(b)) => Some(a.max(b)),
                            _ => None,
                        };
                    }
                    Some(Interval {
                        lower: low,
                        upper: Some(
                            high.map_or_else(|| horizon.clone(), |h| h.min(horizon.clone())),
                        ),
                    })
                }
            };
            if let Some(need) = need {
                if !contains(&lay.st[j].time, &need) {
                    return reject(Rule::Jump, at);
                }
            }
        }
    }

    // 6: coverage
    for (d, dwell) in lay.dw.iter().enumerate() {
        if dwell.depth > jumps {
            return reject(Rule::Coverage, step_pointer(dwell.first));
        }
        let end = ends[d];
        if !(end.reaches || end.empties || end.closes) {
            return reject(Rule::Coverage, step_pointer(dwell.last));
        }
        let mode = &mx.modes[dwell.mode];
        let can_fire = |machine: &mut Machine<'_, '_>, k: usize| -> Result<bool, Fail> {
            let at = step_pointer(k);
            for transition in &mode.transitions {
                let target = &mx.modes[transition.target];
                if machine
                    .fire(transition, &lay.st[k].ap, &target.invariant)
                    .at(&at)?
                    .is_some()
                {
                    return Ok(true);
                }
            }
            Ok(false)
        };
        if coverage.exhaustive.contains(&BoundDimension::Time) && dwell.depth == jumps {
            for k in dwell.first..=dwell.last {
                if lay.s[k].0 <= *horizon && can_fire(machine, k)? {
                    return reject(Rule::Coverage, step_pointer(k));
                }
            }
        }
        if coverage.exhaustive.contains(&BoundDimension::Jumps) && dwell.depth < jumps {
            if !(end.empties || end.closes) {
                return reject(Rule::Coverage, step_pointer(dwell.last));
            }
            for k in dwell.first..=dwell.last {
                if can_fire(machine, k)? {
                    let (_, upper) = lay.covered(k, end.closes);
                    if upper.is_none_or(|u| u > *horizon) {
                        return reject(Rule::Coverage, step_pointer(k));
                    }
                }
            }
        }
    }

    // 7: claim
    claim_rule(machine, mx, claim, &lay, &ends)
}

fn claim_rule(
    machine: &mut Machine<'_, '_>,
    mx: &ModelX,
    claim: &CheckedHybridClaim,
    lay: &Layout,
    ends: &[Ending],
) -> Result<(), Fail> {
    let root = "/certificate";
    let n = mx.n;
    let Some(horizon) = &claim.horizon else {
        return reject(Rule::Claim, root);
    };
    if !matches!(claim.activation, HybridActivation::Origin) {
        return reject(Rule::Claim, root);
    }
    let eval = ClaimEval::build(machine, claim, n).at(root)?;
    // The initial states: the hull of the initial entries' hulls.
    let mut initial: Option<Bx> = None;
    for (mode, condition) in &mx.initial {
        let invariant = &mx.modes[*mode].invariant;
        if let Some(hull) = machine.hull(&[condition, invariant], n).at(root)? {
            initial = Some(match initial {
                None => hull,
                Some(previous) => hull_boxes(&previous, &hull),
            });
        }
    }
    let Some(initial) = initial else {
        return reject(Rule::Claim, root);
    };
    let (ctx, status) = eval
        .context(machine, &initial, &vec![Interval::whole(); n])
        .at(root)?;
    if !matches!(status, CapStatus::Ok) {
        return reject(Rule::Claim, root);
    }
    let mut closing = vec![false; lay.dw.len()];
    for (d, end) in ends.iter().enumerate() {
        closing[d] = end.closes;
    }
    // may-end flags
    let mut may_end = vec![false; lay.st.len()];
    for (k, step) in lay.st.iter().enumerate() {
        let at = step_pointer(k);
        let mode = &mx.modes[step.mode];
        let rates = machine.rates(mode, &step.ap).at(&at)?;
        let region = tube(&step.bx, &step.h, &rates);
        let inside = machine.tri(&mode.invariant, &region).at(&at)? == Tri::True;
        let last_of = lay.dw.iter().position(|d| d.last == k);
        let ending = last_of.is_some_and(|d| ends[d].empties || ends[d].closes);
        may_end[k] = !inside || ending;
    }
    let mut readings: Vec<Reading> = Vec::new();
    let region_end: Rational;
    let spans = match (claim.clock, horizon) {
        (HybridClock::ModelTime, HybridHorizon::Seconds(r)) => {
            region_end = r.clone();
            for (k, step) in lay.st.iter().enumerate() {
                let at = step_pointer(k);
                let vals = eval.atoms_over(machine, &step.ap, &ctx).at(&at)?;
                let d = lay
                    .dw
                    .iter()
                    .position(|d| d.first <= k && k <= d.last)
                    .unwrap_or(0);
                let (lo, hi) = lay.covered(k, closing[d]);
                readings.push(Reading {
                    step: k,
                    segment: 0,
                    lo,
                    hi: if may_end[k] { None } else { hi },
                    vals,
                });
            }
            vec![span(&zero(), r)]
        }
        (HybridClock::ModelSteps, HybridHorizon::Positions(count)) => {
            let positions = usize::try_from(*count).unwrap_or(usize::MAX);
            region_end =
                Rational::from_integer(Integer::from(i64::try_from(*count).unwrap_or(i64::MAX)));
            let point = |p: usize| {
                Rational::from_integer(Integer::from(i64::try_from(p).unwrap_or(i64::MAX)))
            };
            let mut spans = Vec::new();
            for p in 0..=positions {
                spans.push(span(&point(p), &point(p)));
            }
            for p in 0..=positions {
                for (d, dwell) in lay.dw.iter().enumerate() {
                    let first = dwell.first;
                    let mut sources: Vec<(usize, bool)> = Vec::new();
                    if dwell.depth == p {
                        sources.push((first, false));
                    }
                    if dwell.depth < p {
                        for k in dwell.first..=dwell.last {
                            if may_end[k] {
                                sources.push((k, true));
                            }
                        }
                        if closing[d] {
                            sources.push((first, false));
                        }
                    }
                    for (k, apriori) in sources {
                        let at = step_pointer(k);
                        let state = if apriori {
                            &lay.st[k].ap
                        } else {
                            &lay.st[k].bx
                        };
                        let vals = eval.atoms_over(machine, state, &ctx).at(&at)?;
                        readings.push(Reading {
                            step: k,
                            segment: p,
                            lo: point(p),
                            hi: Some(point(p)),
                            vals,
                        });
                    }
                }
            }
            spans
        }
        _ => return reject(Rule::Claim, root),
    };
    let all: Vec<usize> = (0..readings.len()).collect();
    let (_, _, value) = eval.value(&readings, &all, &spans, None);
    let in_region = |reading: &Reading| reading.lo <= region_end;
    let bad = |reading: &Reading| {
        reading
            .vals
            .iter()
            .any(|(tri, def)| !def.is_yes() || *tri != Tri::True)
    };
    let defined = readings
        .iter()
        .filter(|reading| in_region(reading))
        .all(|reading| reading.vals.iter().all(|(_, def)| def.is_yes()));
    if value == Tri::True && defined {
        return Ok(());
    }
    let first_bad = readings
        .iter()
        .filter(|reading| in_region(reading) && bad(reading))
        .map(|reading| reading.step)
        .min();
    let first_read = readings
        .iter()
        .filter(|reading| in_region(reading))
        .map(|reading| reading.step)
        .min();
    reject(
        Rule::Claim,
        step_pointer(first_bad.or(first_read).unwrap_or(0)),
    )
}
