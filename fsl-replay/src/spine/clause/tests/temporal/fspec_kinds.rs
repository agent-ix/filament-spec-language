// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1980 (FR-267-AC-20) and FSpec TC-1485 (FSpec FR-352-AC-16, AC-17):
//! each pending, finite-window and progress row FR-267 writes carries exactly
//! the members, category, basis and outcome of its FSpec FR-352 row, and a
//! temporal kind outside FR-352's sixteen refuses.

use serde_json::{json, Value};

use super::*;
use crate::spine::{
    read_native_run_result, write_clause_run_result, NativeRunReadError, NativeRunResult,
    TemporalIndeterminateReason,
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
    read(&activation).expect("the activation row reads");

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
                "value": "4",
                "setting": "progress.covered_positions",
            }),
            json!("incomplete"),
            false,
            false,
            json!("unavailable")
        )
    );
    read(&limited).expect("the decimal-string progress limit reads");
}

/// FR-267-AC-20, FSpec FR-352-AC-17: a `temporal` whose `kind` is outside
/// the sixteen FSpec FR-352 registers is refused naming the kind, and so is
/// category `inconclusive` on a `pending` row.
#[trace("TC-1980", "FR-267-AC-20")]
#[trace("FSpec-TC-1485", "FSpec-FR-352-AC-17")]
#[test]
fn an_unregistered_temporal_kind_refuses() {
    let pending = document(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::Pending,
    )));
    for kind in ["pending-forever", "held"] {
        let mut edited = pending.clone();
        edited["disposition"]["temporal"]["kind"] = json!(kind);
        refuses_at(&edited, "disposition.temporal.kind");
    }
    let mut inconclusive = pending.clone();
    inconclusive["disposition"]["category"] = json!("inconclusive");
    refuses_at(&inconclusive, "disposition.category");
}

fn refuses_at(value: &Value, expected: &str) {
    let error = read_native_run_result(&serde_json::to_vec(value).expect("JSON"))
        .err()
        .expect("the malformed row refuses");
    match error {
        NativeRunReadError::Invalid { member, .. } => assert_eq!(member, expected),
        NativeRunReadError::MalformedMember { member, .. } => assert_eq!(member, expected),
        other => panic!("expected a located refusal at {expected}, got {other}"),
    }
}

/// Each finite-window/progress verdict restores its exact typed value and
/// rejects a category, basis, truth or outcome that belongs to another row.
#[trace(
    "FSpec-TC-1485",
    "FSpec-FR-352-AC-11",
    "FSpec-FR-352-AC-16",
    "FSpec-FR-352-AC-17"
)]
#[test]
fn temporal_rows_restore_verdicts_and_refuse_mismatched_members() {
    let guard = filament_semantic_value::location::ExpressionLocation::at(
        filament_semantic_value::location::ExpressionOwner::StateClause {
            clause: "Window".to_owned(),
            index: 0,
        },
        &[1, 0],
    );
    let mut verdicts = vec![
        TemporalVerdict::Pending,
        TemporalVerdict::PendingWindow { at: 0, deadline: 5 },
        TemporalVerdict::PendingNextEvent { at: 3 },
        TemporalVerdict::ActivationIncomplete { at: 1, guard },
        TemporalVerdict::ProgressLimitReached(crate::spine::ProgressLimitReached {
            limit: "node count",
            value: 4,
            setting: "progress.covered_positions",
        }),
    ];
    for reason in [
        TemporalIndeterminateReason::MissingValuation {
            position: 0,
            atom: "holds(c.value > 0)".to_owned(),
            instance: Some("c1".to_owned()),
            also_held_by: vec!["c2".to_owned()],
        },
        TemporalIndeterminateReason::ConflictingValuation {
            position: 0,
            atom: "holds(c.value > 0)".to_owned(),
        },
        TemporalIndeterminateReason::IncompleteInput,
        TemporalIndeterminateReason::MissingHistory,
        TemporalIndeterminateReason::StraddlesBound,
    ] {
        verdicts.push(TemporalVerdict::Indeterminate { at: 0, reason });
    }
    for verdict in verdicts {
        let value = document(&with_disposition(ClauseDisposition::Temporal(
            verdict.clone(),
        )));
        let NativeRunResult::Clause(record) =
            read_native_run_result(&serde_json::to_vec(&value).expect("JSON"))
                .expect("the specified row reads")
        else {
            panic!("a clause record");
        };
        assert_eq!(
            record.temporal().expect("temporal row").verdict(),
            Some(verdict)
        );
        for (member, wrong, path) in [
            ("category", json!("success"), "disposition.category"),
            ("truth", json!(false), "disposition.truth"),
            (
                "outcome",
                json!({"kind": "incomplete", "limit": "node count"}),
                "disposition.outcome",
            ),
        ] {
            let mut edited = value.clone();
            edited["disposition"][member] = wrong;
            refuses_at(&edited, path);
        }
        let mut edited = value.clone();
        edited["basis"] = if value["basis"] == "unsettled" {
            json!("unavailable")
        } else {
            json!("unsettled")
        };
        refuses_at(&edited, "basis");
        let members = value["disposition"]["temporal"].as_object().expect("row");
        for member in members.keys().filter(|member| member.as_str() != "kind") {
            let mut edited = value.clone();
            edited["disposition"]["temporal"][member] = Value::Null;
            refuses_at(&edited, &format!("disposition.temporal.{member}"));
            let mut edited = value.clone();
            edited["disposition"]["temporal"]
                .as_object_mut()
                .expect("row")
                .remove(member);
            let error = read(&edited).expect_err("an assigned member is required");
            assert!(error.contains(member), "{error}");
        }
        let mut edited = value.clone();
        edited["disposition"]["temporal"]["extra"] = json!(true);
        let error = read(&edited).expect_err("the row is closed");
        assert!(error.contains("extra"), "{error}");
        if value["disposition"]["temporal"]["reason"]["kind"] == "missing-valuation" {
            let mut edited = value.clone();
            edited["disposition"]["temporal"]["reason"]
                .as_object_mut()
                .expect("reason")
                .remove("also_held_by");
            refuses_at(&edited, "disposition.temporal.reason.also_held_by");
        }
        if value["disposition"]["temporal"]["kind"] == "progress-limit-reached" {
            let mut edited = value.clone();
            edited["disposition"]["temporal"]["setting"] = json!("progress.positions");
            refuses_at(&edited, "disposition.temporal.limit");
        }
    }
}

/// Limit values use canonical decimal strings, including the full u64 range.
#[trace("FSpec-TC-1485", "FSpec-FR-352-AC-22")]
#[test]
fn limit_values_read_decimal_strings_and_refuse_other_encodings() {
    for verdict in [
        TemporalVerdict::WindowLimitReached {
            limit: "max_window_values",
            value: 2,
            setting: "monitor.max_window_values",
        },
        TemporalVerdict::ProgressLimitReached(crate::spine::ProgressLimitReached {
            limit: "node count",
            value: 4,
            setting: "progress.covered_positions",
        }),
    ] {
        let mut value = document(&with_disposition(ClauseDisposition::Temporal(verdict)));
        for count in [0, 2, 4, u64::MAX] {
            value["disposition"]["temporal"]["value"] = json!(count.to_string());
            let NativeRunResult::Clause(record) =
                read_native_run_result(&serde_json::to_vec(&value).expect("JSON"))
                    .expect("canonical decimal reads")
            else {
                panic!("clause record");
            };
            let restored = record
                .temporal()
                .expect("temporal row")
                .verdict()
                .expect("verdict");
            let restored_value = match restored {
                TemporalVerdict::WindowLimitReached { value, .. } => value,
                TemporalVerdict::ProgressLimitReached(limit) => limit.value,
                other => panic!("a limit verdict, got {other:?}"),
            };
            assert_eq!(restored_value, count);
        }
        for wrong in [
            json!(2),
            json!(4),
            json!("02"),
            json!("04"),
            json!(""),
            json!("-1"),
            json!("+4"),
            json!(" 4"),
            json!("4.0"),
            json!("18446744073709551616"),
            Value::Null,
        ] {
            value["disposition"]["temporal"]["value"] = wrong;
            refuses_at(&value, "disposition.temporal.value");
        }
    }
}

/// A progress contradiction retains its refusal and fields; changing either
/// the category or the refusal's code, cause or fields is invalid.
#[trace("FSpec-TC-1485", "FSpec-FR-352-AC-16", "FSpec-FR-352-AC-17")]
#[test]
fn progress_contradictions_read_exact_refusals() {
    use crate::spine::{
        ContradictedDimension, ProgressContradiction, ProgressRefusal, ProgressValue,
    };

    let contradiction = ProgressContradiction {
        dimension: ContradictedDimension::Watermark,
        clock: "event-time".to_owned(),
        retained: ProgressValue::Watermark(100),
        asserted: ProgressValue::Watermark(60),
    };
    let value = document(&with_disposition(ClauseDisposition::ProgressContradiction(
        Box::new(ProgressRefusal::from(contradiction.clone())),
    )));
    assert_eq!(value["disposition"]["category"], "refusal");
    assert_eq!(value["basis"], "unavailable");
    assert!(value["disposition"].get("truth").is_none());
    assert_eq!(
        value["disposition"]["outcome"]["code"],
        "invalid_runtime_input"
    );
    assert_eq!(value["disposition"]["outcome"]["cause"], "invalid-value");
    let NativeRunResult::Clause(record) =
        read_native_run_result(&serde_json::to_vec(&value).expect("JSON"))
            .expect("the contradiction reads")
    else {
        panic!("a clause record");
    };
    assert_eq!(
        record.temporal().expect("temporal row").contradiction(),
        Some(contradiction)
    );
    for category in ["pending", "indeterminate", "incomplete", "violation"] {
        let mut edited = value.clone();
        edited["disposition"]["category"] = json!(category);
        refuses_at(&edited, "disposition.category");
    }
    for (member, wrong) in [
        ("code", json!("unsupported_projection")),
        ("cause", json!("missing-fairness-premise")),
        ("fields", json!({"asserted": "61"})),
    ] {
        let mut edited = value.clone();
        edited["disposition"]["outcome"][member] = wrong;
        refuses_at(&edited, "disposition.outcome");
    }
}

#[trace("FSpec-TC-1485", "FSpec-FR-352-AC-17")]
#[test]
fn incomplete_and_violated_rows_refuse_temporal_category_mismatches() {
    let mut incomplete = document(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::Pending,
    )));
    incomplete["disposition"] = json!({
        "stage": "evaluate", "category": "pending", "temporal": {"kind": "incomplete"},
        "outcome": {"kind": "incomplete", "limit": "node count"},
    });
    incomplete["basis"] = json!("unavailable");
    refuses_at(&incomplete, "disposition.category");

    let mut violated = document(&report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1])));
    assert_eq!(violated["disposition"]["temporal"]["kind"], "violated");
    violated["disposition"]["category"] = json!("indeterminate");
    refuses_at(&violated, "disposition.category");
}

/// The sixteenth registered kind is readable before QSL-1612 adds its writer.
#[trace("FSpec-TC-1485", "FSpec-FR-352-AC-10", "FSpec-FR-352-AC-17")]
#[test]
fn a_condition_refusal_reads_its_position_and_catalog_outcome() {
    let mut value = document(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::Pending,
    )));
    value["basis"] = json!("unavailable");
    value["disposition"] = json!({
        "stage": "evaluate", "category": "refusal",
        "temporal": {"kind": "condition-refused", "position": 2},
        "outcome": {
            "kind": "refused", "code": "invalid_runtime_input", "cause": "invalid-value",
            "fields": {"reason": "condition-input"},
            "locus": {"file": "window.native", "span": {
                "start": {"byte": 0, "line": 1, "column": 1},
                "end": {"byte": 1, "line": 1, "column": 2},
            }},
        },
    });
    let NativeRunResult::Clause(record) =
        read_native_run_result(&serde_json::to_vec(&value).expect("JSON"))
            .expect("the registered condition-refused row reads")
    else {
        panic!("a clause record");
    };
    assert!(matches!(
        record.temporal(),
        Some(crate::spine::TemporalRecord::ConditionRefused { position: 2 })
    ));
    assert_eq!(record.temporal().expect("row").kind(), "condition-refused");
    assert_eq!(record.temporal().expect("row").verdict(), None);
    assert_eq!(record.code(), Some("invalid_runtime_input"));
    assert_eq!(record.cause(), Some("invalid-value"));
    assert_eq!(
        record
            .fields()
            .expect("refusal fields")
            .get("reason")
            .map(String::as_str),
        Some("condition-input")
    );
    let locus = record.outcome_locus().expect("condition source locus");
    assert_eq!(locus.file, "window.native");
    assert_eq!((locus.span.start.byte, locus.span.end.byte), (0, 1));
    for (member, wrong, path) in [
        ("category", json!("unsupported"), "disposition.category"),
        ("truth", json!(false), "disposition.truth"),
        (
            "outcome",
            json!({"kind": "incomplete", "limit": "work_units"}),
            "disposition.outcome",
        ),
    ] {
        let mut edited = value.clone();
        edited["disposition"][member] = wrong;
        refuses_at(&edited, path);
    }
    let mut edited = value.clone();
    edited["disposition"]["temporal"]["position"] = Value::Null;
    refuses_at(&edited, "disposition.temporal.position");
    let mut edited = value.clone();
    edited["disposition"]["temporal"]["extra"] = json!(true);
    assert!(read(&edited).expect_err("closed row").contains("extra"));
    for (code, cause, category) in [
        (
            "unsupported_projection",
            "missing-fairness-premise",
            "unsupported",
        ),
        ("stage_limit_exceeded", "source-bytes", "incomplete"),
    ] {
        let mut edited = value.clone();
        edited["disposition"]["outcome"]["code"] = json!(code);
        edited["disposition"]["outcome"]["cause"] = json!(cause);
        edited["disposition"]["category"] = json!(category);
        read(&edited).expect("the refusal code's category reads");
    }
    let mut edited = value.clone();
    edited["disposition"]["outcome"]["code"] = json!("not-a-code");
    refuses_at(&edited, "disposition.outcome.code");
}
