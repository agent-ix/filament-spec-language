// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-812 and FR-814 lowering of invariant state clauses (FSL FR-1015,
//! FR-1016, FR-1017) over FR-756-AC-1's counter, and the refusal of a state
//! clause this build does not lower.

use crate::plan_export_surface::outcome_json;
use crate::plan_export_surface::{counter_export, counter_request};
use crate::support::config_version_spine::{spine, Case};
use crate::support::refinement_cas::{self, Variant};
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
    assert_eq!(
        names,
        ["v_Counter", "op_inc", "c_Bounded", "Th_Bounded"],
        "{value}"
    );
    let entries = value["export"]["name_map"].as_array().unwrap();
    assert!(
        entries[1]["node"]
            .as_str()
            .unwrap()
            .contains("operation_anchor"),
        "op_inc reads back to the operation node: {value}"
    );
    let variables = module.find("VARIABLES").unwrap();
    assert!(
        module[..variables]
            .lines()
            .next_back()
            .is_some_and(|line| line.starts_with("\\* fsl: ")),
        "VARIABLES follows its fsl: comment: {module}"
    );
}

/// FSpec FR-813-AC-5, FR-812-AC-1 and FR-814-AC-3: the Lean file states the
/// obligation as a proposition over a `Behaviour` with no theorem, proof
/// term or `sorry`, in ASCII.
#[test]
#[trace("TC-3002", "FR-1015-AC-5")]
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

fn source(file: &str, identity: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": identity,
        "document": "W", "formal_revision": 1})
}

type Models<'a> = &'a [(&'a str, &'a str, Vec<u8>)];

/// Writes `models` (file, package identity, bytes) and `unit` into a fresh
/// directory with a request exporting `member`, and runs it.
fn run_export(
    models: Models<'_>,
    unit: &str,
    member: &Value,
) -> (
    tempfile::TempDir,
    Result<fsl_command::command::RunResult, Box<fsl_command::command::RunError>>,
) {
    let directory = tempfile::tempdir().unwrap();
    let mut entries = Vec::new();
    for (file, identity, bytes) in models {
        std::fs::write(directory.path().join(file), bytes).unwrap();
        entries.push(json!({"format": "semantic-ir/2.0.0", "source": source(file, identity)}));
    }
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": entries,
        "program": {"source": source("unit.native", "test:lowering:unit")},
        "export": member}});
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290);
    (directory, result)
}

fn exported(models: Models<'_>, unit: &str, member: &Value) -> (u8, Value) {
    let (_directory, result) = run_export(models, unit, member);
    let result = result.expect("an export result");
    (
        result.exit_code,
        serde_json::from_str(&result.value.to_string()).unwrap(),
    )
}

fn failed(models: Models<'_>, unit: &str, member: &Value) -> Value {
    let (_directory, result) = run_export(models, unit, member);
    let error = result.expect_err("a mapper failure");
    assert_eq!(error.category().exit_code(), 30, "{:?}", error.value());
    error.value().unwrap()
}

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n\
     profile m = \"filament.model.complete/v1\";\n";

fn spec_doc() -> Vec<u8> {
    let variant = Variant {
        tick: true,
        ..Variant::ADD
    };
    refinement_cas::documents(variant).remove(0)
}

fn spec_models() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    vec![("spec.json", refinement_cas::SPEC, spec_doc())]
}

fn spec_member(profile: &str) -> Value {
    json!({"profile": profile, "obligations": ["Low"]})
}

const SPEC_UNIT: &str = "model Spec = \"example/counter-spec\";\n\
     invariant Low using m on Spec::Counter at current { self.value <= 2 }\n";

fn spec_unit() -> String {
    format!("{HEADER}{SPEC_UNIT}")
}

/// FSpec FR-812-AC-1 and FR-814-AC-2/AC-5 over a model with an operation
/// that takes a parameter (`add(n: Bit)`, `Int[0, 1]`) and a receiver-only
/// frame (`tick`): TLA+ binds the parameter over its range and frames only
/// the receiver, with `UNCHANGED`-style per-field conjuncts.
#[test]
#[trace("FSpec-TC-1971", "FSpec-FR-812-AC-1")]
fn an_operation_with_a_parameter_and_a_receiver_frame_lowers_to_tla() {
    let (exit, value) = exported(
        &spec_models(),
        &spec_unit(),
        &spec_member("filament.output.tla2/v1"),
    );
    assert_eq!(exit, 0, "{value}");
    let module = text(&value);
    for part in [
        "op_add(p_self, p_n) ==",
        "\\/ \\E p_self \\in DOMAIN v_Counter, p_n \\in 0..1 : op_add(p_self, p_n)",
        "p_n >= 0",
        "p_n <= 1",
        "\\A k \\in DOMAIN v_Counter : k /= p_self => v_Counter'[k].f_value = v_Counter[k].f_value",
        "\\A k \\in DOMAIN v_Counter : v_Counter'[k].f_id = v_Counter[k].f_id",
        "Init == TypeOK",
    ] {
        assert!(module.contains(part), "{part}: {module}");
    }
    for name in ["op_inc", "op_tick", "op_add"] {
        let entry = value["export"]["name_map"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap_or_else(|| panic!("{name} in the name map: {value}"));
        assert!(entry["node"].is_string(), "{entry}");
    }
}

/// The Lean `op_` of a receiver-only frame keeps its later conjuncts out of
/// the receiver guard: each frame line is parenthesized, so `WF s'` and the
/// remaining lines are not under `Not (k = p_self) ->`.
#[test]
fn a_receiver_frame_does_not_swallow_the_later_lean_conjuncts() {
    let (exit, value) = exported(
        &spec_models(),
        &spec_unit(),
        &spec_member("filament.output.lean4/v1"),
    );
    assert_eq!(exit, 0, "{value}");
    let file = text(&value);
    let tick = file
        .split("\n\n")
        .find(|block| block.contains("def op_tick"))
        .expect("op_tick");
    let guarded = tick
        .lines()
        .find(|line| line.contains("Not (k = p_self) ->"))
        .expect("the receiver frame line");
    assert!(
        guarded.trim_start().starts_with("(forall k,") && guarded.trim_end().ends_with(") /\\"),
        "{guarded}"
    );
    assert!(tick.contains("(WF s')"), "{tick}");
    assert!(
        file.contains("def op_add (s : State) (s' : State) (p_self : List Nat) (p_n : Int)"),
        "{file}"
    );
    assert!(file.contains("exists p_self, exists p_n,"), "{file}");
    assert!(file.contains("p_n >= 0"), "{file}");
    assert!(!file.contains("Prod.fst"), "{file}");
}

/// Boolean fields lower as `BOOLEAN` and as `= true` in Lean, and `not` and
/// `or` over them lower as connectives.
#[test]
fn boolean_fields_and_connectives_lower_in_both_targets() {
    let models = vec![(
        "cas.json",
        refinement_cas::CAS,
        refinement_cas::documents(Variant::CAS).remove(1),
    )];
    let unit = format!(
        "{HEADER}model Impl = \"example/counter-cas\";\n\
         invariant Excl using m on Impl::Counter at current \
         {{ not (self.busyA and self.busyB) or self.value <= 2 }}\n"
    );
    let member = |profile: &str| json!({"profile": profile, "obligations": ["Excl"]});
    let (exit, value) = exported(&models, &unit, &member("filament.output.tla2/v1"));
    assert_eq!(exit, 0, "{value}");
    let module = text(&value);
    assert!(module.contains("f_busyA \\in BOOLEAN"), "{module}");
    assert!(
        module.contains("~(v_Counter[p_self].f_busyA /\\ v_Counter[p_self].f_busyB)"),
        "{module}"
    );
    assert!(module.contains(" \\/ "), "{module}");
    let (exit, value) = exported(&models, &unit, &member("filament.output.lean4/v1"));
    assert_eq!(exit, 0, "{value}");
    let file = text(&value);
    assert!(file.contains(".f_busyA = true)"), "{file}");
    assert!(file.contains("Not ("), "{file}");
    assert!(file.contains("f_busyA : Bool"), "{file}");
}

/// An `or` guards the definedness of its right operand by the negated left
/// operand (FR-006): the `narrow` range of `self.value = pre(self.value) + 1`
/// holds only where `pre(self.id) = 0` is false.
#[test]
fn an_or_guards_the_definedness_of_its_right_operand() {
    let unit = format!(
        "{HEADER}model Spec = \"example/counter-spec\";\n\
         pre CanInc using v on Spec::Counter::inc {{ self.value < 2 }}\n\
         post Inc using v on Spec::Counter::inc \
         {{ pre(self.id) = 0 or self.value = pre(self.value) + 1 }}\n\
         invariant Low using m on Spec::Counter at current {{ self.value <= 2 }}\n"
    );
    let (exit, value) = exported(
        &spec_models(),
        &unit,
        &spec_member("filament.output.tla2/v1"),
    );
    assert_eq!(exit, 0, "{value}");
    let module = text(&value);
    assert!(
        module.contains("~(v_Counter[p_self].f_id = 0) => ((0 <="),
        "{module}"
    );
}

/// Two exported object types keep their own variables: each operation
/// leaves the other's variable unchanged, and every variable reads back
/// through the name map.
#[test]
fn two_exported_object_types_frame_each_other() {
    let gauge = String::from_utf8(spec_doc())
        .unwrap()
        .replace("Counter", "Gauge")
        .replace(refinement_cas::SPEC, "example/gauge-spec")
        .into_bytes();
    let models = vec![
        ("spec.json", refinement_cas::SPEC, spec_doc()),
        ("gauge.json", "example/gauge-spec", gauge),
    ];
    let unit = format!(
        "{HEADER}model Spec = \"example/counter-spec\";\nmodel Other = \"example/gauge-spec\";\n\
         invariant Low using m on Spec::Counter at current {{ self.value <= 2 }}\n\
         invariant High using m on Other::Gauge at current {{ self.value <= 2 }}\n"
    );
    let member = |profile: &str| json!({"profile": profile, "obligations": ["Low", "High"]});
    let (exit, value) = exported(&models, &unit, &member("filament.output.tla2/v1"));
    assert_eq!(exit, 0, "{value}");
    let module = text(&value);
    assert!(module.contains("VARIABLES v_Counter, v_Gauge"), "{module}");
    assert!(module.contains("UNCHANGED v_Gauge"), "{module}");
    assert!(module.contains("UNCHANGED v_Counter"), "{module}");
    let names: Vec<&str> = value["export"]["name_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    assert!(
        names.contains(&"v_Gauge") && names.contains(&"v_Counter"),
        "{names:?}"
    );
    let (exit, value) = exported(&models, &unit, &member("filament.output.lean4/v1"));
    assert_eq!(exit, 0, "{value}");
    assert!(text(&value).contains("s'.v_Gauge = s.v_Gauge"), "{value}");
}

/// Population bounds (`bound`, FSpec FR-812-AC-6) are not lowered, so an
/// export over a bounded population is a mapper failure naming the bound
/// rather than a `preserved` record that drops it.
#[test]
fn a_bounded_population_is_a_mapper_failure() {
    for profile in ["filament.output.tla2/v1", "filament.output.lean4/v1"] {
        let directory = tempfile::tempdir().unwrap();
        let path = counter_request(
            directory.path(),
            json!({"profile": profile, "obligations": ["Bounded"]}),
        );
        let unit = std::fs::read_to_string(directory.path().join("unit.native")).unwrap();
        std::fs::write(
            directory.path().join("unit.native"),
            format!("{unit}bound B on Counter::counters [3]\n"),
        )
        .unwrap();
        let error = fsl_command::command::run(&path, &crate::negotiation::Fr290)
            .expect_err("a mapper failure");
        assert_eq!(error.category().exit_code(), 30, "{profile}");
        let value = error.value().unwrap().to_string();
        assert!(
            value.contains("bound") && value.contains("Bounded"),
            "{value}"
        );
    }
}

/// An operation carrying domain-package `requires` or `ensures` clauses is
/// a mapper failure, never a `preserved` record that omits them.
#[test]
fn an_operation_contract_in_the_model_is_a_mapper_failure() {
    let mut document: Value = serde_json::from_slice(&spec_doc()).unwrap();
    for node in document["types"].as_array_mut().unwrap() {
        for operation in node
            .get_mut("operations")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            let origin = operation["origin"].clone();
            operation["pre"] = json!([{"language": "ix:native", "text": "true", "origin": origin}]);
        }
    }
    let models = vec![(
        "spec.json",
        refinement_cas::SPEC,
        serde_json::to_vec(&document).unwrap(),
    )];
    let value = failed(
        &models,
        &spec_unit(),
        &spec_member("filament.output.tla2/v1"),
    );
    let text = value.to_string();
    assert!(
        text.contains("model/operation") && text.contains("requires"),
        "{text}"
    );
}

/// A type no obligation names never refuses the export, and a refusal names
/// the obligation whose construct failed.
#[test]
fn only_the_exported_types_are_read() {
    let mut document: Value = serde_json::from_slice(&spec_doc()).unwrap();
    let mut other = document["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["kind"]["name"] == "object_type")
        .unwrap()
        .clone();
    let rename = |text: &str| text.replace("/Counter", "/Unused");
    other["identity"] = json!(rename(other["identity"].as_str().unwrap()));
    other["displayName"] = json!(rename(other["displayName"].as_str().unwrap()));
    for field in other["fields"].as_array_mut().unwrap() {
        field["identity"] = json!(rename(field["identity"].as_str().unwrap()));
        field["nullable"] = json!(true);
    }
    other["operations"] = json!([]);
    document["types"].as_array_mut().unwrap().push(other);
    let models = vec![(
        "spec.json",
        refinement_cas::SPEC,
        serde_json::to_vec(&document).unwrap(),
    )];
    let (exit, value) = exported(
        &models,
        &spec_unit(),
        &spec_member("filament.output.tla2/v1"),
    );
    assert_eq!(exit, 0, "{value}");
}

/// FR-865-AC-7: an incomplete export renders its bytes written and the
/// limit it reached.
#[test]
#[trace("TC-2996", "FR-865-AC-7")]
fn an_incomplete_export_renders_its_limit() {
    let directory = tempfile::tempdir().unwrap();
    let mut export = tla();
    export["limits"] = json!({"export.output_bytes": 1});
    let path = counter_request(directory.path(), export);
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290).expect("a result");
    assert_eq!(result.exit_code, 22);
    assert_eq!(
        render(&result),
        [
            "export: incomplete (exit 22) at S4",
            "bytes written: 0",
            "limit reached: export.output_bytes 1"
        ]
    );
}

fn render(result: &fsl_command::command::RunResult) -> Vec<String> {
    use filament_exact::Cancel;
    use fsl_inspect::{render_outcome, RenderForm, RenderSources};
    let rendered = render_outcome(
        result.outcome.as_ref().expect("an outcome document"),
        RenderForm::Text,
        &RenderSources::from_sources(Vec::new()),
        fsl_replay::RenderLimits::default(),
        &Cancel::new(),
    )
    .expect("the document renders");
    rendered
        .value()
        .as_str()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// FR-865-AC-7: an export that names no output file renders `bytes written:
/// 0` and `output: none` as its last two lines.
#[test]
#[trace("TC-2996", "FR-865-AC-7")]
fn an_export_with_no_output_file_renders_none() {
    let directory = tempfile::tempdir().unwrap();
    let path = counter_request(directory.path(), tla());
    let result = fsl_command::command::run(&path, &crate::negotiation::Fr290).expect("a result");
    let lines = render(&result);
    assert_eq!(lines[0], "export: success (exit 0) at S4", "{lines:?}");
    assert_eq!(lines[lines.len() - 2], "bytes written: 0", "{lines:?}");
    assert_eq!(lines[lines.len() - 1], "output: none", "{lines:?}");
}

/// FR-1015 "Meter": writing the text charges one `export.work_units` per
/// output region on top of the node charges, so the least bound that lets
/// the counter export finish is the node charges plus one per definition.
#[test]
fn writing_the_regions_charges_work_units() {
    let least = |setting: &str| {
        (1_u64..400)
            .find(|bound| {
                let mut member = tla();
                member["limits"] = json!({ setting: bound });
                counter_export(member).0 == 0
            })
            .expect("a bound that admits the export")
    };
    let (exit, value) = counter_export(tla());
    assert_eq!(exit, 0, "{value}");
    // The `VARIABLES` comment annotates a declaration, not a definition.
    let definitions = text(&value).matches("\n\\* fsl: ").count() as u64 - 1;
    assert_eq!(
        least("export.work_units"),
        least("export.nodes") + definitions,
        "one work unit per written definition on top of the node charges"
    );
}

/// An invariant over a type with a field this build does not lower is a
/// mapper failure naming the construct and the clause, exit 30, with no
/// `export` member, no file, `last_stage` S4 and stage `emit`, under every
/// floor and in both targets (FSpec FR-811 "Decision", FR-817 exits).
#[test]
fn an_unlowered_state_construct_is_a_mapper_failure() {
    let unit = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"filament.model.complete/v1\";\n\
         model Config = \"example/config-version\";\n\
         invariant Low using v on Config::ConfigVersion at current { self.versionNumber >= 0 }\n";
    for profile in ["filament.output.tla2/v1", "filament.output.lean4/v1"] {
        for floor in [None, Some("any"), Some("preserved")] {
            let directory = tempfile::tempdir().unwrap();
            spine::write(directory.path(), Case::Healthy).unwrap();
            std::fs::write(directory.path().join("unit.native"), unit).unwrap();
            let mut member =
                json!({"profile": profile, "obligations": ["Low"], "output": {"file": "m.out"}});
            if let Some(floor) = floor {
                member["minimum_fidelity"] = json!(floor);
            }
            let request = json!({"format": "native-run/1", "request": {
                "models": [{"format": "semantic-ir/2.0.0",
                    "source": source("spine-model.semantic-ir.json", "test:w53:spine")}],
                "program": {"source": source("unit.native", "test:lowering:spine")},
                "export": member}});
            let path = directory.path().join("request.json");
            std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
            let error = fsl_command::command::run(&path, &crate::negotiation::Fr290)
                .expect_err("a mapper failure");
            assert_eq!(error.category().exit_code(), 30, "{profile} {floor:?}");
            let value = error.value().unwrap();
            let text = value.to_string();
            assert!(
                text.contains("Low") && text.contains("model/field"),
                "{text}"
            );
            assert!(value.get("export").is_none(), "{value}");
            assert_eq!(value["stage"], "emit", "{value}");
            assert!(!directory.path().join("m.out").exists());
            let document = outcome_json(&error.outcome_document(fsl_replay::Operation::Export));
            assert_eq!(document["last_stage"], "S4", "{document}");
        }
    }
}
