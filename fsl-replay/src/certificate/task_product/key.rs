// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-418's task-product state key: a node's `s`, "the complete
//! source-plus-scheduler product key modulo the finite live-job renaming".
//!
//! The key is the discrete part of a symbolic task-product state. Every
//! clock-valued quantity the concrete task-search key (`TaskProductSystem::
//! state_key`, which is what `task_search` nodes carry) writes as a rational
//! (source clock values, arrival ages, a job's demand, jitter, remaining
//! deadline and service) lives in the node's DBM instead, so it is absent
//! here. The remaining fields use that key's token grammar and its transport:
//! the canonical octets are written as one `|`-separated text, and the wire
//! spelling is the unpadded base64url of those octets in a `{key}` State
//! locus (FR-331), which is how `task_search` nodes carry theirs.
//!
//! Live jobs are named by their finite rank and slot, never by occurrence or
//! generation, so two states that differ only by a completed job's reused
//! slot share one key.

use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::simulation::model::{canonical_json, AllowancePhase, CriticalityMode};
use fsl_eval::simulation::StateKey;

/// Whether a task has been activated yet. The age since its last arrival is
/// an arrival clock, so only the phase is discrete.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TaskArrivalPhase {
    /// No activation has happened yet.
    NotArrived,
    /// At least one activation has happened; its age is the arrival clock.
    Arrived,
}

/// Where a live job is in the scheduler.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TaskJobPhase {
    /// Activated and waiting out its release jitter.
    Pending,
    /// Released and waiting in the ready queue.
    Ready,
    /// Running on the processor.
    Running,
}

/// One live job, by its finite identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskLiveJob {
    /// The task's position in the task set.
    pub task: usize,
    /// Zero-based canonical rank among this task's live jobs.
    pub rank: u64,
    /// The finite slot bound to the rank.
    pub slot: u64,
    /// Pending, ready or running.
    pub phase: TaskJobPhase,
    /// The execution allowance available to the job.
    pub allowance: AllowancePhase,
    /// The source operation that activated the job.
    pub release: ModelTransition,
}

/// A mode switch that has not yet raised a live job's allowance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskPendingBudgetChange {
    /// The task's position in the task set.
    pub task: usize,
    /// The affected job's rank.
    pub rank: u64,
    /// The source transition of the entry into HI.
    pub source_transition: ModelTransition,
}

/// The scheduler control of a task-product state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskSchedulerControl {
    /// Each task's arrival phase, in task-set order.
    pub arrivals: Vec<TaskArrivalPhase>,
    /// The live jobs.
    pub jobs: Vec<TaskLiveJob>,
    /// The ready queue, front first, as `(task, rank)` of each `Ready` job.
    pub queue: Vec<(usize, u64)>,
    /// The monotone criticality mode.
    pub mode: CriticalityMode,
    /// The pending allowance changes.
    pub pending_budget_changes: Vec<TaskPendingBudgetChange>,
    /// The deadline-miss latch.
    pub missed: bool,
}

/// Why a scheduler control is not a canonical product key.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskProductKeyRefusal {
    /// A job or change names a task outside the arrival list.
    #[error("a live job names task {task}, outside the task set")]
    UnknownTask {
        /// The named task.
        task: usize,
    },
    /// A task's ranks are not `0..n` without repeats.
    #[error("task {task}'s live-job ranks are not 0..n")]
    RankNotDense {
        /// The task.
        task: usize,
    },
    /// Two live jobs of one task share a slot.
    #[error("task {task} has two live jobs in slot {slot}")]
    SlotShared {
        /// The task.
        task: usize,
        /// The shared slot.
        slot: u64,
    },
    /// More than one job is running.
    #[error("more than one job is running")]
    MultipleRunning,
    /// The queue is not exactly the ready jobs, each once.
    #[error("the ready queue is not the set of ready jobs")]
    QueueMismatch,
    /// A pending change names no live job.
    #[error("a pending budget change names no live job")]
    DanglingChange,
    /// A source transition has no canonical spelling.
    #[error("a source transition has no canonical identity")]
    Transition,
}

/// FSpec FR-418's task-product state key `s`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductKey {
    source: StateKey,
    control: TaskSchedulerControl,
}

impl TaskProductKey {
    /// Builds the canonical key of a source state and scheduler control. The
    /// jobs and pending changes are put in `(task, rank)` order, so two
    /// controls that differ only in the order they list them are equal.
    ///
    /// # Errors
    ///
    /// [`TaskProductKeyRefusal`] when the control's jobs, queue or changes
    /// are not a consistent finite renaming.
    pub fn new(
        source: StateKey,
        mut control: TaskSchedulerControl,
    ) -> Result<Self, TaskProductKeyRefusal> {
        let tasks = control.arrivals.len();
        control.jobs.sort_by_key(|job| (job.task, job.rank));
        control
            .pending_budget_changes
            .sort_by_key(|change| (change.task, change.rank));
        let mut previous: Option<(usize, u64)> = None;
        for job in &control.jobs {
            if job.task >= tasks {
                return Err(TaskProductKeyRefusal::UnknownTask { task: job.task });
            }
            let expected = match previous {
                Some((task, rank)) if task == job.task => rank.checked_add(1),
                _ => Some(0),
            };
            if expected != Some(job.rank) {
                return Err(TaskProductKeyRefusal::RankNotDense { task: job.task });
            }
            previous = Some((job.task, job.rank));
        }
        for (index, job) in control.jobs.iter().enumerate() {
            let shared = control.jobs[..index]
                .iter()
                .any(|other| other.task == job.task && other.slot == job.slot);
            if shared {
                return Err(TaskProductKeyRefusal::SlotShared {
                    task: job.task,
                    slot: job.slot,
                });
            }
        }
        let running = control
            .jobs
            .iter()
            .filter(|job| job.phase == TaskJobPhase::Running)
            .count();
        if running > 1 {
            return Err(TaskProductKeyRefusal::MultipleRunning);
        }
        let ready = control
            .jobs
            .iter()
            .filter(|job| job.phase == TaskJobPhase::Ready)
            .count();
        let queued_ready = control.queue.iter().all(|(task, rank)| {
            control.jobs.iter().any(|job| {
                job.task == *task && job.rank == *rank && job.phase == TaskJobPhase::Ready
            })
        });
        let distinct = control
            .queue
            .iter()
            .enumerate()
            .all(|(index, entry)| !control.queue[..index].contains(entry));
        if control.queue.len() != ready || !queued_ready || !distinct {
            return Err(TaskProductKeyRefusal::QueueMismatch);
        }
        for change in &control.pending_budget_changes {
            let live = control
                .jobs
                .iter()
                .any(|job| job.task == change.task && job.rank == change.rank);
            if !live {
                return Err(TaskProductKeyRefusal::DanglingChange);
            }
        }
        let key = Self { source, control };
        key.octets()?;
        Ok(key)
    }

    /// The source model state's key.
    pub fn source(&self) -> &StateKey {
        &self.source
    }

    /// The scheduler control.
    pub fn control(&self) -> &TaskSchedulerControl {
        &self.control
    }

    /// The key's canonical octets: the source key in lowercase hex, then one
    /// `|`-separated field per concrete-key field that is not clock-valued.
    ///
    /// # Errors
    ///
    /// [`TaskProductKeyRefusal::Transition`] when a transition identity has
    /// no canonical spelling.
    pub fn octets(&self) -> Result<StateKey, TaskProductKeyRefusal> {
        let mut text = self.source.hex();
        // Arrival phases: `task:-;` before the first activation, `task:=;`
        // after it (the concrete key writes the age where `=` stands).
        text.push('|');
        for (task, phase) in self.control.arrivals.iter().enumerate() {
            text.push_str(&format!("{task}:"));
            text.push(match phase {
                TaskArrivalPhase::NotArrived => '-',
                TaskArrivalPhase::Arrived => '=',
            });
            text.push(';');
        }
        for job in &self.control.jobs {
            text.push('|');
            text.push_str(&format!("{}:{}:{}:", job.task, job.rank, job.slot));
            text.push(match job.allowance {
                AllowancePhase::Lo => 'l',
                AllowancePhase::Hi => 'h',
            });
            text.push(':');
            text.push(match job.phase {
                TaskJobPhase::Pending => 'p',
                TaskJobPhase::Ready => 'r',
                TaskJobPhase::Running => 'x',
            });
            text.push(':');
            text.push_str(&transition_text(&job.release)?);
        }
        text.push_str("|q");
        for (task, rank) in &self.control.queue {
            text.push_str(&format!(":{task}.{rank}"));
        }
        text.push(match self.control.mode {
            CriticalityMode::Lo => 'l',
            CriticalityMode::Hi => 'h',
        });
        for change in &self.control.pending_budget_changes {
            text.push_str(&format!(":{}@{}:", change.task, change.rank));
            text.push_str(&transition_text(&change.source_transition)?);
        }
        text.push(if self.control.missed { '!' } else { '.' });
        Ok(StateKey::from_canonical_bytes(text.into_bytes()))
    }

    /// FSpec FR-331's State locus `{key}` of this key: the unpadded
    /// base64url of [`Self::octets`], the spelling `task_search` nodes use.
    ///
    /// # Errors
    ///
    /// [`TaskProductKeyRefusal::Transition`] when the octets cannot be
    /// written.
    pub fn wire(&self) -> Result<serde_json::Value, TaskProductKeyRefusal> {
        let octets = self.octets()?;
        crate::state_key_wire(&octets).map_err(|_| TaskProductKeyRefusal::Transition)
    }
}

fn transition_text(transition: &ModelTransition) -> Result<String, TaskProductKeyRefusal> {
    let value = canonical_json(transition).ok_or(TaskProductKeyRefusal::Transition)?;
    serde_json::to_string(&value).map_err(|_| TaskProductKeyRefusal::Transition)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn release() -> ModelTransition {
        ModelTransition::new(
            "\"Task::release\"".to_owned(),
            None,
            Vec::new(),
            quire_canonical::Limits::new(1 << 16),
        )
        .expect("canonical identity")
    }

    fn job(task: usize, rank: u64, slot: u64, phase: TaskJobPhase) -> TaskLiveJob {
        TaskLiveJob {
            task,
            rank,
            slot,
            phase,
            allowance: AllowancePhase::Lo,
            release: release(),
        }
    }

    fn control(jobs: Vec<TaskLiveJob>, queue: Vec<(usize, u64)>) -> TaskSchedulerControl {
        TaskSchedulerControl {
            arrivals: vec![TaskArrivalPhase::Arrived, TaskArrivalPhase::NotArrived],
            jobs,
            queue,
            mode: CriticalityMode::Lo,
            pending_budget_changes: Vec::new(),
            missed: false,
        }
    }

    fn source() -> StateKey {
        StateKey::from_canonical_bytes(b"{\"n\":1}".to_vec())
    }

    /// The octets use the task-search key's grammar and transport.
    #[trace("TC-371", "FR-418-AC-13")]
    #[test]
    fn the_key_is_the_task_search_grammar_in_unpadded_base64url() {
        let key = TaskProductKey::new(
            source(),
            control(
                vec![
                    job(0, 0, 1, TaskJobPhase::Running),
                    job(0, 1, 0, TaskJobPhase::Ready),
                ],
                vec![(0, 1)],
            ),
        )
        .expect("key");
        let octets = key.octets().expect("octets");
        let text = String::from_utf8(octets.as_bytes().to_vec()).expect("text");
        assert_eq!(
            text,
            format!(
                "{}|0:=;1:-;|0:0:1:l:x:{{\"arguments\":[],\"operation\":\"\\\"Task::release\\\"\",\"receiver\":null,\"type\":\"transition\"}}|0:1:0:l:r:{{\"arguments\":[],\"operation\":\"\\\"Task::release\\\"\",\"receiver\":null,\"type\":\"transition\"}}|q:0.1l.",
                source().hex()
            )
        );
        let wire = key.wire().expect("wire");
        use base64::Engine as _;
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(wire["key"].as_str().expect("key"))
            .expect("base64url");
        assert_eq!(decoded, octets.as_bytes());
    }

    /// Listing order does not matter; slot reuse by a later rank does.
    #[trace("TC-371", "FR-418-AC-13")]
    #[test]
    fn the_key_is_canonical_in_the_job_listing_order() {
        let forward = control(
            vec![
                job(0, 0, 0, TaskJobPhase::Pending),
                job(1, 0, 0, TaskJobPhase::Pending),
            ],
            Vec::new(),
        );
        let reversed = control(
            vec![
                job(1, 0, 0, TaskJobPhase::Pending),
                job(0, 0, 0, TaskJobPhase::Pending),
            ],
            Vec::new(),
        );
        assert_eq!(
            TaskProductKey::new(source(), forward.clone()).expect("key"),
            TaskProductKey::new(source(), reversed).expect("key")
        );
        let other_slot = control(
            vec![
                job(0, 0, 1, TaskJobPhase::Pending),
                job(1, 0, 0, TaskJobPhase::Pending),
            ],
            Vec::new(),
        );
        let first = TaskProductKey::new(source(), forward).expect("key");
        let second = TaskProductKey::new(source(), other_slot).expect("key");
        assert_ne!(
            first.octets().expect("octets"),
            second.octets().expect("octets")
        );
    }

    /// An inconsistent finite renaming is not a key.
    #[trace("TC-371", "FR-418-AC-13")]
    #[test]
    fn an_inconsistent_renaming_is_refused() {
        let refused = |control| TaskProductKey::new(source(), control).expect_err("refused");
        assert_eq!(
            refused(control(
                vec![job(0, 1, 0, TaskJobPhase::Pending)],
                Vec::new()
            )),
            TaskProductKeyRefusal::RankNotDense { task: 0 }
        );
        assert_eq!(
            refused(control(
                vec![job(2, 0, 0, TaskJobPhase::Pending)],
                Vec::new()
            )),
            TaskProductKeyRefusal::UnknownTask { task: 2 }
        );
        assert_eq!(
            refused(control(
                vec![
                    job(0, 0, 0, TaskJobPhase::Pending),
                    job(0, 1, 0, TaskJobPhase::Pending)
                ],
                Vec::new()
            )),
            TaskProductKeyRefusal::SlotShared { task: 0, slot: 0 }
        );
        assert_eq!(
            refused(control(
                vec![
                    job(0, 0, 0, TaskJobPhase::Running),
                    job(1, 0, 0, TaskJobPhase::Running)
                ],
                Vec::new()
            )),
            TaskProductKeyRefusal::MultipleRunning
        );
        assert_eq!(
            refused(control(vec![job(0, 0, 0, TaskJobPhase::Ready)], Vec::new())),
            TaskProductKeyRefusal::QueueMismatch
        );
        assert_eq!(
            refused(control(
                vec![job(0, 0, 0, TaskJobPhase::Pending)],
                vec![(0, 0)]
            )),
            TaskProductKeyRefusal::QueueMismatch
        );
    }
}
