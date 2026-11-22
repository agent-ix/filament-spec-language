// SPDX-License-Identifier: AGPL-3.0-or-later
//! Task-product metadata admitted before external materialization.

use super::*;
use crate::{ReferenceCodecError, ReferenceScope};

impl TaskProductCertificate {
    pub(super) fn write_scoped(
        &self,
        s: &mut ReferenceScope<'_, '_, '_, '_>,
    ) -> Result<Value, ReferenceCodecError> {
        let mut queues = s.array(self.queue_bounds.len())?;
        for (index, row) in self.queue_bounds.iter().enumerate() {
            queues.push(s.at_index("/queue_bounds", index, |s| {
                let arrival = s.text(row.arrival.as_str())?;
                let interarrival = s.rational(&row.interarrival)?;
                let jitter = s.rational(&row.max_jitter)?;
                let deadline = s.rational(&row.relative_deadline)?;
                let lower = s.rational(&row.work_lower)?;
                let upper = s.rational(&row.work_upper)?;
                let task = s.text(&row.task)?;
                let taskset = s.text(&row.task_set)?;
                s.record([
                    ("arrival", arrival),
                    ("interarrival", interarrival),
                    ("max_jitter", jitter),
                    ("max_jobs", Value::from(row.max_jobs)),
                    ("relative_deadline", deadline),
                    ("task", task),
                    ("taskset", taskset),
                    ("work_lower", lower),
                    ("work_upper", upper),
                ])
            })?);
        }
        let mut ceilings = s.array(self.clock_ceilings.len())?;
        for (index, row) in self.clock_ceilings.iter().enumerate() {
            ceilings.push(s.at_index("/clock_ceilings", index, |s| {
                let clock = s.text(&row.clock)?;
                let maximum = s.integer(&row.maximum)?;
                s.record([("clock", clock), ("maximum", maximum)])
            })?);
        }
        let mut diagonals = s.array(self.diagonal_constraints.len())?;
        for (index, row) in self.diagonal_constraints.iter().enumerate() {
            diagonals.push(s.at_index("/diagonal_constraints", index, |s| {
                let left = s.text(&row.left)?;
                let right = s.text(&row.right)?;
                let operator = s.text(row.operator.as_str())?;
                let constant = s.integer(&row.constant)?;
                s.record([
                    ("constant", constant),
                    ("left", left),
                    ("operator", operator),
                    ("right", right),
                ])
            })?);
        }
        let mut subtractions = s.array(self.subtractions.len())?;
        for (index, row) in self.subtractions.iter().enumerate() {
            subtractions.push(s.at_index("/subtractions", index, |s| {
                let clock = s.text(&row.clock)?;
                let constant = s.integer(&row.constant)?;
                let transition =
                    s.at_pointer("/transition", |s| s.encode_task_template(&row.transition))?;
                s.record([
                    ("clock", clock),
                    ("constant", constant),
                    ("transition", transition),
                ])
            })?);
        }
        let method = s.text(self.time_progress.method.as_str())?;
        let closure = s.text(self.time_progress.closure.as_str())?;
        let progress = s.record([
            ("method", method),
            ("closure", closure),
            ("live_job_cap", Value::from(self.time_progress.live_job_cap)),
        ])?;
        let abstraction = s.text(&self.abstraction)?;
        s.record([
            ("abstraction", abstraction),
            ("clock_ceilings", Value::Array(ceilings)),
            ("diagonal_constraints", Value::Array(diagonals)),
            ("queue_bounds", Value::Array(queues)),
            ("subtractions", Value::Array(subtractions)),
            ("time_progress", progress),
        ])
    }
}
