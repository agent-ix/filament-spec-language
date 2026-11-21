// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1015 and FSpec FR-811 stops that need no lowering: a representable
//! obligation this build does not lower is the typed `unimplemented_capability`
//! refusal (exit 21), never a record,
//! a package or a file; the `initial` member is checked.

use crate::plan_export_surface::{braking_member, braking_request, counter_request, outcome_json};
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

fn observer_member(file: Option<&str>) -> Value {
    let mut member = braking_member();
    member["observer"] = json!("tlc");
    if let Some(file) = file {
        member["output"] = json!({"file": file});
    }
    member
}

/// An unsupported observer names the requested output file with no bytes
/// written, and still raises its refusal ahead of any compile.
#[test]
fn an_unsupported_observer_names_its_output_with_no_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let path = braking_request(directory.path(), observer_member(Some("m.tla")));
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290).expect("a result");
    assert_eq!(result.exit_code, 21);
    let value: Value = serde_json::from_str(&result.value.to_string()).unwrap();
    assert_eq!(
        value["export"]["output"],
        json!({"path": "m.tla", "bytes_written": 0}),
        "{value}"
    );
    assert!(!directory.path().join("m.tla").exists());
}

/// The output-path admission refusals win over an unsupported observer:
/// an existing path, and a member plus an override, are exit 20.
#[test]
fn output_admission_refusals_precede_the_observer_refusal() {
    let directory = tempfile::tempdir().unwrap();
    let path = braking_request(directory.path(), observer_member(Some("m.tla")));
    std::fs::write(directory.path().join("m.tla"), b"old").unwrap();
    let error = fsl_command::command::run(&path, &crate::negotiation::Fr290)
        .expect_err("an existing path refuses");
    assert_eq!(error.category().exit_code(), 20);
    names(&error.value().unwrap(), "invalid-request");
    assert_eq!(
        std::fs::read(directory.path().join("m.tla")).unwrap(),
        b"old"
    );
    let document = outcome_json(&error.outcome_document(fsl_replay::Operation::Export));
    assert_eq!(document["last_stage"], Value::Null, "{document}");

    let fresh = tempfile::tempdir().unwrap();
    let path = braking_request(fresh.path(), observer_member(Some("m.tla")));
    let other = fresh.path().join("other.tla");
    let overrides = fsl_command::command::RequestOverrides {
        output: Some(&other),
        ..Default::default()
    };
    let error =
        fsl_command::command::run_with_overrides(&path, &overrides, &crate::negotiation::Fr290)
            .expect_err("a doubled output refuses");
    assert_eq!(error.category().exit_code(), 20);
    names(&error.value().unwrap(), "invalid-request");
    assert!(!other.exists());
}

/// FR-286: a file that cannot be written after a successful export is an
/// output failure (exit 30) whose `last_stage` is the stage the export
/// reached.
#[test]
#[trace("TC-3029", "FR-286-AC-23")]
fn a_file_that_cannot_be_written_is_an_output_failure_after_s4() {
    let directory = tempfile::tempdir().unwrap();
    let mut member = braking_member();
    member["output"] = json!({"file": "missing-directory/m.tla"});
    let path = braking_request(directory.path(), member);
    let error = fsl_command::command::run(&path, &crate::negotiation::Fr290)
        .expect_err("an unwritable path fails");
    assert_eq!(error.category().exit_code(), 30);
    let value = error.value().unwrap();
    assert_eq!(value["code"], "output-failure", "{value}");
    assert_eq!(value["cause"], "output-failure", "{value}");
    assert_eq!(value["stage"], "emit", "{value}");
    let document = outcome_json(&error.outcome_document(fsl_replay::Operation::Export));
    assert_eq!(document["last_stage"], "S4", "{document}");
    assert_eq!(document["operation"], "export", "{document}");
    assert_eq!(document["category"], "internal-failure", "{document}");
    assert_eq!(document["result"], Value::Null, "{document}");
    let diagnostics = document["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1, "{document}");
    assert_eq!(diagnostics[0]["code"], "output-failure", "{document}");
    assert_eq!(diagnostics[0]["cause"], "output-failure", "{document}");
    assert_eq!(diagnostics[0]["details"], Value::Null, "{document}");
    assert!(!directory.path().join("missing-directory").exists());
}
