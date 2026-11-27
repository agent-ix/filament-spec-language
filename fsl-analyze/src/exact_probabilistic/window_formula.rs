// SPDX-License-Identifier: AGPL-3.0-or-later
//! Flat checked plan and typed residuals for bounded timed formulas.

use filament_exact::{Integer, Rational};
use fsl_eval::probabilistic::{DeadlineKind, DeadlinePhase, TimedOperatorState};
use fsl_semantics::check::{
    BinaryTemporal, CheckedFormula, CheckedInterval, CheckedTemporalFormula, FormulaNode,
    FormulaNodeId, UnaryTemporal,
};
use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{ToPrimitive, Zero};
use serde_json::{Value as JsonValue, json};

use super::UndefinedRead;
use fsl_semantics::check::CheckedPredicate;
use ix_trace_rs::implements;

/// A structurally complete residual. The builder interns these immutable
/// values and uses their local handle only inside its own product graph.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct ResidualState {
    time: u64,
    residual: Residual,
}

impl ResidualState {
    pub(super) fn time(&self) -> u64 {
        self.time
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Residual {
    Accept,
    Reject,
    And(Vec<Self>),
    Or(Vec<Self>),
    Obligation {
        kind: OperatorKind,
        clock: TimedOperatorState,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum OperatorKind {
    Eventually,
    Always,
    Until,
    Release,
}

#[derive(Clone)]
enum Node {
    Constant(bool),
    Atom {
        read: usize,
        negated: bool,
    },
    And(Vec<usize>),
    Or(Vec<usize>),
    Temporal {
        kind: OperatorKind,
        occurrence: u64,
        lower: Rational,
        upper: Rational,
        left: Option<usize>,
        right: usize,
    },
}

/// The plan reconstructed from the checked source formula.
pub(super) struct Plan<'c> {
    nodes: Vec<Node>,
    root: usize,
    reads: Vec<(UndefinedRead, &'c CheckedPredicate)>,
    horizon: Rational,
}

impl<'c> Plan<'c> {
    #[implements("FR-204-AC-21")]
    pub(super) fn compile(formula: &'c CheckedTemporalFormula) -> Result<Self, Vec<usize>> {
        let checked = formula.formula();
        let mut compiler = Compiler {
            formula,
            checked,
            nodes: Vec::new(),
            reads: Vec::new(),
            read_indices: std::collections::BTreeMap::new(),
            occurrence: 0,
            horizon: Rational::from_integer(Integer::zero()),
        };
        let root = compiler.compile_node(checked.root(), false, &[])?;
        Ok(Self {
            nodes: compiler.nodes,
            root,
            reads: compiler.reads,
            horizon: compiler.horizon,
        })
    }

    pub(super) fn reads(&self) -> &[(UndefinedRead, &'c CheckedPredicate)] {
        &self.reads
    }

    pub(super) fn horizon(&self) -> &Rational {
        &self.horizon
    }

    #[implements("FR-204-AC-1")]
    pub(super) fn time_constants(&self) -> impl Iterator<Item = &Rational> {
        self.nodes.iter().flat_map(|node| match node {
            Node::Temporal { lower, upper, .. } => vec![lower, upper],
            _ => Vec::new(),
        })
    }

    #[implements("FR-204-AC-21")]
    pub(super) fn start(&self, truths: &[bool], scale: u64) -> Result<ResidualState, ()> {
        if truths.len() != self.reads.len() {
            return Err(());
        }
        Ok(ResidualState {
            time: 0,
            residual: self.start_node(self.root, truths, scale)?,
        })
    }

    /// Advance one observed predicate position. `elapsed` is the current
    /// product clock; `previous.time` is the last position or delay update.
    #[implements("FR-204-AC-21")]
    pub(super) fn observe(
        &self,
        previous: &ResidualState,
        truths: &[bool],
        elapsed: u64,
        scale: u64,
    ) -> Result<ResidualState, ()> {
        if truths.len() != self.reads.len() {
            return Err(());
        }
        let delta = elapsed.checked_sub(previous.time).ok_or(())?;
        Ok(ResidualState {
            time: elapsed,
            residual: self.progress(&previous.residual, delta, truths, scale)?,
        })
    }

    /// Advance clocks across a delay without reading predicates.
    #[implements("FR-204-AC-21")]
    pub(super) fn delay(
        &self,
        previous: &ResidualState,
        elapsed: u64,
    ) -> Result<ResidualState, ()> {
        let delta = elapsed.checked_sub(previous.time).ok_or(())?;
        Ok(ResidualState {
            time: elapsed,
            residual: age_only(&previous.residual, delta),
        })
    }

    pub(super) fn is_decided(state: &ResidualState) -> Option<bool> {
        match state.residual {
            Residual::Accept => Some(true),
            Residual::Reject => Some(false),
            _ => None,
        }
    }

    /// Canonical FR-196 `window` monitor member. All nested arrays contain
    /// only strings and arrays, so compact JSON ordering is JCS ordering.
    #[implements("FR-204-AC-21")]
    pub(super) fn encode(&self, state: &ResidualState, subdivisions: u64) -> JsonValue {
        json!(["window", encode_residual(&state.residual, subdivisions)])
    }

    fn start_node(&self, index: usize, truths: &[bool], scale: u64) -> Result<Residual, ()> {
        match self.nodes.get(index).ok_or(())? {
            Node::Constant(value) => Ok(if *value {
                Residual::Accept
            } else {
                Residual::Reject
            }),
            Node::Atom { read, negated } => {
                let value = truths.get(*read).copied().ok_or(())? ^ *negated;
                Ok(if value {
                    Residual::Accept
                } else {
                    Residual::Reject
                })
            }
            Node::And(children) => self.combine(children, truths, scale, true),
            Node::Or(children) => self.combine(children, truths, scale, false),
            Node::Temporal {
                kind,
                occurrence,
                lower,
                upper,
                ..
            } => {
                let lower = time_units(lower, scale).ok_or(())?;
                let upper = time_units(upper, scale).ok_or(())?;
                let clock = TimedOperatorState {
                    occurrence: *occurrence,
                    kind: timed_kind(*kind),
                    lower,
                    upper,
                    elapsed: 0,
                    phase: DeadlinePhase::Pending,
                };
                self.read_obligation(*kind, clock, truths, scale)
            }
        }
    }

    fn combine(
        &self,
        children: &[usize],
        truths: &[bool],
        scale: u64,
        and: bool,
    ) -> Result<Residual, ()> {
        let values = children
            .iter()
            .map(|child| self.start_node(*child, truths, scale))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(if and {
            conjunction(values)
        } else {
            disjunction(values)
        })
    }

    fn read_obligation(
        &self,
        kind: OperatorKind,
        clock: TimedOperatorState,
        truths: &[bool],
        scale: u64,
    ) -> Result<Residual, ()> {
        let Some(Node::Temporal { left, right, .. }) = self
            .nodes
            .iter()
            .find(|node| matches!(node, Node::Temporal { occurrence, .. } if *occurrence == clock.occurrence))
        else {
            return Err(());
        };
        let in_window = (clock.lower..=clock.upper).contains(&clock.elapsed);
        let rest = Residual::Obligation { kind, clock };
        let start = |node| self.start_node(node, truths, scale);
        let branch = match kind {
            OperatorKind::Eventually => disjunction(vec![
                if in_window {
                    start(*right)?
                } else {
                    Residual::Reject
                },
                rest,
            ]),
            OperatorKind::Always => conjunction(vec![
                if in_window {
                    start(*right)?
                } else {
                    Residual::Accept
                },
                rest,
            ]),
            OperatorKind::Until => disjunction(vec![
                if in_window {
                    start(*right)?
                } else {
                    Residual::Reject
                },
                conjunction(vec![start(left.ok_or(())?)?, rest]),
            ]),
            OperatorKind::Release => conjunction(vec![
                if in_window {
                    start(*right)?
                } else {
                    Residual::Accept
                },
                disjunction(vec![start(left.ok_or(())?)?, rest]),
            ]),
        };
        Ok(branch)
    }

    #[implements("FR-204-AC-21")]
    fn progress(
        &self,
        residual: &Residual,
        delta: u64,
        truths: &[bool],
        scale: u64,
    ) -> Result<Residual, ()> {
        match residual {
            Residual::Accept | Residual::Reject => Ok(residual.clone()),
            Residual::And(children) => Ok(conjunction(
                children
                    .iter()
                    .map(|child| self.progress(child, delta, truths, scale))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Residual::Or(children) => Ok(disjunction(
                children
                    .iter()
                    .map(|child| self.progress(child, delta, truths, scale))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Residual::Obligation { kind, clock } => {
                let mut clock = *clock;
                clock.elapsed = clock.elapsed.saturating_add(delta);
                if clock.elapsed > clock.upper {
                    return Ok(timeout(*kind));
                }
                self.read_obligation(*kind, clock, truths, scale)
            }
        }
    }
}

struct Compiler<'a, 'c> {
    formula: &'c CheckedTemporalFormula,
    checked: &'a CheckedFormula,
    nodes: Vec<Node>,
    reads: Vec<(UndefinedRead, &'c CheckedPredicate)>,
    read_indices: std::collections::BTreeMap<usize, usize>,
    occurrence: u64,
    horizon: Rational,
}

impl Compiler<'_, '_> {
    fn compile_node(
        &mut self,
        id: FormulaNodeId,
        negated: bool,
        parent_path: &[usize],
    ) -> Result<usize, Vec<usize>> {
        let index = id.index();
        let mut path = parent_path.to_vec();
        path.push(index);
        let node = self
            .checked
            .nodes()
            .get(index)
            .cloned()
            .ok_or_else(|| path.clone())?;
        let compiled = match &node {
            FormulaNode::Constant(value) => Node::Constant(*value ^ negated),
            FormulaNode::Holds { .. } => {
                let read = if let Some(read) = self.read_indices.get(&index) {
                    *read
                } else {
                    let predicate = self.formula.condition(index).ok_or_else(|| path.clone())?;
                    let read = self.reads.len();
                    self.reads.push((UndefinedRead::Atom(id), predicate));
                    self.read_indices.insert(index, read);
                    read
                };
                Node::Atom { read, negated }
            }
            FormulaNode::Not(child) => return self.compile_node(*child, !negated, &path),
            FormulaNode::And(left, right) | FormulaNode::Or(left, right) => {
                let is_and = matches!(&node, FormulaNode::And(..)) ^ negated;
                let left = self.compile_node(*left, negated, &path)?;
                let right = self.compile_node(*right, negated, &path)?;
                if is_and {
                    Node::And(vec![left, right])
                } else {
                    Node::Or(vec![left, right])
                }
            }
            FormulaNode::Implies(left, right) => {
                let (left_negated, right_negated, and) = if negated {
                    (false, true, true)
                } else {
                    (true, false, false)
                };
                let left = self.compile_node(*left, left_negated, &path)?;
                let right = self.compile_node(*right, right_negated, &path)?;
                if and {
                    Node::And(vec![left, right])
                } else {
                    Node::Or(vec![left, right])
                }
            }
            FormulaNode::Unary {
                operator,
                interval: Some(interval),
                operand,
            } => {
                let (lower, upper) = interval_bounds(interval).ok_or_else(|| path.clone())?;
                let kind = match (operator, negated) {
                    (UnaryTemporal::Eventually, false) | (UnaryTemporal::Always, true) => {
                        OperatorKind::Eventually
                    }
                    (UnaryTemporal::Always, false) | (UnaryTemporal::Eventually, true) => {
                        OperatorKind::Always
                    }
                    _ => return Err(path),
                };
                let occurrence = self.next_occurrence();
                self.horizon = self.horizon.add(&upper);
                let child = self.compile_node(*operand, negated, &path)?;
                Node::Temporal {
                    kind,
                    occurrence,
                    lower,
                    upper,
                    left: None,
                    right: child,
                }
            }
            FormulaNode::Binary {
                operator,
                interval: Some(interval),
                left,
                right,
            } if matches!(operator, BinaryTemporal::Until | BinaryTemporal::Release) => {
                let (lower, upper) = interval_bounds(interval).ok_or_else(|| path.clone())?;
                let kind = match (operator, negated) {
                    (BinaryTemporal::Until, false) | (BinaryTemporal::Release, true) => {
                        OperatorKind::Until
                    }
                    (BinaryTemporal::Release, false) | (BinaryTemporal::Until, true) => {
                        OperatorKind::Release
                    }
                    _ => return Err(path),
                };
                let occurrence = self.next_occurrence();
                self.horizon = self.horizon.add(&upper);
                let left = self.compile_node(*left, negated, &path)?;
                let right = self.compile_node(*right, negated, &path)?;
                Node::Temporal {
                    kind,
                    occurrence,
                    lower,
                    upper,
                    left: Some(left),
                    right,
                }
            }
            _ => return Err(path),
        };
        let index = self.nodes.len();
        self.nodes.push(compiled);
        Ok(index)
    }

    fn next_occurrence(&mut self) -> u64 {
        let current = self.occurrence;
        self.occurrence = self.occurrence.saturating_add(1);
        current
    }
}

fn interval_bounds(interval: &CheckedInterval) -> Option<(Rational, Rational)> {
    let CheckedInterval::Timed(interval) = interval else {
        return None;
    };
    if !interval.lower_closed() || !interval.upper_closed() {
        return None;
    }
    Some((interval.lower().clone(), interval.upper()?.clone()))
}

fn time_units(value: &Rational, scale: u64) -> Option<u64> {
    let scaled = value.numerator().as_big() * BigInt::from(scale);
    let (quotient, remainder) = scaled.div_rem(value.denominator().as_big());
    remainder.is_zero().then(|| quotient.to_u64()).flatten()
}

fn timed_kind(kind: OperatorKind) -> DeadlineKind {
    match kind {
        OperatorKind::Eventually => DeadlineKind::Eventually,
        OperatorKind::Always => DeadlineKind::Always,
        OperatorKind::Until | OperatorKind::Release => DeadlineKind::Until,
    }
}

#[implements("FR-204-AC-21")]
fn timeout(kind: OperatorKind) -> Residual {
    match kind {
        OperatorKind::Eventually | OperatorKind::Until => Residual::Reject,
        OperatorKind::Always | OperatorKind::Release => Residual::Accept,
    }
}

#[implements("FR-204-AC-21")]
fn age_only(residual: &Residual, delta: u64) -> Residual {
    match residual {
        Residual::Accept | Residual::Reject => residual.clone(),
        Residual::And(children) => conjunction(
            children
                .iter()
                .map(|child| age_only(child, delta))
                .collect(),
        ),
        Residual::Or(children) => disjunction(
            children
                .iter()
                .map(|child| age_only(child, delta))
                .collect(),
        ),
        Residual::Obligation { kind, clock } => {
            let mut clock = *clock;
            clock.elapsed = clock.elapsed.saturating_add(delta);
            if clock.elapsed > clock.upper {
                timeout(*kind)
            } else {
                Residual::Obligation { kind: *kind, clock }
            }
        }
    }
}

#[implements("FR-204-AC-21")]
fn conjunction(values: Vec<Residual>) -> Residual {
    let mut flat = Vec::new();
    for value in values {
        match value {
            Residual::Reject => return Residual::Reject,
            Residual::Accept => {}
            Residual::And(children) => flat.extend(children),
            other => flat.push(other),
        }
    }
    normalize(flat, true)
}

#[implements("FR-204-AC-21")]
fn disjunction(values: Vec<Residual>) -> Residual {
    let mut flat = Vec::new();
    for value in values {
        match value {
            Residual::Accept => return Residual::Accept,
            Residual::Reject => {}
            Residual::Or(children) => flat.extend(children),
            other => flat.push(other),
        }
    }
    normalize(flat, false)
}

#[implements("FR-204-AC-21")]
fn normalize(mut values: Vec<Residual>, and: bool) -> Residual {
    values.sort_by_key(|value| encode_residual(value, 1).to_string());
    values.dedup();
    match values.len() {
        0 if and => Residual::Accept,
        0 => Residual::Reject,
        1 => values.pop().unwrap_or(if and {
            Residual::Accept
        } else {
            Residual::Reject
        }),
        _ if and => Residual::And(values),
        _ => Residual::Or(values),
    }
}

#[implements("FR-204-AC-21")]
fn encode_residual(residual: &Residual, subdivisions: u64) -> JsonValue {
    match residual {
        Residual::Accept => json!(["accept"]),
        Residual::Reject => json!(["reject"]),
        Residual::And(children) => json!([
            "and",
            children
                .iter()
                .map(|child| encode_residual(child, subdivisions))
                .collect::<Vec<_>>()
        ]),
        Residual::Or(children) => json!([
            "or",
            children
                .iter()
                .map(|child| encode_residual(child, subdivisions))
                .collect::<Vec<_>>()
        ]),
        Residual::Obligation { kind, clock } => json!([
            match kind {
                OperatorKind::Eventually => "eventually",
                OperatorKind::Always => "always",
                OperatorKind::Until => "until",
                OperatorKind::Release => "release",
            },
            clock.occurrence.to_string(),
            (clock.elapsed / subdivisions.max(1)).to_string(),
            (clock.elapsed % subdivisions.max(1)).to_string()
        ]),
    }
}
