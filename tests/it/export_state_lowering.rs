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

#[test]
#[trace("FR-1039-AC-1", "FSpec-FR-812-AC-7")]
fn selected_package_export_uses_its_models_to_lower_object_state() {
    use crate::plan_export_surface::{counter_model, COUNTER_UNIT};

    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("counter.json"), counter_model()).unwrap();
    std::fs::write(directory.path().join("unit.native"), COUNTER_UNIT).unwrap();
    let initial = json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "example/counter-third/start"},
        "observation": "current",
        "model": {"identity": "example/counter-third"},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": "ix://example/counter-third/counters", "complete": true,
            "objects": [{"key": "c", "type": "ix://example/counter-third/Counter",
                "fields": {"value": {"integer": "0"}}}]}]
    });
    std::fs::write(directory.path().join("counter-0.json"), initial.to_string()).unwrap();
    let model = json!({"format": "semantic-ir/2.0.0", "source": {
        "file": "counter.json", "authority": "agent-ix",
        "identity": "example/counter-third", "document": "Counter", "formal_revision": 1
    }});
    let compile = json!({"format": "native-compile/1", "request": {
        "models": [model],
        "program": {"source": {"file": "unit.native", "authority": "agent-ix",
            "identity": "test:w21:unit.native", "document": "W21", "formal_revision": 1}},
        "output": {"package": "package.json"},
        "handoff": {"directory": "handoff", "package_name": "test:w21:unit.native",
            "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}
    }});
    let compile_path = directory.path().join("compile.json");
    std::fs::write(&compile_path, compile.to_string()).unwrap();
    fsl_command::command::compile(&compile_path).expect("the checked package compiles");

    let request = json!({"format": "native-run/1", "request": {
        "models": [model],
        "package": {"file": "package.json", "equality": "handoff/equality.json",
            "source_package": {
            "identity": "test:w21:unit.native", "version": "1"
        }},
        "export": {"profile": "filament.output.tla2/v1",
            "obligations": ["Bounded"], "minimum_fidelity": "any",
            "initial": "counter-0.json"}
    }});
    let request_path = directory.path().join("request.json");
    std::fs::write(&request_path, request.to_string()).unwrap();
    let result = fsl_command::command::run(&request_path, &crate::negotiation::Fr290)
        .expect("the checked package exports");
    let value = result.value.as_value();
    assert_eq!(result.exit_code, 0, "{value}");
    let module = text(value);
    assert!(module.contains("VARIABLES v_Counter"), "{module}");
    assert!(module.contains("op_Counter_inc(r_self) =="), "{module}");
    assert!(module.contains("Init ==\n  /\\ v_Counter ="), "{module}");

    let (source_exit, source_value) = crate::plan_export_surface::counter_export(json!({
        "profile": "filament.output.tla2/v1",
        "obligations": ["Bounded"],
        "minimum_fidelity": "any"
    }));
    assert_eq!(source_exit, 0, "{source_value}");
    let selected_text = value["export"]["target_text"].as_str().unwrap();
    let source_text = source_value["export"]["target_text"].as_str().unwrap();
    let first_difference = selected_text
        .bytes()
        .zip(source_text.bytes())
        .take_while(|(selected, source)| selected == source)
        .count();
    assert_eq!(
        selected_text,
        source_text,
        "target text first differs at byte {first_difference}; selected={:?}; source={:?}",
        &selected_text[first_difference.saturating_sub(40)..][..selected_text
            .len()
            .saturating_sub(first_difference.saturating_sub(40))
            .min(100)],
        &source_text[first_difference.saturating_sub(40)..][..source_text
            .len()
            .saturating_sub(first_difference.saturating_sub(40))
            .min(100)]
    );
    assert_eq!(
        value["export"]["records"],
        source_value["export"]["records"]
    );
    assert_eq!(
        value["export"]["target_text"],
        source_value["export"]["target_text"]
    );
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
        "v_Counter = [k \\in {<<99>>} |-> CASE k = <<99>> -> [fld_value |-> 0]]",
        "op_Counter_inc(r_self) ==",
        "Next ==",
        "Spec == Init /\\ [][Next]_vars /\\ WF_vars(Next)",
        "c_Bounded == \\A r_self \\in DOMAIN v_Counter",
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
        [
            "Th_Bounded",
            "c_Bounded",
            "fld_value",
            "op_Counter_inc",
            "v_Counter",
        ],
        "{value}"
    );
    let entries = value["export"]["name_map"].as_array().unwrap();
    assert!(
        entries[3]["node"]
            .as_str()
            .unwrap()
            .contains("operation_anchor"),
        "the qualified operation reads back to the operation node: {value}"
    );
    assert_eq!(entries[2]["binding"], entries[2]["node"], "{value}");
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
        "s.v_Counter = [([99], { fld_value := 0 })]",
        "def op_Counter_inc (s : State)",
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

fn refused(models: Models<'_>, unit: &str, member: &Value) -> Value {
    let (exit, value) = exported(models, unit, member);
    assert_eq!(exit, 21, "{value}");
    refusal_of(&value)
}

/// The `refusal` of an `unimplemented_capability` export document, which
/// names the target and the construct the build does not lower
/// (FSpec FR-817-AC-16).
fn refusal_of(value: &Value) -> Value {
    let refusal = value["export"]["refusal"].clone();
    assert_eq!(refusal["code"], "unimplemented_capability", "{value}");
    assert_eq!(refusal["cause"], "unimplemented-meaning", "{value}");
    assert_eq!(value["export"]["status"], "refused", "{value}");
    refusal
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

fn enum_spec_doc() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&spec_doc()).expect("the spec model is json");
    let tier = "ix://example/counter-spec/Tier";
    document["constructs"].as_array_mut().unwrap().push(json!({
        "kind": {"module": "example/counter-spec", "name": "variant_type"},
        "moduleVersion": "1.0.0",
        "construct": {"identity": "none", "shape": "enumeration",
            "members": {"variants": "required"}, "meaning": "filament.meaning.model.variant-type/v1"}
    }));
    let case = |name: &str, line: u64| {
        json!({"identity": format!("ix://example/counter-spec/variant/Tier-{name}"), "name": name,
            "origin": {"source": {"sourceIdentity": "ix://example/counter-spec/spec",
                "path": "spec.fspec", "startLine": line, "startColumn": 3}}})
    };
    document["types"]
        .as_array_mut()
        .expect("model types")
        .push(json!({
            "identity": tier,
            "displayName": "Tier",
            "kind": {"module": "example/counter-spec", "name": "variant_type"},
            "roles": [], "origin": {"generated": {"generatorIdentity": tier,
                "generatorVersion": "1.0.0", "inputIdentities": [tier]}},
            "constraints": [], "extensions": [], "unknownPolicy": "reject",
            "variants": [case("Gold", 1), case("Silver", 2)]
        }));
    let counter = document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["identity"] == "ix://example/counter-spec/Counter")
        .expect("Counter type");
    counter["fields"].as_array_mut().unwrap().push(json!({
        "identity": "ix://example/counter-spec/Counter/tier", "name": "tier",
        "typeRef": tier, "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": {"generated": {"generatorIdentity": tier,
            "generatorVersion": "1.0.0", "inputIdentities": [tier]}}
    }));
    serde_json::to_vec(&document).expect("the enum model serializes")
}

fn spec_models() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    vec![("spec.json", refinement_cas::SPEC, spec_doc())]
}

fn enum_spec_models() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    vec![("spec.json", refinement_cas::SPEC, enum_spec_doc())]
}

fn spec_member(profile: &str) -> Value {
    json!({"profile": profile, "obligations": ["Low"]})
}

const SPEC_UNIT: &str = "model Spec = \"example/counter-spec\";\n\
     pre AddWithin using v on Spec::Counter::add { self.value <= n }\n\
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
        "op_Counter_add(r_self, p_n) ==",
        "\\/ \\E r_self \\in DOMAIN v_Counter, p_n \\in 0..1 : op_Counter_add(r_self, p_n)",
        "p_n >= 0",
        "p_n <= 1",
        "\\A k \\in DOMAIN v_Counter : k /= r_self => v_Counter'[k].fld_value = v_Counter[k].fld_value",
        "\\A k \\in DOMAIN v_Counter : v_Counter'[k].fld_id = v_Counter[k].fld_id",
        "Init == TypeOK",
    ] {
        assert!(module.contains(part), "{part}: {module}");
    }
    for name in ["op_Counter_inc", "op_Counter_tick", "op_Counter_add"] {
        let entry = value["export"]["name_map"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap_or_else(|| panic!("{name} in the name map: {value}"));
        assert!(entry["node"].is_string(), "{entry}");
    }
    let parameter = value["export"]["name_map"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "p_n")
        .expect("the parameter binding is in the name map");
    assert!(parameter["binding"].is_string(), "{parameter}");
    assert!(parameter["node"].as_str().unwrap().contains("parameter"));
    assert!(parameter["binding"]
        .as_str()
        .unwrap()
        .contains("operation_anchor"));
}

/// The Lean `op_` of a receiver-only frame keeps its later conjuncts out of
/// the receiver guard: each frame line is parenthesized, so `WF s'` and the
/// remaining lines are not under `Not (k = r_self) ->`.
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
        .find(|block| block.contains("def op_Counter_tick"))
        .expect("op_tick");
    let guarded = tick
        .lines()
        .find(|line| line.contains("Not (k = r_self) ->"))
        .expect("the receiver frame line");
    assert!(
        guarded.trim_start().starts_with("(forall k,") && guarded.trim_end().ends_with(") /\\"),
        "{guarded}"
    );
    assert!(tick.contains("(WF s')"), "{tick}");
    assert!(
        file.contains(
            "def op_Counter_add (s : State) (s' : State) (r_self : List Nat) (p_n : Int)"
        ),
        "{file}"
    );
    assert!(file.contains("exists r_self, exists p_n,"), "{file}");
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
    assert!(module.contains("fld_busyA \\in BOOLEAN"), "{module}");
    assert!(
        module.contains("~(v_Counter[r_self].fld_busyA /\\ v_Counter[r_self].fld_busyB)"),
        "{module}"
    );
    assert!(module.contains(" \\/ "), "{module}");
    let (exit, value) = exported(&models, &unit, &member("filament.output.lean4/v1"));
    assert_eq!(exit, 0, "{value}");
    let file = text(&value);
    assert!(file.contains(".fld_busyA = true)"), "{file}");
    assert!(file.contains("Not ("), "{file}");
    assert!(file.contains("fld_busyA : Bool"), "{file}");
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
        module.contains("~(v_Counter[r_self].fld_id = 0) => ((0 <="),
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
    assert!(module.contains("op_Counter_inc(r_self) =="), "{module}");
    assert!(module.contains("op_Gauge_inc(r_self) =="), "{module}");
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
    assert!(
        names.contains(&"op_Counter_inc") && names.contains(&"op_Gauge_inc"),
        "{names:?}"
    );
    let field_rows: Vec<&Value> = value["export"]["name_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["name"] == "fld_value")
        .collect();
    assert_eq!(field_rows.len(), 2, "{value}");
    assert_ne!(
        field_rows[0]["binding"], field_rows[1]["binding"],
        "{value}"
    );
    for entry in field_rows {
        assert_eq!(entry["node"], entry["binding"], "{entry}");
    }
    let (exit, value) = exported(&models, &unit, &member("filament.output.lean4/v1"));
    assert_eq!(exit, 0, "{value}");
    assert!(text(&value).contains("s'.v_Gauge = s.v_Gauge"), "{value}");
}

#[test]
#[trace("FSpec-FR-812-AC-1", "FSpec-FR-814-AC-3")]
fn enum_fields_lower_to_finite_tla_values_and_lean_constructors() {
    let models = enum_spec_models();
    let (exit, tla_value) = exported(
        &models,
        &spec_unit(),
        &spec_member("filament.output.tla2/v1"),
    );
    assert_eq!(exit, 0, "{tla_value}");
    let tla = text(&tla_value);
    assert!(
        tla.contains("v_Counter[k].fld_tier \\in {\"e_Tier_Gold\", \"e_Tier_Silver\"}"),
        "{tla}"
    );
    for name in ["e_Tier_Gold", "e_Tier_Silver"] {
        let entry = tla_value["export"]["name_map"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["name"] == name)
            .expect("enum case is in the name map");
        assert!(
            entry["node"].as_str().unwrap().contains("object_type"),
            "{entry}"
        );
        assert_eq!(entry["binding"], entry["node"], "{entry}");
    }

    let (exit, lean_value) = exported(
        &models,
        &spec_unit(),
        &spec_member("filament.output.lean4/v1"),
    );
    assert_eq!(exit, 0, "{lean_value}");
    let lean = text(&lean_value);
    assert!(
        lean.contains(
            "inductive E_Tier where\n  | e_Tier_Gold\n  | e_Tier_Silver\nderiving DecidableEq"
        ),
        "{lean}"
    );
    assert!(lean.contains("fld_tier : E_Tier"), "{lean}");
}

/// Population bounds (`bound`, FSpec FR-812-AC-6) are not lowered, so an
/// export over a bounded population is the typed `unimplemented_capability` refusal naming the bound
/// rather than a `preserved` record that drops it.
#[test]
fn a_bounded_population_is_an_unimplemented_capability() {
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
        let result = fsl_command::command::run(&path, &crate::negotiation::Fr290)
            .expect("a refused export document");
        assert_eq!(result.exit_code, 21, "{profile}");
        let value: Value = serde_json::from_str(&result.value.to_string()).unwrap();
        let refusal = refusal_of(&value);
        assert_eq!(refusal["construct"], "model/bound", "{value}");
        assert!(
            refusal["capability"]
                .as_str()
                .unwrap()
                .ends_with("model/bound"),
            "{value}"
        );
    }
}

/// An operation carrying domain-package `requires` or `ensures` clauses is
/// the typed `unimplemented_capability` refusal, never a `preserved` record that omits them.
#[test]
fn an_operation_contract_in_the_model_is_an_unimplemented_capability() {
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
    let value = refused(
        &models,
        &spec_unit(),
        &spec_member("filament.output.tla2/v1"),
    );
    let text = value.to_string();
    assert!(
        text.contains("model/operation") && text.contains("unimplemented_capability"),
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

/// An invariant over a type with a field this build does not lower is the
/// typed `unimplemented_capability` refusal naming the construct and its
/// node, exit 21, with no file and `last_stage` S4, under every floor and in
/// both targets (FSpec FR-817-AC-16).
#[test]
fn an_unlowered_state_construct_is_an_unimplemented_capability() {
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
            let result = fsl_command::command::run(&path, &crate::negotiation::Fr290)
                .expect("a refused export document");
            assert_eq!(result.exit_code, 21, "{profile} {floor:?}");
            let value: Value = serde_json::from_str(&result.value.to_string()).unwrap();
            let refusal = refusal_of(&value);
            assert_eq!(refusal["construct"], "model/field", "{value}");
            assert_eq!(
                refusal["target"],
                if profile.contains("tla") {
                    "tla"
                } else {
                    "lean"
                },
                "{value}"
            );
            assert!(
                refusal["node"].is_string() && refusal["locus"].is_object(),
                "{value}"
            );
            assert!(!directory.path().join("m.out").exists());
            let document = outcome_json(&result.outcome.expect("an outcome document"));
            assert_eq!(document["last_stage"], "S4", "{document}");
            assert_eq!(document["category"], "unsupported", "{document}");
        }
    }
}
