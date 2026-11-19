// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1980 (FR-267-AC-20) and FSpec TC-1485 (FSpec FR-352-AC-16, AC-17):
//! each pending, finite-window and progress row FR-267 writes carries exactly
//! the members, category, basis and outcome of its FSpec FR-352 row, and a
//! temporal kind outside FR-352's sixteen refuses.

use serde_json::{json, Value};

use super::*;
use crate::spine::{
    read_native_run_result, write_clause_run_result, NativeRunResult, TemporalIndeterminateReason,
};

fn document(report: &ClauseRunReport) -> Value {
    let bytes = write_clause_run_result(report).expect("every temporal report is written");
    serde_json::from_slice(&bytes).expect("the document is JSON")
}

fn read(value: &Value) -> Result<(), String> {
    match read_native_run_result(&serde_json::to_vec(value).expect("JSON")) {
        Ok(NativeRunResult::Clause(_)) => Ok(()),
        Ok(_) => Err("not a clause-run record".to_owned()),
        Err(error) => Err(error.to_string()),
    }
}

/// The `Window` clause over stamps 0, 2, 5 with position 1's `holds` atom
/// valuation absent (FR-501-AC-4), its disposition replaced.
fn with_disposition(disposition: ClauseDisposition) -> ClauseRunReport {
    let mut request = timestamped(&[0, 2, 5], &[0, 2, 0], 5, Some(5));
    timestamped_trace_mut(&mut request)
        .window
        .missing_valuations
        .push(crate::spine::TimestampedMissingValuation {
            position: 1,
            atom: 1,
        });
    let mut report = report(request);
    report.basis = match &disposition {
        ClauseDisposition::Temporal(verdict) => verdict.basis(),
        _ => SettlementBasis::Unavailable,
    };
    report.disposition = disposition;
    report.counterexample = None;
    report
}

/// The written disposition's FSpec FR-352 row facts: `temporal`, `category`,
/// whether `truth` and `outcome` are present, and the record's `basis`.
fn row(value: &Value) -> (Value, Value, bool, bool, Value) {
    let disposition = &value["disposition"];
    (
        disposition["temporal"].clone(),
        disposition["category"].clone(),
        disposition.get("truth").is_some(),
        disposition.get("outcome").is_some(),
        value["basis"].clone(),
    )
}

/// FR-267-AC-20, FSpec FR-352-AC-16: the three pending rows carry category
/// `pending`, basis `unsettled`, no `truth` and no `outcome`.
#[trace("TC-1980", "FR-267-AC-20")]
#[trace("FSpec-TC-1485", "FSpec-FR-352-AC-10", "FSpec-FR-352-AC-11")]
#[trace("FSpec-TC-1485", "FSpec-FR-352-AC-16")]
#[test]
fn the_pending_rows_match_their_fspec_rows() {
    for (verdict, temporal) in [
        (TemporalVerdict::Pending, json!({"kind": "pending"})),
        (
            TemporalVerdict::PendingWindow { at: 0, deadline: 5 },
            json!({"kind": "pending-window", "position": 0, "deadline": 5}),
        ),
        (
            TemporalVerdict::PendingNextEvent { at: 3 },
            json!({"kind": "pending-next-event", "position": 3}),
        ),
    ] {
        let value = document(&with_disposition(ClauseDisposition::Temporal(verdict)));
        assert_eq!(
            row(&value),
            (temporal, json!("pending"), false, false, json!("unsettled"))
        );
        read(&value).expect("the row reads");
    }
}

/// FR-267-AC-20, FSpec FR-352-AC-16: `indeterminate`, `activation-incomplete`
/// and `progress-limit-reached` carry their FSpec FR-352 category and basis,
/// and the last two write no `outcome`.
#[trace("TC-1980", "FR-267-AC-20")]
#[trace("FSpec-TC-1485", "FSpec-FR-352-AC-16")]
#[test]
fn the_indeterminate_and_incomplete_rows_match_their_fspec_rows() {
    let indeterminate = document(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::Indeterminate {
            at: 0,
            reason: TemporalIndeterminateReason::IncompleteInput,
        },
    )));
    assert_eq!(
        row(&indeterminate),
        (
            json!({"kind": "indeterminate", "position": 0, "reason": {"kind": "incomplete-input"}}),
            json!("indeterminate"),
            false,
            false,
            json!("unavailable")
        )
    );
    read(&indeterminate).expect("the row reads");

    let guard = filament_semantic_value::location::ExpressionLocation::at(
        filament_semantic_value::location::ExpressionOwner::StateClause {
            clause: "Window".to_owned(),
            index: 0,
        },
        &[1, 0],
    );
    let activation = document(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::ActivationIncomplete { at: 1, guard },
    )));
    let (temporal, category, truth, outcome, basis) = row(&activation);
    assert_eq!(temporal["kind"], "activation-incomplete");
    assert_eq!(temporal["position"], 1);
    assert_eq!(
        (category, truth, outcome, basis),
        (json!("incomplete"), false, false, json!("unavailable"))
    );

    let limited = document(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::ProgressLimitReached(crate::spine::ProgressLimitReached {
            limit: "node count",
            value: 4,
            setting: "progress.covered_positions",
        }),
    )));
    assert_eq!(
        row(&limited),
        (
            json!({
                "kind": "progress-limit-reached",
                "limit": "node count",
                "value": 4,
                "setting": "progress.covered_positions",
            }),
            json!("incomplete"),
            false,
            false,
            json!("unavailable")
        )
    );
}

/// FR-267-AC-20, FSpec FR-352-AC-17: a `temporal` whose `kind` is outside
/// the sixteen FSpec FR-352 registers is refused naming the kind, and so is
/// category `inconclusive` on a `pending` row.
#[trace("TC-1980", "FR-267-AC-20")]
#[trace("FSpec-TC-1485", "FSpec-FR-352-AC-17")]
#[test]
#[ignore = "QSL-1610: an unregistered kind is refused naming `disposition.temporal.`, not `disposition.temporal.kind`"]
fn an_unregistered_temporal_kind_refuses() {
    let pending = document(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::Pending,
    )));
    for kind in ["pending-forever", "held"] {
        let mut edited = pending.clone();
        edited["disposition"]["temporal"]["kind"] = json!(kind);
        let error = read(&edited).expect_err("an unregistered kind refuses");
        assert!(error.contains("disposition.temporal.kind"), "`{error}`");
    }
    let mut inconclusive = pending.clone();
    inconclusive["disposition"]["category"] = json!("inconclusive");
    let error = read(&inconclusive).expect_err("a pending row is category pending");
    assert!(error.contains("category"), "`{error}`");
}
