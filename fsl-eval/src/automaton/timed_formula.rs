// SPDX-License-Identifier: AGPL-3.0-or-later
//! A finite, occurrence-aware monitor for bounded timed formulas.

use std::collections::BTreeSet;

use filament_exact::{Cancel, Rational};
use fsl_semantics::check::{
    BinaryTemporal, CheckedFormula, CheckedInterval, CheckedTemporalFormula, FormulaNode,
    UnaryTemporal,
};
use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{ToPrimitive, Zero};
use quire_canonical::{Node, NodeRef};
use serde_json::{json, Value};

/// Bounds checked while constructing a timed formula monitor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimedFormulaMonitorLimits {
    /// Maximum residual and past-candidate cells.
    pub states: u64,
    /// Maximum encoded monitor-key bytes.
    pub storage_bytes: u64,
}

/// A bounded monitor construction or replay failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimedFormulaMonitorError {
    /// The checked formula or observation does not fit this monitor.
    Unsupported,
    /// The monitor exceeded its residual/candidate-cell bound.
    States {
        /// Configured maximum cell count.
        bound: u64,
        /// Required cell count.
        actual: u64,
    },
    /// The canonical monitor key exceeds its encoded-byte bound.
    StorageBytes {
        /// Configured maximum encoded key size.
        bound: u64,
        /// Encoded size required by this key.
        actual: u64,
    },
    /// The caller's cancellation token was tripped.
    Cancelled,
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimedFormulaMonitor {
    root: usize,
    scale: u64,
    specs: Vec<NodeSpec>,
    past: Vec<BTreeSet<Candidate>>,
    residual: Residual,
    time: u64,
    last_values: Vec<bool>,
    started: bool,
    storage_limit: u64,
}

impl TimedFormulaMonitor {
    /// Create an empty monitor from a checked formula.
    pub fn new(
        formula: &CheckedTemporalFormula,
        scale: u64,
        limits: TimedFormulaMonitorLimits,
    ) -> Result<Self, TimedFormulaMonitorError> {
        if scale == 0 {
            return Err(TimedFormulaMonitorError::Unsupported);
        }
        let checked = formula.formula();
        let node_count = u64::try_from(checked.nodes().len()).unwrap_or(u64::MAX);
        if node_count > limits.states {
            return Err(TimedFormulaMonitorError::States {
                bound: limits.states,
                actual: node_count,
            });
        }
        let specs = specs(checked, formula, scale)?;
        let root = checked.root().index();
        if root >= specs.len() {
            return Err(TimedFormulaMonitorError::Unsupported);
        }
        Ok(Self {
            root,
            scale,
            past: vec![BTreeSet::new(); specs.len()],
            specs,
            residual: Residual::Constant(false),
            time: 0,
            last_values: Vec::new(),
            started: false,
            storage_limit: limits.storage_bytes,
        })
    }

    /// Start the formula at its initial timed position.
    pub fn start(
        formula: &CheckedTemporalFormula,
        scale: u64,
        initial_values: &[bool],
        limits: TimedFormulaMonitorLimits,
        cancel: &Cancel,
    ) -> Result<(Self, Option<bool>), TimedFormulaMonitorError> {
        poll(cancel)?;
        let mut monitor = Self::new(formula, scale, limits)?;
        monitor.observe(formula, scale, 0, initial_values, limits, cancel)?;
        poll(cancel)?;
        monitor.check_limits(limits)?;
        poll(cancel)?;
        let verdict = monitor.verdict();
        Ok((monitor, verdict))
    }

    /// Consume one independently reconstructed timed position.
    pub fn observe(
        &mut self,
        formula: &CheckedTemporalFormula,
        scale: u64,
        delta: u64,
        values: &[bool],
        limits: TimedFormulaMonitorLimits,
        cancel: &Cancel,
    ) -> Result<Option<bool>, TimedFormulaMonitorError> {
        poll(cancel)?;
        if formula.formula().nodes().len() != self.specs.len() || scale != self.scale {
            return Err(TimedFormulaMonitorError::Unsupported);
        }
        let was_started = self.started;
        let mut evaluator =
            Evaluator::new(&self.specs, values, &mut self.past, delta, limits, cancel);
        let current = evaluator.node(self.root)?;
        self.residual = if was_started {
            evaluator.progress(&self.residual, delta)?
        } else {
            current
        };
        self.time = self
            .time
            .checked_add(delta)
            .ok_or(TimedFormulaMonitorError::Unsupported)?;
        self.last_values = values.to_vec();
        self.started = true;
        self.storage_limit = limits.storage_bytes;
        poll(cancel)?;
        self.check_limits(limits)?;
        poll(cancel)?;
        Ok(self.verdict())
    }

    /// Consume an observation at an absolute digital timestamp.
    pub fn observe_at(
        &mut self,
        formula: &CheckedTemporalFormula,
        scale: u64,
        time: u64,
        values: &[bool],
        limits: TimedFormulaMonitorLimits,
        cancel: &Cancel,
    ) -> Result<Option<bool>, TimedFormulaMonitorError> {
        if self.started && time < self.time {
            return Err(TimedFormulaMonitorError::Unsupported);
        }
        let delta = if self.started { time - self.time } else { 0 };
        self.observe(formula, scale, delta, values, limits, cancel)
    }

    /// The exact formula verdict once every bounded future obligation closed.
    pub fn verdict(&self) -> Option<bool> {
        match self.residual {
            Residual::Constant(value) => Some(value),
            _ => None,
        }
    }

    /// Exact elapsed units of the last reconstructed timed position.
    pub fn time(&self) -> u64 {
        self.time
    }

    /// Truth values at the previous independently reconstructed position.
    pub fn last_values(&self) -> &[bool] {
        &self.last_values
    }

    /// Canonical finite state included in the replayed product key.
    /// Return the monitor key after checking the storage ceiling established
    /// by its latest admitted observation.
    pub fn key_value(&self) -> Result<Value, TimedFormulaMonitorError> {
        self.encode_key_bounded(self.storage_limit)?;
        Ok(self.key())
    }

    fn key(&self) -> Value {
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

    /// Build the JSON product member only after proving its bounded size.
    pub fn key_checked(&self, max_bytes: u64) -> Result<Value, TimedFormulaMonitorError> {
        self.encode_key_bounded(max_bytes)?;
        Ok(self.key())
    }

    /// Check the conservative encoded-key bound without constructing JSON.
    pub fn check_key_size(&self, max_bytes: u64) -> Result<(), TimedFormulaMonitorError> {
        self.encode_key_bounded(max_bytes)?;
        Ok(())
    }

    /// Current residual and past-candidate cells.
    pub fn cells(&self) -> u64 {
        self.past
            .iter()
            .fold(term_size(&self.residual), |total, candidates| {
                candidates.iter().fold(total, |total, candidate| {
                    total
                        .saturating_add(1)
                        .saturating_add(term_size(&candidate.witness))
                        .saturating_add(term_size(&candidate.guard))
                })
            })
    }

    fn check_limits(
        &self,
        limits: TimedFormulaMonitorLimits,
    ) -> Result<(), TimedFormulaMonitorError> {
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
        let state_bound = limits.states;
        if states > state_bound {
            return Err(TimedFormulaMonitorError::States {
                bound: state_bound,
                actual: states,
            });
        }
        self.encode_key_bounded(limits.storage_bytes)?;
        Ok(())
    }

    fn encode_key_bounded(&self, max_bytes: u64) -> Result<(), TimedFormulaMonitorError> {
        let mut output = Vec::new();
        let mut reservation = KeyReservation {
            used: 0,
            max: max_bytes,
            denied: None,
        };
        quire_canonical::encode_with_reservation(
            &mut output,
            &MonitorKey(self),
            quire_canonical::Limits::new(max_bytes),
            &mut reservation,
        )
        .map(|_| ())
        .map_err(|_| TimedFormulaMonitorError::StorageBytes {
            bound: max_bytes,
            actual: reservation
                .denied
                .unwrap_or_else(|| max_bytes.saturating_add(1)),
        })
    }
}

struct MonitorKey<'a>(&'a TimedFormulaMonitor);

impl quire_canonical::Encode for MonitorKey<'_> {
    fn encode_into<S: quire_canonical::Sink + ?Sized>(
        &self,
        writer: &mut quire_canonical::Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_object()?;
        writer.name("occurrences")?;
        writer.begin_array()?;
        for (index, spec) in self.0.specs.iter().enumerate() {
            if let NodeSpec::Temporal {
                kind, lower, upper, ..
            } = spec
            {
                writer.begin_array()?;
                writer.string(&index.to_string())?;
                writer.string(kind_name(*kind))?;
                writer.string(&lower.to_string())?;
                writer.string(&upper.to_string())?;
                writer.end_array()?;
            }
        }
        writer.end_array()?;
        writer.name("past")?;
        writer.begin_array()?;
        for (index, candidates) in self.0.past.iter().enumerate() {
            if candidates.is_empty() {
                continue;
            }
            writer.begin_array()?;
            writer.string(&index.to_string())?;
            writer.begin_array()?;
            for candidate in candidates {
                writer.begin_array()?;
                writer.string(&candidate.age.to_string())?;
                encode_residual(writer, &candidate.witness)?;
                encode_residual(writer, &candidate.guard)?;
                writer.end_array()?;
            }
            writer.end_array()?;
            writer.end_array()?;
        }
        writer.end_array()?;
        writer.name("residual")?;
        encode_residual(writer, &self.0.residual)?;
        writer.end_object()
    }
}

fn encode_residual<S: quire_canonical::Sink + ?Sized>(
    writer: &mut quire_canonical::Writer<'_, S>,
    residual: &Residual,
) -> Result<(), quire_canonical::Error> {
    match residual {
        Residual::Constant(value) => {
            writer.begin_array()?;
            writer.string("constant")?;
            writer.bool(*value)?;
            writer.end_array()
        }
        Residual::Not(inner) => {
            writer.begin_array()?;
            writer.string("not")?;
            encode_residual(writer, inner)?;
            writer.end_array()
        }
        Residual::And(parts) | Residual::Or(parts) => {
            writer.begin_array()?;
            writer.string(if matches!(residual, Residual::And(_)) {
                "and"
            } else {
                "or"
            })?;
            writer.begin_array()?;
            for part in parts {
                encode_residual(writer, part)?;
            }
            writer.end_array()?;
            writer.end_array()
        }
        Residual::Future { node, lower, upper } => {
            writer.begin_array()?;
            writer.string("future")?;
            writer.string(&node.to_string())?;
            writer.string(&lower.to_string())?;
            writer.string(&upper.to_string())?;
            writer.end_array()
        }
    }
}

struct KeyReservation {
    used: u64,
    max: u64,
    denied: Option<u64>,
}

impl quire_canonical::Reservation for KeyReservation {
    fn reserve(&mut self, bytes: usize) -> Result<(), quire_canonical::ReservationDenied> {
        let bytes = u64::try_from(bytes).map_err(|_| quire_canonical::ReservationDenied)?;
        let Some(next) = self.used.checked_add(bytes) else {
            self.denied = Some(u64::MAX);
            return Err(quire_canonical::ReservationDenied);
        };
        if next > self.max {
            self.denied = Some(next);
            return Err(quire_canonical::ReservationDenied);
        }
        self.used = next;
        Ok(())
    }
}

fn specs(
    checked: &CheckedFormula,
    formula: &CheckedTemporalFormula,
    scale: u64,
) -> Result<Vec<NodeSpec>, TimedFormulaMonitorError> {
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
                        return Err(TimedFormulaMonitorError::Unsupported);
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
                FormulaNode::Aggregate { .. } => return Err(TimedFormulaMonitorError::Unsupported),
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
) -> Result<(u64, u64), TimedFormulaMonitorError> {
    let Some(CheckedInterval::Timed(interval)) = interval else {
        return Err(TimedFormulaMonitorError::Unsupported);
    };
    if !interval.lower_closed() || !interval.upper_closed() {
        return Err(TimedFormulaMonitorError::Unsupported);
    }
    let upper = interval
        .upper()
        .ok_or(TimedFormulaMonitorError::Unsupported)?;
    let lower = time_units(interval.lower(), scale).ok_or(TimedFormulaMonitorError::Unsupported)?;
    let upper = time_units(upper, scale).ok_or(TimedFormulaMonitorError::Unsupported)?;
    if lower > upper {
        return Err(TimedFormulaMonitorError::Unsupported);
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
        limits: TimedFormulaMonitorLimits,
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

    fn node(&mut self, index: usize) -> Result<Residual, TimedFormulaMonitorError> {
        poll(self.cancel)?;
        if let Some(value) = self.cache.get(index).and_then(Option::as_ref) {
            return Ok(value.clone());
        }
        let spec = self
            .specs
            .get(index)
            .cloned()
            .ok_or(TimedFormulaMonitorError::Unsupported)?;
        let value = match spec {
            NodeSpec::Constant(value) => Residual::Constant(value),
            NodeSpec::Atom(slot) => Residual::Constant(
                *self
                    .values
                    .get(slot)
                    .ok_or(TimedFormulaMonitorError::Unsupported)?,
            ),
            NodeSpec::Not(operand) => Self::negate(self.state_bound, self.node(operand)?)?,
            NodeSpec::And(left, right) => Self::join(
                self.state_bound,
                true,
                [self.node(left)?, self.node(right)?],
            )?,
            NodeSpec::Or(left, right) => Self::join(
                self.state_bound,
                false,
                [self.node(left)?, self.node(right)?],
            )?,
            NodeSpec::Implies(left, right) => {
                let left = Self::negate(self.state_bound, self.node(left)?)?;
                Self::join(self.state_bound, false, [left, self.node(right)?])?
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
                    if kind == TemporalKind::Always && upper == 0 {
                        return Ok(witness);
                    }
                    let guard = left.map(|id| self.node(id)).transpose()?;
                    self.future_step(index, kind, lower, upper, 0, witness, guard)?
                } else {
                    self.past_node(index, kind, lower, upper, left, right)?
                }
            }
        };
        self.check_term(&value)?;
        let slot = self
            .cache
            .get_mut(index)
            .ok_or(TimedFormulaMonitorError::Unsupported)?;
        *slot = Some(value.clone());
        Ok(value)
    }

    fn join(
        state_bound: u64,
        conjunction: bool,
        parts: impl IntoIterator<Item = Residual>,
    ) -> Result<Residual, TimedFormulaMonitorError> {
        let unit = conjunction;
        let absorbing = !conjunction;
        let mut terms = BTreeSet::new();
        let mut cells = 1_u64;
        for part in parts {
            let nested = match part {
                Residual::And(inner) if conjunction => inner,
                Residual::Or(inner) if !conjunction => inner,
                other => vec![other],
            };
            for term in nested {
                match term {
                    Residual::Constant(value) if value == absorbing => {
                        return Ok(Residual::Constant(absorbing));
                    }
                    Residual::Constant(value) if value == unit => {}
                    other if terms.contains(&other) => {}
                    other => {
                        let next = cells.saturating_add(term_size(&other));
                        if next > state_bound {
                            return Err(TimedFormulaMonitorError::States {
                                bound: state_bound,
                                actual: next,
                            });
                        }
                        cells = next;
                        terms.insert(other);
                    }
                }
            }
        }
        Ok(match terms.len() {
            0 => Residual::Constant(unit),
            1 => terms.into_iter().next().expect("one residual term"),
            _ if conjunction => Residual::And(terms.into_iter().collect()),
            _ => Residual::Or(terms.into_iter().collect()),
        })
    }

    fn negate(state_bound: u64, term: Residual) -> Result<Residual, TimedFormulaMonitorError> {
        Ok(match term {
            Residual::Constant(value) => Residual::Constant(!value),
            Residual::Not(inner) => *inner,
            Residual::And(parts) => Self::join(
                state_bound,
                false,
                parts
                    .into_iter()
                    .map(|part| Self::negate(state_bound, part))
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
            Residual::Or(parts) => Self::join(
                state_bound,
                true,
                parts
                    .into_iter()
                    .map(|part| Self::negate(state_bound, part))
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
            other => {
                let actual = 1_u64.saturating_add(term_size(&other));
                if actual > state_bound {
                    return Err(TimedFormulaMonitorError::States {
                        bound: state_bound,
                        actual,
                    });
                }
                Residual::Not(Box::new(other))
            }
        })
    }

    fn future_step(
        &self,
        node: usize,
        kind: TemporalKind,
        lower: u64,
        upper: u64,
        delta: u64,
        witness: Residual,
        guard: Option<Residual>,
    ) -> Result<Residual, TimedFormulaMonitorError> {
        if delta > upper {
            return Ok(Residual::Constant(matches!(
                kind,
                TemporalKind::Always | TemporalKind::Release
            )));
        }
        let contains = lower <= delta;
        let rest = Residual::Future {
            node,
            lower: lower.saturating_sub(delta),
            upper: upper - delta,
        };
        match kind {
            TemporalKind::Eventually => {
                if contains {
                    Self::join(self.state_bound, false, [witness, rest])
                } else {
                    Ok(rest)
                }
            }
            TemporalKind::Always => {
                if contains {
                    Self::join(self.state_bound, true, [witness, rest])
                } else {
                    Ok(rest)
                }
            }
            TemporalKind::Until => {
                let guard = guard.unwrap_or(Residual::Constant(false));
                if contains {
                    Self::join(
                        self.state_bound,
                        false,
                        [witness, Self::join(self.state_bound, true, [guard, rest])?],
                    )
                } else {
                    Self::join(self.state_bound, true, [guard, rest])
                }
            }
            TemporalKind::Release => {
                let guard = guard.unwrap_or(Residual::Constant(false));
                if contains {
                    Self::join(
                        self.state_bound,
                        true,
                        [witness, Self::join(self.state_bound, false, [guard, rest])?],
                    )
                } else {
                    Self::join(self.state_bound, false, [guard, rest])
                }
            }
            TemporalKind::Once
            | TemporalKind::Historically
            | TemporalKind::Since
            | TemporalKind::Triggered => Ok(Residual::Constant(false)),
        }
    }

    fn past_node(
        &mut self,
        index: usize,
        kind: TemporalKind,
        lower: u64,
        upper: u64,
        left: Option<usize>,
        right: usize,
    ) -> Result<Residual, TimedFormulaMonitorError> {
        let dual = kind == TemporalKind::Triggered;
        let historical = kind == TemporalKind::Historically;
        let since = matches!(kind, TemporalKind::Since | TemporalKind::Triggered);
        let candidates = std::mem::take(
            self.past
                .get_mut(index)
                .ok_or(TimedFormulaMonitorError::Unsupported)?,
        );
        let source_cells = candidates.iter().fold(0_u64, |total, candidate| {
            total
                .saturating_add(1)
                .saturating_add(term_size(&candidate.witness))
                .saturating_add(term_size(&candidate.guard))
        });
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
                    .ok_or(TimedFormulaMonitorError::Unsupported)?
                    .clone();
                let current = if dual {
                    Self::negate(self.state_bound, current)?
                } else {
                    current
                };
                Self::join(self.state_bound, true, [progressed, current])?
            } else {
                self.progress(&candidate.guard, self.delta)?
            };
            self.check_term(&witness)?;
            self.check_term(&guard)?;
            let next_candidate = Candidate {
                age,
                witness,
                guard,
            };
            self.check_candidate_insert(&next, &next_candidate, source_cells)?;
            next.insert(next_candidate);
        }
        let witness = if dual {
            Self::negate(self.state_bound, right_now)?
        } else {
            right_now
        };
        let current = Candidate {
            age: 0,
            witness,
            guard: Residual::Constant(true),
        };
        self.check_candidate_insert(&next, &current, source_cells)?;
        next.insert(current);

        let selected: Vec<_> = next
            .iter()
            .filter(|candidate| (lower..=upper).contains(&candidate.age))
            .cloned()
            .collect();
        *self
            .past
            .get_mut(index)
            .ok_or(TimedFormulaMonitorError::Unsupported)? = next;

        let terms = selected
            .into_iter()
            .map(|candidate| {
                if since {
                    Self::join(self.state_bound, true, [candidate.witness, candidate.guard])
                } else {
                    Ok(candidate.witness)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let accumulated = if historical {
            Self::join(self.state_bound, true, terms)?
        } else {
            Self::join(self.state_bound, false, terms)?
        };
        Ok(if dual {
            Self::negate(self.state_bound, accumulated)?
        } else {
            accumulated
        })
    }

    fn progress(
        &mut self,
        residual: &Residual,
        delta: u64,
    ) -> Result<Residual, TimedFormulaMonitorError> {
        poll(self.cancel)?;
        let next = match residual {
            Residual::Constant(value) => Residual::Constant(*value),
            Residual::Not(inner) => Self::negate(self.state_bound, self.progress(inner, delta)?)?,
            Residual::And(parts) => Self::join(
                self.state_bound,
                true,
                parts
                    .iter()
                    .map(|part| self.progress(part, delta))
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
            Residual::Or(parts) => Self::join(
                self.state_bound,
                false,
                parts
                    .iter()
                    .map(|part| self.progress(part, delta))
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
            Residual::Future { node, lower, upper } => {
                let spec = self
                    .specs
                    .get(*node)
                    .cloned()
                    .ok_or(TimedFormulaMonitorError::Unsupported)?;
                let NodeSpec::Temporal {
                    kind, left, right, ..
                } = spec
                else {
                    return Err(TimedFormulaMonitorError::Unsupported);
                };
                if !is_future(kind) {
                    return Err(TimedFormulaMonitorError::Unsupported);
                }
                let witness = self.node(right)?;
                let guard = left.map(|id| self.node(id)).transpose()?;
                self.future_step(*node, kind, *lower, *upper, delta, witness, guard)?
            }
        };
        self.check_term(&next)?;
        Ok(next)
    }

    fn check_term(&self, term: &Residual) -> Result<(), TimedFormulaMonitorError> {
        let actual = term_size(term);
        if actual > self.state_bound {
            return Err(TimedFormulaMonitorError::States {
                bound: self.state_bound,
                actual,
            });
        }
        Ok(())
    }

    fn check_candidate_insert(
        &self,
        candidates: &BTreeSet<Candidate>,
        candidate: &Candidate,
        source_cells: u64,
    ) -> Result<(), TimedFormulaMonitorError> {
        if candidates.contains(candidate) {
            return Ok(());
        }
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
            .saturating_add(source_cells)
            .saturating_add(candidates.iter().fold(0_u64, |total, candidate| {
                total
                    .saturating_add(1)
                    .saturating_add(term_size(&candidate.witness))
                    .saturating_add(term_size(&candidate.guard))
            }))
            .saturating_add(1)
            .saturating_add(term_size(&candidate.witness))
            .saturating_add(term_size(&candidate.guard));
        if actual > self.state_bound {
            return Err(TimedFormulaMonitorError::States {
                bound: self.state_bound,
                actual,
            });
        }
        Ok(())
    }
}

fn poll(cancel: &Cancel) -> Result<(), TimedFormulaMonitorError> {
    if cancel.poll() {
        return Err(TimedFormulaMonitorError::Cancelled);
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
/// Validate the canonical shape of a bounded timed formula monitor key.
pub fn validate_timed_formula_monitor_key(
    horizon: NodeRef<'_>,
    monitor: NodeRef<'_>,
    phase: Node<'_>,
) -> bool {
    if canonical_count(horizon).is_none()
        || !matches!(phase, Node::String("pending" | "accept" | "reject"))
    {
        return false;
    }
    let Node::Object(fields) = monitor.node() else {
        return false;
    };
    if fields.len() != 3 {
        return false;
    }
    let (Some(occurrences), Some(residual), Some(past)) = (
        monitor.get("occurrences"),
        monitor.get("residual"),
        monitor.get("past"),
    ) else {
        return false;
    };
    let Node::Array(occurrences) = occurrences.node() else {
        return false;
    };
    let mut operators = Vec::new();
    let mut previous = None;
    for occurrence in occurrences {
        let Node::Array(parts) = occurrence.node() else {
            return false;
        };
        if parts.len() != 4 {
            return false;
        }
        let parts = parts.collect::<Vec<_>>();
        let Some(index) = canonical_count(parts[0]) else {
            return false;
        };
        let Node::String(kind) = parts[1].node() else {
            return false;
        };
        if !matches!(
            kind,
            "eventually"
                | "always"
                | "until"
                | "release"
                | "once"
                | "historically"
                | "since"
                | "triggered"
        ) || previous.is_some_and(|previous| previous >= index)
        {
            return false;
        }
        let (Some(lower), Some(upper)) = (canonical_count(parts[2]), canonical_count(parts[3]))
        else {
            return false;
        };
        if lower > upper {
            return false;
        }
        previous = Some(index);
        operators.push((index, kind, lower, upper));
    }
    if !bounded_formula_residual(residual, &operators) {
        return false;
    }
    let Node::Array(past) = past.node() else {
        return false;
    };
    let mut previous = None;
    for entry in past {
        let Node::Array(parts) = entry.node() else {
            return false;
        };
        if parts.len() != 2 {
            return false;
        }
        let parts = parts.collect::<Vec<_>>();
        let Some(index) = canonical_count(parts[0]) else {
            return false;
        };
        if previous.is_some_and(|previous| previous >= index)
            || !operators.iter().any(|(occurrence, kind, _, _)| {
                *occurrence == index
                    && matches!(*kind, "once" | "historically" | "since" | "triggered")
            })
        {
            return false;
        }
        let Node::Array(candidates) = parts[1].node() else {
            return false;
        };
        if candidates.len() == 0 {
            return false;
        }
        for candidate in candidates {
            let Node::Array(candidate) = candidate.node() else {
                return false;
            };
            if candidate.len() != 3 {
                return false;
            }
            let candidate = candidate.collect::<Vec<_>>();
            let Some(age) = canonical_count(candidate[0]) else {
                return false;
            };
            if !operators
                .iter()
                .any(|(occurrence, _, _, upper)| *occurrence == index && age <= *upper)
                || !bounded_formula_residual(candidate[1], &operators)
                || !bounded_formula_residual(candidate[2], &operators)
            {
                return false;
            }
        }
        previous = Some(index);
    }
    true
}

fn canonical_count(node: NodeRef<'_>) -> Option<u64> {
    let Node::String(text) = node.node() else {
        return None;
    };
    let value = text.parse::<u64>().ok()?;
    (value.to_string() == text).then_some(value)
}

fn bounded_formula_residual(residual: NodeRef<'_>, operators: &[(u64, &str, u64, u64)]) -> bool {
    let Node::Array(parts) = residual.node() else {
        return false;
    };
    if parts.len() == 0 {
        return false;
    }
    let parts = parts.collect::<Vec<_>>();
    let Node::String(tag) = parts[0].node() else {
        return false;
    };
    match tag {
        "constant" => parts.len() == 2 && matches!(parts[1].node(), Node::Bool(_)),
        "not" => parts.len() == 2 && bounded_formula_residual(parts[1], operators),
        "and" | "or" => {
            if parts.len() != 2 {
                return false;
            }
            let Node::Array(mut children) = parts[1].node() else {
                return false;
            };
            children.len() >= 2 && children.all(|child| bounded_formula_residual(child, operators))
        }
        "future" => {
            if parts.len() != 4 {
                return false;
            }
            let (Some(index), Some(lower), Some(upper)) = (
                canonical_count(parts[1]),
                canonical_count(parts[2]),
                canonical_count(parts[3]),
            ) else {
                return false;
            };
            lower <= upper
                && operators.iter().any(|(occurrence, kind, _, end)| {
                    *occurrence == index
                        && matches!(*kind, "eventually" | "always" | "until" | "release")
                        && upper <= *end
                })
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn fixture() -> TimedFormulaMonitor {
        TimedFormulaMonitor {
            root: 0,
            scale: 1,
            specs: vec![NodeSpec::Temporal {
                kind: TemporalKind::Eventually,
                lower: 1,
                upper: 4,
                left: None,
                right: 0,
            }],
            past: vec![BTreeSet::new()],
            residual: Residual::Future {
                node: 0,
                lower: 1,
                upper: 4,
            },
            time: 0,
            last_values: vec![false],
            started: true,
            storage_limit: 1024,
        }
    }

    #[trace("FR-204-AC-21")]
    #[test]
    fn bounded_writer_matches_the_shared_monitor_wire_key() {
        let monitor = fixture();
        let value = monitor.key_checked(1024).expect("bounded key admitted");
        let expected = br#"{"occurrences":[["0","eventually","1","4"]],"past":[],"residual":["future","0","1","4"]}"#;
        assert_eq!(serde_json::to_vec(&value).expect("JSON key"), expected);
    }

    #[trace("FR-204-AC-21")]
    #[test]
    fn monitor_key_and_residual_growth_refuse_at_their_exact_budgets() {
        let monitor = fixture();
        assert!(matches!(
            monitor.key_checked(8),
            Err(TimedFormulaMonitorError::StorageBytes { bound: 8, actual }) if actual > 8
        ));
        assert!(matches!(
            Evaluator::join(
                1,
                true,
                [
                    Residual::Future {
                        node: 0,
                        lower: 0,
                        upper: 1
                    },
                    Residual::Future {
                        node: 1,
                        lower: 0,
                        upper: 1
                    },
                ],
            ),
            Err(TimedFormulaMonitorError::States {
                bound: 1,
                actual: 2
            })
        ));
        let mut reservation = KeyReservation {
            used: 0,
            max: 8,
            denied: None,
        };
        assert_eq!(
            quire_canonical::Reservation::reserve(&mut reservation, 9),
            Err(quire_canonical::ReservationDenied)
        );
        assert_eq!(reservation.used, 0);
        assert_eq!(reservation.denied, Some(9));
    }
}
