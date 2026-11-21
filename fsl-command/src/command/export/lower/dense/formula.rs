// SPDX-License-Identifier: AGPL-3.0-or-later
//! Clock-reading monitors for FR-813's admitted timed intervals.

use super::Quantity;
use crate::command::export::lower::temporal::{ClauseLow, Tf, CONSTRUCT};
use crate::command::export::lower::{
    bullets, defer_at, define, mangle, refusal, Definition, Graph, Lower,
};
use filament_contract_model::{CheckedIntervalEnd, CheckedOperationInterval};
use fsl_package::checked_views::{FormulaNodeUse, FormulaOperation};

#[derive(Clone, Copy, PartialEq)]
pub(in crate::command::export::lower) enum Operator {
    Always,
    Eventually,
}

#[derive(Clone, PartialEq)]
pub(in crate::command::export::lower) struct Window {
    pub(in crate::command::export::lower) operator: Operator,
    pub(in crate::command::export::lower) predicate: Tf,
    at: usize,
    lower: Quantity,
    upper: Option<Quantity>,
    lower_open: bool,
    upper_open: bool,
}

impl Window {
    pub(in crate::command::export::lower) fn read(
        graph: &Graph<'_>,
        kind: FormulaOperation,
        node: &FormulaNodeUse<'_>,
        at: usize,
        mut operand: impl FnMut(usize) -> Lower<Tf>,
    ) -> Lower<Self> {
        let operator = match kind {
            FormulaOperation::Always => Operator::Always,
            FormulaOperation::Eventually => Operator::Eventually,
            _ => {
                return defer_at(
                    at,
                    &graph.construct(at),
                    "a timed interval outside always or eventually",
                );
            }
        };
        let Some(CheckedOperationInterval::Timed {
            lower,
            upper,
            lower_end,
            upper_end,
        }) = &node.interval
        else {
            return Err(refusal(
                CONSTRUCT,
                "a timed operator without a timed interval",
            ));
        };
        let [child] = node.operands.as_slice() else {
            return Err(refusal(
                CONSTRUCT,
                "a unary timed operator without one operand",
            ));
        };
        let predicate = operand(*child)?;
        if !state_only(&predicate) {
            return defer_at(
                at,
                &graph.construct(at),
                "a timed interval with a nonstate operand",
            );
        }
        let quantity = |r: &filament_contract_model::CheckedOperationRational| Quantity {
            numerator: r.numerator.to_string(),
            denominator: r.denominator.to_string(),
        };
        Ok(Self {
            operator,
            predicate,
            at,
            lower: quantity(lower),
            upper: upper.as_ref().map(quantity),
            lower_open: *lower_end == CheckedIntervalEnd::Open,
            upper_open: *upper_end == CheckedIntervalEnd::Open,
        })
    }

    pub(in crate::command::export::lower) fn lean_bounds(&self, elapsed: &str) -> String {
        let lower = if self.lower_open { "Q_lt" } else { "Q_le" };
        let mut terms = vec![format!("{lower} {} {elapsed}", self.lower.lean())];
        if let Some(upper) = &self.upper {
            let cmp = if self.upper_open { "Q_lt" } else { "Q_le" };
            terms.push(format!("{cmp} {elapsed} {}", upper.lean()));
        }
        terms.join(" /\\ ")
    }

    fn inside(&self, elapsed: &str) -> String {
        let cmp = if self.lower_open { "<" } else { "<=" };
        let mut terms = vec![format!("{} {cmp} {elapsed}", self.lower.tla())];
        if let Some(upper) = &self.upper {
            let cmp = if self.upper_open { "<" } else { "<=" };
            terms.push(format!("{elapsed} {cmp} {}", upper.tla()));
        }
        format!("({})", terms.join(" /\\ "))
    }

    fn expired(&self, elapsed: &str) -> String {
        self.upper.as_ref().map_or_else(
            || "FALSE".to_owned(),
            |upper| {
                let cmp = if self.upper_open { ">=" } else { ">" };
                format!("{elapsed} {cmp} {}", upper.tla())
            },
        )
    }
}

fn state_only(tf: &Tf) -> bool {
    match tf {
        Tf::Const(_) | Tf::Atom(_) => true,
        Tf::Not(inner) => state_only(inner),
        Tf::And(left, right) | Tf::Or(left, right) | Tf::Implies(left, right) => {
            state_only(left) && state_only(right)
        }
        _ => false,
    }
}

fn has_window(tf: &Tf) -> bool {
    match tf {
        Tf::Timed(_) => true,
        Tf::And(left, right) => has_window(left) || has_window(right),
        _ => false,
    }
}

fn direct(
    clause: &ClauseLow,
    tf: &Tf,
    state: &impl Fn(&Tf, bool) -> Lower<String>,
) -> Lower<String> {
    let child = |tf: &Tf| direct(clause, tf, state);
    Ok(match tf {
        Tf::Always(iv, inner) if !iv.is_some() => format!("[](tick \\/ {})", child(inner)?),
        Tf::Eventually(iv, inner) if !iv.is_some() => format!("<>(~tick /\\ {})", child(inner)?),
        Tf::Not(inner) => format!("~({})", child(inner)?),
        Tf::And(a, b) => format!("({} /\\ {})", child(a)?, child(b)?),
        Tf::Or(a, b) => format!("({} \\/ {})", child(a)?, child(b)?),
        Tf::Implies(a, b) => format!("({} => {})", child(a)?, child(b)?),
        Tf::Const(_) | Tf::Atom(_) => state(tf, false)?,
        _ => {
            return defer_at(
                clause.refuse_at.unwrap_or(clause.position),
                super::super::temporal::CONSTRUCT,
                "a timed formula outside the direct always and eventually forms",
            );
        }
    })
}

struct Monitor {
    name: String,
    domain: &'static str,
    init: String,
    next: String,
    node: usize,
}

struct Clause<'a, ShowState> {
    clause: &'a ClauseLow,
    show_state: ShowState,
    monitors: Vec<Monitor>,
    statements: Vec<String>,
    accepts: Vec<(String, String, usize)>,
    ordinal: usize,
}

impl<ShowState: Fn(&Tf, bool) -> Lower<String>> Clause<'_, ShowState> {
    fn part(&mut self, tf: &Tf, repeated: bool) -> Lower<()> {
        match tf {
            Tf::And(left, right) => {
                self.part(left, repeated)?;
                self.part(right, repeated)?;
            }
            Tf::Always(iv, inner) if !iv.is_some() && has_window(inner) => {
                self.part(inner, true)?
            }
            Tf::Timed(window) if repeated => self.repeated_monitor(window)?,
            Tf::Timed(window) => self.monitor(window)?,
            _ => {
                let text = direct(self.clause, tf, &self.show_state)?;
                self.statements.push(if repeated {
                    format!("[](tick \\/ {text})")
                } else {
                    text
                });
            }
        }
        Ok(())
    }

    fn monitor(&mut self, window: &Window) -> Lower<()> {
        let idx = self.ordinal;
        self.ordinal += 1;
        let node = window.at;
        let key = &self.clause.binder;
        let current = (self.show_state)(&window.predicate, false)?;
        let next = (self.show_state)(&window.predicate, true)?;
        let base = mangle("m_", &self.clause.name);
        let accept = mangle("accept_", &self.clause.name);
        let accept = if idx == 0 {
            accept
        } else {
            format!("{accept}_{idx}")
        };
        let name = if idx == 0 {
            base
        } else {
            format!("{base}_{idx}")
        };
        let at = |time: &str, predicate: &str| {
            let within = window.inside(time);
            let expired = window.expired(time);
            match window.operator {
                Operator::Always => format!("IF {within} /\\ ~({predicate}) THEN \"rej\" ELSE IF {expired} THEN \"acc\" ELSE \"run\""),
                Operator::Eventually => format!("IF {within} /\\ {predicate} THEN \"acc\" ELSE IF {expired} THEN \"rej\" ELSE \"run\""),
            }
        };
        self.monitors.push(Monitor {
            name: name.clone(),
            domain: "{\"run\", \"acc\", \"rej\"}",
            node,
            init: at("0", &current),
            next: format!(
                "IF {name}[{key}] /= \"run\" THEN {name}[{key}] ELSE {}",
                at("now'", &next)
            ),
        });
        let (base, statement) = match window.operator {
            Operator::Always => (
                format!("{name}[{key}] /= \"rej\""),
                format!("[]{accept}({key})"),
            ),
            Operator::Eventually => (
                format!("{name}[{key}] = \"acc\""),
                format!("[]<>{accept}({key})"),
            ),
        };
        self.accepts.push((accept, base, node));
        self.statements.push(statement);
        Ok(())
    }

    fn repeated_monitor(&mut self, window: &Window) -> Lower<()> {
        let idx = self.ordinal;
        self.ordinal += 1;
        let node = window.at;
        let key = &self.clause.binder;
        let current = (self.show_state)(&window.predicate, false)?;
        let next = (self.show_state)(&window.predicate, true)?;
        let base = mangle("m_", &self.clause.name);
        let starts = format!("{base}_s{idx}");
        let reject = format!("{base}_r{idx}");
        let accept = mangle("accept_", &self.clause.name);
        let accept = if idx == 0 {
            accept
        } else {
            format!("{accept}_{idx}")
        };
        let origin = window.inside("0");
        let duration = "now' - s";
        let inside = window.inside(duration);
        let expired = window.expired(duration);
        let open = format!("{starts}[{key}] \\cup {{now'}}");
        let (initial, retain, initial_reject, fail) = match window.operator {
            Operator::Always => (
                format!("IF {} THEN {{}} ELSE {{0}}", window.expired("0")),
                format!("~({expired})"),
                format!("{origin} /\\ ~({current})"),
                format!("\\E s \\in ({open}) : {inside} /\\ ~({next})"),
            ),
            Operator::Eventually => (
                format!("IF {origin} /\\ {current} THEN {{}} ELSE {{0}}"),
                format!("~({inside} /\\ {next})"),
                window.expired("0"),
                format!("\\E s \\in ({open}) : {expired}"),
            ),
        };
        self.monitors.push(Monitor {
            name: starts.clone(),
            domain: "SUBSET Real",
            node,
            init: initial,
            next: format!("{{s \\in ({open}) : {retain}}}"),
        });
        self.monitors.push(Monitor {
            name: reject.clone(),
            domain: "BOOLEAN",
            node,
            init: initial_reject,
            next: format!("{reject}[{key}] \\/ ({fail})"),
        });
        self.accepts
            .push((accept.clone(), format!("~{reject}[{key}]"), node));
        self.statements.push(format!("[]{accept}({key})"));
        // Time divergence ensures every fixed start has a last occurrence.
        // For an upper-unbounded eventuality, each such start still needs
        // a witness that removes it from the open set.
        if window.operator == Operator::Eventually && window.upper.is_none() {
            self.statements.push(format!(
                "(\\A s \\in Real : []<>(s \\notin {starts}[{key}]))"
            ));
        }
        Ok(())
    }
}

pub(in crate::command::export::lower) fn tla_clause(
    clause: &ClauseLow,
    ids: &str,
    ok: &str,
    show_state: impl Fn(&Tf, bool) -> Lower<String>,
    out: &mut Vec<Definition>,
) -> Lower<Vec<(String, usize)>> {
    let mut lowering = Clause {
        clause,
        show_state,
        monitors: Vec::new(),
        statements: Vec::new(),
        accepts: Vec::new(),
        ordinal: 0,
    };
    lowering.part(&clause.tf, false)?;
    let mut variables = Vec::new();
    let obligation = Some(clause.name.as_str());
    let key = &clause.binder;
    let spec = if lowering.monitors.is_empty() {
        "Spec".to_owned()
    } else {
        let name = |prefix: &str| mangle(prefix, &clause.name);
        let init_name = name("Init_");
        let update_name = name("Mon_");
        let next_name = name("Next_");
        let vars = name("vars_");
        let spec = name("Spec_");
        let type_ok = name("TypeOK_");
        let mut init = vec!["Init".to_owned()];
        let mut next = Vec::new();
        let mut types = Vec::new();
        for mon in lowering.monitors {
            let name = mon.name;
            init.push(format!("{name} = [{key} \\in {ids} |-> {}]", mon.init));
            next.push(format!("{name}' = [{key} \\in {ids} |-> {}]", mon.next));
            types.push(format!("{name} \\in [{ids} -> {}]", mon.domain));
            variables.push((name, mon.node));
        }
        let names = variables
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        init.push(type_ok.clone());
        for (name, body) in [
            (vars.clone(), format!(" <<vars, {names}>>")),
            (type_ok, format!("\n{}", bullets(&types))),
            (init_name.clone(), format!("\n{}", bullets(&init))),
            (update_name.clone(), format!("\n{}", bullets(&next))),
            (next_name.clone(), format!(" ((Next \\/ \\E d \\in Real : Idle(d)) /\\ {update_name}) \\/ (\\E d \\in Real : Tick(d) /\\ UNCHANGED <<{names}>>)")),
            (spec.clone(), format!(" {init_name} /\\ [][{next_name}]_{vars} /\\ WF_{vars}({next_name})")),
        ] {
            out.push(define(clause.position, Some(name.clone()), format!("{name} =={body}"), obligation));
        }
        spec
    };
    for (name, body, node) in lowering.accepts {
        out.push(define(
            node,
            Some(name.clone()),
            format!("{name}({key}) == {body}"),
            obligation,
        ));
    }
    let mut requirements = vec![format!("[](tick \\/ {ok}({key}))")];
    requirements.extend(lowering.statements);
    let theorem = mangle("Th_", &clause.name);
    out.push(define(
        clause.position,
        Some(theorem.clone()),
        format!(
            "THEOREM {theorem} == {spec} => ((NZ /\\ []<>~tick) => \\A {key} \\in {ids} : {})",
            requirements.join(" /\\ "),
        ),
        obligation,
    ));
    Ok(variables)
}
