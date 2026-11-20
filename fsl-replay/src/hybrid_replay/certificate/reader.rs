// SPDX-License-Identifier: AGPL-3.0-or-later
//! The interval reading a hybrid provider builds a certificate with: the
//! contractor, guarded parts and post images, the Picard condition and the
//! narrowed jump time are the functions FSpec FR-652's checker applies, so a
//! box a provider writes with them is a box the checker recomputes. One
//! evaluation of an expression over a box is one unit of the meter the
//! reader was given.

use filament_exact::{Integer, Rational};
use fsl_semantics::check::{CheckedGraph, CheckedHybridModel, CompareOp};

use super::super::expand::Exp;
use super::super::flow::{poly_of, Flow};
use super::super::poly::Poly;
use super::super::interval::{Interval, Tri};
use super::super::{HybridReplayMeter, HybridReplayRefusal};
use super::flowpipe::narrowed_over;
use super::machine::{self, Machine, ModelX, Stop};

/// One interval per continuous variable of the model, in declaration order.
pub type HybridBox = Vec<Interval>;

/// Why a read stopped before it decided.
#[derive(Debug)]
pub enum ReadStop {
    /// The meter the reader was given stopped it.
    WorkLimit,
    /// A fault of the checked model.
    Fault(HybridReplayRefusal),
}

impl From<Stop> for ReadStop {
    fn from(stop: Stop) -> Self {
        match stop {
            Stop::WorkLimit => Self::WorkLimit,
            Stop::Fault(fault) => Self::Fault(fault),
        }
    }
}

type Read<T> = Result<T, ReadStop>;

/// A transition that can fire from a box: its guarded part and post image.
#[derive(Clone, Debug)]
pub struct Fired {
    /// `Contract(guard, X)`.
    pub guarded: HybridBox,
    /// The reset's image of the guarded part, contracted by the target
    /// invariant.
    pub post: HybridBox,
}

/// The initial states of one initial entry as a box.
#[derive(Clone, Debug)]
pub struct InitialHull {
    /// `Hull(condition and invariant)`.
    pub hull: HybridBox,
    /// Whether the condition and the invariant are definitely defined over it.
    pub defined: bool,
}

/// A checked hybrid model read in interval arithmetic.
pub struct ModelReader<'g, 'w> {
    machine: Machine<'g, 'w>,
    mx: ModelX,
}

impl<'g, 'w> ModelReader<'g, 'w> {
    /// The reader of `model`, every expression expanded.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the expansion or a call cannot be
    /// expanded.
    pub fn new(
        graph: &'g CheckedGraph,
        model: &CheckedHybridModel,
        meter: &'w mut HybridReplayMeter,
    ) -> Read<Self> {
        let mut machine = Machine::new(graph, meter);
        let mx = machine.expand_model(model)?;
        Ok(Self { machine, mx })
    }

    /// The number of continuous variables.
    pub const fn variables(&self) -> usize {
        self.mx.n
    }

    /// The number of modes.
    pub fn modes(&self) -> usize {
        self.mx.modes.len()
    }

    /// The number of transitions of `mode`.
    pub fn transitions(&self, mode: usize) -> usize {
        self.mx.modes[mode].transitions.len()
    }

    /// The target mode of transition `transition` of `mode`.
    pub fn target(&self, mode: usize, transition: usize) -> usize {
        self.mx.modes[mode].transitions[transition].target
    }

    /// The number of initial entries.
    pub fn initial_entries(&self) -> usize {
        self.mx.initial.len()
    }

    /// The mode of initial entry `entry`.
    pub fn initial_mode(&self, entry: usize) -> usize {
        self.mx.initial[entry].0
    }

    /// `Hull(condition and invariant)` of initial entry `entry`; `None` when
    /// it is empty.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn initial_hull(&mut self, entry: usize) -> Read<Option<InitialHull>> {
        let (mode, condition) = &self.mx.initial[entry];
        let invariant = &self.mx.modes[*mode].invariant;
        let n = self.mx.n;
        let Some(hull) = self.machine.hull(&[condition, invariant], n)? else {
            return Ok(None);
        };
        let defined = self.machine.defined(condition, &hull)?.is_yes()
            && self.machine.defined(invariant, &hull)?.is_yes();
        Ok(Some(InitialHull { hull, defined }))
    }

    /// Whether `point` definitely satisfies and is definitely defined for
    /// initial entry `entry` and its mode's invariant.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn satisfies_initial(&mut self, entry: usize, point: &[Rational]) -> Read<bool> {
        let bx: HybridBox = point.iter().cloned().map(Interval::point).collect();
        let (mode, condition) = &self.mx.initial[entry];
        let invariant = &self.mx.modes[*mode].invariant;
        let (c, dc) = self.machine.reading(condition, &bx)?;
        let (i, di) = self.machine.reading(invariant, &bx)?;
        Ok(c == Tri::True && i == Tri::True && dc.is_yes() && di.is_yes())
    }

    /// The truth of `mode`'s invariant over `bx` and whether it is
    /// definitely defined there.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn invariant(&mut self, mode: usize, bx: &HybridBox) -> Read<(Tri, bool)> {
        let (tri, def) = self.machine.reading(&self.mx.modes[mode].invariant, bx)?;
        Ok((tri, def.is_yes()))
    }

    /// `Contract(Inv, bx)`, `None` when empty.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn contract_invariant(&mut self, mode: usize, bx: HybridBox) -> Read<Option<HybridBox>> {
        Ok(self
            .machine
            .contract(&[&self.mx.modes[mode].invariant], bx)?)
    }

    /// The rates of `mode` over `bx`.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn rates(&mut self, mode: usize, bx: &HybridBox) -> Read<Vec<Interval>> {
        Ok(self.machine.rates(&self.mx.modes[mode], bx)?)
    }

    /// FSpec FR-652 rule `picard` for one step: every rate and the
    /// invariant definitely defined over `ap`, and the (contracted, for a
    /// contraction-admitted mode) tube of `bx` over `h` inside `ap`.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn picard_holds(
        &mut self,
        mode: usize,
        bx: &HybridBox,
        ap: &HybridBox,
        h: &Rational,
    ) -> Read<bool> {
        Ok(self
            .machine
            .picard_holds(&self.mx.modes[mode], bx, ap, h)?)
    }

    /// Whether every rate and the invariant of `mode` are definitely
    /// defined over `ap`.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn flow_defined(&mut self, mode: usize, ap: &HybridBox) -> Read<bool> {
        let x = &self.mx.modes[mode];
        for exp in x.rates.iter().chain(std::iter::once(&x.invariant)) {
            if !self.machine.defined(exp, ap)?.is_yes() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether every conditional of every rate of `mode` has its condition
    /// strictly decided over `ap` and, with `taken_only`, only the branch
    /// taken is read (FSpec FR-652 `witness-unique`).
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn rates_strictly_decided(
        &mut self,
        mode: usize,
        ap: &HybridBox,
        taken_only: bool,
    ) -> Read<bool> {
        for rate in &self.mx.modes[mode].rates {
            if !self.machine.strictly_decided(rate, ap, taken_only)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The advance of `bx` over `h` by the rates over `ap`, contracted by
    /// `mode`'s invariant; `None` when empty.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn next_box(
        &mut self,
        mode: usize,
        bx: &HybridBox,
        ap: &HybridBox,
        h: &Rational,
    ) -> Read<Option<HybridBox>> {
        let x = &self.mx.modes[mode];
        let rates = self.machine.rates(x, ap)?;
        let advanced = machine::advance(bx, h, &rates);
        Ok(self.machine.contract(&[&x.invariant], advanced)?)
    }

    /// Whether `guard` of transition `transition` of `mode` is definitely
    /// defined over `bx`.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn guard_defined(&mut self, mode: usize, transition: usize, bx: &HybridBox) -> Read<bool> {
        let guard = &self.mx.modes[mode].transitions[transition].guard;
        Ok(self.machine.defined(guard, bx)?.is_yes())
    }

    /// The truth of the guard of transition `transition` of `mode` over
    /// `bx`, `Unknown` unless definitely defined there.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn guard_reading(
        &mut self,
        mode: usize,
        transition: usize,
        bx: &HybridBox,
    ) -> Read<Tri> {
        let guard = &self.mx.modes[mode].transitions[transition].guard;
        Ok(self.machine.reading(guard, bx)?.0)
    }

    /// Whether every reset of the transition is definitely defined over
    /// `guarded`.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn resets_defined(
        &mut self,
        mode: usize,
        transition: usize,
        guarded: &HybridBox,
    ) -> Read<bool> {
        for (_, _, reset) in &self.mx.modes[mode].transitions[transition].resets {
            if !self.machine.defined(reset, guarded)?.is_yes() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The guarded part and post image of the transition when it can fire
    /// from `bx` (FSpec FR-652 "Guards").
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn fire(&mut self, mode: usize, transition: usize, bx: &HybridBox) -> Read<Option<Fired>> {
        let x = &self.mx.modes[mode].transitions[transition];
        let target = &self.mx.modes[x.target].invariant;
        Ok(self
            .machine
            .fire(x, bx, target)?
            .map(|(guarded, post)| Fired { guarded, post }))
    }

    /// FSpec FR-652's narrowed jump time of the transition from a dwell of
    /// `mode` whose first box is `first` and whose first step's start
    /// instants are `starts`: `None` when it does not apply, and
    /// `Some(None)` when it applies and is empty.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn narrowed_time(
        &mut self,
        mode: usize,
        transition: usize,
        first: &HybridBox,
        starts: &(Rational, Rational),
        horizon: &Rational,
    ) -> Read<Option<Option<Interval>>> {
        Ok(narrowed_over(
            &mut self.machine,
            &self.mx,
            mode,
            transition,
            first,
            starts,
            horizon,
        )?)
    }

    /// The solution of `mode`'s flow from the point `start` as one
    /// polynomial in time per variable, lowest coefficient first; `None`
    /// when the flow is not a polynomial flow (its rates depend on one
    /// another cyclically).
    pub fn polynomial_solution(&self, mode: usize, start: &[Rational]) -> Option<Vec<Vec<Rational>>> {
        let flow = Flow::of(self.mx.modes[mode].rates.clone())?;
        Some(
            flow.solve(start)
                .iter()
                .map(|poly| poly.coefficients().to_vec())
                .collect(),
        )
    }
}

/// `bx + [0, h] * rates` per variable.
pub fn box_tube(bx: &HybridBox, h: &Rational, rates: &[Interval]) -> HybridBox {
    machine::tube(bx, h, rates)
}

/// `bx + h * rates` per variable.
pub fn box_advance(bx: &HybridBox, h: &Rational, rates: &[Interval]) -> HybridBox {
    machine::advance(bx, h, rates)
}

/// Whether `a` lies in `b`.
pub fn box_within(a: &[Interval], b: &[Interval]) -> bool {
    machine::within(a, b)
}

/// The interval hull of two boxes.
pub fn box_hull(a: &[Interval], b: &[Interval]) -> HybridBox {
    machine::hull_boxes(a, b)
}

fn root_of(value: &Integer) -> Option<Integer> {
    let root = Integer::from_big(value.as_big().sqrt());
    (root.mul(&root) == *value).then_some(root)
}

fn rational_sqrt(value: &Rational) -> Option<Rational> {
    if value.numerator().is_negative() {
        return None;
    }
    Rational::new(root_of(value.numerator())?, root_of(value.denominator())?).ok()
}

/// The rational roots of the polynomial with `coefficients` (lowest first):
/// 0 for the zero polynomial, none for a nonzero constant, and for degree 1
/// and 2 the roots that are rational; `None` above degree 2.
fn rational_roots(coefficients: &[Rational]) -> Option<Vec<Rational>> {
    let mut c = coefficients.to_vec();
    while c.last().is_some_and(Rational::is_zero) {
        c.pop();
    }
    let zero = Rational::from_integer(Integer::zero());
    match c.len() {
        0 => Some(vec![zero]),
        1 => Some(Vec::new()),
        2 => Some(vec![c[0].neg().div(&c[1])?]),
        3 => {
            let four = Rational::from_integer(Integer::from(4_i64));
            let two = Rational::from_integer(Integer::from(2_i64));
            let discriminant = c[1].mul(&c[1]).sub(&four.mul(&c[2]).mul(&c[0]));
            let Some(root) = rational_sqrt(&discriminant) else {
                return Some(Vec::new());
            };
            let scale = two.mul(&c[2]);
            let mut roots = vec![c[1].neg().add(&root).div(&scale)?];
            if !root.is_zero() {
                roots.push(c[1].neg().sub(&root).div(&scale)?);
            }
            Some(roots)
        }
        _ => None,
    }
}

fn comparisons<'e>(exp: &'e Exp, out: &mut Vec<(&'e Exp, &'e Exp)>) {
    match exp {
        Exp::Compare(_, a, b) => out.push((a, b)),
        Exp::Negate(a) | Exp::Not(a) | Exp::Ranged { value: a, .. } => comparisons(a, out),
        Exp::Arithmetic(_, a, b, _) | Exp::Logic(_, a, b) => {
            comparisons(a, out);
            comparisons(b, out);
        }
        Exp::Conditional(c, a, b, _) => {
            comparisons(c, out);
            comparisons(a, out);
            comparisons(b, out);
        }
        _ => {}
    }
}

impl ModelReader<'_, '_> {
    /// The rational instants `t >= 0`, sorted, at which a comparison of the
    /// guard of transition `transition` of `mode` has a zero difference
    /// along `solution` (one polynomial in time per variable, lowest
    /// coefficient first): the instants at which the guard can change its
    /// truth. `None` when a difference has degree above 2 or is not a
    /// polynomial in time.
    pub fn guard_instants(
        &self,
        mode: usize,
        transition: usize,
        solution: &[Vec<Rational>],
    ) -> Option<Vec<Rational>> {
        let solved: Vec<Poly> = solution.iter().cloned().map(Poly::new).collect();
        let guard = &self.mx.modes[mode].transitions[transition].guard;
        let mut atoms = Vec::new();
        comparisons(guard, &mut atoms);
        let mut instants: Vec<Rational> = Vec::new();
        for (a, b) in atoms {
            let difference = poly_of(a, &solved)?.sub(&poly_of(b, &solved)?);
            for root in rational_roots(difference.coefficients())? {
                if !root.numerator().is_negative() && !instants.contains(&root) {
                    instants.push(root);
                }
            }
        }
        instants.sort();
        Some(instants)
    }

    /// The state a jump over transition `transition` of `mode` reaches from
    /// the point `pre`, when every reset conjunct is `x = e` with `e` a point
    /// over `pre`; `None` otherwise or when a reset is not defined there.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn reset_point(
        &mut self,
        mode: usize,
        transition: usize,
        pre: &[Rational],
    ) -> Read<Option<Vec<Rational>>> {
        let bx: HybridBox = pre.iter().cloned().map(Interval::point).collect();
        let x = &self.mx.modes[mode].transitions[transition];
        let mut post = pre.to_vec();
        for (variable, relation, value) in &x.resets {
            if *relation != CompareOp::Eq || !self.machine.defined(value, &bx)?.is_yes() {
                return Ok(None);
            }
            let found = self.machine.num(value, &bx)?;
            match (found.lower, found.upper) {
                (Some(low), Some(high)) if low == high => post[*variable] = low,
                _ => return Ok(None),
            }
        }
        Ok(Some(post))
    }

    /// The post-jump box a witness jump from `bx` over transition
    /// `transition` of `mode` needs: each `x = e` variable takes the value
    /// of `e` over `bx`, every other variable keeps its interval in `bx`;
    /// `None` when a reset is not an `=` conjunct or not defined over `bx`.
    ///
    /// # Errors
    ///
    /// [`ReadStop`] when the meter stops the read.
    pub fn reset_box(
        &mut self,
        mode: usize,
        transition: usize,
        bx: &HybridBox,
    ) -> Read<Option<HybridBox>> {
        let x = &self.mx.modes[mode].transitions[transition];
        let mut post = bx.clone();
        for (variable, relation, value) in &x.resets {
            if *relation != CompareOp::Eq || !self.machine.defined(value, bx)?.is_yes() {
                return Ok(None);
            }
            post[*variable] = self.machine.num(value, bx)?;
        }
        Ok(Some(post))
    }
}
