// SPDX-License-Identifier: AGPL-3.0-or-later
//! A finite tableau for the negated clause. Invalid guesses reject the
//! monitor run without disabling any transition of the subject.

use super::{conjuncts, fairness, Cond, Ctx, Extra, Iv, Tf, Var};
use crate::command::export::lower::{
    bullets, defer_at, define, mangle, Definition, Kind, Lower, E,
};
use std::collections::BTreeMap;

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
    History,
    Limit(Option<usize>),
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
    fn of(tf: &Tf) -> Option<Self> {
        let Tf::Always(iv, inner) = tf else {
            return None;
        };
        if iv.is_some() {
            return None;
        }
        let Tf::Implies(trigger, response) = &**inner else {
            return None;
        };
        let (Tf::Atom(trigger), Tf::Eventually(window, ready)) = (&**trigger, &**response) else {
            return None;
        };
        let Tf::Atom(ready) = &**ready else {
            return None;
        };
        Some(Self {
            lower: window.lower(),
            upper: window.upper()?,
            trigger: *trigger,
            ready: *ready,
            node: window.node,
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

    fn counterexample(&mut self, tf: &Tf) -> Result<usize, Refused> {
        if let Tf::And(left, right) = tf {
            let left = self.counterexample(left)?;
            let right = self.counterexample(right)?;
            return self.add(Node::Or(left, right));
        }
        let Some(response) = Response::of(tf) else {
            return self.formula(tf, false);
        };
        self.response_counterexample(response)
            .map_err(|_| Refused::Limit(response.node))
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

    fn formula(&mut self, tf: &Tf, positive: bool) -> Result<usize, Refused> {
        match tf {
            Tf::Const(value) => self.add(Node::Constant(positive == *value)),
            Tf::Atom(index) => self.add(Node::Atom {
                index: *index,
                positive,
            }),
            Tf::Not(inner) => self.formula(inner, !positive),
            Tf::And(left, right) | Tf::Or(left, right) => {
                let left = self.formula(left, positive)?;
                let right = self.formula(right, positive)?;
                self.add(if positive == matches!(tf, Tf::And(..)) {
                    Node::And(left, right)
                } else {
                    Node::Or(left, right)
                })
            }
            Tf::Implies(left, right) => {
                let left = self.formula(left, !positive)?;
                let right = self.formula(right, positive)?;
                self.add(if positive {
                    Node::Or(left, right)
                } else {
                    Node::And(left, right)
                })
            }
            Tf::Always(iv, inner) | Tf::Eventually(iv, inner) => {
                let eventually = positive == matches!(tf, Tf::Eventually(..));
                let left = self.add(Node::Constant(eventually))?;
                let right = self.formula(inner, positive)?;
                self.window(*iv, eventually, left, right)
            }
            Tf::Until(iv, left, right) | Tf::Release(iv, left, right) => {
                let until = positive == matches!(tf, Tf::Until(..));
                let left = self.formula(left, positive)?;
                let right = self.formula(right, positive)?;
                self.window(*iv, until, left, right)
            }
            Tf::Hist(_)
            | Tf::Once(..)
            | Tf::Historically(..)
            | Tf::Since(..)
            | Tf::Triggered(..)
            | Tf::Timed(_) => Err(Refused::History),
        }
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
            return Err(Refused::Limit(interval.node));
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
fn mixed_fragment(ctx: &Ctx<'_>, formula: &Tf) -> bool {
    let mut leaves = Vec::new();
    conjuncts(formula, &mut leaves);
    let mut persistence = None;
    let mut recurrence = None;
    let mut responses = Vec::new();
    for leaf in leaves {
        if let Some(response) = Response::of(leaf) {
            responses.extend([response.trigger, response.ready]);
            continue;
        }
        match leaf {
            Tf::Eventually(outer, inner) if !outer.is_some() => {
                let Tf::Always(inner, operand) = &**inner else {
                    return false;
                };
                let Tf::Atom(index) = &**operand else {
                    return false;
                };
                if inner.is_some() || persistence.replace(*index).is_some() {
                    return false;
                }
            }
            Tf::Always(outer, inner) if !outer.is_some() => {
                let Tf::Eventually(inner, operand) = &**inner else {
                    return false;
                };
                let Tf::Atom(index) = &**operand else {
                    return false;
                };
                if inner.is_some() || recurrence.replace(*index).is_some() {
                    return false;
                }
            }
            _ => return false,
        }
    }
    let field = |index| {
        let E::Field {
            object,
            key,
            field,
            primed: false,
        } = ctx.clause.cur.get(index)?
        else {
            return None;
        };
        let value = ctx.input.objects.get(*object)?.fields.get(*field)?;
        (matches!(&**key, E::Param(name) if name == ctx.key()) && matches!(value.kind, Kind::Bool))
            .then_some((*object, *field))
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

pub(super) fn emit(
    ctx: &Ctx<'_>,
    formula: &Tf,
    out: &mut Vec<Definition>,
    extra: &mut Extra,
) -> Lower<()> {
    let clause = ctx.clause;
    let names = &ctx.names;
    let anchor = clause.refuse_at.unwrap_or(clause.position);
    if !mixed_fragment(ctx, formula) {
        return defer_at(
            anchor,
            "temporal/nondeterministic-monitor",
            "a tableau outside the mixed persistence, recurrence and bounded response fragment",
        );
    }
    let mut closure = Closure::default();
    let root = match closure.counterexample(formula) {
        Ok(root) => root,
        Err(reason) => {
            let (node, detail) = match reason {
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
    use super::{Closure, Iv, Node, Tf};
    use ix_trace_rs::trace;

    fn bit(assignment: usize, node: usize) -> bool {
        assignment & (1 << node) != 0
    }

    /// Search the graph of compatible rows over a finite presentation of an
    /// infinite word. A reachable cycle accepts when its strongly connected
    /// component meets every eventuality's acceptance set.
    fn accepts_negation<const N: usize>(
        formula: &Tf,
        word: &[[bool; N]],
        repeat_at: usize,
    ) -> bool {
        let mut closure = Closure::default();
        let root = closure
            .counterexample(formula)
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
        let a = Box::new(Tf::Atom(0));
        let b = Box::new(Tf::Atom(1));
        let formula = Tf::Always(Iv::none(), Box::new(Tf::Until(Iv::none(), a, b)));
        assert!(accepts_negation(
            &formula,
            &[[false, false], [true, true]],
            1
        ));
        assert!(accepts_negation(&formula, &[[true, false]], 0));
        assert!(!accepts_negation(
            &formula,
            &[[true, false], [true, true]],
            1
        ));
        assert!(!accepts_negation(&formula, &[[true, true]], 0));
    }

    #[test]
    #[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
    fn tableau_distinguishes_the_following_position_at_a_deadline() {
        let a = Box::new(Tf::Atom(0));
        let b = Box::new(Tf::Atom(1));
        let interval = Iv {
            bounds: Some((1, Some(1))),
            node: None,
        };
        let response = Tf::Eventually(interval, b);
        let formula = Tf::Always(Iv::none(), Box::new(Tf::Implies(a, Box::new(response))));
        assert!(!accepts_negation(
            &formula,
            &[[true, false], [false, true]],
            1
        ));
        assert!(accepts_negation(
            &formula,
            &[[true, true], [false, false]],
            1
        ));
        assert!(accepts_negation(
            &formula,
            &[[true, false], [false, false], [false, true]],
            2
        ));
        assert!(accepts_negation(
            &formula,
            &[[false, false], [true, false]],
            1
        ));
    }

    #[test]
    #[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
    fn tableau_reads_both_window_ends_after_the_first_deadline() {
        let trigger = Box::new(Tf::Atom(0));
        let ready = Box::new(Tf::Atom(1));
        let interval = Iv {
            bounds: Some((1, Some(2))),
            node: None,
        };
        let response = Tf::Eventually(interval, ready);
        let formula = Tf::Always(
            Iv::none(),
            Box::new(Tf::Implies(trigger, Box::new(response))),
        );
        for word in [
            [[true, false], [false, true], [false, false]],
            [[true, false], [false, false], [false, true]],
            [[false, true], [false, false], [false, false]],
        ] {
            assert!(!accepts_negation(&formula, &word, 2));
        }
        for word in [
            [[true, true], [false, false], [false, false], [false, false]],
            [[true, false], [false, false], [false, false], [false, true]],
        ] {
            assert!(accepts_negation(&formula, &word, 3));
        }
    }

    #[test]
    #[trace("FSpec-TC-1972", "FSpec-FR-813-AC-6")]
    fn tableau_accepts_each_way_a_mixed_obligation_can_fail() {
        let g = |inner| Tf::Always(Iv::none(), Box::new(inner));
        let f = |inner| Tf::Eventually(Iv::none(), Box::new(inner));
        let and = |a, b| Tf::And(Box::new(a), Box::new(b));
        let interval = Iv {
            bounds: Some((1, Some(1))),
            node: None,
        };
        let response = Tf::Eventually(interval, Box::new(Tf::Atom(3)));
        let response = g(Tf::Implies(Box::new(Tf::Atom(2)), Box::new(response)));
        let formula = and(and(f(g(Tf::Atom(0))), g(f(Tf::Atom(1)))), response);
        assert!(!accepts_negation(
            &formula,
            &[
                [true, true, false, false],
                [true, false, true, false],
                [true, true, false, true]
            ],
            0,
        ));
        assert!(accepts_negation(
            &formula,
            &[[false, true, false, false]],
            0
        ));
        assert!(accepts_negation(
            &formula,
            &[[true, false, false, false]],
            0
        ));
        assert!(accepts_negation(&formula, &[[true, true, true, false]], 0));

        // Lamp starts unlit. Repeating noop forever falsifies the clause,
        // but light remains enabled and changes the subject state. Every
        // base-fair trace reaches lit after finitely many noops, then stays
        // lit. The bounded response is vacuous on both traces.
        let unlit = [false, false, false, false];
        let lit = [true, true, false, false];
        assert!(accepts_negation(&formula, &[unlit], 0));
        assert!(!accepts_negation(&formula, &[unlit, lit], 1));
        assert!(!accepts_negation(&formula, &[unlit, unlit, lit], 2));
    }
}
