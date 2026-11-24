// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315's caller-selected bounds on temporal SMT query encoding and runs.

use fsl_foundation::{Setting, SettingLimits};
use std::vec::Vec;

/// Size limits for one temporal SMT query encoding (FR-315).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SmtEncodingLimits {
    /// The most declared state and monitor constants.
    pub max_constants: u64,
    /// The most applications in one transition.
    pub max_applications: u64,
    /// The most atoms in a monitor letter.
    pub max_monitor_atoms: u64,
    /// The most reachable monitor states.
    pub max_monitor_states: u64,
    /// The most candidates expanded for one application.
    pub max_candidates: u64,
    /// The most bytes in a printed query.
    pub max_script_bytes: u64,
}

impl Default for SmtEncodingLimits {
    fn default() -> Self {
        Self {
            max_constants: 1 << 20,
            max_applications: 1 << 16,
            max_monitor_atoms: 8,
            max_monitor_states: 64,
            max_candidates: 1 << 16,
            max_script_bytes: 1 << 28,
        }
    }
}

/// Caller-selected bounds on one SMT solver run (FR-315).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SmtRunLimits {
    /// Maximum solver steps, in work units.
    pub max_solver_steps: u64,
    /// Maximum solver wall-clock time, in whole seconds.
    pub max_solver_time: u64,
}

impl Default for SmtRunLimits {
    fn default() -> Self {
        Self {
            max_solver_steps: 100_000_000,
            max_solver_time: 60,
        }
    }
}

impl SettingLimits for SmtRunLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![
            (Setting::SmtMaxSolverSteps, self.max_solver_steps),
            (Setting::SmtMaxSolverTime, self.max_solver_time),
        ]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        match setting {
            Setting::SmtMaxSolverSteps => self.max_solver_steps = bound,
            Setting::SmtMaxSolverTime => self.max_solver_time = bound,
            _ => return false,
        }
        true
    }
}
