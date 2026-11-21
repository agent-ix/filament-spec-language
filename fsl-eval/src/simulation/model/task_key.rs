// SPDX-License-Identifier: AGPL-3.0-or-later
//! The one writer of the scheduler-control part of a task-product state key.
//!
//! The concrete task-search key ([`super::TaskProductSystem::state_key`]) and
//! FSpec FR-418's symbolic node key `s` (`fsl-replay`'s `TaskProductKey`) both
//! write their scheduler control through [`write_task_control`], so the two
//! encodings share one grammar. Text after the source key and the source
//! clock fields is:
//!
//! ```text
//! | task:age; ...                                   arrival phases
//! | task[:rank]:slot[:demand:jitter:deadline:service]:l|h:r|p:x|-:release  one per live job
//! [| q[:task.rank ...]]                             ready queue, symbolic only
//! l|h                                               criticality mode
//! [:task@index[#generation@occurrence]:source ...]  pending budget changes
//! !|.                                               miss latch
//! ```
//!
//! The concrete path supplies the bracketed concrete fields (an arrival's
//! age, the four job quantities, generation and occurrence); the symbolic
//! path leaves them out because the node's DBM holds them, and adds the
//! documented extension: each job's `rank`, the ready `queue`, and pending
//! changes located by rank. This is the single place the extension is
//! defined. The writer does not sort; the symbolic key sorts jobs and
//! pending changes by `(task, rank)` before calling it.

use filament_semantic_value::model_transition::ModelTransition;

/// A transition identity has no canonical spelling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a transition identity has no canonical spelling")]
pub struct TaskKeyIdentityRefusal;

/// One live job's key fields.
#[derive(Clone, Debug)]
pub struct TaskKeyJob<'a> {
    /// The task's position in the task set.
    pub task: usize,
    /// The canonical rank; symbolic keys only.
    pub rank: Option<u64>,
    /// The finite slot.
    pub slot: u64,
    /// Demand, jitter, remaining deadline and service as key text, in that
    /// order; concrete keys only, empty otherwise.
    pub concrete: Vec<String>,
    /// Whether the job runs on its HI allowance.
    pub high_allowance: bool,
    /// Whether the job is released (`r`) rather than waiting out jitter (`p`).
    pub ready: bool,
    /// Whether the job is running.
    pub running: bool,
    /// The source operation that activated the job.
    pub release: &'a ModelTransition,
}

/// One pending allowance change's key fields.
#[derive(Clone, Debug)]
pub struct TaskKeyPending<'a> {
    /// The task's position in the task set.
    pub task: usize,
    /// The job's slot (concrete) or rank (symbolic).
    pub index: u64,
    /// The slot generation and activation occurrence; concrete keys only.
    pub generation_occurrence: Option<(u64, u64)>,
    /// The source transition of the entry into HI.
    pub source_transition: &'a ModelTransition,
}

/// The scheduler control, in key order.
#[derive(Clone, Debug)]
pub struct TaskKeyControl<'a> {
    /// Per task: `None` before its first activation, else the age text the
    /// key spells (a rational concretely, `=` symbolically).
    pub arrivals: Vec<Option<String>>,
    /// The live jobs.
    pub jobs: Vec<TaskKeyJob<'a>>,
    /// The ready queue as `(task, rank)`, front first; symbolic keys only.
    pub queue: Option<Vec<(usize, u64)>>,
    /// Whether the criticality mode is HI.
    pub high_mode: bool,
    /// The pending allowance changes.
    pub pending: Vec<TaskKeyPending<'a>>,
    /// The deadline-miss latch.
    pub missed: bool,
}

/// Appends `control`'s tokens to `key`, which already holds the source key
/// and any source clock fields.
///
/// # Errors
///
/// [`TaskKeyIdentityRefusal`] when a release or source transition has no
/// canonical spelling.
pub fn write_task_control(
    key: &mut String,
    control: &TaskKeyControl<'_>,
) -> Result<(), TaskKeyIdentityRefusal> {
    key.push('|');
    for (task, arrival) in control.arrivals.iter().enumerate() {
        key.push_str(&task.to_string());
        key.push(':');
        key.push_str(arrival.as_deref().unwrap_or("-"));
        key.push(';');
    }
    for job in &control.jobs {
        key.push('|');
        key.push_str(&job.task.to_string());
        key.push(':');
        if let Some(rank) = job.rank {
            key.push_str(&rank.to_string());
            key.push(':');
        }
        key.push_str(&job.slot.to_string());
        key.push(':');
        for field in &job.concrete {
            key.push_str(field);
            key.push(':');
        }
        key.push(if job.high_allowance { 'h' } else { 'l' });
        key.push(':');
        key.push(if job.ready { 'r' } else { 'p' });
        key.push(':');
        key.push(if job.running { 'x' } else { '-' });
        key.push(':');
        push_identity(key, job.release)?;
    }
    if let Some(queue) = &control.queue {
        key.push_str("|q");
        for (task, rank) in queue {
            key.push_str(&format!(":{task}.{rank}"));
        }
    }
    key.push(if control.high_mode { 'h' } else { 'l' });
    for pending in &control.pending {
        key.push(':');
        key.push_str(&pending.task.to_string());
        key.push('@');
        key.push_str(&pending.index.to_string());
        if let Some((generation, occurrence)) = pending.generation_occurrence {
            key.push('#');
            key.push_str(&generation.to_string());
            key.push('@');
            key.push_str(&occurrence.to_string());
        }
        key.push(':');
        push_identity(key, pending.source_transition)?;
    }
    key.push(if control.missed { '!' } else { '.' });
    Ok(())
}

fn push_identity(
    key: &mut String,
    transition: &ModelTransition,
) -> Result<(), TaskKeyIdentityRefusal> {
    let value = super::canonical_json(transition).ok_or(TaskKeyIdentityRefusal)?;
    let text = serde_json::to_string(&value).map_err(|_| TaskKeyIdentityRefusal)?;
    key.push_str(&text);
    Ok(())
}

#[cfg(test)]
mod tests {
    use quire_canonical::Limits;

    use super::*;

    fn release() -> ModelTransition {
        ModelTransition::new("\"R\"".to_owned(), None, Vec::new(), Limits::new(1 << 16))
            .expect("canonical identity")
    }

    const IDENTITY: &str =
        r#"{"arguments":[],"operation":"\"R\"","receiver":null,"type":"transition"}"#;

    /// The concrete spelling of one running and one pending job with a
    /// pending change, as `TaskProductSystem::state_key` has always written
    /// it, and the symbolic spelling of the same control: the same grammar
    /// with the concrete fields dropped and rank, queue and rank-located
    /// changes added.
    #[test]
    fn the_concrete_and_symbolic_keys_share_one_grammar() {
        let release = release();
        let jobs = |rank: bool, concrete: bool| {
            vec![
                TaskKeyJob {
                    task: 0,
                    rank: rank.then_some(0),
                    slot: 1,
                    concrete: if concrete {
                        vec!["1/1".into(), "0/1".into(), "2/1".into(), "1/2".into()]
                    } else {
                        Vec::new()
                    },
                    high_allowance: false,
                    ready: true,
                    running: true,
                    release: &release,
                },
                TaskKeyJob {
                    task: 1,
                    rank: rank.then_some(0),
                    slot: 0,
                    concrete: if concrete {
                        vec!["3/1".into(), "0/1".into(), "4/1".into(), "0/1".into()]
                    } else {
                        Vec::new()
                    },
                    high_allowance: true,
                    ready: false,
                    running: false,
                    release: &release,
                },
            ]
        };
        let concrete = TaskKeyControl {
            arrivals: vec![Some("5/1".into()), None],
            jobs: jobs(false, true),
            queue: None,
            high_mode: true,
            pending: vec![TaskKeyPending {
                task: 1,
                index: 0,
                generation_occurrence: Some((2, 7)),
                source_transition: &release,
            }],
            missed: false,
        };
        let symbolic = TaskKeyControl {
            arrivals: vec![Some("=".into()), None],
            jobs: jobs(true, false),
            queue: Some(vec![(1, 0)]),
            high_mode: true,
            pending: vec![TaskKeyPending {
                task: 1,
                index: 0,
                generation_occurrence: None,
                source_transition: &release,
            }],
            missed: false,
        };
        let mut concrete_key = String::new();
        write_task_control(&mut concrete_key, &concrete).expect("writes");
        let mut symbolic_key = String::new();
        write_task_control(&mut symbolic_key, &symbolic).expect("writes");
        assert_eq!(
            concrete_key,
            format!(
                "|0:5/1;1:-;|0:1:1/1:0/1:2/1:1/2:l:r:x:{IDENTITY}|1:0:3/1:0/1:4/1:0/1:h:p:-:{IDENTITY}h:1@0#2@7:{IDENTITY}."
            )
        );
        assert_eq!(
            symbolic_key,
            format!(
                "|0:=;1:-;|0:0:1:l:r:x:{IDENTITY}|1:0:0:h:p:-:{IDENTITY}|q:1.0h:1@0:{IDENTITY}."
            )
        );
    }
}
