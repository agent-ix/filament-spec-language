// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255 accounting-boundary details across calls, temporal rules and state rules.

use super::*;
use crate::spine::{CallIncomplete, TemporalVerdict};
use filament_exact::{ChargePoint, Incomplete, Integer, LimitKind};

fn state_rule_report(work_units: u64) -> crate::spine::ClauseRunReport {
    let label = config_version_document_ref("accounting-boundary-snapshot");
    let snapshot = config_version_snapshot(&label, 1, Some(2));
    let selection = config_version_current_selection("ParentOrder", label, "child");
    let mut request = config_version_request(ClauseRunSelection::Clause(selection));
    request.accounting = default_accounting(work_units);
    request.snapshots.insert(
        fsl_semantics::model::observation::document_reference(&snapshot)
            .expect("snapshot has its own identity"),
        snapshot,
    );
    run_clause(request).expect("valid request returns a clause report")
}

fn item(report: &crate::spine::ClauseRunReport) -> serde_json::Value {
    let item = crate::OutcomeItem::from_clause_run(
        crate::RequestIndex::new(0),
        "Config::ConfigVersion::ParentOrder",
        report,
    )
    .expect("a state-rule report is an outcome item");
    serde_json::to_value(item).expect("outcome item serializes")
}

fn reached() -> Incomplete {
    Incomplete {
        limit_kind: LimitKind::WorkUnits,
        limit: 0,
        consumed: 0,
        next_charge: Integer::from(1_u64),
        charge_point: ChargePoint::FunctionCall,
    }
}

/// FR-255: a stopped state-rule item names the same counter, bound and
/// setting as execute and temporal results, while preserving its catalog cause.
#[trace("FR-255-AC-1", "FR-255-AC-2")]
#[test]
fn state_rule_accounting_cause_names_counter_bound_and_setting_like_call_and_temporal() {
    let state = state_rule_report(0);
    assert_eq!(
        state.disposition.category(),
        filament_semantic_value::catalog::Category::Incomplete
    );
    let state = item(&state);
    assert_eq!(state["cause"]["code"], "resource_exhausted");
    assert_eq!(state["cause"]["kind"], "resource_exhausted");

    let record = reached();
    let call =
        crate::OutcomeDocument::from_call(&crate::spine::CallOutcome::Incomplete(CallIncomplete {
            record: record.clone(),
            location: None,
        }));
    let call: serde_json::Value = serde_json::to_value(call).expect("call outcome serializes");

    let mut temporal = state_rule_report(1_000_000);
    temporal.disposition = ClauseDisposition::Temporal(TemporalVerdict::Incomplete(record));
    let temporal = item(&temporal);

    let call_limit = &call["result"]["limit"];
    let temporal_cause = &temporal["cause"];
    let state_cause = &state["cause"];
    for member in ["counter", "bound", "setting"] {
        assert_eq!(
            state_cause[member], call_limit[member],
            "state vs call: {member}"
        );
        assert_eq!(
            state_cause[member], temporal_cause[member],
            "state vs temporal: {member}"
        );
    }
    assert_eq!(state_cause["counter"], "1");
    assert_eq!(state_cause["bound"], "0");
    assert_eq!(state_cause["setting"], "accounting.work_units");
    assert_eq!(state_cause["field"], "work_units");
}

/// FR-255: a run that remains below its bound has no limit details.
#[trace("FR-255-AC-3")]
#[test]
fn state_rule_under_its_accounting_bound_has_no_limit_member() {
    let report = state_rule_report(1_000_000);
    assert_eq!(
        report.disposition.category(),
        filament_semantic_value::catalog::Category::Success
    );
    let item = item(&report);
    assert!(item.get("cause").is_none() || item["cause"].is_null());
    assert!(item.get("limit").is_none());
}
