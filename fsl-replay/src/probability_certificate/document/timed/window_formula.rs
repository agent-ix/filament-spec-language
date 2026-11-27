// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-921's independent bounded window-formula monitor.

use super::*;
use fsl_semantics::check::{
    BinaryTemporal, CheckedFormula, CheckedInterval, CheckedTemporalFormula, FormulaNode,
    FormulaNodeId, UnaryTemporal,
};
use ix_trace_rs::implements;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Operator {
    Eventually,
    Always,
    Until,
    Release,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Formula {
    Constant(bool),
    Atom {
        read: usize,
        negated: bool,
    },
    And(Vec<usize>),
    Or(Vec<usize>),
    Temporal {
        operator: Operator,
        occurrence: usize,
        lower: u64,
        upper: u64,
        left: Option<usize>,
        right: usize,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Residual {
    Start,
    Yes,
    No,
    All(Vec<Self>),
    Any(Vec<Self>),
    Active {
        operator: Operator,
        occurrence: usize,
        lower: u64,
        upper: u64,
        elapsed: u64,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct State {
    position: u64,
    residual: Residual,
}

impl FormulaResidual for State {
    #[implements("FR-204-AC-21")]
    fn verdict(&self) -> Option<bool> {
        match self.residual {
            Residual::Yes => Some(true),
            Residual::No => Some(false),
            _ => None,
        }
    }

    #[implements("FR-204-AC-21")]
    fn canonical_member(&self, subdivisions: u64) -> JsonValue {
        json!(["window", canonical(&self.residual, subdivisions)])
    }
}

struct Plan<'a> {
    nodes: Vec<Formula>,
    root: usize,
    predicates: Vec<&'a fsl_semantics::check::CheckedPredicate>,
    horizon: u64,
}

/// A checker-owned monitor. Its plan and residual transition implementation
/// are reconstructed from the checked claim independently of EN-5.
pub(super) struct Monitor<'a> {
    plan: Plan<'a>,
}

impl FormulaMonitor<State> for Monitor<'_> {
    #[implements("FR-204-AC-21")]
    fn initial(&self) -> State {
        State {
            position: 0,
            residual: Residual::Start,
        }
    }

    fn horizon(&self) -> u64 {
        self.plan.horizon
    }

    #[implements("FR-204-AC-21")]
    fn delay(
        &self,
        _system: &ModelSystem<'_>,
        _binding: Option<&filament_exact::ObjectReference>,
        _state: &ModelSimState,
        previous: &State,
        elapsed: u64,
        _deadline: u64,
        _cancel: &Cancel,
    ) -> Result<State, ProbabilityDocumentError> {
        let delta = elapsed
            .checked_sub(previous.position)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        Ok(State {
            position: elapsed,
            residual: age(&previous.residual, delta),
        })
    }

    #[implements("FR-204-AC-21")]
    fn observe(
        &self,
        system: &ModelSystem<'_>,
        binding: Option<&filament_exact::ObjectReference>,
        state: &ModelSimState,
        previous: &State,
        elapsed: u64,
        _deadline: u64,
        cancel: &Cancel,
    ) -> Result<State, ProbabilityDocumentError> {
        let predicates: Vec<_> = self
            .plan
            .predicates
            .iter()
            .map(|predicate| (*predicate, binding))
            .collect();
        let readings = system
            .state_predicates_with_cancel(state, &predicates, cancel)
            .map_err(|_| ProbabilityDocumentError::Unsupported)?;
        let truths = readings
            .iter()
            .map(|reading| match reading {
                fsl_eval::simulation::model::PredicateReading::True => Ok(true),
                fsl_eval::simulation::model::PredicateReading::False => Ok(false),
                fsl_eval::simulation::model::PredicateReading::Undefined { .. } => {
                    Err(ProbabilityDocumentError::Unsupported)
                }
                fsl_eval::simulation::model::PredicateReading::Refused(_) => {
                    Err(ProbabilityDocumentError::Unsupported)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let residual = match previous.residual {
            Residual::Start => self.plan.at_position(self.plan.root, &truths)?,
            _ => {
                let elapsed_delta = elapsed
                    .checked_sub(previous.position)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                let aged = age(&previous.residual, elapsed_delta);
                self.plan.advance(&aged, &truths)?
            }
        };
        Ok(State {
            position: elapsed,
            residual,
        })
    }
}

impl<'a> Plan<'a> {
    #[implements("FR-204-AC-21")]
    fn compile(formula: &'a CheckedTemporalFormula, scale: u64) -> Result<Self, Vec<usize>> {
        let checked = formula.formula();
        let mut compiler = Compiler {
            source: formula,
            checked,
            scale,
            nodes: Vec::new(),
            predicates: Vec::new(),
            predicate_indices: BTreeMap::new(),
            horizon: 0,
            occurrence: 0,
        };
        let root = compiler.node(checked.root(), false, &[])?;
        Ok(Self {
            nodes: compiler.nodes,
            root,
            predicates: compiler.predicates,
            horizon: compiler.horizon,
        })
    }

    #[implements("FR-204-AC-21")]
    fn at_position(
        &self,
        index: usize,
        truths: &[bool],
    ) -> Result<Residual, ProbabilityDocumentError> {
        match self
            .nodes
            .get(index)
            .ok_or(ProbabilityDocumentError::Malformed)?
        {
            Formula::Constant(value) => Ok(if *value { Residual::Yes } else { Residual::No }),
            Formula::Atom { read, negated } => {
                let value = truths
                    .get(*read)
                    .copied()
                    .ok_or(ProbabilityDocumentError::Malformed)?
                    ^ *negated;
                Ok(if value { Residual::Yes } else { Residual::No })
            }
            Formula::And(children) => self.join(children, truths, true),
            Formula::Or(children) => self.join(children, truths, false),
            Formula::Temporal {
                operator,
                occurrence,
                lower,
                upper,
                ..
            } => self.step(operator, *occurrence, *lower, *upper, 0, truths),
        }
    }

    fn join(
        &self,
        children: &[usize],
        truths: &[bool],
        all: bool,
    ) -> Result<Residual, ProbabilityDocumentError> {
        let residuals = children
            .iter()
            .map(|child| self.at_position(*child, truths))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(if all {
            every(residuals)
        } else {
            some(residuals)
        })
    }

    #[implements("FR-204-AC-21")]
    fn step(
        &self,
        operator: &Operator,
        occurrence: usize,
        lower: u64,
        upper: u64,
        elapsed: u64,
        truths: &[bool],
    ) -> Result<Residual, ProbabilityDocumentError> {
        let Some(Formula::Temporal { left, right, .. }) = self.nodes.iter().find(
            |node| matches!(node, Formula::Temporal { occurrence: at, .. } if *at == occurrence),
        ) else {
            return Err(ProbabilityDocumentError::Malformed);
        };
        let within = (lower..=upper).contains(&elapsed);
        let again = Residual::Active {
            operator: operator.clone(),
            occurrence,
            lower,
            upper,
            elapsed,
        };
        let read = |node| self.at_position(node, truths);
        match operator {
            Operator::Eventually => Ok(some(vec![
                if within { read(*right)? } else { Residual::No },
                again,
            ])),
            Operator::Always => Ok(every(vec![
                if within { read(*right)? } else { Residual::Yes },
                again,
            ])),
            Operator::Until => Ok(some(vec![
                if within { read(*right)? } else { Residual::No },
                every(vec![
                    read(left.ok_or(ProbabilityDocumentError::Malformed)?)?,
                    again,
                ]),
            ])),
            Operator::Release => Ok(every(vec![
                if within { read(*right)? } else { Residual::Yes },
                some(vec![
                    read(left.ok_or(ProbabilityDocumentError::Malformed)?)?,
                    again,
                ]),
            ])),
        }
    }

    #[implements("FR-204-AC-21")]
    fn advance(
        &self,
        residual: &Residual,
        truths: &[bool],
    ) -> Result<Residual, ProbabilityDocumentError> {
        match residual {
            Residual::Start => Err(ProbabilityDocumentError::Malformed),
            Residual::Yes | Residual::No => Ok(residual.clone()),
            Residual::All(parts) => Ok(every(
                parts
                    .iter()
                    .map(|part| self.advance(part, truths))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Residual::Any(parts) => Ok(some(
                parts
                    .iter()
                    .map(|part| self.advance(part, truths))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            Residual::Active {
                operator,
                occurrence,
                lower,
                upper,
                elapsed,
            } => {
                if *elapsed > *upper {
                    return Ok(timeout(operator));
                }
                self.step(operator, *occurrence, *lower, *upper, *elapsed, truths)
            }
        }
    }
}

struct Compiler<'a, 'c> {
    source: &'a CheckedTemporalFormula,
    checked: &'c CheckedFormula,
    scale: u64,
    nodes: Vec<Formula>,
    predicates: Vec<&'a fsl_semantics::check::CheckedPredicate>,
    predicate_indices: BTreeMap<usize, usize>,
    horizon: u64,
    occurrence: usize,
}

impl Compiler<'_, '_> {
    fn node(
        &mut self,
        id: FormulaNodeId,
        negate: bool,
        prefix: &[usize],
    ) -> Result<usize, Vec<usize>> {
        let at = id.index();
        let mut path = prefix.to_vec();
        path.push(at);
        let source = self
            .checked
            .nodes()
            .get(at)
            .cloned()
            .ok_or_else(|| path.clone())?;
        let node = match &source {
            FormulaNode::Constant(value) => Formula::Constant(*value ^ negate),
            FormulaNode::Holds { .. } => {
                let read = if let Some(read) = self.predicate_indices.get(&at) {
                    *read
                } else {
                    let predicate = self.source.condition(at).ok_or_else(|| path.clone())?;
                    let read = self.predicates.len();
                    self.predicates.push(predicate);
                    self.predicate_indices.insert(at, read);
                    read
                };
                Formula::Atom {
                    read,
                    negated: negate,
                }
            }
            FormulaNode::Not(child) => return self.node(*child, !negate, &path),
            FormulaNode::And(left, right) | FormulaNode::Or(left, right) => {
                let all = matches!(&source, FormulaNode::And(..)) ^ negate;
                let left = self.node(*left, negate, &path)?;
                let right = self.node(*right, negate, &path)?;
                if all {
                    Formula::And(vec![left, right])
                } else {
                    Formula::Or(vec![left, right])
                }
            }
            FormulaNode::Implies(left, right) => {
                let (left_negate, right_negate, all) = if negate {
                    (false, true, true)
                } else {
                    (true, false, false)
                };
                let left = self.node(*left, left_negate, &path)?;
                let right = self.node(*right, right_negate, &path)?;
                if all {
                    Formula::And(vec![left, right])
                } else {
                    Formula::Or(vec![left, right])
                }
            }
            FormulaNode::Unary {
                operator,
                interval: Some(interval),
                operand,
            } => {
                let (lower, upper) =
                    closed_window(interval, self.scale).ok_or_else(|| path.clone())?;
                let operator = match (operator, negate) {
                    (UnaryTemporal::Eventually, false) | (UnaryTemporal::Always, true) => {
                        Operator::Eventually
                    }
                    (UnaryTemporal::Always, false) | (UnaryTemporal::Eventually, true) => {
                        Operator::Always
                    }
                    _ => return Err(path),
                };
                let occurrence = self.occurrence;
                self.occurrence = self.occurrence.checked_add(1).ok_or_else(|| path.clone())?;
                self.horizon = self
                    .horizon
                    .checked_add(upper)
                    .ok_or_else(|| path.clone())?;
                let right = self.node(*operand, negate, &path)?;
                Formula::Temporal {
                    operator,
                    occurrence,
                    lower,
                    upper,
                    left: None,
                    right,
                }
            }
            FormulaNode::Binary {
                operator,
                interval: Some(interval),
                left,
                right,
            } if matches!(operator, BinaryTemporal::Until | BinaryTemporal::Release) => {
                let (lower, upper) =
                    closed_window(interval, self.scale).ok_or_else(|| path.clone())?;
                let operator = match (operator, negate) {
                    (BinaryTemporal::Until, false) | (BinaryTemporal::Release, true) => {
                        Operator::Until
                    }
                    (BinaryTemporal::Release, false) | (BinaryTemporal::Until, true) => {
                        Operator::Release
                    }
                    (BinaryTemporal::Since | BinaryTemporal::Triggered, _) => {
                        return Err(path.clone());
                    }
                };
                let occurrence = self.occurrence;
                self.occurrence = self.occurrence.checked_add(1).ok_or_else(|| path.clone())?;
                self.horizon = self
                    .horizon
                    .checked_add(upper)
                    .ok_or_else(|| path.clone())?;
                let left = self.node(*left, negate, &path)?;
                let right = self.node(*right, negate, &path)?;
                Formula::Temporal {
                    operator,
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
        self.nodes.push(node);
        Ok(index)
    }
}

fn closed_window(interval: &CheckedInterval, scale: u64) -> Option<(u64, u64)> {
    let CheckedInterval::Timed(interval) = interval else {
        return None;
    };
    if !interval.lower_closed() || !interval.upper_closed() {
        return None;
    }
    Some((
        super::time_units(interval.lower(), scale)?,
        super::time_units(interval.upper()?, scale)?,
    ))
}

#[implements("FR-204-AC-21")]
fn every(parts: Vec<Residual>) -> Residual {
    let mut kept = Vec::new();
    for part in parts {
        match part {
            Residual::No => return Residual::No,
            Residual::Yes => {}
            Residual::All(children) => kept.extend(children),
            other => kept.push(other),
        }
    }
    canonical_group(kept, true)
}

#[implements("FR-204-AC-21")]
fn some(parts: Vec<Residual>) -> Residual {
    let mut kept = Vec::new();
    for part in parts {
        match part {
            Residual::Yes => return Residual::Yes,
            Residual::No => {}
            Residual::Any(children) => kept.extend(children),
            other => kept.push(other),
        }
    }
    canonical_group(kept, false)
}

#[implements("FR-204-AC-21")]
fn canonical_group(mut parts: Vec<Residual>, all: bool) -> Residual {
    parts.sort_by_key(|part| canonical(part, 1).to_string());
    parts.dedup();
    match parts.len() {
        0 if all => Residual::Yes,
        0 => Residual::No,
        1 => parts.pop().unwrap_or(Residual::No),
        _ if all => Residual::All(parts),
        _ => Residual::Any(parts),
    }
}

#[implements("FR-204-AC-21")]
fn age(residual: &Residual, delta: u64) -> Residual {
    match residual {
        Residual::Start | Residual::Yes | Residual::No => residual.clone(),
        Residual::All(parts) => every(parts.iter().map(|part| age(part, delta)).collect()),
        Residual::Any(parts) => some(parts.iter().map(|part| age(part, delta)).collect()),
        Residual::Active {
            operator,
            occurrence,
            lower,
            upper,
            elapsed,
        } => {
            let elapsed = elapsed.saturating_add(delta);
            if elapsed > *upper {
                timeout(operator)
            } else {
                Residual::Active {
                    operator: operator.clone(),
                    occurrence: *occurrence,
                    lower: *lower,
                    upper: *upper,
                    elapsed,
                }
            }
        }
    }
}

#[implements("FR-204-AC-21")]
fn timeout(operator: &Operator) -> Residual {
    match operator {
        Operator::Eventually | Operator::Until => Residual::No,
        Operator::Always | Operator::Release => Residual::Yes,
    }
}

#[implements("FR-204-AC-21")]
fn canonical(residual: &Residual, subdivisions: u64) -> JsonValue {
    match residual {
        Residual::Start => json!(["start"]),
        Residual::Yes => json!(["accept"]),
        Residual::No => json!(["reject"]),
        Residual::All(parts) => json!([
            "and",
            parts
                .iter()
                .map(|part| canonical(part, subdivisions))
                .collect::<Vec<_>>()
        ]),
        Residual::Any(parts) => json!([
            "or",
            parts
                .iter()
                .map(|part| canonical(part, subdivisions))
                .collect::<Vec<_>>()
        ]),
        Residual::Active {
            operator,
            occurrence,
            elapsed,
            ..
        } => json!([
            match operator {
                Operator::Eventually => "eventually",
                Operator::Always => "always",
                Operator::Until => "until",
                Operator::Release => "release",
            },
            occurrence.to_string(),
            (elapsed / subdivisions.max(1)).to_string(),
            (elapsed % subdivisions.max(1)).to_string()
        ]),
    }
}

#[implements("FR-204-AC-21")]
pub(super) fn monitor(claim: &CheckedProbabilisticClaim, scale: u64) -> Option<Monitor<'_>> {
    let fsl_semantics::check::ProbForm::Probability {
        event:
            fsl_semantics::check::Event::Formula {
                formula,
                timed_horizon: Some(_),
                ..
            },
        ..
    } = &claim.form
    else {
        return None;
    };
    let plan = Plan::compile(formula, scale).ok()?;
    let nodes = formula.formula().nodes();
    let root = nodes.get(formula.formula().root().index())?;
    let simple = match root {
        FormulaNode::Unary { operand, .. } => {
            matches!(nodes.get(operand.index()), Some(FormulaNode::Holds { .. }))
        }
        FormulaNode::Binary {
            operator: BinaryTemporal::Until,
            left,
            right,
            ..
        } => {
            matches!(nodes.get(left.index()), Some(FormulaNode::Holds { .. }))
                && matches!(nodes.get(right.index()), Some(FormulaNode::Holds { .. }))
        }
        _ => false,
    };
    if simple || is_punctual_dual(root, nodes) {
        return None;
    }
    Some(Monitor { plan })
}

#[allow(clippy::too_many_arguments)]
#[implements("FR-204-AC-21")]
pub(super) fn check(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    bytes: &[u8],
    document: ProbabilityCertificateDocument,
    timed: &TimedSystem<'_, '_>,
    route: TimedProbabilityRoute,
    scale: u64,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    relation: super::super::ValueRelation,
    monitor: Monitor<'_>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    if route != TimedProbabilityRoute::Region
        || !matches!(claim.scheduler, fsl_semantics::check::ProbScheduler::Every)
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let fsl_semantics::check::ProbForm::Probability { bound, .. } = &claim.form else {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    };
    let optimization = match bound {
        Bound::AtLeast(_) => ProbabilityObjective::Minimum,
        Bound::AtMost(_) => ProbabilityObjective::Maximum,
    };
    if document.optimization != optimization {
        return Err(ProbabilityDocumentError::OptimizationMismatch);
    }
    if !document.ranking.is_empty()
        || document.cyclic_dtmc
        || document.event.is_some()
        || !document.failures.is_empty()
        || document.reward.is_some()
    {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let binding = claim_binding(system, claim)
        .map_err(|_| ProbabilityDocumentError::Unsupported)?
        .map(|binding| binding.reference);
    let horizon = monitor.horizon();
    let watch = Watch::Formula(&monitor);
    let graph = enumerate_region_deadline(
        system,
        timed,
        claim,
        binding.as_ref(),
        &watch,
        horizon,
        document
            .timed
            .as_ref()
            .ok_or(ProbabilityDocumentError::MalformedMember("objective"))?
            .scale,
        scale,
        limits,
        cancel,
        meter,
        storage,
        &mut BTreeMap::new(),
        &mut BTreeSet::new(),
    )?;
    let targets = target_keys(&graph);
    if targets != document.targets {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let certificate = ExactProbabilityCertificate {
        values: document.values,
    };
    let mut checked = super::super::check_reachability_values(
        &graph,
        &certificate,
        scheduler_objective(optimization),
        bound,
        super::super::GraphCycles::CollapseNonprogressive,
        relation,
        cancel,
    )
    .map_err(ProbabilityDocumentError::Graph)?;
    if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut checked {
        accepted.document = Some(bytes.into());
    }
    Ok(checked)
}

fn is_punctual_dual(root: &FormulaNode, nodes: &[FormulaNode]) -> bool {
    let FormulaNode::Unary {
        operator: outer,
        interval: Some(CheckedInterval::Timed(outer_interval)),
        operand,
    } = root
    else {
        return false;
    };
    let Some(FormulaNode::Unary {
        operator: inner,
        interval: Some(CheckedInterval::Timed(inner_interval)),
        ..
    }) = nodes.get(operand.index())
    else {
        return false;
    };
    matches!(
        (outer, inner),
        (UnaryTemporal::Eventually, UnaryTemporal::Always)
            | (UnaryTemporal::Always, UnaryTemporal::Eventually)
    ) && outer_interval.lower_closed()
        && outer_interval.upper_closed()
        && inner_interval.lower_closed()
        && inner_interval.upper_closed()
        && inner_interval
            .upper()
            .is_some_and(|upper| upper == inner_interval.lower())
}
