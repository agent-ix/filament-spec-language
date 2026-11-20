// SPDX-License-Identifier: AGPL-3.0-or-later
//! The interval reading of FSpec FR-652 "Interval reading": exact rational
//! evaluation over a box, definedness, the contractor, guarded parts and
//! post images, tubes and advances, boundary faces and strictly decided
//! conditionals. Each evaluation of one expression over one box is one unit
//! of the item's work meter ("Work limit").

use filament_exact::Rational;
use fsl_semantics::check::{CheckedGraph, CheckedHybridModel, CompareOp, HybridTerm, LogicOp};

use super::super::expand::{Exp, Expander, RangeRole, TermLocus};
use super::super::interval::{IValue, Interval, IntervalEvaluator, Tri};
use super::super::trace::zero;
use super::super::{HybridReplayMeter, HybridReplayRefusal, UndefinedCause};

pub(super) type Bx = Vec<Interval>;

/// Why a read stopped before it decided.
#[derive(Debug)]
pub(super) enum Stop {
    /// The item's work meter stopped the check.
    WorkLimit,
    /// A fault of the checked model.
    Fault(HybridReplayRefusal),
}

impl From<HybridReplayRefusal> for Stop {
    fn from(refusal: HybridReplayRefusal) -> Self {
        match refusal {
            HybridReplayRefusal::ResourceExhausted { .. } => Self::WorkLimit,
            other => Self::Fault(other),
        }
    }
}

/// Why an expression has no value over a whole box.
#[derive(Clone, Debug)]
pub(super) struct Undefined {
    pub(super) locus: TermLocus,
    pub(super) cause: UndefinedCause,
}

/// An expression's definedness over a box.
#[derive(Clone, Debug)]
pub(super) enum Def {
    Yes,
    /// Definitely without value.
    No(Undefined),
    Maybe,
}

impl Def {
    pub(super) const fn is_yes(&self) -> bool {
        matches!(self, Self::Yes)
    }
}

/// One mode's expanded expressions.
pub(super) struct ModeX {
    pub(super) invariant: Exp,
    pub(super) rates: Vec<Exp>,
    pub(super) transitions: Vec<TransX>,
    /// Whether the mode is contraction-admitted (FSpec FR-652 "Why
    /// contraction is sound").
    pub(super) admitted: bool,
}

pub(super) struct TransX {
    pub(super) target: usize,
    pub(super) guard: Exp,
    pub(super) resets: Vec<(usize, CompareOp, Exp)>,
}

/// The checked model with every expression expanded.
pub(super) struct ModelX {
    pub(super) n: usize,
    pub(super) initial: Vec<(usize, Exp)>,
    pub(super) modes: Vec<ModeX>,
}

pub(super) fn conjuncts<'a>(exp: &'a Exp, out: &mut Vec<&'a Exp>) {
    match exp {
        Exp::Logic(LogicOp::And, a, b) => {
            conjuncts(a, out);
            conjuncts(b, out);
        }
        other => out.push(other),
    }
}

pub(super) const fn flip(op: CompareOp) -> CompareOp {
    match op {
        CompareOp::Lt => CompareOp::Gt,
        CompareOp::Gt => CompareOp::Lt,
        CompareOp::Le => CompareOp::Ge,
        CompareOp::Ge => CompareOp::Le,
        CompareOp::Eq => CompareOp::Eq,
        CompareOp::Ne => CompareOp::Ne,
    }
}

/// `x ~ e` or `e ~ x` with a bare variable on one side: the variable, the
/// other side and the relation read as `x ~ e`.
pub(super) fn bound_form(exp: &Exp) -> Option<(usize, &Exp, CompareOp)> {
    let Exp::Compare(op, a, b) = exp else {
        return None;
    };
    match (&**a, &**b) {
        (Exp::Variable(i), _) => Some((*i, &**b, *op)),
        (_, Exp::Variable(i)) => Some((*i, &**a, flip(*op))),
        _ => None,
    }
}

pub(super) fn is_empty(interval: &Interval) -> bool {
    matches!((&interval.lower, &interval.upper), (Some(l), Some(u)) if l > u)
}

/// `a` lies in `b`: every interval of `a` in the interval of `b`.
pub(super) fn within(a: &[Interval], b: &[Interval]) -> bool {
    a.iter().zip(b).all(|(x, y)| {
        let low = match &y.lower {
            None => true,
            Some(bound) => x.lower.as_ref().is_some_and(|value| value >= bound),
        };
        let high = match &y.upper {
            None => true,
            Some(bound) => x.upper.as_ref().is_some_and(|value| value <= bound),
        };
        low && high
    })
}

pub(super) fn hull_boxes(a: &[Interval], b: &[Interval]) -> Bx {
    a.iter().zip(b).map(|(x, y)| x.hull(y)).collect()
}

fn excludes_zero(interval: &Interval) -> bool {
    !interval.contains_zero()
}

/// The reading machine over one checked graph.
pub(super) struct Machine<'g, 'w> {
    pub(super) expander: Expander<'g>,
    pub(super) scratch: HybridReplayMeter,
    work: &'w mut HybridReplayMeter,
}

impl<'g, 'w> Machine<'g, 'w> {
    pub(super) fn new(graph: &'g CheckedGraph, work: &'w mut HybridReplayMeter) -> Self {
        Self {
            expander: Expander::new(graph),
            scratch: HybridReplayMeter::new(u64::MAX),
            work,
        }
    }

    /// Charge one evaluation to the item's meter.
    pub(super) fn unit(&mut self) -> Result<(), Stop> {
        self.work.charge().map_err(Stop::from)
    }

    pub(super) fn expand(&mut self, term: &HybridTerm) -> Result<Exp, Stop> {
        Ok(self.expander.term(term, &mut self.scratch)?)
    }

    pub(super) fn expand_model(&mut self, model: &CheckedHybridModel) -> Result<ModelX, Stop> {
        let n = model.variables.len();
        let mut initial = Vec::new();
        for entry in &model.initial {
            initial.push((entry.mode.get(), self.expand(&entry.condition)?));
        }
        let mut modes = Vec::new();
        for mode in &model.modes {
            let invariant = self.expand(&mode.invariant)?;
            let mut rates = Vec::new();
            for rate in &mode.rates {
                rates.push(self.expand(rate)?);
            }
            let mut transitions = Vec::new();
            for transition in &mode.transitions {
                let guard = self.expand(&transition.guard)?;
                let mut resets = Vec::new();
                for reset in &transition.resets {
                    resets.push((
                        reset.variable.get(),
                        reset.relation,
                        self.expand(&reset.value)?,
                    ));
                }
                transitions.push(TransX {
                    target: transition.target.get(),
                    guard,
                    resets,
                });
            }
            let plain = |exp: &Exp| {
                !exp.any_node(|node| matches!(node, Exp::Conditional(..) | Exp::Recursive(_)))
            };
            let mut conjunct_list = Vec::new();
            conjuncts(&invariant, &mut conjunct_list);
            let admitted = rates.iter().all(plain)
                && conjunct_list.iter().all(|conjunct| {
                    bound_form(conjunct)
                        .is_none_or(|(_, bound, op)| matches!(op, CompareOp::Ne) || plain(bound))
                });
            modes.push(ModeX {
                invariant,
                rates,
                transitions,
                admitted,
            });
        }
        Ok(ModelX { n, initial, modes })
    }

    fn raw_value(&mut self, exp: &Exp, bx: &[Interval]) -> Result<IValue, Stop> {
        Ok(
            IntervalEvaluator::new(&mut self.expander, zero(), &mut self.scratch)
                .evaluate(exp.clone(), bx)?,
        )
    }

    fn raw_num(&mut self, exp: &Exp, bx: &[Interval]) -> Result<Interval, Stop> {
        Ok(match self.raw_value(exp, bx)? {
            IValue::Number(interval) => interval,
            IValue::Boolean(_) => Interval::whole(),
        })
    }

    fn raw_tri(&mut self, exp: &Exp, bx: &[Interval]) -> Result<Tri, Stop> {
        Ok(match self.raw_value(exp, bx)? {
            IValue::Boolean(tri) => tri,
            IValue::Number(_) => Tri::Unknown,
        })
    }

    /// The interval value of `exp` over `bx`: one unit.
    pub(super) fn num(&mut self, exp: &Exp, bx: &[Interval]) -> Result<Interval, Stop> {
        self.unit()?;
        self.raw_num(exp, bx)
    }

    /// The three-valued truth of `exp` over `bx`: one unit.
    pub(super) fn tri(&mut self, exp: &Exp, bx: &[Interval]) -> Result<Tri, Stop> {
        self.unit()?;
        self.raw_tri(exp, bx)
    }

    /// The truth of `exp` over `bx`, `Unknown` unless it is definitely
    /// defined there, and its definedness: one unit.
    pub(super) fn reading(&mut self, exp: &Exp, bx: &[Interval]) -> Result<(Tri, Def), Stop> {
        self.unit()?;
        let def = self.defined_in(exp, bx)?;
        let tri = if def.is_yes() {
            self.raw_tri(exp, bx)?
        } else {
            Tri::Unknown
        };
        Ok((tri, def))
    }

    /// Whether `exp` is definitely defined over `bx` (FSpec FR-652
    /// "Definedness"): one unit.
    pub(super) fn defined(&mut self, exp: &Exp, bx: &[Interval]) -> Result<Def, Stop> {
        self.unit()?;
        self.defined_in(exp, bx)
    }

    fn defined_in(&mut self, exp: &Exp, bx: &[Interval]) -> Result<Def, Stop> {
        let both = |a: Def, b: Def| match (&a, &b) {
            (Def::No(_), _) => a,
            (_, Def::No(_)) => b,
            (Def::Yes, Def::Yes) => Def::Yes,
            _ => Def::Maybe,
        };
        Ok(match exp {
            Exp::Variable(_)
            | Exp::Activation(_)
            | Exp::Capture(_)
            | Exp::Parameter(_)
            | Exp::Number(_)
            | Exp::Boolean(_) => Def::Yes,
            Exp::Negate(a) | Exp::Not(a) => self.defined_in(a, bx)?,
            Exp::Arithmetic(op, a, b, locus) => {
                let da = self.defined_in(a, bx)?;
                let db = self.defined_in(b, bx)?;
                let joined = both(da, db);
                if let Def::No(_) = joined {
                    return Ok(joined);
                }
                if matches!(op, fsl_semantics::check::ArithmeticOp::Div) {
                    let divisor = self.raw_num(b, bx)?;
                    let zero_point = divisor.lower.as_ref().is_some_and(Rational::is_zero)
                        && divisor.upper.as_ref().is_some_and(Rational::is_zero);
                    if zero_point {
                        return Ok(Def::No(Undefined {
                            locus: locus.clone(),
                            cause: UndefinedCause::DivisionByZero,
                        }));
                    }
                    if divisor.contains_zero() {
                        return Ok(Def::Maybe);
                    }
                }
                joined
            }
            Exp::Compare(_, a, b) => {
                let da = self.defined_in(a, bx)?;
                let db = self.defined_in(b, bx)?;
                both(da, db)
            }
            Exp::Logic(op, a, b) => {
                let da = self.defined_in(a, bx)?;
                match da {
                    Def::No(_) => return Ok(da),
                    Def::Maybe => return Ok(Def::Maybe),
                    Def::Yes => {}
                }
                let ta = self.raw_tri(a, bx)?;
                let short = matches!(
                    (op, ta),
                    (LogicOp::And, Tri::False)
                        | (LogicOp::Or, Tri::True)
                        | (LogicOp::Implies, Tri::False)
                );
                if short {
                    Def::Yes
                } else if ta == Tri::Unknown {
                    if self.defined_in(b, bx)?.is_yes() {
                        Def::Yes
                    } else {
                        Def::Maybe
                    }
                } else {
                    self.defined_in(b, bx)?
                }
            }
            Exp::Conditional(c, a, b, _) => {
                let dc = self.defined_in(c, bx)?;
                match dc {
                    Def::No(_) => return Ok(dc),
                    Def::Maybe => return Ok(Def::Maybe),
                    Def::Yes => {}
                }
                match self.raw_tri(c, bx)? {
                    Tri::True => self.defined_in(a, bx)?,
                    Tri::False => self.defined_in(b, bx)?,
                    Tri::Unknown => {
                        let da = self.defined_in(a, bx)?;
                        let db = self.defined_in(b, bx)?;
                        match (&da, &db) {
                            (Def::Yes, Def::Yes) => Def::Yes,
                            (Def::No(_), Def::No(_)) => da,
                            _ => Def::Maybe,
                        }
                    }
                }
            }
            Exp::Ranged {
                value,
                range,
                role,
                locus,
            } => {
                let dv = self.defined_in(value, bx)?;
                if let Def::No(_) = dv {
                    return Ok(dv);
                }
                let found = self.raw_num(value, bx)?;
                let below = matches!((&range.lower, &found.upper), (Some(l), Some(u)) if u < l);
                let above = matches!((&range.upper, &found.lower), (Some(u), Some(l)) if l > u);
                if below || above {
                    return Ok(Def::No(Undefined {
                        locus: locus.clone(),
                        cause: match role {
                            RangeRole::Parameter {
                                function,
                                parameter,
                            } => UndefinedCause::OutsideParameterRange {
                                function: *function,
                                parameter: *parameter,
                            },
                            RangeRole::Result { function } => UndefinedCause::OutsideResultRange {
                                function: *function,
                            },
                        },
                    }));
                }
                let inside = range
                    .lower
                    .as_ref()
                    .is_none_or(|l| found.lower.as_ref().is_some_and(|v| v >= l))
                    && range
                        .upper
                        .as_ref()
                        .is_none_or(|u| found.upper.as_ref().is_some_and(|v| v <= u));
                if inside {
                    dv
                } else {
                    Def::Maybe
                }
            }
            Exp::Recursive(_) => Def::Maybe,
        })
    }

    /// `Contract(c, X)` (FSpec FR-652 "Contractor"): `None` when the result
    /// is empty. One unit per narrowing conjunct.
    pub(super) fn contract(&mut self, conditions: &[&Exp], mut bx: Bx) -> Result<Option<Bx>, Stop> {
        let mut list = Vec::new();
        for condition in conditions {
            conjuncts(condition, &mut list);
        }
        for conjunct in list {
            let Some((i, bound, op)) = bound_form(conjunct) else {
                continue;
            };
            if i >= bx.len() || op == CompareOp::Ne {
                continue;
            }
            let value = self.num(bound, &bx)?;
            let narrowed = &mut bx[i];
            match op {
                CompareOp::Eq => match narrowed.intersect(&value) {
                    Some(meet) => *narrowed = meet,
                    None => return Ok(None),
                },
                CompareOp::Lt | CompareOp::Le => {
                    if let Some(upper) = value.upper {
                        if narrowed
                            .upper
                            .as_ref()
                            .is_none_or(|current| upper < *current)
                        {
                            narrowed.upper = Some(upper);
                        }
                    }
                }
                CompareOp::Gt | CompareOp::Ge => {
                    if let Some(lower) = value.lower {
                        if narrowed
                            .lower
                            .as_ref()
                            .is_none_or(|current| lower > *current)
                        {
                            narrowed.lower = Some(lower);
                        }
                    }
                }
                CompareOp::Ne => {}
            }
            if is_empty(narrowed) {
                return Ok(None);
            }
        }
        Ok(Some(bx))
    }

    /// `Hull(c)`: the contraction of the unbounded box.
    pub(super) fn hull(&mut self, conditions: &[&Exp], n: usize) -> Result<Option<Bx>, Stop> {
        self.contract(conditions, vec![Interval::whole(); n])
    }

    /// The post image `Post(G)` of the guarded part `g` by the resets
    /// `transition`, contracted by the target invariant.
    pub(super) fn post(
        &mut self,
        transition: &TransX,
        g: &Bx,
        target_invariant: &Exp,
    ) -> Result<Option<Bx>, Stop> {
        let mut image = g.clone();
        for (i, slot) in image.iter_mut().enumerate() {
            let mut value: Option<Interval> = None;
            for (variable, op, bound) in &transition.resets {
                if *variable != i {
                    continue;
                }
                let found = self.num(bound, g)?;
                let part = match op {
                    CompareOp::Eq => found,
                    CompareOp::Lt | CompareOp::Le => Interval {
                        lower: None,
                        upper: found.upper,
                    },
                    CompareOp::Gt | CompareOp::Ge => Interval {
                        lower: found.lower,
                        upper: None,
                    },
                    CompareOp::Ne => Interval::whole(),
                };
                value = Some(match value {
                    None => part,
                    Some(previous) => match previous.intersect(&part) {
                        Some(meet) => meet,
                        None => return Ok(None),
                    },
                });
            }
            if let Some(value) = value {
                *slot = value;
            }
        }
        self.contract(&[target_invariant], image)
    }

    /// The guarded part and post image of `transition` from `x`, when it can
    /// fire from `x`: `G` not empty, the guard not definitely false over
    /// `G`, and `Post(G)` not empty.
    pub(super) fn fire(
        &mut self,
        transition: &TransX,
        x: &Bx,
        target_invariant: &Exp,
    ) -> Result<Option<(Bx, Bx)>, Stop> {
        let Some(g) = self.contract(&[&transition.guard], x.clone())? else {
            return Ok(None);
        };
        if self.tri(&transition.guard, &g)? == Tri::False {
            return Ok(None);
        }
        let Some(post) = self.post(transition, &g, target_invariant)? else {
            return Ok(None);
        };
        Ok(Some((g, post)))
    }

    /// FSpec FR-652 rule `picard` for one step: every rate and the invariant
    /// definitely defined over `ap`, and the (contracted, for an admitted
    /// mode) tube of `bx` over `h` inside `ap`.
    pub(super) fn picard_holds(
        &mut self,
        mode: &ModeX,
        bx: &Bx,
        ap: &Bx,
        h: &Rational,
    ) -> Result<bool, Stop> {
        for exp in mode.rates.iter().chain(std::iter::once(&mode.invariant)) {
            if !self.defined(exp, ap)?.is_yes() {
                return Ok(false);
            }
        }
        let rates = self.rates(mode, ap)?;
        let region = tube(bx, h, &rates);
        if mode.admitted {
            return Ok(match self.contract(&[&mode.invariant], region)? {
                None => true,
                Some(contracted) => within(&contracted, ap),
            });
        }
        for rate in &mode.rates {
            if !self.strictly_decided(rate, ap, false)? {
                return Ok(false);
            }
        }
        Ok(within(&region, ap))
    }

    /// The rates of `mode` over `bx`.
    pub(super) fn rates(&mut self, mode: &ModeX, bx: &Bx) -> Result<Vec<Interval>, Stop> {
        mode.rates.iter().map(|rate| self.num(rate, bx)).collect()
    }

    /// Whether the upper (`upper`) or lower face of variable `i` of `bx` is
    /// a boundary face of `mode`'s invariant (FSpec FR-652 "Boundary face").
    pub(super) fn boundary_face(
        &mut self,
        mode: &ModeX,
        i: usize,
        upper: bool,
        bx: &Bx,
    ) -> Result<bool, Stop> {
        let mut list = Vec::new();
        conjuncts(&mode.invariant, &mut list);
        for conjunct in list {
            let Some((variable, bound, op)) = bound_form(conjunct) else {
                continue;
            };
            let direction = if upper {
                matches!(op, CompareOp::Le | CompareOp::Lt | CompareOp::Eq)
            } else {
                matches!(op, CompareOp::Ge | CompareOp::Gt | CompareOp::Eq)
            };
            if variable != i || !direction {
                continue;
            }
            if bound.any_node(|node| matches!(node, Exp::Conditional(..) | Exp::Recursive(_))) {
                continue;
            }
            let moving = (0..bx.len()).any(|j| {
                bound.reads_variable(j)
                    && !matches!(&mode.rates[j], Exp::Number(rate) if rate.is_zero())
            });
            if moving {
                continue;
            }
            let value = self.num(bound, bx)?;
            let covered = if upper {
                matches!((&bx[i].upper, &value.upper), (Some(face), Some(limit)) if face >= limit)
            } else {
                matches!((&bx[i].lower, &value.lower), (Some(face), Some(limit)) if face <= limit)
            };
            if covered {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether every conditional of `exp` has its condition strictly decided
    /// over `bx` (each comparison it reads has a difference excluding 0),
    /// every conditional anywhere in `exp` when `taken_only` is false and
    /// only those in the branch taken when it is true. A recursive call is
    /// never strictly decided. One unit.
    pub(super) fn strictly_decided(
        &mut self,
        exp: &Exp,
        bx: &Bx,
        taken_only: bool,
    ) -> Result<bool, Stop> {
        self.unit()?;
        let mut pending = vec![exp];
        while let Some(node) = pending.pop() {
            match node {
                Exp::Recursive(_) | Exp::Parameter(_) => return Ok(false),
                Exp::Conditional(condition, yes, no, _) => {
                    if !self.condition_strict(condition, bx)? {
                        return Ok(false);
                    }
                    if taken_only {
                        match self.raw_tri(condition, bx)? {
                            Tri::True => pending.push(yes),
                            Tri::False => pending.push(no),
                            Tri::Unknown => return Ok(false),
                        }
                    } else {
                        pending.extend([&**yes, &**no]);
                    }
                }
                Exp::Negate(a) | Exp::Not(a) | Exp::Ranged { value: a, .. } => pending.push(a),
                Exp::Arithmetic(_, a, b, _) | Exp::Compare(_, a, b) | Exp::Logic(_, a, b) => {
                    pending.extend([&**a, &**b]);
                }
                Exp::Variable(_)
                | Exp::Activation(_)
                | Exp::Capture(_)
                | Exp::Number(_)
                | Exp::Boolean(_) => {}
            }
        }
        Ok(true)
    }

    fn condition_strict(&mut self, condition: &Exp, bx: &Bx) -> Result<bool, Stop> {
        let mut pending = vec![condition];
        while let Some(node) = pending.pop() {
            match node {
                Exp::Recursive(_) | Exp::Parameter(_) => return Ok(false),
                Exp::Compare(_, a, b) => {
                    let left = self.raw_num(a, bx)?;
                    let right = self.raw_num(b, bx)?;
                    if !excludes_zero(&left.add(&right.neg())) {
                        return Ok(false);
                    }
                    pending.extend([&**a, &**b]);
                }
                Exp::Conditional(c, a, b, _) => pending.extend([&**c, &**a, &**b]),
                Exp::Negate(a) | Exp::Not(a) | Exp::Ranged { value: a, .. } => pending.push(a),
                Exp::Arithmetic(_, a, b, _) | Exp::Logic(_, a, b) => {
                    pending.extend([&**a, &**b]);
                }
                Exp::Variable(_)
                | Exp::Activation(_)
                | Exp::Capture(_)
                | Exp::Number(_)
                | Exp::Boolean(_) => {}
            }
        }
        Ok(true)
    }
}

/// `box + [0, h] * rates`, per variable.
pub(super) fn tube(bx: &Bx, h: &Rational, rates: &[Interval]) -> Bx {
    let span = Interval {
        lower: Some(zero()),
        upper: Some(h.clone()),
    };
    bx.iter()
        .zip(rates)
        .map(|(x, rate)| x.add(&span.mul(rate)))
        .collect()
}

/// `box + h * rates`, per variable.
pub(super) fn advance(bx: &Bx, h: &Rational, rates: &[Interval]) -> Bx {
    let point = Interval::point(h.clone());
    bx.iter()
        .zip(rates)
        .map(|(x, rate)| x.add(&point.mul(rate)))
        .collect()
}
