// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1015: the export request surface that runs today. Every obligation is
//! `unrepresented` in this build (no state clause or temporal formula is
//! lowered), so each case that assembles names `minimum_fidelity: any`.

use crate::plan_export_surface::{counter_export, counter_request};
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn names(value: &Value, text: &str) {
    assert!(value.to_string().contains(text), "names {text}: {value}");
}

fn member(floor: &str) -> Value {
    json!({"profile": "filament.output.tla2/v1", "obligations": ["Bounded"],
        "minimum_fidelity": floor})
}

#[test]
#[trace("TC-2865", "FR-1015-AC-1")]
#[trace("FSpec-TC-1970", "FSpec-FR-811-AC-2")]
fn below_the_default_floor_an_export_refuses_with_no_text() {
    let mut request = member("conditional");
    request.as_object_mut().unwrap().remove("minimum_fidelity");
    let (exit, value) = counter_export(request);
    assert_eq!(exit, 21, "{value}");
    assert_eq!(value["export"]["status"], "refused", "{value}");
    assert_eq!(value["export"]["refusal"]["code"], "unsupported_projection");
    names(&value["export"], "below-fidelity-floor");
    assert!(value["export"].get("target_text").is_none(), "{value}");
    let (exit, value) = counter_export(member("any"));
    assert_eq!(exit, 0, "{value}");
    assert_eq!(value["export"]["status"], "assembled", "{value}");
    assert_eq!(
        value["export"]["records"][0]["disposition"], "unrepresented",
        "{value}"
    );
    assert_eq!(value["export"]["records"][0]["regions"], json!([]));
}

#[test]
#[trace("TC-2865", "FR-1015-AC-2")]
fn an_export_stops_at_its_limits_and_refuses_a_doubled_limit() {
    let mut request = member("any");
    request["limits"] = json!({"export.output_bytes": 1});
    let (exit, value) = counter_export(request);
    assert_eq!(exit, 22, "{value}");
    assert_eq!(value["export"]["status"], "incomplete", "{value}");
    names(&value["export"], "export.output_bytes");
    let mut request = member("any");
    request["limits"] = json!({"export.work_units": 0});
    let (exit, value) = counter_export(request);
    assert_eq!(exit, 22, "{value}");
    names(&value["export"], "export.work_units");
    let mut request = member("any");
    request["limits"] = json!({"output_bytes": 1});
    let (exit, value) = counter_export(request);
    assert_eq!(exit, 20, "{value}");
    names(&value, "output_bytes");
    let directory = tempfile::tempdir().unwrap();
    let mut request = member("any");
    request["limits"] = json!({"export.nodes": 9});
    let path = counter_request(directory.path(), request);
    let limits = fsl_replay::spine::SpineLimits {
        export: fsl_replay::spine::ExportLimits {
            nodes: Some(5),
            ..Default::default()
        },
        ..Default::default()
    };
    let overrides = fsl_command::command::RequestOverrides {
        limits: Some(&limits),
        ..Default::default()
    };
    let error =
        fsl_command::command::run_with_overrides(&path, &overrides, &crate::negotiation::Fr290)
            .expect_err("a limit given twice refuses");
    assert_eq!(error.category().exit_code(), 20);
    names(&error.value().unwrap(), "export.nodes");
}

#[test]
#[trace("TC-2992", "FR-1015-AC-3")]
fn an_export_writes_its_file_once_and_never_over_an_existing_path() {
    let directory = tempfile::tempdir().unwrap();
    let mut request = member("any");
    request["output"] = json!({"file": "counter.tla"});
    let path = counter_request(directory.path(), request);
    let run = || {
        fsl_command::command::run(&path, &crate::negotiation::Fr290)
            .map(|result| (result.exit_code, result.value.to_string()))
    };
    let (exit, value) = run().expect("the export runs");
    assert_eq!(exit, 0, "{value}");
    let written = std::fs::read_to_string(directory.path().join("counter.tla")).unwrap();
    let value: Value = serde_json::from_str(&value).unwrap();
    assert_eq!(value["export"]["target_text"], written.as_str());
    assert_eq!(
        value["export"]["output"],
        json!({"path": "counter.tla", "bytes_written": written.len()})
    );
    let error = run().expect_err("an existing path refuses");
    assert_eq!(error.category().exit_code(), 20);
    names(&error.value().unwrap(), "counter.tla");
    assert_eq!(
        std::fs::read_to_string(directory.path().join("counter.tla")).unwrap(),
        written
    );
}

#[test]
#[trace("TC-2992", "FR-1015-AC-4")]
#[trace("TC-2994", "FR-286-AC-22")]
fn an_assembled_export_has_a_package_key_and_an_export_outcome() {
    let directory = tempfile::tempdir().unwrap();
    let path = counter_request(directory.path(), member("any"));
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290).unwrap();
    let value: Value = serde_json::from_str(&result.value.to_string()).unwrap();
    let package = &value["export"]["package"];
    assert_eq!(package["target_profile"], "filament.output.tla2/v1");
    assert_eq!(package["limits"]["export.nodes"], 100_000);
    assert_eq!(package["limits"]["export.output_bytes"], 16_777_216_u64);
    let outcome: Value =
        serde_json::from_slice(&result.outcome.expect("an outcome").to_bytes().unwrap()).unwrap();
    assert_eq!(outcome["operation"], "export", "{outcome}");
    assert_eq!(outcome["result"]["kind"], "exported", "{outcome}");
    assert_eq!(outcome["result"]["export"], value["export"], "{outcome}");
    assert_eq!(outcome["last_stage"], "S4", "{outcome}");
}

#[test]
#[trace("TC-2993", "FR-255-AC-22")]
fn the_export_settings_exist() {
    for name in ["export.output_bytes", "export.nodes", "export.work_units"] {
        assert!(fsl_foundation::Setting::from_name(name).is_some(), "{name}");
    }
}
