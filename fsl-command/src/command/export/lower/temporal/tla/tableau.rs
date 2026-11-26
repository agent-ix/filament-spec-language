// SPDX-License-Identifier: AGPL-3.0-or-later
//! A finite tableau for the negated clause. Invalid guesses reject the
//! monitor run without disabling any transition of the subject.

use super::{conjuncts, fairness, Cond, Ctx, Extra, Iv, Tf, TfId, Var};
use crate::command::export::lower::temporal::{FormulaSite, Formulas};
use crate::command::export::lower::{
    bullets, defer_at, define, mangle, Definition, Kind, Lower, E,
};
use filament_walk::{walk, Children, Walk};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::ControlFlow;

/// A row ranges over a Boolean assignment to this finite formula closure.
/// Bound it before constructing a TLA+ function set of assignments.
const MAX_NODES: usize = 16;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Node {
    Constant(bool),
    Atom { index: usize, positive: bool },
    ResponseViolation(usize),
    And(usize, usize),
    Or(usize, usize),
    Next(usize),
    Until(usize, usize),
    Release(usize, usize),
}

#[derive(Clone, Copy, Debug)]
enum Refused {
    /// An operand that is not in the formula, or lost its value.
    Broken,
    History,
    Limit(Option<usize>),
}

impl Refused {
    /// Attribute an unattributed limit to `node`.
    fn at(self, node: Option<usize>) -> Self {
        match self {
            Self::Limit(None) => Self::Limit(node),
            error => error,
        }
    }
}

/// A use in the checked source tree. Unit formulas have no checked sites.
#[derive(Clone, Copy, Default)]
struct Site<'a> {
    nodes: &'a [FormulaSite],
    at: usize,
}

impl Site<'_> {
    fn node(self) -> Option<usize> {
        self.nodes.get(self.at).map(|site| site.position)
    }

    fn operand(self, index: usize) -> Self {
        match self
            .nodes
            .get(self.at)
            .and_then(|site| site.operands.get(index))
        {
            Some(&at) => Self { at, ..self },
            None => Self::default(),
        }
    }
}

/// A deadline that can be checked when its upper bound has elapsed.
#[derive(Clone, Copy)]
struct Response {
    lower: u64,
    upper: u64,
    trigger: usize,
    ready: usize,
    node: Option<usize>,
}

impl Response {
    fn of(formulas: &Formulas, tf: TfId) -> Option<Self> {
        let get = |id| formulas.get(id).ok();
        let &Tf::Always(iv, inner) = get(tf)? else {
            return None;
        };
        if iv.is_some() {
            return None;
        }
        let &Tf::Implies(trigger, response) = get(inner)? else {
            return None;
        };
        let (&Tf::Atom(trigger), &Tf::Eventually(window, ready)) = (get(trigger)?, get(response)?)
        else {
            return None;
        };
        let &Tf::Atom(ready) = get(ready)? else {
            return None;
        };
        Some(Self {
            lower: window.lower(),
            upper: window.upper()?,
            trigger,
            ready,
            node: None,
        })
    }

    /// Read the trigger at distance `upper` and the responses from distance
    /// `upper - lower` through the current position. The saturating step
    /// counter makes that history eligible only after the first deadline.
    fn monitor(&self, ctx: &Ctx<'_>, ordinal: usize) -> Lower<(Vec<Var>, String)> {
        let names = &ctx.names;
        let key = ctx.key();
        let ids = &names.ids;
        let upper = self.upper;
        let age = names.counter(ordinal);
        let mut variables = vec![Var {
            ty: format!("[{ids} -> 0..{upper}]"),
            init: Some(ctx.per_key("0")),
            update: Some(ctx.per_key(&format!(
                "IF {age}[{key}] < {upper} THEN {age}[{key}] + 1 ELSE {upper}"
            ))),
            name: age.clone(),
        }];
        let history = |ordinal, length, atom| -> Lower<Var> {
            let name = names.variable('r', ordinal);
            let cur = ctx.atom(atom, false)?;
            Ok(Var {
                ty: format!("[{ids} -> [1..{length} -> BOOLEAN]]"),
                init: Some(ctx.per_key(&format!("[i \\in 1..{length} |-> FALSE]"))),
                update: Some(ctx.per_key(&format!(
                    "[i \\in 1..{length} |-> IF i = 1 THEN (IF {}({key}) THEN {cur} ELSE FALSE) ELSE {name}[{key}][i - 1]]",
                    names.ok
                ))),
                name,
            })
        };
        let trigger = if upper == 0 {
            ctx.atom(self.trigger, false)?
        } else {
            let past = history(ordinal as u64 * 2, upper, self.trigger)?;
            let trigger = format!("{}[{key}][{upper}]", past.name);
            variables.push(past);
            trigger
        };
        let ready = ctx.atom(self.ready, false)?;
        let mut test = format!("{age}[{key}] = {upper} /\\ {trigger} /\\ ~({ready})");
        let length = upper - self.lower;
        if length > 0 {
            let past = history(ordinal as u64 * 2 + 1, length, self.ready)?;
            test.push_str(&format!(
                " /\\ (\\A i \\in 1..{length} : ~{}[{key}][i])",
                past.name
            ));
            variables.push(past);
        }
        Ok((variables, test))
    }
}

/// A use of the formula still to turn into closure nodes.
#[derive(Clone, Copy)]
enum Visit<'s> {
    /// A conjunct of the clause, negated.
    Conjunct(TfId, Site<'s>),
    Formula {
        tf: TfId,
        positive: bool,
        site: Site<'s>,
        /// The innermost checked node at or around this use.
        node: Option<usize>,
    },
}

/// What an entered use adds once its operands have their nodes.
enum Add {
    Nothing,
    Binary {
        and: bool,
        node: Option<usize>,
    },
    Window {
        interval: Iv,
        until: bool,
        node: Option<usize>,
    },
}

/// The walk that builds the closure; `done` holds the operands not yet taken.
struct Negation<'c, 's> {
    closure: &'c mut Closure,
    formulas: &'c Formulas,
    done: Vec<usize>,
    sites: PhantomData<Site<'s>>,
}

impl<'s> Negation<'_, 's> {
    fn leaf(&mut self, leaf: Node, node: Option<usize>) -> Result<Add, Refused> {
        let at = self.closure.add(leaf).map_err(|error| error.at(node))?;
        self.done.push(at);
        Ok(Add::Nothing)
    }

    fn conjunct(
        &mut self,
        tf: TfId,
        site: Site<'s>,
        children: &mut Children<'_, Visit<'s>>,
    ) -> Result<Add, Refused> {
        if let &Tf::And(left, right) = self.formulas.get(tf).map_err(|_| Refused::Broken)? {
            children.extend([
                Visit::Conjunct(left, site.operand(0)),
                Visit::Conjunct(right, site.operand(1)),
            ]);
            return Ok(Add::Binary {
                and: false,
                node: site.node(),
            });
        }
        let Some(mut response) = Response::of(self.formulas, tf) else {
            children.push(Visit::Formula {
                tf,
                positive: false,
                site,
                node: site.node(),
            });
            return Ok(Add::Nothing);
        };
        // The interval is the right operand of the implication under `always`.
        response.node = site.operand(0).operand(1).node();
        let at = self
            .closure
            .response_counterexample(response)
            .map_err(|_| Refused::Limit(response.node))?;
        self.done.push(at);
        Ok(Add::Nothing)
    }

    fn formula(
        &mut self,
        (tf, positive): (TfId, bool),
        site: Site<'s>,
        node: Option<usize>,
        children: &mut Children<'_, Visit<'s>>,
    ) -> Result<Add, Refused> {
        let operand = |tf, positive, index| {
            let site = site.operand(index);
            Visit::Formula {
                tf,
                positive,
                site,
                node: site.node().or(node),
            }
        };
        match self.formulas.get(tf).map_err(|_| Refused::Broken)? {
            &Tf::Const(value) => self.leaf(Node::Constant(positive == value), node),
            &Tf::Atom(index) => self.leaf(Node::Atom { index, positive }, node),
            &Tf::Not(inner) => {
                children.push(operand(inner, !positive, 0));
                Ok(Add::Nothing)
            }
            &Tf::And(left, right) => {
                children.extend([operand(left, positive, 0), operand(right, positive, 1)]);
                Ok(Add::Binary {
                    and: positive,
                    node,
                })
            }
            &Tf::Or(left, right) => {
                children.extend([operand(left, positive, 0), operand(right, positive, 1)]);
                Ok(Add::Binary {
                    and: !positive,
                    node,
                })
            }
            &Tf::Implies(left, right) => {
                children.extend([operand(left, !positive, 0), operand(right, positive, 1)]);
                Ok(Add::Binary {
                    and: !positive,
                    node,
                })
            }
            &Tf::Always(interval, inner) => {
                self.leaf(Node::Constant(!positive), node)?;
                children.push(operand(inner, positive, 0));
                Ok(Add::Window {
                    interval,
                    until: !positive,
                    node,
                })
            }
            &Tf::Eventually(interval, inner) => {
                self.leaf(Node::Constant(positive), node)?;
                children.push(operand(inner, positive, 0));
                Ok(Add::Window {
                    interval,
                    until: positive,
                    node,
                })
            }
            &Tf::Until(interval, left, right) => {
                children.extend([operand(left, positive, 0), operand(right, positive, 1)]);
                Ok(Add::Window {
                    interval,
                    until: positive,
                    node,
                })
            }
            &Tf::Release(interval, left, right) => {
                children.extend([operand(left, positive, 0), operand(right, positive, 1)]);
                Ok(Add::Window {
                    interval,
                    until: !positive,
                    node,
                })
            }
            Tf::Hist(_)
            | Tf::Once(..)
            | Tf::Historically(..)
            | Tf::Since(..)
            | Tf::Triggered(..)
            | Tf::Timed(_) => Err(Refused::History),
        }
    }

    /// The nodes of the two operands, left first.
    fn operands(&mut self) -> Result<(usize, usize), Refused> {
        let right = self.done.pop().ok_or(Refused::Broken)?;
        let left = self.done.pop().ok_or(Refused::Broken)?;
        Ok((left, right))
    }

    fn join(&mut self, add: Add) -> Result<(), Refused> {
        let at = match add {
            Add::Nothing => return Ok(()),
            Add::Binary { and, node } => {
                let (left, right) = self.operands()?;
                let joined = if and {
                    Node::And(left, right)
                } else {
                    Node::Or(left, right)
                };
                self.closure.add(joined).map_err(|error| error.at(node))?
            }
            Add::Window {
                interval,
                until,
                node,
            } => {
                let (left, right) = self.operands()?;
                self.closure
                    .window(interval, until, left, right)
                    .map_err(|error| error.at(node))?
            }
        };
        self.done.push(at);
        Ok(())
    }
}

impl<'s> Walk for Negation<'_, 's> {
    type Node = Visit<'s>;
    type Frame = Add;
    type Stop = Refused;

    fn enter(
        &mut self,
        visit: Visit<'s>,
        children: &mut Children<'_, Visit<'s>>,
    ) -> ControlFlow<Refused, Add> {
        let entered = match visit {
            Visit::Conjunct(tf, site) => self.conjunct(tf, site, children),
            Visit::Formula {
                tf,
                positive,
                site,
                node,
            } => self.formula((tf, positive), site, node, children),
        };
        match entered {
            Ok(add) => ControlFlow::Continue(add),
            Err(refused) => ControlFlow::Break(refused),
        }
    }

    fn exit(&mut self, add: Add) -> ControlFlow<Refused> {
        match self.join(add) {
            Ok(()) => ControlFlow::Continue(()),
            Err(refused) => ControlFlow::Break(refused),
        }
    }
}

#[derive(Default)]
struct Closure {
    nodes: Vec<Node>,
    positions: BTreeMap<Node, usize>,
    responses: Vec<Response>,
}

impl Closure {
    fn add(&mut self, node: Node) -> Result<usize, Refused> {
        if let Some(&at) = self.positions.get(&node) {
            return Ok(at);
        }
        let at = self.nodes.len();
        if at >= MAX_NODES {
            return Err(Refused::Limit(None));
        }
        self.nodes.push(node);
        self.positions.insert(node, at);
        Ok(at)
    }

    /// The nodes of the negation of the clause's formula `root`: some
    /// conjunct fails.
    fn counterexample(
        &mut self,
        formulas: &Formulas,
        root: TfId,
        site: Site<'_>,
    ) -> Result<usize, Refused> {
        let mut negation = Negation {
            closure: self,
            formulas,
            done: Vec::new(),
            sites: PhantomData,
        };
        if let ControlFlow::Break(refused) = walk(&mut negation, Visit::Conjunct(root, site)) {
            return Err(refused);
        }
        negation.done.pop().ok_or(Refused::Broken)
    }

    fn response_counterexample(&mut self, response: Response) -> Result<usize, Refused> {
        if response.upper >= MAX_NODES as u64 {
            return Err(Refused::Limit(None));
        }
        let violation = self.add(Node::ResponseViolation(self.responses.len()))?;
        self.responses.push(response);
        // A failed response can be reported at its deadline. At the origin,
        // eventually observing that failure is equivalent to negating the
        // obligation that every trigger receives a response in its window.
        let any = self.add(Node::Constant(true))?;
        self.add(Node::Until(any, violation))
    }

    /// Expand a finite window as recurrence equations with a fixed number
    /// of next-position obligations. The left operand starts at the lower
    /// bound, including when the upper bound is absent.
    fn window(
        &mut self,
        interval: Iv,
        until: bool,
        left: usize,
        right: usize,
    ) -> Result<usize, Refused> {
        let (lower, upper) = interval.bounds();
        if upper.is_some_and(|upper| upper < lower) {
            return self.add(Node::Constant(!until));
        }
        if lower >= MAX_NODES as u64 || upper.is_some_and(|upper| upper >= MAX_NODES as u64) {
            return Err(Refused::Limit(None));
        }
        let mut root = if let Some(upper) = upper {
            let mut step = right;
            for _ in lower..upper {
                let next = self.add(Node::Next(step))?;
                step = if until {
                    let both = self.add(Node::And(left, next))?;
                    self.add(Node::Or(right, both))?
                } else {
                    let either = self.add(Node::Or(left, next))?;
                    self.add(Node::And(right, either))?
                };
            }
            step
        } else {
            self.add(if until {
                Node::Until(left, right)
            } else {
                Node::Release(left, right)
            })?
        };
        for _ in 0..lower {
            root = self.add(Node::Next(root))?;
        }
        Ok(root)
    }

    /// An until guess may stay true only if its right operand eventually
    /// holds. Release needs no progress condition: its greatest fixed
    /// point admits every positive release guess with these equations.
    fn eventualities(&self) -> Vec<(usize, usize)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(at, node)| match node {
                Node::Until(_, right) => Some((at, *right)),
                _ => None,
            })
            .collect()
    }

    fn equations(&self, ctx: &Ctx<'_>, violations: &[String]) -> Lower<Vec<String>> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(at, node)| {
                let rhs = match *node {
                    Node::Constant(value) => super::boolean(value).to_owned(),
                    Node::Atom {
                        index,
                        positive: true,
                    } => ctx.atom(index, false)?,
                    Node::Atom {
                        index,
                        positive: false,
                    } => format!("~({})", ctx.atom(index, false)?),
                    Node::ResponseViolation(index) => violations[index].clone(),
                    Node::And(left, right) => format!("q[{left}] /\\ q[{right}]"),
                    Node::Or(left, right) => format!("q[{left}] \\/ q[{right}]"),
                    Node::Next(inner) => format!("r[{inner}]"),
                    Node::Until(left, right) => {
                        format!("q[{right}] \\/ (q[{left}] /\\ r[{at}])")
                    }
                    Node::Release(left, right) => {
                        format!("q[{right}] /\\ (q[{left}] \\/ r[{at}])")
                    }
                };
                Ok(format!("q[{at}] = ({rhs})"))
            })
            .collect()
    }
}

/// Admit the mixed persistence/recurrence fragment over distinct,
/// directly observed Boolean fields. Over their free alphabet, F G p and
/// G F q jointly need both acceptance conditions.
/// Setting independent response triggers to false reduces the formula to
/// that conjunction, so the bounded-response conjuncts do not eliminate
/// either condition. Formulas outside this fragment may have deterministic
/// monitors even when the current exporter cannot build one.
fn mixed_fragment(ctx: &Ctx<'_>, formulas: &Formulas, formula: TfId) -> bool {
    let Ok(leaves) = conjuncts(formulas, formula) else {
        return false;
    };
    let get = |id| formulas.get(id).ok();
    let mut persistence = None;
    let mut recurrence = None;
    let mut responses = Vec::new();
    for leaf in leaves {
        if let Some(response) = Response::of(formulas, leaf) {
            responses.extend([response.trigger, response.ready]);
            continue;
        }
        match get(leaf) {
            Some(&Tf::Eventually(outer, inner)) if !outer.is_some() => {
                let Some(&Tf::Always(inner, operand)) = get(inner) else {
                    return false;
                };
                let Some(&Tf::Atom(index)) = get(operand) else {
                    return false;
                };
                if inner.is_some() || persistence.replace(index).is_some() {
                    return false;
                }
            }
            Some(&Tf::Always(outer, inner)) if !outer.is_some() => {
                let Some(&Tf::Eventually(inner, operand)) = get(inner) else {
                    return false;
                };
                let Some(&Tf::Atom(index)) = get(operand) else {
                    return false;
                };
                if inner.is_some() || recurrence.replace(index).is_some() {
                    return false;
                }
            }
            _ => return false,
        }
    }
    let field = |index| {
        let exprs = ctx.input.show.exprs;
        let &E::Field {
            object,
            key,
            field,
            primed: false,
        } = exprs.get(*ctx.clause.cur.get(index)?).ok()?
        else {
            return None;
        };
        let value = ctx.input.objects.get(object)?.fields.get(field)?;
        (matches!(exprs.get(key), Ok(E::Param(name)) if name == ctx.key())
            && matches!(value.kind, Kind::Bool))
        .then_some((object, field))
    };
    let (Some(persistence), Some(recurrence)) =
        (persistence.and_then(field), recurrence.and_then(field))
    else {
        return false;
    };
    persistence != recurrence
        && !responses.is_empty()
        && responses
            .into_iter()
            .all(|index| field(index).is_some_and(|atom| atom != persistence && atom != recurrence))
}

pub(super) fn emit(ctx: &Ctx<'_>, out: &mut Vec<Definition>, extra: &mut Extra) -> Lower<()> {
    let clause = ctx.clause;
    let (formulas, formula) = (&clause.formulas, clause.tf);
    let names = &ctx.names;
    let anchor = clause.refuse_at.unwrap_or(clause.position);
    if !mixed_fragment(ctx, formulas, formula) {
        return defer_at(
            anchor,
            "temporal/nondeterministic-monitor",
            "a tableau outside the mixed persistence, recurrence and bounded response fragment",
        );
    }
    let mut closure = Closure::default();
    let site = Site {
        nodes: &clause.formula_sites,
        at: 0,
    };
    let root = match closure.counterexample(formulas, formula, site) {
        Ok(root) => root,
        Err(reason) => {
            let (node, detail) = match reason {
                Refused::Broken => (anchor, "a tableau over an incomplete formula"),
                Refused::History => (
                    anchor,
                    "a nondeterministic tableau with a history or timed operand",
                ),
                Refused::Limit(node) => (
                    node.unwrap_or(anchor),
                    "a nondeterministic tableau above the finite formula-closure limit",
                ),
            };
            return defer_at(node, "temporal/nondeterministic-monitor", detail);
        }
    };
    let key = ctx.key();
    let obligation = Some(clause.name.as_str());
    let emit = |out: &mut Vec<Definition>, name: &str, body: String| {
        out.push(define(
            clause.position,
            Some(name.to_owned()),
            body,
            obligation,
        ));
    };
    super::emit_ok(ctx, out)?;
    let monitor = &names.monitor;
    let live = format!("{monitor}_h0");
    let phase = format!("{monitor}_h1");
    let turn = format!("{monitor}_c0");
    let row_type = mangle("F_bits_", &clause.name);
    let equation = mangle("F_check_", &clause.name);
    let check_turn = mangle("F_progress_", &clause.name);
    let ids = &names.ids;
    let choices = format!("[{ids} -> {row_type}]");
    emit(
        out,
        &row_type,
        format!("{row_type} == [0..{} -> BOOLEAN]", closure.nodes.len() - 1),
    );
    let mut variables = vec![
        (monitor.clone(), clause.position),
        (live.clone(), clause.position),
        (phase.clone(), clause.position),
    ];
    let mut types = vec![
        format!("{monitor} \\in {choices}"),
        format!("{live} \\in [{ids} -> BOOLEAN]"),
        format!("{phase} \\in [{ids} -> BOOLEAN]"),
    ];
    let mut initial = vec![
        "Init".to_owned(),
        // Assign the entire row before an initializer reads it.
        format!("{monitor} \\in {choices}"),
        format!(
            "{live} = {}",
            ctx.per_key(&format!("{monitor}[{key}][{root}]"))
        ),
        format!("{phase} = {}", ctx.per_key("FALSE")),
    ];
    let mut update = vec![
        format!("{monitor}' \\in {choices}"),
        // Each guess is permitted. A wrong row can only set the sticky
        // validity bit to false, never block a subject action.
        format!(
            "{live}' = {}",
            ctx.per_key(&format!(
                "{live}[{key}] /\\ {equation}({key}, {monitor}[{key}], {monitor}'[{key}])"
            ))
        ),
        // Over any nonempty identity universe this makes each real step
        // change the product. With no identity the theorem is vacuous.
        format!("{phase}' = {}", ctx.per_key(&format!("~{phase}[{key}]"))),
    ];
    let mut violations = Vec::new();
    for (ordinal, response) in closure.responses.iter().enumerate() {
        let (registers, violation) = response.monitor(ctx, ordinal)?;
        violations.push(violation);
        for var in registers {
            types.push(format!("{} \\in {}", var.name, var.ty));
            if let Some(init) = var.init {
                initial.push(format!("{} = {init}", var.name));
            }
            if let Some(next) = var.update {
                update.push(format!("{}' = {next}", var.name));
            }
            variables.push((var.name, response.node.unwrap_or(clause.position)));
        }
    }
    let equations = closure
        .equations(ctx, &violations)?
        .into_iter()
        .map(|row| format!("    /\\ {row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let ok = &names.ok;
    emit(
        out,
        &equation,
        format!("{equation}({key}, q, r) ==\n  IF {ok}({key})\n  THEN\n{equations}\n  ELSE FALSE"),
    );
    let eventualities = closure.eventualities();
    let accept = if eventualities.is_empty() {
        format!("{live}[{key}]")
    } else {
        let last = eventualities.len();
        let arms: Vec<_> = eventualities
            .iter()
            .enumerate()
            .map(|(index, (at, right))| format!("i = {} -> (~q[{at}] \\/ q[{right}])", index + 1))
            .collect();
        emit(
            out,
            &check_turn,
            format!("{check_turn}(i, q) == CASE {}", arms.join(" [] ")),
        );
        variables.push((turn.clone(), clause.position));
        types.push(format!("{turn} \\in [{ids} -> 1..{last}]"));
        initial.push(format!("{turn} = {}", ctx.per_key("1")));
        update.push(format!("{turn}' = {}", ctx.per_key(&format!(
            "IF {check_turn}({turn}[{key}], {monitor}[{key}]) THEN (IF {turn}[{key}] = {last} THEN 1 ELSE {turn}[{key}] + 1) ELSE {turn}[{key}]"
        ))));
        format!("{live}[{key}] /\\ {turn}[{key}] = {last} /\\ {check_turn}({turn}[{key}], {monitor}[{key}])")
    };
    let vars = &names.vars;
    let type_ok = &names.type_ok;
    let init = &names.init;
    let mon = &names.mon;
    let next = &names.next;
    let spec = &names.spec;
    let accept_name = &names.accept;
    let theorem = &names.theorem;
    let variable_names: Vec<_> = variables.iter().map(|(name, _)| name.as_str()).collect();
    emit(
        out,
        vars,
        format!("{vars} == <<vars, {}>>", variable_names.join(", ")),
    );
    emit(out, type_ok, format!("{type_ok} ==\n{}", bullets(&types)));
    initial.push(type_ok.clone());
    emit(out, init, format!("{init} ==\n{}", bullets(&initial)));
    emit(out, mon, format!("{mon} ==\n{}", bullets(&update)));
    emit(
        out,
        next,
        format!("{next} == (Next \\/ (~ENABLED Next /\\ UNCHANGED vars)) /\\ {mon}"),
    );
    // A monitor-only change is a product step but cannot satisfy the
    // subject's fairness on state-changing steps.
    let mut spec_body =
        format!("{spec} == {init} /\\ [][{next}]_{vars} /\\ WF_{vars}({next}) /\\ WF_vars(Next)");
    for condition in fairness(ctx)? {
        spec_body.push_str(" /\\ ");
        spec_body.push_str(&condition);
    }
    emit(out, spec, spec_body);
    emit(
        out,
        accept_name,
        format!("{accept_name}({key}) == {accept}"),
    );
    let statement = ctx.guarded(&format!("~([]<>{accept_name}({key}))"));
    emit(
        out,
        theorem,
        format!("THEOREM {theorem} == {spec} => {statement}"),
    );
    extra
        .variables
        .extend(variables.iter().map(|(name, _)| name.clone()));
    extra.aux_names.extend(variables);
    extra.conditional.push(Cond {
        obligation: clause.name.clone(),
        kind: "nondeterministic-monitor",
        node: clause.position,
        type_name: None,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Closure, FormulaSite, Formulas, Iv, Node, Refused, Site, Tf, TfId, MAX_NODES};
    use ix_trace_rs::trace;

    fn bit(assignment: usize, node: usize) -> bool {
        assignment & (1 << node) != 0
    }

    #[test]
    #[trace("FSpec-TC-1972", "FSpec-FR-813-AC-24")]
    fn tableau_names_the_conjunction_when_only_its_combination_overflows() {
        let mut closure = Closure::default();
        for index in 0..MAX_NODES {
            closure
                .add(Node::Atom {
                    index,
                    positive: false,
                })
                .unwrap();
        }
        let mut formulas = Formulas::default();
        let left = formulas.add(Tf::Atom(0));
        let right = formulas.add(Tf::Atom(1));
        let formula = formulas.add(Tf::And(left, right));
        let nodes = [
            FormulaSite {
                position: 90,
                operands: vec![1, 2],
            },
            FormulaSite {
                position: 91,
                operands: Vec::new(),
            },
            FormulaSite {
                position: 92,
                operands: Vec::new(),
            },
        ];
        assert!(matches!(
            closure.counterexample(
                &formulas,
                formula,
                Site {
                    nodes: &nodes,
                    at: 0
                }
            ),
            Err(Refused::Limit(Some(90)))
        ));
    }

    /// Search the graph of compatible rows over a finite presentation of an
    /// infinite word. A reachable cycle accepts when its strongly connected
    /// component meets every eventuality's acceptance set.
    fn accepts_negation<const N: usize>(
        (formulas, formula): (&Formulas, TfId),
        word: &[[bool; N]],
        repeat_at: usize,
    ) -> bool {
        let mut closure = Closure::default();
        let root = closure
            .counterexample(formulas, formula, Site::default())
            .expect("small future formula");
        // Extend the finite prefix until every position in the loop has
        // the same history on each repetition.
        let history = closure.responses.iter().map(|r| r.upper).max().unwrap_or(0) as usize;
        let cycle = &word[repeat_at..];
        let mut positions = word.to_vec();
        let repeat_at = if history == 0 {
            repeat_at
        } else {
            positions.extend((0..history + cycle.len()).map(|i| cycle[i % cycle.len()]));
            word.len() + history
        };
        let word = positions.as_slice();
        let follows = |current, following, at, node| {
            let held = |index| bit(current, index);
            let next = |index| bit(following, index);
            match node {
                Node::Next(inner) => held(at) == next(inner),
                Node::Until(left, right) => held(at) == (held(right) || (held(left) && next(at))),
                Node::Release(left, right) => held(at) == (held(right) && (held(left) || next(at))),
                _ => true,
            }
        };
        let mut rows = Vec::new();
        for (position, atoms) in word.iter().enumerate() {
            let violations: Vec<_> = closure
                .responses
                .iter()
                .map(|r| {
                    let lower = r.lower as usize;
                    let upper = r.upper as usize;
                    position >= upper
                        && word[position - upper][r.trigger]
                        && word[position - upper + lower..=position]
                            .iter()
                            .all(|row| !row[r.ready])
                })
                .collect();
            for guess in 0..(1 << closure.nodes.len()) {
                let held = |index| bit(guess, index);
                let is_local = closure.nodes.iter().enumerate().all(|(at, node)| {
                    held(at)
                        == match *node {
                            Node::Constant(value) => value,
                            Node::Atom { index, positive } => atoms[index] == positive,
                            Node::ResponseViolation(index) => violations[index],
                            Node::And(left, right) => held(left) && held(right),
                            Node::Or(left, right) => held(left) || held(right),
                            Node::Next(_) | Node::Until(..) | Node::Release(..) => held(at),
                        }
                });
                if is_local {
                    rows.push((position, guess));
                }
            }
        }
        let mut reach = vec![vec![false; rows.len()]; rows.len()];
        for (from, &(position, guess)) in rows.iter().enumerate() {
            let next_position = if position + 1 == word.len() {
                repeat_at
            } else {
                position + 1
            };
            for (to, &(candidate, following)) in rows.iter().enumerate() {
                reach[from][to] = candidate == next_position
                    && closure
                        .nodes
                        .iter()
                        .enumerate()
                        .all(|(at, node)| follows(guess, following, at, *node));
            }
        }
        for (through, _) in rows.iter().enumerate() {
            let paths = reach[through].clone();
            for row in &mut reach {
                if row[through] {
                    for (hold, via) in row.iter_mut().zip(&paths) {
                        *hold |= *via;
                    }
                }
            }
        }
        let eventualities = closure.eventualities();
        (0..rows.len()).any(|cycle| {
            reach[cycle][cycle]
                && rows.iter().enumerate().any(|(from, &(position, guess))| {
                    position == 0 && bit(guess, root) && reach[from][cycle]
                })
                && eventualities.iter().all(|&(until, right)| {
                    rows.iter().enumerate().any(|(row, &(_, guess))| {
                        reach[cycle][row]
                            && reach[row][cycle]
                            && (!bit(guess, until) || bit(guess, right))
                    })
                })
        })
    }

    #[test]
    #[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
    fn tableau_accepts_exactly_the_nested_until_violations() {
        let mut formulas = Formulas::default();
        let a = formulas.add(Tf::Atom(0));
        let b = formulas.add(Tf::Atom(1));
        let until = formulas.add(Tf::Until(Iv::none(), a, b));
        let root = formulas.add(Tf::Always(Iv::none(), until));
        let formula = (&formulas, root);
        assert!(accepts_negation(
            formula,
            &[[false, false], [true, true]],
            1
        ));
        assert!(accepts_negation(formula, &[[true, false]], 0));
        assert!(!accepts_negation(
            formula,
            &[[true, false], [true, true]],
            1
        ));
        assert!(!accepts_negation(formula, &[[true, true]], 0));
    }

    /// `always (atom 0 implies eventually[1, upper] atom 1)`.
    fn response(formulas: &mut Formulas, upper: u64) -> TfId {
        let a = formulas.add(Tf::Atom(0));
        let b = formulas.add(Tf::Atom(1));
        let eventually = formulas.add(Tf::Eventually(Iv(Some((1, Some(upper)))), b));
        let implies = formulas.add(Tf::Implies(a, eventually));
        formulas.add(Tf::Always(Iv::none(), implies))
    }

    #[test]
    #[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
    fn tableau_distinguishes_the_following_position_at_a_deadline() {
        let mut formulas = Formulas::default();
        let root = response(&mut formulas, 1);
        let formula = (&formulas, root);
        assert!(!accepts_negation(
            formula,
            &[[true, false], [false, true]],
            1
        ));
        assert!(accepts_negation(
            formula,
            &[[true, true], [false, false]],
            1
        ));
        assert!(accepts_negation(
            formula,
            &[[true, false], [false, false], [false, true]],
            2
        ));
        assert!(accepts_negation(
            formula,
            &[[false, false], [true, false]],
            1
        ));
    }

    #[test]
    #[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
    fn tableau_reads_both_window_ends_after_the_first_deadline() {
        let mut formulas = Formulas::default();
        let root = response(&mut formulas, 2);
        let formula = (&formulas, root);
        for word in [
            [[true, false], [false, true], [false, false]],
            [[true, false], [false, false], [false, true]],
            [[false, true], [false, false], [false, false]],
        ] {
            assert!(!accepts_negation(formula, &word, 2));
        }
        for word in [
            [[true, true], [false, false], [false, false], [false, false]],
            [[true, false], [false, false], [false, false], [false, true]],
        ] {
            assert!(accepts_negation(formula, &word, 3));
        }
    }

    #[test]
    #[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
    fn tableau_accepts_each_way_a_mixed_obligation_can_fail() {
        let mut formulas = Formulas::default();
        let [p, q, trigger, ready] = [0, 1, 2, 3].map(|index| formulas.add(Tf::Atom(index)));
        let inner = formulas.add(Tf::Always(Iv::none(), p));
        let persistence = formulas.add(Tf::Eventually(Iv::none(), inner));
        let inner = formulas.add(Tf::Eventually(Iv::none(), q));
        let recurrence = formulas.add(Tf::Always(Iv::none(), inner));
        let eventually = formulas.add(Tf::Eventually(Iv(Some((1, Some(1)))), ready));
        let implies = formulas.add(Tf::Implies(trigger, eventually));
        let response = formulas.add(Tf::Always(Iv::none(), implies));
        let both = formulas.add(Tf::And(persistence, recurrence));
        let root = formulas.add(Tf::And(both, response));
        let formula = (&formulas, root);
        assert!(!accepts_negation(
            formula,
            &[
                [true, true, false, false],
                [true, false, true, false],
                [true, true, false, true]
            ],
            0,
        ));
        assert!(accepts_negation(formula, &[[false, true, false, false]], 0));
        assert!(accepts_negation(formula, &[[true, false, false, false]], 0));
        assert!(accepts_negation(formula, &[[true, true, true, false]], 0));

        // Lamp starts unlit. Repeating noop forever falsifies the clause,
        // but light remains enabled and changes the subject state. Every
        // base-fair trace reaches lit after finitely many noops, then stays
        // lit. The bounded response is vacuous on both traces.
        let unlit = [false, false, false, false];
        let lit = [true, true, false, false];
        assert!(accepts_negation(formula, &[unlit], 0));
        assert!(!accepts_negation(formula, &[unlit, lit], 1));
        assert!(!accepts_negation(formula, &[unlit, unlit, lit], 2));
    }
}
