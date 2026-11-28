// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-173 "Event-position form" (ADR-018 TP-2, CX-1; ADR-023 HM-2): the closed
//! finite executions a trace variable of an event-position hyper clause ranges
//! over, which `fsl-analyze`'s EN-1 decides the clause over and `fsl-replay`
//! recomputes with the same code (ADR-018 LA-2).
//!
//! A closed finite execution of a subject is a path from an initial state that
//! ends at a terminal state, or that covers the body's ADR-014 TR-4 horizon:
//! exactly `h + 1` positions, 0 to `h`. A terminal state is not
//! stutter-extended and no execution is read as a lasso.

use std::collections::BTreeSet;

use filament_exact::{Cancel, Meter, ScalarLimits};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_foundation::bound::TemporalUpper;
use fsl_semantics::check::{
    CheckedFormula, CheckedHyperClause, CheckedInterval, FormulaNode, FormulaNodeId,
};
use ix_trace_rs::implements;

use crate::hyper::seam::{
    HyperLetters, MatchPart, MatchReading, ParameterBinding, StepLabel, StepMatch,
};
use crate::temporal::{
    evaluate_closed_tuple, read_closed_atom, AtomReading, ComponentPosition, LetterStop,
    TemporalOutcome, TemporalPosition, TupleLetters,
};

use crate::simulation::model::{ModelSimState, ModelSystem};
use crate::simulation::{EncodingRefusal, ExpansionStop, StateKey, TransitionSystem};

/// One step of a closed execution: the transition taken and its post-state's
/// state key.
#[derive(Clone, Debug)]
pub struct ClosedStep {
    /// The transition identity.
    pub transition: ModelTransition,
    /// The `filament.simulation.state-key/v1` key of the post-state.
    pub post_state: StateKey,
    /// Whether the step is skipped by the clause's `align skip` list (ADR-023
    /// PA-1): it enters no projected position.
    pub skipped: bool,
}

/// A closed finite execution of one subject.
#[derive(Clone, Debug)]
pub struct ClosedExecution {
    /// The index of its first state among the subject's initial states.
    pub initial: usize,
    /// Its states, the first the initial state: `steps.len() + 1` of them.
    pub states: Vec<ModelSimState>,
    /// Its steps, in order.
    pub steps: Vec<ClosedStep>,
}

impl ClosedExecution {
    /// The execution read at its visible positions (ADR-023 PA-2): position 0
    /// and the state after each step that is not skipped, with the visible
    /// steps alone. A skipped step enters no position, and skipped steps after
    /// the last visible one are not read.
    #[must_use]
    pub fn projection(&self) -> Self {
        let mut states = Vec::with_capacity(self.states.len());
        let mut steps = Vec::with_capacity(self.steps.len());
        states.extend(self.states.first().cloned());
        for (step, state) in self.steps.iter().zip(self.states.iter().skip(1)) {
            if !step.skipped {
                steps.push(step.clone());
                states.push(state.clone());
            }
        }
        Self {
            initial: self.initial,
            states,
            steps,
        }
    }

    /// The number of visible steps.
    #[must_use]
    pub fn visible_steps(&self) -> usize {
        self.steps.iter().filter(|step| !step.skipped).count()
    }
}

/// Why the closed executions of a subject were not all enumerated.
#[derive(Debug)]
pub enum ClosedStop {
    /// A state could not be expanded.
    Expansion(ExpansionStop),
    /// A state or transition has no canonical encoding.
    Encoding(EncodingRefusal),
    /// The enumeration passed its limit of executions.
    Executions {
        /// The limit.
        limit: usize,
    },
    /// The run was cancelled.
    Cancelled,
    /// A state's expansion evaluated a contract conjunction with no value
    /// (ADR-018 UE-6), never read as a disabled step: the execution that
    /// reaches the state, which ends there, and the conjunction.
    Undefined {
        /// The path from an initial state to the state with no value.
        prefix: Box<ClosedExecution>,
        /// The conjunction.
        contract: Box<crate::simulation::model::UndefinedContract>,
    },
    /// A state's expansion refused a contract conjunction, so what the state
    /// enables is not exactly known.
    Refused,
}

/// The expression of a closed tuple's reading that has no value (ADR-023
/// HV-8).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosedLocus {
    /// A `holds` atom of the body.
    Atom(FormulaNodeId),
    /// A universal `match` conjunct, by its index.
    Match {
        /// The conjunct's index among the universal conjuncts.
        conjunct: usize,
    },
    /// An existential `match` conjunct, by its index.
    ExistentialMatch {
        /// The conjunct's index among the existential conjuncts.
        conjunct: usize,
    },
    /// A `pre` or `post` clause of a contract conjunction, by its name.
    Contract(String),
}

/// The first expression with no value along a tuple of closed executions,
/// in the engine's reading order (ADR-018 UE-4): the joint position, what has
/// no value, the components it reads, the guarded transition of a
/// precondition, and why.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosedUndefined {
    /// The joint position, read over the visible positions of an `align skip`
    /// clause.
    pub position: u64,
    /// The expression.
    pub locus: ClosedLocus,
    /// The components whose states it reads.
    pub components: Vec<usize>,
    /// The transition a precondition guards.
    pub transition: Option<ModelTransition>,
    /// The index of the execution's state a contract conjunction was read at,
    /// among all its states, skipped steps included; `None` for a `match` or an
    /// atom, which read the visible positions.
    pub state: Option<usize>,
    /// Why it has no value.
    pub cause: crate::temporal::UndefinedCause,
}

/// The body's horizon (ADR-014 TR-4): the greatest sum of interval upper
/// bounds along a path from the root. `None` for a body with an operator that
/// has no finite upper bound (an event-position body has none: FR-326 refuses
/// it at S3, so the closed reading never guesses a horizon for it), for an
/// arena whose operands do not follow their operator and for a sum past `u64`.
pub fn horizon(formula: &CheckedFormula) -> Option<u64> {
    let nodes = formula.nodes();
    let mut reach = vec![0_u64; nodes.len()];
    let upper = |interval: &Option<CheckedInterval>| match interval {
        Some(CheckedInterval::Steps(steps)) => match steps.upper() {
            TemporalUpper::Finite(upper) => Some(upper),
            TemporalUpper::Open => None,
        },
        Some(CheckedInterval::Timed(_)) | None => None,
    };
    for (index, node) in nodes.iter().enumerate().rev() {
        let of = |reach: &[u64], operand: usize| {
            (operand > index)
                .then(|| reach.get(operand).copied())
                .flatten()
        };
        let total = match node {
            FormulaNode::Constant(_)
            | FormulaNode::Holds { .. }
            | FormulaNode::Aggregate { .. } => 0,
            FormulaNode::Not(operand) => of(&reach, operand.index())?,
            FormulaNode::And(left, right)
            | FormulaNode::Or(left, right)
            | FormulaNode::Implies(left, right) => {
                of(&reach, left.index())?.max(of(&reach, right.index())?)
            }
            FormulaNode::Unary {
                interval, operand, ..
            } => of(&reach, operand.index())?.checked_add(upper(interval)?)?,
            FormulaNode::Binary {
                interval,
                left,
                right,
                ..
            } => of(&reach, left.index())?
                .max(of(&reach, right.index())?)
                .checked_add(upper(interval)?)?,
        };
        *reach.get_mut(index)? = total;
    }
    reach.get(formula.root().index()).copied()
}

/// Every closed finite execution of `system` over `horizon`, from each
/// initial state in the subject's order, each state's successors in the order
/// of their transition and state keys, depth first. At most `limit` of them.
/// `skips` are the texts of the operations the clause skips (`align skip`):
/// an execution covers `horizon` visible steps and ends there, and a path of
/// more than `limit` steps is a stop, so skipped steps cannot run for ever.
///
/// # Errors
///
/// [`ClosedStop`] when a state cannot be expanded or encoded, the executions
/// pass `limit` or `cancel` stops the run.
pub fn closed_executions(
    system: &ModelSystem<'_>,
    horizon: u64,
    limit: usize,
    skips: &BTreeSet<String>,
    cancel: &Cancel,
) -> Result<Vec<ClosedExecution>, ClosedStop> {
    let depth = usize::try_from(horizon).unwrap_or(usize::MAX);
    let mut complete = Vec::new();
    for (index, initial) in system.initial().into_iter().enumerate() {
        let mut open = vec![ClosedExecution {
            initial: index,
            states: vec![initial],
            steps: Vec::new(),
        }];
        while let Some(execution) = open.pop() {
            if cancel.poll() {
                return Err(ClosedStop::Cancelled);
            }
            let Some(last) = execution.states.last() else {
                continue;
            };
            if execution.steps.len() > limit {
                return Err(ClosedStop::Executions { limit });
            }
            let successors = if execution.visible_steps() >= depth {
                Vec::new()
            } else {
                let expansion = system.successors(last).map_err(ClosedStop::Expansion)?;
                if let Some(contract) = expansion
                    .findings
                    .iter()
                    .find_map(crate::simulation::model::ModelFinding::undefined_contract)
                {
                    // The shortest prefix with such a state is the first refuting
                    // evidence (ADR-018 UE-4), not the first the depth-first walk meets.
                    let (prefix, contract) =
                        shortest_undefined(system, depth, limit, skips, cancel, &execution)?
                            .unwrap_or((execution, contract));
                    return Err(ClosedStop::Undefined {
                        prefix: Box::new(prefix),
                        contract: Box::new(contract),
                    });
                }
                if expansion
                    .findings
                    .iter()
                    .any(crate::simulation::model::ModelFinding::is_refused_contract)
                {
                    return Err(ClosedStop::Refused);
                }
                expansion.successors
            };
            if successors.is_empty() {
                complete.push(execution);
                if complete.len() > limit {
                    return Err(ClosedStop::Executions { limit });
                }
                continue;
            }
            let mut keyed = Vec::with_capacity(successors.len());
            for (transition, next) in successors {
                let (key, post_state) = system
                    .state_key(&next, fsl_foundation::IdentityLimits::default())
                    .map_err(ClosedStop::Encoding)?;
                let order = system
                    .transition_key(&transition)
                    .map_err(ClosedStop::Encoding)?;
                keyed.push(((order, key), transition, next, post_state));
            }
            keyed.sort_by(|left, right| left.0.cmp(&right.0));
            // The stack pops the last pushed first, so push in reverse order.
            for (_, transition, next, post_state) in keyed.into_iter().rev() {
                let mut states = execution.states.clone();
                states.push(next);
                let mut steps = execution.steps.clone();
                steps.push(ClosedStep {
                    skipped: skips.contains(transition.operation()),
                    transition,
                    post_state,
                });
                open.push(ClosedExecution {
                    initial: index,
                    states,
                    steps,
                });
            }
        }
    }
    Ok(complete)
}

/// The execution of `system` with the earliest visible position whose last
/// state has an undefined contract conjunction, when that position is earlier
/// than `found`'s (ADR-018 UE-4: the first refuting evidence, shortest prefix
/// first). The search is over states, each expanded once by its state key,
/// with a parent pointer to the path that reached it, level by visible
/// position: a skipped step stays in its level (ADR-023 PA-2), a level is read
/// in the order its states were reached, and a state's successors come in the
/// order of their transition and state keys. A state whose contract
/// conjunction is refused is not entered. `Ok(None)` when `found` is already
/// the earliest.
///
/// # Errors
///
/// [`ClosedStop::Executions`] when the search expands more than `limit`
/// states (each is charged as the walk charges an execution), and the walk's
/// own stops for a state it cannot expand or encode or a cancelled run.
pub fn shortest_undefined(
    system: &ModelSystem<'_>,
    depth: usize,
    limit: usize,
    skips: &BTreeSet<String>,
    cancel: &Cancel,
    found: &ClosedExecution,
) -> Result<Option<(ClosedExecution, crate::simulation::model::UndefinedContract)>, ClosedStop> {
    /// A state reached, and the step that reached it.
    struct Reached {
        parent: Option<usize>,
        step: Option<ClosedStep>,
        state: ModelSimState,
        initial: usize,
        key: Vec<u8>,
    }
    // The levels up to and including `found`'s own: another state at the same
    // visible position can come first in canonical order.
    let ceiling = found.visible_steps().min(depth.saturating_sub(1));
    let mut nodes: Vec<Reached> = Vec::new();
    let mut expanded_keys = BTreeSet::new();
    // The nodes of each visible level, in the order they were reached.
    let mut levels: Vec<Vec<usize>> = vec![Vec::new()];
    for (initial, state) in system.initial().into_iter().enumerate() {
        let (key, _) = system
            .state_key(&state, fsl_foundation::IdentityLimits::default())
            .map_err(ClosedStop::Encoding)?;
        levels.first_mut().map(|level| level.push(nodes.len()));
        nodes.push(Reached {
            parent: None,
            step: None,
            state,
            initial,
            key,
        });
    }
    let mut expanded = 0_usize;
    for level in 0..=ceiling {
        let mut cursor = 0_usize;
        while let Some(&index) = levels.get(level).and_then(|reached| reached.get(cursor)) {
            cursor += 1;
            if cancel.poll() {
                return Err(ClosedStop::Cancelled);
            }
            let Some(node) = nodes.get(index) else {
                return Ok(None);
            };
            if !expanded_keys.insert(node.key.clone()) {
                continue;
            }
            expanded += 1;
            if expanded > limit {
                return Err(ClosedStop::Executions { limit });
            }
            let expansion = system
                .successors(&node.state)
                .map_err(ClosedStop::Expansion)?;
            if let Some(contract) = expansion
                .findings
                .iter()
                .find_map(crate::simulation::model::ModelFinding::undefined_contract)
            {
                let mut chain = vec![index];
                while let Some(parent) = chain
                    .last()
                    .and_then(|last| nodes.get(*last))
                    .and_then(|node| node.parent)
                {
                    chain.push(parent);
                }
                chain.reverse();
                let mut states = Vec::with_capacity(chain.len());
                let mut steps = Vec::with_capacity(chain.len());
                for link in &chain {
                    if let Some(node) = nodes.get(*link) {
                        states.push(node.state.clone());
                        steps.extend(node.step.clone());
                    }
                }
                let reached = ClosedExecution {
                    initial: node.initial,
                    states,
                    steps,
                };
                return Ok(Some((reached, contract)));
            }
            if expansion
                .findings
                .iter()
                .any(crate::simulation::model::ModelFinding::is_refused_contract)
            {
                continue;
            }
            let initial = node.initial;
            let mut keyed = Vec::with_capacity(expansion.successors.len());
            for (transition, next) in expansion.successors {
                let (key, post_state) = system
                    .state_key(&next, fsl_foundation::IdentityLimits::default())
                    .map_err(ClosedStop::Encoding)?;
                let order = system
                    .transition_key(&transition)
                    .map_err(ClosedStop::Encoding)?;
                keyed.push(((order, key.clone()), key, transition, next, post_state));
            }
            keyed.sort_by(|left, right| left.0.cmp(&right.0));
            for (_, key, transition, next, post_state) in keyed {
                let skipped = skips.contains(transition.operation());
                let into = if skipped { level } else { level + 1 };
                while levels.len() <= into {
                    levels.push(Vec::new());
                }
                levels
                    .get_mut(into)
                    .map(|reached| reached.push(nodes.len()));
                nodes.push(Reached {
                    parent: Some(index),
                    step: Some(ClosedStep {
                        skipped,
                        transition,
                        post_state,
                    }),
                    state: next,
                    initial,
                    key,
                });
            }
        }
    }
    Ok(None)
}

/// What reading a tuple of closed executions settles (FR-173 "Event-position
/// form", ADR-023 HM-3).
#[derive(Debug)]
pub enum ClosedVerdict {
    /// `μ_U` is false at a joint step every universal component takes: the
    /// tuple is outside the quantifier prefix and `match`.
    UniversalUnmatched,
    /// `μ_E` is false at a joint step every component takes.
    ExistentialUnmatched,
    /// The body holds on the matched tuple.
    Holds,
    /// The body is false on the matched tuple; `at` is the activation that is
    /// false.
    Violated {
        /// The joint position.
        at: u64,
    },
    /// A `match` conjunct was refused or incomplete at a joint step
    /// (ADR-023 HV-4).
    MatchUndetermined {
        /// The joint position the refused step enters (FR-177 "Ranking
        /// findings").
        position: u64,
    },
    /// A `match` conjunct or an atom of the body has no value on the tuple
    /// (ADR-023 HV-8).
    Undefined(Box<ClosedUndefined>),
    /// The evaluation meter ran out.
    Incomplete {
        /// The joint position where the reading stopped; `0` when the body's
        /// evaluator reports none, so the stop ranks first and never gives way
        /// to a refutation at a later position (FR-177 "Ranking findings").
        position: u64,
        /// The counter that ran out.
        limit: filament_exact::LimitKind,
    },
    /// An invariant of the evaluation broke.
    Fault(InternalFault),
}

/// What a tuple of closed executions is read against.
pub struct ClosedInputs<'a> {
    /// The recompiled clause.
    pub clause: &'a CheckedHyperClause,
    /// The instance of the clause's object parameters.
    pub instance: &'a [ParameterBinding],
    /// The seam to the body's `holds` atoms.
    pub letters: &'a dyn HyperLetters,
    /// The seam to the clause's `match` block.
    pub matcher: &'a dyn StepMatch,
    /// The evaluation budget of each atom, conjunct and body.
    pub meter: ScalarLimits,
    /// The run's cancellation handle.
    pub cancel: &'a Cancel,
}

fn closed_fault(invariant: &'static str) -> ClosedVerdict {
    ClosedVerdict::Fault(InternalFault::new("hyper-closed", invariant))
}

/// The body's atoms on a tuple of closed executions: component `c` at joint
/// position `p` reads its own execution's state at `p`.
struct ClosedStates<'a> {
    letters: &'a dyn HyperLetters,
    instance: &'a [ParameterBinding],
    executions: &'a [&'a ClosedExecution],
}

impl TupleLetters for ClosedStates<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        _joint: TemporalPosition,
        components: &[ComponentPosition],
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let mut states: Vec<&ModelSimState> = Vec::with_capacity(components.len());
        for (component, read) in components.iter().enumerate() {
            let state = usize::try_from(read.position.index())
                .ok()
                .zip(self.executions.get(component))
                .and_then(|(index, execution)| execution.states.get(index))
                .ok_or_else(|| {
                    LetterStop::Fault(InternalFault::new("hyper-closed", "a-position-is-a-state"))
                })?;
            states.push(state);
        }
        self.letters
            .atom(atom, self.instance, &states, meter)
            .map_err(LetterStop::from)
    }
}

/// The reading of a `match` part at joint step `joint` (`1`-based). A component
/// with no such step is labelled ended, and a conjunct that reads it is not
/// evaluated there, so `μ` constrains only the joint steps every component a
/// conjunct reads takes (FR-173 "Event-position form").
/// Read one entering joint step's match, retaining ended-component handling
/// and the ordinary checked evaluator's refusal and fault distinctions.
/// `joint` is one-based; `part` selects the universal prefix or all components.
///
/// # Errors
///
/// A fault or undetermined match that prevents the tuple from being decided.
pub fn match_closed_step(
    inputs: &ClosedInputs<'_>,
    executions: &[&ClosedExecution],
    joint: usize,
    part: MatchPart,
) -> Result<MatchReading, ClosedVerdict> {
    let Some(step) = joint.checked_sub(1) else {
        return Err(closed_fault("an-entering-closed-step-is-one-based"));
    };
    let components = match part {
        MatchPart::Universal => inputs.clause.universals(),
        MatchPart::Existential => executions.len(),
    };
    if components > executions.len() {
        return Err(closed_fault("a-closed-match-has-every-component"));
    }
    let mut labels = Vec::with_capacity(components);
    for execution in executions.iter().take(components) {
        labels.push(match execution.steps.get(step) {
            Some(step) => StepLabel::Operation(&step.transition),
            None => StepLabel::Ended,
        });
    }
    let mut meter = Meter::new(inputs.meter).with_cancel(inputs.cancel.clone());
    let reading = match part {
        MatchPart::Existential => inputs
            .matcher
            .existential(inputs.instance, &labels, &mut meter),
        MatchPart::Universal => inputs
            .matcher
            .universal(inputs.instance, &labels, &mut meter),
    };
    match reading {
        Ok(MatchReading::Fault(fault)) => Err(ClosedVerdict::Fault(fault)),
        Ok(reading) => Ok(reading),
        Err(_) => Err(ClosedVerdict::MatchUndetermined {
            position: u64::try_from(joint).unwrap_or(u64::MAX),
        }),
    }
}

/// Read `executions`, one closed execution per trace variable in quantifier
/// order, against the clause (FR-173 "Event-position form"): `μ_U` over the
/// universal components' steps, `μ_E` over every component's, then the body
/// over the closed joint trace.
pub fn judge_closed_tuple(
    inputs: &ClosedInputs<'_>,
    executions: &[&ClosedExecution],
) -> ClosedVerdict {
    let clause = inputs.clause;
    // An `align skip` clause is read over the visible positions (ADR-023 PA-2).
    let projected: Vec<ClosedExecution>;
    let executions: Vec<&ClosedExecution> = if clause.align_skip().is_empty() {
        executions.to_vec()
    } else {
        projected = executions
            .iter()
            .map(|execution| execution.projection())
            .collect();
        projected.iter().collect()
    };
    let executions = executions.as_slice();
    let universals = clause.universals();
    let parts = [
        (universals, false, !clause.mu_universal().is_empty()),
        (executions.len(), true, !clause.mu_existential().is_empty()),
    ];
    let longest_of = |components: usize| {
        executions
            .iter()
            .take(components)
            .map(|execution| execution.steps.len())
            .max()
            .unwrap_or(0)
    };
    let overall = longest_of(executions.len());
    // The `match` is read at the joint positions in increasing order, `μ_U`
    // and then `μ_E` at one position (FR-183); the first conjunct with no
    // value waits for the body, whose atom at an earlier position is read
    // first (ADR-018 UE-4).
    let mut undefined_match: Option<Box<ClosedUndefined>> = None;
    'joints: for joint in 1..=overall {
        for (components, existential, present) in parts {
            if !present || joint > longest_of(components) {
                continue;
            }
            let part = if existential {
                MatchPart::Existential
            } else {
                MatchPart::Universal
            };
            match match_closed_step(inputs, executions, joint, part) {
                Ok(MatchReading::Holds) => {}
                Ok(MatchReading::Fails) => {
                    return if existential {
                        ClosedVerdict::ExistentialUnmatched
                    } else {
                        ClosedVerdict::UniversalUnmatched
                    };
                }
                Ok(MatchReading::Undefined { conjunct, cause }) => {
                    undefined_match = Some(Box::new(ClosedUndefined {
                        position: u64::try_from(joint).unwrap_or(u64::MAX),
                        locus: if existential {
                            ClosedLocus::ExistentialMatch { conjunct }
                        } else {
                            ClosedLocus::Match { conjunct }
                        },
                        components: (0..executions.len()).collect(),
                        transition: None,
                        state: None,
                        cause,
                    }));
                    break 'joints;
                }
                Ok(_) => {
                    let stop = ClosedVerdict::MatchUndetermined {
                        position: u64::try_from(joint).unwrap_or(u64::MAX),
                    };
                    return earlier_body_atom(inputs, executions, joint, stop);
                }
                Err(verdict @ ClosedVerdict::MatchUndetermined { .. }) => {
                    return earlier_body_atom(inputs, executions, joint, verdict);
                }
                Err(verdict) => return verdict,
            }
        }
    }
    let body = judge_closed_body(inputs, executions);
    match undefined_match {
        None => body,
        Some(found) => match body {
            ClosedVerdict::Undefined(atom) if atom.position < found.position => {
                ClosedVerdict::Undefined(atom)
            }
            // A body stop that ranks before the `match` conjunct's position
            // is the finding: it leaves the tuple undecided (FR-177).
            stop @ ClosedVerdict::Incomplete { position, .. } if position < found.position => stop,
            _ => ClosedVerdict::Undefined(found),
        },
    }
}

/// A refused or incomplete `match` at `joint` does not hide an atom of the body
/// with no value at an earlier position: that atom is the finding, as an
/// undefined `match` conjunct already waits for the body (FR-177 "Ranking
/// findings"). Otherwise `stop` stands.
fn earlier_body_atom(
    inputs: &ClosedInputs<'_>,
    executions: &[&ClosedExecution],
    joint: usize,
    stop: ClosedVerdict,
) -> ClosedVerdict {
    match judge_closed_body(inputs, executions) {
        ClosedVerdict::Undefined(atom)
            if atom.position < u64::try_from(joint).unwrap_or(u64::MAX) =>
        {
            ClosedVerdict::Undefined(atom)
        }
        _ => stop,
    }
}

/// The verdict of the clause's body over `executions`, read over the
/// executions as given: an undefined atom is the first in position and then
/// clause-node order.
fn judge_closed_body(inputs: &ClosedInputs<'_>, executions: &[&ClosedExecution]) -> ClosedVerdict {
    let clause = inputs.clause;
    let lens: Vec<u64> = executions
        .iter()
        .map(|execution| u64::try_from(execution.states.len()).unwrap_or(u64::MAX))
        .collect();
    let mut letters = ClosedStates {
        letters: inputs.letters,
        instance: inputs.instance,
        executions,
    };
    let mut meter = Meter::new(inputs.meter).with_cancel(inputs.cancel.clone());
    match evaluate_closed_tuple(clause, &lens, &mut letters, &mut meter) {
        Ok(TemporalOutcome::Holds) => ClosedVerdict::Holds,
        Ok(TemporalOutcome::Violated { at }) => ClosedVerdict::Violated { at: at.index() },
        Ok(TemporalOutcome::Undefined(found)) => {
            // The first undefined atom at that position, in clause-node order.
            let index = found.at.index();
            for node in 0..clause.body().nodes().len() {
                let atom = FormulaNodeId::from_arena_index(node);
                let Some(reads) = clause.component_reads(atom) else {
                    continue;
                };
                match read_closed_atom(clause, &lens, &mut letters, atom, index, &mut meter) {
                    Ok(AtomReading::Undefined(cause)) => {
                        let components: BTreeSet<usize> = reads.values().copied().collect();
                        return ClosedVerdict::Undefined(Box::new(ClosedUndefined {
                            position: index,
                            locus: ClosedLocus::Atom(atom),
                            components: components.into_iter().collect(),
                            transition: None,
                            state: None,
                            cause,
                        }));
                    }
                    Ok(AtomReading::True | AtomReading::False) => {}
                    Err(_) => break,
                }
            }
            closed_fault("an-undefined-closed-atom-is-found")
        }
        Ok(TemporalOutcome::Incomplete(incomplete)) => ClosedVerdict::Incomplete {
            position: 0,
            limit: incomplete.limit_kind,
        },
        Ok(
            TemporalOutcome::WindowLimitReached(_)
            | TemporalOutcome::Pending
            | TemporalOutcome::InsufficientData(_)
            | TemporalOutcome::MissingFairnessPremise { .. },
        ) => closed_fault("a-closed-body-evaluates-to-a-truth-value"),
        Err(_) => closed_fault("the-evaluator-reads-the-closed-tuple"),
    }
}

/// The index tuples of lists of `lens` members, in lexicographic order, the
/// first list varying slowest: one empty tuple for no list, none when a list
/// is empty.
pub struct Tuples {
    lens: Vec<usize>,
    next: Option<Vec<usize>>,
}

impl Tuples {
    /// The tuples over lists of `lens` members.
    pub fn over(lens: Vec<usize>) -> Self {
        let next = lens.iter().all(|len| *len > 0).then(|| vec![0; lens.len()]);
        Self { lens, next }
    }
}

impl Iterator for Tuples {
    type Item = Vec<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next.take()?;
        let mut following = current.clone();
        let mut place = following.len();
        while place > 0 {
            place -= 1;
            if let (Some(index), Some(len)) = (following.get_mut(place), self.lens.get(place)) {
                *index += 1;
                if *index < *len {
                    self.next = Some(following);
                    return Some(current);
                }
                *index = 0;
            }
        }
        Some(current)
    }
}

/// What the search of an HP-3 clause's existential tuples against one
/// universal tuple settles (ADR-023 HC-2, FR-173 "Event-position form").
#[derive(Debug)]
pub enum PartnerSearch {
    /// Some existential tuple is matched and the body holds on it.
    Witnessed,
    /// No existential tuple is matched with the body holding: the universal
    /// tuple has no partner. `position` is the first joint position at which
    /// the last candidate is lost, 0 when there were no matched candidates.
    Exhausted {
        /// The joint position.
        position: u64,
    },
    /// The universal tuple is outside `μ_U`.
    UniversalUnmatched,
    /// The existential tuples passed the budget.
    TooMany {
        /// The budget.
        limit: u64,
    },
    /// An expression has no value on the universal tuple with the existential
    /// tuple of these indices (ADR-023 HV-8): the clause is refuted.
    Undefined {
        /// What has no value.
        found: Box<ClosedUndefined>,
        /// The index of each existential variable's execution in its list.
        existentials: Vec<usize>,
    },
    /// A reading stopped the search; it settles no partner.
    Stopped(ClosedVerdict),
    /// The run was cancelled.
    Cancelled,
}

/// Search the product of `existentials`, one list of closed executions per
/// existential variable in quantifier order, for a partner of `universal`,
/// one execution per universal variable. At most `limit` existential tuples.
/// Every partner is read: one with an expression with no value refutes, and the
/// one whose position is earliest is reported, the first in list order on a
/// tie (ADR-018 UE-4, ADR-023 HV-8, FR-177), whatever holding partner comes
/// before or after it. Only when none is undefined does a holding partner
/// witness. A partner whose reading was refused or incomplete is ranked by the
/// joint position where it stopped (ADR-023 HV-8 HP-3, FR-177 "Ranking
/// findings"): one that stops at a position before the earliest undefined
/// partner's leaves the tuple undecided, and at the same position the
/// undefined partner ranks first; with no partner undefined or holding the
/// earliest stop leaves it undecided. A fault ends the search at once.
#[implements("FR-177-AC-14")]
pub fn search_partner(
    inputs: &ClosedInputs<'_>,
    universal: &[&ClosedExecution],
    existentials: &[&[ClosedExecution]],
    limit: u64,
) -> PartnerSearch {
    let lens: Vec<usize> = existentials.iter().map(|list| list.len()).collect();
    let count = lens
        .iter()
        .try_fold(1_u64, |product, len| {
            product.checked_mul(u64::try_from(*len).unwrap_or(u64::MAX))
        })
        .unwrap_or(u64::MAX);
    if count > limit {
        return PartnerSearch::TooMany { limit };
    }
    let mut position = 0_u64;
    let mut witnessed = false;
    // The earliest undefined partner so far; a shorter one replaces it, a tie
    // keeps the first in list order.
    let mut earliest: Option<(Box<ClosedUndefined>, Vec<usize>)> = None;
    // The partner whose reading stopped without a value or a verdict at the
    // least joint position, the first in list order on a tie.
    let mut undecided: Option<ClosedVerdict> = None;
    for picked in Tuples::over(lens) {
        if inputs.cancel.poll() {
            return PartnerSearch::Cancelled;
        }
        let mut tuple: Vec<&ClosedExecution> = universal.to_vec();
        for (list, index) in existentials.iter().zip(&picked) {
            match list.get(*index) {
                Some(execution) => tuple.push(execution),
                None => return PartnerSearch::Stopped(closed_fault("an-index-is-in-its-list")),
            }
        }
        let verdict = judge_closed_tuple(inputs, &tuple);
        match verdict {
            ClosedVerdict::Holds => witnessed = true,
            ClosedVerdict::UniversalUnmatched => return PartnerSearch::UniversalUnmatched,
            ClosedVerdict::ExistentialUnmatched => {}
            ClosedVerdict::Violated { at } => position = position.max(at),
            ClosedVerdict::Undefined(found) => {
                if earliest
                    .as_ref()
                    .map_or(true, |(best, _)| found.position < best.position)
                {
                    earliest = Some((found, picked));
                }
            }
            fault @ ClosedVerdict::Fault(_) => return PartnerSearch::Stopped(fault),
            stopped @ (ClosedVerdict::MatchUndetermined { .. }
            | ClosedVerdict::Incomplete { .. }) => {
                let at = stop_position(&stopped);
                if undecided
                    .as_ref()
                    .map_or(true, |best| at < stop_position(best))
                {
                    undecided = Some(stopped);
                }
            }
        }
    }
    // FR-177 "Ranking findings": the finding at the least joint position is the
    // instance's, and at one position an undefined evaluation comes before a
    // refused or incomplete one.
    match (earliest, undecided) {
        (Some((found, _)), Some(stopped)) if stop_position(&stopped) < found.position => {
            PartnerSearch::Stopped(stopped)
        }
        (Some((found, existentials)), _) => PartnerSearch::Undefined {
            found,
            existentials,
        },
        (None, _) if witnessed => PartnerSearch::Witnessed,
        (None, Some(stopped)) => PartnerSearch::Stopped(stopped),
        (None, None) => PartnerSearch::Exhausted { position },
    }
}

/// The joint position a refused or incomplete verdict stopped at.
#[implements("FR-177-AC-14")]
fn stop_position(verdict: &ClosedVerdict) -> u64 {
    match verdict {
        ClosedVerdict::MatchUndetermined { position }
        | ClosedVerdict::Incomplete { position, .. } => *position,
        _ => 0,
    }
}

/// What reading a tuple of closed executions, or a joint prefix of them, for
/// an expression with no value found (ADR-023 HV-8).
#[derive(Debug)]
pub enum ClosedRead {
    /// The first expression with no value, in reading order.
    Undefined(Box<ClosedUndefined>),
    /// `μ` is false at a joint step: the tuple is outside the clause's scope.
    Unmatched,
    /// Every expression the tuple reads has a value.
    Clear,
    /// A state could not be expanded for its contract conjunctions.
    Expansion {
        /// The component whose state it was.
        component: usize,
        /// Why.
        stop: ExpansionStop,
    },
    /// A reading stopped before it found a value.
    Stopped(ClosedVerdict),
}

/// Read `executions`, one per trace variable in quantifier order, for the
/// first expression with no value (ADR-018 UE-4, UE-6; ADR-023 HV-8), in the
/// one order the producer and the replay share and FR-183 states: at each
/// joint position in increasing order, the `match` over the step entering it
/// (`μ_U`, then `μ_E`), then the atoms of the body at it, then the contract
/// conjunctions of the states it holds, by component. A contract conjunction
/// of any component, universal or existential, refutes (FR-177). An `align
/// skip` clause reads its `match` and atoms over the visible positions and its
/// contract conjunctions on the unprojected states, each at the visible
/// position it follows. `systems` holds each component's subject. The
/// executions may differ in length: a component that has ended reads false
/// (FR-043) and is expanded no further.
pub fn read_closed_undefined(
    inputs: &ClosedInputs<'_>,
    systems: &[&ModelSystem<'_>],
    executions: &[&ClosedExecution],
) -> ClosedRead {
    match judge_closed_tuple(inputs, executions) {
        // A contract at a position before the `match` or atom with no value
        // is read first (UE-4).
        ClosedVerdict::Undefined(found) => {
            return match read_contracts(inputs, systems, executions, Some(found.position)) {
                ClosedRead::Clear => ClosedRead::Undefined(found),
                earlier => earlier,
            };
        }
        ClosedVerdict::UniversalUnmatched | ClosedVerdict::ExistentialUnmatched => {
            return ClosedRead::Unmatched;
        }
        stopped @ (ClosedVerdict::MatchUndetermined { .. }
        | ClosedVerdict::Incomplete { .. }
        | ClosedVerdict::Fault(_)) => return ClosedRead::Stopped(stopped),
        ClosedVerdict::Holds | ClosedVerdict::Violated { .. } => {}
    }
    read_contracts(inputs, systems, executions, None)
}

/// The visible position each state of `execution` follows: the number of
/// visible steps before it (every step of a clause with no `align skip`).
fn visible_positions(execution: &ClosedExecution) -> Vec<u64> {
    let mut positions = Vec::with_capacity(execution.states.len());
    let mut position = 0_u64;
    positions.push(position);
    for step in &execution.steps {
        if !step.skipped {
            position += 1;
        }
        positions.push(position);
    }
    positions
}

/// The first contract conjunction with no value, by visible position, then
/// component, then state, at the positions before `before` when given;
/// [`ClosedRead::Clear`] when there is none.
fn read_contracts(
    inputs: &ClosedInputs<'_>,
    systems: &[&ModelSystem<'_>],
    executions: &[&ClosedExecution],
    before: Option<u64>,
) -> ClosedRead {
    let clause = inputs.clause;
    let Some(depth) = horizon(clause.body()) else {
        return ClosedRead::Stopped(closed_fault("a-body-has-a-horizon"));
    };
    let limit = before.map_or(depth, |before| before.min(depth));
    let positions: Vec<Vec<u64>> = executions
        .iter()
        .map(|execution| visible_positions(execution))
        .collect();
    let last = positions
        .iter()
        .filter_map(|of| of.last().copied())
        .max()
        .unwrap_or(0);
    for position in 0..=last {
        if position >= limit {
            break;
        }
        for (component, execution) in executions.iter().enumerate() {
            for (index, state) in execution.states.iter().enumerate() {
                if positions[component].get(index) != Some(&position) {
                    continue;
                }
                let Some(system) = systems.get(component) else {
                    return ClosedRead::Stopped(closed_fault("a-component-has-a-system"));
                };
                let expansion = match system.successors(state) {
                    Ok(expansion) => expansion,
                    Err(stop) => return ClosedRead::Expansion { component, stop },
                };
                if let Some(contract) = expansion
                    .findings
                    .iter()
                    .find_map(crate::simulation::model::ModelFinding::undefined_contract)
                {
                    return ClosedRead::Undefined(Box::new(ClosedUndefined {
                        position,
                        locus: ClosedLocus::Contract(contract.clause),
                        components: vec![component],
                        transition: contract.guard_of,
                        state: Some(index),
                        cause: contract.cause,
                    }));
                }
            }
        }
    }
    ClosedRead::Clear
}

/// The state key of the state a [`ClosedUndefined`] names in `execution`, the
/// component's execution, which the producer and the replay both spell: the
/// unprojected state of a contract conjunction, otherwise the state at the
/// joint position, read over the visible positions of an `align skip` clause
/// (ADR-023 PA-2). `None` when the execution has no such state.
///
/// # Errors
///
/// [`EncodingRefusal`] when the state has no canonical key.
pub fn closed_found_state_key(
    system: &ModelSystem<'_>,
    clause: &CheckedHyperClause,
    execution: &ClosedExecution,
    found: &ClosedUndefined,
) -> Result<Option<StateKey>, EncodingRefusal> {
    let Some(index) = found.state else {
        return closed_state_key(system, clause, execution, found.position);
    };
    let Some(state) = execution.states.get(index) else {
        return Ok(None);
    };
    system
        .state_key(state, fsl_foundation::IdentityLimits::default())
        .map(|(_, key)| Some(key))
}

/// The state key of `execution`'s state at joint `position` of `clause`, read
/// over the visible positions of an `align skip` clause (ADR-023 PA-2): the
/// state a [`ClosedUndefined`] names, which the producer and the replay both
/// spell. `None` when the execution has no such position.
///
/// # Errors
///
/// [`EncodingRefusal`] when the state has no canonical key.
pub fn closed_state_key(
    system: &ModelSystem<'_>,
    clause: &CheckedHyperClause,
    execution: &ClosedExecution,
    position: u64,
) -> Result<Option<StateKey>, EncodingRefusal> {
    let Ok(index) = usize::try_from(position) else {
        return Ok(None);
    };
    let projected;
    let read = if clause.align_skip().is_empty() {
        execution
    } else {
        projected = execution.projection();
        &projected
    };
    let Some(state) = read.states.get(index) else {
        return Ok(None);
    };
    system
        .state_key(state, fsl_foundation::IdentityLimits::default())
        .map(|(_, key)| Some(key))
}
