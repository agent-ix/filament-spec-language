// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-812 and FR-814 lowering of invariant state clauses (FSL FR-1015,
//! FR-1016, FR-1017) over FR-756-AC-1's counter, and the refusal of a state
//! clause this build does not lower.

use crate::plan_export_surface::{counter_export, counter_request};
use crate::support::config_version_spine::{spine, Case};
use ix_trace_rs::trace;
use serde_json::{json, Value};

fn tla() -> Value {
    json!({"profile": "filament.output.tla2/v1", "obligations": ["Bounded"]})
}

fn lean() -> Value {
    json!({"profile": "filament.output.lean4/v1", "obligations": ["Bounded"]})
}

fn text(value: &Value) -> &str {
    value["export"]["target_text"]
        .as_str()
        .expect("target text")
}

fn assert_canonical(text: &str, marker: &str) {
    assert!(text.is_ascii(), "{text}");
    assert!(text.ends_with('\n') && !text.ends_with("\n\n"), "{text}");
    assert!(!text.lines().any(|line| line.ends_with(' ')), "{text}");
    let comments = text
        .lines()
        .filter(|line| line.starts_with(&format!("{marker} fsl: ")))
        .count();
    assert!(comments > 0, "{text}");
}

/// FR-1015-AC-5 and FSpec FR-812-AC-1/AC-7: the counter's `Bounded` over the
/// `Counter` population exports to TLA+ with its variable, `TypeOK`, an
/// `Init` stating the `initial` snapshot, the operation `inc`, `Next`, `Spec`
/// with `WF_vars(Next)` and the obligation as a theorem; the record is
/// `preserved` with no cause and claims the clause's regions.
#[test]
#[trace("TC-3002", "FR-1015-AC-5")]
#[trace("FSpec-TC-1971", "FSpec-FR-812-AC-1")]
#[trace("FSpec-TC-2031", "FSpec-FR-812-AC-7")]
#[trace("FSpec-TC-1973", "FSpec-FR-814-AC-3")]
fn the_counter_invariant_lowers_to_tla() {
    let (exit, value) = counter_export(tla());
    assert_eq!(exit, 0, "{value}");
    let module = text(&value);
    assert!(module.starts_with("---- MODULE M_"), "{module}");
    assert!(module.ends_with("====\n"), "{module}");
    assert_canonical(module, "\\*");
    for part in [
        "VARIABLES v_Counter",
        "TypeOK ==",
        "v_Counter = [k \\in {<<99>>} |-> CASE k = <<99>> -> [f_value |-> 0]]",
        "op_inc(p_self) ==",
        "Next ==",
        "Spec == Init /\\ [][Next]_vars /\\ WF_vars(Next)",
        "c_Bounded == \\A p_self \\in DOMAIN v_Counter",
        "THEOREM Th_Bounded == Spec => []c_Bounded",
    ] {
        assert!(module.contains(part), "{part}: {module}");
    }
    assert!(!module.contains("Cardinality"), "{module}");
    let record = &value["export"]["records"][0];
    assert_eq!(record["disposition"], "preserved", "{value}");
    assert_eq!(record["causes"], json!([]), "{value}");
    let regions = record["regions"].as_array().expect("regions");
    assert!(!regions.is_empty(), "{value}");
    for region in regions {
        let (start, end) = (region[0].as_u64().unwrap(), region[1].as_u64().unwrap());
        let claimed = &module[usize::try_from(start).unwrap()..usize::try_from(end).unwrap()];
        assert!(
            claimed.contains("Bounded"),
            "a region states the obligation: {claimed}"
        );
    }
    let names: Vec<&str> = value["export"]["name_map"]
        .as_array()
        .expect("name map")
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    assert_eq!(names, ["op_inc", "c_Bounded", "Th_Bounded"], "{value}");
}

/// FSpec FR-813-AC-5, FR-812-AC-1 and FR-814-AC-3: the Lean file states the
/// obligation as a proposition over a `Behaviour` with no theorem, proof
/// term or `sorry`, in ASCII.
#[test]
#[trace("TC-3002", "FR-1015-AC-5")]
#[trace("FSpec-TC-1972", "FSpec-FR-813-AC-5")]
#[trace("FSpec-TC-1971", "FSpec-FR-812-AC-1")]
#[trace("FSpec-TC-1973", "FSpec-FR-814-AC-3")]
fn the_counter_invariant_lowers_to_lean() {
    let (exit, value) = counter_export(lean());
    assert_eq!(exit, 0, "{value}");
    let file = text(&value);
    assert!(file.starts_with("namespace M_"), "{file}");
    assert_canonical(file, "--");
    for part in [
        "structure State where",
        "def WF (s : State)",
        "def Init (s : State)",
        "s.v_Counter = [([99], { f_value := 0 })]",
        "def op_inc (s : State)",
        "def Step (s : State)",
        "structure Behaviour where",
        "def c_Bounded (s : State)",
        "def Ob_Bounded : Prop :=",
    ] {
        assert!(file.contains(part), "{part}: {file}");
    }
    for word in ["theorem", "sorry", "∀", "∧", "≤"] {
        assert!(!file.contains(word), "{word}: {file}");
    }
    assert_eq!(
        value["export"]["records"][0]["disposition"], "preserved",
        "{value}"
    );
}

/// FSpec FR-814-AC-3: two exports of one request give byte-identical text.
#[test]
#[trace("FSpec-TC-1973", "FSpec-FR-814-AC-3")]
fn equal_requests_give_equal_text() {
    for member in [tla(), lean()] {
        let (_, first) = counter_export(member.clone());
        let (_, second) = counter_export(member);
        assert_eq!(text(&first), text(&second));
    }
}

/// Without `initial` a state model's `Init` is its well-typedness
/// (FSpec FR-812-AC-7).
#[test]
#[trace("FSpec-TC-2031", "FSpec-FR-812-AC-7")]
fn init_without_a_snapshot_is_the_well_typedness() {
    let directory = tempfile::tempdir().unwrap();
    let path = counter_request(directory.path(), tla());
    let mut request: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    request["request"]["export"]
        .as_object_mut()
        .unwrap()
        .remove("initial");
    std::fs::write(&path, request.to_string()).unwrap();
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290).expect("exports");
    assert_eq!(result.exit_code, 0);
    let value: Value = serde_json::from_str(&result.value.to_string()).unwrap();
    assert!(text(&value).contains("Init == TypeOK"), "{value}");
}

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:w53:{file}"),
        "document": "W53", "formal_revision": 1})
}

/// Exports `Low` over the spine model, whose `ConfigVersion` has fields this
/// build does not lower.
fn spine_low(profile: &str) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model Config = \"example/config-version\";\n\
         invariant Low using v on Config::ConfigVersion at current { self.versionNumber >= 0 }\n";
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0",
            "source": source("spine-model.semantic-ir.json")}],
        "program": {"source": source("unit.native")},
        "export": {"profile": profile, "obligations": ["Low"], "output": {"file": "m.out"}}}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let error =
        fsl_command::command::run(&path, &crate::negotiation::Fr290).expect_err("a mapper failure");
    assert!(!directory.path().join("m.out").exists());
    (error.category().exit_code(), error.value().unwrap())
}

/// A state clause over a type with a field this build does not lower stays a
/// mapper failure naming the clause and the type, exit 30, with no file, in
/// both targets.
#[test]
#[trace("TC-3002", "FR-1015-AC-5")]
fn an_unlowered_state_construct_is_a_mapper_failure() {
    for profile in ["filament.output.tla2/v1", "filament.output.lean4/v1"] {
        let (exit, value) = spine_low(profile);
        assert_eq!(exit, 30, "{value}");
        let value = value.to_string();
        assert!(value.contains("Low"), "{value}");
        assert!(value.contains("model/field"), "{value}");
    }
}

/// FR-865-AC-7: an incomplete export renders its bytes written and the
/// limit it reached.
#[test]
#[trace("TC-2996", "FR-865-AC-7")]
fn an_incomplete_export_renders_its_limit() {
    use filament_exact::Cancel;
    use fsl_inspect::{render_outcome, RenderForm, RenderSources};
    let directory = tempfile::tempdir().unwrap();
    let mut export = tla();
    export["limits"] = json!({"export.output_bytes": 1});
    let path = counter_request(directory.path(), export);
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290).expect("a result");
    assert_eq!(result.exit_code, 22);
    let rendered = render_outcome(
        &result.outcome.expect("an outcome document"),
        RenderForm::Text,
        &RenderSources::from_sources(Vec::new()),
        fsl_replay::RenderLimits::default(),
        &Cancel::new(),
    )
    .expect("the document renders");
    assert_eq!(
        rendered.value().as_str().lines().collect::<Vec<_>>(),
        [
            "export: incomplete (exit 22) at S4",
            "bytes written: 0",
            "limit reached: export.output_bytes 1"
        ]
    );
}
