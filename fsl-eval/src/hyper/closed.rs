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

use crate::hyper::seam::{
    HyperLetters, MatchPart, MatchReading, ParameterBinding, StepLabel, StepMatch,
};
use crate::temporal::{
    evaluate_closed_tuple, AtomReading, ComponentPosition, LetterStop, TemporalOutcome,
    TemporalPosition, TupleLetters,
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
                system
                    .successors(last)
                    .map_err(ClosedStop::Expansion)?
                    .successors
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
    /// A `match` conjunct has no value or was refused or incomplete at a
    /// joint step (ADR-023 HV-4).
    MatchUndetermined,
    /// An atom of the body has no value on the tuple (ADR-023 HV-8).
    Undefined,
    /// The evaluation meter ran out.
    Incomplete(filament_exact::LimitKind),
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
        Err(_) => Err(ClosedVerdict::MatchUndetermined),
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
    for (components, existential, present) in parts {
        if !present {
            continue;
        }
        let longest = executions
            .iter()
            .take(components)
            .map(|execution| execution.steps.len())
            .max()
            .unwrap_or(0);
        for joint in 1..=longest {
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
                Ok(_) => return ClosedVerdict::MatchUndetermined,
                Err(verdict) => return verdict,
            }
        }
    }
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
        Ok(TemporalOutcome::Undefined(_)) => ClosedVerdict::Undefined,
        Ok(TemporalOutcome::Incomplete(incomplete)) => {
            ClosedVerdict::Incomplete(incomplete.limit_kind)
        }
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
    /// A reading stopped the search; it settles no partner.
    Stopped(ClosedVerdict),
    /// The run was cancelled.
    Cancelled,
}

/// Search the product of `existentials`, one list of closed executions per
/// existential variable in quantifier order, for a partner of `universal`,
/// one execution per universal variable. At most `limit` existential tuples.
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
        match judge_closed_tuple(inputs, &tuple) {
            ClosedVerdict::Holds => return PartnerSearch::Witnessed,
            ClosedVerdict::UniversalUnmatched => return PartnerSearch::UniversalUnmatched,
            ClosedVerdict::ExistentialUnmatched => {}
            ClosedVerdict::Violated { at } => position = position.max(at),
            stopped @ (ClosedVerdict::MatchUndetermined
            | ClosedVerdict::Undefined
            | ClosedVerdict::Incomplete(_)
            | ClosedVerdict::Fault(_)) => return PartnerSearch::Stopped(stopped),
        }
    }
    PartnerSearch::Exhausted { position }
}
