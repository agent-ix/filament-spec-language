// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1015 and FSpec FR-811 stops that need no lowering: a representable
//! obligation this build does not lower is a mapper failure, never a record,
//! a package or a file; the `initial` member is checked.

use crate::plan_export_surface::{counter_request, outcome_json};
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn names(value: &Value, text: &str) {
    assert!(value.to_string().contains(text), "names {text}: {value}");
}

fn counter_member(floor: Option<&str>) -> Value {
    let mut member = json!({"profile": "filament.output.tla2/v1", "obligations": ["Bounded"],
        "output": {"file": "counter.tla"}});
    if let Some(floor) = floor {
        member["minimum_fidelity"] = json!(floor);
    }
    member
}

/// FSpec FR-811 "Decision" and FR-817 Exits: an invariant the target can
/// state, which this build does not lower, stops the export as a mapper
/// failure: internal-failure, exit 30, no package and no file, under every
/// floor.
#[test]
fn an_unlowered_invariant_is_a_mapper_failure() {
    for floor in [None, Some("any"), Some("preserved")] {
        let directory = tempfile::tempdir().unwrap();
        let path = counter_request(directory.path(), counter_member(floor));
        let error = fsl_command::command::run(&path, &crate::negotiation::Fr290)
            .expect_err("a mapper failure");
        assert_eq!(error.category().exit_code(), 30, "{floor:?}");
        let value = error.value().unwrap();
        names(&value, "runtime_invariant");
        names(&value, "Bounded");
        assert!(value.get("export").is_none(), "{value}");
        assert!(!directory.path().join("counter.tla").exists());
        // The program reached S4 before the mapper ran (FR-286).
        let document = outcome_json(&error.outcome_document(fsl_replay::Operation::Export));
        assert_eq!(document["last_stage"], "S4", "{document}");
        assert_eq!(value["stage"], "emit", "{value}");
    }
}

/// The floor is decided before the mapper meets a representable obligation:
/// a refused record fails the default floor, exit 21, ahead of the failure.
#[test]
#[trace("TC-3002", "FR-1015-AC-6")]
fn a_refused_record_fails_the_floor_before_a_mapper_failure() {
    let directory = tempfile::tempdir().unwrap();
    let mut member = counter_member(None);
    member["obligations"] = json!(["Nope", "Bounded"]);
    let path = counter_request(directory.path(), member);
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290).expect("refused");
    assert_eq!(result.exit_code, 21);
    assert!(!directory.path().join("counter.tla").exists());
}

/// The charges of FR-255 are made while the dependency closures are read,
/// before the floor and the text.
#[test]
#[trace("TC-2993", "FR-255-AC-22")]
fn the_closure_of_an_obligation_is_charged_node_by_node() {
    for (setting, bound) in [("export.nodes", 1), ("export.work_units", 1)] {
        let directory = tempfile::tempdir().unwrap();
        let mut member = counter_member(Some("any"));
        member["limits"] = json!({ setting: bound });
        let path = counter_request(directory.path(), member);
        let result = fsl_command::command::run(&path, &crate::negotiation::Fr290)
            .expect("an incomplete export is a result");
        assert_eq!(result.exit_code, 22, "{setting}");
        let value: Value = serde_json::from_str(&result.value.to_string()).unwrap();
        assert_eq!(
            value["export"]["cause"],
            json!({"type": "limit-reached",
                "limit": setting.strip_prefix("export.").unwrap(),
                "value": bound, "setting": setting}),
            "{value}"
        );
    }
}

/// FSpec FR-817 `initial` names a `filament.state.snapshot/v1` document; any
/// other document refuses at its pointer, exit 20.
#[test]
fn the_initial_member_must_name_a_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let path = counter_request(directory.path(), counter_member(Some("any")));
    std::fs::write(
        directory.path().join("counter-0.json"),
        json!({"format": "not-a-snapshot"}).to_string(),
    )
    .unwrap();
    let error = fsl_command::command::run(&path, &crate::negotiation::Fr290)
        .expect_err("a non-snapshot refuses");
    assert_eq!(error.category().exit_code(), 20);
    let value = error.value().unwrap();
    names(&value, "/request/export/initial");
    assert!(!directory.path().join("counter.tla").exists());
}
