// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-100-AC-8: replay-owned task transition values and evaluator conversions.

use filament_exact::Rational;
use filament_semantic_value::model_transition::ModelTransition;

use fsl_eval::simulation::model as evaluator;

/// An action in the task-product scheduler (FSpec FR-419).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskTransitionAction {
    /// A source transition nominally activates one task job.
    Activate,
    /// The job becomes available after its selected release jitter.
    Release,
    /// The scheduler starts running the job.
    Dispatch,
    /// The scheduler stops the job because a higher-ranked job is ready.
    Preempt,
    /// The scheduler starts a previously preempted job again.
    Resume,
    /// The job has received its selected total demand.
    Complete,
    /// A source mode change increases a live high-criticality job's allowance.
    BudgetChange,
    /// The job's nominal deadline is reached with positive remaining work.
    DeadlineMiss,
}

/// Exact identity and provenance of one derived task-scheduler step
/// (FSpec FR-181/FR-419).
///
/// The identity repeats immutable job fields on each step. `job` is the
/// zero-based occurrence among all nominal activations of the task, regardless
/// of which bound source operation activated it. `slot` and `generation`
/// identify its storage while live. Certificate edges use FR-418's finite
/// template instead.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTransitionIdentity {
    /// The scheduler action.
    pub action: TaskTransitionAction,
    /// The checked task-set name.
    pub task_set: String,
    /// The checked task name.
    pub task: String,
    /// The exact activation occurrence shared across this task's release operations.
    pub job: u64,
    /// The finite slot that stores this live job.
    pub slot: u64,
    /// The number of times this slot has been reused.
    pub generation: u64,
    /// The source operation whose transition activated the job.
    pub release: ModelTransition,
    /// The source transition that enabled this scheduler step, when present.
    pub source_transition: Option<ModelTransition>,
    /// The exact demand selected for this job.
    pub demand: Rational,
    /// The exact release delay selected for this job.
    pub jitter: Rational,
    /// The absolute deadline measured from nominal activation.
    pub deadline: Rational,
    /// The declared priority, absent when the task has none.
    pub priority: Option<u64>,
    /// The exact high-mode allowance increase on a budget-change step only.
    pub budget_delta: Option<Rational>,
}

impl fsl_eval::simulation::SubjectTransition for TaskTransitionIdentity {
    type Fairness = filament_semantic_value::fairness::FairnessConstraint;
}

/// A transition on the lowered task-product subject, preserving the existing
/// source-transition and FR-364 task-transition wire arms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskProductTransition {
    /// A source transition that activates no checked task.
    Source(ModelTransition),
    /// A source-synchronized or autonomous scheduler action.
    Task(Box<TaskTransitionIdentity>),
    /// The timed source is quiescent and lets time elapse forever without a
    /// discrete source or scheduler step (FR-181 terminal stutter).
    TerminalStutter,
}

impl fsl_eval::simulation::SubjectTransition for TaskProductTransition {
    type Fairness = filament_semantic_value::fairness::FairnessConstraint;

    fn formula_lasso_trace_position(prefix: &[Self]) -> Option<u64> {
        prefix
            .iter()
            .position(|step| {
                matches!(
                    step,
                    Self::Task(identity)
                        if identity.action == TaskTransitionAction::DeadlineMiss
                )
            })
            .and_then(|position| u64::try_from(position).ok())
    }
}

impl From<evaluator::TaskTransitionAction> for TaskTransitionAction {
    fn from(action: evaluator::TaskTransitionAction) -> Self {
        match action {
            evaluator::TaskTransitionAction::Activate => Self::Activate,
            evaluator::TaskTransitionAction::Release => Self::Release,
            evaluator::TaskTransitionAction::Dispatch => Self::Dispatch,
            evaluator::TaskTransitionAction::Preempt => Self::Preempt,
            evaluator::TaskTransitionAction::Resume => Self::Resume,
            evaluator::TaskTransitionAction::Complete => Self::Complete,
            evaluator::TaskTransitionAction::BudgetChange => Self::BudgetChange,
            evaluator::TaskTransitionAction::DeadlineMiss => Self::DeadlineMiss,
        }
    }
}

impl From<TaskTransitionAction> for evaluator::TaskTransitionAction {
    fn from(action: TaskTransitionAction) -> Self {
        match action {
            TaskTransitionAction::Activate => Self::Activate,
            TaskTransitionAction::Release => Self::Release,
            TaskTransitionAction::Dispatch => Self::Dispatch,
            TaskTransitionAction::Preempt => Self::Preempt,
            TaskTransitionAction::Resume => Self::Resume,
            TaskTransitionAction::Complete => Self::Complete,
            TaskTransitionAction::BudgetChange => Self::BudgetChange,
            TaskTransitionAction::DeadlineMiss => Self::DeadlineMiss,
        }
    }
}

impl From<evaluator::TaskTransitionIdentity> for TaskTransitionIdentity {
    fn from(identity: evaluator::TaskTransitionIdentity) -> Self {
        Self {
            action: identity.action.into(),
            task_set: identity.task_set,
            task: identity.task,
            job: identity.job,
            slot: identity.slot,
            generation: identity.generation,
            release: identity.release,
            source_transition: identity.source_transition,
            demand: identity.demand,
            jitter: identity.jitter,
            deadline: identity.deadline,
            priority: identity.priority,
            budget_delta: identity.budget_delta,
        }
    }
}

impl From<TaskTransitionIdentity> for evaluator::TaskTransitionIdentity {
    fn from(identity: TaskTransitionIdentity) -> Self {
        Self {
            action: identity.action.into(),
            task_set: identity.task_set,
            task: identity.task,
            job: identity.job,
            slot: identity.slot,
            generation: identity.generation,
            release: identity.release,
            source_transition: identity.source_transition,
            demand: identity.demand,
            jitter: identity.jitter,
            deadline: identity.deadline,
            priority: identity.priority,
            budget_delta: identity.budget_delta,
        }
    }
}

impl From<evaluator::TaskProductTransition> for TaskProductTransition {
    fn from(transition: evaluator::TaskProductTransition) -> Self {
        match transition {
            evaluator::TaskProductTransition::Source(source) => Self::Source(source),
            evaluator::TaskProductTransition::Task(task) => Self::Task(Box::new((*task).into())),
            evaluator::TaskProductTransition::TerminalStutter => Self::TerminalStutter,
        }
    }
}

impl From<TaskProductTransition> for evaluator::TaskProductTransition {
    fn from(transition: TaskProductTransition) -> Self {
        match transition {
            TaskProductTransition::Source(source) => Self::Source(source),
            TaskProductTransition::Task(task) => Self::Task(Box::new((*task).into())),
            TaskProductTransition::TerminalStutter => Self::TerminalStutter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// The facade conversion retains every scheduler identity field and arm.
    #[trace("FR-100-AC-8")]
    #[test]
    fn task_transition_wrappers_preserve_identity_and_alphabet() {
        let source = ModelTransition::new(
            r#"{"name":"release"}"#.to_owned(),
            None,
            Vec::new(),
            quire_canonical::Limits::new(4096),
        )
        .expect("source transition");
        let rational = |value| Rational::from_integer(filament_exact::Integer::from(value));
        let actions = [
            evaluator::TaskTransitionAction::Activate,
            evaluator::TaskTransitionAction::Release,
            evaluator::TaskTransitionAction::Dispatch,
            evaluator::TaskTransitionAction::Preempt,
            evaluator::TaskTransitionAction::Resume,
            evaluator::TaskTransitionAction::Complete,
            evaluator::TaskTransitionAction::BudgetChange,
            evaluator::TaskTransitionAction::DeadlineMiss,
        ];
        let mut converted_actions = Vec::new();
        for action in actions {
            let identity = evaluator::TaskTransitionIdentity {
                action,
                task_set: "tasks".to_owned(),
                task: "worker".to_owned(),
                job: 3,
                slot: 5,
                generation: 7,
                release: source.clone(),
                source_transition: Some(source.clone()),
                demand: rational(11_u64),
                jitter: rational(13_u64),
                deadline: rational(17_u64),
                priority: Some(19),
                budget_delta: Some(rational(23_u64)),
            };
            let transition = evaluator::TaskProductTransition::Task(Box::new(identity));
            let replay = TaskProductTransition::from(transition.clone());
            let TaskProductTransition::Task(task) = &replay else {
                panic!("task arm is preserved");
            };
            let wire = crate::task_transition_wire(task).expect("facade task spelling");
            assert_eq!(wire["taskset"], "tasks");
            assert_eq!(wire["task"], "worker");
            assert_eq!(wire["job"], 3);
            assert_eq!(wire["slot"], 5);
            assert_eq!(wire["generation"], 7);
            assert_eq!(wire["demand"]["numerator"], "11");
            assert_eq!(wire["jitter"]["numerator"], "13");
            assert_eq!(wire["deadline"]["numerator"], "17");
            assert_eq!(wire["priority"], 19);
            assert_eq!(wire["budget_delta"]["numerator"], "23");
            assert_eq!(wire["release"]["operation"], r#"{"name":"release"}"#);
            assert_eq!(
                wire["source_transition"]["operation"],
                r#"{"name":"release"}"#
            );
            assert!(
                !converted_actions.contains(&task.action),
                "actions stay distinct"
            );
            converted_actions.push(task.action);
            assert_eq!(evaluator::TaskProductTransition::from(replay), transition);
        }
        for transition in [
            evaluator::TaskProductTransition::Source(source),
            evaluator::TaskProductTransition::TerminalStutter,
        ] {
            let replay = TaskProductTransition::from(transition.clone());
            assert_eq!(evaluator::TaskProductTransition::from(replay), transition);
        }
    }
}
