// SPDX-License-Identifier: AGPL-3.0-or-later
//! A finite, occurrence-aware monitor for bounded timed formulas.

use std::collections::BTreeSet;

use filament_exact::{Cancel, Integer, Rational};
use fsl_semantics::check::{
    BinaryTemporal, CheckedFormula, CheckedInterval, CheckedTemporalFormula, FormulaNode,
    UnaryTemporal,
};
use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::Zero;
use serde_json::{json, Value};

use super::{ProbabilityDocumentError, ProbabilityDocumentLimit, ProbabilityDocumentLimits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TemporalKind {
    Eventually,
    Always,
    Once,
    Historically,
    Until,
    Release,
    Since,
    Triggered,
}

#[derive(Clone, Debug)]
enum NodeSpec {
    Constant(bool),
    Atom(usize),
    Not(usize),
    And(usize, usize),
    Or(usize, usize),
    Implies(usize, usize),
    Temporal {
        kind: TemporalKind,
        lower: u64,
        upper: u64,
        left: Option<usize>,
        right: usize,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Residual {
    Constant(bool),
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
    Future { node: usize, lower: u64, upper: u64 },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Candidate {
    age: u64,
    witness: Residual,
    guard: Residual,
}

/// Product state for independently replaying one checked timed formula.
///
/// Future obligations are canonical residual formulas. Past occurrences keep
/// only bounded-age candidate residuals; equal candidates at the same age are
/// merged, while every observation is still processed in sequence.
pub(super) struct Monitor {
    root: usize,
    scale: u64,
    specs: Vec<NodeSpec>,
    past: Vec<BTreeSet<Candidate>>,
    residual: Residual,
}

impl Monitor {
    /// Start the formula at its initial timed position.
    pub(super) fn start(
        formula: &CheckedTemporalFormula,
        scale: u64,
        initial_values: &[bool],
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
    ) -> Result<(Self, Option<bool>), ProbabilityDocumentError> {
        poll(cancel)?;
        if scale == 0 {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let checked = formula.formula();
        let specs = specs(checked, formula, scale)?;
        let root = checked.root().index();
        let atom_count = specs
            .iter()
            .filter(|spec| matches!(spec, NodeSpec::Atom(_)))
            .count();
        if atom_count != initial_values.len() {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let mut monitor = Self {
            root,
            scale,
            past: vec![BTreeSet::new(); specs.len()],
            specs,
            residual: Residual::Constant(false),
        };
        let mut evaluator = Evaluator::new(
            &monitor.specs,
            initial_values,
            &mut monitor.past,
            0,
            limits,
            cancel,
        );
        monitor.residual = evaluator.node(root)?;
        monitor.check_limits(limits)?;
        let verdict = monitor.verdict();
        Ok((monitor, verdict))
    }

    /// Consume one independently reconstructed timed position.
    pub(super) fn observe(
        &mut self,
        formula: &CheckedTemporalFormula,
        scale: u64,
        delta: u64,
        values: &[bool],
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
    ) -> Result<Option<bool>, ProbabilityDocumentError> {
        poll(cancel)?;
        if formula.formula().nodes().len() != self.specs.len() || scale != self.scale {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        let mut evaluator =
            Evaluator::new(&self.specs, values, &mut self.past, delta, limits, cancel);
        evaluator.node(self.root)?;
        self.residual = evaluator.progress(&self.residual, delta)?;
        self.check_limits(limits)?;
        Ok(self.verdict())
    }

    /// The exact formula verdict once every bounded future obligation closed.
    pub(super) fn verdict(&self) -> Option<bool> {
        match self.residual {
            Residual::Constant(value) => Some(value),
            _ => None,
        }
    }

    /// Canonical finite state included in the replayed product key.
    pub(super) fn key(&self) -> Value {
        json!({
            "occurrences": self.specs.iter().enumerate().filter_map(|(node, spec)| {
                let NodeSpec::Temporal { kind, lower, upper, .. } = spec else {
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

    fn check_limits(
        &self,
        limits: ProbabilityDocumentLimits,
    ) -> Result<(), ProbabilityDocumentError> {
        let states = self
            .past
            .iter()
            .fold(term_size(&self.residual), |total, candidates| {
                candidates.iter().fold(total, |total, candidate| {
                    total
                        .saturating_add(1)
                        .saturating_add(term_size(&candidate.witness))
                        .saturating_add(term_size(&candidate.guard))
                })
            });
        let state_bound = u64::try_from(limits.states).unwrap_or(u64::MAX);
        if states > state_bound {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::States,
                bound: state_bound,
                actual: states,
            });
        }
        let key_size = u64::try_from(
            serde_json::to_vec(&self.key())
                .map_err(|_| ProbabilityDocumentError::MalformedMember("timed monitor key"))?
                .len(),
        )
        .unwrap_or(u64::MAX);
        if key_size > limits.storage_bytes {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::StorageBytes,
                bound: limits.storage_bytes,
                actual: key_size,
            });
        }
        Ok(())
    }
}

fn specs(
    checked: &CheckedFormula,
    formula: &CheckedTemporalFormula,
    scale: u64,
) -> Result<Vec<NodeSpec>, ProbabilityDocumentError> {
    let mut atoms = 0;
    checked
        .nodes()
        .iter()
        .map(|node| {
            Ok(match node {
                FormulaNode::Constant(value) => NodeSpec::Constant(*value),
                FormulaNode::Holds { .. } => {
                    let atom = atoms;
                    atoms += 1;
                    if formula.condition(atom_index(checked, atom)).is_none() {
                        return Err(ProbabilityDocumentError::Unsupported);
                    }
                    NodeSpec::Atom(atom)
                }
                FormulaNode::Not(operand) => NodeSpec::Not(operand.index()),
                FormulaNode::And(left, right) => NodeSpec::And(left.index(), right.index()),
                FormulaNode::Or(left, right) => NodeSpec::Or(left.index(), right.index()),
                FormulaNode::Implies(left, right) => NodeSpec::Implies(left.index(), right.index()),
                FormulaNode::Unary {
                    operator,
                    interval,
                    operand,
                } => {
                    let (lower, upper) = closed_bounds(interval.as_ref(), scale)?;
                    let kind = match operator {
                        UnaryTemporal::Eventually => TemporalKind::Eventually,
                        UnaryTemporal::Always => TemporalKind::Always,
                        UnaryTemporal::Once => TemporalKind::Once,
                        UnaryTemporal::Historically => TemporalKind::Historically,
                    };
                    NodeSpec::Temporal {
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
                    let (lower, upper) = closed_bounds(interval.as_ref(), scale)?;
                    let kind = match operator {
                        BinaryTemporal::Until => TemporalKind::Until,
                        BinaryTemporal::Release => TemporalKind::Release,
                        BinaryTemporal::Since => TemporalKind::Since,
                        BinaryTemporal::Triggered => TemporalKind::Triggered,
                    };
                    NodeSpec::Temporal {
                        kind,
                        lower,
                        upper,
                        left: Some(left.index()),
                        right: right.index(),
                    }
                }
                FormulaNode::Aggregate { .. } => return Err(ProbabilityDocumentError::Unsupported),
            })
        })
        .collect()
}

// Checked formulas keep atoms at their arena positions; retrieve that position
// for the checked-predicate presence guard while the value slot stays compact.
fn atom_index(checked: &CheckedFormula, atom: usize) -> usize {
    checked
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
        .nth(atom)
        .map(|(index, _)| index)
        .unwrap_or(usize::MAX)
}

fn closed_bounds(
    interval: Option<&CheckedInterval>,
    scale: u64,
) -> Result<(u64, u64), ProbabilityDocumentError> {
    let Some(CheckedInterval::Timed(interval)) = interval else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    if !interval.lower_closed() || !interval.upper_closed() {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let upper = interval
        .upper()
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    let lower = time_units(interval.lower(), scale).ok_or(ProbabilityDocumentError::Unsupported)?;
    let upper = time_units(upper, scale).ok_or(ProbabilityDocumentError::Unsupported)?;
    if lower > upper {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    Ok((lower, upper))
}

fn time_units(value: &Rational, scale: u64) -> Option<u64> {
    let scaled = value.numerator().as_big() * BigInt::from(scale);
    let (quotient, remainder) = scaled.div_rem(value.denominator().as_big());
    remainder.is_zero().then(|| quotient.to_u64()).flatten()
}

struct Evaluator<'a> {
    specs: &'a [NodeSpec],
    values: &'a [bool],
    past: &'a mut [BTreeSet<Candidate>],
    delta: u64,
    state_bound: u64,
    cancel: &'a Cancel,
    cache: Vec<Option<Residual>>,
}

impl<'a> Evaluator<'a> {
    fn new(
        specs: &'a [NodeSpec],
        values: &'a [bool],
        past: &'a mut [BTreeSet<Candidate>],
        delta: u64,
        limits: ProbabilityDocumentLimits,
        cancel: &'a Cancel,
    ) -> Self {
        Self {
            specs,
            values,
            past,
            delta,
            state_bound: u64::try_from(limits.states).unwrap_or(u64::MAX),
            cancel,
            cache: vec![None; specs.len()],
        }
    }

    fn node(&mut self, index: usize) -> Result<Residual, ProbabilityDocumentError> {
        poll(self.cancel)?;
        if let Some(value) = self.cache.get(index).and_then(Option::as_ref) {
            return Ok(value.clone());
        }
        let spec = self
            .specs
            .get(index)
            .cloned()
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        let value = match spec {
            NodeSpec::Constant(value) => Residual::Constant(value),
            NodeSpec::Atom(slot) => Residual::Constant(
                *self
                    .values
                    .get(slot)
                    .ok_or(ProbabilityDocumentError::Unsupported)?,
            ),
            NodeSpec::Not(operand) => negate(self.node(operand)?),
            NodeSpec::And(left, right) => conjunction([self.node(left)?, self.node(right)?]),
            NodeSpec::Or(left, right) => disjunction([self.node(left)?, self.node(right)?]),
            NodeSpec::Implies(left, right) => {
                disjunction([negate(self.node(left)?), self.node(right)?])
            }
            NodeSpec::Temporal {
                kind,
                lower,
                upper,
                left,
                right,
            } => {
                if is_future(kind) {
                    let witness = self.node(right)?;
                    let guard = left.map(|id| self.node(id)).transpose()?;
                    future_step(index, kind, lower, upper, 0, witness, guard)
                } else {
                    self.past_node(index, kind, lower, upper, left, right)?
                }
            }
        };
        self.check_term(&value)?;
        let slot = self
            .cache
            .get_mut(index)
            .ok_or(ProbabilityDocumentError::Unsupported)?;
        *slot = Some(value.clone());
        Ok(value)
    }

    fn past_node(
        &mut self,
        index: usize,
        kind: TemporalKind,
        lower: u64,
        upper: u64,
        left: Option<usize>,
        right: usize,
    ) -> Result<Residual, ProbabilityDocumentError> {
        let dual = kind == TemporalKind::Triggered;
        let historical = kind == TemporalKind::Historically;
        let since = matches!(kind, TemporalKind::Since | TemporalKind::Triggered);
        let candidates = std::mem::take(
            self.past
                .get_mut(index)
                .ok_or(ProbabilityDocumentError::Unsupported)?,
        );
        let left_now = left.map(|id| self.node(id)).transpose()?;
        let right_now = self.node(right)?;
        let mut next = BTreeSet::new();
        for candidate in candidates.iter() {
            poll(self.cancel)?;
            let Some(age) = candidate.age.checked_add(self.delta) else {
                continue;
            };
            if age > upper {
                continue;
            }
            let witness = self.progress(&candidate.witness, self.delta)?;
            let guard = if since {
                let progressed = self.progress(&candidate.guard, self.delta)?;
                let current = left_now
                    .as_ref()
                    .ok_or(ProbabilityDocumentError::Unsupported)?
                    .clone();
                conjunction([progressed, if dual { negate(current) } else { current }])
            } else {
                self.progress(&candidate.guard, self.delta)?
            };
            self.check_term(&witness)?;
            self.check_term(&guard)?;
            next.insert(Candidate {
                age,
                witness,
                guard,
            });
            self.check_candidates(&next)?;
        }
        let witness = if dual { negate(right_now) } else { right_now };
        next.insert(Candidate {
            age: 0,
            witness,
            guard: Residual::Constant(true),
        });
        self.check_candidates(&next)?;

        let selected: Vec<_> = next
            .iter()
            .filter(|candidate| (lower..=upper).contains(&candidate.age))
            .cloned()
            .collect();
        *self
            .past
            .get_mut(index)
            .ok_or(ProbabilityDocumentError::Unsupported)? = next;

        let terms = selected.into_iter().map(|candidate| {
            if since {
                conjunction([candidate.witness, candidate.guard])
            } else {
                candidate.witness
            }
        });
        let accumulated = if historical {
            conjunction(terms)
        } else {
            disjunction(terms)
        };
        Ok(if dual {
            negate(accumulated)
        } else {
            accumulated
        })
    }

    fn progress(
        &mut self,
        residual: &Residual,
        delta: u64,
    ) -> Result<Residual, ProbabilityDocumentError> {
        poll(self.cancel)?;
        let next = match residual {
            Residual::Constant(value) => Residual::Constant(*value),
            Residual::Not(inner) => negate(self.progress(inner, delta)?),
            Residual::And(parts) => conjunction(
                parts
                    .iter()
                    .map(|part| self.progress(part, delta))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Residual::Or(parts) => disjunction(
                parts
                    .iter()
                    .map(|part| self.progress(part, delta))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Residual::Future { node, lower, upper } => {
                let spec = self
                    .specs
                    .get(*node)
                    .cloned()
                    .ok_or(ProbabilityDocumentError::Unsupported)?;
                let NodeSpec::Temporal {
                    kind, left, right, ..
                } = spec
                else {
                    return Err(ProbabilityDocumentError::Unsupported);
                };
                if !is_future(kind) {
                    return Err(ProbabilityDocumentError::Unsupported);
                }
                let witness = self.node(right)?;
                let guard = left.map(|id| self.node(id)).transpose()?;
                future_step(*node, kind, *lower, *upper, delta, witness, guard)
            }
        };
        self.check_term(&next)?;
        Ok(next)
    }

    fn check_term(&self, term: &Residual) -> Result<(), ProbabilityDocumentError> {
        let actual = term_size(term);
        if actual > self.state_bound {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::States,
                bound: self.state_bound,
                actual,
            });
        }
        Ok(())
    }

    fn check_candidates(
        &self,
        candidates: &BTreeSet<Candidate>,
    ) -> Result<(), ProbabilityDocumentError> {
        let actual = self
            .past
            .iter()
            .fold(0_u64, |total, node| {
                node.iter().fold(total, |total, candidate| {
                    total
                        .saturating_add(1)
                        .saturating_add(term_size(&candidate.witness))
                        .saturating_add(term_size(&candidate.guard))
                })
            })
            .saturating_add(candidates.iter().fold(0_u64, |total, candidate| {
                total
                    .saturating_add(1)
                    .saturating_add(term_size(&candidate.witness))
                    .saturating_add(term_size(&candidate.guard))
            }));
        if actual > self.state_bound {
            return Err(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::States,
                bound: self.state_bound,
                actual,
            });
        }
        Ok(())
    }
}

fn poll(cancel: &Cancel) -> Result<(), ProbabilityDocumentError> {
    if cancel.poll() {
        return Err(ProbabilityDocumentError::Cancelled(
            cancel
                .tripped()
                .or_else(|| cancel.cause())
                .expect("a tripped cancellation has a cause"),
        ));
    }
    Ok(())
}

fn term_size(term: &Residual) -> u64 {
    match term {
        Residual::Constant(_) | Residual::Future { .. } => 1,
        Residual::Not(inner) => 1_u64.saturating_add(term_size(inner)),
        Residual::And(parts) | Residual::Or(parts) => parts
            .iter()
            .fold(1_u64, |sum, part| sum.saturating_add(term_size(part))),
    }
}

fn is_future(kind: TemporalKind) -> bool {
    matches!(
        kind,
        TemporalKind::Eventually
            | TemporalKind::Always
            | TemporalKind::Until
            | TemporalKind::Release
    )
}

fn kind_name(kind: TemporalKind) -> &'static str {
    match kind {
        TemporalKind::Eventually => "eventually",
        TemporalKind::Always => "always",
        TemporalKind::Once => "once",
        TemporalKind::Historically => "historically",
        TemporalKind::Until => "until",
        TemporalKind::Release => "release",
        TemporalKind::Since => "since",
        TemporalKind::Triggered => "triggered",
    }
}

fn future_step(
    node: usize,
    kind: TemporalKind,
    lower: u64,
    upper: u64,
    delta: u64,
    witness: Residual,
    guard: Option<Residual>,
) -> Residual {
    if delta > upper {
        return Residual::Constant(matches!(kind, TemporalKind::Always | TemporalKind::Release));
    }
    let contains = lower <= delta;
    let remaining_lower = lower.saturating_sub(delta);
    let remaining_upper = upper - delta;
    let rest = Residual::Future {
        node,
        lower: remaining_lower,
        upper: remaining_upper,
    };
    match kind {
        TemporalKind::Eventually => {
            if contains {
                disjunction([witness, rest])
            } else {
                rest
            }
        }
        TemporalKind::Always => {
            if contains {
                conjunction([witness, rest])
            } else {
                rest
            }
        }
        TemporalKind::Until => {
            let guard = guard.unwrap_or(Residual::Constant(false));
            if contains {
                disjunction([witness, conjunction([guard, rest])])
            } else {
                conjunction([guard, rest])
            }
        }
        TemporalKind::Release => {
            let guard = guard.unwrap_or(Residual::Constant(false));
            if contains {
                conjunction([witness, disjunction([guard, rest])])
            } else {
                disjunction([guard, rest])
            }
        }
        TemporalKind::Once
        | TemporalKind::Historically
        | TemporalKind::Since
        | TemporalKind::Triggered => Residual::Constant(false),
    }
}

fn negate(term: Residual) -> Residual {
    match term {
        Residual::Constant(value) => Residual::Constant(!value),
        Residual::Not(inner) => *inner,
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
    let (unit, absorbing) = if conjunction {
        (true, false)
    } else {
        (false, true)
    };
    let mut terms = BTreeSet::new();
    for part in parts {
        let nested = match part {
            Residual::And(inner) if conjunction => inner,
            Residual::Or(inner) if !conjunction => inner,
            other => vec![other],
        };
        for term in nested {
            match term {
                Residual::Constant(value) if value == absorbing => {
                    return Residual::Constant(absorbing)
                }
                Residual::Constant(value) if value == unit => {}
                other => {
                    terms.insert(other);
                }
            }
        }
    }
    match terms.len() {
        0 => Residual::Constant(unit),
        1 => terms.into_iter().next().expect("one residual term"),
        _ if conjunction => Residual::And(terms.into_iter().collect()),
        _ => Residual::Or(terms.into_iter().collect()),
    }
}

fn residual_json(residual: &Residual) -> Value {
    match residual {
        Residual::Constant(value) => json!(["constant", value]),
        Residual::Not(inner) => json!(["not", residual_json(inner)]),
        Residual::And(parts) => json!(["and", parts.iter().map(residual_json).collect::<Vec<_>>()]),
        Residual::Or(parts) => json!(["or", parts.iter().map(residual_json).collect::<Vec<_>>()]),
        Residual::Future { node, lower, upper } => json!([
            "future",
            node.to_string(),
            lower.to_string(),
            upper.to_string(),
        ]),
    }
}
