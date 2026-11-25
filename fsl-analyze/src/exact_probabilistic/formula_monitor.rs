// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite residual monitor for a bounded timed formula.

use std::collections::BTreeSet;

use fsl_semantics::check::{
    BinaryTemporal, CheckedInterval, CheckedTemporalFormula, FormulaNode, UnaryTemporal,
};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Kind {
    Eventually,
    Always,
    Until,
    Release,
    Once,
    Historically,
    Since,
    Triggered,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Spec {
    Constant(bool),
    Atom(usize),
    Not(usize),
    And(usize, usize),
    Or(usize, usize),
    Implies(usize, usize),
    Temporal {
        kind: Kind,
        lower: u64,
        upper: u64,
        left: Option<usize>,
        right: usize,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Residual {
    Constant(bool),
    Not(Box<Residual>),
    And(Vec<Residual>),
    Or(Vec<Residual>),
    Future {
        node: usize,
        lower: u64,
        upper: u64,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Candidate {
    age: u64,
    witness: Residual,
    guard: Residual,
}

/// The canonical state carried by one admitted checked timed formula.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Monitor {
    specs: Vec<Spec>,
    root: usize,
    residual: Residual,
    past: Vec<BTreeSet<Candidate>>,
    time: u64,
    last_values: Vec<bool>,
    started: bool,
}

impl Monitor {
    /// Build from the checked AST. `scale` is the digital product's exact
    /// common denominator for every timed interval.
    pub(super) fn new(formula: &CheckedTemporalFormula, scale: u64) -> Result<Self, &'static str> {
        let checked = formula.formula();
        let mut atom = 0_usize;
        let mut specs = Vec::with_capacity(checked.nodes().len());
        for node in checked.nodes() {
            let spec = match node {
                FormulaNode::Constant(value) => Spec::Constant(*value),
                FormulaNode::Holds { .. } => {
                    let index = atom;
                    atom = atom.checked_add(1).ok_or("timed-formula-atom-count")?;
                    Spec::Atom(index)
                }
                FormulaNode::Aggregate { .. } => return Err("timed-formula-has-no-aggregate"),
                FormulaNode::Not(operand) => Spec::Not(operand.index()),
                FormulaNode::And(left, right) => Spec::And(left.index(), right.index()),
                FormulaNode::Or(left, right) => Spec::Or(left.index(), right.index()),
                FormulaNode::Implies(left, right) => {
                    Spec::Implies(left.index(), right.index())
                }
                FormulaNode::Unary {
                    operator,
                    interval,
                    operand,
                } => {
                    let kind = match operator {
                        UnaryTemporal::Eventually => Kind::Eventually,
                        UnaryTemporal::Always => Kind::Always,
                        UnaryTemporal::Once => Kind::Once,
                        UnaryTemporal::Historically => Kind::Historically,
                    };
                    let (lower, upper) = interval_bounds(interval.as_ref(), scale)?;
                    Spec::Temporal {
                        kind,
                        lower,
                        upper,
                        left: None,
                        right: operand.index(),
                    }
                }
                FormulaNode::Binary {
                    operator,
                    interval,
                    left,
                    right,
                } => {
                    let kind = match operator {
                        BinaryTemporal::Until => Kind::Until,
                        BinaryTemporal::Release => Kind::Release,
                        BinaryTemporal::Since => Kind::Since,
                        BinaryTemporal::Triggered => Kind::Triggered,
                    };
                    let (lower, upper) = interval_bounds(interval.as_ref(), scale)?;
                    Spec::Temporal {
                        kind,
                        lower,
                        upper,
                        left: Some(left.index()),
                        right: right.index(),
                    }
                }
            };
            specs.push(spec);
        }
        let root = checked.root().index();
        if root >= specs.len() {
            return Err("timed-formula-root-in-arena");
        }
        Ok(Self {
            past: vec![BTreeSet::new(); specs.len()],
            specs,
            root,
            residual: Residual::Constant(false),
            time: 0,
            last_values: Vec::new(),
            started: false,
        })
    }

    /// Consume one actual product position, including a same-time position.
    /// Returns a verdict once the residual has become a Boolean constant.
    pub(super) fn observe(
        &mut self,
        time: u64,
        values: &[bool],
    ) -> Result<Option<bool>, &'static str> {
        if self.started && time < self.time {
            return Err("timed-formula-time-is-monotone");
        }
        let delta = if self.started { time - self.time } else { 0 };
        let mut past = std::mem::take(&mut self.past);
        let mut evaluation = Evaluation {
            specs: &self.specs,
            truths: values,
            delta,
            past: &mut past,
            cache: vec![None; self.specs.len()],
        };
        let current = evaluation.node(self.root)?;
        let residual = if self.started {
            evaluation.progress(self.residual.clone())?
        } else {
            current
        };
        self.past = past;
        self.residual = residual;
        self.time = time;
        self.last_values = values.to_vec();
        self.started = true;
        Ok(match self.residual {
            Residual::Constant(value) => Some(value),
            _ => None,
        })
    }

    /// The finite canonical key state. It contains no unbounded observation
    /// history; equal residuals and past candidates are ordered and merged.
    pub(super) fn key(&self) -> Value {
        json!({
            "occurrences": self.specs.iter().enumerate().filter_map(|(node, spec)| {
                let Spec::Temporal { kind, lower, upper, .. } = spec else {
                    return None;
                };
                Some(json!([
                    node.to_string(),
                    kind_name(*kind),
                    lower.to_string(),
                    upper.to_string(),
                ]))
            }).collect::<Vec<_>>(),
            "residual": residual_json(&self.residual),
            "past": self.past.iter().enumerate().filter_map(|(node, candidates)| {
                if candidates.is_empty() {
                    return None;
                }
                Some(json!([
                    node.to_string(),
                    candidates.iter().map(|candidate| json!([
                        candidate.age.to_string(),
                        residual_json(&candidate.witness),
                        residual_json(&candidate.guard),
                    ])).collect::<Vec<_>>(),
                ]))
            }).collect::<Vec<_>>(),
        })
    }

    pub(super) fn time(&self) -> u64 {
        self.time
    }

    pub(super) fn last_values(&self) -> &[bool] {
        &self.last_values
    }

    /// Number of residual/candidate cells retained by the monitor, charged
    /// against the caller's existing automaton-state budget.
    pub(super) fn cells(&self) -> usize {
        let mut count = residual_cells(&self.residual);
        for candidates in &self.past {
            count = count.saturating_add(candidates.len());
            for candidate in candidates {
                count = count
                    .saturating_add(residual_cells(&candidate.witness))
                    .saturating_add(residual_cells(&candidate.guard));
            }
        }
        count
    }
}

fn interval_bounds(
    interval: Option<&CheckedInterval>,
    scale: u64,
) -> Result<(u64, u64), &'static str> {
    let Some(CheckedInterval::Timed(interval)) = interval else {
        return Err("timed-formula-operator-has-timed-interval");
    };
    if !interval.lower_closed() || !interval.upper_closed() {
        return Err("timed-formula-interval-is-closed");
    }
    let upper = interval.upper().ok_or("timed-formula-interval-is-bounded")?;
    let lower = super::units(interval.lower(), scale).ok_or("timed-formula-lower-scales")?;
    let upper = super::units(upper, scale).ok_or("timed-formula-upper-scales")?;
    if lower > upper {
        return Err("timed-formula-interval-is-ordered");
    }
    Ok((lower, upper))
}

struct Evaluation<'a> {
    specs: &'a [Spec],
    truths: &'a [bool],
    delta: u64,
    past: &'a mut [BTreeSet<Candidate>],
    cache: Vec<Option<Residual>>,
}

impl Evaluation<'_> {
    fn node(&mut self, index: usize) -> Result<Residual, &'static str> {
        if let Some(value) = self.cache.get(index).and_then(Option::as_ref) {
            return Ok(value.clone());
        }
        let spec = *self.specs.get(index).ok_or("timed-formula-node-in-arena")?;
        let value = match spec {
            Spec::Constant(value) => Residual::Constant(value),
            Spec::Atom(atom) => Residual::Constant(
                *self
                    .truths
                    .get(atom)
                    .ok_or("timed-formula-atom-has-truth")?,
            ),
            Spec::Not(operand) => negate(self.node(operand)?),
            Spec::And(left, right) => conjunction([self.node(left)?, self.node(right)?]),
            Spec::Or(left, right) => disjunction([self.node(left)?, self.node(right)?]),
            Spec::Implies(left, right) => {
                disjunction([negate(self.node(left)?), self.node(right)?])
            }
            Spec::Temporal {
                kind,
                lower,
                upper,
                left,
                right,
            } => match kind {
                Kind::Eventually | Kind::Always | Kind::Until | Kind::Release => self
                    .future(index, kind, lower, upper, left, right, 0)?,
                Kind::Once | Kind::Historically | Kind::Since | Kind::Triggered => {
                    self.past_node(index, kind, lower, upper, left, right)?
                }
            },
        };
        *self
            .cache
            .get_mut(index)
            .ok_or("timed-formula-cache-index")? = Some(value.clone());
        Ok(value)
    }

    fn future(
        &mut self,
        node: usize,
        kind: Kind,
        lower: u64,
        upper: u64,
        left: Option<usize>,
        right: usize,
        delta: u64,
    ) -> Result<Residual, &'static str> {
        if kind == Kind::Always
            && matches!(
                self.specs.get(node),
                Some(Spec::Temporal {
                    kind: Kind::Always,
                    upper: 0,
                    ..
                })
            )
        {
            return self.node(right);
        }
        if delta > upper {
            return Ok(Residual::Constant(matches!(kind, Kind::Always | Kind::Release)));
        }
        let eligible = delta >= lower;
        let lower = lower.saturating_sub(delta);
        let upper = upper - delta;
        let rest = Residual::Future { node, lower, upper };
        let right = self.node(right)?;
        let left = left.map(|left| self.node(left)).transpose()?;
        Ok(match kind {
            Kind::Eventually => {
                if eligible {
                    disjunction([right, rest])
                } else {
                    rest
                }
            }
            Kind::Always => {
                if eligible {
                    conjunction([right, rest])
                } else {
                    rest
                }
            }
            Kind::Until => {
                let left = left.ok_or("timed-until-has-left-operand")?;
                if eligible {
                    disjunction([right, conjunction([left, rest])])
                } else {
                    conjunction([left, rest])
                }
            }
            Kind::Release => {
                let left = left.ok_or("timed-release-has-left-operand")?;
                if eligible {
                    conjunction([right, disjunction([left, rest])])
                } else {
                    disjunction([left, rest])
                }
            }
            _ => return Err("timed-future-kind-is-future"),
        })
    }

    fn past_node(
        &mut self,
        node: usize,
        kind: Kind,
        lower: u64,
        upper: u64,
        left: Option<usize>,
        right: usize,
    ) -> Result<Residual, &'static str> {
        let mut previous = std::mem::take(
            self.past
                .get_mut(node)
                .ok_or("timed-past-state-in-arena")?,
        );
        let mut candidates = BTreeSet::new();
        let is_since = matches!(kind, Kind::Since | Kind::Triggered);
        let dual = kind == Kind::Triggered;
        let guard_now = if is_since {
            let index = left.ok_or("timed-since-has-left-operand")?;
            let value = self.node(index)?;
            Some(if dual { negate(value) } else { value })
        } else {
            None
        };
        for candidate in std::mem::take(&mut previous) {
            let Some(age) = candidate.age.checked_add(self.delta) else {
                continue;
            };
            if age > upper {
                continue;
            }
            let witness = self.progress(candidate.witness)?;
            let guard = if let Some(current) = &guard_now {
                conjunction([self.progress(candidate.guard)?, current.clone()])
            } else {
                Residual::Constant(true)
            };
            candidates.insert(Candidate {
                age,
                witness,
                guard,
            });
        }
        let right = self.node(right)?;
        candidates.insert(Candidate {
            age: 0,
            witness: if dual { negate(right) } else { right },
            guard: Residual::Constant(true),
        });
        *self
            .past
            .get_mut(node)
            .ok_or("timed-past-state-in-arena")? = candidates.clone();

        let selected = candidates
            .iter()
            .filter(|candidate| (lower..=upper).contains(&candidate.age));
        let terms = selected.map(|candidate| {
            if is_since {
                conjunction([candidate.witness.clone(), candidate.guard.clone()])
            } else {
                candidate.witness.clone()
            }
        });
        let result = match kind {
            Kind::Once | Kind::Since => disjunction(terms),
            Kind::Historically => conjunction(terms),
            Kind::Triggered => negate(disjunction(terms)),
            _ => return Err("timed-past-kind-is-past"),
        };
        Ok(result)
    }

    fn progress(&mut self, residual: Residual) -> Result<Residual, &'static str> {
        Ok(match residual {
            Residual::Constant(_) => residual,
            Residual::Not(inner) => negate(self.progress(*inner)?),
            Residual::And(parts) => conjunction(
                parts
                    .into_iter()
                    .map(|part| self.progress(part))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Residual::Or(parts) => disjunction(
                parts
                    .into_iter()
                    .map(|part| self.progress(part))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Residual::Future {
                node,
                lower,
                upper,
            } => {
                let Spec::Temporal {
                    kind,
                    left,
                    right,
                    ..
                } = *self.specs.get(node).ok_or("timed-future-node-in-arena")?
                else {
                    return Err("timed-future-residual-names-temporal-node");
                };
                self.future(node, kind, lower, upper, left, right, self.delta)?
            }
        })
    }
}

fn negate(term: Residual) -> Residual {
    match term {
        Residual::Constant(value) => Residual::Constant(!value),
        Residual::Not(inner) => *inner,
        Residual::And(parts) => disjunction(parts.into_iter().map(negate)),
        Residual::Or(parts) => conjunction(parts.into_iter().map(negate)),
        other => Residual::Not(Box::new(other)),
    }
}

fn conjunction(parts: impl IntoIterator<Item = Residual>) -> Residual {
    join(true, parts)
}

fn disjunction(parts: impl IntoIterator<Item = Residual>) -> Residual {
    join(false, parts)
}

fn join(conjunction: bool, parts: impl IntoIterator<Item = Residual>) -> Residual {
    let mut terms = BTreeSet::new();
    for part in parts {
        let part = match part {
            Residual::And(inner) if conjunction => inner,
            Residual::Or(inner) if !conjunction => inner,
            other => vec![other],
        };
        for term in part {
            match term {
                Residual::Constant(value) if value != conjunction => {
                    return Residual::Constant(!conjunction);
                }
                Residual::Constant(_) => {}
                other => {
                    terms.insert(other);
                }
            }
        }
    }
    match terms.len() {
        0 => Residual::Constant(conjunction),
        1 => terms.into_iter().next().expect("one term"),
        _ if conjunction => Residual::And(terms.into_iter().collect()),
        _ => Residual::Or(terms.into_iter().collect()),
    }
}

fn residual_cells(residual: &Residual) -> usize {
    match residual {
        Residual::Constant(_) | Residual::Future { .. } => 1,
        Residual::Not(inner) => 1_usize.saturating_add(residual_cells(inner)),
        Residual::And(parts) | Residual::Or(parts) => parts
            .iter()
            .fold(1_usize, |count, part| count.saturating_add(residual_cells(part))),
    }
}

fn residual_json(residual: &Residual) -> Value {
    match residual {
        Residual::Constant(value) => json!(["constant", value]),
        Residual::Not(inner) => json!(["not", residual_json(inner)]),
        Residual::And(parts) => json!(["and", parts.iter().map(residual_json).collect::<Vec<_>>()]),
        Residual::Or(parts) => json!(["or", parts.iter().map(residual_json).collect::<Vec<_>>()]),
        Residual::Future { node, lower, upper } => {
            json!(["future", node.to_string(), lower.to_string(), upper.to_string()])
        }
    }
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Eventually => "eventually",
        Kind::Always => "always",
        Kind::Until => "until",
        Kind::Release => "release",
        Kind::Once => "once",
        Kind::Historically => "historically",
        Kind::Since => "since",
        Kind::Triggered => "triggered",
    }
}
