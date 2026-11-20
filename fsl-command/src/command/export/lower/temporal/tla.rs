// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-813: temporal clauses and fairness in TLA+.
//!
//! A clause built from `always`, `eventually` and Boolean connectives over
//! state formulas is `[]` and `<>` directly. A past operator becomes an
//! auxiliary variable: a Boolean for an unbounded operator, a saturating
//! counter or a delay register for a lower-bounded one, a shift register for
//! a closed interval. `until`, `release` and an interval operator at the
//! root of a clause become a deterministic monitor. Every other clause is a
//! nondeterministic Buchi monitor that guesses the truth of its future
//! subformulas, and its record is `conditional`.
//!
//! Every temporal-level range is the model's constant identity universe
//! `Ids_<T>`, guarded at the origin in the theorem, never `DOMAIN v`, which
//! varies with the state. The step counter and the histories advance on
//! every step of `Next_<clause>`, which includes the terminal step, so both
//! targets count FSL's terminal stutter positions.

use super::{CONSTRUCT, ClauseLow, Iv, Tf};
use crate::command::export::lower::{
    Definition, Input, Lower, Object, Operation, Target, bullets, defer, define, mangle,
    parameters, refusal, scalars, snapshot_rows, tla_domain,
};
use std::collections::BTreeSet;

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
}

// ---- Normal form -----------------------------------------------------------

/// `historically` and `triggered` as the duals of `once` and `since`.
fn normalize(tf: &Tf) -> Tf {
    let go = |inner: &Tf| Box::new(normalize(inner));
    let not = |inner: Tf| Tf::Not(Box::new(inner));
    match tf {
        Tf::Const(_) | Tf::Atom(_) | Tf::Hist(_) => tf.clone(),
        Tf::Not(inner) => Tf::Not(go(inner)),
        Tf::And(left, right) => Tf::And(go(left), go(right)),
        Tf::Or(left, right) => Tf::Or(go(left), go(right)),
        Tf::Implies(left, right) => Tf::Implies(go(left), go(right)),
        Tf::Always(iv, inner) => Tf::Always(*iv, go(inner)),
        Tf::Eventually(iv, inner) => Tf::Eventually(*iv, go(inner)),
        Tf::Once(iv, inner) => Tf::Once(*iv, go(inner)),
        Tf::Until(iv, left, right) => Tf::Until(*iv, go(left), go(right)),
        Tf::Release(iv, left, right) => Tf::Release(*iv, go(left), go(right)),
        Tf::Since(iv, left, right) => Tf::Since(*iv, go(left), go(right)),
        Tf::Next(inner) => Tf::Next(go(inner)),
        Tf::Historically(iv, inner) => not(Tf::Once(*iv, Box::new(not(normalize(inner))))),
        Tf::Triggered(iv, left, right) => not(Tf::Since(
            *iv,
            Box::new(not(normalize(left))),
            Box::new(not(normalize(right))),
        )),
    }
}

fn is_state(tf: &Tf) -> bool {
    match tf {
        Tf::Const(_) | Tf::Atom(_) | Tf::Hist(_) => true,
        Tf::Not(inner) => is_state(inner),
        Tf::And(left, right) | Tf::Or(left, right) | Tf::Implies(left, right) => {
            is_state(left) && is_state(right)
        }
        _ => false,
    }
}

/// A formula `[]` and `<>` state directly.
fn is_direct(tf: &Tf) -> bool {
    match tf {
        Tf::Not(inner) => is_direct(inner),
        Tf::And(left, right) | Tf::Or(left, right) | Tf::Implies(left, right) => {
            is_direct(left) && is_direct(right)
        }
        Tf::Always(iv, inner) | Tf::Eventually(iv, inner) => !iv.is_some() && is_direct(inner),
        _ => is_state(tf),
    }
}

// ---- Histories -------------------------------------------------------------

enum HKind {
    /// `once p`: a Boolean `h`.
    Once { p: Tf },
    /// `p since q`: a Boolean `h`.
    Since { p: Tf, q: Tf },
    /// `once[a,*] p`: a counter `c` of the steps since `p` first held,
    /// saturating at `a`.
    OnceFrom { p: Tf, a: u64 },
    /// `p since[a,*] q`: a register `r` of length `a` over the Boolean of
    /// the unbounded `p since q`.
    SinceFrom { inner: usize, a: u64 },
    /// `once[a,b] p`: a shift register `r` of the last `b` values of `p`.
    OnceWithin { p: Tf, a: u64, b: u64 },
    /// `p since[a,b] q`: registers `r` over `q` and `r + 1` over `p`.
    SinceWithin { p: Tf, q: Tf, a: u64, b: u64 },
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

/// The value `tf` has at every position before the origin, where each atom
/// is false.
fn before_origin(tf: &Tf, histories: &[History]) -> bool {
    match tf {
        Tf::Const(value) => *value,
        Tf::Hist(index) => histories.get(*index).is_some_and(|history| history.before),
        Tf::Not(inner) => !before_origin(inner, histories),
        Tf::And(left, right) => before_origin(left, histories) && before_origin(right, histories),
        Tf::Or(left, right) => before_origin(left, histories) || before_origin(right, histories),
        Tf::Implies(left, right) => {
            !before_origin(left, histories) || before_origin(right, histories)
        }
        _ => false,
    }
}

/// Replace each past operator of `tf` by a history.
fn extract(tf: &Tf, histories: &mut Vec<History>, ordinal: &mut u64) -> Lower<Tf> {
    fn go(inner: &Tf, histories: &mut Vec<History>, ordinal: &mut u64) -> Lower<Box<Tf>> {
        extract(inner, histories, ordinal).map(Box::new)
    }
    Ok(match tf {
        Tf::Const(_) | Tf::Atom(_) | Tf::Hist(_) => tf.clone(),
        Tf::Not(inner) => Tf::Not(go(inner, histories, ordinal)?),
        Tf::And(left, right) => Tf::And(
            go(left, histories, ordinal)?,
            go(right, histories, ordinal)?,
        ),
        Tf::Or(left, right) => Tf::Or(
            go(left, histories, ordinal)?,
            go(right, histories, ordinal)?,
        ),
        Tf::Implies(left, right) => Tf::Implies(
            go(left, histories, ordinal)?,
            go(right, histories, ordinal)?,
        ),
        Tf::Always(iv, inner) => Tf::Always(*iv, go(inner, histories, ordinal)?),
        Tf::Eventually(iv, inner) => Tf::Eventually(*iv, go(inner, histories, ordinal)?),
        Tf::Until(iv, left, right) => Tf::Until(
            *iv,
            go(left, histories, ordinal)?,
            go(right, histories, ordinal)?,
        ),
        Tf::Release(iv, left, right) => Tf::Release(
            *iv,
            go(left, histories, ordinal)?,
            go(right, histories, ordinal)?,
        ),
        Tf::Next(inner) => Tf::Next(go(inner, histories, ordinal)?),
        Tf::Once(iv, inner) => {
            let ord = *ordinal;
            *ordinal += 1;
            let p = *go(inner, histories, ordinal)?;
            past_state(&p)?;
            let before_p = before_origin(&p, histories);
            let (kind, before) = match iv.bounds() {
                (0, None) => (HKind::Once { p }, false),
                (a, None) => (HKind::OnceFrom { p, a }, false),
                (a, Some(b)) => (HKind::OnceWithin { p, a, b }, before_p),
            };
            histories.push(History {
                kind,
                ord,
                before,
                before_p,
                before_q: false,
            });
            Tf::Hist(histories.len() - 1)
        }
        Tf::Since(iv, left, right) => {
            let ord = *ordinal;
            *ordinal += 2;
            let p = *go(left, histories, ordinal)?;
            let q = *go(right, histories, ordinal)?;
            past_state(&p)?;
            past_state(&q)?;
            let (before_p, before_q) = (before_origin(&p, histories), before_origin(&q, histories));
            match iv.bounds() {
                (a, Some(b)) => histories.push(History {
                    kind: HKind::SinceWithin { p, q, a, b },
                    ord,
                    before: before_q,
                    before_p,
                    before_q,
                }),
                (a, None) => {
                    histories.push(History {
                        kind: HKind::Since { p, q },
                        ord,
                        before: false,
                        before_p,
                        before_q,
                    });
                    if a > 0 {
                        let inner = histories.len() - 1;
                        histories.push(History {
                            kind: HKind::SinceFrom { inner, a },
                            ord,
                            before: false,
                            before_p,
                            before_q,
                        });
                    }
                }
            }
            Tf::Hist(histories.len() - 1)
        }
        Tf::Historically(..) | Tf::Triggered(..) => {
            return refuse_internal("a past operator that normalization leaves");
        }
    })
}

fn refuse_internal<T>(detail: &str) -> Lower<T> {
    Err(refusal(CONSTRUCT, detail))
}

/// The operand of a past operator must read no future position.
fn past_state(tf: &Tf) -> Lower<()> {
    if is_state(tf) {
        Ok(())
    } else {
        defer(
            "temporal/formula",
            "a past operator over a formula that holds a future operator",
        )
    }
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
    left: Tf,
    right: Option<Tf>,
}

/// One conjunct of a clause: a formula `[]` and `<>` state directly or a
/// deterministic monitor.
enum Part {
    Direct(Tf),
    Mon(Monitor),
}

enum Shape {
    Direct,
    Monitors(Vec<Part>),
    Tableau,
}

fn conjuncts<'a>(tf: &'a Tf, out: &mut Vec<&'a Tf>) {
    match tf {
        Tf::And(left, right) => {
            conjuncts(left, out);
            conjuncts(right, out);
        }
        other => out.push(other),
    }
}

/// The deterministic monitor of `tf`, when it has one.
fn monitor_of(tf: &Tf) -> Option<Monitor> {
    let (op, left, right, iv) = match tf {
        Tf::Until(iv, left, right) => (MonOp::Until, left, Some(right), *iv),
        Tf::Release(iv, left, right) => (MonOp::Release, left, Some(right), *iv),
        Tf::Always(iv, inner) if iv.is_some() => (MonOp::Always, inner, None, *iv),
        Tf::Eventually(iv, inner) if iv.is_some() => (MonOp::Eventually, inner, None, *iv),
        _ => return None,
    };
    if !is_state(left) || right.is_some_and(|right| !is_state(right)) {
        return None;
    }
    Some(Monitor {
        op,
        iv,
        left: (**left).clone(),
        right: right.map(|right| (**right).clone()),
    })
}

fn shape_of(tf: &Tf) -> Shape {
    if is_direct(tf) {
        return Shape::Direct;
    }
    let mut leaves = Vec::new();
    conjuncts(tf, &mut leaves);
    let mut parts = Vec::new();
    for leaf in leaves {
        if is_direct(leaf) {
            parts.push(Part::Direct(leaf.clone()));
        } else if let Some(monitor) = monitor_of(leaf) {
            parts.push(Part::Mon(monitor));
        } else {
            return Shape::Tableau;
        }
    }
    Shape::Monitors(parts)
}

// ---- The tableau -----------------------------------------------------------

/// The widest nested interval, in offsets, the nondeterministic monitor
/// expands.
const MAX_EXPANSION: u64 = 32;

fn next_n(tf: Tf, n: u64) -> Tf {
    (0..n).fold(tf, |inner, _| Tf::Next(Box::new(inner)))
}

fn fold(items: Vec<Tf>, combine: fn(Box<Tf>, Box<Tf>) -> Tf) -> Tf {
    let mut items = items.into_iter();
    let first = items.next().unwrap_or(Tf::Const(true));
    items.fold(first, |left, right| {
        combine(Box::new(left), Box::new(right))
    })
}

/// Interval operators as offsets, so the nondeterministic monitor needs
/// only the unbounded operators and the next position.
fn desugar(tf: &Tf) -> Lower<Tf> {
    let go = |inner: &Tf| desugar(inner).map(Box::new);
    let width = |a: u64, b: u64| {
        if b.saturating_sub(a) > MAX_EXPANSION {
            defer("temporal/formula", "a nested interval wider than 32 steps")
        } else {
            Ok(())
        }
    };
    Ok(match tf {
        Tf::Const(_) | Tf::Atom(_) | Tf::Hist(_) => tf.clone(),
        Tf::Not(inner) => Tf::Not(go(inner)?),
        Tf::And(left, right) => Tf::And(go(left)?, go(right)?),
        Tf::Or(left, right) => Tf::Or(go(left)?, go(right)?),
        Tf::Implies(left, right) => Tf::Implies(go(left)?, go(right)?),
        Tf::Next(inner) => Tf::Next(go(inner)?),
        Tf::Always(iv, inner) => {
            let p = desugar(inner)?;
            match iv.bounds() {
                (0, None) => Tf::Always(Iv::none(), Box::new(p)),
                (a, None) => next_n(Tf::Always(Iv::none(), Box::new(p)), a),
                (a, Some(b)) => {
                    width(a, b)?;
                    fold((a..=b).map(|d| next_n(p.clone(), d)).collect(), Tf::And)
                }
            }
        }
        Tf::Eventually(iv, inner) => {
            let p = desugar(inner)?;
            match iv.bounds() {
                (0, None) => Tf::Eventually(Iv::none(), Box::new(p)),
                (a, None) => next_n(Tf::Eventually(Iv::none(), Box::new(p)), a),
                (a, Some(b)) => {
                    width(a, b)?;
                    fold((a..=b).map(|d| next_n(p.clone(), d)).collect(), Tf::Or)
                }
            }
        }
        Tf::Until(iv, left, right) => {
            let (p, q) = (desugar(left)?, desugar(right)?);
            match iv.bounds() {
                (0, None) => Tf::Until(Iv::none(), Box::new(p), Box::new(q)),
                (a, None) => next_n(Tf::Until(Iv::none(), Box::new(p), Box::new(q)), a),
                (a, Some(b)) => {
                    width(a, b)?;
                    fold(
                        (a..=b)
                            .map(|d| {
                                let mut all = vec![next_n(q.clone(), d)];
                                all.extend((a..d).map(|e| next_n(p.clone(), e)));
                                fold(all, Tf::And)
                            })
                            .collect(),
                        Tf::Or,
                    )
                }
            }
        }
        Tf::Release(iv, left, right) => desugar(&Tf::Not(Box::new(Tf::Until(
            *iv,
            Box::new(Tf::Not(left.clone())),
            Box::new(Tf::Not(right.clone())),
        ))))?,
        Tf::Once(..) | Tf::Historically(..) | Tf::Since(..) | Tf::Triggered(..) => {
            return refuse_internal("a past operator left in a future formula");
        }
    })
}

/// A guessed subformula of the nondeterministic monitor.
enum Guess {
    Next(Tf),
    Until(Tf, Tf),
    Release(Tf, Tf),
    Always(Tf),
    Eventually(Tf),
}

#[derive(Default)]
struct Tableau {
    guesses: Vec<(Tf, Guess)>,
}

impl Tableau {
    /// The index of the guess for `tf`, registering it when it is new.
    fn register(&mut self, tf: &Tf) -> Lower<usize> {
        if let Some(index) = self.guesses.iter().position(|(seen, _)| seen == tf) {
            return Ok(index);
        }
        let guess = match tf {
            Tf::Next(inner) => Guess::Next((**inner).clone()),
            Tf::Until(_, left, right) => Guess::Until((**left).clone(), (**right).clone()),
            Tf::Release(_, left, right) => Guess::Release((**left).clone(), (**right).clone()),
            Tf::Always(_, inner) => Guess::Always((**inner).clone()),
            Tf::Eventually(_, inner) => Guess::Eventually((**inner).clone()),
            _ => return refuse_internal("a guess for a formula that is no future operator"),
        };
        if self.guesses.len() >= 128 {
            return defer(
                "temporal/formula",
                "a nested formula with more than 128 future subformulas",
            );
        }
        self.guesses.push((tf.clone(), guess));
        Ok(self.guesses.len() - 1)
    }
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

    fn guess(&self, index: usize) -> String {
        format!("{}_g{index}", self.monitor)
    }
}

/// An auxiliary variable of a clause: its type and, unless it is a free
/// guess, its initial and updated values.
struct Var {
    name: String,
    ty: String,
    init: Option<String>,
    update: Option<String>,
}

struct Ctx<'a> {
    input: &'a Input<'a>,
    clause: &'a ClauseLow,
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
        let e = atoms
            .get(index)
            .ok_or_else(|| refusal(CONSTRUCT, "an atom with no lowered operand"))?;
        self.input.show.expr(e)
    }

    /// A state formula at the current or the next state.
    fn state(&self, tf: &Tf, primed: bool) -> Lower<String> {
        Ok(match tf {
            Tf::Const(true) => "TRUE".to_owned(),
            Tf::Const(false) => "FALSE".to_owned(),
            Tf::Atom(index) => self.atom(*index, primed)?,
            Tf::Hist(index) => self.hist_value(*index, primed)?,
            Tf::Not(inner) => format!("~({})", self.state(inner, primed)?),
            Tf::And(left, right) => format!(
                "({} /\\ {})",
                self.state(left, primed)?,
                self.state(right, primed)?
            ),
            Tf::Or(left, right) => format!(
                "({} \\/ {})",
                self.state(left, primed)?,
                self.state(right, primed)?
            ),
            Tf::Implies(left, right) => format!(
                "({} => {})",
                self.state(left, primed)?,
                self.state(right, primed)?
            ),
            _ => {
                return Err(refusal(
                    CONSTRUCT,
                    "a temporal operator read as a state formula",
                ));
            }
        })
    }

    /// The variable of a history that holds a Boolean, a counter or a
    /// register.
    fn variable(&self, history: &History) -> String {
        let letter = match history.kind {
            HKind::Once { .. } | HKind::Since { .. } => 'h',
            HKind::OnceFrom { .. } => 'c',
            HKind::SinceFrom { .. } | HKind::OnceWithin { .. } | HKind::SinceWithin { .. } => 'r',
        };
        self.names.variable(letter, history.ord)
    }

    /// The value of the history at the current or the next position.
    fn hist_value(&self, index: usize, primed: bool) -> Lower<String> {
        let history = self
            .histories
            .get(index)
            .ok_or_else(|| refusal(CONSTRUCT, "a history that does not exist"))?;
        let name = self.variable(history);
        let tick = if primed { "'" } else { "" };
        let key = self.key();
        Ok(match &history.kind {
            HKind::Once { .. } | HKind::Since { .. } => format!("{name}{tick}[{key}]"),
            HKind::OnceFrom { a, .. } => format!("{name}{tick}[{key}] = {a}"),
            HKind::SinceFrom { a, .. } => format!("{name}{tick}[{key}][{a}]"),
            HKind::OnceWithin { p, a, b } => {
                let mut parts = Vec::new();
                if *a == 0 {
                    parts.push(self.state(p, primed)?);
                }
                let lower = (*a).max(1);
                if lower <= *b {
                    parts.push(format!(
                        "(\\E k \\in {lower}..{b} : {name}{tick}[{key}][k])"
                    ));
                }
                format!("({})", parts.join(" \\/ "))
            }
            HKind::SinceWithin { p, q, a, b } => {
                if *b == 0 {
                    return self.state(q, primed);
                }
                let (current_p, current_q) = (self.state(p, primed)?, self.state(q, primed)?);
                let p_name = self.names.variable('r', history.ord + 1);
                format!(
                    "(\\E d \\in {a}..{b} : (IF d = 0 THEN {current_q} ELSE {name}{tick}[{key}][d]) /\\ (\\A e \\in {a}..(d - 1) : IF e = 0 THEN {current_p} ELSE {p_name}{tick}[{key}][e]))"
                )
            }
        })
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
            match &history.kind {
                HKind::Once { p } => vars.push(Var {
                    ty: function("BOOLEAN"),
                    init: Some(self.per_key(&self.state(p, false)?)),
                    update: Some(self.per_key(&format!(
                        "({} \\/ {name}[{key}])",
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
                    let source = self.hist_value(*inner, false)?;
                    vars.push(Var {
                        ty: function(&format!("[1..{a} -> BOOLEAN]")),
                        init: Some(self.per_key(&register(*a, "FALSE"))),
                        update: Some(self.per_key(&register(
                            *a,
                            &format!("IF k = 1 THEN {source} ELSE {name}[{key}][k - 1]"),
                        ))),
                        name,
                    });
                }
                HKind::OnceWithin { p, b, .. } if *b >= 1 => {
                    vars.push(self.register_var(&name, p, *b, history.before_p)?);
                }
                HKind::SinceWithin { p, q, b, .. } if *b >= 1 => {
                    vars.push(self.register_var(&name, q, *b, history.before_q)?);
                    vars.push(self.register_var(
                        &self.names.variable('r', history.ord + 1),
                        p,
                        *b,
                        history.before_p,
                    )?);
                }
                HKind::OnceWithin { .. } | HKind::SinceWithin { .. } => {}
            }
        }
        Ok(vars)
    }

    /// A shift register of the last `size` values of `operand`.
    fn register_var(&self, name: &str, operand: &Tf, size: u64, before: bool) -> Lower<Var> {
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
    fn direct(&self, tf: &Tf) -> Lower<String> {
        if is_state(tf) {
            return self.state(tf, false);
        }
        Ok(match tf {
            Tf::Not(inner) => format!("~({})", self.direct(inner)?),
            Tf::And(left, right) => {
                format!("({} /\\ {})", self.direct(left)?, self.direct(right)?)
            }
            Tf::Or(left, right) => {
                format!("({} \\/ {})", self.direct(left)?, self.direct(right)?)
            }
            Tf::Implies(left, right) => {
                format!("({} => {})", self.direct(left)?, self.direct(right)?)
            }
            Tf::Always(_, inner) => format!("[]({})", self.direct(inner)?),
            Tf::Eventually(_, inner) => format!("<>({})", self.direct(inner)?),
            _ => {
                return Err(refusal(
                    CONSTRUCT,
                    "a temporal operator read as `[]` or `<>`",
                ));
            }
        })
    }
}

fn boolean(value: bool) -> &'static str {
    if value { "TRUE" } else { "FALSE" }
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
    let _ = object;
    let name = mangle("op_", &operation.name);
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
        if universes.insert(clause.object) {
            universe(input, clause, object, &names, out, &mut extra)?;
        }
        if input.snapshot.is_none() {
            extra.conditional.push(Cond {
                obligation: clause.name.clone(),
                kind: "bounded-identity-universe",
                node: object.node,
                type_name: Some(object.name.clone()),
            });
        }
        let ctx = Ctx {
            input,
            clause,
            object,
            names,
            histories: Vec::new(),
        };
        tla_clause(ctx, out, &mut extra)?;
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

fn tla_clause(mut ctx: Ctx<'_>, out: &mut Vec<Definition>, extra: &mut Extra) -> Lower<()> {
    let clause = ctx.clause;
    let obligation = Some(clause.name.as_str());
    let mut histories = Vec::new();
    let root = extract(&normalize(&clause.tf), &mut histories, &mut 0)?;
    ctx.histories = histories;
    let shape = shape_of(&root);

    let mut vars = ctx.history_vars()?;
    let mut extra_init = Vec::new();
    let mut extra_update = Vec::new();
    // The conjuncts of the theorem after `ok_`.
    let mut statements = Vec::new();
    let mut accepts: Vec<(String, String)> = Vec::new();
    let mut conditional = false;
    let key = ctx.key().to_owned();
    match &shape {
        Shape::Direct => statements.push(ctx.direct(&root)?),
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
                    Part::Direct(tf) => statements.push(ctx.direct(tf)?),
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
        Shape::Tableau => {
            conditional = true;
            let condition =
                tableau_vars(&ctx, &root, &mut vars, &mut extra_init, &mut extra_update)?;
            let accept = ctx.names.accept.clone();
            statements.push(format!("{accept}({key})"));
            accepts.push((accept, condition));
        }
    }

    let names = &ctx.names;
    let composed = !vars.is_empty();
    let fair = fairness(&ctx)?;
    let spec_t = composed || !fair.is_empty();

    out.push(define(
        clause.position,
        Some(names.ok.clone()),
        format!(
            "{}({key}) == {key} \\in DOMAIN {}",
            names.ok,
            ctx.object.variable()
        ),
        obligation,
    ));
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
        init.extend(extra_init);
        init.push(names.type_ok.clone());
        out.push(define(
            clause.position,
            Some(names.init.clone()),
            format!("{} ==\n{}", names.init, bullets(&init)),
            obligation,
        ));
        let mut updates: Vec<String> = vars
            .iter()
            .filter_map(|var| {
                var.update
                    .as_ref()
                    .map(|update| format!("{}' = {update}", var.name))
            })
            .collect();
        updates.extend(extra_update);
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
    if conditional {
        extra.conditional.push(Cond {
            obligation: clause.name.clone(),
            kind: "nondeterministic-monitor",
            node: clause.position,
            type_name: None,
        });
    }
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
        let left = ctx.state(&monitor.left, primed)?;
        let right = match &monitor.right {
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

/// The guess variables and constraints of the nondeterministic monitor of
/// `root`; the result is the acceptance condition over its eventualities.
fn tableau_vars(
    ctx: &Ctx<'_>,
    root: &Tf,
    vars: &mut Vec<Var>,
    init: &mut Vec<String>,
    update: &mut Vec<String>,
) -> Lower<String> {
    let mut tableau = Tableau::default();
    let desugared = desugar(root)?;
    let root_value = value(ctx, &mut tableau, &desugared, false)?;
    init.push(format!(
        "\\A {} \\in {} : {root_value}",
        ctx.key(),
        ctx.names.ids
    ));
    let key = ctx.key();
    let mut constraints = Vec::new();
    let mut eventualities = Vec::new();
    let mut index = 0;
    while index < tableau.guesses.len() {
        let g = ctx.names.guess(index);
        let now = format!("{g}[{key}]");
        let next = format!("{g}'[{key}]");
        match tableau.guesses[index].1.clone_guess() {
            Guess::Next(inner) => {
                let after = value(ctx, &mut tableau, &inner, true)?;
                constraints.push(format!("({now} <=> {after})"));
            }
            Guess::Until(left, right) => {
                let l = value(ctx, &mut tableau, &left, false)?;
                let r = value(ctx, &mut tableau, &right, false)?;
                constraints.push(format!("({now} <=> ({r} \\/ ({l} /\\ {next})))"));
                eventualities.push(format!("[]<>(~{now} \\/ {r})"));
            }
            Guess::Release(left, right) => {
                let l = value(ctx, &mut tableau, &left, false)?;
                let r = value(ctx, &mut tableau, &right, false)?;
                constraints.push(format!("({now} <=> ({r} /\\ ({l} \\/ {next})))"));
            }
            Guess::Always(inner) => {
                let f = value(ctx, &mut tableau, &inner, false)?;
                constraints.push(format!("({now} <=> ({f} /\\ {next}))"));
            }
            Guess::Eventually(inner) => {
                let f = value(ctx, &mut tableau, &inner, false)?;
                constraints.push(format!("({now} <=> ({f} \\/ {next}))"));
                eventualities.push(format!("[]<>(~{now} \\/ {f})"));
            }
        }
        index += 1;
    }
    for index in 0..tableau.guesses.len() {
        let name = ctx.names.guess(index);
        update.push(format!("{name}' \\in [{} -> BOOLEAN]", ctx.names.ids));
        vars.push(Var {
            name,
            ty: format!("[{} -> BOOLEAN]", ctx.names.ids),
            init: None,
            update: None,
        });
    }
    if !constraints.is_empty() {
        update.push(format!(
            "\\A {key} \\in {} : {}",
            ctx.names.ids,
            constraints.join(" /\\ ")
        ));
    }
    Ok(if eventualities.is_empty() {
        "TRUE".to_owned()
    } else {
        eventualities.join(" /\\ ")
    })
}

impl Guess {
    fn clone_guess(&self) -> Self {
        match self {
            Self::Next(inner) => Self::Next(inner.clone()),
            Self::Until(left, right) => Self::Until(left.clone(), right.clone()),
            Self::Release(left, right) => Self::Release(left.clone(), right.clone()),
            Self::Always(inner) => Self::Always(inner.clone()),
            Self::Eventually(inner) => Self::Eventually(inner.clone()),
        }
    }
}

/// The value of `tf` at the current or the next position, with a guess
/// variable for each future operator.
fn value(ctx: &Ctx<'_>, tableau: &mut Tableau, tf: &Tf, primed: bool) -> Lower<String> {
    let tick = if primed { "'" } else { "" };
    Ok(match tf {
        _ if is_state(tf) => ctx.state(tf, primed)?,
        Tf::Not(inner) => format!("~({})", value(ctx, tableau, inner, primed)?),
        Tf::And(left, right) => format!(
            "({} /\\ {})",
            value(ctx, tableau, left, primed)?,
            value(ctx, tableau, right, primed)?
        ),
        Tf::Or(left, right) => format!(
            "({} \\/ {})",
            value(ctx, tableau, left, primed)?,
            value(ctx, tableau, right, primed)?
        ),
        Tf::Implies(left, right) => format!(
            "({} => {})",
            value(ctx, tableau, left, primed)?,
            value(ctx, tableau, right, primed)?
        ),
        other => {
            let index = tableau.register(other)?;
            format!("{}{tick}[{}]", ctx.names.guess(index), ctx.key())
        }
    })
}

/// The fairness conjuncts of the clause's composed specification.
fn fairness(ctx: &Ctx<'_>) -> Lower<Vec<String>> {
    let mut fair = Vec::new();
    for constraint in &ctx.clause.fairness {
        if constraint.object != ctx.clause.object {
            return defer(
                "temporal/fairness-target",
                "a fairness over an operation of another object type than the clause's subject",
            );
        }
        let operation = ctx
            .object
            .operations
            .get(constraint.operation)
            .ok_or_else(|| refusal("temporal/fairness", "an operation outside the state"))?;
        let kind = if constraint.strong { "SF" } else { "WF" };
        let text = if constraint.each {
            let action = operation_action(ctx.object, operation, &ctx.names.ids, true);
            format!("\\A r_self \\in {} : {kind}_vars({action})", ctx.names.ids)
        } else {
            let action = operation_action(ctx.object, operation, &ctx.names.ids, false);
            format!("{kind}_vars({action})")
        };
        fair.push(text);
    }
    Ok(fair)
}
