// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-621 item reporting for claims with no admitted instance.

use fsl_replay::{
    Execution, InconclusiveCause, ResultLabel, RunStats, SettlementBasis, StateGraphMethod,
    StateGraphRecord, TerminalValue, Truth, EXPLICIT_STATE,
};
use ix_trace_rs::implements;

use super::{EngineReport, GraphReport};

pub(crate) use fsl_replay::combine_instance_records as decisive;

/// The report of an item with no instance to decide, which settles
/// inconclusive with `cause` and is never a fault (FR-621).
#[implements("FR-181-AC-11")]
pub(super) fn undecided(
    cause: InconclusiveCause,
    method: &StateGraphMethod,
    witness_samples: u64,
) -> EngineReport {
    let record = StateGraphRecord {
        value: TerminalValue::Inconclusive(cause),
        label: ResultLabel::Inconclusive,
        basis: SettlementBasis::Unsettled,
        execution: Execution::Completed,
        truth: Truth::Pending,
        method: EXPLICIT_STATE,
        max_depth: method.max_depth,
        limits: method.limits.used(),
        limit_reached: None,
        seed: method.seed,
        witness_samples,
        walks_stopped: 0,
        end: None,
        undefined: None,
    };
    EngineReport::StateGraph(Box::new(GraphReport {
        record,
        exploration: RunStats::default(),
        counterexample: None,
        evidence: None,
        certificate_document: None,
        checker: None,
    }))
}
