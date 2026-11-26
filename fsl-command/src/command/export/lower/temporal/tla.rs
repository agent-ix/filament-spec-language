// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-813: temporal clauses and fairness in TLA+.
//!
//! A clause built from `always`, `eventually` and Boolean connectives over
//! state formulas is `[]` and `<>` directly. A past operator becomes an
//! auxiliary variable: a Boolean for an unbounded operator, a saturating
//! counter or a delay register for a lower-bounded one, a shift register for
//! a closed interval. `until`, `release` and an interval operator at the
//! root of a clause become a deterministic monitor. A clause with no
//! deterministic monitor can use the bounded future tableau in
//! [`tableau`].
//!
//! Every temporal-level range is the model's constant identity universe
//! `Ids_<T>`, never `DOMAIN v`, which varies with the state; `ok_<clause>`
//! states that every read is defined. The step counter and the histories advance on
//! every step of `Next_<clause>`, which includes the terminal step, so both
//! targets count FSL's terminal stutter positions.

use super::{ClauseLow, Formulas, Iv, Tf, TfId, CONSTRUCT};
use crate::command::export::lower::{
    bullets, define, mangle, parameters, refusal, render, scalars, snapshot_rows, tla_domain,
    Definition, Input, Lower, Object, Operation, Piece, Refusal, Target,
};
use filament_walk::{walk, Children, Walk};
use std::collections::BTreeSet;
use std::ops::ControlFlow;

mod tableau;

/// A condition of a `conditional` record (FSpec FR-298).
pub(in crate::command::export::lower) struct Cond {
    pub(in crate::command::export::lower) obligation: String,
    pub(in crate::command::export::lower) kind: &'static str,
    /// The node the condition names: the clause or the object type.
    pub(in crate::command::export::lower) node: usize,
    pub(in crate::command::export::lower) type_name: Option<String>,
}

/// What the TLA+ lowering adds around the object state.
#[derive(Default)]
pub(in crate::command::export::lower) struct Extra {
    /// The variables the clauses add.
    pub(in crate::command::export::lower) variables: Vec<String>,
    /// The identity universes the model-checker configuration supplies.
    pub(in crate::command::export::lower) constants: Vec<String>,
    /// Each obligation whose TLA+ encoding is a nondeterministic monitor,
    /// with its clause's node.
    pub(in crate::command::export::lower) conditional: Vec<Cond>,
    pub(in crate::command::export::lower) dense: bool,
    pub(in crate::command::export::lower) aux_names: Vec<(String, usize)>,
}

// ---- Normal form -----------------------------------------------------------

/// `historically` and `triggered` over an interval as the duals of `once`
/// and `since`; the unbounded forms keep their own history. The result is a
/// new arena, built in one pass over the operands-first order of `source`.
fn normalize(source: &Formulas, root: TfId) -> Lower<(Formulas, TfId)> {
    let mut formulas = Formulas::default();
    // The normal form of each source node, by its arena position.
    let mut done: Vec<TfId> = Vec::new();
    let of = |done: &[TfId], id: TfId| {
        done.get(id.index())
            .copied()
            .ok_or_else(|| refusal(CONSTRUCT, "a formula operand is not in the formula"))
    };
    for (_, tf) in source.iter() {
        let normal = match tf {
            Tf::Const(_) | Tf::Atom(_) | Tf::Hist(_) => tf.clone(),
            Tf::Timed(window) => {
                let mut window = window.clone();
                window.predicate = of(&done, window.predicate)?;
                Tf::Timed(window)
            }
            &Tf::Not(inner) => Tf::Not(of(&done, inner)?),
            &Tf::And(left, right) => Tf::And(of(&done, left)?, of(&done, right)?),
            &Tf::Or(left, right) => Tf::Or(of(&done, left)?, of(&done, right)?),
            &Tf::Implies(left, right) => Tf::Implies(of(&done, left)?, of(&done, right)?),
            &Tf::Always(iv, inner) => Tf::Always(iv, of(&done, inner)?),
            &Tf::Eventually(iv, inner) => Tf::Eventually(iv, of(&done, inner)?),
            &Tf::Once(iv, inner) => Tf::Once(iv, of(&done, inner)?),
            &Tf::Until(iv, left, right) => Tf::Until(iv, of(&done, left)?, of(&done, right)?),
            &Tf::Release(iv, left, right) => Tf::Release(iv, of(&done, left)?, of(&done, right)?),
            &Tf::Since(iv, left, right) => Tf::Since(iv, of(&done, left)?, of(&done, right)?),
            &Tf::Historically(iv, inner) if !iv.is_some() => {
                Tf::Historically(iv, of(&done, inner)?)
            }
            &Tf::Triggered(iv, left, right) if !iv.is_some() => {
                Tf::Triggered(iv, of(&done, left)?, of(&done, right)?)
            }
            &Tf::Historically(iv, inner) => {
                let inner = formulas.add(Tf::Not(of(&done, inner)?));
                Tf::Not(formulas.add(Tf::Once(iv, inner)))
            }
            &Tf::Triggered(iv, left, right) => {
                let left = formulas.add(Tf::Not(of(&done, left)?));
                let right = formulas.add(Tf::Not(of(&done, right)?));
                Tf::Not(formulas.add(Tf::Since(iv, left, right)))
            }
        };
        done.push(formulas.add(normal));
    }
    let root = of(&done, root)?;
    Ok((formulas, root))
}

// ---- Histories -------------------------------------------------------------

enum HKind {
    /// `once p`: a Boolean `h`.
    Once { p: TfId },
    /// `p since q`: a Boolean `h`.
    Since { p: TfId, q: TfId },
    /// `historically p`: a Boolean `h` of `p` at every position so far.
    Historically { p: TfId },
    /// `p triggered q`: a Boolean `h`.
    Triggered { p: TfId, q: TfId },
    /// `once[a,*] p`: a counter `c` of the steps since `p` first held,
    /// saturating at `a`.
    OnceFrom { p: TfId, a: u64 },
    /// `p since[a,*] q`: a register `r` of length `a` over the Boolean of
    /// the unbounded `p since q`.
    SinceFrom { inner: usize, a: u64 },
    /// `once[a,b] p`: a shift register `r` of the last `b` values of `p`.
    OnceWithin { p: TfId, a: u64, b: u64 },
    /// `p since[a,b] q`: registers `r` over `q` and `r + 1` over `p`.
    SinceWithin { p: TfId, q: TfId, a: u64, b: u64 },
}

struct History {
    kind: HKind,
    /// The ordinal of the operator among the clause's past operators, in
    /// pre-order; a `since` takes two.
    ord: u64,
    /// The value at every position before the origin: what a closed
    /// interval reads there.
    before: bool,
    /// The value of the operand `p` and `q` before the origin.
    before_p: bool,
    before_q: bool,
}

/// An extracted formula and the value it has at every position before the
/// origin, where each atom is false.
#[derive(Clone, Copy)]
struct Extracted {
    tf: TfId,
    before: bool,
}

/// The walk that replaces each past operator by a history; `done` holds the
/// operands not yet taken.
struct Extract<'a> {
    formulas: &'a mut Formulas,
    histories: Vec<History>,
    ordinal: u64,
    done: Vec<Extracted>,
}

impl Extract<'_> {
    fn operand(&mut self) -> Lower<Extracted> {
        self.done
            .pop()
            .ok_or_else(|| refusal(CONSTRUCT, "a temporal operator lost an operand"))
    }

    /// The operands of a binary operator, left first.
    fn operands(&mut self) -> Lower<(Extracted, Extracted)> {
        let right = self.operand()?;
        let left = self.operand()?;
        Ok((left, right))
    }

    fn push(&mut self, tf: Tf, before: bool) {
        let tf = self.formulas.add(tf);
        self.done.push(Extracted { tf, before });
    }

    /// Name the history last added as the value of its operator.
    fn push_history(&mut self) -> Lower<()> {
        let index = self.histories.len().saturating_sub(1);
        let before = self
            .histories
            .get(index)
            .ok_or_else(|| refusal(CONSTRUCT, "a history that does not exist"))?
            .before;
        self.push(Tf::Hist(index), before);
        Ok(())
    }

    /// The operand of a past operator must read no future position.
    fn past_state(&self, operand: Extracted) -> Lower<TfId> {
        if self.formulas.facts(operand.tf).state {
            Ok(operand.tf)
        } else {
            refuse_internal("a past operator over a formula that holds a future operator")
        }
    }

    fn history(&mut self, tf: &Tf, ord: u64) -> Lower<()> {
        match tf {
            &Tf::Once(iv, _) => {
                let operand = self.operand()?;
                let p = self.past_state(operand)?;
                let before_p = operand.before;
                let (kind, before) = match iv.bounds() {
                    (0, None) => (HKind::Once { p }, false),
                    (a, None) => (HKind::OnceFrom { p, a }, false),
                    (a, Some(b)) => (HKind::OnceWithin { p, a, b }, before_p),
                };
                self.histories.push(History {
                    kind,
                    ord,
                    before,
                    before_p,
                    before_q: false,
                });
            }
            &Tf::Historically(..) => {
                let operand = self.operand()?;
                let p = self.past_state(operand)?;
                self.histories.push(History {
                    kind: HKind::Historically { p },
                    ord,
                    before: true,
                    before_p: operand.before,
                    before_q: false,
                });
            }
            &Tf::Since(iv, ..) => {
                let (left, right) = self.operands()?;
                let (p, q) = (self.past_state(left)?, self.past_state(right)?);
                let (before_p, before_q) = (left.before, right.before);
                match iv.bounds() {
                    (a, Some(b)) => self.histories.push(History {
                        kind: HKind::SinceWithin { p, q, a, b },
                        ord,
                        before: before_q,
                        before_p,
                        before_q,
                    }),
                    (a, None) => {
                        self.histories.push(History {
                            kind: HKind::Since { p, q },
                            ord,
                            before: false,
                            before_p,
                            before_q,
                        });
                        if a > 0 {
                            let inner = self.histories.len() - 1;
                            self.histories.push(History {
                                kind: HKind::SinceFrom { inner, a },
                                ord,
                                before: false,
                                before_p,
                                before_q,
                            });
                        }
                    }
                }
            }
            &Tf::Triggered(..) => {
                let (left, right) = self.operands()?;
                let (p, q) = (self.past_state(left)?, self.past_state(right)?);
                self.histories.push(History {
                    kind: HKind::Triggered { p, q },
                    ord,
                    before: true,
                    before_p: left.before,
                    before_q: right.before,
                });
            }
            _ => return refuse_internal("a history for no past operator"),
        }
        self.push_history()
    }

    fn rebuild(&mut self, tf: Tf, ord: u64) -> Lower<()> {
        match tf {
            Tf::Const(value) => self.push(tf, value),
            Tf::Hist(index) => {
                let before = self
                    .histories
                    .get(index)
                    .is_some_and(|history| history.before);
                self.push(tf, before);
            }
            Tf::Atom(_) | Tf::Timed(_) => self.push(tf, false),
            Tf::Not(_) => {
                let inner = self.operand()?;
                self.push(Tf::Not(inner.tf), !inner.before);
            }
            Tf::And(..) => {
                let (left, right) = self.operands()?;
                self.push(Tf::And(left.tf, right.tf), left.before && right.before);
            }
            Tf::Or(..) => {
                let (left, right) = self.operands()?;
                self.push(Tf::Or(left.tf, right.tf), left.before || right.before);
            }
            Tf::Implies(..) => {
                let (left, right) = self.operands()?;
                self.push(Tf::Implies(left.tf, right.tf), !left.before || right.before);
            }
            Tf::Always(iv, _) => {
                let inner = self.operand()?;
                self.push(Tf::Always(iv, inner.tf), false);
            }
            Tf::Eventually(iv, _) => {
                let inner = self.operand()?;
                self.push(Tf::Eventually(iv, inner.tf), false);
            }
            Tf::Until(iv, ..) => {
                let (left, right) = self.operands()?;
                self.push(Tf::Until(iv, left.tf, right.tf), false);
            }
            Tf::Release(iv, ..) => {
                let (left, right) = self.operands()?;
                self.push(Tf::Release(iv, left.tf, right.tf), false);
            }
            Tf::Once(..) | Tf::Since(..) | Tf::Historically(..) | Tf::Triggered(..) => {
                self.history(&tf, ord)?;
            }
        }
        Ok(())
    }
}

impl Walk for Extract<'_> {
    type Node = TfId;
    /// The node and, for a past operator, its ordinal.
    type Frame = (TfId, u64);
    type Stop = Refusal;

    fn enter(
        &mut self,
        id: TfId,
        children: &mut Children<'_, TfId>,
    ) -> ControlFlow<Refusal, (TfId, u64)> {
        let ord = self.ordinal;
        match self.formulas.get(id) {
            Err(refusal) => return ControlFlow::Break(refusal),
            Ok(tf) => match tf {
                Tf::Const(_) | Tf::Atom(_) | Tf::Hist(_) | Tf::Timed(_) => {}
                &Tf::Not(inner) | &Tf::Always(_, inner) | &Tf::Eventually(_, inner) => {
                    children.push(inner);
                }
                &Tf::And(left, right)
                | &Tf::Or(left, right)
                | &Tf::Implies(left, right)
                | &Tf::Until(_, left, right)
                | &Tf::Release(_, left, right) => children.extend([left, right]),
                &Tf::Once(_, inner) => {
                    self.ordinal += 1;
                    children.push(inner);
                }
                &Tf::Historically(iv, inner) if !iv.is_some() => {
                    self.ordinal += 1;
                    children.push(inner);
                }
                &Tf::Since(_, left, right) => {
                    self.ordinal += 2;
                    children.extend([left, right]);
                }
                &Tf::Triggered(iv, left, right) if !iv.is_some() => {
                    self.ordinal += 2;
                    children.extend([left, right]);
                }
                Tf::Historically(..) | Tf::Triggered(..) => {
                    return ControlFlow::Break(refusal(
                        CONSTRUCT,
                        "a bounded past operator that normalization leaves",
                    ));
                }
            },
        }
        ControlFlow::Continue((id, ord))
    }

    fn exit(&mut self, (id, ord): (TfId, u64)) -> ControlFlow<Refusal> {
        let rebuilt = self
            .formulas
            .get(id)
            .cloned()
            .and_then(|tf| self.rebuild(tf, ord));
        match rebuilt {
            Ok(()) => ControlFlow::Continue(()),
            Err(refusal) => ControlFlow::Break(refusal),
        }
    }
}

/// Replace each past operator under `root` by a history.
fn extract(formulas: &mut Formulas, root: TfId) -> Lower<(TfId, Vec<History>)> {
    let mut extract = Extract {
        formulas,
        histories: Vec::new(),
        ordinal: 0,
        done: Vec::new(),
    };
    if let ControlFlow::Break(refusal) = walk(&mut extract, root) {
        return Err(refusal);
    }
    let root = extract.operand()?.tf;
    Ok((root, extract.histories))
}

fn refuse_internal<T>(detail: &str) -> Lower<T> {
    Err(refusal(CONSTRUCT, detail))
}

// ---- Shape -----------------------------------------------------------------

#[derive(Clone, Copy, Eq, PartialEq)]
enum MonOp {
    Until,
    Eventually,
    Release,
    Always,
}

impl MonOp {
    /// Whether the monitor accepts once it settles on `acc`.
    fn reaches(self) -> bool {
        matches!(self, Self::Until | Self::Eventually)
    }
}

struct Monitor {
    op: MonOp,
    iv: Iv,
    left: TfId,
    right: Option<TfId>,
}

/// One conjunct of a clause: a formula `[]` and `<>` state directly or a
/// deterministic monitor.
enum Part {
    Direct(TfId),
    Mon(Monitor),
}

enum Shape {
    Direct,
    Monitors(Vec<Part>),
    Tableau,
}

/// The conjuncts of `root`, left to right.
fn conjuncts(formulas: &Formulas, root: TfId) -> Lower<Vec<TfId>> {
    let mut out = Vec::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        match formulas.get(id)? {
            &Tf::And(left, right) => pending.extend([right, left]),
            _ => out.push(id),
        }
    }
    Ok(out)
}

/// The deterministic monitor of `tf`, when it has one.
fn monitor_of(formulas: &Formulas, tf: TfId) -> Lower<Option<Monitor>> {
    let (op, left, right, iv) = match formulas.get(tf)? {
        &Tf::Until(iv, left, right) => (MonOp::Until, left, Some(right), iv),
        &Tf::Release(iv, left, right) => (MonOp::Release, left, Some(right), iv),
        &Tf::Always(iv, inner) if iv.is_some() => (MonOp::Always, inner, None, iv),
        &Tf::Eventually(iv, inner) if iv.is_some() => (MonOp::Eventually, inner, None, iv),
        _ => return Ok(None),
    };
    let state = |id| formulas.facts(id).state;
    if !state(left) || right.is_some_and(|right| !state(right)) {
        return Ok(None);
    }
    Ok(Some(Monitor {
        op,
        iv,
        left,
        right,
    }))
}

fn shape_of(formulas: &Formulas, root: TfId) -> Lower<Shape> {
    if formulas.facts(root).direct {
        return Ok(Shape::Direct);
    }
    let mut parts = Vec::new();
    for leaf in conjuncts(formulas, root)? {
        if formulas.facts(leaf).direct {
            parts.push(Part::Direct(leaf));
        } else if let Some(monitor) = monitor_of(formulas, leaf)? {
            parts.push(Part::Mon(monitor));
        } else {
            return Ok(Shape::Tableau);
        }
    }
    Ok(Shape::Monitors(parts))
}

// ---- Rendering -------------------------------------------------------------

struct Names {
    monitor: String,
    counter: String,
    vars: String,
    type_ok: String,
    init: String,
    mon: String,
    next: String,
    spec: String,
    accept: String,
    ok: String,
    theorem: String,
    ids: String,
}

impl Names {
    fn new(name: &str, object: &Object) -> Self {
        Self {
            monitor: mangle("m_", name),
            counter: mangle("n_", name),
            vars: mangle("vars_", name),
            type_ok: mangle("TypeOK_", name),
            init: mangle("Init_", name),
            mon: mangle("Mon_", name),
            next: mangle("Next_", name),
            spec: mangle("Spec_", name),
            accept: mangle("accept_", name),
            ok: mangle("ok_", name),
            theorem: mangle("Th_", name),
            ids: mangle("Ids_", &object.name),
        }
    }

    /// `m_<clause>_<letter><ordinal>`: a monitor and history variable.
    fn variable(&self, letter: char, ordinal: u64) -> String {
        format!("{}_{letter}{ordinal}", self.monitor)
    }

    /// The step counter of the `index`th interval operator.
    fn counter(&self, index: usize) -> String {
        format!("{}_{index}", self.counter)
    }

    /// The monitor variable and acceptance definition of the `index`th
    /// monitor of a clause, unnumbered when it has one.
    fn monitor_names(&self, index: usize, multi: bool) -> (String, String) {
        if multi {
            (
                format!("{}_{index}", self.monitor),
                format!("{}_{index}", self.accept),
            )
        } else {
            (self.monitor.clone(), self.accept.clone())
        }
    }
}

/// An auxiliary variable of a clause: its type and its initial and updated
/// values.
struct Var {
    name: String,
    ty: String,
    init: Option<String>,
    update: Option<String>,
}

/// A text to write for one clause.
#[derive(Clone, Copy)]
enum Say {
    /// A state formula at the current or the next state.
    State(TfId, bool),
    /// A formula whose temporal operators are `[]` and `<>`.
    Direct(TfId),
    /// The value of a history at the current or the next position.
    Hist(usize, bool),
}

struct Ctx<'a> {
    input: &'a Input<'a>,
    clause: &'a ClauseLow,
    /// The formulas the ids of this lowering name.
    formulas: &'a Formulas,
    object: &'a Object,
    names: Names,
    histories: Vec<History>,
}

impl Ctx<'_> {
    fn key(&self) -> &str {
        &self.clause.binder
    }

    /// `body` for every identity present at the origin.
    fn guarded(&self, body: &str) -> String {
        format!(
            "\\A {key} \\in {ids} : []({ok}({key})) /\\ {body}",
            key = self.key(),
            ids = self.names.ids,
            ok = self.names.ok,
        )
    }

    /// A function over the identity universe.
    fn per_key(&self, body: &str) -> String {
        format!(
            "[{key} \\in {ids} |-> {body}]",
            key = self.key(),
            ids = self.names.ids,
        )
    }

    fn atom(&self, index: usize, primed: bool) -> Lower<String> {
        let atoms = if primed {
            &self.clause.next
        } else {
            &self.clause.cur
        };
        let &e = atoms
            .get(index)
            .ok_or_else(|| refusal(CONSTRUCT, "an atom with no lowered operand"))?;
        self.input.show.expr(e)
    }

    /// A state formula at the current or the next state.
    fn state(&self, tf: TfId, primed: bool) -> Lower<String> {
        self.say(Say::State(tf, primed))
    }

    fn say(&self, root: Say) -> Lower<String> {
        render(root, |say, out| match say {
            Say::State(tf, primed) => self.state_pieces(tf, primed, out),
            Say::Direct(tf) => self.direct_pieces(tf, out),
            Say::Hist(index, primed) => self.hist_pieces(index, primed, out),
        })
    }

    fn state_pieces(&self, tf: TfId, primed: bool, out: &mut Vec<Piece<Say>>) -> Lower<()> {
        let state = |id| Piece::Node(Say::State(id, primed));
        match self.formulas.get(tf)? {
            Tf::Const(true) => out.push(Piece::text("TRUE")),
            Tf::Const(false) => out.push(Piece::text("FALSE")),
            &Tf::Atom(index) => out.push(Piece::text(self.atom(index, primed)?)),
            &Tf::Hist(index) => out.push(Piece::Node(Say::Hist(index, primed))),
            &Tf::Not(inner) => out.extend([Piece::text("~("), state(inner), Piece::text(")")]),
            &Tf::And(left, right) => binary(out, state(left), " /\\ ", state(right)),
            &Tf::Or(left, right) => binary(out, state(left), " \\/ ", state(right)),
            &Tf::Implies(left, right) => binary(out, state(left), " => ", state(right)),
            _ => {
                return Err(refusal(
                    CONSTRUCT,
                    "a temporal operator read as a state formula",
                ));
            }
        }
        Ok(())
    }

    /// The variable of a history that holds a Boolean, a counter or a
    /// register.
    fn variable(&self, history: &History) -> String {
        let letter = match history.kind {
            HKind::Once { .. }
            | HKind::Since { .. }
            | HKind::Historically { .. }
            | HKind::Triggered { .. } => 'h',
            HKind::OnceFrom { .. } => 'c',
            HKind::SinceFrom { .. } | HKind::OnceWithin { .. } | HKind::SinceWithin { .. } => 'r',
        };
        self.names.variable(letter, history.ord)
    }

    /// The value of the history at the current or the next position.
    fn hist_value(&self, index: usize, primed: bool) -> Lower<String> {
        self.say(Say::Hist(index, primed))
    }

    fn hist_pieces(&self, index: usize, primed: bool, out: &mut Vec<Piece<Say>>) -> Lower<()> {
        let history = self
            .histories
            .get(index)
            .ok_or_else(|| refusal(CONSTRUCT, "a history that does not exist"))?;
        let name = self.variable(history);
        let tick = if primed { "'" } else { "" };
        let key = self.key();
        let state = |id| Piece::Node(Say::State(id, primed));
        match history.kind {
            HKind::Once { .. }
            | HKind::Since { .. }
            | HKind::Historically { .. }
            | HKind::Triggered { .. } => out.push(Piece::text(format!("{name}{tick}[{key}]"))),
            HKind::OnceFrom { a, .. } => {
                out.push(Piece::text(format!("{name}{tick}[{key}] = {a}")));
            }
            HKind::SinceFrom { a, .. } => {
                out.push(Piece::text(format!("{name}{tick}[{key}][{a}]")));
            }
            HKind::OnceWithin { p, a, b } => {
                out.push(Piece::text("("));
                if a == 0 {
                    out.push(state(p));
                }
                let lower = a.max(1);
                if lower <= b {
                    if a == 0 {
                        out.push(Piece::text(" \\/ "));
                    }
                    out.push(Piece::text(format!(
                        "(\\E k \\in {lower}..{b} : {name}{tick}[{key}][k])"
                    )));
                }
                out.push(Piece::text(")"));
            }
            HKind::SinceWithin { p, q, a, b } => {
                if b == 0 {
                    out.push(state(q));
                    return Ok(());
                }
                let p_name = self.names.variable('r', history.ord + 1);
                out.extend([
                    Piece::text(format!("(\\E d \\in {a}..{b} : (IF d = 0 THEN ")),
                    state(q),
                    Piece::text(format!(
                        " ELSE {name}{tick}[{key}][d]) /\\ (\\A e \\in {a}..(d - 1) : IF e = 0 THEN "
                    )),
                    state(p),
                    Piece::text(format!(" ELSE {p_name}{tick}[{key}][e]))")),
                ]);
            }
        }
        Ok(())
    }

    /// The auxiliary variables the histories need.
    fn history_vars(&self) -> Lower<Vec<Var>> {
        let ids = &self.names.ids;
        let key = self.key();
        let mut vars = Vec::new();
        let function = |ty: &str| format!("[{ids} -> {ty}]");
        let register = |size: u64, value: &str| format!("[k \\in 1..{size} |-> {value}]");
        let mut order: Vec<usize> = (0..self.histories.len()).collect();
        order.sort_by_key(|&index| (std::cmp::Reverse(self.histories[index].ord), index));
        for history in order.into_iter().map(|index| &self.histories[index]) {
            let name = self.variable(history);
            match history.kind {
                HKind::Once { p } => vars.push(Var {
                    ty: function("BOOLEAN"),
                    init: Some(self.per_key(&self.state(p, false)?)),
                    update: Some(self.per_key(&format!(
                        "({} \\/ {name}[{key}])",
                        self.state(p, true)?
                    ))),
                    name,
                }),
                HKind::Historically { p } => vars.push(Var {
                    ty: function("BOOLEAN"),
                    init: Some(self.per_key(&self.state(p, false)?)),
                    update: Some(self.per_key(&format!(
                        "({} /\\ {name}[{key}])",
                        self.state(p, true)?
                    ))),
                    name,
                }),
                HKind::Triggered { p, q } => vars.push(Var {
                    ty: function("BOOLEAN"),
                    init: Some(self.per_key(&self.state(q, false)?)),
                    update: Some(self.per_key(&format!(
                        "({} /\\ ({} \\/ {name}[{key}]))",
                        self.state(q, true)?,
                        self.state(p, true)?
                    ))),
                    name,
                }),
                HKind::Since { p, q } => vars.push(Var {
                    ty: function("BOOLEAN"),
                    init: Some(self.per_key(&self.state(q, false)?)),
                    update: Some(self.per_key(&format!(
                        "({} \\/ ({} /\\ {name}[{key}]))",
                        self.state(q, true)?,
                        self.state(p, true)?
                    ))),
                    name,
                }),
                HKind::OnceFrom { p, a } => vars.push(Var {
                    ty: function(&format!("-1..{a}")),
                    init: Some(self.per_key(&format!(
                        "IF {} THEN 0 ELSE -1",
                        self.state(p, false)?
                    ))),
                    update: Some(self.per_key(&format!(
                        "IF {name}[{key}] = -1 THEN (IF {} THEN 0 ELSE -1) ELSE IF {name}[{key}] < {a} THEN {name}[{key}] + 1 ELSE {a}",
                        self.state(p, true)?
                    ))),
                    name,
                }),
                HKind::SinceFrom { inner, a } => {
                    let source = self.hist_value(inner, false)?;
                    vars.push(Var {
                        ty: function(&format!("[1..{a} -> BOOLEAN]")),
                        init: Some(self.per_key(&register(a, "FALSE"))),
                        update: Some(self.per_key(&register(
                            a,
                            &format!("IF k = 1 THEN {source} ELSE {name}[{key}][k - 1]"),
                        ))),
                        name,
                    });
                }
                HKind::OnceWithin { p, b, .. } if b >= 1 => {
                    vars.push(self.register_var(&name, p, b, history.before_p)?);
                }
                HKind::SinceWithin { p, q, b, .. } if b >= 1 => {
                    vars.push(self.register_var(&name, q, b, history.before_q)?);
                    vars.push(self.register_var(
                        &self.names.variable('r', history.ord + 1),
                        p,
                        b,
                        history.before_p,
                    )?);
                }
                HKind::OnceWithin { .. } | HKind::SinceWithin { .. } => {}
            }
        }
        Ok(vars)
    }

    /// A shift register of the last `size` values of `operand`.
    fn register_var(&self, name: &str, operand: TfId, size: u64, before: bool) -> Lower<Var> {
        let key = self.key();
        let value = |value: &str| format!("[k \\in 1..{size} |-> {value}]");
        Ok(Var {
            name: name.to_owned(),
            ty: format!("[{} -> [1..{size} -> BOOLEAN]]", self.names.ids),
            init: Some(self.per_key(&value(boolean(before)))),
            update: Some(self.per_key(&value(&format!(
                "IF k = 1 THEN {} ELSE {name}[{key}][k - 1]",
                self.state(operand, false)?
            )))),
        })
    }

    /// A formula whose temporal operators are `[]` and `<>`.
    fn direct(&self, tf: TfId) -> Lower<String> {
        self.say(Say::Direct(tf))
    }

    fn direct_pieces(&self, tf: TfId, out: &mut Vec<Piece<Say>>) -> Lower<()> {
        if self.formulas.facts(tf).state {
            out.push(Piece::Node(Say::State(tf, false)));
            return Ok(());
        }
        let direct = |id| Piece::Node(Say::Direct(id));
        let unary = |out: &mut Vec<Piece<Say>>, open: &'static str, id| {
            out.extend([Piece::text(open), direct(id), Piece::text(")")]);
        };
        match self.formulas.get(tf)? {
            &Tf::Not(inner) => unary(out, "~(", inner),
            &Tf::And(left, right) => binary(out, direct(left), " /\\ ", direct(right)),
            &Tf::Or(left, right) => binary(out, direct(left), " \\/ ", direct(right)),
            &Tf::Implies(left, right) => binary(out, direct(left), " => ", direct(right)),
            &Tf::Always(_, inner) => unary(out, "[](", inner),
            &Tf::Eventually(_, inner) => unary(out, "<>(", inner),
            _ => {
                return Err(refusal(
                    CONSTRUCT,
                    "a temporal operator read as `[]` or `<>`",
                ));
            }
        }
        Ok(())
    }
}

/// `(left symbol right)`.
fn binary(out: &mut Vec<Piece<Say>>, left: Piece<Say>, symbol: &'static str, right: Piece<Say>) {
    out.extend([
        Piece::text("("),
        left,
        Piece::text(symbol),
        right,
        Piece::text(")"),
    ]);
}

fn boolean(value: bool) -> &'static str {
    if value {
        "TRUE"
    } else {
        "FALSE"
    }
}

// ---- Deterministic monitors ------------------------------------------------

/// `left /\ right`, dropping a `TRUE` operand.
fn both(left: &str, right: &str) -> String {
    if left == "TRUE" {
        right.to_owned()
    } else {
        format!("{left} /\\ {right}")
    }
}

fn in_window(iv: Iv, position: &str) -> String {
    match iv.bounds() {
        (0, None) => "TRUE".to_owned(),
        (lower, None) => format!("({lower} <= {position})"),
        (lower, Some(upper)) => format!("({lower} <= {position} /\\ {position} <= {upper})"),
    }
}

fn from_lower(iv: Iv, position: &str) -> String {
    match iv.bounds().0 {
        0 => "TRUE".to_owned(),
        lower => format!("({lower} <= {position})"),
    }
}

/// The monitor's next state from its current state `m` (`None` at the
/// origin, where it is running), the position of the state it reads and the
/// operands `left` and `right` there. A monitor settles on `acc` or `rel`
/// when its interval ends without a verdict.
fn monitor_step(
    monitor: &Monitor,
    m: Option<&str>,
    position: &str,
    left: &str,
    right: &str,
) -> String {
    let iv = monitor.iv;
    let window = in_window(iv, position);
    let tail = from_lower(iv, position);
    let ended = iv.bounds().1.map(|upper| format!("{position} >= {upper}"));
    let settle = |verdict: &str| match &ended {
        Some(ended) => format!("IF {ended} THEN \"{verdict}\" ELSE \"run\""),
        None => "\"run\"".to_owned(),
    };
    let body = match monitor.op {
        MonOp::Until => format!(
            "IF {} THEN \"acc\" ELSE IF {} THEN \"rej\" ELSE {}",
            both(&window, right),
            both(&tail, &format!("~{left}")),
            settle("rej")
        ),
        MonOp::Eventually => format!(
            "IF {} THEN \"acc\" ELSE {}",
            both(&window, left),
            settle("rej")
        ),
        MonOp::Release => format!(
            "IF {} THEN \"rej\" ELSE IF {} THEN \"rel\" ELSE {}",
            both(&window, &format!("~{right}")),
            both(&tail, left),
            settle("rel")
        ),
        MonOp::Always => format!(
            "IF {} THEN \"rej\" ELSE {}",
            both(&window, &format!("~{left}")),
            settle("rel")
        ),
    };
    match m {
        None => body,
        Some(m) => format!("IF {m} /= \"run\" THEN {m} ELSE {body}"),
    }
}

// ---- Fairness --------------------------------------------------------------

/// The action an operation contributes, with its arguments chosen.
fn operation_action(object: &Object, operation: &Operation, ids: &str, receiver: bool) -> String {
    let name = object.operation(operation);
    let params = parameters(operation);
    let names: Vec<&str> = params.iter().map(|(name, _)| name.as_str()).collect();
    let binders: Vec<String> = params
        .iter()
        .skip(usize::from(receiver))
        .map(|(param, kind)| match kind {
            None => format!("{param} \\in {ids}"),
            Some(kind) => format!("{param} \\in {}", tla_domain(kind)),
        })
        .collect();
    let call = format!("{name}({})", names.join(", "));
    if binders.is_empty() {
        call
    } else {
        format!("\\E {} : {call}", binders.join(", "))
    }
}

// ---- The clause ------------------------------------------------------------

/// Lower the clauses of `input` to TLA+.
pub(in crate::command::export::lower) fn tla(
    input: &Input<'_>,
    out: &mut Vec<Definition>,
) -> Lower<Extra> {
    let mut extra = Extra::default();
    let mut universes = BTreeSet::new();
    for clause in input.temporals {
        let object = input
            .objects
            .get(clause.object)
            .ok_or_else(|| refusal(CONSTRUCT, "an unexported clause context"))?;
        let names = Names::new(&clause.name, object);
        let attribute = |mut refusal: super::super::Refusal| {
            refusal.obligation = Some(clause.name.clone());
            refusal
        };
        if universes.insert(clause.object) {
            universe(input, clause, object, &names, out, &mut extra).map_err(attribute)?;
        }
        if let Some(model) = input.dense {
            extra.conditional.push(Cond {
                obligation: clause.name.clone(),
                kind: "real-valued-time",
                node: model.node,
                type_name: None,
            });
        }
        let ctx = Ctx {
            input,
            clause,
            formulas: &clause.formulas,
            object,
            names,
            histories: Vec::new(),
        };
        if input.dense.is_some() {
            emit_ok(&ctx, out).map_err(attribute)?;
            let variables = super::super::dense::tla_clause(
                clause,
                &ctx.names.ids,
                &ctx.names.ok,
                |tf, primed| ctx.state(tf, primed),
                out,
            )
            .map_err(attribute)?;
            extra
                .variables
                .extend(variables.iter().map(|(name, _)| name.clone()));
            extra.aux_names.extend(variables);
        } else {
            tla_clause(ctx, out, &mut extra).map_err(attribute)?;
        }
        if input.snapshot.is_none() {
            extra.conditional.push(Cond {
                obligation: clause.name.clone(),
                kind: "bounded-identity-universe",
                node: object.node,
                type_name: Some(object.name.clone()),
            });
        }
    }
    Ok(extra)
}

/// The model's constant identity universe: the keys of the initial snapshot,
/// or the constant `Keys_<T>` the model checker's configuration supplies
/// when the request fixes none.
fn universe(
    input: &Input<'_>,
    clause: &ClauseLow,
    object: &Object,
    names: &Names,
    out: &mut Vec<Definition>,
    extra: &mut Extra,
) -> Lower<()> {
    let body = match input.snapshot {
        Some(snapshot) => {
            let keys: Vec<String> = snapshot_rows(object, snapshot)?
                .iter()
                .map(|(key, _)| scalars(key, Target::Tla))
                .collect();
            format!("{{{}}}", keys.join(", "))
        }
        None => {
            let keys = mangle("Keys_", &object.name);
            extra.constants.push(keys.clone());
            keys
        }
    };
    out.push(define(
        clause.position,
        Some(names.ids.clone()),
        format!("{} == {body}", names.ids),
        Some(clause.name.as_str()),
    ));
    Ok(())
}

fn tla_clause(ctx: Ctx<'_>, out: &mut Vec<Definition>, extra: &mut Extra) -> Lower<()> {
    let clause = ctx.clause;
    let obligation = Some(clause.name.as_str());
    let (mut formulas, normal) = normalize(&clause.formulas, clause.tf)?;
    let (root, histories) = extract(&mut formulas, normal)?;
    let shape = shape_of(&formulas, root)?;
    let ctx = Ctx {
        formulas: &formulas,
        histories,
        ..ctx
    };

    let mut vars = ctx.history_vars()?;
    // The conjuncts of the theorem after `ok_`.
    let mut statements = Vec::new();
    let mut accepts: Vec<(String, String)> = Vec::new();
    let key = ctx.key().to_owned();
    match &shape {
        Shape::Direct => statements.push(ctx.direct(root)?),
        Shape::Monitors(parts) => {
            let multi = parts
                .iter()
                .filter(|part| matches!(part, Part::Mon(_)))
                .count()
                > 1;
            let mut index = 0;
            let mut counters = 0;
            for part in parts {
                match part {
                    &Part::Direct(tf) => statements.push(ctx.direct(tf)?),
                    Part::Mon(monitor) => {
                        let (m, accept) = ctx.names.monitor_names(index, multi);
                        index += 1;
                        let counter = match monitor.iv.bounds() {
                            (0, None) => None,
                            _ => {
                                counters += 1;
                                Some(ctx.names.counter(counters - 1))
                            }
                        };
                        monitor_vars(&ctx, monitor, &m, counter.as_deref(), &mut vars)?;
                        let (condition, wrap) = if monitor.op.reaches() {
                            (format!("{m}[{key}] = \"acc\""), "[]<>")
                        } else {
                            (format!("{m}[{key}] /= \"rej\""), "[]")
                        };
                        statements.push(format!("{wrap}{accept}({key})"));
                        accepts.push((accept, condition));
                    }
                }
            }
        }
        Shape::Tableau => return tableau::emit(&ctx, out, extra),
    }
    let names = &ctx.names;
    let composed = !vars.is_empty();
    let fair = fairness(&ctx)?;
    let spec_t = composed || !fair.is_empty();

    emit_ok(&ctx, out)?;
    if composed {
        let list: Vec<&str> = std::iter::once("vars")
            .chain(vars.iter().map(|var| var.name.as_str()))
            .collect();
        out.push(define(
            clause.position,
            Some(names.vars.clone()),
            format!("{} == <<{}>>", names.vars, list.join(", ")),
            obligation,
        ));
        let types: Vec<String> = vars
            .iter()
            .map(|var| format!("{} \\in {}", var.name, var.ty))
            .collect();
        out.push(define(
            clause.position,
            Some(names.type_ok.clone()),
            format!("{} ==\n{}", names.type_ok, bullets(&types)),
            obligation,
        ));
        let mut init = vec!["Init".to_owned()];
        init.extend(vars.iter().filter_map(|var| {
            var.init
                .as_ref()
                .map(|init| format!("{} = {init}", var.name))
        }));
        init.push(names.type_ok.clone());
        out.push(define(
            clause.position,
            Some(names.init.clone()),
            format!("{} ==\n{}", names.init, bullets(&init)),
            obligation,
        ));
        let updates: Vec<String> = vars
            .iter()
            .filter_map(|var| {
                var.update
                    .as_ref()
                    .map(|update| format!("{}' = {update}", var.name))
            })
            .collect();
        out.push(define(
            clause.position,
            Some(names.mon.clone()),
            format!("{} ==\n{}", names.mon, bullets(&updates)),
            obligation,
        ));
        out.push(define(
            clause.position,
            Some(names.next.clone()),
            format!(
                "{} == (Next \\/ (~ENABLED Next /\\ UNCHANGED vars)) /\\ {}",
                names.next, names.mon
            ),
            obligation,
        ));
    }
    if spec_t {
        let mut spec = if composed {
            format!(
                "{} == {} /\\ [][{}]_{} /\\ WF_{}({})",
                names.spec, names.init, names.next, names.vars, names.vars, names.next
            )
        } else {
            format!("{} == Init /\\ [][Next]_vars /\\ WF_vars(Next)", names.spec)
        };
        for text in &fair {
            spec.push_str(&format!(" /\\ {text}"));
        }
        out.push(define(
            clause.position,
            Some(names.spec.clone()),
            spec,
            obligation,
        ));
    }
    let premise = if spec_t { names.spec.as_str() } else { "Spec" };
    for (name, condition) in &accepts {
        out.push(define(
            clause.position,
            Some(name.clone()),
            format!("{name}({key}) == {condition}"),
            obligation,
        ));
    }
    let statement = ctx.guarded(&statements.join(" /\\ "));
    out.push(define(
        clause.position,
        Some(names.theorem.clone()),
        format!("THEOREM {} == {premise} => {statement}", names.theorem),
        obligation,
    ));
    extra
        .variables
        .extend(vars.iter().map(|var| var.name.clone()));
    Ok(())
}

fn emit_ok(ctx: &Ctx<'_>, out: &mut Vec<Definition>) -> Lower<()> {
    let key = ctx.key();
    let mut presence = vec![format!("{key} \\in DOMAIN {}", ctx.object.variable())];
    presence.extend(ctx.input.show.exprs(&ctx.clause.defs)?);
    let name = &ctx.names.ok;
    out.push(define(
        ctx.clause.position,
        Some(name.clone()),
        format!("{name}({key}) == {}", presence.join(" /\\ ")),
        Some(ctx.clause.name.as_str()),
    ));
    Ok(())
}

/// The monitor variable `m`, and the step counter of its interval.
fn monitor_vars(
    ctx: &Ctx<'_>,
    monitor: &Monitor,
    m: &str,
    counter: Option<&str>,
    vars: &mut Vec<Var>,
) -> Lower<()> {
    let names = &ctx.names;
    let key = ctx.key();
    let states = if monitor.op.reaches() {
        "{\"run\", \"acc\", \"rej\"}"
    } else {
        "{\"run\", \"rel\", \"rej\"}"
    };
    let pair = |primed: bool| -> Lower<(String, String)> {
        let left = ctx.state(monitor.left, primed)?;
        let right = match monitor.right {
            Some(right) => ctx.state(right, primed)?,
            None => String::new(),
        };
        Ok((left, right))
    };
    let cap = match monitor.iv.bounds() {
        (0, None) => None,
        (lower, None) => Some(lower),
        (_, Some(upper)) => Some(upper),
    };
    let (left, right) = pair(false)?;
    let init = ctx.per_key(&monitor_step(monitor, None, "0", &left, &right));
    let (left, right) = pair(true)?;
    let position = counter.map_or_else(|| "0".to_owned(), |n| format!("{n}'"));
    let update = ctx.per_key(&monitor_step(
        monitor,
        Some(&format!("{m}[{key}]")),
        &position,
        &left,
        &right,
    ));
    if let (Some(cap), Some(n)) = (cap, counter) {
        vars.push(Var {
            ty: format!("0..{cap}"),
            init: Some("0".to_owned()),
            update: Some(format!("IF {n} < {cap} THEN {n} + 1 ELSE {cap}")),
            name: n.to_owned(),
        });
    }
    vars.push(Var {
        ty: format!("[{} -> {states}]", names.ids),
        name: m.to_owned(),
        init: Some(init),
        update: Some(update),
    });
    Ok(())
}

/// The fairness conjuncts of the clause's composed specification.
fn fairness(ctx: &Ctx<'_>) -> Lower<Vec<String>> {
    let mut fair = Vec::new();
    for constraint in &ctx.clause.fairness {
        let operation = ctx
            .object
            .operations
            .get(constraint.operation)
            .ok_or_else(|| refusal("temporal/fairness", "an operation outside the state"))?;
        let kind = if constraint.strong { "SF" } else { "WF" };
        let text = if constraint.each {
            let action = operation_action(ctx.object, operation, &ctx.names.ids, true);
            let range = if ctx.input.snapshot.is_none() {
                mangle("Keys_", &ctx.object.name)
            } else {
                ctx.names.ids.clone()
            };
            format!("\\A r_self \\in {range} : {kind}_vars({action})")
        } else {
            let action = operation_action(ctx.object, operation, &ctx.names.ids, false);
            format!("{kind}_vars({action})")
        };
        fair.push(text);
    }
    Ok(fair)
}
